"""dateutil.parser - Date/time string parsing."""

from dateutil._native import _ParserInfoBase, isoparser, parse, parse_to_dict


class parserinfo(_ParserInfoBase):
    """Customisable lookup tables for the parser.

    Subclass and override ``WEEKDAYS``, ``MONTHS``, etc. for non-English dates.
    """

    def __init__(self, dayfirst: bool = False, yearfirst: bool = False) -> None:
        # dayfirst/yearfirst are captured by __new__ (Rust side).
        # _build reads class variables (incl. subclass overrides).
        self._build(type(self))


class ParserError(ValueError):
    """Exception raised when a string cannot be parsed as a date/time."""

    def __str__(self) -> str:
        try:
            return self.args[0] % self.args[1:]
        except (TypeError, IndexError):
            return super().__str__()


class UnknownTimezoneWarning(RuntimeWarning):
    """Warning raised when an unknown timezone string is found during parsing."""


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
