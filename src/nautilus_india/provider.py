"""Load Indian instrument definitions from ClickHouse into Nautilus."""

from __future__ import annotations

import asyncio
from typing import Any

from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.model.currencies import INR
from nautilus_trader.model.identifiers import InstrumentId

from nautilus_india.adapt import to_instrument
from nautilus_india.clickhouse.store import ClickHouseMarketStore


class IndiaInstrumentProvider(InstrumentProvider):
    """Instrument provider backed by the ``instruments`` table.

    ``load_ids`` queries those ids directly. Loading every F&O contract is
    opt-in through ``load_all`` because the contract universe is large.
    """

    def __init__(
        self,
        store: ClickHouseMarketStore,
        config: InstrumentProviderConfig | None = None,
    ) -> None:
        super().__init__(config=config)
        self._store = store
        self.add_currency(INR)

    async def load_all_async(self, filters: dict | None = None) -> None:
        records = await asyncio.to_thread(
            self._store.fetch_instruments,
            **_filter_kwargs(filters),
        )
        self._install(records)

    async def load_ids_async(
        self,
        instrument_ids: list[InstrumentId],
        filters: dict | None = None,
    ) -> None:
        if not instrument_ids:
            return
        kwargs = _filter_kwargs(filters)
        kwargs["instrument_ids"] = [instrument_id.value for instrument_id in instrument_ids]
        records = await asyncio.to_thread(self._store.fetch_instruments, **kwargs)
        self._install(records)

    def _install(self, records) -> None:
        for record in records:
            self.add(to_instrument(record))


def _filter_kwargs(filters: dict | None) -> dict[str, Any]:
    raw = dict(filters or {})
    return {
        "venues": _as_list(raw.get("venue")),
        "products": _as_list(raw.get("product")),
        "underlyings": _as_list(raw.get("underlying")),
    }


def _as_list(value: Any) -> list[str] | None:
    if value is None:
        return None
    if isinstance(value, str):
        return [value]
    return [str(item) for item in value]
