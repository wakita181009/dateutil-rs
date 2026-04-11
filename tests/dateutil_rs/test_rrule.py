"""Tests for dateutil_rs.rrule module (rrule, rruleset, rrulestr)."""

from datetime import datetime, timedelta

import pytest

from dateutil_rs import (
    DAILY,
    FR,
    HOURLY,
    MINUTELY,
    MO,
    MONTHLY,
    SA,
    SECONDLY,
    SU,
    TH,
    TU,
    WE,
    WEEKLY,
    YEARLY,
    rrule,
    rruleset,
    rrulestr,
)

# ============================================================================
# Frequency constants
# ============================================================================


class TestFrequencyConstants:
    def test_values(self):
        assert YEARLY == 0
        assert MONTHLY == 1
        assert WEEKLY == 2
        assert DAILY == 3
        assert HOURLY == 4
        assert MINUTELY == 5
        assert SECONDLY == 6

    def test_ordering(self):
        assert YEARLY < MONTHLY < WEEKLY < DAILY < HOURLY < MINUTELY < SECONDLY


# ============================================================================
# rrule — Basic generation
# ============================================================================


class TestRRuleDaily:
    def test_count(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), count=5)
        result = r.all()
        assert len(result) == 5
        assert result[0] == datetime(2024, 1, 1)
        assert result[-1] == datetime(2024, 1, 5)

    def test_until(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), until=datetime(2024, 1, 5))
        result = r.all()
        assert len(result) == 5

    def test_interval(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), interval=2, count=3)
        result = r.all()
        assert result == [
            datetime(2024, 1, 1),
            datetime(2024, 1, 3),
            datetime(2024, 1, 5),
        ]


class TestRRuleWeekly:
    def test_basic(self):
        r = rrule(WEEKLY, dtstart=datetime(2024, 1, 1), count=4)
        result = r.all()
        assert all(
            (result[i + 1] - result[i]).days == 7 for i in range(len(result) - 1)
        )

    def test_byweekday(self):
        r = rrule(
            WEEKLY,
            dtstart=datetime(2024, 1, 1),
            count=5,
            byweekday=[MO, WE, FR],
        )
        result = r.all()
        assert all(d.weekday() in (0, 2, 4) for d in result)

    def test_wkst(self):
        # Start week on Sunday
        r = rrule(WEEKLY, dtstart=datetime(2024, 1, 1), count=3, wkst=SU.weekday)
        result = r.all()
        assert len(result) == 3


class TestRRuleMonthly:
    def test_basic(self):
        r = rrule(MONTHLY, dtstart=datetime(2024, 1, 15), count=3)
        result = r.all()
        assert result == [
            datetime(2024, 1, 15),
            datetime(2024, 2, 15),
            datetime(2024, 3, 15),
        ]

    def test_bymonthday(self):
        r = rrule(
            MONTHLY,
            dtstart=datetime(2024, 1, 1),
            count=6,
            bymonthday=[1, 15],
        )
        result = r.all()
        assert all(d.day in (1, 15) for d in result)

    def test_byweekday_nth(self):
        # Second Tuesday of each month
        r = rrule(
            MONTHLY,
            dtstart=datetime(2024, 1, 1),
            count=3,
            byweekday=TU(2),
        )
        result = r.all()
        for d in result:
            assert d.weekday() == 1  # Tuesday
            assert 8 <= d.day <= 14  # 2nd week

    def test_last_weekday(self):
        # Last Friday of each month
        r = rrule(
            MONTHLY,
            dtstart=datetime(2024, 1, 1),
            count=3,
            byweekday=FR(-1),
        )
        result = r.all()
        for d in result:
            assert d.weekday() == 4  # Friday

    def test_negative_bymonthday(self):
        # Last day of month
        r = rrule(
            MONTHLY,
            dtstart=datetime(2024, 1, 1),
            count=3,
            bymonthday=-1,
        )
        result = r.all()
        assert result[0] == datetime(2024, 1, 31)
        assert result[1] == datetime(2024, 2, 29)  # leap year
        assert result[2] == datetime(2024, 3, 31)


