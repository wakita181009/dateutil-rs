"""Compatibility tests: compare python-dateutil vs dateutil_rs output directly.

Both libraries are imported side-by-side so we can verify that dateutil_rs
produces identical results to the reference python-dateutil implementation.

Run:
    uv run pytest tests/test_compat.py -v
"""

import os
from datetime import date, datetime, timedelta

import pytest

# Reference (python-dateutil)
import dateutil.easter
from dateutil.relativedelta import FR, MO, SA, SU, TU, WE
from dateutil.relativedelta import relativedelta as py_relativedelta
from dateutil.utils import within_delta as py_within_delta

# Rust implementation
dateutil_rs = pytest.importorskip("dateutil_rs", exc_type=ImportError)
from dateutil_rs.easter import easter as rs_easter
from dateutil_rs.relativedelta import FR as RS_FR
from dateutil_rs.relativedelta import MO as RS_MO
from dateutil_rs.relativedelta import SA as RS_SA
from dateutil_rs.relativedelta import SU as RS_SU
from dateutil_rs.relativedelta import TU as RS_TU
from dateutil_rs.relativedelta import WE as RS_WE
from dateutil_rs.relativedelta import relativedelta as rs_relativedelta
from dateutil_rs.utils import within_delta as rs_within_delta


# ---------------------------------------------------------------------------
# Easter
# ---------------------------------------------------------------------------
class TestEasterCompat:
    @pytest.mark.parametrize("year", range(1, 300))
    def test_julian_early_years(self, year):
        py_result = dateutil.easter.easter(year, 1)
        rs_result = rs_easter(year, 1)
        assert py_result == rs_result, f"Julian easter mismatch for year {year}"

    @pytest.mark.parametrize("year", range(1583, 2500))
    def test_western_wide_range(self, year):
        py_result = dateutil.easter.easter(year, 3)
        rs_result = rs_easter(year, 3)
        assert py_result == rs_result, f"Western easter mismatch for year {year}"

    @pytest.mark.parametrize("year", range(1583, 2500))
    def test_orthodox_wide_range(self, year):
        py_result = dateutil.easter.easter(year, 2)
        rs_result = rs_easter(year, 2)
        assert py_result == rs_result, f"Orthodox easter mismatch for year {year}"

    def test_invalid_method_both_raise(self):
        with pytest.raises(ValueError):
            dateutil.easter.easter(2024, 4)
        with pytest.raises(ValueError):
            rs_easter(2024, 4)

    def test_invalid_year_both_raise(self):
        with pytest.raises(ValueError):
            dateutil.easter.easter(0)
        with pytest.raises(ValueError):
            rs_easter(0)


# ---------------------------------------------------------------------------
# RelativeDelta — addition to dates/datetimes
# ---------------------------------------------------------------------------
class TestRelativeDeltaAddCompat:
    now = datetime(2003, 9, 17, 20, 54, 47, 282310)
    today = date(2003, 9, 17)

    def _assert_same(self, base, py_kwargs, rs_kwargs=None):
        """Apply relativedelta with same kwargs to both impls, compare results."""
        if rs_kwargs is None:
            rs_kwargs = py_kwargs
        py_result = base + py_relativedelta(**py_kwargs)
        rs_result = base + rs_relativedelta(**rs_kwargs)
        assert py_result == rs_result, f"Mismatch: py={py_result}, rs={rs_result}"

    def test_next_month(self):
        self._assert_same(self.now, dict(months=1))

    def test_next_month_plus_one_week(self):
        self._assert_same(self.now, dict(months=1, weeks=1))

    def test_next_month_plus_one_week_10am(self):
        self._assert_same(self.today, dict(months=1, weeks=1, hour=10))

    def test_one_year_minus_one_month(self):
        self._assert_same(self.now, dict(years=1, months=-1))

    @pytest.mark.parametrize(
        "base,months",
        [
            (date(2003, 1, 27), 1),
            (date(2003, 1, 31), 1),
            (date(2003, 1, 31), 2),
        ],
    )
    def test_month_clamping(self, base, months):
        self._assert_same(base, dict(months=months))

    @pytest.mark.parametrize(
        "base,years",
        [
            (date(2000, 2, 28), 1),
            (date(2000, 2, 29), 1),
            (date(1999, 2, 28), 1),
            (date(2001, 2, 28), -1),
        ],
    )
    def test_year_leap_boundary(self, base, years):
        self._assert_same(base, dict(years=years))

    def test_next_friday(self):
        py_result = self.today + py_relativedelta(weekday=FR)
        rs_result = self.today + rs_relativedelta(weekday=RS_FR)
        assert py_result == rs_result

    def test_last_friday_in_month(self):
        py_result = self.today + py_relativedelta(day=31, weekday=FR(-1))
        rs_result = self.today + rs_relativedelta(day=31, weekday=RS_FR(-1))
        assert py_result == rs_result

    def test_next_wednesday_is_today(self):
        py_result = self.today + py_relativedelta(weekday=WE)
        rs_result = self.today + rs_relativedelta(weekday=RS_WE)
        assert py_result == rs_result

    def test_add_more_than_12_months(self):
        self._assert_same(date(2003, 12, 1), dict(months=13))

    def test_add_negative_months(self):
        self._assert_same(date(2003, 1, 1), dict(months=-2))

    def test_last_day_of_february(self):
        self._assert_same(date(2021, 2, 1), dict(day=31))

    def test_last_day_of_february_leap(self):
        self._assert_same(date(2020, 2, 1), dict(day=31))

    def test_absolute_fields(self):
        self._assert_same(datetime(2024, 1, 1), dict(year=2025, month=6, day=15))

    def test_yearday(self):
        self._assert_same(date(2003, 1, 1), dict(yearday=260))
        self._assert_same(date(2000, 1, 1), dict(yearday=260))

    def test_nlyearday(self):
        self._assert_same(date(2003, 1, 1), dict(nlyearday=260))
        self._assert_same(date(2000, 1, 1), dict(nlyearday=260))

    def test_subtract(self):
        py_result = datetime(2024, 3, 15) - py_relativedelta(months=2)
        rs_result = datetime(2024, 3, 15) - rs_relativedelta(months=2)
        assert py_result == rs_result


# ---------------------------------------------------------------------------
# RelativeDelta — diff between two datetimes
# ---------------------------------------------------------------------------
class TestRelativeDeltaDiffCompat:
    now = datetime(2003, 9, 17, 20, 54, 47, 282310)

    def _compare_diff(self, dt1, dt2):
        """Compare relativedelta diff results field-by-field."""
        py_rd = py_relativedelta(dt1, dt2)
        rs_rd = rs_relativedelta(dt1=dt1, dt2=dt2)
        assert py_rd.years == rs_rd.years, f"years: py={py_rd.years}, rs={rs_rd.years}"
        assert py_rd.months == rs_rd.months, (
            f"months: py={py_rd.months}, rs={rs_rd.months}"
        )
        assert py_rd.days == rs_rd.days, f"days: py={py_rd.days}, rs={rs_rd.days}"
        assert py_rd.hours == rs_rd.hours, f"hours: py={py_rd.hours}, rs={rs_rd.hours}"
        assert py_rd.minutes == rs_rd.minutes, (
            f"minutes: py={py_rd.minutes}, rs={rs_rd.minutes}"
        )
        assert py_rd.seconds == rs_rd.seconds, (
            f"seconds: py={py_rd.seconds}, rs={rs_rd.seconds}"
        )
        assert py_rd.microseconds == rs_rd.microseconds, (
            f"microseconds: py={py_rd.microseconds}, rs={rs_rd.microseconds}"
        )

    def test_millennium_age(self):
        self._compare_diff(self.now, date(2001, 1, 1))

    def test_john_age(self):
        self._compare_diff(self.now, datetime(1978, 4, 5, 12, 0))

    def test_month_end_to_beginning(self):
        self._compare_diff(
            datetime(2003, 1, 31, 23, 59, 59), datetime(2003, 3, 1, 0, 0, 0)
        )

    def test_beginning_to_month_end(self):
        self._compare_diff(
            datetime(2003, 3, 1, 0, 0, 0), datetime(2003, 1, 31, 23, 59, 59)
        )

    def test_leap_year_diff(self):
        self._compare_diff(
            datetime(2012, 1, 31, 23, 59, 59), datetime(2012, 3, 1, 0, 0, 0)
        )

    @pytest.mark.parametrize(
        "dt1,dt2",
        [
            (datetime(2024, 6, 15), datetime(2024, 1, 1)),
            (datetime(2025, 1, 1), datetime(2024, 1, 1)),
            (datetime(2024, 3, 1), datetime(2024, 2, 29)),
            (datetime(2024, 1, 1), datetime(2023, 12, 31, 23, 59, 59)),
        ],
    )
    def test_various_diffs(self, dt1, dt2):
        self._compare_diff(dt1, dt2)
        self._compare_diff(dt2, dt1)  # reverse direction too


