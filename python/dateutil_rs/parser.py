"""dateutil_rs.parser - Date/time string parsing."""

from dateutil_rs._native import _ParserInfoBase, isoparse, parse, parse_to_dict


class parserinfo(_ParserInfoBase):
    """Customisable lookup tables for the parser.

    Subclass and override ``WEEKDAYS``, ``MONTHS``, etc. for non-English dates.
    """

    def __init__(self, dayfirst: bool = False, yearfirst: bool = False) -> None:
        # dayfirst/yearfirst are captured by __new__ (Rust side).
        # _build reads class variables (incl. subclass overrides).
        self._build(type(self))


__all__ = ["isoparse", "parse", "parse_to_dict", "parserinfo"]
