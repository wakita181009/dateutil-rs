"""Type stubs for the v1 Rust native extension module (dateutil-py)."""

from __future__ import annotations

import datetime
import sys
from collections.abc import Callable, Iterator, Mapping
from typing import Final, Literal, TypedDict

if sys.version_info >= (3, 11):
    from typing import Self
else:
    from typing_extensions import Self

# ---------------------------------------------------------------------------
# common — Weekday
# ---------------------------------------------------------------------------

class weekday:
    """Weekday with optional N-th occurrence qualifier.

    weekday: 0=Monday .. 6=Sunday
    n: N-th occurrence (e.g. 2 = "2nd Monday"), None for unqualified.
    """

    @property
    def weekday(self) -> int: ...
    @property
    def n(self) -> int | None: ...
    def __init__(self, weekday: int, n: int | None = None) -> None: ...
    def __call__(self, n: int | None = None) -> Self: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

MO: weekday
TU: weekday
WE: weekday
TH: weekday
FR: weekday
SA: weekday
SU: weekday

_Weekday = weekday

# ---------------------------------------------------------------------------
# easter
# ---------------------------------------------------------------------------

EASTER_JULIAN: Final = 1
EASTER_ORTHODOX: Final = 2
EASTER_WESTERN: Final = 3

def easter(year: int, method: Literal[1, 2, 3] = 3) -> datetime.date: ...

# ---------------------------------------------------------------------------
# parser
# ---------------------------------------------------------------------------

class ParseResultDict(TypedDict):
    year: int | None
    month: int | None
    day: int | None
    weekday: int | None
    hour: int | None
    minute: int | None
    second: int | None
    microsecond: int | None
    tzname: str | None
    tzoffset: int | None

class _ParserInfoBase:
    """Customisable lookup tables for the parser.

    Subclass and override the class variables to support non-English dates.
    """

    JUMP: list[str]
    WEEKDAYS: list[tuple[str, str]]
    MONTHS: list[list[str]]
    HMS: list[tuple[str, str, str]]
    AMPM: list[tuple[str, str]]
    UTCZONE: list[str]
    PERTAIN: list[str]
    TZOFFSET: dict[str, int]
    dayfirst: bool
    yearfirst: bool
    def __init__(
        self,
        dayfirst: bool | None = None,
        yearfirst: bool | None = None,
    ) -> None: ...
    def __repr__(self) -> str: ...

_TzData = datetime.tzinfo | int | str | None
_TzInfos = Mapping[str, _TzData] | Callable[[str, int], _TzData]

def parse(
    timestr: str,
    parserinfo: _ParserInfoBase | None = None,
    *,
    dayfirst: bool | None = None,
    yearfirst: bool | None = None,
    default: datetime.datetime | None = None,
    ignoretz: bool = False,
    tzinfos: _TzInfos | None = None,
) -> datetime.datetime: ...
def parse_to_dict(
    timestr: str,
    *,
    parserinfo: _ParserInfoBase | None = None,
    dayfirst: bool | None = None,
    yearfirst: bool | None = None,
) -> ParseResultDict: ...
def isoparse(dt_str: str) -> datetime.datetime: ...

# ---------------------------------------------------------------------------
# relativedelta
# ---------------------------------------------------------------------------

class relativedelta:
    """Relative date/time delta."""

    @property
    def years(self) -> int: ...
    @property
    def months(self) -> int: ...
    @property
    def days(self) -> int: ...
    @property
    def hours(self) -> int: ...
    @property
    def minutes(self) -> int: ...
    @property
    def seconds(self) -> int: ...
    @property
    def microseconds(self) -> int: ...
    @property
    def weeks(self) -> int: ...
    @property
    def leapdays(self) -> int: ...
    @property
    def year(self) -> int | None: ...
    @property
    def month(self) -> int | None: ...
    @property
    def day(self) -> int | None: ...
    @property
    def hour(self) -> int | None: ...
    @property
    def minute(self) -> int | None: ...
    @property
    def second(self) -> int | None: ...
    @property
    def microsecond(self) -> int | None: ...
    @property
    def weekday(self) -> _Weekday | None: ...
    def __init__(
        self,
        years: int = 0,
        months: int = 0,
        days: int = 0,
        weeks: int = 0,
        hours: int = 0,
        minutes: int = 0,
        seconds: int = 0,
        microseconds: int = 0,
        leapdays: int = 0,
        year: int | None = None,
        month: int | None = None,
        day: int | None = None,
        weekday: _Weekday | None = None,
        yearday: int | None = None,
        nlyearday: int | None = None,
        hour: int | None = None,
        minute: int | None = None,
        second: int | None = None,
        microsecond: int | None = None,
    ) -> None: ...
    @staticmethod
    def from_diff(
        dt1: datetime.datetime,
        dt2: datetime.datetime,
    ) -> relativedelta: ...
    def add_to_datetime(
        self,
        dt: datetime.datetime,
    ) -> datetime.datetime: ...
    def add_to_date(self, dt: datetime.date) -> datetime.date: ...
    def has_time(self) -> bool: ...
    def is_zero(self) -> bool: ...
    def __add__(self, other: relativedelta) -> Self: ...
    def __sub__(self, other: relativedelta) -> Self: ...
    def __neg__(self) -> Self: ...
    def __mul__(self, factor: int | float) -> Self: ...
    def __rmul__(self, factor: int | float) -> Self: ...
    def __truediv__(self, factor: int | float) -> Self: ...
    def __abs__(self) -> Self: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# ---------------------------------------------------------------------------
