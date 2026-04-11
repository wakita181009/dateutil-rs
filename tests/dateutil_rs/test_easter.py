"""Tests for dateutil_rs.easter module."""

from datetime import date

import pytest

from dateutil_rs import EASTER_JULIAN, EASTER_ORTHODOX, EASTER_WESTERN, easter


class TestEasterWestern:
    """Western (Gregorian) Easter computation."""

    def test_known_dates(self):
        assert easter(2024) == date(2024, 3, 31)
        assert easter(2025) == date(2025, 4, 20)
        assert easter(2026) == date(2026, 4, 5)

    def test_explicit_method(self):
        assert easter(2024, EASTER_WESTERN) == date(2024, 3, 31)

    def test_earliest_possible(self):
        # March 22 is the earliest possible Easter
        assert easter(1818) == date(1818, 3, 22)

    def test_latest_possible(self):
        # April 25 is the latest possible Easter
        assert easter(1943) == date(1943, 4, 25)

    def test_leap_year(self):
        assert easter(2000) == date(2000, 4, 23)

    def test_returns_date_type(self):
        result = easter(2024)
        assert type(result) is date


class TestEasterJulian:
    """Julian calendar Easter computation."""

    def test_known_dates(self):
        assert easter(179, EASTER_JULIAN) == date(179, 4, 12)
        assert easter(2024, EASTER_JULIAN) == date(2024, 4, 22)

    def test_early_year(self):
        result = easter(1, EASTER_JULIAN)
        assert isinstance(result, date)


class TestEasterOrthodox:
    """Orthodox Easter computation (Julian method, Gregorian calendar)."""

    def test_known_dates(self):
        assert easter(2024, EASTER_ORTHODOX) == date(2024, 5, 5)
        assert easter(2025, EASTER_ORTHODOX) == date(2025, 4, 20)

    def test_differs_from_western(self):
        # Orthodox Easter often differs from Western
        assert easter(2024, EASTER_ORTHODOX) != easter(2024, EASTER_WESTERN)

    def test_same_as_western(self):
        # Sometimes they coincide
        assert easter(2025, EASTER_ORTHODOX) == easter(2025, EASTER_WESTERN)


class TestEasterConstants:
    def test_values(self):
        assert EASTER_JULIAN == 1
        assert EASTER_ORTHODOX == 2
        assert EASTER_WESTERN == 3


class TestEasterErrors:
    def test_invalid_method(self):
        with pytest.raises(ValueError, match="invalid method"):
            easter(2024, 0)

    def test_invalid_method_high(self):
        with pytest.raises(ValueError, match="invalid method"):
            easter(2024, 4)

    def test_year_zero(self):
        with pytest.raises(ValueError, match="invalid year"):
            easter(0)

    def test_negative_year(self):
        with pytest.raises(ValueError, match="invalid year"):
            easter(-1)
