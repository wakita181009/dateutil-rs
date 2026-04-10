# TODOs — dateutil-rs v1 Feature Gaps

Feature gaps between python-dateutil v2.9.0 and dateutil-rs v1 (dateutil-core / dateutil-py).

## Completed

### Phase 4: Timezone Module ✅
- [x] `TzUtc` — UTC timezone
- [x] `TzOffset` — Fixed UTC offset with name
- [x] `TzLocal` — System local timezone
- [x] `TzFile` — TZif binary file parsing with optimized abbreviations
- [x] `gettz(name)` — Timezone factory with caching
- [x] `datetime_exists(dt, tz)` — Check DST gap
- [x] `datetime_ambiguous(dt, tz)` — Check DST overlap
- [x] `resolve_imaginary(dt)` — Shift imaginary datetime forward
- [x] `datetime.tzinfo` protocol — All tz types extend `PyTzInfo` (utcoffset/dst/tzname/fromutc)
- [x] PyO3 bindings for all tz types (`dateutil-py`)
- [x] Python wrappers (`python/dateutil_rs/v1/tz.py`)
- [x] Type stubs (`python/dateutil_rs/v1/_native.pyi`)
- [x] Rust unit tests (124 tz tests passing in dateutil-core)

## In Progress

### Phase 5: Release Preparation
- [ ] v1 Python integration tests for timezone module
- [ ] Full v1 test suite (all modules end-to-end via Python)
- [ ] Publish `dateutil-core` to crates.io
- [ ] Publish `python-dateutil-rs` 1.0 to PyPI

### Parser: tzinfos & parserinfo ✅
- [x] `tzinfos` callback — Custom timezone name → tzinfo resolution (dict or callable)
- [x] `parserinfo` class — Custom month/weekday names, jump words, etc. (non-English parsing)
- [x] `ParserInfo` struct in `dateutil-core` (HashMap-based, PHF default fast path preserved)
- [x] PyO3 bindings (`parse()` with `tzinfos`, `ignoretz`, `parserinfo_config` params)
- [x] Python `parserinfo` class in `v1/parser.py` with `_to_rust_config()` serialization
- [x] Type stubs updated (`v1/_native.pyi`)
- [x] Rust unit tests for `ParserInfo` (parserinfo.rs)

## Remaining Feature Gaps

### Low Priority
- [ ] `fuzzy_with_tokens()` — Return `(datetime, tuple_of_tokens)`

## Intentionally Excluded (v1 design decision)

These python-dateutil features are excluded from v1 by design:

- `parser fuzzy mode` — basic fuzzy is supported, `fuzzy_with_tokens` is low priority
- `tzrange` / `tzstr` — POSIX TZ strings (IANA names suffice)
- `tzical` — iCalendar VTIMEZONE parsing
- `tzwin` / `tzwinlocal` — Windows-only timezone classes

## v0 Gaps (minor)

v0 is nearly 100% compatible. Only missing:

- [ ] `tzical` — iCalendar VTIMEZONE parser (low usage)
- [ ] `tzwin` / `tzwinlocal` — Windows-specific (non-critical on Unix)
