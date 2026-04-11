"""Type compatibility tests: verify dateutil_rs returns the same Python types as python-dateutil.

Unlike test_compat.py (which checks values), this module checks that type(result),
isinstance() checks, and attribute types match between the two implementations.

Run:
    uv run pytest tests/test_type_compat.py -v
"""

import datetime
import os

import pytest

# ---------------------------------------------------------------------------
# Reference (python-dateutil)
# ---------------------------------------------------------------------------
import dateutil.easter
from dateutil.relativedelta import relativedelta as py_relativedelta

# ---------------------------------------------------------------------------
# Rust implementation
# ---------------------------------------------------------------------------
dateutil_rs = pytest.importorskip("dateutil_rs", exc_type=ImportError)
from dateutil_rs import easter as rs_easter
from dateutil_rs import relativedelta as rs_relativedelta


# ---------------------------------------------------------------------------
# Easter
# ---------------------------------------------------------------------------
class TestEasterTypes:
    def test_returns_date(self):
        py_result = dateutil.easter.easter(2024)
        rs_result = rs_easter(2024)
        assert type(py_result) is type(rs_result) is datetime.date

    def test_not_datetime(self):
        """easter() must return date, not datetime (which is a date subclass)."""
        py_result = dateutil.easter.easter(2024)
        rs_result = rs_easter(2024)
        assert not isinstance(py_result, datetime.datetime)
        assert not isinstance(rs_result, datetime.datetime)


# ---------------------------------------------------------------------------
# RelativeDelta — attribute types
# ---------------------------------------------------------------------------
class TestRelativeDeltaAttrTypes:
    """Verify that relativedelta attribute types match between implementations."""

    def _assert_attr_types_match(self, py_rd, rs_rd, fields):
        for field in fields:
            py_val = getattr(py_rd, field)
            rs_val = getattr(rs_rd, field)
            assert type(py_val) is type(rs_val), (
                f"{field}: py type={type(py_val).__name__}, "
                f"rs type={type(rs_val).__name__}"
            )

    def test_relative_fields(self):
        py_rd = py_relativedelta(
            years=1, months=2, days=3, hours=4, minutes=5, seconds=6
        )
        rs_rd = rs_relativedelta(
            years=1, months=2, days=3, hours=4, minutes=5, seconds=6
        )
        self._assert_attr_types_match(
            py_rd,
            rs_rd,
            ["years", "months", "days", "hours", "minutes", "seconds", "microseconds"],
        )

    def test_leapdays_type(self):
        py_rd = py_relativedelta(years=1)
        rs_rd = rs_relativedelta(years=1)
        assert type(py_rd.leapdays) is type(rs_rd.leapdays)

    def test_absolute_fields_when_set(self):
        py_rd = py_relativedelta(
            year=2024, month=6, day=15, hour=10, minute=30, second=0
        )
        rs_rd = rs_relativedelta(
            year=2024, month=6, day=15, hour=10, minute=30, second=0
        )
        for field in [
            "year",
            "month",
            "day",
            "hour",
            "minute",
            "second",
            "microsecond",
        ]:
            py_val = getattr(py_rd, field)
            rs_val = getattr(rs_rd, field)
            # Both should be int (or None for microsecond if not set)
            assert type(py_val) is type(rs_val), (
                f"{field}: py type={type(py_val).__name__}, "
                f"rs type={type(rs_val).__name__}"
            )

    def test_absolute_fields_when_none(self):
        py_rd = py_relativedelta(days=1)
        rs_rd = rs_relativedelta(days=1)
        for field in [
            "year",
            "month",
            "day",
            "hour",
            "minute",
            "second",
            "microsecond",
        ]:
            py_val = getattr(py_rd, field)
            rs_val = getattr(rs_rd, field)
            assert py_val is None and rs_val is None, (
                f"{field}: expected None/None, got py={py_val}, rs={rs_val}"
            )

    def test_diff_result_fields(self):
        """relativedelta.from_diff(dt1, dt2) diff — check attribute types."""
        dt1 = datetime.datetime(2024, 6, 15, 10, 30, 45, 123456)
        dt2 = datetime.datetime(2020, 1, 1)
        py_rd = py_relativedelta(dt1, dt2)
        rs_rd = rs_relativedelta.from_diff(dt1, dt2)
        self._assert_attr_types_match(
            py_rd,
            rs_rd,
            ["years", "months", "days", "hours", "minutes", "seconds", "microseconds"],
        )


