"""Benchmarks comparing v0 (dateutil-rs) vs v1 (dateutil-core) Rust implementations.

Run with:
    uv run pytest benchmarks/bench_v0_v1.py --benchmark-enable -v
"""

import datetime

import pytest

# ---------------------------------------------------------------------------
# Import v0 and v1
# ---------------------------------------------------------------------------

try:
    from dateutil_rs.easter import (
        EASTER_JULIAN as V0_EASTER_JULIAN,
        EASTER_ORTHODOX as V0_EASTER_ORTHODOX,
        EASTER_WESTERN as V0_EASTER_WESTERN,
        easter as v0_easter,
    )
    from dateutil_rs.parser import isoparse as v0_isoparse, parse as v0_parse
    from dateutil_rs.relativedelta import relativedelta as v0_relativedelta

    _v0 = True
except ImportError:
    _v0 = False

try:
    from dateutil_rs.v1._native import (
        EASTER_JULIAN as V1_EASTER_JULIAN,
        EASTER_ORTHODOX as V1_EASTER_ORTHODOX,
        EASTER_WESTERN as V1_EASTER_WESTERN,
        easter as v1_easter,
        isoparse as v1_isoparse,
        parse as v1_parse,
        relativedelta as v1_relativedelta,
    )

    _v1 = True
except ImportError:
    _v1 = False


def skip_v0():
    if not _v0:
        pytest.skip("v0 not installed")


def skip_v1():
    if not _v1:
        pytest.skip("v1 not installed")


# ===================================================================
# Easter
# ===================================================================


class TestEasterV0:
    def test_single(self, benchmark):
        skip_v0()
        benchmark(v0_easter, 2024, V0_EASTER_WESTERN)

    def test_range_1000(self, benchmark):
        skip_v0()

        def compute():
            return [v0_easter(y, V0_EASTER_WESTERN) for y in range(1583, 2583)]

        benchmark(compute)

    def test_all_methods_500(self, benchmark):
        skip_v0()

        def compute():
            return [
                v0_easter(y, m)
                for y in range(1583, 2083)
                for m in (V0_EASTER_JULIAN, V0_EASTER_ORTHODOX, V0_EASTER_WESTERN)
            ]

        benchmark(compute)


class TestEasterV1:
    def test_single(self, benchmark):
        skip_v1()
        benchmark(v1_easter, 2024, V1_EASTER_WESTERN)

    def test_range_1000(self, benchmark):
        skip_v1()

        def compute():
            return [v1_easter(y, V1_EASTER_WESTERN) for y in range(1583, 2583)]

        benchmark(compute)

    def test_all_methods_500(self, benchmark):
        skip_v1()

        def compute():
            return [
                v1_easter(y, m)
                for y in range(1583, 2083)
                for m in (V1_EASTER_JULIAN, V1_EASTER_ORTHODOX, V1_EASTER_WESTERN)
            ]

        benchmark(compute)


# ===================================================================
# Parser — parse()
# ===================================================================

PARSE_STRINGS = [
    "2024-01-15",
    "2024-01-15 14:30:00",
    "January 15, 2024",
    "15 Jan 2024 14:30",
    "Mon Jan 15 14:30:00 2024",
    "01/15/2024 2:30 PM",
]


class TestParseV0:
    def test_simple_date(self, benchmark):
        skip_v0()
        benchmark(v0_parse, "2024-01-15")

    def test_datetime_with_time(self, benchmark):
        skip_v0()
        benchmark(v0_parse, "2024-01-15 14:30:00")

    def test_american_format(self, benchmark):
        skip_v0()
        benchmark(v0_parse, "January 15, 2024")

    def test_various_formats(self, benchmark):
        skip_v0()

        def compute():
            return [v0_parse(s) for s in PARSE_STRINGS]

        benchmark(compute)


