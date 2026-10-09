from nautilus_trader.config import RoutingConfig
from nautilus_trader.config import TradingNodeConfig
from nautilus_trader.live.node import TradingNode
from nautilus_trader.model.identifiers import ClientId

from nautilus_india.data_client import IndiaClickHouseDataClient
from nautilus_india.data_client import IndiaClickHouseDataClientFactory
from nautilus_india.data_client import IndiaDataClientConfig
from nautilus_india.venues import INDIA_VENUE_CODES


def test_trading_node_builds_the_clickhouse_client_without_connecting():
    config = TradingNodeConfig(
        trader_id="INDIA-001",
        data_clients={
            "INDIA": IndiaDataClientConfig(
                host="clickhouse.internal",
                database="market_lab",
                routing=RoutingConfig(venues=frozenset(INDIA_VENUE_CODES)),
            ),
        },
    )
    node = TradingNode(config=config)
    node.add_data_client_factory("INDIA", IndiaClickHouseDataClientFactory)

    node.build()

    client = node.kernel.data_engine._clients[ClientId("INDIA")]
    assert isinstance(client, IndiaClickHouseDataClient)
    assert client._store.config.host == "clickhouse.internal"
    assert client._store.config.database == "market_lab"
    assert not client._store.connected
    node.dispose()
