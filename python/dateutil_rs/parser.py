"""dateutil_rs.parser - Date/time string parsing.

Delegates to python-dateutil until Rust implementation is ready.
"""

from dateutil.parser import (
    ParserError,
    isoparse,
    isoparser,
    parse,
    parserinfo,
)

__all__ = [
    "ParserError",
    "isoparse",
    "isoparser",
    "parse",
    "parserinfo",
]