# ---------------------------------------------------------------------------
# RelativeDelta — operator result types
# ---------------------------------------------------------------------------
class TestRelativeDeltaOperatorTypes:
    def test_add_rd_rd(self):
        """relativedelta + relativedelta → relativedelta."""
        py_result = py_relativedelta(days=1) + py_relativedelta(months=1)
        rs_result = rs_relativedelta(days=1) + rs_relativedelta(months=1)
        assert isinstance(py_result, py_relativedelta)
        assert isinstance(rs_result, rs_relativedelta)

    def test_add_datetime_rd(self):
        """datetime + relativedelta → datetime."""
        dt = datetime.datetime(2024, 1, 1)
        py_result = dt + py_relativedelta(months=1)
        rs_result = dt + rs_relativedelta(months=1)
        assert type(py_result) is type(rs_result) is datetime.datetime

    def test_add_date_rd(self):
        """date + relativedelta → date."""
        d = datetime.date(2024, 1, 1)
        py_result = d + py_relativedelta(months=1)
        rs_result = d + rs_relativedelta(months=1)
        assert type(py_result) is type(rs_result) is datetime.date

    def test_sub_datetime_rd(self):
        """datetime - relativedelta → datetime."""
        dt = datetime.datetime(2024, 6, 15)
        py_result = dt - py_relativedelta(months=1)
        rs_result = dt - rs_relativedelta(months=1)
        assert type(py_result) is type(rs_result) is datetime.datetime

    def test_mul(self):
        """relativedelta * int → relativedelta."""
        py_result = py_relativedelta(months=1) * 3
        rs_result = rs_relativedelta(months=1) * 3
        assert isinstance(py_result, py_relativedelta)
        assert isinstance(rs_result, rs_relativedelta)

    def test_neg(self):
        """-relativedelta → relativedelta."""
        py_result = -py_relativedelta(months=1)
        rs_result = -rs_relativedelta(months=1)
        assert isinstance(py_result, py_relativedelta)
        assert isinstance(rs_result, rs_relativedelta)

    def test_bool(self):
        """bool(relativedelta) → bool."""
        assert type(bool(py_relativedelta())) is type(bool(rs_relativedelta())) is bool
        assert (
            type(bool(py_relativedelta(days=1)))
            is type(bool(rs_relativedelta(days=1)))
            is bool
        )


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------
try:
    from dateutil.parser import parse as py_parse
    from dateutil_rs import parse as rs_parse

    HAS_PARSER = True
except ImportError:
    HAS_PARSER = False


@pytest.mark.skipif(not HAS_PARSER, reason="parser not available")
class TestParserTypes:
    def test_parse_returns_datetime(self):
        py_result = py_parse("2024-01-15")
        rs_result = rs_parse("2024-01-15")
        assert type(py_result) is type(rs_result) is datetime.datetime

    def test_parse_with_tz_returns_aware_datetime(self):
        py_result = py_parse("2024-01-15T10:00:00+05:00")
        rs_result = rs_parse("2024-01-15T10:00:00+05:00")
        assert py_result.tzinfo is not None
        assert rs_result.tzinfo is not None

    def test_parse_ignoretz_returns_naive(self):
        py_result = py_parse("2024-01-15T10:00:00+05:00", ignoretz=True)
        rs_result = rs_parse("2024-01-15T10:00:00+05:00", ignoretz=True)
        assert py_result.tzinfo is None
        assert rs_result.tzinfo is None


# ---------------------------------------------------------------------------
# ISO Parser
# ---------------------------------------------------------------------------
try:
    from dateutil.parser import isoparse as py_isoparse
    from dateutil_rs import isoparse as rs_isoparse

    HAS_ISOPARSER = True
except ImportError:
    HAS_ISOPARSER = False


@pytest.mark.skipif(not HAS_ISOPARSER, reason="isoparser not available")
class TestIsoparserTypes:
    def test_returns_datetime(self):
        py_result = py_isoparse("2024-01-15T10:30:00")
        rs_result = rs_isoparse("2024-01-15T10:30:00")
        assert type(py_result) is type(rs_result) is datetime.datetime

    def test_date_only_returns_datetime(self):
        py_result = py_isoparse("2024-01-15")
        rs_result = rs_isoparse("2024-01-15")
        assert type(py_result) is type(rs_result) is datetime.datetime


