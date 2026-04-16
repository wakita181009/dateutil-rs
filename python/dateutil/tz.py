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


_NOT_IMPL = (
    "{cls} is not supported in dateutil-rs. "
    "Use IANA timezone names via gettz() instead."
)


class tzrange(datetime.tzinfo):
    """Stub for python-dateutil compatibility (not implemented in dateutil-rs).

    Accepts any arguments so class-level assignments in test code succeed.
    Raises ``NotImplementedError`` when used as a real timezone.
    """

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        self._args = args
        self._kwargs = kwargs

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta:
        raise NotImplementedError(_NOT_IMPL.format(cls="tzrange"))

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta:
        raise NotImplementedError(_NOT_IMPL.format(cls="tzrange"))

    def tzname(self, dt: datetime.datetime | None) -> str:
        raise NotImplementedError(_NOT_IMPL.format(cls="tzrange"))

    def __repr__(self) -> str:
        parts = [repr(a) for a in self._args]
        parts.extend(f"{k}={v!r}" for k, v in self._kwargs.items())
        return f"tzrange({', '.join(parts)})"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, tzrange):
            return NotImplemented
        return self._args == other._args and self._kwargs == other._kwargs

    def __hash__(self) -> int:
        return hash(("tzrange", self._args, tuple(sorted(self._kwargs.items()))))


class tzstr(datetime.tzinfo):
    """Stub for python-dateutil compatibility (not implemented in dateutil-rs).

    Accepts any arguments so class-level assignments in test code succeed.
    Raises ``NotImplementedError`` when used as a real timezone.
    """

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        self._args = args
        self._kwargs = kwargs

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta:
        raise NotImplementedError(_NOT_IMPL.format(cls="tzstr"))

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta:
        raise NotImplementedError(_NOT_IMPL.format(cls="tzstr"))

    def tzname(self, dt: datetime.datetime | None) -> str:
        raise NotImplementedError(_NOT_IMPL.format(cls="tzstr"))

    def __repr__(self) -> str:
        parts = [repr(a) for a in self._args]
        parts.extend(f"{k}={v!r}" for k, v in self._kwargs.items())
        return f"tzstr({', '.join(parts)})"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, tzstr):
            return NotImplemented
        return self._args == other._args and self._kwargs == other._kwargs

    def __hash__(self) -> int:
        return hash(("tzstr", self._args, tuple(sorted(self._kwargs.items()))))


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
