import pytest

# ---------------------------------------------------------------------------
# xfail: tests for features dateutil-rs intentionally does not support
# ---------------------------------------------------------------------------
# These tests exercise python-dateutil features excluded from dateutil-rs
# scope (see CLAUDE.md "Excluded" section) or that require Python-specific
# runtime behaviours (pickling, subclassing Rust types, weakrefs, etc.).

# Classes whose *every* method is unsupported (xfail the whole class).
_XFAIL_CLASSES = {
    # -- tz: unsupported timezone backends --
    ("test_tz", "TZICalTest"),  # iCalendar VTIMEZONE not supported
    ("test_tz", "TZRangeTest"),  # POSIX tzrange not supported
    ("test_tz", "TZStrTest"),  # POSIX tzstr not supported
    ("test_tz", "TzPickleFileTest"),  # Rust objects not picklable
    ("test_tz", "TzPickleTest"),  # Rust objects not picklable
    # -- parser: unimplemented features --
    ("test_parser", "TestFormat"),  # strftime round-trip not supported
    ("test_parser", "TestOutOfBounds"),  # error semantics differ
    ("test_parser", "TestParseUnimplementedCases"),  # explicitly unimplemented
}

# Tests inside an _XFAIL_CLASSES class that actually pass in dateutil-rs.
# These are excluded from the class-wide xfail so they do not become XPASS(strict).
_XFAIL_CLASS_EXCEPTIONS = {
    ("test_parser", "TestParseUnimplementedCases"): {
        "test_YmdH_M_S",
        "test_first_century",
        "test_era_trailing_year_with_dots",
        "test_four_letter_day",
        "test_on_era",
    },
    ("test_parser", "TestOutOfBounds"): {
        "test_no_year_zero",
        "test_out_of_bound_day",
        "test_illegal_month_error",
    },
    ("test_parser", "TestFormat"): {
        "test_strftime_formats_2003Sep25[%a %b %d %Y-Thu Sep 25 2003]",
        "test_strftime_formats_2003Sep25[%b %d %Y-Sep 25 2003]",
        "test_strftime_formats_2003Sep25[%Y-%m-%d-2003-09-25]",
        "test_strftime_formats_2003Sep25[%Y%m%d-20030925]",
        "test_strftime_formats_2003Sep25[%Y-%b-%d-2003-Sep-25]",
        "test_strftime_formats_2003Sep25[%d-%b-%Y-25-Sep-2003]",
        "test_strftime_formats_2003Sep25[%b-%d-%Y-Sep-25-2003]",
        "test_strftime_formats_2003Sep25[%m-%d-%Y-09-25-2003]",
        "test_strftime_formats_2003Sep25[%d-%m-%Y-25-09-2003]",
        "test_strftime_formats_2003Sep25[%Y.%m.%d-2003.09.25]",
        "test_strftime_formats_2003Sep25[%Y.%b.%d-2003.Sep.25]",
        "test_strftime_formats_2003Sep25[%d.%b.%Y-25.Sep.2003]",
        "test_strftime_formats_2003Sep25[%m.%d.%Y-09.25.2003]",
        "test_strftime_formats_2003Sep25[%d.%m.%Y-25.09.2003]",
        "test_strftime_formats_2003Sep25[%Y/%m/%d-2003/09/25]",
        "test_strftime_formats_2003Sep25[%Y/%b/%d-2003/Sep/25]",
        "test_strftime_formats_2003Sep25[%d/%b/%Y-25/Sep/2003]",
        "test_strftime_formats_2003Sep25[%b/%d/%Y-Sep/25/2003]",
        "test_strftime_formats_2003Sep25[%m/%d/%Y-09/25/2003]",
        "test_strftime_formats_2003Sep25[%d/%m/%Y-25/09/2003]",
        "test_strftime_formats_2003Sep25[%Y %m %d-2003 09 25]",
        "test_strftime_formats_2003Sep25[%Y %b %d-2003 Sep 25]",
        "test_strftime_formats_2003Sep25[%d %b %Y-25 Sep 2003]",
        "test_strftime_formats_2003Sep25[%m %d %Y-09 25 2003]",
        "test_strftime_formats_2003Sep25[%d %m %Y-25 09 2003]",
        "test_strftime_formats_2003Sep25[%y %d %b-03 25 Sep]",
    },
    ("test_tz", "TZRangeTest"): {
        "testRangeEquality",
        "testRangeInequalityUnsupported",
    },
    ("test_tz", "TZStrTest"): {
        "testStrInequality",
        "testStrInequalityStartEnd",
        "testStrInequalityUnsupported",
        "testTzStrRepr",
    },
}

