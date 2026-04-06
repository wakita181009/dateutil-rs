"""Benchmarks for dateutil.rrule module."""

import datetime

BASE_DT = datetime.datetime(2024, 1, 1)


# --- Single rule creation and iteration ---


def test_rrule_daily_100(benchmark, du):
    """Generate 100 daily occurrences."""
    DAILY = du.rrule.DAILY
    rrule = du.rrule.rrule

    def compute():
        return list(rrule(DAILY, count=100, dtstart=BASE_DT))

    benchmark(compute)


def test_rrule_weekly_52(benchmark, du):
    """Generate 52 weekly occurrences (1 year)."""
    WEEKLY = du.rrule.WEEKLY
    rrule = du.rrule.rrule

    def compute():
        return list(rrule(WEEKLY, count=52, dtstart=BASE_DT))

    benchmark(compute)


def test_rrule_monthly_120(benchmark, du):
    """Generate 120 monthly occurrences (10 years)."""
    MONTHLY = du.rrule.MONTHLY
    rrule = du.rrule.rrule

    def compute():
        return list(rrule(MONTHLY, count=120, dtstart=BASE_DT))

    benchmark(compute)


def test_rrule_yearly_100(benchmark, du):
    """Generate 100 yearly occurrences."""
    YEARLY = du.rrule.YEARLY
    rrule = du.rrule.rrule

    def compute():
        return list(rrule(YEARLY, count=100, dtstart=BASE_DT))

    benchmark(compute)


def test_rrule_hourly_1000(benchmark, du):
    """Generate 1000 hourly occurrences."""
    HOURLY = du.rrule.HOURLY
    rrule = du.rrule.rrule

    def compute():
        return list(rrule(HOURLY, count=1000, dtstart=BASE_DT))

    benchmark(compute)


# --- Rules with interval and byXXX ---


def test_rrule_daily_interval3(benchmark, du):
    """Generate 100 occurrences every 3 days."""
    DAILY = du.rrule.DAILY
    rrule = du.rrule.rrule

    def compute():
        return list(rrule(DAILY, interval=3, count=100, dtstart=BASE_DT))

    benchmark(compute)


def test_rrule_monthly_byday(benchmark, du):
    """Generate 24 monthly occurrences on MO,WE,FR."""
    MONTHLY = du.rrule.MONTHLY
    rrule = du.rrule.rrule
    MO = du.rrule.MO
    WE = du.rrule.WE
    FR = du.rrule.FR

    def compute():
        return list(rrule(MONTHLY, count=24, byweekday=(MO, WE, FR), dtstart=BASE_DT))

    benchmark(compute)


def test_rrule_yearly_bymonth(benchmark, du):
    """Generate 50 yearly occurrences in Jan and Jul."""
    YEARLY = du.rrule.YEARLY
    rrule = du.rrule.rrule

    def compute():
        return list(rrule(YEARLY, count=50, bymonth=(1, 7), dtstart=BASE_DT))

    benchmark(compute)


def test_rrule_monthly_bymonthday(benchmark, du):
    """Generate 60 monthly occurrences on 1st and 15th."""
    MONTHLY = du.rrule.MONTHLY
    rrule = du.rrule.rrule

    def compute():
        return list(rrule(MONTHLY, count=60, bymonthday=(1, 15), dtstart=BASE_DT))

    benchmark(compute)


def test_rrule_weekly_byday_interval2(benchmark, du):
    """Generate 100 biweekly occurrences on TU and TH."""
    WEEKLY = du.rrule.WEEKLY
    rrule = du.rrule.rrule
    TU = du.rrule.TU
    TH = du.rrule.TH

    def compute():
        return list(
            rrule(WEEKLY, interval=2, count=100, byweekday=(TU, TH), dtstart=BASE_DT)
        )

    benchmark(compute)


# --- rruleset benchmarks ---


def test_rruleset_union(benchmark, du):
    """Union of two daily rules with different starts."""
    DAILY = du.rrule.DAILY
    rrule = du.rrule.rrule
    rruleset = du.rrule.rruleset

    def compute():
        rs = rruleset()
        rs.rrule(rrule(DAILY, count=50, dtstart=BASE_DT))
        rs.rrule(rrule(DAILY, count=50, dtstart=BASE_DT + datetime.timedelta(hours=12)))
        return list(rs)

    benchmark(compute)


def test_rruleset_exdate(benchmark, du):
    """Daily rule with 10 exclusion dates."""
    DAILY = du.rrule.DAILY
    rrule = du.rrule.rrule
    rruleset = du.rrule.rruleset

    def compute():
        rs = rruleset()
        rs.rrule(rrule(DAILY, count=100, dtstart=BASE_DT))
        for i in range(0, 100, 10):
            rs.exdate(BASE_DT + datetime.timedelta(days=i))
        return list(rs)

    benchmark(compute)


def test_rruleset_exrule(benchmark, du):
    """Daily rule excluding every Saturday and Sunday."""
    DAILY = du.rrule.DAILY
    WEEKLY = du.rrule.WEEKLY
    rrule = du.rrule.rrule
    rruleset = du.rrule.rruleset
    SA = du.rrule.SA
    SU = du.rrule.SU

    def compute():
        rs = rruleset()
        rs.rrule(rrule(DAILY, count=365, dtstart=BASE_DT))
        rs.exrule(rrule(WEEKLY, byweekday=(SA, SU), count=104, dtstart=BASE_DT))
        return list(rs)

    benchmark(compute)


# --- rrulestr benchmarks ---


def test_rrulestr_simple(benchmark, du):
    """Parse a simple RRULE string."""
    benchmark(du.rrule.rrulestr, "RRULE:FREQ=DAILY;COUNT=100")


def test_rrulestr_complex(benchmark, du):
    """Parse a complex RRULE string with BYDAY, BYMONTH."""
    s = "RRULE:FREQ=MONTHLY;BYDAY=MO,WE,FR;BYMONTH=1,3,5,7,9,11;COUNT=50"
    benchmark(du.rrule.rrulestr, s)


def test_rrulestr_with_dtstart(benchmark, du):
    """Parse RRULE string with DTSTART."""
    s = "DTSTART:20240101T000000\nRRULE:FREQ=WEEKLY;INTERVAL=2;COUNT=52"
    benchmark(du.rrule.rrulestr, s)
