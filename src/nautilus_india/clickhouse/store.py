"""Read Indian market rows from ClickHouse."""

from __future__ import annotations

from datetime import date
from datetime import datetime
from decimal import Decimal
from importlib.resources import files
from typing import Any

from nautilus_india.clickhouse.queries import BoundQuery
from nautilus_india.clickhouse.queries import select_bars
from nautilus_india.clickhouse.queries import select_holidays
from nautilus_india.clickhouse.queries import select_instruments
from nautilus_india.clickhouse.queries import select_quotes
from nautilus_india.clickhouse.queries import select_trades
from nautilus_india.config import ClickHouseConfig
from nautilus_india.records import BarRecord
from nautilus_india.records import InstrumentRecord
from nautilus_india.records import QuoteRecord
from nautilus_india.records import TradeRecord


class ClickHouseMarketStore:
    """Synchronous reader. Call it from a worker thread inside the live client."""

    def __init__(self, config: ClickHouseConfig, client: Any | None = None) -> None:
        self.config = config
        self._client = client

    @property
    def connected(self) -> bool:
        return self._client is not None

    def connect(self) -> None:
        if self._client is not None:
            return
        import clickhouse_connect

        self._client = clickhouse_connect.get_client(
            host=self.config.host,
            port=self.config.port,
            username=self.config.username,
            password=self.config.password,
            database=self.config.database,
            secure=self.config.secure,
        )

    def close(self) -> None:
        client = self._client
        self._client = None
        if client is not None and hasattr(client, "close"):
            client.close()

    def apply_schema(self) -> None:
        """Create the database and tables.

        Uses a separate connection so this can run before ``nautilus_india``
        exists. An already connected reader is left unchanged.
        """
        import clickhouse_connect

        bootstrap = clickhouse_connect.get_client(
            host=self.config.host,
            port=self.config.port,
            username=self.config.username,
            password=self.config.password,
            secure=self.config.secure,
        )
        try:
            for statement in schema_statements(self.config.database):
                bootstrap.command(statement)
        finally:
            bootstrap.close()

    def fetch_instruments(
        self,
        *,
        instrument_ids: list[str] | None = None,
        venues: list[str] | None = None,
        products: list[str] | None = None,
        underlyings: list[str] | None = None,
    ) -> list[InstrumentRecord]:
        query = select_instruments(
            instrument_ids=instrument_ids,
            venues=venues,
            products=products,
            underlyings=underlyings,
        )
        return [instrument_record(row) for row in self._query(query)]

    def fetch_bars(
        self,
        *,
        instrument_id: str,
        bar_spec: str,
        start_ns: int | None = None,
        end_ns: int | None = None,
        limit: int | None = None,
    ) -> list[BarRecord]:
        query = select_bars(
            instrument_id=instrument_id,
            bar_spec=bar_spec,
            start_ns=start_ns,
            end_ns=end_ns,
            limit=limit,
        )
        return [bar_record(row) for row in self._query(query)]

    def fetch_trades(
        self,
        *,
        instrument_id: str,
        start_ns: int | None = None,
        end_ns: int | None = None,
        limit: int | None = None,
    ) -> list[TradeRecord]:
        query = select_trades(
            instrument_id=instrument_id,
            start_ns=start_ns,
            end_ns=end_ns,
            limit=limit,
        )
        return [trade_record(row) for row in self._query(query)]

    def fetch_quotes(
        self,
        *,
        instrument_id: str,
        start_ns: int | None = None,
        end_ns: int | None = None,
        limit: int | None = None,
    ) -> list[QuoteRecord]:
        query = select_quotes(
            instrument_id=instrument_id,
            start_ns=start_ns,
            end_ns=end_ns,
            limit=limit,
        )
        return [quote_record(row) for row in self._query(query)]

    def fetch_holidays(self, venue: str) -> set[date]:
        query = select_holidays(venue=venue)
        return {_as_date(row["session_date"]) for row in self._query(query)}

    def _query(self, query: BoundQuery) -> list[dict[str, Any]]:
        result = self._require_client().query(query.sql, parameters=query.parameters)
        named = getattr(result, "named_results", None)
        if callable(named):
            return list(named())
        raise TypeError("ClickHouse query result does not provide named_results()")

    def _require_client(self) -> Any:
        if self._client is None:
            raise RuntimeError("ClickHouse client is not connected")
        return self._client


