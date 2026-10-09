"""UTC nanosecond conversions.

Python ``datetime`` resolves to microseconds. ClickHouse stores Nautilus
timestamps as ``Int64`` UNIX nanoseconds, so values that originate in Python
are microsecond-aligned.
"""

from __future__ import annotations

from datetime import datetime
from datetime import timezone

_NS = 1_000_000_000


def ensure_utc(value: datetime) -> datetime:
    if value.tzinfo is None:
        return value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


def datetime_to_unix_ns(value: datetime) -> int:
    """Convert a datetime to UNIX nanoseconds."""
    utc = ensure_utc(value)
    return int(utc.timestamp()) * _NS + utc.microsecond * 1_000


def unix_ns_to_datetime(timestamp_ns: int) -> datetime:
    """Convert UNIX nanoseconds to a timezone-aware UTC datetime."""
    seconds, remainder = divmod(int(timestamp_ns), _NS)
    return datetime.fromtimestamp(seconds, tz=timezone.utc).replace(
        microsecond=remainder // 1_000,
    )
