"""Benchmarks for dateutil.relativedelta module."""

import datetime

BASE_DT = datetime.datetime(2024, 1, 15, 14, 30, 0)


# --- Construction benchmarks ---


def test_relativedelta_create_simple(benchmark, du):
    """Create a simple relativedelta(months=+1)."""
    benchmark(du.relativedelta.relativedelta, months=1)


def test_relativedelta_create_complex(benchmark, du):
    """Create a complex relativedelta with multiple fields."""
    benchmark(
        du.relativedelta.relativedelta,
        years=1,
        months=2,
        days=3,
        hours=4,
        minutes=5,
        seconds=6,
    )


def test_relativedelta_create_absolute(benchmark, du):
    """Create relativedelta with absolute fields (year, month, day)."""
    benchmark(du.relativedelta.relativedelta, year=2025, month=6, day=15, hour=12)


def test_relativedelta_create_weekday(benchmark, du):
    """Create relativedelta with weekday: next Monday."""
    MO = du.relativedelta.MO
    benchmark(du.relativedelta.relativedelta, weekday=MO(+1))


# --- Arithmetic benchmarks ---


def test_relativedelta_add_months(benchmark, du):
    """Add 1 month to a datetime."""
    rd = du.relativedelta.relativedelta(months=1)
    benchmark(lambda: BASE_DT + rd)


def test_relativedelta_add_complex(benchmark, du):
    """Add a complex relativedelta to a datetime."""
    rd = du.relativedelta.relativedelta(
        years=1, months=2, days=3, hours=4, minutes=5, seconds=6
    )
    benchmark(lambda: BASE_DT + rd)


def test_relativedelta_subtract(benchmark, du):
    """Subtract relativedelta from a datetime."""
    rd = du.relativedelta.relativedelta(months=3, days=10)
    benchmark(lambda: BASE_DT - rd)


def test_relativedelta_add_weekday(benchmark, du):
    """Add relativedelta with weekday (next Friday)."""
    FR = du.relativedelta.FR
    rd = du.relativedelta.relativedelta(weekday=FR(+1))
    benchmark(lambda: BASE_DT + rd)


def test_relativedelta_month_end_overflow(benchmark, du):
    """Add months causing day overflow (Jan 31 + 1 month -> Feb 28/29)."""
    dt = datetime.datetime(2024, 1, 31)
    rd = du.relativedelta.relativedelta(months=1)
    benchmark(lambda: dt + rd)


# --- Difference benchmarks ---


def test_relativedelta_diff_datetimes(benchmark, du):
    """Compute relativedelta between two datetimes."""
    dt1 = datetime.datetime(2024, 3, 20, 10, 0, 0)
    dt2 = datetime.datetime(2023, 1, 15, 8, 30, 0)
    benchmark(du.relativedelta.relativedelta, dt1=dt1, dt2=dt2)


def test_relativedelta_diff_dates(benchmark, du):
    """Compute relativedelta between two dates."""
    d1 = datetime.date(2024, 12, 25)
    d2 = datetime.date(2020, 1, 1)
    benchmark(du.relativedelta.relativedelta, dt1=d1, dt2=d2)


# --- Batch operations ---


def test_relativedelta_sequential_add(benchmark, du):
    """Add relativedelta(months=1) sequentially 12 times."""
    rd_cls = du.relativedelta.relativedelta

    def compute():
        rd = rd_cls(months=1)
        dt = BASE_DT
        results = []
        for _ in range(12):
            dt = dt + rd
            results.append(dt)
        return results

    benchmark(compute)


def test_relativedelta_multiply(benchmark, du):
    """Multiply a relativedelta by a scalar."""
    rd = du.relativedelta.relativedelta(months=1, days=5, hours=3)
    benchmark(lambda: rd * 6)