class TestParseV1:
    def test_simple_date(self, benchmark):
        skip_v1()
        benchmark(v1_parse, "2024-01-15")

    def test_datetime_with_time(self, benchmark):
        skip_v1()
        benchmark(v1_parse, "2024-01-15 14:30:00")

    def test_american_format(self, benchmark):
        skip_v1()
        benchmark(v1_parse, "January 15, 2024")

    def test_various_formats(self, benchmark):
        skip_v1()

        def compute():
            return [v1_parse(s) for s in PARSE_STRINGS]

        benchmark(compute)


# ===================================================================
# Parser — isoparse()
# ===================================================================

ISO_STRINGS = [
    "2024-01-15",
    "2024-01-15T14:30:00",
    "20240115",
    "20240115T143000",
    "2024-01-15T14:30:00.123456",
]


class TestIsoparseV0:
    def test_date(self, benchmark):
        skip_v0()
        benchmark(v0_isoparse, "2024-01-15")

    def test_datetime(self, benchmark):
        skip_v0()
        benchmark(v0_isoparse, "2024-01-15T14:30:00")

    def test_compact(self, benchmark):
        skip_v0()
        benchmark(v0_isoparse, "20240115T143000")

    def test_various(self, benchmark):
        skip_v0()

        def compute():
            return [v0_isoparse(s) for s in ISO_STRINGS]

        benchmark(compute)


class TestIsoparseV1:
    def test_date(self, benchmark):
        skip_v1()
        benchmark(v1_isoparse, "2024-01-15")

    def test_datetime(self, benchmark):
        skip_v1()
        benchmark(v1_isoparse, "2024-01-15T14:30:00")

    def test_compact(self, benchmark):
        skip_v1()
        benchmark(v1_isoparse, "20240115T143000")

    def test_various(self, benchmark):
        skip_v1()

        def compute():
            return [v1_isoparse(s) for s in ISO_STRINGS]

        benchmark(compute)


# ===================================================================
# RelativeDelta
# ===================================================================

BASE_DT = datetime.datetime(2024, 1, 15, 14, 30, 0)


class TestRelativeDeltaV0:
    def test_create_simple(self, benchmark):
        skip_v0()
        benchmark(v0_relativedelta, months=1)

    def test_create_complex(self, benchmark):
        skip_v0()
        benchmark(
            v0_relativedelta,
            years=1, months=2, days=3, hours=4, minutes=5, seconds=6,
        )

    def test_add_months(self, benchmark):
        skip_v0()
        rd = v0_relativedelta(months=1)
        benchmark(lambda: BASE_DT + rd)

    def test_add_complex(self, benchmark):
        skip_v0()
        rd = v0_relativedelta(
            years=1, months=2, days=3, hours=4, minutes=5, seconds=6,
        )
        benchmark(lambda: BASE_DT + rd)

    def test_diff(self, benchmark):
        skip_v0()
        dt1 = datetime.datetime(2024, 3, 20, 10, 0, 0)
        dt2 = datetime.datetime(2023, 1, 15, 8, 30, 0)
        benchmark(v0_relativedelta, dt1=dt1, dt2=dt2)


class TestRelativeDeltaV1:
    def test_create_simple(self, benchmark):
        skip_v1()
        benchmark(v1_relativedelta, months=1)

    def test_create_complex(self, benchmark):
        skip_v1()
        benchmark(
            v1_relativedelta,
            years=1, months=2, days=3, hours=4, minutes=5, seconds=6,
        )

    def test_add_months(self, benchmark):
        skip_v1()
        rd = v1_relativedelta(months=1)
        benchmark(rd.add_to_datetime, BASE_DT)

    def test_add_complex(self, benchmark):
        skip_v1()
        rd = v1_relativedelta(
            years=1, months=2, days=3, hours=4, minutes=5, seconds=6,
        )
        benchmark(rd.add_to_datetime, BASE_DT)

    def test_diff(self, benchmark):
        skip_v1()
        dt1 = datetime.datetime(2024, 3, 20, 10, 0, 0)
        dt2 = datetime.datetime(2023, 1, 15, 8, 30, 0)
        benchmark(v1_relativedelta.from_diff, dt1, dt2)
