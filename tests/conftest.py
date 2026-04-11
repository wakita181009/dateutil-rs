import os
import sys

import pytest

# ---------------------------------------------------------------------------
# --rust flag: redirect dateutil.* → dateutil_rs.* for Rust-ported modules
# ---------------------------------------------------------------------------
# Self-contained modules that can be redirected without breaking imports.
#
# NOT mapped (incompatible exports):
#   dateutil.parser — test_parser.py imports ParserError, UnknownTimezoneWarning,
#                     isoparser class, parser class which dateutil_rs does not provide
#   dateutil.tz     — tests import UTC constant, tzstr, tzrange, enfold, zoneinfo
#                     which dateutil_rs does not provide
#   dateutil.utils  — excluded from dateutil_rs scope
_MODULE_MAP = {
    "dateutil.easter": "dateutil_rs",
    "dateutil.relativedelta": "dateutil_rs",
    "dateutil.rrule": "dateutil_rs",
}


def pytest_addoption(parser):
    parser.addoption(
        "--rust",
        action="store_true",
        default=False,
        help="Test against dateutil_rs (Rust + python-dateutil hybrid)",
    )


def pytest_configure(config):
    if not config.getoption("--rust"):
        return

    try:
        import dateutil_rs
    except ImportError:
        pytest.exit("--rust requires dateutil_rs to be installed (run: uv sync)")

    for py_mod, rs_mod in _MODULE_MAP.items():
        rust_module = __import__(rs_mod, fromlist=[""])
        sys.modules[py_mod] = rust_module


# ---------------------------------------------------------------------------
# --rust xfail: tests for features dateutil_rs intentionally does not support
# ---------------------------------------------------------------------------
# These tests exercise python-dateutil features that are excluded from
# dateutil_rs scope (see CLAUDE.md "Excluded" section): float relativedelta
# args, tzical/VTIMEZONE, rrulestr TZID, rrule.xafter/replace, etc.
#
# Keyed by (file_stem, class_or_empty, test_name).
_RUST_XFAIL = {
    # -- relativedelta: float arguments not supported --
    ("test_relativedelta", "RelativeDeltaTest", "testAdditionFloatFractionals"),
    ("test_relativedelta", "RelativeDeltaTest", "testAdditionFloatValue"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalDays"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalHours"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalMinutes"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalMonth"),
    (
        "test_relativedelta",
        "RelativeDeltaTest",
        "testRelativeDeltaFractionalNegativeDays",
    ),
    (
        "test_relativedelta",
        "RelativeDeltaTest",
        "testRelativeDeltaFractionalNegativeOverflow",
    ),
    (
        "test_relativedelta",
        "RelativeDeltaTest",
        "testRelativeDeltaFractionalPositiveOverflow",
    ),
    (
        "test_relativedelta",
        "RelativeDeltaTest",
        "testRelativeDeltaFractionalPositiveOverflow2",
    ),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalRepr"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalSeconds"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalWeeks"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalYear"),
    (
        "test_relativedelta",
        "RelativeDeltaTest",
        "testRelativeDeltaNormalizeFractionalDays",
    ),
    (
        "test_relativedelta",
        "RelativeDeltaTest",
        "testRelativeDeltaNormalizeFractionalDays2",
    ),
    (
        "test_relativedelta",
        "RelativeDeltaTest",
        "testRelativeDeltaNormalizeFractionalMinutes",
    ),
    (
        "test_relativedelta",
        "RelativeDeltaTest",
        "testRelativeDeltaNormalizeFractionalSeconds",
    ),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalAbsolutes"),
    # -- rrule: unsupported features (TZID, aware dtstart) --
    ("test_rrule", "RRuleTest", "testStrSetExDateValueDateTimeWithTZID"),
    ("test_rrule", "RRuleTest", "testStrSetExDateWithTZID"),
    ("test_rrule", "RRuleTest", "testStrUntilMustBeUTC"),
    ("test_rrule", "RRuleTest", "testStrUntilWithTZ"),
    ("test_rrule", "RRuleTest", "testStrWithConflictingTZID"),
    ("test_rrule", "RRuleTest", "testStrWithTZID"),
    ("test_rrule", "RRuleTest", "testStrWithTZIDCallable"),
    ("test_rrule", "RRuleTest", "testStrWithTZIDCallableFailure"),
    ("test_rrule", "RRuleTest", "testStrWithTZIDMapping"),
    ("test_rrule", "", "test_generated_aware_dtstart"),
    # -- tz: TZICalTest — iCalendar VTIMEZONE not supported --
    ("test_tz", "TZICalTest", "testAmbiguousNegativeUTCOffset"),
    ("test_tz", "TZICalTest", "testAmbiguousPositiveUTCOffset"),
    ("test_tz", "TZICalTest", "testESTEndDST"),
    ("test_tz", "TZICalTest", "testESTEndName"),
    ("test_tz", "TZICalTest", "testESTEndOffset"),
    ("test_tz", "TZICalTest", "testESTStartDST"),
    ("test_tz", "TZICalTest", "testESTStartName"),
    ("test_tz", "TZICalTest", "testESTStartOffset"),
    ("test_tz", "TZICalTest", "testESTValueDatetime"),
    ("test_tz", "TZICalTest", "testFoldIndependence"),
    ("test_tz", "TZICalTest", "testFoldLondon"),
    ("test_tz", "TZICalTest", "testFoldNegativeUTCOffset"),
    ("test_tz", "TZICalTest", "testFoldPositiveUTCOffset"),
    ("test_tz", "TZICalTest", "testGap"),
    ("test_tz", "TZICalTest", "testGapNegativeUTCOffset"),
    ("test_tz", "TZICalTest", "testGapPositiveUTCOffset"),
    ("test_tz", "TZICalTest", "testImaginaryNegativeUTCOffset"),
    ("test_tz", "TZICalTest", "testImaginaryPositiveUTCOffset"),
    ("test_tz", "TZICalTest", "testInZoneFoldEquality"),
    ("test_tz", "TZICalTest", "testMultiZoneEndDST"),
    ("test_tz", "TZICalTest", "testMultiZoneEndName"),
    ("test_tz", "TZICalTest", "testMultiZoneEndOffset"),
    ("test_tz", "TZICalTest", "testMultiZoneGet"),
    ("test_tz", "TZICalTest", "testMultiZoneKeys"),
    ("test_tz", "TZICalTest", "testMultiZoneStartDST"),
    ("test_tz", "TZICalTest", "testMultiZoneStartName"),
    ("test_tz", "TZICalTest", "testMultiZoneStartOffset"),
    ("test_tz", "TZICalTest", "testNotImaginaryFoldNegativeUTCOffset"),
    ("test_tz", "TZICalTest", "testNotImaginaryFoldPositiveUTCOffset"),
    ("test_tz", "TZICalTest", "testNotImaginaryNegativeUTCOffset"),
    ("test_tz", "TZICalTest", "testNotImaginaryPositiveUTCOffset"),
    ("test_tz", "TZICalTest", "testRepr"),
    ("test_tz", "TZICalTest", "testUnambiguousGapNegativeUTCOffset"),
    ("test_tz", "TZICalTest", "testUnambiguousGapPositiveUTCOffset"),
    ("test_tz", "TZICalTest", "testUnambiguousNegativeUTCOffset"),
    ("test_tz", "TZICalTest", "testUnambiguousPositiveUTCOffset"),
}


