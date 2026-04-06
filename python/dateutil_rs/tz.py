"""dateutil_rs.tz - Timezone support.

Delegates to python-dateutil until Rust implementation is ready.
"""

from dateutil.tz import (
    UTC,
    datetime_ambiguous,
    datetime_exists,
    gettz,
    resolve_imaginary,
    tzfile,
    tzlocal,
    tzoffset,
    tzrange,
    tzstr,
    tzutc,
)

__all__ = [
    "UTC",
    "datetime_ambiguous",
    "datetime_exists",
    "gettz",
    "resolve_imaginary",
    "tzfile",
    "tzlocal",
    "tzoffset",
    "tzrange",
    "tzstr",
    "tzutc",
]
