"""Copy a ClickHouse window into a Nautilus Parquet catalog for backtests.

``BacktestNode`` reads the Parquet catalog. The live node reads ClickHouse
directly through :class:`IndiaClickHouseDataClient`.
"""

from __future__ import annotations

from pathlib import Path

from nautilus_trader.model.data import BarType
from nautilus_trader.persistence.catalog.parquet import ParquetDataCatalog

from nautilus_india.adapt import to_bar
from nautilus_india.adapt import to_instrument
from nautilus_india.clickhouse.store import ClickHouseMarketStore


def export_parquet_catalog(
    store: ClickHouseMarketStore,
    catalog_path: str | Path,
    instrument_ids: list[str],
    bar_spec: str,
    start_ns: int | None = None,
    end_ns: int | None = None,
) -> ParquetDataCatalog:
    """Write instrument definitions and externally aggregated last bars."""
    catalog = ParquetDataCatalog(str(catalog_path))
    records = store.fetch_instruments(instrument_ids=instrument_ids)
    instruments = [to_instrument(record) for record in records]
    if instruments:
        catalog.write_data(instruments)
    for record in records:
        bar_type = BarType.from_str(f"{record.instrument_id}-{bar_spec}-LAST-EXTERNAL")
        bars = [
            to_bar(bar, bar_type, record.price_precision)
            for bar in store.fetch_bars(
                instrument_id=record.instrument_id,
                bar_spec=bar_spec,
                start_ns=start_ns,
                end_ns=end_ns,
            )
        ]
        if bars:
            catalog.write_data(bars)
    return catalog