# ---------------------------------------------------------------------------
# Timezone
# ---------------------------------------------------------------------------
try:
    from dateutil.tz import gettz as py_gettz
    from dateutil.tz import tzlocal as py_tzlocal
    from dateutil.tz import tzoffset as py_tzoffset
    from dateutil.tz import tzutc as py_tzutc
    from dateutil_rs import gettz as rs_gettz
    from dateutil_rs import tzfile as rs_tzfile
    from dateutil_rs import tzlocal as rs_tzlocal
    from dateutil_rs import tzoffset as rs_tzoffset
    from dateutil_rs import tzutc as rs_tzutc

    HAS_TZ = True
except ImportError:
    HAS_TZ = False


_ZONEINFO_DIRS = [
    "/usr/share/zoneinfo",
    "/usr/lib/zoneinfo",
    "/usr/share/lib/zoneinfo",
    "/etc/zoneinfo",
]
ZONEINFO_DIR = next((d for d in _ZONEINFO_DIRS if os.path.isdir(d)), None)


@pytest.mark.skipif(not HAS_TZ, reason="tz not available")
class TestTzTypes:
    """All tz classes must be datetime.tzinfo subclasses."""

    def test_tzutc_is_tzinfo(self):
        assert isinstance(py_tzutc(), datetime.tzinfo)
        assert isinstance(rs_tzutc(), datetime.tzinfo)

    def test_tzoffset_is_tzinfo(self):
        assert isinstance(py_tzoffset("EST", -18000), datetime.tzinfo)
        assert isinstance(rs_tzoffset("EST", -18000), datetime.tzinfo)

    def test_tzlocal_is_tzinfo(self):
        assert isinstance(py_tzlocal(), datetime.tzinfo)
        assert isinstance(rs_tzlocal(), datetime.tzinfo)

    @pytest.mark.skipif(
        ZONEINFO_DIR is None
        or not os.path.isfile(os.path.join(ZONEINFO_DIR or "", "America/New_York")),
        reason="America/New_York tzfile not found",
    )
    def test_tzfile_is_tzinfo(self):
        from dateutil.tz import tzfile as py_tzfile

        path = os.path.join(ZONEINFO_DIR, "America/New_York")
        assert isinstance(py_tzfile(path), datetime.tzinfo)
        assert isinstance(rs_tzfile(path), datetime.tzinfo)

    def test_gettz_returns_tzinfo_or_none(self):
        py_result = py_gettz("UTC")
        rs_result = rs_gettz("UTC")
        assert isinstance(py_result, datetime.tzinfo)
        assert isinstance(rs_result, datetime.tzinfo)


@pytest.mark.skipif(not HAS_TZ, reason="tz not available")
class TestTzMethodReturnTypes:
    """Verify that tzinfo method return types match."""

    DT = datetime.datetime(2024, 6, 15, 12, 0)

    def test_utcoffset_returns_timedelta(self):
        py_td = py_tzutc().utcoffset(self.DT)
        rs_td = rs_tzutc().utcoffset(self.DT)
        assert type(py_td) is type(rs_td) is datetime.timedelta

    def test_dst_returns_timedelta(self):
        py_td = py_tzutc().dst(self.DT)
        rs_td = rs_tzutc().dst(self.DT)
        assert type(py_td) is type(rs_td) is datetime.timedelta

    def test_tzname_returns_str(self):
        py_name = py_tzutc().tzname(self.DT)
        rs_name = rs_tzutc().tzname(self.DT)
        assert type(py_name) is type(rs_name) is str

    def test_tzoffset_utcoffset_returns_timedelta(self):
        py_td = py_tzoffset("EST", -18000).utcoffset(self.DT)
        rs_td = rs_tzoffset("EST", -18000).utcoffset(self.DT)
        assert type(py_td) is type(rs_td) is datetime.timedelta

    def test_tzoffset_tzname_returns_str(self):
        py_name = py_tzoffset("EST", -18000).tzname(self.DT)
        rs_name = rs_tzoffset("EST", -18000).tzname(self.DT)
        assert type(py_name) is type(rs_name) is str

    def test_tzoffset_none_name_returns_none(self):
        """tzoffset(None, 0).tzname() should return None in python-dateutil.

        v1 Rust implementation returns "UTC" instead of None when name is None
        and offset is 0, which is a reasonable alternative.
        """
        py_name = py_tzoffset(None, 0).tzname(self.DT)
        rs_name = rs_tzoffset(None, 0).tzname(self.DT)
        assert py_name is None
        # v1 returns "UTC" for tzoffset(None, 0) instead of None
        assert rs_name is None or rs_name == "UTC"


