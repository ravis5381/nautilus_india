"""Map ClickHouse rows onto Nautilus instruments and market data.

Lot size is the exchange market lot in units. Quantity ordered on NSE is a
multiple of that lot. Multiplier stays 1 for cash equities and for F&O
contracts whose notional is price times quantity. Currency and commodity
contracts that quote a separate contract size set ``multiplier`` explicitly.

Open interest is kept on ``BarRecord``. The Nautilus ``Bar`` type has no
field for it.
"""

from __future__ import annotations

from decimal import Decimal

from nautilus_trader.model.data import Bar
from nautilus_trader.model.data import BarType
from nautilus_trader.model.data import QuoteTick
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.enums import AggregationSource
from nautilus_trader.model.enums import AggressorSide
from nautilus_trader.model.enums import AssetClass
from nautilus_trader.model.enums import BarAggregation
from nautilus_trader.model.enums import OptionKind
from nautilus_trader.model.enums import PriceType
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.identifiers import TradeId
from nautilus_trader.model.instruments import Equity
from nautilus_trader.model.instruments import FuturesContract
from nautilus_trader.model.instruments import IndexInstrument
from nautilus_trader.model.instruments import OptionContract
from nautilus_trader.model.objects import Currency
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity

from nautilus_india.records import BarRecord
from nautilus_india.records import InstrumentRecord
from nautilus_india.records import QuoteRecord
from nautilus_india.records import TradeRecord
from nautilus_india.symbols import parse_instrument_id

_ASSET_CLASS = {
    "EQUITY": AssetClass.EQUITY,
    "INDEX": AssetClass.INDEX,
    "COMMODITY": AssetClass.COMMODITY,
    "CURRENCY": AssetClass.FX,
    "DEBT": AssetClass.DEBT,
}

_AGGRESSOR = {
    "BUYER": AggressorSide.BUYER,
    "SELLER": AggressorSide.SELLER,
    "NO_AGGRESSOR": AggressorSide.NO_AGGRESSOR,
}


def bar_spec_key(bar_type: BarType) -> str:
    """Return the ClickHouse ``bar_spec`` for an externally aggregated last bar."""
    require_external_last_bar(bar_type)
    aggregation = BarAggregation(bar_type.spec.aggregation).name
    return f"{bar_type.spec.step}-{aggregation}"


def require_external_last_bar(bar_type: BarType) -> None:
    source = AggregationSource(bar_type.aggregation_source)
    price_type = PriceType(bar_type.spec.price_type)
    if source != AggregationSource.EXTERNAL or price_type != PriceType.LAST:
        raise ValueError(
            f"{bar_type} must be LAST price with EXTERNAL aggregation. "
            "ClickHouse stores exchange bars and does not aggregate ticks.",
        )


def to_instrument(record: InstrumentRecord, ts_init: int = 0):
    parsed = parse_instrument_id(record.instrument_id)
    if parsed.venue != record.venue:
        raise ValueError(
            f"{record.instrument_id} venue does not match row venue {record.venue}",
        )
    increment = to_price(record.price_increment, record.price_precision)
    currency = Currency.from_str(record.currency)
    instrument_id = InstrumentId.from_str(record.instrument_id)
    raw_symbol = Symbol(record.raw_symbol)
    info = _info(record)
    ts_event = record.activation_ns or 0

    if record.product == "equity":
        return Equity(
            instrument_id=instrument_id,
            raw_symbol=raw_symbol,
            currency=currency,
            price_precision=record.price_precision,
            price_increment=increment,
            lot_size=_whole_quantity(record.lot_size, "lot_size"),
            ts_event=ts_event,
            ts_init=ts_init,
            isin=record.isin or None,
            info=info,
        )
    if record.product == "index":
        return IndexInstrument(
            instrument_id=instrument_id,
            raw_symbol=raw_symbol,
            currency=currency,
            price_precision=record.price_precision,
            size_precision=0,
            price_increment=increment,
            size_increment=Quantity.from_int(1),
            ts_event=ts_event,
            ts_init=ts_init,
            info=info,
        )
    if record.product == "future":
        return FuturesContract(
            instrument_id=instrument_id,
            raw_symbol=raw_symbol,
            asset_class=_asset_class(record.asset_class),
            currency=currency,
            price_precision=record.price_precision,
            price_increment=increment,
            multiplier=_whole_quantity(record.multiplier, "multiplier"),
            lot_size=_whole_quantity(record.lot_size, "lot_size"),
            underlying=record.underlying,
            activation_ns=record.activation_ns or 0,
            expiration_ns=_required_ns(record.expiration_ns, record.instrument_id),
            ts_event=ts_event,
            ts_init=ts_init,
            exchange=record.venue,
            info=info,
        )
    if record.product == "option":
        kind = record.nautilus_option_kind
        if kind is None or record.strike is None:
            raise ValueError(f"Option {record.instrument_id} is missing strike or kind")
        return OptionContract(
            instrument_id=instrument_id,
            raw_symbol=raw_symbol,
            asset_class=_asset_class(record.asset_class),
            currency=currency,
            price_precision=record.price_precision,
            price_increment=increment,
            multiplier=_whole_quantity(record.multiplier, "multiplier"),
            lot_size=_whole_quantity(record.lot_size, "lot_size"),
            underlying=record.underlying,
            option_kind=OptionKind.CALL if kind == "CALL" else OptionKind.PUT,
            strike_price=to_price(record.strike, record.price_precision),
            activation_ns=record.activation_ns or 0,
            expiration_ns=_required_ns(record.expiration_ns, record.instrument_id),
            ts_event=ts_event,
            ts_init=ts_init,
            exchange=record.venue,
            info=info,
        )
    raise ValueError(f"Unsupported product {record.product!r} on {record.instrument_id}")


