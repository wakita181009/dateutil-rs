import pytest

# ---------------------------------------------------------------------------
# xfail: tests for features dateutil intentionally does not support
# ---------------------------------------------------------------------------
# These tests exercise python-dateutil features excluded from dateutil-rs
# scope (see CLAUDE.md "Excluded" section).

# Classes whose *every* method is unsupported (xfail the whole class).
_XFAIL_CLASSES = {
    ("test_tz", "TZICalTest"),  # iCalendar VTIMEZONE not supported
    ("test_tz", "TZRangeTest"),  # POSIX tzrange not supported
    ("test_tz", "TzPickleFileTest"),  # Rust objects not picklable
    ("test_tz", "TestEnfold"),  # enfold() not supported
}

# Individual tests that are unsupported.
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
}


def _make_xfail_key(item):
    """Build a (file_stem, class_name, test_name) key from a pytest Item."""
    file_stem = item.path.stem if hasattr(item, "path") else ""
    cls = item.cls.__name__ if item.cls else ""
    return file_stem, cls, item.name


def _should_xfail(item):
    """Return True if a test is known-unsupported in dateutil-rs."""
    file_stem = item.path.stem if hasattr(item, "path") else ""
    cls = item.cls.__name__ if item.cls else ""
    # Whole-class match
    if (file_stem, cls) in _XFAIL_CLASSES:
        return True
    # Individual test match
    return (file_stem, cls, item.name) in _RUST_XFAIL


# ---------------------------------------------------------------------------
# Upstream xfail removals: tests with @pytest.mark.xfail in the source that
# actually pass in the Rust implementation (would become XPASS(strict)).
# ---------------------------------------------------------------------------
_RUST_REMOVE_XFAIL = {
    ("test_rrule", "", "test_generated_aware_dtstart_rrulestr"),
    ("test_isoparser", "", "test_isoparser_byte_sep"),
}


# Configure pytest to ignore xfailing tests
# See: https://stackoverflow.com/a/53198349/467366
def pytest_collection_modifyitems(config, items):
    for item in items:
        if _should_xfail(item):
            item.add_marker(
                pytest.mark.xfail(
                    reason="not supported by dateutil-rs",
                    strict=True,
                )
            )

        # Remove upstream xfail markers for tests that pass in Rust
        if _make_xfail_key(item) in _RUST_REMOVE_XFAIL:
            item.own_markers = [m for m in item.own_markers if m.name != "xfail"]

        marker = getattr(item, "get_closest_marker", lambda n: None)("xfail")
        if marker and (not marker.args or marker.args[0]):
            item.add_marker(pytest.mark.no_cover)
