"""dateutil_rs.v1.parser - Date/time string parsing."""

from dateutil_rs.v1._native import _ParserInfoBase, isoparse, parse, parse_to_dict


class parserinfo(_ParserInfoBase):
    """Customisable lookup tables for the parser.

    Subclass and override ``WEEKDAYS``, ``MONTHS``, etc. for non-English dates.
    """

    def __init__(self, **_: object) -> None:
        # __new__ (Rust) already set dayfirst/yearfirst.
        # _build reads class variables (including subclass overrides).
        self._build(type(self))


__all__ = ["isoparse", "parse", "parse_to_dict", "parserinfo"]
