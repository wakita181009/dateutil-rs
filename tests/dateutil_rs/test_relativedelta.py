"""Tests for dateutil_rs.relativedelta module."""

from datetime import date, datetime, timedelta, timezone

import pytest

from dateutil_rs import FR, MO, SA, SU, TH, TU, WE, relativedelta

# ============================================================================
# Construction
# ============================================================================


class TestConstruction:
    def test_empty(self):
        rd = relativedelta()
        assert rd.years == 0
        assert rd.months == 0
        assert rd.days == 0

    def test_all_relative_fields(self):
        rd = relativedelta(
            years=1,
            months=2,
            days=3,
            hours=4,
            minutes=5,
            seconds=6,
            microseconds=7,
        )
        assert rd.years == 1
        assert rd.months == 2
        assert rd.days == 3
        assert rd.hours == 4
        assert rd.minutes == 5
        assert rd.seconds == 6
        assert rd.microseconds == 7

    def test_absolute_fields(self):
        rd = relativedelta(year=2025, month=6, day=15, hour=10, minute=30)
        assert rd.year == 2025
        assert rd.month == 6
        assert rd.day == 15
        assert rd.hour == 10
        assert rd.minute == 30

    def test_absolute_fields_none_by_default(self):
        rd = relativedelta(months=1)
        assert rd.year is None
        assert rd.month is None
        assert rd.day is None

    def test_weeks(self):
        rd = relativedelta(weeks=2)
        assert rd.days == 14
        assert rd.weeks == 2

    def test_weekday(self):
        rd = relativedelta(weekday=MO)
        assert rd.weekday is not None
        assert rd.weekday.weekday == 0

    def test_weekday_with_n(self):
        rd = relativedelta(weekday=FR(-1))
        assert rd.weekday.weekday == 4
        assert rd.weekday.n == -1

    def test_leapdays(self):
        rd = relativedelta(leapdays=1)
        assert rd.leapdays == 1

    def test_yearday(self):
        # yearday=75 in a leap year (2024) = March 15
        rd = relativedelta(yearday=75)
        result = datetime(2024, 1, 1) + rd
        assert result.month == 3
        assert result.day == 15

    def test_nlyearday(self):
        # nlyearday=74 = March 15 (counts as if no leap day)
        rd = relativedelta(nlyearday=74)
        result = datetime(2024, 1, 1) + rd
        assert result.month == 3
        assert result.day == 15


class TestFromDiff:
    def test_datetimes(self):
        dt1 = datetime(2025, 6, 15, 14, 30)
        dt2 = datetime(2024, 1, 10, 10, 0)
        rd = relativedelta.from_diff(dt1, dt2)
        assert rd.years == 1
        assert rd.months == 5
        assert rd.days == 5
        assert rd.hours == 4
        assert rd.minutes == 30

    def test_dates(self):
        rd = relativedelta.from_diff(date(2025, 1, 1), date(2024, 1, 1))
        assert rd.years == 1
        assert rd.months == 0

    def test_same_datetime(self):
        dt = datetime(2024, 6, 15)
        rd = relativedelta.from_diff(dt, dt)
        assert not rd  # is_zero

    def test_reverse_order(self):
        dt1 = datetime(2024, 1, 1)
        dt2 = datetime(2025, 1, 1)
        rd = relativedelta.from_diff(dt1, dt2)
        assert rd.years == -1

    def test_roundtrip(self):
        dt1 = datetime(2024, 3, 15, 10, 30)
        dt2 = datetime(2023, 1, 1, 8, 0)
        rd = relativedelta.from_diff(dt1, dt2)
        assert dt2 + rd == dt1

    def test_constructor_form(self):
        dt1 = datetime(2025, 6, 15)
        dt2 = datetime(2024, 1, 10)
        rd = relativedelta(dt1, dt2)
        assert rd.years == 1
        assert rd.months == 5


# ============================================================================
# Normalization
# ============================================================================