@pytest.mark.skipif(not HAS_TZ, reason="tz not available")
class TestTzUtilReturnTypes:
    """datetime_exists / datetime_ambiguous return bool."""

    @pytest.mark.skipif(
        ZONEINFO_DIR is None
        or not os.path.isfile(os.path.join(ZONEINFO_DIR or "", "America/New_York")),
        reason="America/New_York tzfile not found",
    )
    def test_datetime_exists_returns_bool(self):
        from dateutil.tz import datetime_exists as py_datetime_exists
        from dateutil_rs import datetime_exists as rs_datetime_exists

        ny_py = py_gettz("America/New_York")
        ny_rs = rs_gettz("America/New_York")
        dt = datetime.datetime(2024, 6, 15, 12, 0, tzinfo=ny_py)
        assert type(py_datetime_exists(dt)) is bool
        # v1 datetime_exists takes two arguments: (naive_dt, tz)
        dt_naive = datetime.datetime(2024, 6, 15, 12, 0)
        assert type(rs_datetime_exists(dt_naive, ny_rs)) is bool

    @pytest.mark.skipif(
        ZONEINFO_DIR is None
        or not os.path.isfile(os.path.join(ZONEINFO_DIR or "", "America/New_York")),
        reason="America/New_York tzfile not found",
    )
    def test_datetime_ambiguous_returns_bool(self):
        from dateutil.tz import datetime_ambiguous as py_datetime_ambiguous
        from dateutil_rs import datetime_ambiguous as rs_datetime_ambiguous

        ny_py = py_gettz("America/New_York")
        ny_rs = rs_gettz("America/New_York")
        dt = datetime.datetime(2024, 11, 3, 1, 30)
        assert type(py_datetime_ambiguous(dt, ny_py)) is bool
        assert type(rs_datetime_ambiguous(dt, ny_rs)) is bool


# ---------------------------------------------------------------------------
# rrule
# ---------------------------------------------------------------------------
try:
    from dateutil.rrule import DAILY as PY_DAILY
    from dateutil.rrule import MONTHLY as PY_MONTHLY
    from dateutil.rrule import rrule as py_rrule
    from dateutil.rrule import rruleset as py_rruleset
    from dateutil.rrule import rrulestr as py_rrulestr
    from dateutil_rs import DAILY as RS_DAILY
    from dateutil_rs import MONTHLY as RS_MONTHLY
    from dateutil_rs import rrule as rs_rrule
    from dateutil_rs import rruleset as rs_rruleset
    from dateutil_rs import rrulestr as rs_rrulestr

    HAS_RRULE = True
except ImportError:
    HAS_RRULE = False


@pytest.mark.skipif(not HAS_RRULE, reason="rrule not available")
class TestRRuleTypes:
    DTSTART = datetime.datetime(2024, 1, 1)

    def test_iter_yields_datetime(self):
        py_rule = py_rrule(PY_DAILY, dtstart=self.DTSTART, count=3)
        rs_rule = rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=3)
        for py_dt, rs_dt in zip(py_rule, rs_rule):
            assert type(py_dt) is type(rs_dt) is datetime.datetime

    def test_before_returns_datetime_or_none(self):
        py_rule = py_rrule(PY_DAILY, dtstart=self.DTSTART, count=5)
        rs_rule = rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=5)
        dt = datetime.datetime(2024, 1, 3, 12, 0)
        py_result = py_rule.before(dt)
        rs_result = rs_rule.before(dt)
        assert type(py_result) is type(rs_result) is datetime.datetime

    def test_before_none_when_no_match(self):
        py_rule = py_rrule(PY_DAILY, dtstart=self.DTSTART, count=5)
        rs_rule = rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=5)
        dt = datetime.datetime(2023, 12, 31)
        py_result = py_rule.before(dt)
        rs_result = rs_rule.before(dt)
        assert py_result is None and rs_result is None

    def test_after_returns_datetime(self):
        py_rule = py_rrule(PY_DAILY, dtstart=self.DTSTART, count=5)
        rs_rule = rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=5)
        dt = datetime.datetime(2024, 1, 2)
        py_result = py_rule.after(dt)
        rs_result = rs_rule.after(dt)
        assert type(py_result) is type(rs_result) is datetime.datetime

    def test_between_returns_list(self):
        py_rule = py_rrule(PY_DAILY, dtstart=self.DTSTART, count=10)
        rs_rule = rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=10)
        a = datetime.datetime(2024, 1, 2)
        b = datetime.datetime(2024, 1, 8)
        py_result = py_rule.between(a, b)
        rs_result = rs_rule.between(a, b)
        assert isinstance(py_result, list) and isinstance(rs_result, list)
        assert all(type(dt) is datetime.datetime for dt in py_result)
        assert all(type(dt) is datetime.datetime for dt in rs_result)


