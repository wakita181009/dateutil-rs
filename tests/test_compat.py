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
    from dateutil_rs._native import _TzOffset

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
        assert py_tzutc().is_ambiguous(dt) == rs_tzutc().is_ambiguous(dt) == False

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
        assert py_tz.is_ambiguous(dt) == rs_tz.is_ambiguous(dt) == False


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
            pytest.param(
                datetime(2024, 6, 15, 12, 0),
                marks=pytest.mark.xfail(
                    reason="Rust tzfile dst() incorrect during DST"
                ),
            ),
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
        assert ny_py.is_ambiguous(dt) == ny_rs.is_ambiguous(dt) == True

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
        assert ny_py.is_ambiguous(dt) == ny_rs.is_ambiguous(dt) == False


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
            pytest.param(
                "America/Chicago",
                datetime(2024, 6, 15, 12, 0),
                marks=pytest.mark.xfail(
                    reason="Rust tzfile dst() incorrect during DST"
                ),
            ),
            pytest.param(
                "America/Los_Angeles",
                datetime(2024, 6, 15, 12, 0),
                marks=pytest.mark.xfail(
                    reason="Rust tzfile dst() incorrect during DST"
                ),
            ),
            pytest.param(
                "Europe/London",
                datetime(2024, 6, 15, 12, 0),
                marks=pytest.mark.xfail(
                    reason="Rust tzfile dst() incorrect during DST"
                ),
            ),
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
        assert py_tz.is_ambiguous(dt) == rs_tz.is_ambiguous(dt) == True

    def test_not_ambiguous_normal(self):
        tz_string = "EST5EDT,M3.2.0/2,M11.1.0/2"
        dt = datetime(2024, 6, 15, 12, 0)
        py_tz = py_tzstr(tz_string)
        rs_tz = rs_tzstr(tz_string)
        assert py_tz.is_ambiguous(dt) == rs_tz.is_ambiguous(dt) == False


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
        assert py_datetime_exists(dt_py) == rs_datetime_exists(dt_rs) == True

    @pytest.mark.xfail(reason="Rust datetime_exists() gap detection not yet correct")
    def test_spring_forward_gap(self):
        """Mar 10, 2024 02:30 doesn't exist in US/Eastern (spring forward)."""
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 3, 10, 2, 30, tzinfo=py_tz)
        dt_rs = datetime(2024, 3, 10, 2, 30, tzinfo=rs_tz)
        assert py_datetime_exists(dt_py) == rs_datetime_exists(dt_rs) == False

    def test_fall_back_exists(self):
        """Nov 3, 2024 01:30 exists (it's ambiguous but it exists)."""
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 11, 3, 1, 30, tzinfo=py_tz)
        dt_rs = datetime(2024, 11, 3, 1, 30, tzinfo=rs_tz)
        assert py_datetime_exists(dt_py) == rs_datetime_exists(dt_rs) == True


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
        assert py_datetime_ambiguous(dt_py) == rs_datetime_ambiguous(dt_rs) == False

    @pytest.mark.xfail(
        reason="Rust datetime_ambiguous() overlap detection not yet correct"
    )
    def test_fall_back_ambiguous(self):
        """Nov 3, 2024 01:30 is ambiguous in US/Eastern (fall back)."""
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 11, 3, 1, 30, tzinfo=py_tz)
        dt_rs = datetime(2024, 11, 3, 1, 30, tzinfo=rs_tz)
        assert py_datetime_ambiguous(dt_py) == rs_datetime_ambiguous(dt_rs) == True

    def test_spring_forward_not_ambiguous(self):
        """Mar 10, 2024 02:30 is NOT ambiguous (it's in a gap, not an overlap)."""
        py_tz = py_gettz("America/New_York")
        rs_tz = rs_gettz("America/New_York")
        dt_py = datetime(2024, 3, 10, 2, 30, tzinfo=py_tz)
        dt_rs = datetime(2024, 3, 10, 2, 30, tzinfo=rs_tz)
        assert py_datetime_ambiguous(dt_py) == rs_datetime_ambiguous(dt_rs) == False
