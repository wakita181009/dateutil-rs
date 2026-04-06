"""Tests for dateutil_rs PyO3 bindings.

These test Rust-specific behavior that the reference Python tests don't cover:
- Python type conversions (NaiveDate → datetime.date, etc.)
- Error types (ValueError instead of PanicException)
- Timezone-aware datetime rejection
- Submodule import paths
- Weekday PyO3 bindings (not in reference tests)

For algorithm correctness (easter dates, within_delta logic), use:
  pytest tests/test_easter.py --rust
"""

from datetime import date, datetime, timedelta, timezone

import pytest

dateutil_rs = pytest.importorskip("dateutil_rs")


class TestEasterBindings:
    def test_returns_date_not_datetime(self):
        from dateutil_rs.easter import easter

        result = easter(2024)
        assert isinstance(result, date)
        assert not isinstance(result, datetime)

    def test_invalid_method_raises_valueerror(self):
        from dateutil_rs.easter import easter

        with pytest.raises(ValueError, match="invalid method"):
            easter(2024, 4)

    def test_invalid_year_raises_valueerror(self):
        from dateutil_rs.easter import easter

        with pytest.raises(ValueError, match="invalid year"):
            easter(0)

    def test_constants_are_ints(self):
        assert dateutil_rs.EASTER_JULIAN == 1
        assert dateutil_rs.EASTER_ORTHODOX == 2
        assert dateutil_rs.EASTER_WESTERN == 3


class TestWeekday:
    def test_constants(self):
        assert str(dateutil_rs.MO) == "MO"
        assert str(dateutil_rs.SU) == "SU"

    def test_with_n(self):
        assert str(dateutil_rs.MO(1)) == "MO(+1)"
        assert str(dateutil_rs.FR(-1)) == "FR(-1)"

    def test_call_none_clears_n(self):
        assert str(dateutil_rs.MO(1)(None)) == "MO"

    def test_equality(self):
        assert dateutil_rs.weekday(0) == dateutil_rs.MO
        assert dateutil_rs.weekday(0, 1) == dateutil_rs.MO(1)
        assert dateutil_rs.MO != dateutil_rs.TU

    def test_hashable(self):
        s = {dateutil_rs.MO, dateutil_rs.MO, dateutil_rs.TU}
        assert len(s) == 2

    def test_invalid_raises_valueerror(self):
        with pytest.raises(ValueError, match=r"weekday must be 0..=6"):
            dateutil_rs.weekday(7)

    def test_getters(self):
        w = dateutil_rs.MO(2)
        assert w.weekday == 0
        assert w.n == 2


class TestWithinDeltaBindings:
    def test_rejects_aware_datetime(self):
        d1 = datetime(2024, 1, 1, tzinfo=timezone.utc)
        d2 = datetime(2024, 1, 2, tzinfo=timezone.utc)
        with pytest.raises(TypeError):
            dateutil_rs.within_delta(d1, d2, timedelta(days=1))


