"""dateutil_rs.parser - Date/time string parsing (Rust-accelerated).

Uses native Rust implementation for the default parser and isoparser.
When a custom parserinfo is supplied, its lookup tables are serialised
and forwarded to the Rust parser so parsing is always Rust-accelerated.
"""

from __future__ import annotations

import time
from collections.abc import Callable, Mapping
from datetime import datetime, tzinfo
from typing import Any, Literal, overload

from dateutil_rs._native import ParserError, isoparse, isoparser
from dateutil_rs._native import parse as _parse_rs

_TzData = tzinfo | int | str | None
_TzInfos = Mapping[str, _TzData] | Callable[[str, int], _TzData]

# ---- UnknownTimezoneWarning ------------------------------------------------
# Standalone definition — no python-dateutil dependency.


class UnknownTimezoneWarning(RuntimeWarning):
    """Raised when the parser finds a timezone it cannot parse into a tzinfo.

    .. versionadded:: 2.7.0
    """


# ---- parserinfo ------------------------------------------------------------
# Port of dateutil.parser.parserinfo.  Fully subclass-compatible.


class parserinfo:
    """Class which handles what inputs are accepted. Subclass this to customize
    the language and acceptable values for each parameter.

    :param dayfirst:
        Whether to interpret the first value in an ambiguous 3-integer date
        (e.g. 01/05/09) as the day (``True``) or month (``False``). If
        ``yearfirst`` is set to ``True``, this distinguishes between YDM
        and YMD. Default is ``False``.

    :param yearfirst:
        Whether to interpret the first value in an ambiguous 3-integer date
        (e.g. 01/05/09) as the year. If ``True``, the first number is taken
        to be the year, otherwise the last number is taken to be the year.
        Default is ``False``.
    """

    # m from a.m/p.m, t from ISO T separator
    JUMP: list[str] = [
        " ",
        ".",
        ",",
        ";",
        "-",
        "/",
        "'",
        "at",
        "on",
        "and",
        "ad",
        "m",
        "t",
        "of",
        "st",
        "nd",
        "rd",
        "th",
    ]

    WEEKDAYS: list[tuple[str, str]] = [
        ("Mon", "Monday"),
        ("Tue", "Tuesday"),
        ("Wed", "Wednesday"),
        ("Thu", "Thursday"),
        ("Fri", "Friday"),
        ("Sat", "Saturday"),
        ("Sun", "Sunday"),
    ]
    MONTHS: list[tuple[str, ...]] = [
        ("Jan", "January"),
        ("Feb", "February"),
        ("Mar", "March"),
        ("Apr", "April"),
        ("May", "May"),
        ("Jun", "June"),
        ("Jul", "July"),
        ("Aug", "August"),
        ("Sep", "Sept", "September"),
        ("Oct", "October"),
        ("Nov", "November"),
        ("Dec", "December"),
    ]
    HMS: list[tuple[str, str, str]] = [
        ("h", "hour", "hours"),
        ("m", "minute", "minutes"),
        ("s", "second", "seconds"),
    ]
    AMPM: list[tuple[str, str]] = [("am", "a"), ("pm", "p")]
    UTCZONE: list[str] = ["UTC", "GMT", "Z", "z"]
    PERTAIN: list[str] = ["of"]
    TZOFFSET: dict[str, int] = {}

    def __init__(self, dayfirst: bool = False, yearfirst: bool = False) -> None:
        self._jump = self._convert(self.JUMP)
        self._weekdays = self._convert(self.WEEKDAYS)
        self._months = self._convert(self.MONTHS)
        self._hms = self._convert(self.HMS)
        self._ampm = self._convert(self.AMPM)
        self._utczone = self._convert(self.UTCZONE)
        self._pertain = self._convert(self.PERTAIN)

        self.dayfirst = dayfirst
        self.yearfirst = yearfirst

        self._year = time.localtime().tm_year
        self._century = self._year // 100 * 100

    def _convert(self, lst: list[str] | list[tuple[str, ...]]) -> dict[str, int]:
        dct: dict[str, int] = {}
        for i, v in enumerate(lst):
            if isinstance(v, tuple):
                for v in v:
                    dct[v.lower()] = i
            else:
                dct[v.lower()] = i
        return dct

    def jump(self, name: str) -> bool:
        return name.lower() in self._jump

    def weekday(self, name: str) -> int | None:
        try:
            return self._weekdays[name.lower()]
        except KeyError:
            pass
        return None

    def month(self, name: str) -> int | None:
        try:
            return self._months[name.lower()] + 1
        except KeyError:
            pass
        return None

    def hms(self, name: str) -> int | None:
        try:
            return self._hms[name.lower()]
        except KeyError:
            return None

    def ampm(self, name: str) -> int | None:
        try:
            return self._ampm[name.lower()]
        except KeyError:
            return None

    def pertain(self, name: str) -> bool:
        return name.lower() in self._pertain

    def utczone(self, name: str) -> bool:
        return name.lower() in self._utczone

    def tzoffset(self, name: str) -> int | None:
        if name in self._utczone:
            return 0
        return self.TZOFFSET.get(name)

    def convertyear(self, year: int, century_specified: bool = False) -> int:
        """Converts two-digit years to year within [-50, 49]
        range of self._year (current local time)
        """
        assert year >= 0

        if year < 100 and not century_specified:
            year += self._century
            if year >= self._year + 50:
                year -= 100
            elif year < self._year - 50:
                year += 100

        return year

    def validate(self, res: Any) -> bool:
        if res.year is not None:
            res.year = self.convertyear(res.year, res.century_specified)

        if (res.tzoffset == 0 and not res.tzname) or (
            res.tzname == "Z" or res.tzname == "z"
        ):
            res.tzname = "UTC"
            res.tzoffset = 0
        elif res.tzoffset != 0 and res.tzname and self.utczone(res.tzname):
            res.tzoffset = 0
        return True

    def _to_rust_config(self) -> dict[str, Any]:
        """Serialise the lookup tables to a dict for the Rust parser."""
        return {
            "dayfirst": self.dayfirst,
            "yearfirst": self.yearfirst,
            "jump": self._jump,
            "weekdays": self._weekdays,
            "months": self._months,
            "hms": self._hms,
            "ampm": self._ampm,
            "utczone": self._utczone,
            "pertain": self._pertain,
            "tzoffset": dict(self.TZOFFSET),
        }


