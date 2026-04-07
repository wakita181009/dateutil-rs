"""dateutil_rs.parser - Date/time string parsing (Rust-accelerated).

Uses native Rust implementation for the default parser and isoparser.
When a custom parserinfo is supplied, its lookup tables are serialised
and forwarded to the Rust parser so parsing is always Rust-accelerated.
"""

import time

from dateutil_rs._native import ParserError, isoparse, isoparser
from dateutil_rs._native import parse as _parse_rs

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
    JUMP = [
        " ", ".", ",", ";", "-", "/", "'",
        "at", "on", "and", "ad", "m", "t", "of",
        "st", "nd", "rd", "th",
    ]

    WEEKDAYS = [
        ("Mon", "Monday"),
        ("Tue", "Tuesday"),
        ("Wed", "Wednesday"),
        ("Thu", "Thursday"),
        ("Fri", "Friday"),
        ("Sat", "Saturday"),
        ("Sun", "Sunday"),
    ]
    MONTHS = [
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
    HMS = [
        ("h", "hour", "hours"),
        ("m", "minute", "minutes"),
        ("s", "second", "seconds"),
    ]
    AMPM = [("am", "a"), ("pm", "p")]
    UTCZONE = ["UTC", "GMT", "Z", "z"]
    PERTAIN = ["of"]
    TZOFFSET = {}

    def __init__(self, dayfirst=False, yearfirst=False):
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

    def _convert(self, lst):
        dct = {}
        for i, v in enumerate(lst):
            if isinstance(v, tuple):
                for v in v:
                    dct[v.lower()] = i
            else:
                dct[v.lower()] = i
        return dct

    def jump(self, name):
        return name.lower() in self._jump

    def weekday(self, name):
        try:
            return self._weekdays[name.lower()]
        except KeyError:
            pass
        return None

    def month(self, name):
        try:
            return self._months[name.lower()] + 1
        except KeyError:
            pass
        return None

    def hms(self, name):
        try:
            return self._hms[name.lower()]
        except KeyError:
            return None

    def ampm(self, name):
        try:
            return self._ampm[name.lower()]
        except KeyError:
            return None

    def pertain(self, name):
        return name.lower() in self._pertain

    def utczone(self, name):
        return name.lower() in self._utczone

    def tzoffset(self, name):
        if name in self._utczone:
            return 0
        return self.TZOFFSET.get(name)

    def convertyear(self, year, century_specified=False):
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

    def validate(self, res):
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

    def _to_rust_config(self):
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


def parse(timestr, parserinfo=None, **kwargs):
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