class TestRRuleYearly:
    def test_basic(self):
        r = rrule(YEARLY, dtstart=datetime(2024, 3, 15), count=3)
        result = r.all()
        assert result == [
            datetime(2024, 3, 15),
            datetime(2025, 3, 15),
            datetime(2026, 3, 15),
        ]

    def test_bymonth(self):
        r = rrule(
            YEARLY,
            dtstart=datetime(2024, 1, 1),
            count=4,
            bymonth=[3, 9],
        )
        result = r.all()
        assert all(d.month in (3, 9) for d in result)

    def test_byyearday(self):
        r = rrule(YEARLY, dtstart=datetime(2024, 1, 1), count=2, byyearday=1)
        result = r.all()
        assert result[0] == datetime(2024, 1, 1)
        assert result[1] == datetime(2025, 1, 1)

    def test_byweekno(self):
        r = rrule(
            YEARLY,
            dtstart=datetime(2024, 1, 1),
            count=3,
            byweekno=1,
            byweekday=MO,
        )
        result = r.all()
        assert len(result) == 3

    def test_byeaster(self):
        r = rrule(YEARLY, dtstart=datetime(2024, 1, 1), count=3, byeaster=0)
        result = r.all()
        assert len(result) == 3

    def test_byeaster_offset(self):
        # Good Friday = Easter - 2
        r = rrule(YEARLY, dtstart=datetime(2024, 1, 1), count=1, byeaster=-2)
        result = r.all()
        assert result[0] == datetime(2024, 3, 29)


class TestRRuleHourly:
    def test_basic(self):
        r = rrule(HOURLY, dtstart=datetime(2024, 1, 1, 0, 0), count=4)
        result = r.all()
        assert result == [
            datetime(2024, 1, 1, 0, 0),
            datetime(2024, 1, 1, 1, 0),
            datetime(2024, 1, 1, 2, 0),
            datetime(2024, 1, 1, 3, 0),
        ]

    def test_byhour(self):
        r = rrule(
            HOURLY,
            dtstart=datetime(2024, 1, 1),
            count=3,
            byhour=[9, 17],
        )
        result = r.all()
        assert all(d.hour in (9, 17) for d in result)


class TestRRuleMinutely:
    def test_basic(self):
        r = rrule(MINUTELY, dtstart=datetime(2024, 1, 1, 0, 0), count=3, interval=30)
        result = r.all()
        assert result == [
            datetime(2024, 1, 1, 0, 0),
            datetime(2024, 1, 1, 0, 30),
            datetime(2024, 1, 1, 1, 0),
        ]


class TestRRuleSecondly:
    def test_basic(self):
        r = rrule(SECONDLY, dtstart=datetime(2024, 1, 1, 0, 0), count=3, interval=30)
        result = r.all()
        assert result == [
            datetime(2024, 1, 1, 0, 0, 0),
            datetime(2024, 1, 1, 0, 0, 30),
            datetime(2024, 1, 1, 0, 1, 0),
        ]


class TestRRuleBysetpos:
    def test_last_weekday_of_month(self):
        # Last workday of each month
        r = rrule(
            MONTHLY,
            dtstart=datetime(2024, 1, 1),
            count=3,
            byweekday=[MO, TU, WE, TH, FR],
            bysetpos=-1,
        )
        result = r.all()
        assert result[0] == datetime(2024, 1, 31)  # Wed
        assert result[1] == datetime(2024, 2, 29)  # Thu (leap year)
        assert result[2] == datetime(2024, 3, 29)  # Fri

    def test_first_weekday_of_month(self):
        r = rrule(
            MONTHLY,
            dtstart=datetime(2024, 1, 1),
            count=3,
            byweekday=[MO, TU, WE, TH, FR],
            bysetpos=1,
        )
        result = r.all()
        assert result[0] == datetime(2024, 1, 1)  # Mon
        assert result[1] == datetime(2024, 2, 1)  # Thu
        assert result[2] == datetime(2024, 3, 1)  # Fri


# ============================================================================
# rrule — Properties
# ============================================================================


class TestRRuleProperties:
    def test_freq(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), count=1)
        assert r.freq == DAILY

    def test_dtstart(self):
        dt = datetime(2024, 3, 15, 10, 30)
        r = rrule(DAILY, dtstart=dt, count=1)
        assert r.dtstart == dt

    def test_interval(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), interval=3, count=1)
        assert r.interval == 3

    def test_count_property(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), count=10)
        assert r._count == 10

    def test_until_property(self):
        until = datetime(2024, 12, 31)
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), until=until)
        assert r.until == until

    def test_wkst(self):
        r = rrule(WEEKLY, dtstart=datetime(2024, 1, 1), count=1, wkst=6)
        assert r.wkst == 6


# ============================================================================
# rrule — Querying: before, after, between, count, contains
# ============================================================================


