from datetime import date
from decimal import Decimal

from nautilus_india.clickhouse.queries import select_bars
from nautilus_india.clickhouse.queries import select_instruments
from nautilus_india.clickhouse.store import ClickHouseMarketStore
from nautilus_india.clickhouse.store import schema_statements
from nautilus_india.config import ClickHouseConfig
from nautilus_india.records import BarRecord


class _Result:
    def __init__(self, rows):
        self._rows = rows

    def named_results(self):
        return self._rows


class _Client:
    def __init__(self, rows):
        self.rows = rows
        self.calls = []

    def query(self, sql, parameters=None):
        self.calls.append((sql, parameters))
        return _Result(self.rows)


def test_bar_query_keeps_the_newest_limited_rows_in_time_order():
    query = select_bars(
        instrument_id="RELIANCE.NSE",
        bar_spec="1-MINUTE",
        start_ns=10,
        end_ns=50,
        limit=2,
    )

    assert "ts_event >= {start_ns:Int64}" in query.sql
    assert "ts_event < {end_ns:Int64}" in query.sql
    assert "ORDER BY ts_event DESC" in query.sql
    assert query.parameters["limit"] == 2
    assert query.parameters["end_ns"] == 50


def test_instrument_query_binds_venue_and_product_filters():
    query = select_instruments(venues=["NFO"], products=["option"], underlyings=["NIFTY"])

    assert query.parameters["venue"] == ["NFO"]
    assert query.parameters["product"] == ["option"]
    assert "FINAL" in query.sql


def test_store_maps_bar_rows_from_an_injected_client():
    client = _Client(
        [
            {
                "instrument_id": "RELIANCE.NSE",
                "bar_spec": "1-DAY",
                "ts_event": 1_700_000_000_000_000_000,
                "open": Decimal("2450.05"),
                "high": Decimal("2460.00"),
                "low": Decimal("2440.00"),
                "close": Decimal("2455.10"),
                "volume": 1000,
                "open_interest": None,
            }
        ]
    )
    store = ClickHouseMarketStore(ClickHouseConfig(), client=client)

    bars = store.fetch_bars(instrument_id="RELIANCE.NSE", bar_spec="1-DAY")

    assert bars == [
        BarRecord(
            instrument_id="RELIANCE.NSE",
            bar_spec="1-DAY",
            ts_event=1_700_000_000_000_000_000,
            open=Decimal("2450.05"),
            high=Decimal("2460.00"),
            low=Decimal("2440.00"),
            close=Decimal("2455.10"),
            volume=Decimal("1000"),
            open_interest=None,
        )
    ]
    assert client.calls[0][1]["instrument_id"] == "RELIANCE.NSE"


def test_schema_creates_the_configured_database():
    statements = schema_statements("market_lab")

    assert statements[0] == "CREATE DATABASE IF NOT EXISTS market_lab"
    assert any("market_lab.bars" in statement for statement in statements)
    assert any("market_lab.trading_holidays" in statement for statement in statements)
    assert all("nautilus_india" not in statement for statement in statements)


def test_holiday_dates_are_normalized():
    client = _Client([{"session_date": date(2026, 10, 2)}])
    store = ClickHouseMarketStore(ClickHouseConfig(), client=client)

    assert store.fetch_holidays("NSE") == {date(2026, 10, 2)}
