"""Live market-data client that reads Indian listings from ClickHouse.

Historical requests return the stored bars, trades, and quotes. Subscriptions
poll for rows newer than the subscribe time. A broker websocket feeder is a
separate process that writes those rows. This client does not place orders.
"""

from __future__ import annotations

import asyncio

from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.config import LiveDataClientConfig
from nautilus_trader.data.messages import RequestBars
from nautilus_trader.data.messages import RequestInstrument
from nautilus_trader.data.messages import RequestInstruments
from nautilus_trader.data.messages import RequestQuoteTicks
from nautilus_trader.data.messages import RequestTradeTicks
from nautilus_trader.data.messages import SubscribeBars
from nautilus_trader.data.messages import SubscribeQuoteTicks
from nautilus_trader.data.messages import SubscribeTradeTicks
from nautilus_trader.data.messages import UnsubscribeBars
from nautilus_trader.data.messages import UnsubscribeQuoteTicks
from nautilus_trader.data.messages import UnsubscribeTradeTicks
from nautilus_trader.live.data_client import LiveMarketDataClient
from nautilus_trader.live.factories import LiveDataClientFactory
from nautilus_trader.model.data import BarType
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Venue

from nautilus_india.adapt import bar_spec_key
from nautilus_india.adapt import require_external_last_bar
from nautilus_india.adapt import to_bar
from nautilus_india.adapt import to_quote
from nautilus_india.adapt import to_trade
from nautilus_india.clickhouse.store import ClickHouseMarketStore
from nautilus_india.config import ClickHouseConfig
from nautilus_india.provider import IndiaInstrumentProvider
from nautilus_india.timeutil import datetime_to_unix_ns
from nautilus_india.venues import INDIA_VENUE_CODES

CLIENT_NAME = "INDIA"


class IndiaDataClientConfig(LiveDataClientConfig, frozen=True):
    """ClickHouse connection fields plus the standard live data-client fields."""

    host: str = "localhost"
    port: int = 8123
    username: str = "default"
    password: str = ""
    database: str = "nautilus_india"
    secure: bool = False
    poll_interval_secs: float = 1.0

    def clickhouse(self) -> ClickHouseConfig:
        return ClickHouseConfig(
            host=self.host,
            port=self.port,
            username=self.username,
            password=self.password,
            database=self.database,
            secure=self.secure,
        )


