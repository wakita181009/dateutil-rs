"""Type stubs for the v1 Rust native extension module (dateutil-py)."""

from __future__ import annotations

import datetime
import sys
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

def parse(
    timestr: str,
    dayfirst: bool = False,
    yearfirst: bool = False,
    default: datetime.datetime | None = None,
) -> datetime.datetime: ...
def parse_to_dict(
    timestr: str,
    dayfirst: bool = False,
    yearfirst: bool = False,
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
    def weekday(self) -> weekday | None: ...
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
        weekday: weekday | None = None,
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
    def __eq__(self, other: object) -> bool: ...
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
        byweekday: list[int | weekday] | None = None,
        byhour: list[int] | None = None,
        byminute: list[int] | None = None,
        bysecond: list[int] | None = None,
    ) -> None: ...
    @property
    def freq(self) -> int: ...
    @property
    def dtstart(self) -> datetime.datetime: ...
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
    def __iter__(self) -> Self: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

_RRule = rrule

class rruleset:
    """Set of recurrence rules, dates, exclusion rules, and exclusion dates."""

    def __init__(self) -> None: ...
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
    def __iter__(self) -> Self: ...

def rrulestr(
    s: str,
    dtstart: datetime.datetime | None = None,
    forceset: bool = False,
    compatible: bool = False,
    unfold: bool = False,
) -> rrule | rruleset: ...
