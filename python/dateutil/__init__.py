"""dateutil - Fast date utility library for Python, powered by Rust."""

import os as _os
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _pkg_version

try:
    __version__ = _pkg_version("python-dateutil-rs")
except PackageNotFoundError:  # pragma: no cover - package not installed
    __version__ = "0.0.0"


def _bootstrap_tzdata() -> None:
    """Expose the tzdata PyPI package's zoneinfo dir to the Rust gettz()
    lookup via PYTHONTZPATH. Required on Windows (no system /usr/share/zoneinfo)
    and helpful anywhere a packaged tz database is preferred."""
    try:
        import tzdata  # type: ignore[import-not-found]
    except ImportError:
        return
    pkg_dir = _os.path.dirname(getattr(tzdata, "__file__", "") or "")
    if not pkg_dir:
        return
    zone_dir = _os.path.join(pkg_dir, "zoneinfo")
    if not _os.path.isdir(zone_dir):
        return
    existing = _os.environ.get("PYTHONTZPATH", "")
    parts = [p for p in existing.split(_os.pathsep) if p]
    if zone_dir in parts:
        return
    parts.insert(0, zone_dir)
    _os.environ["PYTHONTZPATH"] = _os.pathsep.join(parts)


_bootstrap_tzdata()

from dateutil._native import (
    DAILY,
    EASTER_JULIAN,
    EASTER_ORTHODOX,
    EASTER_WESTERN,
    FR,
    HOURLY,
    MINUTELY,
    MO,
    MONTHLY,
    SA,
    SECONDLY,
    SU,
    TH,
    TU,
    WE,
    WEEKLY,
    YEARLY,
    datetime_ambiguous,
    datetime_exists,
    easter,
    gettz,
    parse,
    parse_to_dict,
    relativedelta,
    resolve_imaginary,
    rrule,
    rruleset,
    rrulestr,
    tzfile,
    tzlocal,
    tzoffset,
    tzutc,
    weekday,
)
from dateutil.parser import isoparse, isoparser, parserinfo
from dateutil.utils import default_tzinfo, today, within_delta

__all__ = [
    "DAILY",
    "EASTER_JULIAN",
    "EASTER_ORTHODOX",
    "EASTER_WESTERN",
    "FR",
    "HOURLY",
    "MINUTELY",
    "MO",
    "MONTHLY",
    "SA",
    "SECONDLY",
    "SU",
    "TH",
    "TU",
    "WE",
    "WEEKLY",
    "YEARLY",
    "datetime_ambiguous",
    "datetime_exists",
    "default_tzinfo",
    "easter",
    "gettz",
    "isoparse",
    "parse",
    "parse_to_dict",
    "parserinfo",
    "relativedelta",
    "resolve_imaginary",
    "rrule",
    "rruleset",
    "rrulestr",
    "today",
    "tzfile",
    "tzlocal",
    "tzoffset",
    "tzutc",
    "weekday",
    "within_delta",
]