@overload
def parse(
    timestr: str,
    parserinfo: parserinfo | None = None,
    *,
    dayfirst: bool | None = ...,
    yearfirst: bool | None = ...,
    ignoretz: bool = ...,
    fuzzy: bool = ...,
    fuzzy_with_tokens: Literal[False] = False,
    default: datetime | None = ...,
    tzinfos: _TzInfos | None = ...,
) -> datetime: ...


@overload
def parse(
    timestr: str,
    parserinfo: parserinfo | None = None,
    *,
    dayfirst: bool | None = ...,
    yearfirst: bool | None = ...,
    ignoretz: bool = ...,
    fuzzy: bool = ...,
    fuzzy_with_tokens: Literal[True],
    default: datetime | None = ...,
    tzinfos: _TzInfos | None = ...,
) -> tuple[datetime, tuple[str, ...]]: ...


def parse(
    timestr: str,
    parserinfo: parserinfo | None = None,
    **kwargs: Any,
) -> datetime | tuple[datetime, tuple[str, ...]]:
    """Parse a date/time string.

    When *parserinfo* is ``None`` (the default), the fast Rust parser is
    used.  If a custom *parserinfo* is supplied, its lookup tables are
    forwarded to the Rust parser so parsing is still Rust-accelerated.
    """
    if parserinfo is not None:
        config = parserinfo._to_rust_config()
        return _parse_rs(timestr, parserinfo_config=config, **kwargs)

    return _parse_rs(timestr, **kwargs)


__all__ = [
    "ParserError",
    "UnknownTimezoneWarning",
    "isoparse",
    "isoparser",
    "parse",
    "parserinfo",
]