class TestRRuleQuerying:
    @pytest.fixture
    def daily_jan(self):
        return rrule(DAILY, dtstart=datetime(2024, 1, 1), count=31)

    def test_before(self, daily_jan):
        result = daily_jan.before(datetime(2024, 1, 15))
        assert result == datetime(2024, 1, 14)

    def test_before_inc(self, daily_jan):
        result = daily_jan.before(datetime(2024, 1, 15), inc=True)
        assert result == datetime(2024, 1, 15)

    def test_before_no_match(self, daily_jan):
        result = daily_jan.before(datetime(2023, 12, 31))
        assert result is None

    def test_after(self, daily_jan):
        result = daily_jan.after(datetime(2024, 1, 15))
        assert result == datetime(2024, 1, 16)

    def test_after_inc(self, daily_jan):
        result = daily_jan.after(datetime(2024, 1, 15), inc=True)
        assert result == datetime(2024, 1, 15)

    def test_after_no_match(self, daily_jan):
        result = daily_jan.after(datetime(2024, 2, 1))
        assert result is None

    def test_between(self, daily_jan):
        result = daily_jan.between(datetime(2024, 1, 10), datetime(2024, 1, 15))
        assert len(result) == 4  # 11, 12, 13, 14

    def test_between_inc(self, daily_jan):
        result = daily_jan.between(
            datetime(2024, 1, 10), datetime(2024, 1, 15), inc=True
        )
        assert len(result) == 6  # 10, 11, 12, 13, 14, 15

    def test_count(self, daily_jan):
        assert daily_jan.count() == 31

    def test_contains(self, daily_jan):
        assert datetime(2024, 1, 15) in daily_jan
        assert datetime(2024, 2, 1) not in daily_jan


# ============================================================================
# rrule — Indexing and slicing
# ============================================================================


class TestRRuleIndexing:
    @pytest.fixture
    def daily_10(self):
        return rrule(DAILY, dtstart=datetime(2024, 1, 1), count=10)

    def test_getitem_positive(self, daily_10):
        assert daily_10[0] == datetime(2024, 1, 1)
        assert daily_10[4] == datetime(2024, 1, 5)

    def test_getitem_negative(self, daily_10):
        assert daily_10[-1] == datetime(2024, 1, 10)
        assert daily_10[-2] == datetime(2024, 1, 9)

    def test_getitem_out_of_range(self, daily_10):
        with pytest.raises(IndexError):
            daily_10[10]

    def test_slice(self, daily_10):
        result = daily_10[2:5]
        assert result == [
            datetime(2024, 1, 3),
            datetime(2024, 1, 4),
            datetime(2024, 1, 5),
        ]

    def test_slice_with_step(self, daily_10):
        result = daily_10[0:6:2]
        assert result == [
            datetime(2024, 1, 1),
            datetime(2024, 1, 3),
            datetime(2024, 1, 5),
        ]

    def test_slice_negative(self, daily_10):
        result = daily_10[-3:]
        assert result == [
            datetime(2024, 1, 8),
            datetime(2024, 1, 9),
            datetime(2024, 1, 10),
        ]


# ============================================================================
# rrule — Iteration
# ============================================================================


class TestRRuleIteration:
    def test_iter(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), count=3)
        result = list(r)
        assert result == [
            datetime(2024, 1, 1),
            datetime(2024, 1, 2),
            datetime(2024, 1, 3),
        ]

    def test_iter_matches_all(self):
        r = rrule(MONTHLY, dtstart=datetime(2024, 1, 15), count=5)
        assert list(r) == r.all()


# ============================================================================
# rrule — repr/str
# ============================================================================


class TestRRuleRepr:
    def test_repr_contains_rrule(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), count=3)
        assert "RRULE" in repr(r) or "rrule" in repr(r).lower()

    def test_str(self):
        r = rrule(DAILY, dtstart=datetime(2024, 1, 1), count=3)
        s = str(r)
        assert "FREQ=DAILY" in s


# ============================================================================
# rruleset
# ============================================================================


