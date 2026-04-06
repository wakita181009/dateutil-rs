"""dateutil_rs.parser - Date/time string parsing (Rust-accelerated).

Uses native Rust implementation for the default parser and isoparser.
Falls back to python-dateutil for custom parserinfo subclasses.
"""

# Re-export parserinfo and UnknownTimezoneWarning from python-dateutil
# (these are Python-only types not yet ported to Rust)
from dateutil.parser import UnknownTimezoneWarning, parserinfo
from dateutil_rs._native import ParserError, isoparse, isoparser
from dateutil_rs._native import parse as _parse_rs


def parse(timestr, parserinfo=None, **kwargs):
    """Parse a date/time string.

    When *parserinfo* is ``None`` (the default), the fast Rust parser is
    used.  If a custom *parserinfo* is supplied, falls back to the
    original python-dateutil implementation.
    """
    if parserinfo is not None:
        # Custom parserinfo → fall back to pure-Python dateutil
        from dateutil.parser import parser as _parser

        return _parser(parserinfo).parse(timestr, **kwargs)

    return _parse_rs(timestr, **kwargs)


__all__ = [
    "ParserError",
    "UnknownTimezoneWarning",
    "isoparse",
    "isoparser",
    "parse",
    "parserinfo",
]