class IndiaClickHouseDataClient(LiveMarketDataClient):
    """Multi-venue data client. Routing is by instrument venue, not by client venue."""

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client_id: ClientId,
        venue: Venue | None,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider: IndiaInstrumentProvider,
        config: IndiaDataClientConfig,
        store: ClickHouseMarketStore,
    ) -> None:
        super().__init__(
            loop=loop,
            client_id=client_id,
            venue=venue,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
            config=config,
        )
        self._settings = config
        self._store = store
        self._bar_types: dict[str, BarType] = {}
        self._bar_cursors: dict[str, int] = {}
        self._trade_cursors: dict[str, int] = {}
        self._quote_cursors: dict[str, int] = {}

    async def _connect(self) -> None:
        await asyncio.to_thread(self._store.connect)
        await self._instrument_provider.initialize()
        for instrument in self._instrument_provider.list_all():
            self._handle_data(instrument)
        self.create_task(self._poll(), log_msg="clickhouse poll")

    async def _disconnect(self) -> None:
        await asyncio.to_thread(self._store.close)

    async def _subscribe_bars(self, command: SubscribeBars) -> None:
        require_external_last_bar(command.bar_type)
        key = str(command.bar_type)
        self._bar_types[key] = command.bar_type
        self._bar_cursors[key] = self._clock.timestamp_ns()

    async def _unsubscribe_bars(self, command: UnsubscribeBars) -> None:
        key = str(command.bar_type)
        self._bar_types.pop(key, None)
        self._bar_cursors.pop(key, None)

    async def _subscribe_trade_ticks(self, command: SubscribeTradeTicks) -> None:
        self._trade_cursors[command.instrument_id.value] = self._clock.timestamp_ns()

    async def _unsubscribe_trade_ticks(self, command: UnsubscribeTradeTicks) -> None:
        self._trade_cursors.pop(command.instrument_id.value, None)

    async def _subscribe_quote_ticks(self, command: SubscribeQuoteTicks) -> None:
        self._quote_cursors[command.instrument_id.value] = self._clock.timestamp_ns()

    async def _unsubscribe_quote_ticks(self, command: UnsubscribeQuoteTicks) -> None:
        self._quote_cursors.pop(command.instrument_id.value, None)

    async def _request_instrument(self, request: RequestInstrument) -> None:
        instrument = await self._load_instrument(request.instrument_id)
        if instrument is None:
            self._log.error(f"Cannot find instrument for {request.instrument_id}")
            return
        self._handle_instrument(
            instrument,
            request.id,
            request.start,
            request.end,
            request.params,
        )

    async def _request_instruments(self, request: RequestInstruments) -> None:
        filters = dict(getattr(self._instrument_provider, "_filters", None) or {})
        if request.venue is not None:
            filters["venue"] = request.venue.value
        await self._instrument_provider.load_all_async(filters or None)
        instruments = self._instrument_provider.list_all()
        venue = request.venue or Venue(INDIA_VENUE_CODES[0])
        self._handle_instruments(
            venue,
            instruments,
            request.id,
            request.start,
            request.end,
            request.params,
        )

    async def _request_bars(self, request: RequestBars) -> None:
        require_external_last_bar(request.bar_type)
        instrument = await self._load_instrument(request.bar_type.instrument_id)
        bars = []
        if instrument is None:
            self._log.error(f"Cannot find instrument for {request.bar_type.instrument_id}")
        else:
            records = await asyncio.to_thread(
                self._store.fetch_bars,
                instrument_id=request.bar_type.instrument_id.value,
                bar_spec=bar_spec_key(request.bar_type),
                start_ns=_unix_ns(request.start),
                end_ns=_unix_ns(request.end),
                limit=_limit(request.limit),
            )
            bars = [
                to_bar(record, request.bar_type, instrument.price_precision)
                for record in records
            ]
        self._handle_bars(
            request.bar_type,
            bars,
            request.id,
            request.start,
            request.end,
            request.params,
        )

    async def _request_trade_ticks(self, request: RequestTradeTicks) -> None:
        instrument = await self._load_instrument(request.instrument_id)
        trades = []
        if instrument is None:
            self._log.error(f"Cannot find instrument for {request.instrument_id}")
        else:
            records = await asyncio.to_thread(
                self._store.fetch_trades,
                instrument_id=request.instrument_id.value,
                start_ns=_unix_ns(request.start),
                end_ns=_unix_ns(request.end),
                limit=_limit(request.limit),
            )
            trades = [to_trade(record, instrument.price_precision) for record in records]
        self._handle_trade_ticks(
            request.instrument_id,
            trades,
            request.id,
            request.start,
            request.end,
            request.params,
        )

    async def _request_quote_ticks(self, request: RequestQuoteTicks) -> None:
        instrument = await self._load_instrument(request.instrument_id)
        quotes = []
        if instrument is None:
            self._log.error(f"Cannot find instrument for {request.instrument_id}")
        else:
            records = await asyncio.to_thread(
                self._store.fetch_quotes,
                instrument_id=request.instrument_id.value,
                start_ns=_unix_ns(request.start),
                end_ns=_unix_ns(request.end),
                limit=_limit(request.limit),
            )
            quotes = [to_quote(record, instrument.price_precision) for record in records]
        self._handle_quote_ticks(
            request.instrument_id,
            quotes,
            request.id,
            request.start,
            request.end,
            request.params,
        )

    async def _poll(self) -> None:
        while True:
            await asyncio.sleep(self._settings.poll_interval_secs)
            await self._poll_bars()
            await self._poll_trades()
            await self._poll_quotes()

    async def _poll_bars(self) -> None:
        for key, cursor in list(self._bar_cursors.items()):
            bar_type = self._bar_types[key]
            instrument = self._instrument_provider.find(bar_type.instrument_id)
            if instrument is None:
                self._log.error(f"Skipping {bar_type}: instrument is not loaded")
                continue
            records = await asyncio.to_thread(
                self._store.fetch_bars,
                instrument_id=bar_type.instrument_id.value,
                bar_spec=bar_spec_key(bar_type),
                start_ns=cursor + 1,
            )
            for record in records:
                self._handle_data(to_bar(record, bar_type, instrument.price_precision))
                self._bar_cursors[key] = record.ts_event

    async def _poll_trades(self) -> None:
        for instrument_id, cursor in list(self._trade_cursors.items()):
            instrument = self._instrument_provider.find(InstrumentId.from_str(instrument_id))
            if instrument is None:
                continue
            records = await asyncio.to_thread(
                self._store.fetch_trades,
                instrument_id=instrument_id,
                start_ns=cursor + 1,
            )
            for record in records:
                self._handle_data(to_trade(record, instrument.price_precision))
                self._trade_cursors[instrument_id] = record.ts_event

    async def _poll_quotes(self) -> None:
        for instrument_id, cursor in list(self._quote_cursors.items()):
            instrument = self._instrument_provider.find(InstrumentId.from_str(instrument_id))
            if instrument is None:
                continue
            records = await asyncio.to_thread(
                self._store.fetch_quotes,
                instrument_id=instrument_id,
                start_ns=cursor + 1,
            )
            for record in records:
                self._handle_data(to_quote(record, instrument.price_precision))
                self._quote_cursors[instrument_id] = record.ts_event

    async def _load_instrument(self, instrument_id: InstrumentId):
        instrument = self._instrument_provider.find(instrument_id)
        if instrument is not None:
            return instrument
        await self._instrument_provider.load_async(instrument_id)
        instrument = self._instrument_provider.find(instrument_id)
        if instrument is not None:
            self._handle_data(instrument)
        return instrument


class IndiaClickHouseDataClientFactory(LiveDataClientFactory):
    """Factory registered on ``TradingNode`` under :data:`CLIENT_NAME`."""

    @staticmethod
    def create(
        loop: asyncio.AbstractEventLoop,
        name: str,
        config: LiveDataClientConfig,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
    ) -> IndiaClickHouseDataClient:
        settings = config if isinstance(config, IndiaDataClientConfig) else IndiaDataClientConfig()
        store = ClickHouseMarketStore(settings.clickhouse())
        provider = IndiaInstrumentProvider(store, config=settings.instrument_provider)
        return IndiaClickHouseDataClient(
            loop=loop,
            client_id=ClientId(name),
            venue=None,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=provider,
            config=settings,
            store=store,
        )


def _unix_ns(value) -> int | None:
    if value is None:
        return None
    return datetime_to_unix_ns(value)


def _limit(value: int) -> int | None:
    if value is None or value <= 0:
        return None
    return value