class TestRRuleSet:
    def test_single_rrule(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=5))
        result = rs.all()
        assert len(result) == 5

    def test_multiple_rrules(self):
        rs = rruleset()
        rs.rrule(rrule(MONTHLY, dtstart=datetime(2024, 1, 1), count=3))
        rs.rrule(rrule(MONTHLY, dtstart=datetime(2024, 1, 15), count=3))
        result = rs.all()
        assert len(result) == 6

    def test_rdate(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=3))
        rs.rdate(datetime(2024, 6, 15))
        result = rs.all()
        assert datetime(2024, 6, 15) in result
        assert len(result) == 4

    def test_exdate(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=5))
        rs.exdate(datetime(2024, 1, 3))
        result = rs.all()
        assert datetime(2024, 1, 3) not in result
        assert len(result) == 4

    def test_exrule(self):
        rs = rruleset()
        # Every day in Jan
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=7))
        # Exclude weekends
        rs.exrule(
            rrule(WEEKLY, dtstart=datetime(2024, 1, 6), byweekday=[SA, SU], count=2)
        )
        result = rs.all()
        assert datetime(2024, 1, 6) not in result  # Saturday
        assert datetime(2024, 1, 7) not in result  # Sunday

    def test_before(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=10))
        result = rs.before(datetime(2024, 1, 5))
        assert result == datetime(2024, 1, 4)

    def test_after(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=10))
        result = rs.after(datetime(2024, 1, 5))
        assert result == datetime(2024, 1, 6)

    def test_between(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=10))
        result = rs.between(datetime(2024, 1, 3), datetime(2024, 1, 7))
        assert len(result) == 3  # 4, 5, 6

    def test_count(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=10))
        rs.exdate(datetime(2024, 1, 5))
        assert rs.count() == 9

    def test_contains(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=5))
        assert datetime(2024, 1, 3) in rs
        assert datetime(2024, 2, 1) not in rs

    def test_getitem(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=5))
        assert rs[0] == datetime(2024, 1, 1)
        assert rs[-1] == datetime(2024, 1, 5)

    def test_iter(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=3))
        assert list(rs) == [
            datetime(2024, 1, 1),
            datetime(2024, 1, 2),
            datetime(2024, 1, 3),
        ]

    def test_deduplication(self):
        rs = rruleset()
        rs.rrule(rrule(DAILY, dtstart=datetime(2024, 1, 1), count=3))
        rs.rdate(datetime(2024, 1, 2))  # duplicate
        result = rs.all()
        assert len(result) == 3  # no dup


# ============================================================================
# rrulestr
# ============================================================================


class TestRRuleStr:
    def test_basic(self):
        r = rrulestr("RRULE:FREQ=DAILY;COUNT=5", dtstart=datetime(2024, 1, 1))
        result = r.all()
        assert len(result) == 5

    def test_with_dtstart_in_string(self):
        s = "DTSTART:20240101T000000\nRRULE:FREQ=DAILY;COUNT=3"
        r = rrulestr(s)
        result = r.all()
        assert result[0] == datetime(2024, 1, 1)
        assert len(result) == 3

    def test_weekly_byday(self):
        r = rrulestr(
            "RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;COUNT=6",
            dtstart=datetime(2024, 1, 1),
        )
        result = r.all()
        assert len(result) == 6
        assert all(d.weekday() in (0, 2, 4) for d in result)

    def test_monthly_bymonthday(self):
        r = rrulestr(
            "RRULE:FREQ=MONTHLY;BYMONTHDAY=15;COUNT=3",
            dtstart=datetime(2024, 1, 1),
        )
        result = r.all()
        assert all(d.day == 15 for d in result)

    def test_forceset_returns_rruleset(self):
        r = rrulestr(
            "RRULE:FREQ=DAILY;COUNT=3",
            dtstart=datetime(2024, 1, 1),
            forceset=True,
        )
        assert isinstance(r, rruleset)

    def test_with_exdate(self):
        s = "DTSTART:20240101T000000\nRRULE:FREQ=DAILY;COUNT=5\nEXDATE:20240103T000000"
        r = rrulestr(s, forceset=True)
        result = r.all()
        assert datetime(2024, 1, 3) not in result

    def test_until(self):
        r = rrulestr(
            "RRULE:FREQ=DAILY;UNTIL=20240105T000000",
            dtstart=datetime(2024, 1, 1),
        )
        result = r.all()
        assert result[-1] == datetime(2024, 1, 5)

    def test_interval(self):
        r = rrulestr(
            "RRULE:FREQ=DAILY;INTERVAL=2;COUNT=3",
            dtstart=datetime(2024, 1, 1),
        )
        result = r.all()
        assert result == [
            datetime(2024, 1, 1),
            datetime(2024, 1, 3),
            datetime(2024, 1, 5),
        ]

    def test_unfold(self):
        # RFC 5545 line folding
        s = "DTSTART:20240101T000000\r\nRRULE:FREQ=DAI\r\n LY;COUNT=3"
        r = rrulestr(s, unfold=True)
        assert len(r.all()) == 3

    def test_invalid_string(self):
        with pytest.raises(ValueError):
            rrulestr("not a valid rrule string")