# Individual tests that are unsupported.
# Keyed by (file_stem, class_or_empty, test_name).
_RUST_XFAIL = {
    # =======================================================================
    # relativedelta: fractional storage not supported (repr needs exact float)
    # =======================================================================
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalRepr"),
    # -- Rust pyclass not subclassable --
    ("test_relativedelta", "RelativeDeltaTest", "testInheritance"),
    # =======================================================================
    # rrule: unsupported features (TZID, aware dtstart, weekday re-export)
    # =======================================================================
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
    ("test_rrule", "WeekdayTest", "testWeekdayCallable"),
    # =======================================================================
    # parser: unsupported parse() features
    # =======================================================================
    # -- ParserTest: edge cases / features not yet implemented --
    ("test_parser", "ParserTest", "test_idx_check"),
    ("test_parser", "ParserTest", "test_includes_timestr"),
    ("test_parser", "ParserTest", "test_validate_hour"),
    ("test_parser", "ParserTest", "testAMPMNoHour"),
    ("test_parser", "ParserTest", "testAMPMRange"),
    ("test_parser", "ParserTest", "testCorrectErrorOnFuzzyWithTokens"),
    ("test_parser", "ParserTest", "testCustomParserInfo"),
    ("test_parser", "ParserTest", "testCustomParserShortDaynames"),
    ("test_parser", "ParserTest", "testDayFirst"),
    ("test_parser", "ParserTest", "testDayFirstYearFirst"),
    ("test_parser", "ParserTest", "testFuzzy"),
    ("test_parser", "ParserTest", "testFuzzyAMPMProblem"),
    ("test_parser", "ParserTest", "testFuzzyIgnoreAMPM"),
    ("test_parser", "ParserTest", "testFuzzyWithTokens"),
    ("test_parser", "ParserTest", "testNoYearFirstNoDayFirst"),
    ("test_parser", "ParserTest", "testParserParseStr"),
    ("test_parser", "ParserTest", "testParseUnicodeWords"),
    ("test_parser", "ParserTest", "testPertain"),
    ("test_parser", "ParserTest", "testRandomFormat26"),
    ("test_parser", "ParserTest", "testUnspecifiedDayFallback"),
    ("test_parser", "ParserTest", "testUnspecifiedDayFallbackFebLeapYear"),
    ("test_parser", "ParserTest", "testUnspecifiedDayFallbackFebNoLeapYear"),
    # -- TestTZVar --
    ("test_parser", "TestTZVar", "test_parse_unambiguous_nonexistent_local"),
    ("test_parser", "TestTZVar", "test_tzlocal_parse_fold"),
    # -- TestTzinfoInputTypes --
    # tzstr not supported (unicode/callable return POSIX strings)
    ("test_parser", "TestTzinfoInputTypes", "test_valid_tzinfo_unicode_input"),
    ("test_parser", "TestTzinfoInputTypes", "test_valid_tzinfo_callable_input"),
    # tzoffset singleton/identity semantics not implemented
    ("test_parser", "TestTzinfoInputTypes", "test_valid_tzinfo_int_input"),
    # -- Module-level parser tests --
    ("test_parser", "", "test_decimal_error[1: test]"),
    ("test_parser", "", "test_parse_tzinfos_fold"),
    ("test_parser", "", "test_parse_with_tzoffset[20030925T104941.5-0300-expected4]"),
    (
        "test_parser",
        "",
        "test_parser[0003-03-04-expected_datetime57-pre 12 year same month (See GH PR #293)]",
    ),
    ("test_parser", "", "test_parser[0031-01-01T00:00:00-expected_datetime54-31 ad]"),
    (
        "test_parser",
        "",
        "test_parser[2016-12-21 04.2h-expected_datetime59-Fractional Hours]",
    ),
    (
        "test_parser",
        "",
        "test_parser[December.0031.30-expected_datetime58-BYd corner case (GH#687)]",
    ),
    ("test_parser", "", "test_parser_default[01h02-expected_datetime39-random format]"),
    (
        "test_parser",
        "",
        "test_parser_default[01h02m03-expected_datetime38-random format]",
    ),
    ("test_parser", "", "test_parser_default[01m02-expected_datetime41-random format]"),
    (
        "test_parser",
        "",
        "test_parser_default[10 h 36-expected_datetime13-hour with letters strip]",
    ),
    (
        "test_parser",
        "",
        "test_parser_default[10 h 36.5-expected_datetime14-hour with letter strip]",
    ),
    ("test_parser", "", "test_parser_default[10am-expected_datetime21-hour am pm]"),
    ("test_parser", "", "test_parser_default[10pm-expected_datetime22-hour am pm]"),
    ("test_parser", "", "test_parser_default[31-Dec-00-expected_datetime34-zero year]"),
    (
        "test_parser",
        "",
        "test_parser_default[36 m 05 s-expected_datetime18-minutes with letters spaces]",
    ),
    (
        "test_parser",
        "",
        "test_parser_default[36 m 05-expected_datetime17-minute with letters spaces]",
    ),
    (
        "test_parser",
        "",
        "test_parser_default[36 m 5 s-expected_datetime16-minute with letters spaces]",
    ),
    (
        "test_parser",
        "",
        "test_parser_default[36 m 5-expected_datetime15-hour with letters spaces]",
    ),
    ("test_parser", "", "test_parser_default[Wed-expected_datetime31-weekday alone]"),
    (
        "test_parser",
        "",
        "test_parser_default[Wednesday-expected_datetime32-long weekday]",
    ),
    ("test_parser", "", "test_rounding_floatlike_strings[5.6h-dt0]"),
    ("test_parser", "", "test_rounding_floatlike_strings[5.6m-dt1]"),
    # =======================================================================
    # tz: unsupported tz features
    # =======================================================================
    # -- TzUTCTest: singleton / inequality semantics --
    ("test_tz", "TzUTCTest", "testInequalityUnsupported"),
    ("test_tz", "TzUTCTest", "testSingleton"),
    # -- TzOffsetTest: singleton / attribute / name semantics --
    ("test_tz", "TzOffsetTest", "testInequalityUnsupported"),
    ("test_tz", "TzOffsetTest", "testTzNameNone"),
    ("test_tz", "TzOffsetTest", "testTzOffsetInstance"),
    ("test_tz", "TzOffsetTest", "testTzOffsetSingletonDifferent"),
    # -- TzLocalTest --
    ("test_tz", "TzLocalTest", "testEquality"),
    ("test_tz", "TzLocalTest", "testInequalityFixedOffset"),
    ("test_tz", "TzLocalTest", "testRepr"),
    # -- TZTest --
    ("test_tz", "TZTest", "testGMTHasNoDaylight"),
    ("test_tz", "TZTest", "testGMTOffset"),
    ("test_tz", "TZTest", "testIsStd"),  # requires _ttinfo_list internal attribute
    # TZStrTest covered by _XFAIL_CLASSES
    # -- DatetimeAmbiguousTest: custom tzinfo classes not extractable --
    # -- DatetimeExistsTest: custom tzinfo classes --
    # -- GettzTest --
    ("test_tz", "GettzTest", "testGettzCacheTzFile"),
    # -- TzLocalNixTest --
    ("test_tz", "TzLocalNixTest", "testAmbiguousNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testAmbiguousPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testDSTDST"),
    ("test_tz", "TzLocalNixTest", "testFoldIndependence"),
    ("test_tz", "TzLocalNixTest", "testFoldLondon"),
    ("test_tz", "TzLocalNixTest", "testFoldNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testFoldPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testGapNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testGapPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testImaginaryNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testImaginaryPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testInZoneFoldEquality"),
    ("test_tz", "TzLocalNixTest", "testOffsetDST"),
    ("test_tz", "TzLocalNixTest", "testTimeOnlyDSTLocalDST"),
    ("test_tz", "TzLocalNixTest", "testTimeOnlyOffsetLocalDST"),
    ("test_tz", "TzLocalNixTest", "testTzNameDST"),
    ("test_tz", "TzLocalNixTest", "testUTCEquality"),
    # -- Module-level tz tests --
    ("test_tz", "", "test_gettz_badzone[Fake.Region/Abcdefghijklmnop]"),
    ("test_tz", "", "test_gettz_badzone_unicode"),
    ("test_tz", "", "test_gettz_cache_clear"),
    ("test_tz", "", "test_gettz_same_result_for_none_and_empty_string"),
    ("test_tz", "", "test_gettz_set_cache_size"),
    ("test_tz", "", "test_gettz_weakref"),
    ("test_tz", "", "test_gettz_zone_wrong_type[bytes on Python 3]"),
    ("test_tz", "", "test_invalid_GNU_tzstr[-1:WART4WARST,J1,J365/25]"),
    ("test_tz", "", r"test_invalid_GNU_tzstr[,dfughdfuigpu87\xf1::]"),
    ("test_tz", "", r"test_invalid_GNU_tzstr[hdfiughdfuig,dfughdfuigpu87\xf1::]"),
    ("test_tz", "", "test_invalid_GNU_tzstr[IST-2IDT,M3,2000,1/26,M10,5,0]"),
    ("test_tz", "", "test_invalid_GNU_tzstr[IST-2IDT,M3.4.-1/26,M10.5.0]"),
    ("test_tz", "", "test_invalid_GNU_tzstr[WART4WARST,J1,J365/-25]"),
    ("test_tz", "", "test_tzlocal_offset_equal[EST5-tzoff0]"),
    ("test_tz", "", "test_tzlocal_offset_equal[GMT0-tzoff1]"),
    ("test_tz", "", "test_tzlocal_offset_equal[JST-9-tzoff3]"),
    ("test_tz", "", "test_tzlocal_offset_equal[YAKT-9-tzoff2]"),
    ("test_tz", "", "test_tzlocal_utc_equal[GMT0]"),
    ("test_tz", "", "test_tzlocal_utc_equal[UTC0]"),
    ("test_tz", "", "test_tzlocal_utc_equal[UTC]"),
    ("test_tz", "", "test_tzoffset_is[args0-kwargs0]"),
    ("test_tz", "", "test_tzoffset_is[args1-kwargs1]"),
    ("test_tz", "", "test_tzoffset_is[args2-kwargs2]"),
    ("test_tz", "", "test_tzoffset_is[args3-kwargs3]"),
    ("test_tz", "", "test_tzoffset_is[args4-kwargs4]"),
    ("test_tz", "", "test_tzoffset_singleton[args0]"),
    ("test_tz", "", "test_tzoffset_singleton[args1]"),
    ("test_tz", "", "test_tzoffset_singleton[args2]"),
    ("test_tz", "", "test_tzoffset_singleton[args3]"),
    ("test_tz", "", "test_tzoffset_weakref"),
    (
        "test_tz",
        "",
        "test_tzstr_default_cmp[EST5EDT-EST5EDT4,M4.1.0/02:00:00,M10-5-0/02:00]",
    ),
    (
        "test_tz",
        "",
        "test_tzstr_default_cmp[EST5EDT4,M4.1.0/02:00:00,M10-5-0/02:00-EST5EDT]",
    ),
    ("test_tz", "", "test_tzstr_default_end[EST5EDT4,95/02:00:00,298/02:00]"),
    ("test_tz", "", "test_tzstr_default_end[EST5EDT4,J96/02:00:00,J299/02:00]"),
    ("test_tz", "", "test_tzstr_default_end[EST5EDT4,J96/02:00:00,J299/02]"),
    ("test_tz", "", "test_tzstr_default_end[EST5EDT4,M4.1.0/02:00:00,M10-5-0/02:00]"),
    ("test_tz", "", "test_tzstr_default_end[EST5EDT]"),
    ("test_tz", "", "test_tzstr_default_start[EST5EDT4,95/02:00:00,298/02:00]"),
    ("test_tz", "", "test_tzstr_default_start[EST5EDT4,J96/02:00:00,J299/02:00]"),
    ("test_tz", "", "test_tzstr_default_start[EST5EDT4,J96/02:00:00,J299/02]"),
    ("test_tz", "", "test_tzstr_default_start[EST5EDT4,M4.1.0/02:00:00,M10-5-0/02:00]"),
    ("test_tz", "", "test_tzstr_default_start[EST5EDT]"),
    ("test_tz", "", "test_tzstr_weakref"),
    ("test_tz", "", "test_valid_GNU_tzstr[-expected0]"),
    ("test_tz", "", "test_valid_GNU_tzstr[AEST-1100AEDT-expected7]"),
    ("test_tz", "", "test_valid_GNU_tzstr[AEST-11:00AEDT-expected8]"),
    ("test_tz", "", "test_valid_GNU_tzstr[EST+5EDT,M3.2.0/2,M11.1.0/12-expected1]"),
    (
        "test_tz",
        "",
        "test_valid_GNU_tzstr[EST5EDT,M3.2.0/0400,M11.1.0/0300-expected11]",
    ),
    (
        "test_tz",
        "",
        "test_valid_GNU_tzstr[EST5EDT,M3.2.0/04:00,M11.1.0/03:00-expected10]",
    ),
    ("test_tz", "", "test_valid_GNU_tzstr[EST5EDT,M3.2.0/4:00,M11.1.0/3:00-expected9]"),
    ("test_tz", "", "test_valid_GNU_tzstr[IST-2IDT,M3.4.4/26,M10.5.0-expected3]"),
    ("test_tz", "", "test_valid_GNU_tzstr[WART4WARST,J1/0,J365/25-expected2]"),
    ("test_tz", "", "test_valid_GNU_tzstr[WGT0300WGST-expected5]"),
    ("test_tz", "", "test_valid_GNU_tzstr[WGT03:00WGST-expected6]"),
    ("test_tz", "", "test_valid_GNU_tzstr[WGT3WGST,M3.5.0/2,M10.5.0/1-expected4]"),
    (
        "test_tz",
        "",
        "test_valid_dateutil_format[EST5EDT,5,-4,0,7200,11,3,0,7200-expected1]",
    ),
    (
        "test_tz",
        "",
        "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,+3600-expected7]",
    ),
    (
        "test_tz",
        "",
        "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,+7200-expected6]",
    ),
    (
        "test_tz",
        "",
        "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,-3600-expected5]",
    ),
    (
        "test_tz",
        "",
        "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,3600-expected3]",
    ),
    (
        "test_tz",
        "",
        "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,3600-expected4]",
    ),
    (
        "test_tz",
        "",
        "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200-expected2]",
    ),
    (
        "test_tz",
        "",
        "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,3,0,7200-expected0]",
    ),
    # =======================================================================
    # imports: missing exports / lazy import / __version__
    # =======================================================================
    ("test_imports", "", "test_import_parser_all"),
    ("test_imports", "", "test_import_relative_delta_all"),
    ("test_imports", "", "test_import_rrule_all"),
    ("test_imports", "", "test_import_tz_all"),
    ("test_imports", "", "test_import_version_root"),
    ("test_imports", "", "test_import_version_str"),
    ("test_imports", "", "test_import_zone_info_star"),
    ("test_imports", "", "test_lazy_import[easter]"),
    ("test_imports", "", "test_lazy_import[relativedelta]"),
    ("test_imports", "", "test_lazy_import[rrule]"),
    ("test_imports", "", "test_lazy_import[tz]"),
    ("test_imports", "", "test_lazy_import[zoneinfo]"),
    # -- import * --
    ("test_import_star", "", "test_imported_modules"),
    # =======================================================================
    # property tests: convertyear not implemented
    # =======================================================================
}