class TestNormalization:
    def test_minutes_overflow(self):
        rd = relativedelta(minutes=90)
        assert rd.hours == 1
        assert rd.minutes == 30

    def test_seconds_overflow(self):
        rd = relativedelta(seconds=3661)
        assert rd.hours == 1
        assert rd.minutes == 1
        assert rd.seconds == 1

    def test_months_overflow(self):
        rd = relativedelta(months=25)
        assert rd.years == 2
        assert rd.months == 1

    def test_microseconds_overflow(self):
        rd = relativedelta(microseconds=1500000)
        assert rd.seconds == 1
        assert rd.microseconds == 500000

    def test_negative_normalization(self):
        rd = relativedelta(months=-14)
        assert rd.years == -1
        assert rd.months == -2


# ============================================================================
# Addition to datetime/date
# ============================================================================


class TestAddition:
    def test_add_months_to_datetime(self):
        result = datetime(2024, 1, 15, 14, 30) + relativedelta(months=1)
        assert result == datetime(2024, 2, 15, 14, 30)

    def test_add_to_date(self):
        result = date(2024, 1, 15) + relativedelta(months=1)
        assert result == date(2024, 2, 15)

    def test_date_plus_time_returns_datetime(self):
        result = date(2024, 1, 15) + relativedelta(hours=5)
        assert isinstance(result, datetime)
        assert result == datetime(2024, 1, 15, 5, 0)

    def test_month_end_clamping(self):
        result = datetime(2024, 1, 31) + relativedelta(months=1)
        assert result == datetime(2024, 2, 29)  # 2024 is leap year

    def test_month_end_clamping_non_leap(self):
        result = datetime(2023, 1, 31) + relativedelta(months=1)
        assert result == datetime(2023, 2, 28)

    def test_add_years_leap_day(self):
        result = date(2024, 2, 29) + relativedelta(years=1)
        assert result == date(2025, 2, 28)

    def test_absolute_year(self):
        result = datetime(2024, 6, 15) + relativedelta(year=2025)
        assert result == datetime(2025, 6, 15)

    def test_absolute_day(self):
        result = datetime(2024, 3, 15) + relativedelta(day=1)
        assert result == datetime(2024, 3, 1)

    def test_mixed_relative_and_absolute(self):
        # months=+1 then day=1 => first day of next month
        result = datetime(2024, 1, 15) + relativedelta(months=1, day=1)
        assert result == datetime(2024, 2, 1)

    def test_radd(self):
        rd = relativedelta(months=1)
        result = datetime(2024, 1, 15) + rd
        assert result == datetime(2024, 2, 15)

    def test_weekday_next(self):
        # 2024-01-16 (Tue) + weekday=FR(+1) => 2024-01-19 (Fri)
        result = datetime(2024, 1, 16) + relativedelta(weekday=FR(+1))
        assert result == datetime(2024, 1, 19)

    def test_weekday_noop(self):
        # 2024-01-15 (Mon) + weekday=MO(+1) => stays 2024-01-15
        dt = datetime(2024, 1, 15)
        assert dt + relativedelta(weekday=MO(+1)) == dt

    def test_weekday_last(self):
        # Last Friday in Sep 2003
        result = date(2003, 9, 17) + relativedelta(day=31, weekday=FR(-1))
        assert result == date(2003, 9, 26)


# ============================================================================
# Subtraction
# ============================================================================


class TestSubtraction:
    def test_sub_from_datetime(self):
        result = datetime(2024, 3, 15) - relativedelta(months=2)
        assert result == datetime(2024, 1, 15)

    def test_sub_from_date(self):
        result = date(2024, 3, 15) - relativedelta(months=2)
        assert result == date(2024, 1, 15)

    def test_sub_two_relativedeltas(self):
        rd1 = relativedelta(months=3, days=10)
        rd2 = relativedelta(months=1, days=5)
        rd = rd1 - rd2
        assert rd.months == 2
        assert rd.days == 5


