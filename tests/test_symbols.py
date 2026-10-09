from datetime import date
from decimal import Decimal

import pytest

from nautilus_india.symbols import parse_instrument_id


@pytest.mark.parametrize(
    ("instrument_id", "root", "venue", "product"),
    [
        ("RELIANCE.NSE", "RELIANCE", "NSE", "cash"),
        ("bajaj-auto.bse", "BAJAJ-AUTO", "BSE", "cash"),
        ("M&M.NSE", "M&M", "NSE", "cash"),
    ],
)
def test_cash_symbols_round_trip(instrument_id, root, venue, product):
    parsed = parse_instrument_id(instrument_id)

    assert parsed.root == root
    assert parsed.venue == venue
    assert parsed.product == product
    assert parsed.instrument_id == instrument_id.upper()


def test_future_symbol_round_trip():
    parsed = parse_instrument_id("NIFTY-20260625-FUT.NFO")

    assert parsed.product == "future"
    assert parsed.root == "NIFTY"
    assert parsed.expiry == date(2026, 6, 25)
    assert parsed.instrument_id == "NIFTY-20260625-FUT.NFO"


def test_option_symbol_keeps_fractional_strike():
    parsed = parse_instrument_id("banknifty-20260625-25000.5-pe.nfo")

    assert parsed.product == "option"
    assert parsed.root == "BANKNIFTY"
    assert parsed.strike == Decimal("25000.5")
    assert parsed.option_kind == "PUT"
    assert parsed.instrument_id == "BANKNIFTY-20260625-25000.5-PE.NFO"


def test_hyphenated_underlying_future():
    parsed = parse_instrument_id("BAJAJ-AUTO-20260625-FUT.NFO")

    assert parsed.root == "BAJAJ-AUTO"
    assert parsed.expiry == date(2026, 6, 25)


@pytest.mark.parametrize(
    "instrument_id",
    ["RELIANCE", "RELIANCE.NYSE", "NIFTY-20260231-FUT.NFO", "NIFTY-20260625-FUT", " "],
)
def test_invalid_instrument_ids_are_rejected(instrument_id):
    with pytest.raises(ValueError):
        parse_instrument_id(instrument_id)
