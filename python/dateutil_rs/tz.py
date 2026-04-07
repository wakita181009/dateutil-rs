"""dateutil_rs.tz — Timezone support backed by Rust.

Provides timezone classes compatible with python-dateutil's tz module.
All classes inherit from datetime.tzinfo so they work with Python's datetime.
"""

from __future__ import annotations

import datetime
from typing import Any, TypeVar

from dateutil_rs._native import (
    _TzFile,
    _TzLocal,
    _TzOffset,
    _TzRange,
    _TzStr,
    _TzUtc,
)
from dateutil_rs._native import (
    gettz as _gettz_native,
)

_DateTimeT = TypeVar("_DateTimeT", bound=datetime.datetime)

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

ZERO = datetime.timedelta(0)


def _offset_to_seconds(offset: int | datetime.timedelta | None) -> int | None:
    """Convert an offset (None, int seconds, or timedelta) to seconds or None."""
    if offset is None:
        return None
    if isinstance(offset, datetime.timedelta):
        return int(offset.total_seconds())
    return int(offset)


def _relativedelta_to_rule(
    rule: object | None,
) -> tuple[int, int, int, int] | None:
    """Convert a relativedelta transition rule to (month, week, weekday, time_secs).

    Returns None if rule is None (Rust will use default US rules).
    Weekday uses POSIX convention: 0=Sunday, 6=Saturday.
    """
    if rule is None:
        return None

    month = getattr(rule, "month", None)
    wd = getattr(rule, "weekday", None)
    if month is None or wd is None:
        return None

    # python-dateutil weekday: MO=0, TU=1, ..., SU=6
    # POSIX/Rust DateRule weekday: 0=Sunday, 1=Monday, ..., 6=Saturday
    py_wd = wd.weekday
    rust_wd = (py_wd + 1) % 7

    # Week occurrence (1-5, where 5=last)
    week = wd.n if wd.n is not None else 1
    if week < 0:
        week = 5  # last occurrence

    # Time of day in seconds
    hours = getattr(rule, "hours", 0) or 0
    minutes = getattr(rule, "minutes", 0) or 0
    seconds = getattr(rule, "seconds", 0) or 0
    time_secs = hours * 3600 + minutes * 60 + seconds

    return month, week, rust_wd, time_secs


class tzutc(datetime.tzinfo):
    """UTC timezone.

    Singleton — all instances are the same object.
    """

    _instance: tzutc | None = None
    _inner: _TzUtc | None = None

    def __new__(cls) -> tzutc:
        if cls._instance is None:
            inst = super().__new__(cls)
            inst._inner = _TzUtc()
            cls._instance = inst
        return cls._instance

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta:
        return ZERO

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta:
        return ZERO

    def tzname(self, dt: datetime.datetime | None) -> str:
        return "UTC"

    def is_ambiguous(self, dt: datetime.datetime | None) -> bool:
        return False

    def fromutc(self, dt: _DateTimeT) -> _DateTimeT:
        return dt.replace(tzinfo=self)

    def __repr__(self) -> str:
        return "tzutc()"

    def __str__(self) -> str:
        return "tzutc()"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, tzutc):
            return True
        if isinstance(other, tzoffset) and other._offset == ZERO:
            return True
        return NotImplemented

    def __ne__(self, other: object) -> bool:
        result = self.__eq__(other)
        if result is NotImplemented:
            return result
        return not result

    __hash__ = None  # type: ignore[assignment]

    def __reduce__(self) -> tuple[type[tzutc], tuple[()]]:
        return self.__class__, ()


class tzoffset(datetime.tzinfo):
    """Fixed-offset timezone (no DST)."""

    def __init__(self, name: str | None, offset: float | datetime.timedelta) -> None:
        super().__init__()
        if isinstance(offset, datetime.timedelta):
            self._offset = offset
            self._offset_secs = int(offset.total_seconds())
        else:
            self._offset_secs = int(offset)
            self._offset = datetime.timedelta(seconds=self._offset_secs)
        self._name = name
        self._inner = _TzOffset(name, self._offset_secs)

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta:
        return self._offset

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta:
        return ZERO

    def tzname(self, dt: datetime.datetime | None) -> str | None:
        return self._name

    def is_ambiguous(self, dt: datetime.datetime | None) -> bool:
        return False

    def fromutc(self, dt: _DateTimeT) -> _DateTimeT:
        return (dt + self._offset).replace(tzinfo=self)

    def __repr__(self) -> str:
        return f"tzoffset({self._name!r}, {self._offset_secs})"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, tzutc) and self._offset == ZERO:
            return True
        if isinstance(other, tzoffset):
            return self._name == other._name and self._offset == other._offset
        return NotImplemented

    def __ne__(self, other: object) -> bool:
        result = self.__eq__(other)
        if result is NotImplemented:
            return result
        return not result

    __hash__ = None  # type: ignore[assignment]

    def __reduce__(self) -> tuple[type[tzoffset], tuple[str | None, int]]:
        return self.__class__, (self._name, self._offset_secs)