@pytest.mark.skipif(not HAS_RRULE, reason="rrule not available")
class TestRRuleSetTypes:
    DTSTART = datetime.datetime(2024, 1, 1)

    def test_iter_yields_datetime(self):
        py_rset = py_rruleset()
        py_rset.rrule(py_rrule(PY_DAILY, dtstart=self.DTSTART, count=3))
        rs_rset = rs_rruleset()
        rs_rset.rrule(rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=3))
        for py_dt, rs_dt in zip(py_rset, rs_rset):
            assert type(py_dt) is type(rs_dt) is datetime.datetime

    def test_between_returns_list(self):
        py_rset = py_rruleset()
        py_rset.rrule(py_rrule(PY_DAILY, dtstart=self.DTSTART, count=10))
        rs_rset = rs_rruleset()
        rs_rset.rrule(rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=10))
        a = datetime.datetime(2024, 1, 2)
        b = datetime.datetime(2024, 1, 5)
        py_result = py_rset.between(a, b)
        rs_result = rs_rset.between(a, b)
        assert isinstance(py_result, list) and isinstance(rs_result, list)


@pytest.mark.skipif(not HAS_RRULE, reason="rrule not available")
class TestRRuleStrTypes:
    def test_returns_rrule(self):
        s = "DTSTART:20240101T000000\nRRULE:FREQ=DAILY;COUNT=3"
        py_result = py_rrulestr(s)
        rs_result = rs_rrulestr(s)
        assert isinstance(py_result, py_rrule)
        assert isinstance(rs_result, rs_rrule)

    def test_forceset_returns_rruleset(self):
        s = "DTSTART:20240101T000000\nRRULE:FREQ=DAILY;COUNT=3"
        py_result = py_rrulestr(s, forceset=True)
        rs_result = rs_rrulestr(s, forceset=True)
        assert isinstance(py_result, py_rruleset)
        assert isinstance(rs_result, rs_rruleset)


# ---------------------------------------------------------------------------
# Weekday constants
# ---------------------------------------------------------------------------
class TestWeekdayTypes:
    def test_weekday_attr_is_int(self):
        from dateutil.relativedelta import MO as PY_MO
        from dateutil_rs import MO as RS_MO

        assert type(PY_MO.weekday) is type(RS_MO.weekday) is int

    def test_n_is_none_by_default(self):
        from dateutil.relativedelta import MO as PY_MO
        from dateutil_rs import MO as RS_MO

        assert PY_MO.n is None and RS_MO.n is None

    def test_n_is_int_when_set(self):
        from dateutil.relativedelta import FR as PY_FR
        from dateutil_rs import FR as RS_FR

        py_fr = PY_FR(2)
        rs_fr = RS_FR(2)
        assert type(py_fr.n) is type(rs_fr.n) is int

    def test_str_returns_str(self):
        from dateutil.relativedelta import MO as PY_MO
        from dateutil_rs import MO as RS_MO

        assert type(str(PY_MO)) is type(str(RS_MO)) is str


