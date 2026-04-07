"""dateutil_rs.rrule - Recurrence rules (RFC 5545).

Rust-accelerated implementation of dateutil.rrule.
"""

# Re-export weekday for rrule.weekday(n) calls
from dateutil_rs._native import (
    DAILY,
    FR,
    HOURLY,
    MINUTELY,
    MO,
    MONTHLY,
    SA,
    SECONDLY,
    SU,
    TH,
    TU,
    WE,
    WEEKLY,
    YEARLY,
    rrule,
    rruleset,
    rrulestr,
    weekday,
)

__all__ = [
    "DAILY",
    "FR",
    "HOURLY",
    "MINUTELY",
    "MO",
    "MONTHLY",
    "SA",
    "SECONDLY",
    "SU",
    "TH",
    "TU",
    "WE",
    "WEEKLY",
    "YEARLY",
    "rrule",
    "rruleset",
    "rrulestr",
    "weekday",
]
