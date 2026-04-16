"""dateutil.parser - Date/time string parsing."""

from dateutil._native import (
    ParserError,
    UnknownTimezoneWarning,
    _ParserInfoBase,
    isoparser,
    parse,
    parse_to_dict,
)


class parserinfo(_ParserInfoBase):
    """Customisable lookup tables for the parser.

    Subclass and override ``WEEKDAYS``, ``MONTHS``, etc. for non-English dates.
    """

    def __init__(self, dayfirst: bool = False, yearfirst: bool = False) -> None:
        # dayfirst/yearfirst are captured by __new__ (Rust side).
        # _build reads class variables (incl. subclass overrides).
        self._build(type(self))


# Module-level convenience — matches python-dateutil's DEFAULT_ISOPARSER pattern
_DEFAULT_ISOPARSER = isoparser()
isoparse = _DEFAULT_ISOPARSER.isoparse

__all__ = [
    "ParserError",
    "UnknownTimezoneWarning",
    "isoparse",
    "isoparser",
    "parse",
    "parse_to_dict",
    "parserinfo",
]
