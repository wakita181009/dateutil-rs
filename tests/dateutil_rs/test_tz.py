"""Tests for dateutil_rs.tz module."""

from datetime import datetime, timedelta, timezone

import pytest

from dateutil_rs import (
    datetime_ambiguous,
    datetime_exists,
    gettz,
    resolve_imaginary,
    tzfile,
    tzlocal,
    tzoffset,
    tzutc,
)

# ============================================================================
# tzutc
# ============================================================================


class TestTzutc:
    def test_utcoffset(self):
        tz = tzutc()
        assert tz.utcoffset(None) == timedelta(0)

    def test_dst(self):
        tz = tzutc()
        assert tz.dst(None) == timedelta(0)

    def test_tzname(self):
        tz = tzutc()
        assert tz.tzname(None) == "UTC"

    def test_is_ambiguous(self):
        tz = tzutc()
        dt = datetime(2024, 6, 15, 12, 0)
        assert not tz.is_ambiguous(dt)

    def test_repr(self):
        tz = tzutc()
        assert "tzutc" in repr(tz)

    def test_fromutc(self):
        tz = tzutc()
        dt = datetime(2024, 6, 15, 12, 0, tzinfo=tz)
        result = tz.fromutc(dt)
        assert result == dt

    def test_is_tzinfo(self):
        import datetime as dt_mod

        tz = tzutc()
        assert isinstance(tz, dt_mod.tzinfo)

    def test_attach_to_datetime(self):
        tz = tzutc()
        dt = datetime(2024, 6, 15, 12, 0, tzinfo=tz)
        assert dt.utcoffset() == timedelta(0)


# ============================================================================
# tzoffset
# ============================================================================


class TestTzoffset:
    def test_positive_offset(self):
        tz = tzoffset("JST", 9 * 3600)
        assert tz.utcoffset(None) == timedelta(hours=9)

    def test_negative_offset(self):
        tz = tzoffset("EST", -5 * 3600)
        assert tz.utcoffset(None) == timedelta(hours=-5)

    def test_zero_offset(self):
        tz = tzoffset("UTC", 0)
        assert tz.utcoffset(None) == timedelta(0)

    def test_dst_is_zero(self):
        tz = tzoffset("JST", 9 * 3600)
        assert tz.dst(None) == timedelta(0)

    def test_tzname(self):
        tz = tzoffset("JST", 9 * 3600)
        assert tz.tzname(None) == "JST"

    def test_tzname_none(self):
        tz = tzoffset(None, 3600)
        name = tz.tzname(None)
        assert name is not None  # should generate a name

    def test_is_ambiguous(self):
        tz = tzoffset("JST", 9 * 3600)
        dt = datetime(2024, 6, 15, 12, 0)
        assert not tz.is_ambiguous(dt)

    def test_repr(self):
        tz = tzoffset("JST", 9 * 3600)
        assert "tzoffset" in repr(tz)

    def test_fromutc(self):
        tz = tzoffset("JST", 9 * 3600)
        dt = datetime(2024, 6, 15, 3, 0, tzinfo=tz)
        result = tz.fromutc(dt)
        assert result.hour == 12

    def test_attach_to_datetime(self):
        tz = tzoffset("JST", 9 * 3600)
        dt = datetime(2024, 6, 15, 12, 0, tzinfo=tz)
        assert dt.utcoffset() == timedelta(hours=9)


# ============================================================================
# tzfile
# ============================================================================


class TestTzfile:
    @pytest.fixture
    def eastern(self):
        return tzfile("/usr/share/zoneinfo/US/Eastern")

    @pytest.fixture
    def tokyo(self):
        return tzfile("/usr/share/zoneinfo/Asia/Tokyo")

    def test_utcoffset_summer(self, eastern):
        # EDT: UTC-4
        dt = datetime(2024, 7, 15, 12, 0)
        assert eastern.utcoffset(dt) == timedelta(hours=-4)

    def test_utcoffset_winter(self, eastern):
        # EST: UTC-5
        dt = datetime(2024, 1, 15, 12, 0)
        assert eastern.utcoffset(dt) == timedelta(hours=-5)

    def test_dst_summer(self, eastern):
        dt = datetime(2024, 7, 15, 12, 0)
        assert eastern.dst(dt) == timedelta(hours=1)

    def test_dst_winter(self, eastern):
        dt = datetime(2024, 1, 15, 12, 0)
        assert eastern.dst(dt) == timedelta(0)

    def test_tzname_summer(self, eastern):
        dt = datetime(2024, 7, 15, 12, 0)
        assert eastern.tzname(dt) == "EDT"

    def test_tzname_winter(self, eastern):
        dt = datetime(2024, 1, 15, 12, 0)
        assert eastern.tzname(dt) == "EST"

    def test_no_dst_timezone(self, tokyo):
        dt = datetime(2024, 7, 15, 12, 0)
        assert tokyo.utcoffset(dt) == timedelta(hours=9)
        assert tokyo.dst(dt) == timedelta(0)
        assert tokyo.tzname(dt) == "JST"

    def test_is_ambiguous_fall_back(self, eastern):
        # Nov 3, 2024 1:30 AM is ambiguous (EDT->EST transition)
        dt = datetime(2024, 11, 3, 1, 30)
        assert eastern.is_ambiguous(dt)

    def test_not_ambiguous_normal(self, eastern):
        dt = datetime(2024, 6, 15, 12, 0)
        assert not eastern.is_ambiguous(dt)

    def test_repr(self, eastern):
        assert "tzfile" in repr(eastern)

    def test_invalid_path(self):
        with pytest.raises((ValueError, OSError)):
            tzfile("/nonexistent/timezone/file")

    def test_fromutc(self, eastern):
        dt = datetime(2024, 7, 15, 16, 0, tzinfo=eastern)
        result = eastern.fromutc(dt)
        assert result.hour == 12  # UTC-4 in summer