class TestRelativeDeltaBindings:
    def test_construction_simple(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(months=1)
        assert rd.months == 1
        assert rd.years == 0

    def test_construction_complex(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(years=1, months=2, days=3, hours=4, minutes=5, seconds=6)
        assert rd.years == 1
        assert rd.months == 2
        assert rd.days == 3.0

    def test_add_to_datetime(self):
        from dateutil_rs.relativedelta import relativedelta

        dt = datetime(2024, 1, 15, 14, 30, 0)
        result = dt + relativedelta(months=1)
        assert result == datetime(2024, 2, 15, 14, 30, 0)

    def test_month_end_clamping(self):
        from dateutil_rs.relativedelta import relativedelta

        result = datetime(2024, 1, 31) + relativedelta(months=1)
        assert result == datetime(2024, 2, 29)  # 2024 is leap year

    def test_add_to_date(self):
        from dateutil_rs.relativedelta import relativedelta

        result = date(2024, 1, 15) + relativedelta(months=1)
        assert result == date(2024, 2, 15)

    def test_add_date_with_time_returns_datetime(self):
        from dateutil_rs.relativedelta import relativedelta

        result = date(2024, 1, 15) + relativedelta(hours=5)
        assert isinstance(result, datetime)

    def test_diff_datetimes(self):
        from dateutil_rs.relativedelta import relativedelta

        dt1 = datetime(2024, 3, 15, 10, 0, 0)
        dt2 = datetime(2024, 1, 15, 10, 0, 0)
        rd = relativedelta(dt1=dt1, dt2=dt2)
        assert rd.months == 2
        assert rd.years == 0

    def test_diff_dates(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(dt1=date(2025, 1, 1), dt2=date(2024, 1, 1))
        assert rd.years == 1

    def test_weekday_positive(self):
        from dateutil_rs.relativedelta import MO, relativedelta

        # 2024-01-16 is Tuesday -> next Monday is 2024-01-22
        result = datetime(2024, 1, 16) + relativedelta(weekday=MO(+1))
        assert result == datetime(2024, 1, 22)

    def test_weekday_negative(self):
        from dateutil_rs.relativedelta import FR, relativedelta

        # Last Friday in Sep 2003: day=31 clamps to 30 (Sep has 30 days), then FR(-1)
        result = date(2003, 9, 17) + relativedelta(day=31, weekday=FR(-1))
        assert result == date(2003, 9, 26)

    def test_weekday_noop_when_already_on_day(self):
        from dateutil_rs.relativedelta import MO, relativedelta

        # 2024-01-15 is Monday -> MO(+1) is no-op
        dt = datetime(2024, 1, 15)
        assert dt + relativedelta(weekday=MO(+1)) == dt

    def test_subtract(self):
        from dateutil_rs.relativedelta import relativedelta

        result = datetime(2024, 3, 15) - relativedelta(months=2)
        assert result == datetime(2024, 1, 15)

    def test_multiply(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(months=1, days=5)
        result = rd * 3
        assert result.months == 3
        assert result.days == 15.0

    def test_negation(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(months=1, days=5)
        neg = -rd
        assert neg.months == -1
        assert neg.days == -5.0

    def test_bool_false_when_empty(self):
        from dateutil_rs.relativedelta import relativedelta

        assert not relativedelta()

    def test_bool_true_when_set(self):
        from dateutil_rs.relativedelta import relativedelta

        assert relativedelta(months=1)

    def test_equality(self):
        from dateutil_rs.relativedelta import relativedelta

        assert relativedelta(months=1) == relativedelta(months=1)
        assert relativedelta(months=1) != relativedelta(months=2)

    def test_eq_returns_notimplemented_for_other_types(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(months=1)
        assert rd.__eq__(42) is NotImplemented

    def test_repr(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(months=1, days=5)
        assert "months=+1" in repr(rd)
        assert "days=+5" in repr(rd)

    def test_non_integer_years_raises(self):
        from dateutil_rs.relativedelta import relativedelta

        with pytest.raises(ValueError, match="Non-integer"):
            relativedelta(years=1.5)

    def test_weekday_int_form(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(weekday=0)  # 0 = Monday
        assert rd.weekday.weekday == 0

    def test_absolute_fields(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(year=2025, month=6, day=15)
        result = datetime(2024, 1, 1) + rd
        assert result == datetime(2025, 6, 15)

    def test_fix_cascade(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(minutes=90)
        assert rd.hours == 1.0
        assert rd.minutes == 30.0

    def test_weeks_property(self):
        from dateutil_rs.relativedelta import relativedelta

        rd = relativedelta(days=14)
        assert rd.weeks == 2.0


class TestSubmoduleImports:
    def test_easter_module(self):
        from dateutil_rs.easter import EASTER_WESTERN, easter

        assert easter(2024, EASTER_WESTERN) == date(2024, 3, 31)

    def test_common_module(self):
        from dateutil_rs.common import MO, weekday

        assert str(MO) == "MO"
        assert str(weekday(0, 1)) == "MO(+1)"

    def test_relativedelta_module(self):
        from dateutil_rs.relativedelta import FR, MO, relativedelta

        assert str(MO) == "MO"
        rd = relativedelta(months=1)
        assert rd.months == 1
        assert str(FR(-1)) == "FR(-1)"

    def test_utils_module(self):
        from dateutil_rs.utils import within_delta

        assert within_delta(
            datetime(2024, 1, 1), datetime(2024, 1, 1), timedelta(seconds=0)
        )