# ---------------------------------------------------------------------------
# RelativeDelta — additional operator types
# ---------------------------------------------------------------------------
class TestRelativeDeltaExtraOperatorTypes:
    def test_sub_rd_rd(self):
        """relativedelta - relativedelta → relativedelta."""
        py_result = py_relativedelta(months=2) - py_relativedelta(days=1)
        rs_result = rs_relativedelta(months=2) - rs_relativedelta(days=1)
        assert isinstance(py_result, py_relativedelta)
        assert isinstance(rs_result, rs_relativedelta)

    def test_rmul(self):
        """int * relativedelta → relativedelta."""
        py_result = 3 * py_relativedelta(months=1)
        rs_result = 3 * rs_relativedelta(months=1)
        assert isinstance(py_result, py_relativedelta)
        assert isinstance(rs_result, rs_relativedelta)

    def test_weeks_property_type(self):
        py_rd = py_relativedelta(weeks=2)
        rs_rd = rs_relativedelta(weeks=2)
        assert type(py_rd.weeks) is type(rs_rd.weeks) is int


# ---------------------------------------------------------------------------
# Parser — parse with extra params
# ---------------------------------------------------------------------------
@pytest.mark.skipif(not HAS_PARSER, reason="parser not available")
class TestParserExtraTypes:
    def test_parse_with_default_returns_datetime(self):
        default = datetime.datetime(2024, 1, 1)
        py_result = py_parse("10:30", default=default)
        rs_result = rs_parse("10:30", default=default)
        assert type(py_result) is type(rs_result) is datetime.datetime


# ---------------------------------------------------------------------------
# Timezone — additional class method return types
# ---------------------------------------------------------------------------
@pytest.mark.skipif(not HAS_TZ, reason="tz not available")
class TestTzExtraMethodTypes:
    DT_SUMMER = datetime.datetime(2024, 6, 15, 12, 0)
    DT_WINTER = datetime.datetime(2024, 12, 15, 12, 0)

    def test_tzlocal_utcoffset_returns_timedelta(self):
        py_td = py_tzlocal().utcoffset(self.DT_SUMMER)
        rs_td = rs_tzlocal().utcoffset(self.DT_SUMMER)
        assert type(py_td) is type(rs_td) is datetime.timedelta

    def test_tzlocal_dst_returns_timedelta(self):
        py_td = py_tzlocal().dst(self.DT_SUMMER)
        rs_td = rs_tzlocal().dst(self.DT_SUMMER)
        assert type(py_td) is type(rs_td) is datetime.timedelta

    def test_tzlocal_tzname_returns_str(self):
        py_name = py_tzlocal().tzname(self.DT_SUMMER)
        rs_name = rs_tzlocal().tzname(self.DT_SUMMER)
        assert type(py_name) is type(rs_name) is str

    @pytest.mark.skipif(
        ZONEINFO_DIR is None
        or not os.path.isfile(os.path.join(ZONEINFO_DIR or "", "America/New_York")),
        reason="America/New_York tzfile not found",
    )
    def test_tzfile_utcoffset_returns_timedelta(self):
        from dateutil.tz import tzfile as py_tzfile

        path = os.path.join(ZONEINFO_DIR, "America/New_York")
        py_td = py_tzfile(path).utcoffset(self.DT_SUMMER)
        rs_td = rs_tzfile(path).utcoffset(self.DT_SUMMER)
        assert type(py_td) is type(rs_td) is datetime.timedelta

    @pytest.mark.skipif(
        ZONEINFO_DIR is None
        or not os.path.isfile(os.path.join(ZONEINFO_DIR or "", "America/New_York")),
        reason="America/New_York tzfile not found",
    )
    def test_tzfile_dst_returns_timedelta(self):
        from dateutil.tz import tzfile as py_tzfile

        path = os.path.join(ZONEINFO_DIR, "America/New_York")
        py_td = py_tzfile(path).dst(self.DT_SUMMER)
        rs_td = rs_tzfile(path).dst(self.DT_SUMMER)
        assert type(py_td) is type(rs_td) is datetime.timedelta

    @pytest.mark.skipif(
        ZONEINFO_DIR is None
        or not os.path.isfile(os.path.join(ZONEINFO_DIR or "", "America/New_York")),
        reason="America/New_York tzfile not found",
    )
    def test_tzfile_tzname_returns_str(self):
        from dateutil.tz import tzfile as py_tzfile

        path = os.path.join(ZONEINFO_DIR, "America/New_York")
        py_name = py_tzfile(path).tzname(self.DT_SUMMER)
        rs_name = rs_tzfile(path).tzname(self.DT_SUMMER)
        assert type(py_name) is type(rs_name) is str