# ---------------------------------------------------------------------------
# RelativeDelta — arithmetic (add, sub, mul, neg)
# ---------------------------------------------------------------------------
class TestRelativeDeltaArithmeticCompat:
    def test_add_two_deltas(self):
        py_result = py_relativedelta(days=10) + py_relativedelta(
            years=1, months=2, days=3
        )
        rs_result = rs_relativedelta(days=10) + rs_relativedelta(
            years=1, months=2, days=3
        )
        assert py_result.years == rs_result.years
        assert py_result.months == rs_result.months
        assert py_result.days == rs_result.days

    def test_multiply(self):
        py_result = py_relativedelta(months=1, days=5) * 3
        rs_result = rs_relativedelta(months=1, days=5) * 3
        assert py_result.months == rs_result.months
        assert py_result.days == rs_result.days

    def test_negation(self):
        py_result = -py_relativedelta(months=1, days=5)
        rs_result = -rs_relativedelta(months=1, days=5)
        assert py_result.months == rs_result.months
        assert py_result.days == rs_result.days

    def test_bool_empty(self):
        assert not py_relativedelta()
        assert not rs_relativedelta()

    def test_bool_nonzero(self):
        assert bool(py_relativedelta(months=1))
        assert bool(rs_relativedelta(months=1))


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------
try:
    from dateutil.parser import parse as py_parse
    from dateutil_rs.parser import parse as rs_parse

    HAS_PARSER = True
except ImportError:
    HAS_PARSER = False

PARSER_TEST_STRINGS = [
    "Thu Sep 25 10:36:28 2003",
    "Thu Sep 25 2003",
    "2003-09-25T10:49:41",
    "2003-09-25T10:49",
    "2003-09-25T10",
    "2003-09-25",
    "20030925T104941",
    "20030925",
    "09-25-2003",
    "25-09-2003",
    "2003.09.25",
    "09.25.2003",
    "25.09.2003",
    "2003/09/25",
    "09/25/2003",
    "25/09/2003",
    "2003 09 25",
    "09 25 2003",
    "25 09 2003",
    "July 4, 1976",
    "4 jul 1976",
    "7-4-76",
    "19760704",
    "0:01:02 on July 4, 1976",
    "July 4, 1976 12:01:02 am",
    "Mon Jan  2 04:24:27 1995",
    "Jan 1 1999 11:23:34.578",
    "3rd of May 2001",
    "5th of March 2001",
    "1st of May 2003",
    "13NOV2017",
    "  July   4 ,  1976   12:01:02   am  ",
    "1996.July.10 AD 12:08 PM",
    "Wed, July 10, '96",
]


@pytest.mark.skipif(not HAS_PARSER, reason="dateutil_rs.parser not available")
class TestParserCompat:
    @pytest.mark.parametrize("timestr", PARSER_TEST_STRINGS)
    def test_parse_default(self, timestr):
        py_result = py_parse(timestr)
        rs_result = rs_parse(timestr)
        assert py_result == rs_result, (
            f"Parse mismatch for {timestr!r}: py={py_result}, rs={rs_result}"
        )

    @pytest.mark.parametrize(
        "timestr",
        [
            "10-09-2003",
            "10.09.2003",
            "10/09/2003",
            "10 09 2003",
        ],
    )
    def test_dayfirst(self, timestr):
        py_result = py_parse(timestr, dayfirst=True)
        rs_result = rs_parse(timestr, dayfirst=True)
        assert py_result == rs_result, (
            f"dayfirst mismatch for {timestr!r}: py={py_result}, rs={rs_result}"
        )

    @pytest.mark.parametrize(
        "timestr",
        [
            "10-09-03",
            "10.09.03",
            "10/09/03",
        ],
    )
    def test_yearfirst(self, timestr):
        py_result = py_parse(timestr, yearfirst=True)
        rs_result = rs_parse(timestr, yearfirst=True)
        assert py_result == rs_result, (
            f"yearfirst mismatch for {timestr!r}: py={py_result}, rs={rs_result}"
        )

    def test_parse_with_default(self):
        default = datetime(2003, 9, 25)
        for timestr in ["10:36:28", "10:36", "Sep 2003", "Sep", "2003"]:
            py_result = py_parse(timestr, default=default)
            rs_result = rs_parse(timestr, default=default)
            assert py_result == rs_result, (
                f"default mismatch for {timestr!r}: py={py_result}, rs={rs_result}"
            )

    def test_fuzzy_parse(self):
        timestr = "Today is 25 of September of 2003, exactly at 10:49:41 with timezone"
        py_result = py_parse(timestr, fuzzy=True)
        rs_result = rs_parse(timestr, fuzzy=True)
        assert py_result == rs_result


# ---------------------------------------------------------------------------
# Utils — within_delta
# ---------------------------------------------------------------------------
class TestWithinDeltaCompat:
    @pytest.mark.parametrize(
        "d1,d2,delta,expected",
        [
            (
                datetime(2016, 1, 1, 12, 14, 1, 9),
                datetime(2016, 1, 1, 12, 14, 1, 15),
                timedelta(seconds=1),
                True,
            ),
            (
                datetime(2016, 1, 1, 12, 14, 1, 9),
                datetime(2016, 1, 1, 12, 14, 1, 15),
                timedelta(microseconds=1),
                False,
            ),
            (
                datetime(2016, 1, 1),
                datetime(2015, 12, 31),
                timedelta(days=-1),
                True,
            ),
            (
                datetime(2024, 1, 1),
                datetime(2024, 1, 1),
                timedelta(seconds=0),
                True,
            ),
            (
                datetime(2024, 1, 1, 0, 0, 0),
                datetime(2024, 1, 1, 0, 0, 1),
                timedelta(milliseconds=500),
                False,
            ),
        ],
    )
    def test_within_delta(self, d1, d2, delta, expected):
        py_result = py_within_delta(d1, d2, delta)
        rs_result = rs_within_delta(d1, d2, delta)
        assert py_result == rs_result == expected


# ---------------------------------------------------------------------------
# Timezone
# ---------------------------------------------------------------------------
try:
    from dateutil_rs._native import _TzOffset

    from dateutil.tz import (
        datetime_ambiguous as py_datetime_ambiguous,
    )
    from dateutil.tz import (
        datetime_exists as py_datetime_exists,
    )
    from dateutil.tz import (
        gettz as py_gettz,
    )
    from dateutil.tz import (
        resolve_imaginary as py_resolve_imaginary,
    )
    from dateutil.tz import (
        tzfile as py_tzfile,
    )
    from dateutil.tz import (
        tzlocal as py_tzlocal,
    )
    from dateutil.tz import (
        tzoffset as py_tzoffset,
    )
    from dateutil.tz import (
        tzstr as py_tzstr,
    )
    from dateutil.tz import (
        tzutc as py_tzutc,
    )
    from dateutil_rs.tz import (
        datetime_ambiguous as rs_datetime_ambiguous,
    )
    from dateutil_rs.tz import (
        datetime_exists as rs_datetime_exists,
    )
    from dateutil_rs.tz import (
        gettz as rs_gettz,
    )
    from dateutil_rs.tz import (
        resolve_imaginary as rs_resolve_imaginary,
    )
    from dateutil_rs.tz import (
        tzfile as rs_tzfile,
    )
    from dateutil_rs.tz import (
        tzlocal as rs_tzlocal,
    )
    from dateutil_rs.tz import (
        tzoffset as rs_tzoffset,
    )
    from dateutil_rs.tz import (
        tzstr as rs_tzstr,
    )
    from dateutil_rs.tz import (
        tzutc as rs_tzutc,
    )

    HAS_TZ = True
except ImportError:
    HAS_TZ = False

# Find a usable zoneinfo directory
_ZONEINFO_DIRS = [
    "/usr/share/zoneinfo",
    "/usr/lib/zoneinfo",
    "/usr/share/lib/zoneinfo",
    "/etc/zoneinfo",
]
ZONEINFO_DIR = next((d for d in _ZONEINFO_DIRS if os.path.isdir(d)), None)


