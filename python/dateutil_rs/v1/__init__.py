"""dateutil_rs.v1 - Rust-optimized date utility library (v1 core).

This package wraps the dateutil-core Rust crate via PyO3 bindings,
providing a streamlined, performance-focused API.
"""

from dateutil_rs.v1.common import FR, MO, SA, SU, TH, TU, WE, weekday
from dateutil_rs.v1.easter import EASTER_JULIAN, EASTER_ORTHODOX, EASTER_WESTERN, easter
from dateutil_rs.v1.parser import isoparse, parse, parse_to_dict
from dateutil_rs.v1.relativedelta import relativedelta
from dateutil_rs.v1.rrule import (
    DAILY,
    HOURLY,
    MINUTELY,
    MONTHLY,
    SECONDLY,
    WEEKLY,
    YEARLY,
    rrule,
    rruleset,
    rrulestr,
)

__all__ = [
    "DAILY",
    "EASTER_JULIAN",
    "EASTER_ORTHODOX",
    "EASTER_WESTERN",
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
    "easter",
    "isoparse",
    "parse",
    "parse_to_dict",
    "relativedelta",
    "rrule",
    "rruleset",
    "rrulestr",
    "weekday",
]
