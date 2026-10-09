"""Indian market adapter for NautilusTrader."""

from nautilus_india.catalog import export_parquet_catalog
from nautilus_india.clickhouse.store import ClickHouseMarketStore
from nautilus_india.config import ClickHouseConfig
from nautilus_india.data_client import CLIENT_NAME
from nautilus_india.data_client import IndiaClickHouseDataClient
from nautilus_india.data_client import IndiaClickHouseDataClientFactory
from nautilus_india.data_client import IndiaDataClientConfig
from nautilus_india.provider import IndiaInstrumentProvider

__all__ = [
    "CLIENT_NAME",
    "ClickHouseConfig",
    "ClickHouseMarketStore",
    "IndiaClickHouseDataClient",
    "IndiaClickHouseDataClientFactory",
    "IndiaDataClientConfig",
    "IndiaInstrumentProvider",
    "export_parquet_catalog",
]

__version__ = "0.1.0"