def _has_tzfile(name):
    """Check if a named timezone file exists on this system."""
    if ZONEINFO_DIR is None:
        return False
    return os.path.isfile(os.path.join(ZONEINFO_DIR, name))


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
class TestTzUtcCompat:
    def test_utcoffset(self):
        dt = datetime(2024, 6, 15, 12, 0)
        assert py_tzutc().utcoffset(dt) == rs_tzutc().utcoffset(dt)

    def test_dst(self):
        dt = datetime(2024, 6, 15, 12, 0)
        assert py_tzutc().dst(dt) == rs_tzutc().dst(dt)

    def test_tzname(self):
        dt = datetime(2024, 6, 15, 12, 0)
        assert py_tzutc().tzname(dt) == rs_tzutc().tzname(dt)

    def test_is_ambiguous(self):
        dt = datetime(2024, 6, 15, 12, 0)
        assert py_tzutc().is_ambiguous(dt) is rs_tzutc().is_ambiguous(dt) is False

    def test_utcoffset_is_zero(self):
        dt = datetime(2024, 1, 1)
        assert rs_tzutc().utcoffset(dt) == timedelta(0)

    def test_dst_is_zero(self):
        dt = datetime(2024, 1, 1)
        assert rs_tzutc().dst(dt) == timedelta(0)


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
class TestTzOffsetCompat:
    @pytest.mark.parametrize(
        "name,offset_secs",
        [
            ("EST", -18000),
            ("IST", 19800),
            (None, 0),
            ("Custom", 3600),
            ("Neg", -7200),
        ],
    )
    def test_utcoffset(self, name, offset_secs):
        dt = datetime(2024, 6, 15, 12, 0)
        py_tz = py_tzoffset(name, offset_secs)
        rs_tz = rs_tzoffset(name, offset_secs)
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)

    @pytest.mark.parametrize("offset_secs", [-18000, 0, 3600, 19800])
    def test_dst_always_zero(self, offset_secs):
        dt = datetime(2024, 6, 15, 12, 0)
        py_tz = py_tzoffset("X", offset_secs)
        rs_tz = rs_tzoffset("X", offset_secs)
        assert py_tz.dst(dt) == rs_tz.dst(dt) == timedelta(0)

    def test_tzname(self):
        dt = datetime(2024, 6, 15, 12, 0)
        py_tz = py_tzoffset("EST", -18000)
        rs_tz = rs_tzoffset("EST", -18000)
        assert py_tz.tzname(dt) == rs_tz.tzname(dt)

    def test_is_ambiguous(self):
        dt = datetime(2024, 6, 15, 12, 0)
        py_tz = py_tzoffset("EST", -18000)
        rs_tz = rs_tzoffset("EST", -18000)
        assert py_tz.is_ambiguous(dt) == rs_tz.is_ambiguous(dt) is False


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
class TestDurationToPydelta:
    """Boundary-value tests for Rust duration_to_pydelta via _TzOffset.utcoffset()."""

    DT = datetime(2024, 6, 15, 12, 0)

    @pytest.mark.parametrize(
        "offset_secs,expected_td",
        [
            (0, timedelta(0)),
            (1, timedelta(seconds=1)),
            (-1, timedelta(seconds=-1)),
            (86400, timedelta(days=1)),
            (-86400, timedelta(days=-1)),
            (86399, timedelta(seconds=86399)),
            (-86399, timedelta(days=-1, seconds=1)),
            (86401, timedelta(days=1, seconds=1)),
            (-86401, timedelta(days=-2, seconds=86399)),
            (-18000, timedelta(seconds=-18000)),  # EST
            (19800, timedelta(seconds=19800)),  # IST (+5:30)
            (32400, timedelta(seconds=32400)),  # JST (+9)
            (43200, timedelta(seconds=43200)),  # +12:00
            (-43200, timedelta(seconds=-43200)),  # -12:00
        ],
        ids=[
            "zero",
            "plus_1s",
            "minus_1s",
            "plus_1day",
            "minus_1day",
            "plus_1day_minus_1s",
            "minus_1day_plus_1s",
            "plus_1day_plus_1s",
            "minus_1day_minus_1s",
            "est_minus_5h",
            "ist_plus_5h30",
            "jst_plus_9h",
            "plus_12h",
            "minus_12h",
        ],
    )
    def test_utcoffset_boundary(self, offset_secs, expected_td):
        """_TzOffset.utcoffset() calls duration_to_pydelta; verify exact timedelta."""
        native_tz = _TzOffset(None, offset_secs)
        result = native_tz.utcoffset(self.DT)
        assert result == expected_td
        assert result.total_seconds() == offset_secs


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
@pytest.mark.skipif(
    not _has_tzfile("America/New_York"),
    reason="America/New_York tzfile not found",
)
class TestTzFileCompat:
    """Compare tzfile for America/New_York (EDT/EST transitions)."""

    @pytest.fixture
    def ny_py(self):
        return py_tzfile(os.path.join(ZONEINFO_DIR, "America/New_York"))

    @pytest.fixture
    def ny_rs(self):
        return rs_tzfile(os.path.join(ZONEINFO_DIR, "America/New_York"))

    # Summer (EDT: UTC-4)
    @pytest.mark.parametrize(
        "dt",
        [
            datetime(2024, 6, 15, 12, 0),
            datetime(2024, 7, 4, 0, 0),
            datetime(2024, 8, 1, 23, 59, 59),
        ],
    )
    def test_utcoffset_summer(self, dt, ny_py, ny_rs):
        assert ny_py.utcoffset(dt) == ny_rs.utcoffset(dt)

    # Winter (EST: UTC-5)
    @pytest.mark.parametrize(
        "dt",
        [
            datetime(2024, 1, 15, 12, 0),
            datetime(2024, 2, 1, 0, 0),
            datetime(2024, 12, 25, 18, 0),
        ],
    )
    def test_utcoffset_winter(self, dt, ny_py, ny_rs):
        assert ny_py.utcoffset(dt) == ny_rs.utcoffset(dt)

    @pytest.mark.parametrize(
        "dt",
        [
            datetime(2024, 6, 15, 12, 0),
            datetime(2024, 1, 15, 12, 0),  # winter no DST
        ],
    )
    def test_dst(self, dt, ny_py, ny_rs):
        assert ny_py.dst(dt) == ny_rs.dst(dt)

    @pytest.mark.parametrize(
        "dt",
        [
            datetime(2024, 6, 15, 12, 0),
            datetime(2024, 1, 15, 12, 0),
        ],
    )
    def test_tzname(self, dt, ny_py, ny_rs):
        assert ny_py.tzname(dt) == ny_rs.tzname(dt)

    # Fall-back: Nov 3, 2024 01:30 is ambiguous
    @pytest.mark.xfail(reason="Rust tzfile is_ambiguous() not yet correct")
    def test_is_ambiguous_fall_back(self, ny_py, ny_rs):
        dt = datetime(2024, 11, 3, 1, 30)
        assert ny_py.is_ambiguous(dt) == ny_rs.is_ambiguous(dt) is True

    # Normal times are not ambiguous
    @pytest.mark.parametrize(
        "dt",
        [
            datetime(2024, 6, 15, 12, 0),
            datetime(2024, 1, 15, 12, 0),
            datetime(2024, 11, 3, 3, 0),
        ],
    )
    def test_not_ambiguous(self, dt, ny_py, ny_rs):
        assert ny_py.is_ambiguous(dt) == ny_rs.is_ambiguous(dt) is False


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
@pytest.mark.skipif(
    not _has_tzfile("America/New_York"),
    reason="America/New_York tzfile not found",
)
class TestTzFileMultiZoneCompat:
    """Test tzfile across multiple timezones."""

    @pytest.mark.parametrize(
        "zone,dt",
        [
            ("America/Chicago", datetime(2024, 6, 15, 12, 0)),
            ("America/Chicago", datetime(2024, 1, 15, 12, 0)),
            ("America/Los_Angeles", datetime(2024, 6, 15, 12, 0)),
            ("America/Los_Angeles", datetime(2024, 1, 15, 12, 0)),
            ("Europe/London", datetime(2024, 6, 15, 12, 0)),
            ("Europe/London", datetime(2024, 1, 15, 12, 0)),
            ("Asia/Tokyo", datetime(2024, 6, 15, 12, 0)),
            ("Asia/Tokyo", datetime(2024, 1, 15, 12, 0)),
            ("Australia/Sydney", datetime(2024, 6, 15, 12, 0)),
            ("Australia/Sydney", datetime(2024, 1, 15, 12, 0)),
        ],
    )
    def test_utcoffset(self, zone, dt):
        path = os.path.join(ZONEINFO_DIR, zone)
        if not os.path.isfile(path):
            pytest.skip(f"{zone} tzfile not found")
        py_tz = py_tzfile(path)
        rs_tz = rs_tzfile(path)
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt), (
            f"{zone} @ {dt}: py={py_tz.utcoffset(dt)}, rs={rs_tz.utcoffset(dt)}"
        )

    @pytest.mark.parametrize(
        "zone,dt",
        [
            ("America/Chicago", datetime(2024, 6, 15, 12, 0)),
            ("America/Los_Angeles", datetime(2024, 6, 15, 12, 0)),
            ("Europe/London", datetime(2024, 6, 15, 12, 0)),
            ("Asia/Tokyo", datetime(2024, 6, 15, 12, 0)),
        ],
    )
    def test_dst(self, zone, dt):
        path = os.path.join(ZONEINFO_DIR, zone)
        if not os.path.isfile(path):
            pytest.skip(f"{zone} tzfile not found")
        py_tz = py_tzfile(path)
        rs_tz = rs_tzfile(path)
        assert py_tz.dst(dt) == rs_tz.dst(dt), (
            f"{zone} @ {dt}: py={py_tz.dst(dt)}, rs={rs_tz.dst(dt)}"
        )


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
class TestTzLocalCompat:
    """Compare tzlocal — both should return the same offset for the same datetime."""

    def test_utcoffset_now(self):
        dt = datetime.now()
        py_tz = py_tzlocal()
        rs_tz = rs_tzlocal()
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)

    def test_dst_now(self):
        dt = datetime.now()
        py_tz = py_tzlocal()
        rs_tz = rs_tzlocal()
        assert py_tz.dst(dt) == rs_tz.dst(dt)

    @pytest.mark.parametrize(
        "dt",
        [
            datetime(2024, 1, 15, 12, 0),
            datetime(2024, 6, 15, 12, 0),
        ],
    )
    def test_utcoffset_fixed_dates(self, dt):
        py_tz = py_tzlocal()
        rs_tz = rs_tzlocal()
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
class TestTzStrCompat:
    """Compare tzstr (POSIX TZ string) parsing and behavior."""

    @pytest.mark.parametrize(
        "tz_string,dt",
        [
            # US Eastern
            ("EST5EDT,M3.2.0/2,M11.1.0/2", datetime(2024, 6, 15, 12, 0)),
            ("EST5EDT,M3.2.0/2,M11.1.0/2", datetime(2024, 1, 15, 12, 0)),
            # US Central
            ("CST6CDT,M3.2.0/2,M11.1.0/2", datetime(2024, 6, 15, 12, 0)),
            ("CST6CDT,M3.2.0/2,M11.1.0/2", datetime(2024, 1, 15, 12, 0)),
            # US Pacific
            ("PST8PDT,M3.2.0/2,M11.1.0/2", datetime(2024, 6, 15, 12, 0)),
            ("PST8PDT,M3.2.0/2,M11.1.0/2", datetime(2024, 1, 15, 12, 0)),
            # Europe Central
            ("CET-1CEST,M3.5.0/2,M10.5.0/3", datetime(2024, 6, 15, 12, 0)),
            ("CET-1CEST,M3.5.0/2,M10.5.0/3", datetime(2024, 1, 15, 12, 0)),
            # No DST (fixed offset)
            ("JST-9", datetime(2024, 6, 15, 12, 0)),
            ("JST-9", datetime(2024, 1, 15, 12, 0)),
        ],
    )
    def test_utcoffset(self, tz_string, dt):
        py_tz = py_tzstr(tz_string)
        rs_tz = rs_tzstr(tz_string)
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt), (
            f"{tz_string!r} @ {dt}: py={py_tz.utcoffset(dt)}, rs={rs_tz.utcoffset(dt)}"
        )

    @pytest.mark.parametrize(
        "tz_string,dt",
        [
            ("EST5EDT,M3.2.0/2,M11.1.0/2", datetime(2024, 6, 15, 12, 0)),
            ("EST5EDT,M3.2.0/2,M11.1.0/2", datetime(2024, 1, 15, 12, 0)),
            ("JST-9", datetime(2024, 6, 15, 12, 0)),
        ],
    )
    def test_dst(self, tz_string, dt):
        py_tz = py_tzstr(tz_string)
        rs_tz = rs_tzstr(tz_string)
        assert py_tz.dst(dt) == rs_tz.dst(dt), (
            f"{tz_string!r} @ {dt}: py={py_tz.dst(dt)}, rs={rs_tz.dst(dt)}"
        )

    @pytest.mark.parametrize(
        "tz_string,dt",
        [
            ("EST5EDT,M3.2.0/2,M11.1.0/2", datetime(2024, 6, 15, 12, 0)),
            ("EST5EDT,M3.2.0/2,M11.1.0/2", datetime(2024, 1, 15, 12, 0)),
        ],
    )
    def test_tzname(self, tz_string, dt):
        py_tz = py_tzstr(tz_string)
        rs_tz = rs_tzstr(tz_string)
        assert py_tz.tzname(dt) == rs_tz.tzname(dt)

    @pytest.mark.xfail(reason="Rust tzstr is_ambiguous() not yet correct")
    def test_is_ambiguous_fall_back(self):
        """US Eastern fall back: Nov 3, 2024 01:30 is ambiguous."""
        tz_string = "EST5EDT,M3.2.0/2,M11.1.0/2"
        dt = datetime(2024, 11, 3, 1, 30)
        py_tz = py_tzstr(tz_string)
        rs_tz = rs_tzstr(tz_string)
        assert py_tz.is_ambiguous(dt) == rs_tz.is_ambiguous(dt) is True

    def test_not_ambiguous_normal(self):
        tz_string = "EST5EDT,M3.2.0/2,M11.1.0/2"
        dt = datetime(2024, 6, 15, 12, 0)
        py_tz = py_tzstr(tz_string)
        rs_tz = rs_tzstr(tz_string)
        assert py_tz.is_ambiguous(dt) == rs_tz.is_ambiguous(dt) is False


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
class TestGettzCompat:
    """Compare gettz() factory function."""

    def test_utc(self):
        py_tz = py_gettz("UTC")
        rs_tz = rs_gettz("UTC")
        dt = datetime(2024, 6, 15, 12, 0)
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)

    def test_gmt(self):
        py_tz = py_gettz("GMT")
        rs_tz = rs_gettz("GMT")
        dt = datetime(2024, 6, 15, 12, 0)
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)

    @pytest.mark.skipif(
        not _has_tzfile("America/New_York"),
        reason="America/New_York tzfile not found",
    )
    @pytest.mark.parametrize(
        "dt",
        [
            datetime(2024, 6, 15, 12, 0),
            datetime(2024, 1, 15, 12, 0),
        ],
    )
    def test_iana_new_york(self, dt):
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)

    @pytest.mark.skipif(
        not _has_tzfile("Asia/Tokyo"),
        reason="Asia/Tokyo tzfile not found",
    )
    def test_iana_tokyo(self):
        dt = datetime(2024, 6, 15, 12, 0)
        py_tz = py_gettz("Asia/Tokyo")
        rs_tz = rs_gettz("Asia/Tokyo")
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)

    @pytest.mark.skipif(
        not _has_tzfile("Europe/London"),
        reason="Europe/London tzfile not found",
    )
    @pytest.mark.parametrize(
        "dt",
        [
            datetime(2024, 6, 15, 12, 0),  # BST
            datetime(2024, 1, 15, 12, 0),  # GMT
        ],
    )
    def test_iana_london(self, dt):
        py_tz = py_gettz("Europe/London")
        rs_tz = rs_gettz("Europe/London")
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)

    def test_posix_tz_string(self):
        dt = datetime(2024, 6, 15, 12, 0)
        py_tz = py_gettz("EST5EDT,M3.2.0/2,M11.1.0/2")
        rs_tz = rs_gettz("EST5EDT,M3.2.0/2,M11.1.0/2")
        assert py_tz is not None and rs_tz is not None
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)

    def test_none_returns_local(self):
        """gettz(None) should return local timezone for both."""
        py_tz = py_gettz(None)
        rs_tz = rs_gettz(None)
        assert py_tz is not None
        assert rs_tz is not None
        dt = datetime.now()
        assert py_tz.utcoffset(dt) == rs_tz.utcoffset(dt)


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
@pytest.mark.skipif(
    not _has_tzfile("America/New_York"),
    reason="America/New_York tzfile not found",
)
class TestDatetimeExistsCompat:
    """Compare datetime_exists for DST gap detection."""

    def test_normal_time_exists(self):
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 6, 15, 12, 0, tzinfo=py_tz)
        dt_rs = datetime(2024, 6, 15, 12, 0, tzinfo=rs_tz)
        assert py_datetime_exists(dt_py) == rs_datetime_exists(dt_rs) is True

    @pytest.mark.xfail(reason="Rust datetime_exists() gap detection not yet correct")
    def test_spring_forward_gap(self):
        """Mar 10, 2024 02:30 doesn't exist in US/Eastern (spring forward)."""
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 3, 10, 2, 30, tzinfo=py_tz)
        dt_rs = datetime(2024, 3, 10, 2, 30, tzinfo=rs_tz)
        assert py_datetime_exists(dt_py) == rs_datetime_exists(dt_rs) is False

    def test_fall_back_exists(self):
        """Nov 3, 2024 01:30 exists (it's ambiguous but it exists)."""
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 11, 3, 1, 30, tzinfo=py_tz)
        dt_rs = datetime(2024, 11, 3, 1, 30, tzinfo=rs_tz)
        assert py_datetime_exists(dt_py) == rs_datetime_exists(dt_rs) is True


