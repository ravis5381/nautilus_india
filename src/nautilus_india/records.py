"""Row shapes returned by ClickHouse, independent of Nautilus objects."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import date
from decimal import Decimal
from typing import Literal

Product = Literal["equity", "index", "future", "option"]
AssetClassName = Literal["EQUITY", "INDEX", "COMMODITY", "CURRENCY", "DEBT"]
Aggressor = Literal["BUYER", "SELLER", "NO_AGGRESSOR"]


@dataclass(frozen=True, slots=True)
class InstrumentRecord:
    instrument_id: str
    venue: str
    raw_symbol: str
    product: Product
    asset_class: AssetClassName
    underlying: str
    expiry: date | None
    strike: Decimal | None
    option_kind: str | None
    currency: str
    price_precision: int
    price_increment: Decimal
    lot_size: Decimal
    multiplier: Decimal
    isin: str
    series: str
    segment: str
    activation_ns: int | None
    expiration_ns: int | None

    @property
    def nautilus_option_kind(self) -> str | None:
        if self.option_kind in {None, ""}:
            return None
        normalized = self.option_kind.upper()
        if normalized in {"CE", "CALL"}:
            return "CALL"
        if normalized in {"PE", "PUT"}:
            return "PUT"
        raise ValueError(f"Unknown option kind {self.option_kind!r}")


@dataclass(frozen=True, slots=True)
class BarRecord:
    instrument_id: str
    bar_spec: str
    ts_event: int
    open: Decimal
    high: Decimal
    low: Decimal
    close: Decimal
    volume: Decimal
    open_interest: Decimal | None = None


@dataclass(frozen=True, slots=True)
class TradeRecord:
    instrument_id: str
    ts_event: int
    trade_id: str
    price: Decimal
    size: Decimal
    aggressor_side: Aggressor


@dataclass(frozen=True, slots=True)
class QuoteRecord:
    instrument_id: str
    ts_event: int
    bid_price: Decimal
    ask_price: Decimal
    bid_size: Decimal
    ask_size: Decimal