# rrule — Recurrence rules (RFC 5545)
# ---------------------------------------------------------------------------

YEARLY: Final[int]
MONTHLY: Final[int]
WEEKLY: Final[int]
DAILY: Final[int]
HOURLY: Final[int]
MINUTELY: Final[int]
SECONDLY: Final[int]

class rrule:
    """RFC 5545 recurrence rule."""

    def __init__(
        self,
        freq: int,
        dtstart: datetime.datetime | None = None,
        interval: int = 1,
        wkst: int | None = None,
        count: int | None = None,
        until: datetime.datetime | None = None,
        bysetpos: list[int] | None = None,
        bymonth: list[int] | None = None,
        bymonthday: list[int] | None = None,
        byyearday: list[int] | None = None,
        byeaster: list[int] | None = None,
        byweekno: list[int] | None = None,
        byweekday: list[int | _Weekday] | None = None,
        byhour: list[int] | None = None,
        byminute: list[int] | None = None,
        bysecond: list[int] | None = None,
        cache: bool = False,
    ) -> None: ...
    @property
    def freq(self) -> int: ...
    @property
    def dtstart(self) -> datetime.datetime: ...
    @property
    def interval(self) -> int: ...
    @property
    def wkst(self) -> int: ...
    @property
    def count(self) -> int | None: ...
    @property
    def until(self) -> datetime.datetime | None: ...
    @property
    def bysetpos(self) -> list[int] | None: ...
    @property
    def bymonth(self) -> list[int] | None: ...
    @property
    def byyearday(self) -> list[int] | None: ...
    @property
    def byeaster(self) -> list[int] | None: ...
    @property
    def byweekno(self) -> list[int] | None: ...
    @property
    def byweekday(self) -> list[_Weekday] | None: ...
    @property
    def byhour(self) -> list[int] | None: ...
    @property
    def byminute(self) -> list[int] | None: ...
    @property
    def bysecond(self) -> list[int] | None: ...
    def all(self) -> list[datetime.datetime]: ...
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
    ) -> list[datetime.datetime]: ...
    def __iter__(self) -> Iterator[datetime.datetime]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

_RRule = rrule

class rruleset:
    """Set of recurrence rules, dates, exclusion rules, and exclusion dates."""

    def __init__(self, cache: bool = False) -> None: ...
    def rrule(self, rule: _RRule) -> None: ...
    def rdate(self, dt: datetime.datetime) -> None: ...
    def exrule(self, rule: _RRule) -> None: ...
    def exdate(self, dt: datetime.datetime) -> None: ...
    def all(self) -> list[datetime.datetime]: ...
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
    ) -> list[datetime.datetime]: ...
    def __iter__(self) -> Iterator[datetime.datetime]: ...

def rrulestr(
    s: str,
    dtstart: datetime.datetime | None = None,
    forceset: bool = False,
    compatible: bool = False,
    unfold: bool = False,
    cache: bool = False,
) -> rrule | rruleset: ...

# ---------------------------------------------------------------------------
# tz — Timezone types and utilities
# ---------------------------------------------------------------------------

class tzutc:
    """UTC timezone (zero offset, no DST)."""

    def __init__(self) -> None: ...
    def utcoffset(self, dt: datetime.datetime | None = None) -> datetime.timedelta: ...
    def dst(self, dt: datetime.datetime | None = None) -> datetime.timedelta: ...
    def tzname(self, dt: datetime.datetime | None = None) -> str: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def fromutc(self, dt: datetime.datetime) -> datetime.datetime: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class tzoffset:
    """Fixed UTC offset timezone (no DST)."""

    def __init__(
        self,
        name: str | None = None,
        offset: int = 0,
    ) -> None: ...
    def utcoffset(self, dt: datetime.datetime | None = None) -> datetime.timedelta: ...
    def dst(self, dt: datetime.datetime | None = None) -> datetime.timedelta: ...
    def tzname(self, dt: datetime.datetime | None = None) -> str: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def fromutc(self, dt: datetime.datetime) -> datetime.datetime: ...
    def __repr__(self) -> str: ...

class tzfile:
    """TZif file-based timezone (DST-aware)."""

    def __init__(self, path: str) -> None: ...
    def utcoffset(
        self, dt: datetime.datetime, fold: bool = False
    ) -> datetime.timedelta: ...
    def dst(self, dt: datetime.datetime, fold: bool = False) -> datetime.timedelta: ...
    def tzname(self, dt: datetime.datetime, fold: bool = False) -> str: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def fromutc(self, dt: datetime.datetime) -> datetime.datetime: ...
    def __repr__(self) -> str: ...

class tzlocal:
    """System local timezone."""

    def __init__(self) -> None: ...
    def utcoffset(
        self, dt: datetime.datetime, fold: bool = False
    ) -> datetime.timedelta: ...
    def dst(self, dt: datetime.datetime, fold: bool = False) -> datetime.timedelta: ...
    def tzname(self, dt: datetime.datetime, fold: bool = False) -> str: ...
    def is_ambiguous(self, dt: datetime.datetime) -> bool: ...
    def fromutc(self, dt: datetime.datetime) -> datetime.datetime: ...
    def __repr__(self) -> str: ...

_Tz = tzutc | tzoffset | tzfile | tzlocal

def gettz(name: str | None = None) -> _Tz: ...
def datetime_exists(dt: datetime.datetime, tz: _Tz) -> bool: ...
def datetime_ambiguous(dt: datetime.datetime, tz: _Tz) -> bool: ...
def resolve_imaginary(
    dt: datetime.datetime,
    tz: _Tz,
) -> datetime.datetime: ...