@pytest.mark.skipif(not HAS_TZ, reason="dateutil_rs.tz not available")
@pytest.mark.skipif(
    not _has_tzfile("America/New_York"),
    reason="America/New_York tzfile not found",
)
class TestDatetimeAmbiguousCompat:
    """Compare datetime_ambiguous for DST overlap detection."""

    def test_normal_not_ambiguous(self):
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 6, 15, 12, 0, tzinfo=py_tz)
        dt_rs = datetime(2024, 6, 15, 12, 0, tzinfo=rs_tz)
        assert py_datetime_ambiguous(dt_py) == rs_datetime_ambiguous(dt_rs) is False

    @pytest.mark.xfail(
        reason="Rust datetime_ambiguous() overlap detection not yet correct"
    )
    def test_fall_back_ambiguous(self):
        """Nov 3, 2024 01:30 is ambiguous in US/Eastern (fall back)."""
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 11, 3, 1, 30, tzinfo=py_tz)
        dt_rs = datetime(2024, 11, 3, 1, 30, tzinfo=rs_tz)
        assert py_datetime_ambiguous(dt_py) == rs_datetime_ambiguous(dt_rs) is True

    def test_spring_forward_not_ambiguous(self):
        """Mar 10, 2024 02:30 is NOT ambiguous (it's in a gap, not an overlap)."""
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 3, 10, 2, 30, tzinfo=py_tz)
        dt_rs = datetime(2024, 3, 10, 2, 30, tzinfo=rs_tz)
        assert py_datetime_ambiguous(dt_py) == rs_datetime_ambiguous(dt_rs) is False


