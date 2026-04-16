"""dateutil.parser - Date/time string parsing."""

from typing import Any

from dateutil._native import _ParserInfoBase, isoparser, parse_to_dict
from dateutil._native import parse as _native_parse


def _coerce_timestr(timestr: Any) -> str:
    """Accept str, bytes, bytearray, or a stream with ``.read()``."""
    if isinstance(timestr, str):
        return timestr
    if isinstance(timestr, (bytes, bytearray)):
        return bytes(timestr).decode("ascii")
    read = getattr(timestr, "read", None)
    if callable(read):
        data = read()
        if isinstance(data, (bytes, bytearray)):
            return bytes(data).decode("ascii")
        if isinstance(data, str):
            return data
    raise TypeError(
        f"Parser must be called with a string, bytes, bytearray, or stream, "
        f"got {type(timestr).__name__}"
    )


def parse(timestr: Any, *args: Any, **kwargs: Any) -> Any:
    """Parse a datetime string; accepts str, bytes, bytearray, or a stream."""
    try:
        return _native_parse(_coerce_timestr(timestr), *args, **kwargs)
    except ParserError:
        raise
    except ValueError as exc:
        raise ParserError(*exc.args) from exc


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