class tzfile(datetime.tzinfo):
    """Timezone loaded from a TZif binary file."""

    def __init__(self, fileobj: str | Any, filename: str | None = None) -> None:
        super().__init__()
        if isinstance(fileobj, str):
            self._filename = fileobj
        elif hasattr(fileobj, "name"):
            self._filename = fileobj.name
        elif filename is not None:
            self._filename = filename
        else:
            self._filename = repr(fileobj)
        self._inner = _TzFile(self._filename)

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta | None:
        return self._inner.utcoffset(dt)

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta | None:
        return self._inner.dst(dt)

    def tzname(self, dt: datetime.datetime | None) -> str | None:
        return self._inner.tzname(dt)

    def is_ambiguous(self, dt: datetime.datetime | None) -> bool:
        return self._inner.is_ambiguous(dt)

    def fromutc(self, dt: datetime.datetime) -> datetime.datetime:
        y, mo, d, h, mi, s, us, fold = self._inner.fromutc_naive(dt)
        return datetime.datetime(y, mo, d, h, mi, s, us, tzinfo=self, fold=int(fold))

    def __repr__(self) -> str:
        return f"tzfile('{self._filename}')"

    def __reduce__(self) -> tuple[type[tzfile], tuple[str]]:
        return self.__class__, (self._filename,)


class tzlocal(datetime.tzinfo):
    """System local timezone."""

    def __init__(self) -> None:
        super().__init__()
        self._inner = _TzLocal()

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta | None:
        return self._inner.utcoffset(dt)

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta | None:
        return self._inner.dst(dt)

    def tzname(self, dt: datetime.datetime | None) -> str | None:
        return self._inner.tzname(dt)

    def is_ambiguous(self, dt: datetime.datetime | None) -> bool:
        return self._inner.is_ambiguous(dt)

    def fromutc(self, dt: datetime.datetime) -> datetime.datetime:
        y, mo, d, h, mi, s, us, fold = self._inner.fromutc_naive(dt)
        return datetime.datetime(y, mo, d, h, mi, s, us, tzinfo=self, fold=int(fold))

    def __repr__(self) -> str:
        return "tzlocal()"

    def __reduce__(self) -> tuple[type[tzlocal], tuple[()]]:
        return self.__class__, ()


class tzrange(datetime.tzinfo):
    """Timezone with annual DST transitions defined by rules."""

    def __init__(
        self,
        stdabbr: str,
        stdoffset: int | datetime.timedelta | None = None,
        dstabbr: str | None = None,
        dstoffset: int | datetime.timedelta | None = None,
        start: object | None = None,
        end: object | None = None,
    ) -> None:
        super().__init__()
        std_secs = _offset_to_seconds(stdoffset)
        dst_secs = _offset_to_seconds(dstoffset)
        start_tuple = _relativedelta_to_rule(start)
        end_tuple = _relativedelta_to_rule(end)
        self._inner = _TzRange(
            stdabbr, std_secs, dstabbr, dst_secs, start_tuple, end_tuple
        )
        self._stdabbr = stdabbr
        self._dstabbr = dstabbr

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta | None:
        return self._inner.utcoffset(dt)

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta | None:
        return self._inner.dst(dt)

    def tzname(self, dt: datetime.datetime | None) -> str | None:
        return self._inner.tzname(dt)

    def is_ambiguous(self, dt: datetime.datetime | None) -> bool:
        return self._inner.is_ambiguous(dt)

    def fromutc(self, dt: datetime.datetime) -> datetime.datetime:
        y, mo, d, h, mi, s, us, fold = self._inner.fromutc_naive(dt)
        return datetime.datetime(y, mo, d, h, mi, s, us, tzinfo=self, fold=int(fold))

    def __repr__(self) -> str:
        return f"tzrange({self._stdabbr!r}, {self._dstabbr!r})"


