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
    ("test_tz", "TzPickleFileTest"),  # Rust objects not picklable
    ("test_tz", "TzPickleTest"),  # Rust objects not picklable
    ("test_tz", "TestEnfold"),  # enfold() not supported
    ("test_tz", "ImaginaryDateTest"),  # resolve_imaginary needs generic tzinfo extraction
    # -- parser: unimplemented features --
    ("test_parser", "TestFormat"),  # strftime round-trip not supported
    ("test_parser", "TestOutOfBounds"),  # error semantics differ
    ("test_parser", "TestParseUnimplementedCases"),  # explicitly unimplemented
}

# Individual tests that are unsupported.
# Keyed by (file_stem, class_or_empty, test_name).
_RUST_XFAIL = {
    # =======================================================================
    # relativedelta: float arguments not supported
    # =======================================================================
    ("test_relativedelta", "RelativeDeltaTest", "testAdditionFloatFractionals"),
    ("test_relativedelta", "RelativeDeltaTest", "testAdditionFloatValue"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalDays"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalHours"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalMinutes"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalMonth"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalNegativeDays"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalNegativeOverflow"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalPositiveOverflow"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalPositiveOverflow2"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalRepr"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalSeconds"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalWeeks"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalYear"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaNormalizeFractionalDays"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaNormalizeFractionalDays2"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaNormalizeFractionalMinutes"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaNormalizeFractionalSeconds"),
    ("test_relativedelta", "RelativeDeltaTest", "testRelativeDeltaFractionalAbsolutes"),
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
    ("test_parser", "ParserTest", "testErrorType01"),
    ("test_parser", "ParserTest", "testFuzzyAMPMProblem"),
    ("test_parser", "ParserTest", "testFuzzyIgnoreAMPM"),
    ("test_parser", "ParserTest", "testFuzzySimple"),
    ("test_parser", "ParserTest", "testFuzzyWithTokens"),
    ("test_parser", "ParserTest", "testISOFormat"),
    ("test_parser", "ParserTest", "testISOFormatStrip"),
    ("test_parser", "ParserTest", "testNoSeparator1"),
    ("test_parser", "ParserTest", "testNoSeparator2"),
    ("test_parser", "ParserTest", "testNoYearFirstNoDayFirst"),
    ("test_parser", "ParserTest", "testParserParseStr"),
    ("test_parser", "ParserTest", "testParseUnicodeWords"),
    ("test_parser", "ParserTest", "testPertain"),
    ("test_parser", "ParserTest", "testRandomFormat26"),
    ("test_parser", "ParserTest", "testUnspecifiedDayFallback"),
    ("test_parser", "ParserTest", "testUnspecifiedDayFallbackFebLeapYear"),
    ("test_parser", "ParserTest", "testUnspecifiedDayFallbackFebNoLeapYear"),
    # -- TestInputTypes: bytes/stream/bytearray input --
    ("test_parser", "TestInputTypes", "test_duck_typing"),
    ("test_parser", "TestInputTypes", "test_empty_string_invalid"),
    ("test_parser", "TestInputTypes", "test_parse_bytearray"),
    ("test_parser", "TestInputTypes", "test_parse_bytes"),
    ("test_parser", "TestInputTypes", "test_parse_str"),
    ("test_parser", "TestInputTypes", "test_parse_stream"),
    # -- TestTZVar --
    ("test_parser", "TestTZVar", "test_tzinfo_arg_parseerror"),
    ("test_parser", "TestTZVar", "test_tzinfo_arg_typeerror"),
    # -- TestTzinfoInputTypes --
    ("test_parser", "TestTzinfoInputTypes", "test_tzinfo_dict_parseerror"),
    ("test_parser", "TestTzinfoInputTypes", "test_tzinfo_input_number"),
    ("test_parser", "TestTzinfoInputTypes", "test_tzinfo_input_timedelta"),
    # -- Module-level parser tests --
    ("test_parser", "", "test_decimal_error[1: test]"),
    ("test_parser", "", "test_decimal_error[Nan]"),
    ("test_parser", "", "test_parse_tzinfos_fold"),
    ("test_parser", "", "test_parse_with_tzoffset[20030925T104941.5-0300-expected4]"),
    ("test_parser", "", "test_parser[0003-03-04-expected_datetime57-pre 12 year same month (See GH PR #293)]"),
    ("test_parser", "", "test_parser[0031-01-01T00:00:00-expected_datetime54-31 ad]"),
    ("test_parser", "", "test_parser[199709020908-expected_datetime11-no separator]"),
    ("test_parser", "", "test_parser[19970902090807-expected_datetime12-no separator]"),
    ("test_parser", "", "test_parser[20030925T1049-expected_datetime7-iso stripped format strip]"),
    ("test_parser", "", "test_parser[2016-12-21 04.2h-expected_datetime59-Fractional Hours]"),
    ("test_parser", "", "test_parser[December.0031.30-expected_datetime58-BYd corner case (GH#687)]"),
    ("test_parser", "", "test_parser_default[01h02-expected_datetime39-random format]"),
    ("test_parser", "", "test_parser_default[01h02m03-expected_datetime38-random format]"),
    ("test_parser", "", "test_parser_default[01m02-expected_datetime41-random format]"),
    ("test_parser", "", "test_parser_default[10 h 36-expected_datetime13-hour with letters strip]"),
    ("test_parser", "", "test_parser_default[10 h 36.5-expected_datetime14-hour with letter strip]"),
    ("test_parser", "", "test_parser_default[10am-expected_datetime21-hour am pm]"),
    ("test_parser", "", "test_parser_default[10pm-expected_datetime22-hour am pm]"),
    ("test_parser", "", "test_parser_default[31-Dec-00-expected_datetime34-zero year]"),
    ("test_parser", "", "test_parser_default[36 m 05 s-expected_datetime18-minutes with letters spaces]"),
    ("test_parser", "", "test_parser_default[36 m 05-expected_datetime17-minute with letters spaces]"),
    ("test_parser", "", "test_parser_default[36 m 5 s-expected_datetime16-minute with letters spaces]"),
    ("test_parser", "", "test_parser_default[36 m 5-expected_datetime15-hour with letters spaces]"),
    ("test_parser", "", "test_parser_default[Wed-expected_datetime31-weekday alone]"),
    ("test_parser", "", "test_parser_default[Wednesday-expected_datetime32-long weekday]"),
    ("test_parser", "", "test_rounding_floatlike_strings[5.6h-dt0]"),
    ("test_parser", "", "test_rounding_floatlike_strings[5.6m-dt1]"),
    # =======================================================================
    # tz: unsupported tz features
    # =======================================================================
    # -- TzUTCTest: singleton / inequality semantics --
    ("test_tz", "TzUTCTest", "testAmbiguity"),
    ("test_tz", "TzUTCTest", "testInequalityUnsupported"),
    ("test_tz", "TzUTCTest", "testSingleton"),
    # -- TzOffsetTest: singleton / attribute / name semantics --
    ("test_tz", "TzOffsetTest", "testAmbiguity"),
    ("test_tz", "TzOffsetTest", "testInequalityUnsupported"),
    ("test_tz", "TzOffsetTest", "testTzNameNone"),
    ("test_tz", "TzOffsetTest", "testTzOffsetInstance"),
    ("test_tz", "TzOffsetTest", "testTzOffsetSingletonDifferent"),
    # -- TzLocalTest --
    ("test_tz", "TzLocalTest", "testEquality"),
    ("test_tz", "TzLocalTest", "testInequalityFixedOffset"),
    ("test_tz", "TzLocalTest", "testRepr"),
    # -- TZTest --
    ("test_tz", "TZTest", "testFileEnd1"),
    ("test_tz", "TZTest", "testFileLastTransition"),
    ("test_tz", "TZTest", "testFileStart1"),
    ("test_tz", "TZTest", "testGMTHasNoDaylight"),
    ("test_tz", "TZTest", "testGMTOffset"),
    ("test_tz", "TZTest", "testImaginaryNaiveEquality"),
    ("test_tz", "TZTest", "testLeapCountDecodesProperly"),
    ("test_tz", "TZTest", "testRoundTrip"),
    ("test_tz", "TZTest", "testTZFileEquality"),
    # -- TZStrTest (tzstr not supported, but 4 tests pass) --
    ("test_tz", "TZStrTest", "testTzStrAmbiguity"),
    ("test_tz", "TZStrTest", "testTzStrBrokenIsDst"),
    ("test_tz", "TZStrTest", "testTzStrEnd1"),
    ("test_tz", "TZStrTest", "testTzStrEnd2"),
    ("test_tz", "TZStrTest", "testTzStrEnd3"),
    ("test_tz", "TZStrTest", "testTzStrEnd4"),
    ("test_tz", "TZStrTest", "testTzStrEnd5"),
    ("test_tz", "TZStrTest", "testTzStrEnd6"),
    ("test_tz", "TZStrTest", "testTzStrEnd7"),
    ("test_tz", "TZStrTest", "testTzStrImaginary"),
    ("test_tz", "TZStrTest", "testTzStrInstance"),
    ("test_tz", "TZStrTest", "testTzStrRepr"),
    ("test_tz", "TZStrTest", "testTzStrSingleton"),
    ("test_tz", "TZStrTest", "testTzStrSingletonPosix"),
    ("test_tz", "TZStrTest", "testTzStrStart1"),
    ("test_tz", "TZStrTest", "testTzStrStart2"),
    ("test_tz", "TZStrTest", "testTzStrStart3"),
    ("test_tz", "TZStrTest", "testTzStrStart4"),
    ("test_tz", "TZStrTest", "testTzStrStart5"),
    ("test_tz", "TZStrTest", "testTzStrStart6"),
    ("test_tz", "TZStrTest", "testTzStrStart7"),
    ("test_tz", "TZStrTest", "testTzStrType"),
    ("test_tz", "TZStrTest", "testTzStrUTCOffset"),
    ("test_tz", "TZStrTest", "testUnambiguousGapNegativeUTCOffset"),
    ("test_tz", "TZStrTest", "testUnambiguousGapPositiveUTCOffset"),
    # -- DatetimeAmbiguousTest: custom tzinfo classes not extractable --
    ("test_tz", "DatetimeAmbiguousTest", "testAmbiguousDatetime"),
    ("test_tz", "DatetimeAmbiguousTest", "testAmbiguousError"),
    ("test_tz", "DatetimeAmbiguousTest", "testAmbiguousLocal"),
    ("test_tz", "DatetimeAmbiguousTest", "testAmbiguousOffset"),
    ("test_tz", "DatetimeAmbiguousTest", "testAmbiguousUTC"),
    ("test_tz", "DatetimeAmbiguousTest", "testIncompatibleAmbiguityFold0"),
    ("test_tz", "DatetimeAmbiguousTest", "testIncompatibleAmbiguityFold1"),
    ("test_tz", "DatetimeAmbiguousTest", "testNoSupportAmbiguity"),
    ("test_tz", "DatetimeAmbiguousTest", "testNoTzSpecified"),
    ("test_tz", "DatetimeAmbiguousTest", "testNotAmbiguousLocal"),
    ("test_tz", "DatetimeAmbiguousTest", "testNotAmbiguousOffset"),
    ("test_tz", "DatetimeAmbiguousTest", "testNotAmbiguousUTC"),
    ("test_tz", "DatetimeAmbiguousTest", "testSpecifiedTzOverridesAttached"),
    ("test_tz", "DatetimeAmbiguousTest", "testSupportAmbiguity"),
    ("test_tz", "DatetimeAmbiguousTest", "testSupportNoAmbiguity"),
    ("test_tz", "DatetimeAmbiguousTest", "testUnambiguousDatetime"),
    # -- DatetimeExistsTest: custom tzinfo classes --
    ("test_tz", "DatetimeExistsTest", "testExistsLocal"),
    ("test_tz", "DatetimeExistsTest", "testInGapLocal"),
    ("test_tz", "DatetimeExistsTest", "testNoTzSpecified"),
    ("test_tz", "DatetimeExistsTest", "testSpecifiedTzOverridesAttached"),
    # -- GettzTest --
    ("test_tz", "GettzTest", "testGetTZFromFile"),
    ("test_tz", "GettzTest", "testGetTZFromFileobj"),
    ("test_tz", "GettzTest", "testGettz"),
    ("test_tz", "GettzTest", "testGettzCacheTzFile"),
    ("test_tz", "GettzTest", "testGettzCacheTzLocal"),
    ("test_tz", "GettzTest", "testGettzTimeZoneName"),
    ("test_tz", "GettzTest", "testGMTNegHHMM"),
    ("test_tz", "GettzTest", "testGMTNegHHMMSS"),
    # -- TzLocalNixTest --
    ("test_tz", "TzLocalNixTest", "testAmbiguousNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testAmbiguousPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testDSTUTC"),
    ("test_tz", "TzLocalNixTest", "testFoldIndependence"),
    ("test_tz", "TzLocalNixTest", "testFoldLondon"),
    ("test_tz", "TzLocalNixTest", "testFoldNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testGapNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testGapPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testImaginaryNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testImaginaryPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testInZoneFoldEquality"),
    ("test_tz", "TzLocalNixTest", "testNotImaginaryFoldNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testNotImaginaryFoldPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testNotImaginaryNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testNotImaginaryPositiveUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testOffsetUTC"),
    ("test_tz", "TzLocalNixTest", "testTzNameUTC"),
    ("test_tz", "TzLocalNixTest", "testUnambiguousGapNegativeUTCOffset"),
    ("test_tz", "TzLocalNixTest", "testUnambiguousGapPositiveUTCOffset"),
    # -- Module-level tz tests --
    ("test_tz", "", "test_gettz_badzone[Fake.Region/Abcdefghijklmnop]"),
    ("test_tz", "", "test_gettz_badzone_unicode"),
    ("test_tz", "", "test_gettz_cache_clear"),
    ("test_tz", "", "test_gettz_same_result_for_none_and_empty_string"),
    ("test_tz", "", "test_gettz_set_cache_size"),
    ("test_tz", "", "test_gettz_weakref"),
    ("test_tz", "", "test_gettz_zone_wrong_type[bytes on Python 3]"),
    ("test_tz", "", "test_gettz_zone_wrong_type[no startswith()]"),
    ("test_tz", "", "test_invalid_GNU_tzstr[-1:WART4WARST,J1,J365/25]"),
    ("test_tz", "", r"test_invalid_GNU_tzstr[,dfughdfuigpu87\xf1::]"),
    ("test_tz", "", r"test_invalid_GNU_tzstr[hdfiughdfuig,dfughdfuigpu87\xf1::]"),
    ("test_tz", "", "test_invalid_GNU_tzstr[IST-2IDT,M3,2000,1/26,M10,5,0]"),
    ("test_tz", "", "test_invalid_GNU_tzstr[IST-2IDT,M3.4.-1/26,M10.5.0]"),
    ("test_tz", "", "test_invalid_GNU_tzstr[WART4WARST,J1,J365/-25]"),
    ("test_tz", "", "test_resolve_imaginary[tzi0-dt0-dt_exp0]"),
    ("test_tz", "", "test_resolve_imaginary[tzi1-dt1-dt_exp1]"),
    ("test_tz", "", "test_resolve_imaginary[tzi2-dt2-dt_exp2]"),
    ("test_tz", "", "test_resolve_imaginary[tzi3-dt3-dt_exp3]"),
    ("test_tz", "", "test_resolve_imaginary[tzi4-dt4-dt_exp4]"),
    ("test_tz", "", "test_resolve_imaginary_ambiguous[dt0]"),
    ("test_tz", "", "test_resolve_imaginary_ambiguous[dt1]"),
    ("test_tz", "", "test_resolve_imaginary_ambiguous[dt2]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt0]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt1]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt2]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt3]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt4]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt5]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt6]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt7]"),
    ("test_tz", "", "test_resolve_imaginary_existing[dt8]"),
    ("test_tz", "", "test_tzfile_sub_minute_offset"),
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
    ("test_tz", "", "test_tzstr_default_cmp[EST5EDT-EST5EDT4,M4.1.0/02:00:00,M10-5-0/02:00]"),
    ("test_tz", "", "test_tzstr_default_cmp[EST5EDT4,M4.1.0/02:00:00,M10-5-0/02:00-EST5EDT]"),
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
    ("test_tz", "", "test_valid_GNU_tzstr[EST5EDT,M3.2.0/0400,M11.1.0/0300-expected11]"),
    ("test_tz", "", "test_valid_GNU_tzstr[EST5EDT,M3.2.0/04:00,M11.1.0/03:00-expected10]"),
    ("test_tz", "", "test_valid_GNU_tzstr[EST5EDT,M3.2.0/4:00,M11.1.0/3:00-expected9]"),
    ("test_tz", "", "test_valid_GNU_tzstr[IST-2IDT,M3.4.4/26,M10.5.0-expected3]"),
    ("test_tz", "", "test_valid_GNU_tzstr[WART4WARST,J1/0,J365/25-expected2]"),
    ("test_tz", "", "test_valid_GNU_tzstr[WGT0300WGST-expected5]"),
    ("test_tz", "", "test_valid_GNU_tzstr[WGT03:00WGST-expected6]"),
    ("test_tz", "", "test_valid_GNU_tzstr[WGT3WGST,M3.5.0/2,M10.5.0/1-expected4]"),
    ("test_tz", "", "test_valid_dateutil_format[EST5EDT,5,-4,0,7200,11,3,0,7200-expected1]"),
    ("test_tz", "", "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,+3600-expected7]"),
    ("test_tz", "", "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,+7200-expected6]"),
    ("test_tz", "", "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,-3600-expected5]"),
    ("test_tz", "", "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,3600-expected3]"),
    ("test_tz", "", "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200,3600-expected4]"),
    ("test_tz", "", "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,-3,0,7200-expected2]"),
    ("test_tz", "", "test_valid_dateutil_format[EST5EDT,5,4,0,7200,11,3,0,7200-expected0]"),
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
    ("test_parser_prop", "", "test_convertyear"),
    ("test_parser_prop", "", "test_convertyear_no_specified_century"),
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
