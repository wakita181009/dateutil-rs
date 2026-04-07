"""Compatibility tests: compare python-dateutil vs dateutil_rs output directly.

Both libraries are imported side-by-side so we can verify that dateutil_rs
produces identical results to the reference python-dateutil implementation.

Run:
    uv run pytest tests/test_compat.py -v
"""

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
        assert py_rd.months == rs_rd.months, f"months: py={py_rd.months}, rs={rs_rd.months}"
        assert py_rd.days == rs_rd.days, f"days: py={py_rd.days}, rs={rs_rd.days}"
        assert py_rd.hours == rs_rd.hours, f"hours: py={py_rd.hours}, rs={rs_rd.hours}"
        assert py_rd.minutes == rs_rd.minutes, f"minutes: py={py_rd.minutes}, rs={rs_rd.minutes}"
        assert py_rd.seconds == rs_rd.seconds, f"seconds: py={py_rd.seconds}, rs={rs_rd.seconds}"
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
        py_result = py_relativedelta(days=10) + py_relativedelta(years=1, months=2, days=3)
        rs_result = rs_relativedelta(days=10) + rs_relativedelta(years=1, months=2, days=3)
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