class tzstr(datetime.tzinfo):
    """Timezone parsed from a POSIX TZ string."""

    def __init__(self, s: str, posix_offset: bool = False) -> None:
        super().__init__()
        self._s = s
        self._posix_offset = posix_offset
        self._inner = _TzStr(s, posix_offset)

    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta | None:
        return self._inner.utcoffset(dt)

    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta | None:
        return self._inner.dst(dt)

    def tzname(self, dt: datetime.datetime | None) -> str | None:
        return self._inner.tzname(dt)

    def is_ambiguous(self, dt: datetime.datetime | None) -> bool:
        return self._inner.is_ambiguous(dt)

    def fromutc(self, dt: datetime.datetime) -> datetime.datetime:
        y, mo, d, h, mi, s, us, fold = self._inner.fromutc_naive(dt)
        return datetime.datetime(y, mo, d, h, mi, s, us, tzinfo=self, fold=int(fold))

    def __repr__(self) -> str:
        return f"tzstr({self._s!r})"

    def __reduce__(self) -> tuple[type[tzstr], tuple[str, bool]]:
        return self.__class__, (self._s, self._posix_offset)


def gettz(name: str | None = None) -> datetime.tzinfo | None:
    """Get a timezone by name.

    Supports:
    - None / '' → system local timezone
    - 'UTC' / 'GMT' → UTC
    - IANA names (e.g. 'America/New_York')
    - Absolute file paths
    - POSIX TZ strings (e.g. 'EST5EDT,M3.2.0/2,M11.1.0/2')
    """
    result = _gettz_native(name)
    if result is None:
        return None

    # Wrap the native result in the appropriate Python class
    cls_name = type(result).__name__
    if cls_name == "_TzUtc":
        return tzutc()
    elif cls_name == "_TzOffset":
        return tzoffset(result.name(), result.offset_seconds())
    elif cls_name == "_TzFile":
        # Create a tzfile wrapper around the existing native object
        tz = datetime.tzinfo.__new__(tzfile)
        tz._inner = result
        tz._filename = result.filename() or name
        return tz
    elif cls_name == "_TzLocal":
        return tzlocal()
    elif cls_name == "_TzStr":
        tz = datetime.tzinfo.__new__(tzstr)
        tz._inner = result
        tz._s = result.source()
        tz._posix_offset = False
        return tz
    elif cls_name == "_TzRange":
        tz = datetime.tzinfo.__new__(tzrange)
        tz._inner = result
        tz._stdabbr = result.std_abbr()
        tz._dstabbr = result.dst_abbr()
        return tz
    else:
        # Fallback: return the raw native object
        return result


def datetime_exists(dt: datetime.datetime, tz: datetime.tzinfo | None = None) -> bool:
    """Check if a datetime exists in the given timezone.

    Returns False for datetimes in DST gaps (spring forward).
    """
    if tz is None:
        if dt.tzinfo is None:
            raise ValueError("Datetime is naive and no timezone provided")
        tz = dt.tzinfo

    naive = dt.replace(tzinfo=None)
    offset = tz.utcoffset(dt)
    if offset is None:
        return True

    utc_dt = (naive - offset).replace(tzinfo=tz)
    try:
        wall = tz.fromutc(utc_dt)
    except Exception:
        return True
    return wall.replace(tzinfo=None) == naive


def datetime_ambiguous(
    dt: datetime.datetime, tz: datetime.tzinfo | None = None
) -> bool:
    """Check if a datetime is ambiguous in the given timezone.

    Returns True for datetimes in DST overlaps (fall back).
    """
    if tz is None:
        if dt.tzinfo is None:
            raise ValueError("Datetime is naive and no timezone provided")
        tz = dt.tzinfo

    if hasattr(tz, "is_ambiguous"):
        return tz.is_ambiguous(dt)

    # Fallback: compare fold=0 and fold=1
    naive = dt.replace(tzinfo=None)
    dt0 = naive.replace(tzinfo=tz, fold=0)
    dt1 = naive.replace(tzinfo=tz, fold=1)
    return tz.utcoffset(dt0) != tz.utcoffset(dt1)


def resolve_imaginary(dt: _DateTimeT) -> _DateTimeT:
    """Resolve an imaginary datetime (in a DST gap) to a real one.

    Shifts the datetime forward by the gap amount.
    """
    if dt.tzinfo is None:
        return dt

    tz = dt.tzinfo
    naive = dt.replace(tzinfo=None)

    if datetime_exists(dt, tz):
        return dt

    # Round-trip through UTC to get the correct wall time
    offset = tz.utcoffset(dt)
    if offset is None:
        return dt
    utc_naive = naive - offset
    utc_dt = utc_naive.replace(tzinfo=tz)
    return tz.fromutc(utc_dt)


def enfold(dt: _DateTimeT, fold: int = 1) -> _DateTimeT:
    """Set the fold attribute on a datetime."""
    return dt.replace(fold=fold)


# Singleton UTC instance
UTC = tzutc()
