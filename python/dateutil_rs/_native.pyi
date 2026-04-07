"""Type stubs for the Rust native extension module."""

from __future__ import annotations

import datetime
import sys
from collections.abc import Callable, Iterator, Mapping
from typing import Any, Final, Literal, SupportsFloat, TypeAlias, overload

if sys.version_info >= (3, 11):
    from typing import Self
else:
    from typing_extensions import Self

# Type aliases to avoid name shadowing in class definitions
_Weekday: TypeAlias = "weekday"
_RRule: TypeAlias = "rrule"

# ---------------------------------------------------------------------------
# common — Weekday
# ---------------------------------------------------------------------------

class weekday:
    weekday: int
    n: int | None
    def __init__(self, weekday: int, n: int | None = None) -> None: ...
    def __call__(self, n: int) -> Self: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...

MO: weekday
TU: weekday
WE: weekday
TH: weekday
FR: weekday
SA: weekday
SU: weekday

# ---------------------------------------------------------------------------
# easter
# ---------------------------------------------------------------------------

EASTER_JULIAN: Final = 1
EASTER_ORTHODOX: Final = 2
EASTER_WESTERN: Final = 3

def easter(year: int, method: Literal[1, 2, 3] = 3) -> datetime.date: ...

# ---------------------------------------------------------------------------
# utils
# ---------------------------------------------------------------------------

def within_delta(
    dt1: datetime.datetime, dt2: datetime.datetime, delta: datetime.timedelta
) -> bool: ...
def today(tzinfo: datetime.tzinfo | None = None) -> datetime.datetime: ...
def default_tzinfo(
    dt: datetime.datetime, tzinfo: datetime.tzinfo
) -> datetime.datetime: ...

# ---------------------------------------------------------------------------
# relativedelta
# ---------------------------------------------------------------------------

class relativedelta:
    years: int
    months: int
    days: int
    leapdays: int
    hours: int
    minutes: int
    seconds: int
    microseconds: int
    year: int | None
    month: int | None
    day: int | None
    weekday: weekday | None
    hour: int | None
    minute: int | None
    second: int | None
    microsecond: int | None
    def __init__(
        self,
        dt1: datetime.date | None = None,
        dt2: datetime.date | None = None,
        *,
        years: float = 0,
        months: float = 0,
        days: float = 0,
        leapdays: int = 0,
        weeks: float = 0,
        hours: float = 0,
        minutes: float = 0,
        seconds: float = 0,
        microseconds: float = 0,
        year: int | None = None,
        month: int | None = None,
        day: int | None = None,
        weekday: int | _Weekday | None = None,
        yearday: int | None = None,
        nlyearday: int | None = None,
        hour: int | None = None,
        minute: int | None = None,
        second: int | None = None,
        microsecond: int | None = None,
    ) -> None: ...
    @property
    def weeks(self) -> int: ...
    @weeks.setter
    def weeks(self, value: float) -> None: ...
    def normalized(self) -> Self: ...
    @overload
    def __add__(self, other: datetime.timedelta | relativedelta) -> Self: ...
    @overload
    def __add__(self, other: datetime.datetime) -> datetime.datetime: ...
    @overload
    def __add__(self, other: datetime.date) -> datetime.date: ...
    @overload
    def __radd__(self, other: datetime.timedelta | relativedelta) -> Self: ...
    @overload
    def __radd__(self, other: datetime.datetime) -> datetime.datetime: ...
    @overload
    def __radd__(self, other: datetime.date) -> datetime.date: ...
    @overload
    def __rsub__(self, other: datetime.timedelta | relativedelta) -> Self: ...
    @overload
    def __rsub__(self, other: datetime.datetime) -> datetime.datetime: ...
    @overload
    def __rsub__(self, other: datetime.date) -> datetime.date: ...
    def __sub__(self, other: relativedelta) -> Self: ...
    def __neg__(self) -> Self: ...
    def __bool__(self) -> bool: ...
    def __mul__(self, other: SupportsFloat) -> Self: ...
    def __rmul__(self, other: SupportsFloat) -> Self: ...
    def __truediv__(self, other: SupportsFloat) -> Self: ...
    def __abs__(self) -> Self: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# parser
# ---------------------------------------------------------------------------

class ParserError(ValueError): ...