# ============================================================================
# tzlocal
# ============================================================================


class TestTzlocal:
    def test_construction(self):
        tz = tzlocal()
        assert tz is not None

    def test_utcoffset(self):
        tz = tzlocal()
        dt = datetime(2024, 6, 15, 12, 0)
        offset = tz.utcoffset(dt)
        assert isinstance(offset, timedelta)

    def test_dst(self):
        tz = tzlocal()
        dt = datetime(2024, 6, 15, 12, 0)
        dst = tz.dst(dt)
        assert isinstance(dst, timedelta)

    def test_tzname(self):
        tz = tzlocal()
        dt = datetime(2024, 6, 15, 12, 0)
        name = tz.tzname(dt)
        assert isinstance(name, str)
        assert len(name) > 0

    def test_repr(self):
        tz = tzlocal()
        assert "tzlocal" in repr(tz)


# ============================================================================
# gettz
# ============================================================================


class TestGettz:
    def test_utc(self):
        tz = gettz("UTC")
        assert tz.utcoffset(None) == timedelta(0)

    def test_iana_name(self):
        tz = gettz("US/Eastern")
        dt = datetime(2024, 1, 15, 12, 0)
        assert tz.utcoffset(dt) == timedelta(hours=-5)

    def test_asia_tokyo(self):
        tz = gettz("Asia/Tokyo")
        dt = datetime(2024, 6, 15, 12, 0)
        assert tz.utcoffset(dt) == timedelta(hours=9)

    def test_europe_london(self):
        tz = gettz("Europe/London")
        # BST in summer
        dt_summer = datetime(2024, 7, 15, 12, 0)
        assert tz.utcoffset(dt_summer) == timedelta(hours=1)
        # GMT in winter
        dt_winter = datetime(2024, 1, 15, 12, 0)
        assert tz.utcoffset(dt_winter) == timedelta(0)

    def test_none_returns_local(self):
        tz = gettz(None)
        assert tz is not None

    def test_invalid_name(self):
        with pytest.raises((ValueError, KeyError)):
            gettz("Not/A/Timezone")


# ============================================================================
# datetime_exists
# ============================================================================


class TestDatetimeExists:
    def test_normal_time_exists(self):
        tz = gettz("US/Eastern")
        dt = datetime(2024, 6, 15, 12, 0)
        assert datetime_exists(dt, tz)

    def test_spring_forward_gap(self):
        # March 10, 2024 2:30 AM doesn't exist (spring forward)
        tz = gettz("US/Eastern")
        dt = datetime(2024, 3, 10, 2, 30)
        assert not datetime_exists(dt, tz)

    def test_before_gap(self):
        tz = gettz("US/Eastern")
        dt = datetime(2024, 3, 10, 1, 30)
        assert datetime_exists(dt, tz)

    def test_after_gap(self):
        tz = gettz("US/Eastern")
        dt = datetime(2024, 3, 10, 3, 30)
        assert datetime_exists(dt, tz)

    def test_utc_always_exists(self):
        tz = tzutc()
        dt = datetime(2024, 3, 10, 2, 30)
        assert datetime_exists(dt, tz)

    def test_fixed_offset_always_exists(self):
        tz = tzoffset("JST", 9 * 3600)
        dt = datetime(2024, 3, 10, 2, 30)
        assert datetime_exists(dt, tz)


# ============================================================================
# datetime_ambiguous
# ============================================================================


class TestDatetimeAmbiguous:
    def test_normal_time_not_ambiguous(self):
        tz = gettz("US/Eastern")
        dt = datetime(2024, 6, 15, 12, 0)
        assert not datetime_ambiguous(dt, tz)

    def test_fall_back_ambiguous(self):
        # Nov 3, 2024 1:30 AM is ambiguous (fall back)
        tz = gettz("US/Eastern")
        dt = datetime(2024, 11, 3, 1, 30)
        assert datetime_ambiguous(dt, tz)

    def test_before_fallback(self):
        tz = gettz("US/Eastern")
        dt = datetime(2024, 11, 3, 0, 30)
        assert not datetime_ambiguous(dt, tz)

    def test_after_fallback(self):
        tz = gettz("US/Eastern")
        dt = datetime(2024, 11, 3, 2, 30)
        assert not datetime_ambiguous(dt, tz)

    def test_utc_never_ambiguous(self):
        tz = tzutc()
        dt = datetime(2024, 11, 3, 1, 30)
        assert not datetime_ambiguous(dt, tz)

    def test_fixed_offset_never_ambiguous(self):
        tz = tzoffset("EST", -5 * 3600)
        dt = datetime(2024, 11, 3, 1, 30)
        assert not datetime_ambiguous(dt, tz)


# ============================================================================
# resolve_imaginary
# ============================================================================


class TestResolveImaginary:
    def test_imaginary_resolved(self):
        tz = gettz("US/Eastern")
        # 2:30 AM doesn't exist, should resolve to 3:30 AM EDT
        dt = datetime(2024, 3, 10, 2, 30)
        result = resolve_imaginary(dt, tz)
        assert result.hour == 3
        assert result.minute == 30

    def test_real_time_unchanged(self):
        tz = gettz("US/Eastern")
        dt = datetime(2024, 6, 15, 12, 0)
        result = resolve_imaginary(dt, tz)
        assert result == dt

    def test_utc_passthrough(self):
        tz = tzutc()
        dt = datetime(2024, 3, 10, 2, 30)
        result = resolve_imaginary(dt, tz)
        assert result == dt
