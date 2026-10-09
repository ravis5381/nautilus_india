"""Default Indian market sessions, in Asia/Kolkata local time.

These windows are the fallback used when ClickHouse has no overriding row in
``sessions``. Cash and equity-derivative hours have changed by circular, so a
historical study should load the ``sessions`` table for the date being
replayed. MCX hours depend on the commodity and are not given a default
continuous window.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import date
from datetime import datetime
from datetime import time
from datetime import timezone
from zoneinfo import ZoneInfo

from nautilus_india.venues import VENUES

IST = ZoneInfo("Asia/Kolkata")


@dataclass(frozen=True, slots=True)
class SessionWindow:
    """Inclusive local-time window. A 15:30 close belongs to the session."""

    name: str
    open: time
    close: time

    def contains(self, local: time) -> bool:
        return self.open <= local <= self.close


@dataclass(frozen=True, slots=True)
class VenueSchedule:
    code: str
    continuous: SessionWindow | None
    auctions: tuple[SessionWindow, ...] = ()


_PRE_OPEN = SessionWindow("pre_open", time(9, 0), time(9, 8))
_CASH_CLOSE = SessionWindow("closing", time(15, 40), time(16, 0))

SCHEDULES: dict[str, VenueSchedule] = {
    "NSE": VenueSchedule(
        "NSE",
        SessionWindow("continuous", time(9, 15), time(15, 30)),
        (_PRE_OPEN, _CASH_CLOSE),
    ),
    "BSE": VenueSchedule(
        "BSE",
        SessionWindow("continuous", time(9, 15), time(15, 30)),
        (_PRE_OPEN, _CASH_CLOSE),
    ),
    "NFO": VenueSchedule(
        "NFO",
        SessionWindow("continuous", time(9, 15), time(15, 40)),
        (_PRE_OPEN,),
    ),
    "BFO": VenueSchedule(
        "BFO",
        SessionWindow("continuous", time(9, 15), time(15, 40)),
        (_PRE_OPEN,),
    ),
    "CDS": VenueSchedule(
        "CDS",
        SessionWindow("continuous", time(9, 0), time(17, 0)),
    ),
    "MCX": VenueSchedule("MCX", None),
}


def schedule_for(venue: str) -> VenueSchedule:
    try:
        return SCHEDULES[venue]
    except KeyError as exc:
        known = ", ".join(sorted(VENUES))
        raise ValueError(f"Unknown venue {venue!r}. Known venues: {known}") from exc


def in_continuous_session(venue: str, moment: datetime) -> bool:
    """Return whether ``moment`` falls inside the venue's continuous session."""
    window = schedule_for(venue).continuous
    if window is None:
        return False
    local = _as_ist(moment)
    if not is_weekday(local.date()):
        return False
    return window.contains(local.timetz().replace(tzinfo=None))


def is_weekday(day: date) -> bool:
    return day.weekday() < 5


def is_trading_day(venue: str, day: date, holidays: set[date] | None = None) -> bool:
    """Weekday that is not present in the venue holiday set.

    ``holidays`` is the exchange holiday calendar. MCX uses the same weekday
    rule; product-specific evening sessions still come from ``sessions``.
    """
    schedule_for(venue)
    if not is_weekday(day):
        return False
    return day not in (holidays or set())


def continuous_bounds(venue: str, session_date: date) -> tuple[datetime, datetime]:
    """Inclusive continuous-session bounds as timezone-aware UTC datetimes."""
    window = schedule_for(venue).continuous
    if window is None:
        raise ValueError(
            f"{venue} has no default continuous session. "
            "Load product hours from the ClickHouse sessions table.",
        )
    start = datetime.combine(session_date, window.open, tzinfo=IST)
    end = datetime.combine(session_date, window.close, tzinfo=IST)
    return start.astimezone(timezone.utc), end.astimezone(timezone.utc)


def _as_ist(moment: datetime) -> datetime:
    if moment.tzinfo is None:
        raise ValueError("Session checks require a timezone-aware datetime")
    return moment.astimezone(IST)
