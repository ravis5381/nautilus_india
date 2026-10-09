from datetime import date
from decimal import Decimal

import pytest
from nautilus_trader.model.data import BarType
from nautilus_trader.model.enums import AssetClass
from nautilus_trader.model.enums import OptionKind

from nautilus_india.adapt import bar_spec_key
from nautilus_india.adapt import to_bar
from nautilus_india.adapt import to_instrument
from nautilus_india.adapt import to_price
from nautilus_india.records import BarRecord
from nautilus_india.records import InstrumentRecord


def _record(**overrides) -> InstrumentRecord:
    fields = dict(
        instrument_id="RELIANCE.NSE",
        venue="NSE",
        raw_symbol="RELIANCE-EQ",
        product="equity",
        asset_class="EQUITY",
        underlying="RELIANCE",
        expiry=None,
        strike=None,
        option_kind=None,
        currency="INR",
        price_precision=2,
        price_increment=Decimal("0.05"),
        lot_size=Decimal("1"),
        multiplier=Decimal("1"),
        isin="INE002A01018",
        series="EQ",
        segment="CM",
        activation_ns=0,
        expiration_ns=None,
    )
    fields.update(overrides)
    return InstrumentRecord(**fields)


def test_equity_uses_inr_and_the_exchange_tick():
    instrument = to_instrument(_record())

    assert instrument.id.value == "RELIANCE.NSE"
    assert instrument.quote_currency.code == "INR"
    assert instrument.price_increment == to_price(Decimal("0.05"), 2)
    assert instrument.lot_size == 1
    assert instrument.isin == "INE002A01018"


def test_index_future_keeps_lot_size_separate_from_multiplier():
    instrument = to_instrument(
        _record(
            instrument_id="NIFTY-20260625-FUT.NFO",
            venue="NFO",
            raw_symbol="NIFTY25JUNFUT",
            product="future",
            asset_class="INDEX",
            underlying="NIFTY",
            expiry=date(2026, 6, 25),
            lot_size=Decimal("65"),
            multiplier=Decimal("1"),
            series="",
            segment="FO",
            isin="",
            expiration_ns=1_782_345_600_000_000_000,
        )
    )

    assert instrument.asset_class == AssetClass.INDEX
    assert instrument.lot_size == 65
    assert instrument.multiplier == 1
    assert instrument.underlying == "NIFTY"
    assert instrument.expiration_ns == 1_782_345_600_000_000_000


def test_option_contract_maps_call_and_strike():
    instrument = to_instrument(
        _record(
            instrument_id="NIFTY-20260625-25000-CE.NFO",
            venue="NFO",
            raw_symbol="NIFTY25JUN25000CE",
            product="option",
            asset_class="INDEX",
            underlying="NIFTY",
            expiry=date(2026, 6, 25),
            strike=Decimal("25000"),
            option_kind="CE",
            lot_size=Decimal("65"),
            series="",
            segment="FO",
            isin="",
            expiration_ns=1_782_345_600_000_000_000,
        )
    )

    assert instrument.option_kind == OptionKind.CALL
    assert instrument.strike_price == to_price(Decimal("25000"), 2)


def test_bar_uses_the_row_close_timestamp():
    bar_type = BarType.from_str("RELIANCE.NSE-1-MINUTE-LAST-EXTERNAL")
    record = BarRecord(
        instrument_id="RELIANCE.NSE",
        bar_spec="1-MINUTE",
        ts_event=1_700_000_000_000_000_000,
        open=Decimal("100.05"),
        high=Decimal("101.00"),
        low=Decimal("99.50"),
        close=Decimal("100.55"),
        volume=Decimal("250"),
        open_interest=Decimal("1000"),
    )

    bar = to_bar(record, bar_type, price_precision=2)

    assert bar_spec_key(bar_type) == "1-MINUTE"
    assert bar.ts_event == record.ts_event
    assert bar.ts_init == record.ts_event
    assert bar.close == to_price(Decimal("100.55"), 2)
    assert bar.volume == 250


def test_internal_bars_are_rejected():
    bar_type = BarType.from_str("RELIANCE.NSE-1-MINUTE-LAST-INTERNAL")

    with pytest.raises(ValueError, match="EXTERNAL"):
        bar_spec_key(bar_type)


def test_price_precision_must_match_the_instrument():
    with pytest.raises(ValueError, match="precision"):
        to_price(Decimal("0.005"), 2)
