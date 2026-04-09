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
    def __eq__(self, other: object) -> bool: ...
    def __bool__(self) -> bool: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