# ---------------------------------------------------------------------------
# RRule
# ---------------------------------------------------------------------------
try:
    from dateutil.rrule import (
        DAILY as PY_DAILY,
    )
    from dateutil.rrule import (
        FR as PY_FR,
    )
    from dateutil.rrule import (
        HOURLY as PY_HOURLY,
    )
    from dateutil.rrule import (
        MINUTELY as PY_MINUTELY,
    )
    from dateutil.rrule import (
        MO as PY_MO,
    )
    from dateutil.rrule import (
        MONTHLY as PY_MONTHLY,
    )
    from dateutil.rrule import (
        SA as PY_SA,
    )
    from dateutil.rrule import (
        SECONDLY as PY_SECONDLY,
    )
    from dateutil.rrule import (
        SU as PY_SU,
    )
    from dateutil.rrule import (
        TH as PY_TH,
    )
    from dateutil.rrule import (
        TU as PY_TU,
    )
    from dateutil.rrule import (
        WE as PY_WE,
    )
    from dateutil.rrule import (
        WEEKLY as PY_WEEKLY,
    )
    from dateutil.rrule import (
        YEARLY as PY_YEARLY,
    )
    from dateutil.rrule import (
        rrule as py_rrule,
    )
    from dateutil.rrule import (
        rruleset as py_rruleset,
    )
    from dateutil.rrule import (
        rrulestr as py_rrulestr,
    )
    from dateutil_rs.rrule import (
        DAILY as RS_DAILY,
    )
    from dateutil_rs.rrule import (
        FR as RSFR,
    )
    from dateutil_rs.rrule import (
        HOURLY as RS_HOURLY,
    )
    from dateutil_rs.rrule import (
        MINUTELY as RS_MINUTELY,
    )
    from dateutil_rs.rrule import (
        MO as RSMO,
    )
    from dateutil_rs.rrule import (
        MONTHLY as RS_MONTHLY,
    )
    from dateutil_rs.rrule import (
        SA as RSSA,
    )
    from dateutil_rs.rrule import (
        SECONDLY as RS_SECONDLY,
    )
    from dateutil_rs.rrule import (
        SU as RSSU,
    )
    from dateutil_rs.rrule import (
        TH as RSTH,
    )
    from dateutil_rs.rrule import (
        TU as RSTU,
    )
    from dateutil_rs.rrule import (
        WE as RSWE,
    )
    from dateutil_rs.rrule import (
        WEEKLY as RS_WEEKLY,
    )
    from dateutil_rs.rrule import (
        YEARLY as RS_YEARLY,
    )
    from dateutil_rs.rrule import (
        rrule as rs_rrule,
    )
    from dateutil_rs.rrule import (
        rruleset as rs_rruleset,
    )
    from dateutil_rs.rrule import (
        rrulestr as rs_rrulestr,
    )

    HAS_RRULE = True
