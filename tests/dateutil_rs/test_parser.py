"""Tests for dateutil_rs.parser module (parse, isoparse, parse_to_dict, parserinfo)."""

from datetime import date, datetime, timedelta, timezone

import pytest

from dateutil_rs import isoparse, parse, parse_to_dict
from dateutil_rs.parser import parserinfo

# ============================================================================
# parse()
# ============================================================================


class TestParse:
    """Basic parse() functionality."""

    def test_iso_format(self):
        assert parse("2024-03-15") == datetime(2024, 3, 15)

    def test_us_format(self):
        assert parse("03/15/2024") == datetime(2024, 3, 15)

    def test_with_time(self):
        assert parse("2024-03-15 14:30:00") == datetime(2024, 3, 15, 14, 30)

    def test_with_microseconds(self):
        result = parse("2024-03-15 14:30:00.123456")
        assert result.microsecond == 123456

    def test_month_name(self):
        assert parse("March 15, 2024") == datetime(2024, 3, 15)

    def test_month_abbr(self):
        assert parse("Mar 15, 2024") == datetime(2024, 3, 15)

    def test_day_month_year_words(self):
        assert parse("15 March 2024") == datetime(2024, 3, 15)

    def test_ampm(self):
        assert parse("March 15, 2024 2:30 PM") == datetime(2024, 3, 15, 14, 30)

    def test_ampm_lowercase(self):
        assert parse("March 15, 2024 2:30 pm") == datetime(2024, 3, 15, 14, 30)

    def test_12am_is_midnight(self):
        assert parse("March 15, 2024 12:00 AM") == datetime(2024, 3, 15, 0, 0)

    def test_12pm_is_noon(self):
        assert parse("March 15, 2024 12:00 PM") == datetime(2024, 3, 15, 12, 0)

    def test_returns_datetime(self):
        result = parse("2024-03-15")
        assert isinstance(result, datetime)


class TestParseDefault:
    """parse() with default datetime."""

    def test_fills_missing_fields(self):
        default = datetime(2024, 6, 15, 10, 30, 0)
        result = parse("14:00", default=default)
        assert result == datetime(2024, 6, 15, 14, 0, 0)

    def test_year_only(self):
        default = datetime(2000, 6, 15)
        result = parse("2024", default=default)
        assert result.year == 2024
        assert result.month == 6
        assert result.day == 15

    def test_month_day_only(self):
        default = datetime(2024, 1, 1)
        result = parse("March 15", default=default)
        assert result == datetime(2024, 3, 15)


class TestParseDayfirst:
    """parse() with dayfirst option."""

    def test_dayfirst_true(self):
        result = parse("15/03/2024", dayfirst=True)
        assert result == datetime(2024, 3, 15)

    def test_dayfirst_false(self):
        result = parse("03/15/2024", dayfirst=False)
        assert result == datetime(2024, 3, 15)

    def test_ambiguous_date_dayfirst(self):
        # 01/02/2024: dayfirst=True -> Feb 1, dayfirst=False -> Jan 2
        result_df = parse("01/02/2024", dayfirst=True)
        result_mf = parse("01/02/2024", dayfirst=False)
        assert result_df == datetime(2024, 2, 1)
        assert result_mf == datetime(2024, 1, 2)


class TestParseYearfirst:
    """parse() with yearfirst option."""

    def test_yearfirst_true(self):
        result = parse("2024/03/15", yearfirst=True)
        assert result == datetime(2024, 3, 15)

    def test_ambiguous_date_yearfirst(self):
        result = parse("24/03/15", yearfirst=True)
        assert result.year == 2024


class TestParseTimezone:
    """parse() with timezone handling."""

    def test_utc_suffix(self):
        result = parse("2024-03-15 14:30:00 UTC")
        assert result.tzinfo is not None
        assert result.utcoffset() == timedelta(0)

    def test_offset_suffix(self):
        result = parse("2024-03-15 14:30:00+09:00")
        assert result.utcoffset() == timedelta(hours=9)

    def test_negative_offset(self):
        result = parse("2024-03-15 14:30:00-05:00")
        assert result.utcoffset() == timedelta(hours=-5)

    def test_ignoretz(self):
        result = parse("2024-03-15 14:30:00 UTC", ignoretz=True)
        assert result.tzinfo is None

    def test_tzinfos_mapping(self):
        tzinfos = {"JST": 9 * 3600}
        result = parse("2024-03-15 14:30 JST", tzinfos=tzinfos)
        assert result.utcoffset() == timedelta(hours=9)

    def test_tzinfos_callable(self):
        def tzinfos(name, offset):
            if name == "JST":
                return 9 * 3600
            return offset

        result = parse("2024-03-15 14:30 JST", tzinfos=tzinfos)
        assert result.utcoffset() == timedelta(hours=9)


class TestParseErrors:
    """parse() error handling."""

    def test_empty_string(self):
        with pytest.raises(ValueError):
            parse("")

    def test_garbage(self):
        with pytest.raises(ValueError):
            parse("not a date at all")


# ============================================================================
# isoparse()
# ============================================================================


