"""Benchmarks for dateutil.parser module."""

import datetime

# --- parse() benchmarks ---


def test_parse_simple_date(benchmark, du):
    """Parse a simple date string: '2024-01-15'."""
    benchmark(du.parser.parse, "2024-01-15")


def test_parse_datetime_with_time(benchmark, du):
    """Parse date with time: '2024-01-15 14:30:00'."""
    benchmark(du.parser.parse, "2024-01-15 14:30:00")


def test_parse_datetime_with_tz(benchmark, du):
    """Parse datetime with timezone: '2024-01-15T14:30:00+09:00'."""
    benchmark(du.parser.parse, "2024-01-15T14:30:00+09:00")


def test_parse_american_format(benchmark, du):
    """Parse American date format: 'January 15, 2024'."""
    benchmark(du.parser.parse, "January 15, 2024")


def test_parse_european_format(benchmark, du):
    """Parse European-style format with dayfirst: '15/01/2024'."""
    benchmark(du.parser.parse, "15/01/2024", dayfirst=True)


def test_parse_with_microseconds(benchmark, du):
    """Parse datetime with microseconds: '2024-01-15T14:30:00.123456'."""
    benchmark(du.parser.parse, "2024-01-15T14:30:00.123456")


def test_parse_fuzzy(benchmark, du):
    """Parse with fuzzy matching: 'Today is January 15, 2024 at 2:30 PM'."""
    benchmark(du.parser.parse, "Today is January 15, 2024 at 2:30 PM", fuzzy=True)


def test_parse_relative_with_default(benchmark, du):
    """Parse partial string with default: '14:30' with default date."""
    default = datetime.datetime(2024, 1, 15)
    benchmark(du.parser.parse, "14:30", default=default)


VARIOUS_FORMATS = [
    "2024-01-15",
    "2024-01-15T14:30:00",
    "2024-01-15 14:30:00+09:00",
    "January 15, 2024",
    "15 Jan 2024 14:30",
    "Mon Jan 15 14:30:00 2024",
    "01/15/2024 2:30 PM",
    "2024-01-15T14:30:00.123456Z",
    "Jan 15, 2024 14:30:00 UTC",
    "20240115T143000",
]


def test_parse_various_formats(benchmark, du):
    """Parse 10 different date/time string formats."""
    parse = du.parser.parse

    def compute():
        return [parse(s) for s in VARIOUS_FORMATS]

    benchmark(compute)


# --- isoparse() benchmarks ---


def test_isoparse_date(benchmark, du):
    """ISO parse date: '2024-01-15'."""
    benchmark(du.parser.isoparse, "2024-01-15")


def test_isoparse_datetime(benchmark, du):
    """ISO parse datetime: '2024-01-15T14:30:00'."""
    benchmark(du.parser.isoparse, "2024-01-15T14:30:00")


def test_isoparse_datetime_tz(benchmark, du):
    """ISO parse datetime with tz: '2024-01-15T14:30:00+09:00'."""
    benchmark(du.parser.isoparse, "2024-01-15T14:30:00+09:00")


def test_isoparse_datetime_utc(benchmark, du):
    """ISO parse datetime UTC: '2024-01-15T14:30:00Z'."""
    benchmark(du.parser.isoparse, "2024-01-15T14:30:00Z")


def test_isoparse_compact(benchmark, du):
    """ISO parse compact format: '20240115T143000'."""
    benchmark(du.parser.isoparse, "20240115T143000")


def test_isoparse_with_microseconds(benchmark, du):
    """ISO parse with microseconds: '2024-01-15T14:30:00.123456'."""
    benchmark(du.parser.isoparse, "2024-01-15T14:30:00.123456")


ISO_STRINGS = [
    "2024-01-15",
    "2024-01-15T14:30:00",
    "2024-01-15T14:30:00Z",
    "2024-01-15T14:30:00+09:00",
    "2024-01-15T14:30:00-05:00",
    "20240115",
    "20240115T143000",
    "2024-01-15T14:30:00.123456",
    "2024-01-15T14:30:00.123456Z",
    "2024-01-15T14:30:00.123456+09:00",
]


def test_isoparse_various(benchmark, du):
    """ISO parse 10 different ISO-8601 strings."""
    isoparse = du.parser.isoparse

    def compute():
        return [isoparse(s) for s in ISO_STRINGS]

    benchmark(compute)
