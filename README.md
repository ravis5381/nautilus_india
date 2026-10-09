# Nautilus India

Indian market data for [NautilusTrader](https://nautilustrader.io/) 1.231, with ClickHouse as the store. Nautilus is a dependency. Strategies and research code stay in Python.

This package reads instruments, bars, trades, and quotes. It does not submit orders. A broker adapter can be added later on the same instrument ids.

## Layout

```text
src/nautilus_india/     Python interface
  sql/                  ClickHouse schema
  clickhouse/           Queries and the row reader
  adapt.py              Rows to Nautilus instruments, bars, trades, and quotes
  data_client.py        Live data client registered on TradingNode
  catalog.py            ClickHouse window to the Parquet catalog used by backtests
native/                 Same instrument-id grammar in Rust, for a later batch decoder
examples/live_node.py   Wires the data client into a live node
```

The installed Nautilus 1.231 wheel exposes `LiveMarketDataClient`. The data client subclasses that. The Rust crate does not link Nautilus model types. Python constructs those objects from the installed wheel.

## Instrument ids

The Nautilus id is `{symbol}.{venue}`. The exchange ticker is stored separately as `raw_symbol`.

| Listing | Instrument id | Venue |
| --- | --- | --- |
| Cash equity | `RELIANCE.NSE`, `BAJAJ-AUTO.BSE` | `NSE`, `BSE` |
| Index | `NIFTY.NSE` | `NSE` |
| Future | `NIFTY-20260625-FUT.NFO` | `NFO`, `BFO`, `CDS`, `MCX` |
| Option | `NIFTY-20260625-25000-CE.NFO` | `NFO`, `BFO`, `CDS`, `MCX` |

`CE` is a call and `PE` is a put. The date is the contract expiry date. Lot size is the exchange market lot in units, and multiplier stays `1` unless the contract spec says otherwise. Order quantity is a multiple of the lot. Open interest is stored on the bar row. A Nautilus `Bar` has no open-interest field, so it remains on `BarRecord`.

Bars in ClickHouse are already aggregated. Request them as external last bars, for example `RELIANCE.NSE-1-MINUTE-LAST-EXTERNAL`. `bar_spec` in the table is `1-MINUTE`. `ts_event` is the bar close, in UNIX nanoseconds UTC. A request window is inclusive of `start` and exclusive of `end`.

## Sessions

Default continuous sessions are in `Asia/Kolkata`:

| Venue | Continuous session |
| --- | --- |
| `NSE`, `BSE` | 09:15–15:30 |
| `NFO`, `BFO` | 09:15–15:40 |
| `CDS` | 09:00–17:00 |
| `MCX` | no default; hours depend on the commodity |

The official close is inside the window. Weekends are closed. Exchange holidays come from `trading_holidays`. When hours change, record the new window in `sessions` instead of editing the defaults.

## ClickHouse

```bash
python -c "from nautilus_india import ClickHouseConfig, ClickHouseMarketStore; ClickHouseMarketStore(ClickHouseConfig.from_env()).apply_schema()"
```

Environment variables: `CLICKHOUSE_HOST`, `CLICKHOUSE_PORT`, `CLICKHOUSE_USER`, `CLICKHOUSE_PASSWORD`, `CLICKHOUSE_DATABASE`, `CLICKHOUSE_SECURE`.

`instruments` holds the contract master. `bars`, `trades`, and `quotes` hold market data partitioned by Indian session month. A feeder, not this package, writes the rows. The live client polls for timestamps newer than the subscription.

## Live node

```python
from nautilus_trader.config import RoutingConfig, TradingNodeConfig
from nautilus_trader.live.node import TradingNode

from nautilus_india import (
    CLIENT_NAME,
    IndiaClickHouseDataClientFactory,
    IndiaDataClientConfig,
)
from nautilus_india.venues import INDIA_VENUE_CODES

config = TradingNodeConfig(
    trader_id="INDIA-001",
    data_clients={
        CLIENT_NAME: IndiaDataClientConfig(
            host="localhost",
            routing=RoutingConfig(venues=frozenset(INDIA_VENUE_CODES)),
        ),
    },
)
node = TradingNode(config=config)
node.add_data_client_factory(CLIENT_NAME, IndiaClickHouseDataClientFactory)
node.build()
```

`examples/live_node.py` builds that node. It connects only when `RUN_NODE=1`.

One client serves every Indian venue. Set `instrument_provider.load_ids` to the contracts you need. `load_all=True` reads the whole `instruments` table, including the F&O universe.

## Backtests

`BacktestNode` reads a Parquet catalog. Copy a window out of ClickHouse, then point the backtest at that catalog:

```python
from nautilus_india import ClickHouseConfig, ClickHouseMarketStore, export_parquet_catalog

store = ClickHouseMarketStore(ClickHouseConfig.from_env())
store.connect()
export_parquet_catalog(
    store,
    "catalog",
    ["RELIANCE.NSE"],
    "1-MINUTE",
)
```

## Rust parser

`native/` parses the same instrument ids and is covered by `cargo test --manifest-path native/Cargo.toml`. The Python parser is the one the data client calls. Move batch decoding into this crate when a ClickHouse scan needs to leave the GIL. Build any future extension against the installed Nautilus wheel and return Python objects. Do not vendor a second copy of the Nautilus model.

## Development

```bash
python3 -m venv .venv
.venv/bin/python -m pip install -e ".[dev]"
.venv/bin/python -m pytest
cargo test --manifest-path native/Cargo.toml
```

Requires Python 3.12–3.14, matching the Nautilus 1.231 wheel.