class isoparser:
    def __init__(self, sep: str | None = None) -> None: ...
    def isoparse(self, dt_str: str) -> datetime.datetime: ...

def isoparse(dt_str: str) -> datetime.datetime: ...

_TzData = datetime.tzinfo | int | str | None
_TzInfos = Mapping[str, _TzData] | Callable[[str, int], _TzData]

@overload
def parse(
    timestr: str,
    *,
    parserinfo_config: dict[str, Any] | None = None,
    default: datetime.datetime | None = None,
    ignoretz: bool = False,
    tzinfos: _TzInfos | None = None,
    dayfirst: bool | None = None,
    yearfirst: bool | None = None,
    fuzzy: bool = False,
    fuzzy_with_tokens: Literal[False] = False,
) -> datetime.datetime: ...
@overload
def parse(
    timestr: str,
    *,
    parserinfo_config: dict[str, Any] | None = None,
    default: datetime.datetime | None = None,
    ignoretz: bool = False,
    tzinfos: _TzInfos | None = None,
    dayfirst: bool | None = None,
    yearfirst: bool | None = None,
    fuzzy: bool = False,
    fuzzy_with_tokens: Literal[True],
) -> tuple[datetime.datetime, tuple[str, ...]]: ...

# ---------------------------------------------------------------------------
# rrule
# ---------------------------------------------------------------------------

YEARLY: Final = 0
MONTHLY: Final = 1
WEEKLY: Final = 2
DAILY: Final = 3
HOURLY: Final = 4
MINUTELY: Final = 5
SECONDLY: Final = 6

class rrule:
    def __init__(
        self,
        freq: Literal[0, 1, 2, 3, 4, 5, 6],
        dtstart: datetime.date | None = None,
        interval: int = 1,
        wkst: weekday | int | None = None,
        count: int | None = None,
        until: datetime.date | None = None,
        bysetpos: int | list[int] | None = None,
        bymonth: int | list[int] | None = None,
        bymonthday: int | list[int] | None = None,
        byyearday: int | list[int] | None = None,
        byeaster: int | list[int] | None = None,
        byweekno: int | list[int] | None = None,
        byweekday: int | weekday | list[int] | list[weekday] | None = None,
        byhour: int | list[int] | None = None,
        byminute: int | list[int] | None = None,
        bysecond: int | list[int] | None = None,
        cache: bool = False,
    ) -> None: ...
    def __iter__(self) -> Iterator[datetime.datetime]: ...
    @overload
    def __getitem__(self, item: int) -> datetime.datetime: ...
    @overload
    def __getitem__(self, item: slice) -> list[datetime.datetime]: ...
    def __contains__(self, item: datetime.datetime) -> bool: ...
    def __len__(self) -> int: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def count(self) -> int: ...
    def before(
        self, dt: datetime.datetime, inc: bool = False
    ) -> datetime.datetime | None: ...
    def after(
        self, dt: datetime.datetime, inc: bool = False
    ) -> datetime.datetime | None: ...
    def xafter(
        self,
        dt: datetime.datetime,
        count: int | None = None,
        inc: bool = False,
    ) -> Iterator[datetime.datetime]: ...
    def between(
        self,
        after: datetime.datetime,
        before: datetime.datetime,
        inc: bool = False,
        count: int | None = None,
    ) -> list[datetime.datetime]: ...
    def replace(self, **kwargs: Any) -> rrule: ...

class rruleset:
    def __init__(self, cache: bool = False) -> None: ...
    def rrule(self, rrule: _RRule) -> None: ...
    def rdate(self, rdate: datetime.datetime) -> None: ...
    def exrule(self, exrule: _RRule) -> None: ...
    def exdate(self, exdate: datetime.datetime) -> None: ...
    def __iter__(self) -> Iterator[datetime.datetime]: ...
    @overload
    def __getitem__(self, item: int) -> datetime.datetime: ...
    @overload
    def __getitem__(self, item: slice) -> list[datetime.datetime]: ...
    def __contains__(self, item: datetime.datetime) -> bool: ...
    def __len__(self) -> int: ...
    def count(self) -> int: ...
    def before(
        self, dt: datetime.datetime, inc: bool = False
    ) -> datetime.datetime | None: ...
    def after(
        self, dt: datetime.datetime, inc: bool = False
    ) -> datetime.datetime | None: ...
    def between(
        self,
        after: datetime.datetime,
        before: datetime.datetime,
        inc: bool = False,
        count: int | None = None,
    ) -> list[datetime.datetime]: ...