def _validate_database(database: str) -> None:
    if not database or any(not (char.isalnum() or char == "_") for char in database):
        raise ValueError(f"Invalid ClickHouse database name {database!r}")
    if database[0].isdigit():
        raise ValueError(f"Invalid ClickHouse database name {database!r}")


def schema_sql() -> str:
    return files("nautilus_india").joinpath("sql/001_india_market.sql").read_text(encoding="utf-8")


def schema_statements(database: str = "nautilus_india") -> list[str]:
    _validate_database(database)
    statements: list[str] = []
    script = schema_sql().replace("nautilus_india", database)
    for chunk in script.split(";"):
        lines = [
            line
            for line in chunk.splitlines()
            if line.strip() and not line.strip().startswith("--")
        ]
        statement = "\n".join(lines).strip()
        if statement:
            statements.append(statement)
    return statements


def instrument_record(row: dict[str, Any]) -> InstrumentRecord:
    return InstrumentRecord(
        instrument_id=str(row["instrument_id"]),
        venue=str(row["venue"]),
        raw_symbol=str(row["raw_symbol"]),
        product=row["product"],
        asset_class=row["asset_class"],
        underlying=str(row["underlying"] or ""),
        expiry=_optional_date(row.get("expiry")),
        strike=_optional_decimal(row.get("strike")),
        option_kind=_optional_text(row.get("option_kind")),
        currency=str(row["currency"]),
        price_precision=int(row["price_precision"]),
        price_increment=_decimal(row["price_increment"]),
        lot_size=_decimal(row["lot_size"]),
        multiplier=_decimal(row["multiplier"]),
        isin=str(row.get("isin") or ""),
        series=str(row.get("series") or ""),
        segment=str(row.get("segment") or ""),
        activation_ns=_optional_int(row.get("activation_ns")),
        expiration_ns=_optional_int(row.get("expiration_ns")),
    )


def bar_record(row: dict[str, Any]) -> BarRecord:
    return BarRecord(
        instrument_id=str(row["instrument_id"]),
        bar_spec=str(row["bar_spec"]),
        ts_event=int(row["ts_event"]),
        open=_decimal(row["open"]),
        high=_decimal(row["high"]),
        low=_decimal(row["low"]),
        close=_decimal(row["close"]),
        volume=_decimal(row["volume"]),
        open_interest=_optional_decimal(row.get("open_interest")),
    )


def trade_record(row: dict[str, Any]) -> TradeRecord:
    side = str(row.get("aggressor_side") or "NO_AGGRESSOR").upper()
    if side not in {"BUYER", "SELLER", "NO_AGGRESSOR"}:
        side = "NO_AGGRESSOR"
    return TradeRecord(
        instrument_id=str(row["instrument_id"]),
        ts_event=int(row["ts_event"]),
        trade_id=str(row["trade_id"]),
        price=_decimal(row["price"]),
        size=_decimal(row["size"]),
        aggressor_side=side,  # type: ignore[arg-type]
    )


def quote_record(row: dict[str, Any]) -> QuoteRecord:
    return QuoteRecord(
        instrument_id=str(row["instrument_id"]),
        ts_event=int(row["ts_event"]),
        bid_price=_decimal(row["bid_price"]),
        ask_price=_decimal(row["ask_price"]),
        bid_size=_decimal(row["bid_size"]),
        ask_size=_decimal(row["ask_size"]),
    )


def _decimal(value: Any) -> Decimal:
    if isinstance(value, Decimal):
        return value
    return Decimal(str(value))


def _optional_decimal(value: Any) -> Decimal | None:
    if value is None or value == "":
        return None
    return _decimal(value)


def _optional_int(value: Any) -> int | None:
    if value is None or value == "":
        return None
    return int(value)


def _optional_text(value: Any) -> str | None:
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def _optional_date(value: Any) -> date | None:
    if value is None or value == "":
        return None
    return _as_date(value)


def _as_date(value: Any) -> date:
    if isinstance(value, datetime):
        return value.date()
    if isinstance(value, date):
        return value
    return date.fromisoformat(str(value)[:10])