class TestIsoparse:
    """ISO-8601 strict parsing."""

    def test_date_only(self):
        assert isoparse("2024-03-15") == datetime(2024, 3, 15)

    def test_datetime(self):
        assert isoparse("2024-03-15T14:30:00") == datetime(2024, 3, 15, 14, 30)

    def test_datetime_with_z(self):
        # isoparse returns naive datetime (tz info stripped)
        result = isoparse("2024-03-15T14:30:00Z")
        assert result == datetime(2024, 3, 15, 14, 30)

    def test_datetime_with_offset(self):
        # isoparse returns naive datetime (tz info stripped)
        result = isoparse("2024-03-15T14:30:00+09:00")
        assert result == datetime(2024, 3, 15, 14, 30)

    def test_compact_date(self):
        assert isoparse("20240315") == datetime(2024, 3, 15)

    def test_compact_datetime(self):
        assert isoparse("20240315T143000") == datetime(2024, 3, 15, 14, 30)

    def test_microseconds(self):
        result = isoparse("2024-03-15T14:30:00.123456")
        assert result.microsecond == 123456

    def test_milliseconds(self):
        result = isoparse("2024-03-15T14:30:00.123")
        assert result.microsecond == 123000

    def test_date_with_week_not_supported(self):
        # ISO week dates not supported in dateutil_rs isoparse
        with pytest.raises(ValueError):
            isoparse("2024-W11-5")

    def test_ordinal_date_not_supported(self):
        # ISO ordinal dates not supported in dateutil_rs isoparse
        with pytest.raises(ValueError):
            isoparse("2024-075")

    def test_hours_only(self):
        assert isoparse("2024-03-15T14") == datetime(2024, 3, 15, 14, 0)

    def test_hours_minutes(self):
        assert isoparse("2024-03-15T14:30") == datetime(2024, 3, 15, 14, 30)

    def test_negative_offset(self):
        # isoparse returns naive datetime (tz info stripped)
        result = isoparse("2024-03-15T14:30:00-05:30")
        assert result == datetime(2024, 3, 15, 14, 30)

    def test_invalid_iso(self):
        with pytest.raises(ValueError):
            isoparse("March 15, 2024")


# ============================================================================
# parse_to_dict()
# ============================================================================


class TestParseToDict:
    """parse_to_dict() returns parsed fields as dict."""

    def test_full_datetime(self):
        result = parse_to_dict("2024-03-15 14:30:45")
        assert result["year"] == 2024
        assert result["month"] == 3
        assert result["day"] == 15
        assert result["hour"] == 14
        assert result["minute"] == 30
        assert result["second"] == 45

    def test_date_only(self):
        result = parse_to_dict("2024-03-15")
        assert result["year"] == 2024
        assert result["month"] == 3
        assert result["day"] == 15
        assert result["hour"] is None
        assert result["minute"] is None
        assert result["second"] is None

    def test_has_all_keys(self):
        result = parse_to_dict("2024-03-15")
        expected_keys = {
            "year",
            "month",
            "day",
            "weekday",
            "hour",
            "minute",
            "second",
            "microsecond",
            "tzname",
            "tzoffset",
        }
        assert set(result.keys()) == expected_keys

    def test_with_timezone(self):
        result = parse_to_dict("2024-03-15 14:30:00+09:00")
        assert result["tzoffset"] == 32400  # 9 * 3600

    def test_weekday_field(self):
        # "Monday" sets the weekday field
        result = parse_to_dict("Monday")
        assert result["weekday"] is not None

    def test_microsecond(self):
        result = parse_to_dict("2024-03-15 14:30:00.123456")
        assert result["microsecond"] == 123456

    def test_dayfirst(self):
        result = parse_to_dict("15/03/2024", dayfirst=True)
        assert result["day"] == 15
        assert result["month"] == 3

    def test_yearfirst(self):
        result = parse_to_dict("2024/03/15", yearfirst=True)
        assert result["year"] == 2024


# ============================================================================
# parserinfo
# ============================================================================


class TestParserinfo:
    """Customizable parser lookup tables."""

    def test_default_construction(self):
        info = parserinfo()
        assert isinstance(info, parserinfo)

    def test_dayfirst(self):
        info = parserinfo(dayfirst=True)
        result = parse("01/02/2024", parserinfo=info)
        assert result == datetime(2024, 2, 1)

    def test_yearfirst(self):
        info = parserinfo(yearfirst=True)
        result = parse("24/03/15", parserinfo=info)
        assert result.year == 2024

    def test_repr(self):
        info = parserinfo()
        r = repr(info)
        assert "parserinfo" in r.lower() or "ParserInfo" in r


class TestParserinfoSubclass:
    """Subclassing parserinfo for custom locale."""

    def test_custom_months(self):

        class GermanParserInfo(parserinfo):
            MONTHS = [
                ["Januar", "Jan"],
                ["Februar", "Feb"],
                ["Maerz", "Mae"],
                ["April", "Apr"],
                ["Mai"],
                ["Juni", "Jun"],
                ["Juli", "Jul"],
                ["August", "Aug"],
                ["September", "Sep"],
                ["Oktober", "Okt"],
                ["November", "Nov"],
                ["Dezember", "Dez"],
            ]

        info = GermanParserInfo()
        result = parse("15 Maerz 2024", parserinfo=info)
        assert result.month == 3
        assert result.day == 15
