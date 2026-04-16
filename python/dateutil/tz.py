"""dateutil.tz - Timezone types and utilities."""

from __future__ import annotations

import datetime
from typing import Any

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


class tzrange(datetime.tzinfo):
    """Not implemented in dateutil-rs. Use IANA timezone names via gettz()."""

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        raise NotImplementedError(
            "tzrange is not supported in dateutil-rs. "
            "Use IANA timezone names via gettz() instead."
        )

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta:
        raise NotImplementedError

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta:
        raise NotImplementedError

    def tzname(self, dt: datetime.datetime | None) -> str:
        raise NotImplementedError


class tzstr(datetime.tzinfo):
    """Not implemented in dateutil-rs. Use IANA timezone names via gettz()."""

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        raise NotImplementedError(
            "tzstr is not supported in dateutil-rs. "
            "Use IANA timezone names via gettz() instead."
        )

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta:
        raise NotImplementedError

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta:
        raise NotImplementedError

    def tzname(self, dt: datetime.datetime | None) -> str:
        raise NotImplementedError


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
