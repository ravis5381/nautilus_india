from datetime import date
from datetime import datetime
from datetime import timezone

import pytest

from nautilus_india.sessions import IST
from nautilus_india.sessions import continuous_bounds
from nautilus_india.sessions import in_continuous_session
from nautilus_india.sessions import is_trading_day


def _ist(hour, minute, day=9):
    return datetime(2026, 10, day, hour, minute, tzinfo=IST)


def test_nse_cash_session_includes_the_official_close():
    assert in_continuous_session("NSE", _ist(9, 15))
    assert not in_continuous_session("NSE", _ist(9, 14))
    assert in_continuous_session("NSE", _ist(15, 30))
    assert not in_continuous_session("NSE", _ist(15, 31))


def test_equity_derivatives_stay_open_until_1540():
    assert in_continuous_session("NFO", _ist(15, 35))
    assert not in_continuous_session("NSE", _ist(15, 35))
    assert in_continuous_session("NFO", _ist(15, 40))


def test_weekends_and_holidays_are_closed():
    saturday = datetime(2026, 10, 10, 10, 0, tzinfo=IST)
    assert not in_continuous_session("NSE", saturday)
    assert not is_trading_day("BSE", date(2026, 10, 10))
    assert not is_trading_day("NSE", date(2026, 10, 9), holidays={date(2026, 10, 9)})
    assert is_trading_day("NSE", date(2026, 10, 9))


def test_session_bounds_are_utc():
    start, end = continuous_bounds("NSE", date(2026, 10, 9))

    assert start == datetime(2026, 10, 9, 3, 45, tzinfo=timezone.utc)
    assert end == datetime(2026, 10, 9, 10, 0, tzinfo=timezone.utc)


def test_mcx_has_no_default_continuous_window():
    assert not in_continuous_session("MCX", _ist(12, 0))
    with pytest.raises(ValueError, match="sessions table"):
        continuous_bounds("MCX", date(2026, 10, 9))


def test_naive_datetimes_are_rejected():
    with pytest.raises(ValueError, match="timezone-aware"):
        in_continuous_session("NSE", datetime(2026, 10, 9, 9, 15))
