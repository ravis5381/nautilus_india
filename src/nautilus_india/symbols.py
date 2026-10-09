"""Canonical instrument ids for Indian listings.

The Nautilus id is ``{symbol}.{venue}``. The exchange ticker stays in
``raw_symbol`` (for example ``RELIANCE-EQ`` or ``NIFTY25JUNFUT``) and is not
parsed. Cash symbols may contain hyphens, as in ``BAJAJ-AUTO.NSE``.

Derivative symbols are parsed from the right:

- Future: ``{root}-{YYYYMMDD}-FUT.{venue}``
- Option: ``{root}-{YYYYMMDD}-{strike}-{CE|PE}.{venue}``

An index and a cash equity share the same shape. The ``product`` column in
ClickHouse distinguishes them.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import date
from datetime import datetime
from decimal import Decimal
from typing import Literal

from nautilus_india.venues import VENUES

ProductKind = Literal["cash", "future", "option"]
OptionSide = Literal["CALL", "PUT"]

_ROOT_CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789&")


@dataclass(frozen=True, slots=True)
class IndiaSymbol:
    root: str
    venue: str
    product: ProductKind
    expiry: date | None = None
    strike: Decimal | None = None
    option_kind: OptionSide | None = None

    @property
    def symbol(self) -> str:
        if self.product == "cash":
            return self.root
        if self.expiry is None:
            raise ValueError(f"{self.product} symbol {self.root} is missing an expiry")
        expiry = self.expiry.strftime("%Y%m%d")
        if self.product == "future":
            return f"{self.root}-{expiry}-FUT"
        if self.product == "option":
            if self.strike is None or self.option_kind is None:
                raise ValueError(f"Option {self.root} is missing a strike or kind")
            side = "CE" if self.option_kind == "CALL" else "PE"
            return f"{self.root}-{expiry}-{format_strike(self.strike)}-{side}"
        raise ValueError(f"Unsupported product {self.product}")

    @property
    def instrument_id(self) -> str:
        return f"{self.symbol}.{self.venue}"


def format_strike(value: Decimal) -> str:
    text = format(value, "f")
    if "." in text:
        text = text.rstrip("0").rstrip(".")
    return text


def parse_instrument_id(value: str) -> IndiaSymbol:
    """Parse a canonical instrument id. The input is normalized to uppercase."""
    text = value.strip().upper()
    if not text or any(char.isspace() for char in text):
        raise ValueError(f"Invalid instrument id {value!r}")
    symbol, separator, venue = text.rpartition(".")
    if not separator or not symbol or venue not in VENUES:
        known = ", ".join(sorted(VENUES))
        raise ValueError(f"Invalid instrument id {value!r}. Venue must be one of: {known}")

    future = _split_future(symbol)
    if future is not None:
        root, expiry = future
        _validate_root(root)
        return IndiaSymbol(root=root, venue=venue, product="future", expiry=expiry)

    option = _split_option(symbol)
    if option is not None:
        root, expiry, strike, kind = option
        _validate_root(root)
        return IndiaSymbol(
            root=root,
            venue=venue,
            product="option",
            expiry=expiry,
            strike=strike,
            option_kind=kind,
        )

    _validate_root(symbol)
    return IndiaSymbol(root=symbol, venue=venue, product="cash")


def _split_future(symbol: str) -> tuple[str, date] | None:
    root, separator, tail = symbol.rpartition("-")
    if separator != "-" or tail != "FUT":
        return None
    underlying, expiry_separator, expiry_text = root.rpartition("-")
    if expiry_separator != "-" or not _is_yyyymmdd(expiry_text) or not underlying:
        return None
    return underlying, _parse_expiry(expiry_text)


def _split_option(symbol: str) -> tuple[str, date, Decimal, OptionSide] | None:
    body, separator, side = symbol.rpartition("-")
    if separator != "-" or side not in {"CE", "PE"}:
        return None
    root_and_expiry, strike_separator, strike_text = body.rpartition("-")
    if strike_separator != "-" or not _is_strike(strike_text):
        return None
    underlying, expiry_separator, expiry_text = root_and_expiry.rpartition("-")
    if expiry_separator != "-" or not _is_yyyymmdd(expiry_text) or not underlying:
        return None
    kind: OptionSide = "CALL" if side == "CE" else "PUT"
    return underlying, _parse_expiry(expiry_text), Decimal(strike_text), kind


def _validate_root(root: str) -> None:
    pieces = root.split("-")
    if not pieces or any(not piece for piece in pieces):
        raise ValueError("Instrument symbol contains an empty component")
    for piece in pieces:
        if any(char not in _ROOT_CHARS for char in piece):
            raise ValueError(f"Unsupported character in symbol component {piece!r}")


def _is_yyyymmdd(value: str) -> bool:
    return len(value) == 8 and value.isdigit()


def _is_strike(value: str) -> bool:
    if not value:
        return False
    whole, dot, fraction = value.partition(".")
    if not whole.isdigit():
        return False
    return dot == "" or fraction.isdigit()


def _parse_expiry(value: str) -> date:
    try:
        return datetime.strptime(value, "%Y%m%d").date()
    except ValueError as exc:
        raise ValueError(f"Invalid expiry date {value}") from exc
