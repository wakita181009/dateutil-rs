"""dateutil.tz - Timezone types and utilities."""

from __future__ import annotations

import datetime
from typing import Any

from dateutil._native import (
    datetime_ambiguous as _native_datetime_ambiguous,
)
from dateutil._native import (
    datetime_exists as _native_datetime_exists,
)
from dateutil._native import (
    gettz,
    tzfile,
    tzlocal,
    tzoffset,
    tzutc,
)
from dateutil._native import (
    resolve_imaginary as _native_resolve_imaginary,
)

UTC = tzutc()

_NATIVE_TZ_TYPES = (tzutc, tzoffset, tzfile, tzlocal)


def _resolve_tz(dt: datetime.datetime, tz: datetime.tzinfo | None) -> datetime.tzinfo:
    if tz is not None:
        return tz
    if dt.tzinfo is None:
        raise ValueError("Datetime is naive and no timezone provided.")
    return dt.tzinfo


def datetime_ambiguous(
    dt: datetime.datetime, tz: datetime.tzinfo | None = None
) -> bool:
    """Return True iff ``dt`` falls in an ambiguous wall-clock window."""
    resolved = _resolve_tz(dt, tz)
    if isinstance(resolved, _NATIVE_TZ_TYPES):
        return _native_datetime_ambiguous(dt.replace(tzinfo=None), resolved)

    # Generic tzinfo: try its own is_ambiguous, then fall back to fold compare.
    is_ambiguous_fn = getattr(resolved, "is_ambiguous", None)
    if is_ambiguous_fn is not None:
        try:
            return bool(is_ambiguous_fn(dt))
        except (AttributeError, NotImplementedError, TypeError):
            # AttributeError/NotImplementedError: explicit opt-out.
            # TypeError: incompatible signature (e.g. extra required args).
            pass

    wall = dt.replace(tzinfo=resolved)
    wall_0 = wall.replace(fold=0)
    wall_1 = wall.replace(fold=1)
    same_offset = wall_0.utcoffset() == wall_1.utcoffset()
    same_dst = wall_0.dst() == wall_1.dst()
    return not (same_offset and same_dst)


def datetime_exists(dt: datetime.datetime, tz: datetime.tzinfo | None = None) -> bool:
    """Return True iff ``dt`` is a real wall-clock time (not a DST gap)."""
    resolved = _resolve_tz(dt, tz)
    if isinstance(resolved, _NATIVE_TZ_TYPES):
        return _native_datetime_exists(dt.replace(tzinfo=None), resolved)

    wall = dt.replace(tzinfo=resolved)
    offset = wall.utcoffset()
    if offset is None:
        return True
    utc_equiv = (wall - offset).replace(tzinfo=resolved)
    return wall == utc_equiv


def enfold(dt: datetime.datetime, fold: int = 1) -> datetime.datetime:
    """Return a datetime with the ``fold`` attribute set to ``fold``."""
    return dt.replace(fold=fold)


def resolve_imaginary(dt: datetime.datetime) -> datetime.datetime:
    """Shift a non-existent wall-clock datetime forward by the DST gap."""
    if dt.tzinfo is None:
        return dt
    if isinstance(dt.tzinfo, _NATIVE_TZ_TYPES):
        naive = dt.replace(tzinfo=None)
        if _native_datetime_exists(naive, dt.tzinfo):
            return dt
        resolved_naive = _native_resolve_imaginary(naive, dt.tzinfo)
        return resolved_naive.replace(tzinfo=dt.tzinfo)
    if datetime_exists(dt):
        return dt
    # Generic tzinfo fallback: offsets 24h before and after bracket the gap.
    day = datetime.timedelta(hours=24)
    off_before = (dt - day).utcoffset()
    off_after = (dt + day).utcoffset()
    if off_before is None or off_after is None:
        return dt
    return dt + (off_after - off_before)


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
    "enfold",
    "gettz",
    "resolve_imaginary",
    "tzfile",
    "tzlocal",
    "tzoffset",
    "tzrange",
    "tzstr",
    "tzutc",
]