except ImportError:
    HAS_RRULE = False


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleBasicCompat:
    """Basic frequency tests: compare dateutil vs dateutil_rs rrule output."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule, limit=None):
        py_list = list(py_rule)
        rs_list = list(rs_rule)
        if limit:
            py_list = py_list[:limit]
            rs_list = rs_list[:limit]
        assert py_list == rs_list, (
            f"Mismatch:\n  py={py_list[:5]}...\n  rs={rs_list[:5]}..."
        )

    def test_yearly(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, dtstart=self.DTSTART),
        )

    def test_yearly_interval(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, interval=2, dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, interval=2, dtstart=self.DTSTART),
        )

    def test_monthly(self):
        self._assert_same(
            py_rrule(PY_MONTHLY, count=3, dtstart=self.DTSTART),
            rs_rrule(RS_MONTHLY, count=3, dtstart=self.DTSTART),
        )

    def test_monthly_interval(self):
        self._assert_same(
            py_rrule(PY_MONTHLY, count=3, interval=2, dtstart=self.DTSTART),
            rs_rrule(RS_MONTHLY, count=3, interval=2, dtstart=self.DTSTART),
        )

    def test_weekly(self):
        self._assert_same(
            py_rrule(PY_WEEKLY, count=3, dtstart=self.DTSTART),
            rs_rrule(RS_WEEKLY, count=3, dtstart=self.DTSTART),
        )

    def test_weekly_interval(self):
        self._assert_same(
            py_rrule(PY_WEEKLY, count=3, interval=2, dtstart=self.DTSTART),
            rs_rrule(RS_WEEKLY, count=3, interval=2, dtstart=self.DTSTART),
        )

    def test_daily(self):
        self._assert_same(
            py_rrule(PY_DAILY, count=3, dtstart=self.DTSTART),
            rs_rrule(RS_DAILY, count=3, dtstart=self.DTSTART),
        )

    def test_daily_interval(self):
        self._assert_same(
            py_rrule(PY_DAILY, count=3, interval=2, dtstart=self.DTSTART),
            rs_rrule(RS_DAILY, count=3, interval=2, dtstart=self.DTSTART),
        )

    def test_hourly(self):
        self._assert_same(
            py_rrule(PY_HOURLY, count=3, dtstart=self.DTSTART),
            rs_rrule(RS_HOURLY, count=3, dtstart=self.DTSTART),
        )

    def test_minutely(self):
        self._assert_same(
            py_rrule(PY_MINUTELY, count=3, dtstart=self.DTSTART),
            rs_rrule(RS_MINUTELY, count=3, dtstart=self.DTSTART),
        )

    def test_secondly(self):
        self._assert_same(
            py_rrule(PY_SECONDLY, count=3, dtstart=self.DTSTART),
            rs_rrule(RS_SECONDLY, count=3, dtstart=self.DTSTART),
        )

    def test_until(self):
        until = datetime(1997, 12, 31)
        self._assert_same(
            py_rrule(PY_MONTHLY, dtstart=self.DTSTART, until=until),
            rs_rrule(RS_MONTHLY, dtstart=self.DTSTART, until=until),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleByMonthCompat:
    """bymonth filter tests."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_bymonth(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, bymonth=(1, 3), dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, bymonth=(1, 3), dtstart=self.DTSTART),
        )

    def test_monthly_bymonth(self):
        self._assert_same(
            py_rrule(PY_MONTHLY, count=3, bymonth=(1, 3), dtstart=self.DTSTART),
            rs_rrule(RS_MONTHLY, count=3, bymonth=(1, 3), dtstart=self.DTSTART),
        )

    def test_weekly_bymonth(self):
        self._assert_same(
            py_rrule(PY_WEEKLY, count=3, bymonth=(1, 3), dtstart=self.DTSTART),
            rs_rrule(RS_WEEKLY, count=3, bymonth=(1, 3), dtstart=self.DTSTART),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleByMonthDayCompat:
    """bymonthday filter tests."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_bymonthday(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, bymonthday=(1, 3), dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, bymonthday=(1, 3), dtstart=self.DTSTART),
        )

    def test_monthly_bymonthday(self):
        self._assert_same(
            py_rrule(PY_MONTHLY, count=3, bymonthday=(1, 3), dtstart=self.DTSTART),
            rs_rrule(RS_MONTHLY, count=3, bymonthday=(1, 3), dtstart=self.DTSTART),
        )

    def test_monthly_bymonthday_negative(self):
        self._assert_same(
            py_rrule(PY_MONTHLY, count=3, bymonthday=(-1,), dtstart=self.DTSTART),
            rs_rrule(RS_MONTHLY, count=3, bymonthday=(-1,), dtstart=self.DTSTART),
        )

    def test_yearly_bymonth_and_bymonthday(self):
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=3,
                bymonth=(1, 3),
                bymonthday=(5, 7),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=3,
                bymonth=(1, 3),
                bymonthday=(5, 7),
                dtstart=self.DTSTART,
            ),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleByWeekDayCompat:
    """byweekday filter tests."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_byweekday(self):
        self._assert_same(
            py_rrule(
                PY_YEARLY, count=3, byweekday=(PY_TU, PY_TH), dtstart=self.DTSTART
            ),
            rs_rrule(RS_YEARLY, count=3, byweekday=(RSTU, RSTH), dtstart=self.DTSTART),
        )

    def test_monthly_byweekday(self):
        self._assert_same(
            py_rrule(
                PY_MONTHLY, count=3, byweekday=(PY_TU, PY_TH), dtstart=self.DTSTART
            ),
            rs_rrule(RS_MONTHLY, count=3, byweekday=(RSTU, RSTH), dtstart=self.DTSTART),
        )

    def test_weekly_byweekday(self):
        self._assert_same(
            py_rrule(
                PY_WEEKLY, count=3, byweekday=(PY_TU, PY_TH), dtstart=self.DTSTART
            ),
            rs_rrule(RS_WEEKLY, count=3, byweekday=(RSTU, RSTH), dtstart=self.DTSTART),
        )

    def test_yearly_by_nweekday(self):
        """Nth weekday of the year (e.g. 1st Tuesday)."""
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, byweekday=PY_TU(1), dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, byweekday=RSTU(1), dtstart=self.DTSTART),
        )

    def test_monthly_by_nweekday(self):
        """1st Friday and -1st Friday of each month."""
        self._assert_same(
            py_rrule(
                PY_MONTHLY,
                count=6,
                byweekday=(PY_FR(1), PY_FR(-1)),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_MONTHLY,
                count=6,
                byweekday=(RSFR(1), RSFR(-1)),
                dtstart=self.DTSTART,
            ),
        )

    def test_yearly_bymonth_and_byweekday(self):
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=3,
                bymonth=(1, 3),
                byweekday=(PY_TU, PY_TH),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=3,
                bymonth=(1, 3),
                byweekday=(RSTU, RSTH),
                dtstart=self.DTSTART,
            ),
        )

    def test_yearly_bymonth_and_nweekday(self):
        """3rd Tuesday of Jan and Mar."""
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=3,
                bymonth=(1, 3),
                byweekday=PY_TU(3),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=3,
                bymonth=(1, 3),
                byweekday=RSTU(3),
                dtstart=self.DTSTART,
            ),
        )

    def test_monthly_bymonthday_and_byweekday(self):
        self._assert_same(
            py_rrule(
                PY_MONTHLY,
                count=3,
                bymonthday=(1, 3),
                byweekday=(PY_TU, PY_TH),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_MONTHLY,
                count=3,
                bymonthday=(1, 3),
                byweekday=(RSTU, RSTH),
                dtstart=self.DTSTART,
            ),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleByYearDayCompat:
    """byyearday filter tests."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_byyearday(self):
        self._assert_same(
            py_rrule(
                PY_YEARLY, count=4, byyearday=(1, 100, 200, 365), dtstart=self.DTSTART
            ),
            rs_rrule(
                RS_YEARLY, count=4, byyearday=(1, 100, 200, 365), dtstart=self.DTSTART
            ),
        )

    def test_yearly_byyearday_neg(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=4, byyearday=(-1,), dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=4, byyearday=(-1,), dtstart=self.DTSTART),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleByWeekNoCompat:
    """byweekno filter tests."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_byweekno(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, byweekno=20, dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, byweekno=20, dtstart=self.DTSTART),
        )

    def test_yearly_byweekno_and_byweekday(self):
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=3,
                byweekno=20,
                byweekday=PY_MO,
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=3,
                byweekno=20,
                byweekday=RSMO,
                dtstart=self.DTSTART,
            ),
        )

    def test_yearly_byweekno_53(self):
        """Week 53 — tricky edge case."""
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=3,
                byweekno=53,
                byweekday=PY_MO,
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=3,
                byweekno=53,
                byweekday=RSMO,
                dtstart=self.DTSTART,
            ),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleByEasterCompat:
    """byeaster filter tests."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_byeaster(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, byeaster=0, dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, byeaster=0, dtstart=self.DTSTART),
        )

    def test_yearly_byeaster_positive_offset(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, byeaster=1, dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, byeaster=1, dtstart=self.DTSTART),
        )

    def test_yearly_byeaster_negative_offset(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, byeaster=-2, dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, byeaster=-2, dtstart=self.DTSTART),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleByTimeCompat:
    """byhour / byminute / bysecond filter tests."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_byhour(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, byhour=(6, 18), dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, byhour=(6, 18), dtstart=self.DTSTART),
        )

    def test_yearly_byminute(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, byminute=(6, 18), dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, byminute=(6, 18), dtstart=self.DTSTART),
        )

    def test_yearly_bysecond(self):
        self._assert_same(
            py_rrule(PY_YEARLY, count=3, bysecond=(6, 18), dtstart=self.DTSTART),
            rs_rrule(RS_YEARLY, count=3, bysecond=(6, 18), dtstart=self.DTSTART),
        )

    def test_daily_byhour_and_byminute(self):
        self._assert_same(
            py_rrule(
                PY_DAILY,
                count=6,
                byhour=(9, 17),
                byminute=(0, 30),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_DAILY,
                count=6,
                byhour=(9, 17),
                byminute=(0, 30),
                dtstart=self.DTSTART,
            ),
        )

    def test_hourly_byminute_and_bysecond(self):
        self._assert_same(
            py_rrule(
                PY_HOURLY,
                count=3,
                byminute=(15, 45),
                bysecond=(0,),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_HOURLY,
                count=3,
                byminute=(15, 45),
                bysecond=(0,),
                dtstart=self.DTSTART,
            ),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleBySetPosCompat:
    """bysetpos filter tests."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_bysetpos(self):
        """Last day of the year that is TU or TH."""
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=3,
                byweekday=(PY_TU, PY_TH),
                bysetpos=-1,
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=3,
                byweekday=(RSTU, RSTH),
                bysetpos=-1,
                dtstart=self.DTSTART,
            ),
        )

    def test_monthly_bysetpos(self):
        """3rd instance of monthday 7 or 1 (i.e. 3rd occurrence in the set)."""
        self._assert_same(
            py_rrule(
                PY_MONTHLY,
                count=3,
                bymonthday=(7, 1),
                bysetpos=3,
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_MONTHLY,
                count=3,
                bymonthday=(7, 1),
                bysetpos=3,
                dtstart=self.DTSTART,
            ),
        )

    def test_monthly_byday_bysetpos_neg(self):
        """Last weekday of month (MO-FR, bysetpos=-1)."""
        self._assert_same(
            py_rrule(
                PY_MONTHLY,
                count=3,
                byweekday=(PY_MO, PY_TU, PY_WE, PY_TH, PY_FR),
                bysetpos=-1,
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_MONTHLY,
                count=3,
                byweekday=(RSMO, RSTU, RSWE, RSTH, RSFR),
                bysetpos=-1,
                dtstart=self.DTSTART,
            ),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleWkstCompat:
    """wkst (week start day) tests."""

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_weekly_wkst_su(self):
        self._assert_same(
            py_rrule(
                PY_WEEKLY,
                count=3,
                wkst=PY_SU,
                dtstart=datetime(1997, 9, 2, 9, 0),
            ),
            rs_rrule(
                RS_WEEKLY,
                count=3,
                wkst=RSSU,
                dtstart=datetime(1997, 9, 2, 9, 0),
            ),
        )

    def test_weekly_wkst_su_byweekday(self):
        self._assert_same(
            py_rrule(
                PY_WEEKLY,
                count=3,
                wkst=PY_SU,
                byweekday=(PY_TU, PY_TH),
                dtstart=datetime(1997, 9, 2, 9, 0),
            ),
            rs_rrule(
                RS_WEEKLY,
                count=3,
                wkst=RSSU,
                byweekday=(RSTU, RSTH),
                dtstart=datetime(1997, 9, 2, 9, 0),
            ),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleQueryCompat:
    """before / after / between / count query methods."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def test_before(self):
        py_r = py_rrule(PY_DAILY, count=30, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=30, dtstart=self.DTSTART)
        target = datetime(1997, 9, 15, 9, 0)
        assert py_r.before(target) == rs_r.before(target)

    def test_before_inc(self):
        py_r = py_rrule(PY_DAILY, count=30, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=30, dtstart=self.DTSTART)
        target = datetime(1997, 9, 15, 9, 0)
        assert py_r.before(target, inc=True) == rs_r.before(target, inc=True)

    def test_after(self):
        py_r = py_rrule(PY_DAILY, count=30, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=30, dtstart=self.DTSTART)
        target = datetime(1997, 9, 15, 9, 0)
        assert py_r.after(target) == rs_r.after(target)

    def test_after_inc(self):
        py_r = py_rrule(PY_DAILY, count=30, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=30, dtstart=self.DTSTART)
        target = datetime(1997, 9, 15, 9, 0)
        assert py_r.after(target, inc=True) == rs_r.after(target, inc=True)

    def test_between(self):
        py_r = py_rrule(PY_DAILY, count=30, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=30, dtstart=self.DTSTART)
        after = datetime(1997, 9, 10, 9, 0)
        before = datetime(1997, 9, 20, 9, 0)
        assert list(py_r.between(after, before, count=100)) == list(
            rs_r.between(after, before, count=100)
        )

    def test_between_inc(self):
        py_r = py_rrule(PY_DAILY, count=30, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=30, dtstart=self.DTSTART)
        after = datetime(1997, 9, 10, 9, 0)
        before = datetime(1997, 9, 20, 9, 0)
        assert list(py_r.between(after, before, inc=True, count=100)) == list(
            rs_r.between(after, before, inc=True, count=100)
        )

    def test_count(self):
        py_r = py_rrule(PY_DAILY, count=30, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=30, dtstart=self.DTSTART)
        assert py_r.count() == rs_r.count() == 30

    def test_getitem(self):
        py_r = py_rrule(PY_DAILY, count=10, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=10, dtstart=self.DTSTART)
        assert py_r[0] == rs_r[0]
        assert py_r[5] == rs_r[5]
        assert py_r[-1] == rs_r[-1]

    def test_contains(self):
        py_r = py_rrule(PY_DAILY, count=10, dtstart=self.DTSTART)
        rs_r = rs_rrule(RS_DAILY, count=10, dtstart=self.DTSTART)
        target = datetime(1997, 9, 5, 9, 0)
        assert (target in py_r) == (target in rs_r) is True
        miss = datetime(1997, 9, 5, 10, 0)
        assert (miss in py_r) == (miss in rs_r) is False


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleSetCompat:
    """rruleset tests — combine rules, rdates, exrules, exdates."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def test_rruleset_two_rules(self):
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=3, dtstart=datetime(1997, 9, 2, 9, 0)))
        py_set.rrule(py_rrule(PY_DAILY, count=3, dtstart=datetime(1997, 9, 5, 9, 0)))

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=3, dtstart=datetime(1997, 9, 2, 9, 0)))
        rs_set.rrule(rs_rrule(RS_DAILY, count=3, dtstart=datetime(1997, 9, 5, 9, 0)))

        assert list(py_set) == list(rs_set)

    def test_rruleset_rdate(self):
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=3, dtstart=self.DTSTART))
        py_set.rdate(datetime(1997, 9, 10, 9, 0))

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=3, dtstart=self.DTSTART))
        rs_set.rdate(datetime(1997, 9, 10, 9, 0))

        assert list(py_set) == list(rs_set)

    def test_rruleset_exdate(self):
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=5, dtstart=self.DTSTART))
        py_set.exdate(datetime(1997, 9, 4, 9, 0))

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=5, dtstart=self.DTSTART))
        rs_set.exdate(datetime(1997, 9, 4, 9, 0))

        assert list(py_set) == list(rs_set)

    def test_rruleset_exrule(self):
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=10, dtstart=self.DTSTART))
        py_set.exrule(py_rrule(PY_DAILY, count=5, interval=2, dtstart=self.DTSTART))

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=10, dtstart=self.DTSTART))
        rs_set.exrule(rs_rrule(RS_DAILY, count=5, interval=2, dtstart=self.DTSTART))

        assert list(py_set) == list(rs_set)

    def test_rruleset_count(self):
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=5, dtstart=self.DTSTART))
        py_set.exdate(datetime(1997, 9, 4, 9, 0))

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=5, dtstart=self.DTSTART))
        rs_set.exdate(datetime(1997, 9, 4, 9, 0))

        assert py_set.count() == rs_set.count()

    def test_rruleset_before_after(self):
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=10, dtstart=self.DTSTART))

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=10, dtstart=self.DTSTART))

        target = datetime(1997, 9, 5, 9, 0)
        assert py_set.before(target) == rs_set.before(target)
        assert py_set.after(target) == rs_set.after(target)


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleStrCompat:
    """rrulestr() parsing — compare round-trip results."""

    def _assert_same(self, rule_str, **kwargs):
        py_result = list(py_rrulestr(rule_str, **kwargs))
        rs_result = list(rs_rrulestr(rule_str, **kwargs))
        assert py_result == rs_result, (
            f"rrulestr mismatch for {rule_str!r}:\n  py={py_result[:5]}\n  rs={rs_result[:5]}"
        )

    def test_yearly_count(self):
        self._assert_same("DTSTART:19970902T090000\nRRULE:FREQ=YEARLY;COUNT=3")

    def test_monthly_until(self):
        self._assert_same(
            "DTSTART:19970902T090000\nRRULE:FREQ=MONTHLY;UNTIL=19971231T090000"
        )

    def test_weekly_byday(self):
        self._assert_same(
            "DTSTART:19970902T090000\nRRULE:FREQ=WEEKLY;COUNT=6;BYDAY=TU,TH"
        )

    def test_daily_interval(self):
        self._assert_same(
            "DTSTART:19970902T090000\nRRULE:FREQ=DAILY;INTERVAL=10;COUNT=5"
        )

    def test_monthly_byday_with_n(self):
        self._assert_same(
            "DTSTART:19970902T090000\nRRULE:FREQ=MONTHLY;COUNT=3;BYDAY=1FR"
        )

    def test_yearly_bymonth_byday(self):
        self._assert_same(
            "DTSTART:19970902T090000\nRRULE:FREQ=YEARLY;COUNT=3;BYMONTH=1,3;BYDAY=TU,TH"
        )

    def test_monthly_bysetpos(self):
        self._assert_same(
            "DTSTART:19970902T090000\n"
            "RRULE:FREQ=MONTHLY;COUNT=3;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1"
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleCombinedFiltersCompat:
    """Complex combined filter tests to catch subtle differences."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def _assert_same(self, py_rule, rs_rule):
        assert list(py_rule) == list(rs_rule)

    def test_yearly_bymonth_bymonthday_byweekday(self):
        """Intersection of month, monthday, and weekday filters."""
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=3,
                bymonth=(1, 3),
                bymonthday=(1, 3),
                byweekday=(PY_TU, PY_TH),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=3,
                bymonth=(1, 3),
                bymonthday=(1, 3),
                byweekday=(RSTU, RSTH),
                dtstart=self.DTSTART,
            ),
        )

    def test_monthly_large_interval_bymonthday(self):
        """Monthly with large interval + bymonthday."""
        self._assert_same(
            py_rrule(
                PY_MONTHLY,
                count=3,
                interval=18,
                bymonthday=(10, 11, 12, 13, 14, 15),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_MONTHLY,
                count=3,
                interval=18,
                bymonthday=(10, 11, 12, 13, 14, 15),
                dtstart=self.DTSTART,
            ),
        )

    def test_yearly_bymonth_byyearday(self):
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=4,
                bymonth=(4, 7),
                byyearday=(1, 100, 200, 365),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=4,
                bymonth=(4, 7),
                byyearday=(1, 100, 200, 365),
                dtstart=self.DTSTART,
            ),
        )

    def test_yearly_byhour_byminute_bysecond(self):
        """All three time-based filters."""
        self._assert_same(
            py_rrule(
                PY_YEARLY,
                count=4,
                byhour=(6, 18),
                byminute=(0, 30),
                bysecond=(0,),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_YEARLY,
                count=4,
                byhour=(6, 18),
                byminute=(0, 30),
                bysecond=(0,),
                dtstart=self.DTSTART,
            ),
        )

    def test_daily_bymonth_byweekday(self):
        """Daily occurrences filtered to specific month and weekday."""
        self._assert_same(
            py_rrule(
                PY_DAILY,
                count=5,
                bymonth=(1,),
                byweekday=(PY_MO, PY_FR),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_DAILY,
                count=5,
                bymonth=(1,),
                byweekday=(RSMO, RSFR),
                dtstart=self.DTSTART,
            ),
        )

    def test_weekly_interval_byweekday(self):
        """Bi-weekly on specific days."""
        self._assert_same(
            py_rrule(
                PY_WEEKLY,
                count=6,
                interval=2,
                byweekday=(PY_MO, PY_WE, PY_FR),
                dtstart=self.DTSTART,
            ),
            rs_rrule(
                RS_WEEKLY,
                count=6,
                interval=2,
                byweekday=(RSMO, RSWE, RSFR),
                dtstart=self.DTSTART,
            ),
        )


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleLazyIndexingCompat:
    """Test that positive indexing and slicing work lazily on infinite rules,
    matching Python dateutil behavior."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def test_positive_index_zero(self):
        """rule[0] on an infinite rrule should return the first occurrence."""
        py_result = py_rrule(PY_DAILY, dtstart=self.DTSTART)[0]
        rs_result = rs_rrule(RS_DAILY, dtstart=self.DTSTART)[0]
        assert py_result == rs_result

    def test_positive_index_five(self):
        """rule[5] on an infinite rrule should return the 6th occurrence."""
        py_result = py_rrule(PY_DAILY, dtstart=self.DTSTART)[5]
        rs_result = rs_rrule(RS_DAILY, dtstart=self.DTSTART)[5]
        assert py_result == rs_result

    def test_positive_index_finite(self):
        """rule[2] on a finite rrule should match."""
        py_result = py_rrule(PY_DAILY, count=5, dtstart=self.DTSTART)[2]
        rs_result = rs_rrule(RS_DAILY, count=5, dtstart=self.DTSTART)[2]
        assert py_result == rs_result

    def test_negative_index_finite(self):
        """rule[-1] on a finite rrule should match."""
        py_result = py_rrule(PY_DAILY, count=5, dtstart=self.DTSTART)[-1]
        rs_result = rs_rrule(RS_DAILY, count=5, dtstart=self.DTSTART)[-1]
        assert py_result == rs_result

    def test_slice_positive_finite(self):
        """rule[1:4] on a finite rrule should match."""
        py_result = list(py_rrule(PY_DAILY, count=10, dtstart=self.DTSTART)[1:4])
        rs_result = list(rs_rrule(RS_DAILY, count=10, dtstart=self.DTSTART)[1:4])
        assert py_result == rs_result

    def test_slice_positive_infinite(self):
        """rule[:3] on an infinite rrule should return first 3 lazily."""
        py_result = list(py_rrule(PY_DAILY, dtstart=self.DTSTART)[:3])
        rs_result = list(rs_rrule(RS_DAILY, dtstart=self.DTSTART)[:3])
        assert py_result == rs_result

    def test_slice_with_step(self):
        """rule[0:6:2] on an infinite rrule should return every other."""
        py_result = list(py_rrule(PY_DAILY, dtstart=self.DTSTART)[0:6:2])
        rs_result = list(rs_rrule(RS_DAILY, dtstart=self.DTSTART)[0:6:2])
        assert py_result == rs_result

    def test_index_out_of_range(self):
        """rule[10] on a rule with count=3 should raise IndexError."""
        with pytest.raises(IndexError):
            py_rrule(PY_DAILY, count=3, dtstart=self.DTSTART)[10]
        with pytest.raises(IndexError):
            rs_rrule(RS_DAILY, count=3, dtstart=self.DTSTART)[10]

    def test_iter_infinite_rrule(self):
        """Iterating an infinite rrule and taking first N should work."""
        import itertools
        py_first5 = list(itertools.islice(py_rrule(PY_DAILY, dtstart=self.DTSTART), 5))
        rs_first5 = list(itertools.islice(rs_rrule(RS_DAILY, dtstart=self.DTSTART), 5))
        assert py_first5 == rs_first5

    def test_iter_infinite_rruleset(self):
        """Iterating an infinite rruleset and taking first N should work."""
        import itertools
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, dtstart=self.DTSTART))
        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, dtstart=self.DTSTART))

        py_first5 = list(itertools.islice(py_set, 5))
        rs_first5 = list(itertools.islice(rs_set, 5))
        assert py_first5 == rs_first5


@pytest.mark.skipif(not HAS_RRULE, reason="dateutil_rs.rrule not available")
class TestRRuleExclusionCompat:
    """Test exclusion behavior matches Python dateutil."""

    DTSTART = datetime(1997, 9, 2, 9, 0)

    def test_exdate_basic(self):
        """rruleset with exdate should exclude matching dates."""
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=5, dtstart=self.DTSTART))
        py_set.exdate(datetime(1997, 9, 4, 9, 0))

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=5, dtstart=self.DTSTART))
        rs_set.exdate(datetime(1997, 9, 4, 9, 0))

        assert list(py_set) == list(rs_set)

    def test_exrule_basic(self):
        """rruleset with exrule should exclude matching dates."""
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=5, dtstart=self.DTSTART))
        py_set.exrule(
            py_rrule(PY_DAILY, count=1, dtstart=datetime(1997, 9, 3, 9, 0))
        )

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=5, dtstart=self.DTSTART))
        rs_set.exrule(
            rs_rrule(RS_DAILY, count=1, dtstart=datetime(1997, 9, 3, 9, 0))
        )

        assert list(py_set) == list(rs_set)

    def test_exrule_and_exdate_same_dt(self):
        """When both exrule and exdate match the same dt, both should be
        consumed and subsequent dates should still appear."""
        py_set = py_rruleset()
        py_set.rrule(py_rrule(PY_DAILY, count=5, dtstart=self.DTSTART))
        py_set.exrule(
            py_rrule(PY_DAILY, count=1, dtstart=datetime(1997, 9, 3, 9, 0))
        )
        py_set.exdate(datetime(1997, 9, 3, 9, 0))
        py_set.exdate(datetime(1997, 9, 5, 9, 0))

        rs_set = rs_rruleset()
        rs_set.rrule(rs_rrule(RS_DAILY, count=5, dtstart=self.DTSTART))
        rs_set.exrule(
            rs_rrule(RS_DAILY, count=1, dtstart=datetime(1997, 9, 3, 9, 0))
        )
        rs_set.exdate(datetime(1997, 9, 3, 9, 0))
        rs_set.exdate(datetime(1997, 9, 5, 9, 0))

        assert list(py_set) == list(rs_set)
