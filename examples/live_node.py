"""Register the ClickHouse data client on a Nautilus live node.

The process exits before ``node.run()`` unless ``RUN_NODE=1``. Running the
node connects to ClickHouse and blocks until the node stops.
"""

from __future__ import annotations

import os

from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.config import RoutingConfig
from nautilus_trader.config import TradingNodeConfig
from nautilus_trader.live.node import TradingNode
from nautilus_trader.model.identifiers import InstrumentId

from nautilus_india import CLIENT_NAME
from nautilus_india import IndiaClickHouseDataClientFactory
from nautilus_india import IndiaDataClientConfig
from nautilus_india.config import ClickHouseConfig
from nautilus_india.venues import INDIA_VENUE_CODES


def build_node() -> TradingNode:
    clickhouse = ClickHouseConfig.from_env()
    config = TradingNodeConfig(
        trader_id="INDIA-001",
        data_clients={
            CLIENT_NAME: IndiaDataClientConfig(
                host=clickhouse.host,
                port=clickhouse.port,
                username=clickhouse.username,
                password=clickhouse.password,
                database=clickhouse.database,
                secure=clickhouse.secure,
                instrument_provider=InstrumentProviderConfig(
                    load_ids=frozenset(
                        InstrumentId.from_str(item.strip())
                        for item in os.environ.get("INDIA_INSTRUMENTS", "RELIANCE.NSE").split(",")
                        if item.strip()
                    ),
                ),
                routing=RoutingConfig(venues=frozenset(INDIA_VENUE_CODES)),
            ),
        },
    )
    node = TradingNode(config=config)
    node.add_data_client_factory(CLIENT_NAME, IndiaClickHouseDataClientFactory)
    node.build()
    return node


if __name__ == "__main__":
    node = build_node()
    if os.environ.get("RUN_NODE") == "1":
        node.run()
    else:
        print(f"Built {CLIENT_NAME} data client. Set RUN_NODE=1 to connect to ClickHouse.")
        node.dispose()