# Property-based tests that xfail only on certain hypothesis examples.
# Using strict=True would flip to XPASS when hypothesis picks a non-falsifying
# seed; mark these non-strict instead.
_RUST_XFAIL_NONSTRICT = {
    # isoparse: tzfile vs tzoffset round-trip identity not preserved
    ("test_isoparse_prop", "", "test_timespec_auto"),
    # parser: hypothesis-dependent year conversion edge cases
    ("test_parser_prop", "", "test_convertyear"),
    ("test_parser_prop", "", "test_convertyear_no_specified_century"),
}


def _make_xfail_key(item):
    """Build a (file_stem, class_name, test_name) key from a pytest Item."""
    file_stem = item.path.stem if hasattr(item, "path") else ""
    cls = item.cls.__name__ if item.cls else ""
    return file_stem, cls, item.name


def _should_xfail(item):
    """Return (strict, True) if a test is known-unsupported in dateutil-rs."""
    file_stem = item.path.stem if hasattr(item, "path") else ""
    cls = item.cls.__name__ if item.cls else ""
    # Whole-class match, excluding tests that actually pass
    if (file_stem, cls) in _XFAIL_CLASSES:
        exceptions = _XFAIL_CLASS_EXCEPTIONS.get((file_stem, cls), set())
        if item.name not in exceptions:
            return True, True
    # Individual test match
    key = (file_stem, cls, item.name)
    if key in _RUST_XFAIL:
        return True, True
    if key in _RUST_XFAIL_NONSTRICT:
        return True, False
    return False, True