# ============================================================================
# Arithmetic
# ============================================================================


class TestArithmetic:
    def test_add_two_relativedeltas(self):
        rd1 = relativedelta(months=1, days=5)
        rd2 = relativedelta(months=2, days=10)
        rd = rd1 + rd2
        assert rd.months == 3
        assert rd.days == 15

    def test_multiply(self):
        rd = relativedelta(months=1, days=5) * 3
        assert rd.months == 3
        assert rd.days == 15

    def test_rmul(self):
        rd = 3 * relativedelta(months=1, days=5)
        assert rd.months == 3
        assert rd.days == 15

    def test_divide(self):
        rd = relativedelta(months=6, days=10) / 2
        assert rd.months == 3
        assert rd.days == 5

    def test_negate(self):
        rd = -relativedelta(months=1, days=5)
        assert rd.months == -1
        assert rd.days == -5

    def test_abs(self):
        rd = abs(relativedelta(months=-1, days=-5))
        assert rd.months == 1
        assert rd.days == 5

    def test_add_timedelta(self):
        rd = relativedelta(months=1) + timedelta(days=5)
        assert rd.months == 1
        assert rd.days == 5


# ============================================================================
# Equality / Bool / Repr
# ============================================================================


class TestEquality:
    def test_equal(self):
        assert relativedelta(months=1) == relativedelta(months=1)

    def test_not_equal(self):
        assert relativedelta(months=1) != relativedelta(months=2)

    def test_notimplemented_for_other_types(self):
        assert relativedelta(months=1).__eq__(42) is NotImplemented

    def test_hash(self):
        s = {relativedelta(months=1), relativedelta(months=1)}
        assert len(s) == 1


class TestBool:
    def test_false_when_empty(self):
        assert not relativedelta()

    def test_true_when_relative(self):
        assert relativedelta(months=1)

    def test_true_when_absolute(self):
        assert relativedelta(year=2024)

    def test_true_when_weekday(self):
        assert relativedelta(weekday=MO)


class TestRepr:
    def test_simple(self):
        r = repr(relativedelta(months=1))
        assert "months=+1" in r

    def test_negative(self):
        r = repr(relativedelta(months=-1))
        assert "months=-1" in r

    def test_contains_relativedelta(self):
        r = repr(relativedelta(days=1))
        assert "relativedelta" in r

    def test_absolute_field(self):
        r = repr(relativedelta(year=2025))
        assert "year=2025" in r


class TestHasTime:
    def test_no_time(self):
        assert not relativedelta(months=1).has_time()

    def test_with_hours(self):
        assert relativedelta(hours=1).has_time()

    def test_with_absolute_hour(self):
        assert relativedelta(hour=10).has_time()


class TestIsZero:
    def test_zero(self):
        assert relativedelta().is_zero()

    def test_not_zero(self):
        assert not relativedelta(days=1).is_zero()


# ============================================================================
# Timezone-aware datetimes
# ============================================================================


class TestAwareDatetime:
    def test_preserves_utc(self):
        dt = datetime(2024, 1, 15, 12, 0, tzinfo=timezone.utc)
        result = dt + relativedelta(months=1)
        assert result.tzinfo is timezone.utc
        assert result == datetime(2024, 2, 15, 12, 0, tzinfo=timezone.utc)

    def test_preserves_fixed_offset(self):
        tz = timezone(timedelta(hours=9))
        dt = datetime(2024, 6, 15, 10, 0, tzinfo=tz)
        result = dt + relativedelta(days=10)
        assert result.utcoffset() == timedelta(hours=9)

    def test_diff_aware_datetimes(self):
        tz = timezone.utc
        dt1 = datetime(2024, 3, 15, 10, 0, tzinfo=tz)
        dt2 = datetime(2024, 1, 15, 10, 0, tzinfo=tz)
        rd = relativedelta.from_diff(dt1, dt2)
        assert rd.months == 2