def _make_xfail_key(item):
    """Build a (file_stem, class_name, test_name) key from a pytest Item."""
    file_stem = item.path.stem if hasattr(item, "path") else ""
    cls = item.cls.__name__ if item.cls else ""
    return file_stem, cls, item.name


# ---------------------------------------------------------------------------
# Upstream xfail removals: tests with @pytest.mark.xfail in the source that
# actually pass in the Rust implementation (would become XPASS(strict)).
# ---------------------------------------------------------------------------
_RUST_REMOVE_XFAIL = {
    ("test_rrule", "", "test_generated_aware_dtstart_rrulestr"),
}


# Configure pytest to ignore xfailing tests
# See: https://stackoverflow.com/a/53198349/467366
def pytest_collection_modifyitems(config, items):
    rust_mode = config.getoption("--rust", default=False)
    for item in items:
        # Auto-xfail unsupported tests under --rust
        if rust_mode and _make_xfail_key(item) in _RUST_XFAIL:
            item.add_marker(
                pytest.mark.xfail(
                    reason="not supported by dateutil_rs",
                    strict=True,
                )
            )
        # Remove upstream xfail markers for tests that pass in Rust
        if rust_mode and _make_xfail_key(item) in _RUST_REMOVE_XFAIL:
            item.own_markers = [m for m in item.own_markers if m.name != "xfail"]

        marker_getter = getattr(item, "get_closest_marker", None)

        # Python 3.3 support
        if marker_getter is None:
            marker_getter = item.get_marker

        marker = marker_getter("xfail")

        # Need to query the args because conditional xfail tests still have
        # the xfail mark even if they are not expected to fail
        if marker and (not marker.args or marker.args[0]):
            item.add_marker(pytest.mark.no_cover)


def set_tzpath():
    """
    Sets the TZPATH variable if it's specified in an environment variable.
    """
    tzpath = os.environ.get("DATEUTIL_TZPATH", None)

    if tzpath is None:
        return

    path_components = tzpath.split(":")

    print(f"Setting TZPATH to {path_components}")

    from dateutil import tz

    tz.TZPATHS.clear()
    tz.TZPATHS.extend(path_components)


set_tzpath()
