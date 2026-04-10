"""Benchmarks for dateutil.tz module."""

import datetime

import pytest

# --- Timezone creation benchmarks ---


def test_tz_tzutc_create(benchmark, du):
    """Create a tzutc instance."""
    benchmark(du.tz.tzutc)


def test_tz_tzoffset_create(benchmark, du):
    """Create a tzoffset instance (JST +9:00)."""
    benchmark(du.tz.tzoffset, "JST", 32400)


def test_tz_tzlocal_create(benchmark, du):
    """Create a tzlocal instance."""
    benchmark(du.tz.tzlocal)


def test_tz_gettz_utc(benchmark, du):
    """Look up UTC timezone via gettz."""
    benchmark(du.tz.gettz, "UTC")


def test_tz_gettz_named(benchmark, du):
    """Look up a named timezone via gettz: 'America/New_York'."""
    benchmark(du.tz.gettz, "America/New_York")


def test_tz_gettz_offset(benchmark, du):
    """Look up a fixed-offset timezone via gettz: '+05:00'."""
    if du.name == "v1":
        pytest.skip("v1 gettz does not support offset strings")
    benchmark(du.tz.gettz, "+05:00")


# --- Timezone conversion benchmarks ---


def test_tz_convert_utc_to_eastern(benchmark, du):
    """Convert UTC datetime to US/Eastern."""
    UTC_DT = datetime.datetime(2024, 7, 15, 14, 30, 0, tzinfo=du.tz.UTC)
    eastern = du.tz.gettz("America/New_York")

    def compute():
        return UTC_DT.astimezone(eastern)

    benchmark(compute)


def test_tz_convert_utc_to_jst(benchmark, du):
    """Convert UTC datetime to Asia/Tokyo."""
    UTC_DT = datetime.datetime(2024, 7, 15, 14, 30, 0, tzinfo=du.tz.UTC)
    jst = du.tz.gettz("Asia/Tokyo")

    def compute():
        return UTC_DT.astimezone(jst)

    benchmark(compute)


def test_tz_localize_naive(benchmark, du):
    """Attach timezone to naive datetime via replace."""
    NAIVE_DT = datetime.datetime(2024, 7, 15, 14, 30, 0)
    jst = du.tz.gettz("Asia/Tokyo")

    def compute():
        return NAIVE_DT.replace(tzinfo=jst)

    benchmark(compute)


def test_tz_convert_chain(benchmark, du):
    """Chain conversion: UTC -> Eastern -> Pacific -> JST."""
    UTC_DT = datetime.datetime(2024, 7, 15, 14, 30, 0, tzinfo=du.tz.UTC)
    eastern = du.tz.gettz("America/New_York")
    pacific = du.tz.gettz("America/Los_Angeles")
    jst = du.tz.gettz("Asia/Tokyo")

    def compute():
        dt = UTC_DT.astimezone(eastern)
        dt = dt.astimezone(pacific)
        dt = dt.astimezone(jst)
        return dt

    benchmark(compute)


# --- Utility function benchmarks ---
# python-dateutil / v0: datetime_exists(aware_dt, tz=None) — tz from dt.tzinfo
# v1: datetime_exists(naive_dt, tz) — separate args


def test_tz_datetime_exists(benchmark, du):
    """Check if a datetime exists (DST gap check)."""
    eastern = du.tz.gettz("America/New_York")
    if du.name == "v1":
        dt = datetime.datetime(2024, 3, 10, 2, 30, 0)
        benchmark(du.tz.datetime_exists, dt, eastern)
    else:
        dt = datetime.datetime(2024, 3, 10, 2, 30, 0, tzinfo=eastern)
        benchmark(du.tz.datetime_exists, dt)


def test_tz_datetime_ambiguous(benchmark, du):
    """Check if a datetime is ambiguous (DST overlap check)."""
    eastern = du.tz.gettz("America/New_York")
    if du.name == "v1":
        dt = datetime.datetime(2024, 11, 3, 1, 30, 0)
        benchmark(du.tz.datetime_ambiguous, dt, eastern)
    else:
        dt = datetime.datetime(2024, 11, 3, 1, 30, 0, tzinfo=eastern)
        benchmark(du.tz.datetime_ambiguous, dt)


def test_tz_resolve_imaginary(benchmark, du):
    """Resolve an imaginary datetime (in DST gap)."""
    eastern = du.tz.gettz("America/New_York")
    if du.name == "v1":
        dt = datetime.datetime(2024, 3, 10, 2, 30, 0)
        benchmark(du.tz.resolve_imaginary, dt, eastern)
    else:
        dt = datetime.datetime(2024, 3, 10, 2, 30, 0, tzinfo=eastern)
        benchmark(du.tz.resolve_imaginary, dt)


# --- Batch timezone lookups ---


TIMEZONE_NAMES = [
    "UTC",
    "America/New_York",
    "America/Los_Angeles",
    "America/Chicago",
    "Europe/London",
    "Europe/Berlin",
    "Europe/Paris",
    "Asia/Tokyo",
    "Asia/Shanghai",
    "Australia/Sydney",
]


def test_tz_gettz_various(benchmark, du):
    """Look up 10 different timezones via gettz."""
    gettz = du.tz.gettz

    def compute():
        return [gettz(name) for name in TIMEZONE_NAMES]

    benchmark(compute)