@overload
def rrulestr(
    s: str,
    *,
    dtstart: datetime.date | None = None,
    cache: bool = False,
    unfold: bool = False,
    forceset: Literal[True],
    compatible: bool = False,
    ignoretz: bool = False,
    tzids: Mapping[str, datetime.tzinfo] | None = None,
    tzinfos: _TzInfos | None = None,
) -> rruleset: ...
@overload
def rrulestr(
    s: str,
    *,
    dtstart: datetime.date | None = None,
    cache: bool = False,
    unfold: bool = False,
    forceset: bool = False,
    compatible: Literal[True],
    ignoretz: bool = False,
    tzids: Mapping[str, datetime.tzinfo] | None = None,
    tzinfos: _TzInfos | None = None,
) -> rruleset: ...
@overload
def rrulestr(
    s: str,
    *,
    dtstart: datetime.date | None = None,
    cache: bool = False,
    unfold: bool = False,
    forceset: bool = False,
    compatible: bool = False,
    ignoretz: bool = False,
    tzids: Mapping[str, datetime.tzinfo] | None = None,
    tzinfos: _TzInfos | None = None,
) -> rrule | rruleset: ...

# ---------------------------------------------------------------------------
# tz (internal native types — wrapped by python/dateutil_rs/tz.py)
# ---------------------------------------------------------------------------

class _TzUtc:
    def __init__(self) -> None: ...
    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta: ...
    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta: ...
    def tzname(self, dt: datetime.datetime | None) -> str: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def __repr__(self) -> str: ...

class _TzOffset:
    def __init__(self, name: str | None, offset: int) -> None: ...
    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta: ...
    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta: ...
    def tzname(self, dt: datetime.datetime | None) -> str | None: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def offset_seconds(self) -> int: ...
    def name(self) -> str | None: ...
    def __repr__(self) -> str: ...

class _TzFile:
    def __init__(self, path: str) -> None: ...
    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta | None: ...
    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta | None: ...
    def tzname(self, dt: datetime.datetime | None) -> str | None: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def fromutc_naive(
        self, dt: datetime.datetime
    ) -> tuple[int, int, int, int, int, int, int, bool]: ...
    def filename(self) -> str | None: ...
    def __repr__(self) -> str: ...

class _TzLocal:
    def __init__(self) -> None: ...
    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta | None: ...
    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta | None: ...
    def tzname(self, dt: datetime.datetime | None) -> str | None: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def fromutc_naive(
        self, dt: datetime.datetime
    ) -> tuple[int, int, int, int, int, int, int, bool]: ...
    def __repr__(self) -> str: ...

class _TzStr:
    def __init__(self, s: str, posix_offset: bool = False) -> None: ...
    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta | None: ...
    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta | None: ...
    def tzname(self, dt: datetime.datetime | None) -> str | None: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def fromutc_naive(
        self, dt: datetime.datetime
    ) -> tuple[int, int, int, int, int, int, int, bool]: ...
    def source(self) -> str: ...
    def __repr__(self) -> str: ...

class _TzRange:
    def __init__(
        self,
        std_abbr: str,
        std_offset: int | None = None,
        dst_abbr: str | None = None,
        dst_offset: int | None = None,
        start: tuple[int, int, int, int] | None = None,
        end: tuple[int, int, int, int] | None = None,
    ) -> None: ...
    def utcoffset(self, dt: datetime.datetime | None) -> datetime.timedelta | None: ...
    def dst(self, dt: datetime.datetime | None) -> datetime.timedelta | None: ...
    def tzname(self, dt: datetime.datetime | None) -> str | None: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def fromutc_naive(
        self, dt: datetime.datetime
    ) -> tuple[int, int, int, int, int, int, int, bool]: ...
    def std_abbr(self) -> str: ...
    def dst_abbr(self) -> str | None: ...
    def __repr__(self) -> str: ...

def gettz(name: str | None = None) -> Any: ...
def datetime_exists(dt: datetime.datetime, tz: datetime.tzinfo) -> bool: ...
def datetime_ambiguous(dt: datetime.datetime, tz: datetime.tzinfo) -> bool: ...