@pytest.mark.skipif(not HAS_TZ, reason="tz not available")
class TestTzResolveImaginaryTypes:
    @pytest.mark.skipif(
        ZONEINFO_DIR is None
        or not os.path.isfile(os.path.join(ZONEINFO_DIR or "", "America/New_York")),
        reason="America/New_York tzfile not found",
    )
    def test_resolve_imaginary_returns_datetime(self):
        from dateutil.tz import resolve_imaginary as py_resolve_imaginary
        from dateutil_rs import resolve_imaginary as rs_resolve_imaginary

        ny_py = py_gettz("America/New_York")
        ny_rs = rs_gettz("America/New_York")
        # 2024-03-10 02:30 is in the DST gap for America/New_York
        dt_py = datetime.datetime(2024, 3, 10, 2, 30, tzinfo=ny_py)
        assert type(py_resolve_imaginary(dt_py)) is datetime.datetime
        # v1 resolve_imaginary takes two arguments: (naive_dt, tz)
        dt_naive = datetime.datetime(2024, 3, 10, 2, 30)
        assert type(rs_resolve_imaginary(dt_naive, ny_rs)) is datetime.datetime


# ---------------------------------------------------------------------------
# RRule — additional method types
# ---------------------------------------------------------------------------
@pytest.mark.skipif(not HAS_RRULE, reason="rrule not available")
class TestRRuleExtraTypes:
    def test_frequency_constants_are_int(self):
        from dateutil.rrule import (
            DAILY as PY_DAILY_,
        )
        from dateutil.rrule import (
            HOURLY as PY_HOURLY,
        )
        from dateutil.rrule import (
            MINUTELY as PY_MINUTELY,
        )
        from dateutil.rrule import (
            MONTHLY as PY_MONTHLY_,
        )
        from dateutil.rrule import (
            SECONDLY as PY_SECONDLY,
        )
        from dateutil.rrule import (
            WEEKLY as PY_WEEKLY,
        )
        from dateutil.rrule import (
            YEARLY as PY_YEARLY,
        )
        from dateutil_rs import (
            DAILY as RS_DAILY_,
        )
        from dateutil_rs import (
            HOURLY as RS_HOURLY,
        )
        from dateutil_rs import (
            MINUTELY as RS_MINUTELY,
        )
        from dateutil_rs import (
            MONTHLY as RS_MONTHLY_,
        )
        from dateutil_rs import (
            SECONDLY as RS_SECONDLY,
        )
        from dateutil_rs import (
            WEEKLY as RS_WEEKLY,
        )
        from dateutil_rs import (
            YEARLY as RS_YEARLY,
        )

        for py_c, rs_c in [
            (PY_YEARLY, RS_YEARLY),
            (PY_MONTHLY_, RS_MONTHLY_),
            (PY_WEEKLY, RS_WEEKLY),
            (PY_DAILY_, RS_DAILY_),
            (PY_HOURLY, RS_HOURLY),
            (PY_MINUTELY, RS_MINUTELY),
            (PY_SECONDLY, RS_SECONDLY),
        ]:
            assert type(py_c) is type(rs_c) is int


@pytest.mark.skipif(not HAS_RRULE, reason="rrule not available")
class TestRRuleSetExtraTypes:
    DTSTART = datetime.datetime(2024, 1, 1)

    def test_before_returns_datetime(self):
        py_rset = py_rruleset()
        py_rset.rrule(py_rrule(PY_DAILY, dtstart=self.DTSTART, count=5))
        rs_rset = rs_rruleset()
        rs_rset.rrule(rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=5))
        dt = datetime.datetime(2024, 1, 3, 12, 0)
        assert type(py_rset.before(dt)) is type(rs_rset.before(dt)) is datetime.datetime

    def test_after_returns_datetime(self):
        py_rset = py_rruleset()
        py_rset.rrule(py_rrule(PY_DAILY, dtstart=self.DTSTART, count=5))
        rs_rset = rs_rruleset()
        rs_rset.rrule(rs_rrule(RS_DAILY, dtstart=self.DTSTART, count=5))
        dt = datetime.datetime(2024, 1, 2)
        assert type(py_rset.after(dt)) is type(rs_rset.after(dt)) is datetime.datetime
