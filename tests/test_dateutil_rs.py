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
        result = dateutil_rs.easter(2024)
        assert isinstance(result, date)
        assert not isinstance(result, datetime)

    def test_invalid_method_raises_valueerror(self):
        with pytest.raises(ValueError, match="invalid method"):
            dateutil_rs.easter(2024, 4)

    def test_invalid_year_raises_valueerror(self):
        with pytest.raises(ValueError, match="invalid year"):
            dateutil_rs.easter(0)

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
        assert dateutil_rs.MO == dateutil_rs.weekday(0)
        assert dateutil_rs.MO(1) == dateutil_rs.weekday(0, 1)
        assert dateutil_rs.MO != dateutil_rs.TU

    def test_hashable(self):
        s = {dateutil_rs.MO, dateutil_rs.MO, dateutil_rs.TU}
        assert len(s) == 2

    def test_invalid_raises_valueerror(self):
        with pytest.raises(ValueError, match="weekday must be 0..=6"):
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


class TestSubmoduleImports:
    def test_easter_module(self):
        from dateutil_rs.easter import easter, EASTER_WESTERN
        assert easter(2024, EASTER_WESTERN) == date(2024, 3, 31)

    def test_common_module(self):
        from dateutil_rs.common import MO, weekday
        assert str(MO) == "MO"
        assert str(weekday(0, 1)) == "MO(+1)"

    def test_utils_module(self):
        from dateutil_rs.utils import within_delta
        assert within_delta(datetime(2024, 1, 1), datetime(2024, 1, 1), timedelta(seconds=0))
