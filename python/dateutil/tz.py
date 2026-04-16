"""dateutil.tz - Timezone types and utilities."""

from dateutil._native import (
    datetime_ambiguous,
    datetime_exists,
    gettz,
    resolve_imaginary,
    tzfile,
    tzlocal,
    tzoffset,
    tzutc,
)

UTC = tzutc()

__all__ = [
    "UTC",
    "datetime_ambiguous",
    "datetime_exists",
    "gettz",
    "resolve_imaginary",
    "tzfile",
    "tzlocal",
    "tzoffset",
    "tzutc",
]
