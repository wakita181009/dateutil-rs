"""Tests for dateutil_rs weekday constants and class."""

import pytest

from dateutil_rs import FR, MO, SA, SU, TH, TU, WE, weekday


class TestWeekdayConstants:
    """MO through SU constants."""

    def test_all_seven(self):
        days = [MO, TU, WE, TH, FR, SA, SU]
        assert [d.weekday for d in days] == [0, 1, 2, 3, 4, 5, 6]

    def test_n_is_none(self):
        for d in (MO, TU, WE, TH, FR, SA, SU):
            assert d.n is None

    def test_str(self):
        assert str(MO) == "MO"
        assert str(TU) == "TU"
        assert str(WE) == "WE"
        assert str(TH) == "TH"
        assert str(FR) == "FR"
        assert str(SA) == "SA"
        assert str(SU) == "SU"

    def test_repr(self):
        assert repr(MO) == "MO"
        assert repr(SU) == "SU"


class TestWeekdayConstructor:
    """weekday(n) constructor."""

    def test_from_int(self):
        for i in range(7):
            w = weekday(i)
            assert w.weekday == i
            assert w.n is None

    def test_with_n(self):
        w = weekday(0, 2)
        assert w.weekday == 0
        assert w.n == 2

    def test_invalid_weekday_high(self):
        with pytest.raises(ValueError, match=r"must be 0\.\.=6"):
            weekday(7)

    def test_invalid_weekday_negative(self):
        with pytest.raises((ValueError, OverflowError)):
            weekday(-1)


class TestWeekdayCall:
    """weekday.__call__ for nth occurrence."""

    def test_positive_n(self):
        first_monday = MO(1)
        assert first_monday.weekday == 0
        assert first_monday.n == 1
        assert str(first_monday) == "MO(+1)"

    def test_negative_n(self):
        last_friday = FR(-1)
        assert last_friday.weekday == 4
        assert last_friday.n == -1
        assert str(last_friday) == "FR(-1)"

    def test_large_n(self):
        w = MO(5)
        assert w.n == 5

    def test_call_none_clears_n(self):
        w = MO(1)
        cleared = w(None)
        assert cleared.n is None
        assert str(cleared) == "MO"

    def test_chained_calls(self):
        w = MO(1)(2)(-1)(None)
        assert w.n is None
        assert w.weekday == 0

    def test_call_returns_new_instance(self):
        original = MO
        with_n = original(1)
        assert original.n is None
        assert with_n.n == 1


class TestWeekdayEquality:
    def test_equal_same_weekday(self):
        assert weekday(0) == MO
        assert weekday(4) == FR

    def test_equal_with_n(self):
        assert weekday(0, 1) == MO(1)
        assert weekday(4, -1) == FR(-1)

    def test_not_equal_different_weekday(self):
        assert MO != TU

    def test_not_equal_different_n(self):
        assert MO(1) != MO(2)

    def test_not_equal_n_vs_none(self):
        assert MO(1) != MO


class TestWeekdayHash:
    def test_hashable(self):
        s = {MO, MO, TU}
        assert len(s) == 2

    def test_with_n_hashable(self):
        s = {MO(1), MO(1), MO(2)}
        assert len(s) == 2

    def test_dict_key(self):
        d = {MO: "Monday", FR: "Friday"}
        assert d[weekday(0)] == "Monday"