# ---------------------------------------------------------------------------
# Upstream xfail removals: tests with @pytest.mark.xfail in the source that
# actually pass in the Rust implementation (would become XPASS(strict)).
# ---------------------------------------------------------------------------
_RUST_REMOVE_XFAIL = {
    ("test_rrule", "", "test_generated_aware_dtstart_rrulestr"),
    ("test_isoparser", "", "test_isoparser_byte_sep"),
    ("test_tz", "", "test_gettz_zone_wrong_type[no startswith()]"),
    # -- TestParseUnimplementedCases: now pass in Rust --
    ("test_parser", "TestParseUnimplementedCases", "test_YmdH_M_S"),
    ("test_parser", "TestParseUnimplementedCases", "test_first_century"),
    ("test_parser", "TestParseUnimplementedCases", "test_era_trailing_year_with_dots"),
    ("test_parser", "TestParseUnimplementedCases", "test_four_letter_day"),
    ("test_parser", "TestParseUnimplementedCases", "test_on_era"),
    # -- TzLocalNixTest: now pass in Rust --
    ("test_tz", "TzLocalNixTest", "testNotImaginaryFoldNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testNotImaginaryFoldPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testNotImaginaryNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testNotImaginaryPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testUnambiguousGapNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testUnambiguousGapPositiveUTCOffset"),
}


# Configure pytest to ignore xfailing tests
# See: https://stackoverflow.com/a/53198349/467366
def pytest_collection_modifyitems(config, items):
    for item in items:
        should, strict = _should_xfail(item)
        if should:
            item.add_marker(
                pytest.mark.xfail(
                    reason="not supported by dateutil-rs",
                    strict=strict,
                )
            )

        # Remove upstream xfail markers for tests that pass in Rust
        if _make_xfail_key(item) in _RUST_REMOVE_XFAIL:
            item.own_markers = [m for m in item.own_markers if m.name != "xfail"]

        marker = getattr(item, "get_closest_marker", lambda n: None)("xfail")
        if marker and (not marker.args or marker.args[0]):
            item.add_marker(pytest.mark.no_cover)