def to_bar(record: BarRecord, bar_type: BarType, price_precision: int, ts_init: int | None = None) -> Bar:
    if record.instrument_id != bar_type.instrument_id.value:
        raise ValueError(
            f"Bar {record.instrument_id} does not match {bar_type.instrument_id}",
        )
    if record.bar_spec != bar_spec_key(bar_type):
        raise ValueError(f"Bar spec {record.bar_spec} does not match {bar_type}")
    return Bar(
        bar_type=bar_type,
        open=to_price(record.open, price_precision),
        high=to_price(record.high, price_precision),
        low=to_price(record.low, price_precision),
        close=to_price(record.close, price_precision),
        volume=_whole_quantity(record.volume, "volume"),
        ts_event=record.ts_event,
        ts_init=record.ts_event if ts_init is None else ts_init,
    )


def to_trade(record: TradeRecord, price_precision: int, ts_init: int | None = None) -> TradeTick:
    return TradeTick(
        instrument_id=InstrumentId.from_str(record.instrument_id),
        price=to_price(record.price, price_precision),
        size=_whole_quantity(record.size, "size"),
        aggressor_side=_AGGRESSOR[record.aggressor_side],
        trade_id=TradeId(record.trade_id),
        ts_event=record.ts_event,
        ts_init=record.ts_event if ts_init is None else ts_init,
    )


def to_quote(record: QuoteRecord, price_precision: int, ts_init: int | None = None) -> QuoteTick:
    if record.bid_price <= 0 or record.ask_price <= 0:
        raise ValueError(f"Quote for {record.instrument_id} has a non-positive price")
    return QuoteTick(
        instrument_id=InstrumentId.from_str(record.instrument_id),
        bid_price=to_price(record.bid_price, price_precision),
        ask_price=to_price(record.ask_price, price_precision),
        bid_size=_whole_quantity(record.bid_size, "bid_size"),
        ask_size=_whole_quantity(record.ask_size, "ask_size"),
        ts_event=record.ts_event,
        ts_init=record.ts_event if ts_init is None else ts_init,
    )


def to_price(value: Decimal, precision: int) -> Price:
    quantum = Decimal("1").scaleb(-precision)
    quantized = Decimal(value).quantize(quantum)
    if quantized != Decimal(value):
        raise ValueError(f"Price {value} does not fit precision {precision}")
    return Price.from_str(f"{quantized:.{precision}f}")


def _whole_quantity(value: Decimal, label: str) -> Quantity:
    number = Decimal(value)
    if number <= 0 or number != number.to_integral_value():
        raise ValueError(f"{label} must be a positive whole number, got {value}")
    return Quantity.from_int(int(number))


def _asset_class(name: str) -> AssetClass:
    try:
        return _ASSET_CLASS[name]
    except KeyError as exc:
        raise ValueError(f"Unsupported asset class {name!r}") from exc


def _required_ns(value: int | None, instrument_id: str) -> int:
    if value is None:
        raise ValueError(f"{instrument_id} is missing expiration_ns")
    return value


def _info(record: InstrumentRecord) -> dict[str, str]:
    info = {
        "segment": record.segment,
        "series": record.series,
        "product": record.product,
    }
    if record.isin:
        info["isin"] = record.isin
    return info
