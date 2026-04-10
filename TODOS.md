# TODOs — dateutil-rs v1 Feature Gaps

Feature gaps between python-dateutil v2.9.0 and dateutil-rs v1 (dateutil-core / dateutil-py).

## High Priority

### Phase 4: Timezone Module
- [x] `TzUtc` — UTC timezone
- [x] `TzOffset` — Fixed UTC offset with name
- [x] `TzLocal` — System local timezone
- [x] `TzFile` — TZif binary file parsing with optimized abbreviations
- [x] `gettz(name)` — Timezone factory with caching
- [x] `datetime_exists(dt, tz)` — Check DST gap
- [x] `datetime_ambiguous(dt, tz)` — Check DST overlap
- [x] `resolve_imaginary(dt)` — Shift imaginary datetime forward

### Parser: Missing Parameters
- [ ] `tzinfos` callback — Custom timezone name → tzinfo resolution

## Medium Priority

### Parser: parserinfo Customization
- [ ] `parserinfo` class — Custom month/weekday names, jump words, etc. (enables non-English parsing)

## Low Priority

### Parser
- [ ] `fuzzy_with_tokens()` — Return `(datetime, tuple_of_tokens)`

## Intentionally Excluded (v1 design decision)

These python-dateutil features are excluded from v1 by design:

- `parserinfo` customization (complex, rarely used) — listed as medium priority for reconsideration
- `parser fuzzy mode` — basic fuzzy is supported, `fuzzy_with_tokens` is low priority
- `tzrange` / `tzstr` — POSIX TZ strings (IANA names suffice)
- `tzical` — iCalendar VTIMEZONE parsing
- `tzwin` / `tzwinlocal` — Windows-only timezone classes
- Parser timezone resolution via Python `tzinfos` callback — listed as high priority for reconsideration

## v0 Gaps (minor)

v0 is nearly 100% compatible. Only missing:

- [ ] `tzical` — iCalendar VTIMEZONE parser (low usage)
- [ ] `tzwin` / `tzwinlocal` — Windows-specific (non-critical on Unix)
