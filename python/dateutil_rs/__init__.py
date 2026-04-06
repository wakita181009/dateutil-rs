"""dateutil_rs - Rust-accelerated date utilities."""

from dateutil_rs._native import (
    EASTER_JULIAN,
    EASTER_ORTHODOX,
    EASTER_WESTERN,
    FR,
    MO,
    SA,
    SU,
    TH,
    TU,
    WE,
    weekday,
    within_delta,
)

__all__ = [
    "EASTER_JULIAN",
    "EASTER_ORTHODOX",
    "EASTER_WESTERN",
    "MO",
    "TU",
    "WE",
    "TH",
    "FR",
    "SA",
    "SU",
    "easter",
    "relativedelta",
    "weekday",
    "within_delta",
]
