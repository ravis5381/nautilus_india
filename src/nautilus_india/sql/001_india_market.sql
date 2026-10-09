-- Indian market store for NautilusTrader.
-- Timestamps are UNIX nanoseconds (UTC). session_date is the exchange date
-- in Asia/Kolkata, which is the date used for holidays and contract expiry.
-- Bar ts_event is the bar close. A Nautilus request treats start as inclusive
-- and end as exclusive.

CREATE DATABASE IF NOT EXISTS nautilus_india;

CREATE TABLE IF NOT EXISTS nautilus_india.instruments
(
    instrument_id String,
    venue LowCardinality(String),
    raw_symbol String,
    product LowCardinality(String),
    asset_class LowCardinality(String),
    underlying String,
    expiry Nullable(Date),
    strike Nullable(Decimal(18, 4)),
    option_kind LowCardinality(String),
    currency FixedString(3),
    price_precision UInt8,
    price_increment Decimal(18, 8),
    lot_size Decimal(18, 4),
    multiplier Decimal(18, 4),
    isin String,
    series LowCardinality(String),
    segment LowCardinality(String),
    activation_ns Nullable(Int64),
    expiration_ns Nullable(Int64),
    updated_at DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY (venue, instrument_id);

CREATE TABLE IF NOT EXISTS nautilus_india.bars
(
    instrument_id String,
    venue LowCardinality(String),
    session_date Date,
    bar_spec LowCardinality(String),
    ts_event Int64,
    open Decimal(18, 8),
    high Decimal(18, 8),
    low Decimal(18, 8),
    close Decimal(18, 8),
    volume UInt64,
    open_interest Nullable(UInt64),
    updated_at DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(updated_at)
PARTITION BY toYYYYMM(session_date)
ORDER BY (instrument_id, bar_spec, ts_event);

CREATE TABLE IF NOT EXISTS nautilus_india.trades
(
    instrument_id String,
    venue LowCardinality(String),
    session_date Date,
    ts_event Int64,
    trade_id String,
    price Decimal(18, 8),
    size UInt64,
    aggressor_side LowCardinality(String),
    updated_at DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(updated_at)
PARTITION BY toYYYYMM(session_date)
ORDER BY (instrument_id, ts_event, trade_id);

CREATE TABLE IF NOT EXISTS nautilus_india.quotes
(
    instrument_id String,
    venue LowCardinality(String),
    session_date Date,
    ts_event Int64,
    bid_price Decimal(18, 8),
    ask_price Decimal(18, 8),
    bid_size UInt64,
    ask_size UInt64,
    updated_at DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(updated_at)
PARTITION BY toYYYYMM(session_date)
ORDER BY (instrument_id, ts_event);

CREATE TABLE IF NOT EXISTS nautilus_india.trading_holidays
(
    venue LowCardinality(String),
    session_date Date,
    name String,
    updated_at DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY (venue, session_date);

CREATE TABLE IF NOT EXISTS nautilus_india.sessions
(
    venue LowCardinality(String),
    effective_from Date,
    name LowCardinality(String),
    kind LowCardinality(String),
    open_local String,
    close_local String,
    updated_at DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY (venue, effective_from, kind, name);
