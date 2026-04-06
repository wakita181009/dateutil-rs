# dateutil-rs — Rust Rewrite of python-dateutil

## Project Goal

Rewrite **python-dateutil** (v2.9.0) in Rust to achieve significant performance improvements while maintaining API compatibility. The Rust implementation will be usable both as a native Rust crate and as a Python extension module via PyO3/maturin.

## Directory Structure

```
dateutil-rs/
├── pyproject.toml                  # Python project config (maturin build backend)
│
├── crates/
│   └── dateutil-rs/
│       ├── Cargo.toml              # Rust crate config (the only Cargo.toml)
│       └── src/
│           ├── lib.rs              # Crate root + #[pymodule] definition
│           ├── common.rs           # Weekday (MO-SU with N-th occurrence)
│           ├── easter.rs           # easter() — 3 calendar methods
│           ├── utils.rs            # today(), default_tzinfo(), within_delta()
│           ├── relativedelta.rs    # RelativeDelta
│           ├── parser/
│           │   ├── mod.rs          # parse(), ParserInfo, ParserError
│           │   └── isoparser.rs    # isoparse()
│           ├── rrule/
│           │   ├── mod.rs          # RRule, RRuleSet, rrulestr()
│           │   └── iter.rs         # Iterator implementation
│           └── tz/
│               ├── mod.rs          # gettz(), public API
│               ├── utc.rs          # TzUtc
│               ├── offset.rs       # TzOffset
│               ├── file.rs         # TzFile (POSIX tzfile binary)
│               ├── local.rs        # TzLocal
│               └── range.rs        # TzStr, TzRange
│
├── python/                         # Python package (maturin mixed layout)
│   └── dateutil_rs/
│       ├── __init__.py             # Re-exports from Rust native module
│       ├── py.typed                # PEP 561 marker
│       └── compat.py              # python-dateutil compatibility wrappers
│
├── src/                            # Original python-dateutil v2.9.0 (reference only)
│   └── dateutil/
│       ├── __init__.py
│       ├── _common.py
│       ├── easter.py
│       ├── parser/
│       ├── relativedelta.py
│       ├── rrule.py
│       ├── tz/
│       ├── utils.py
│       └── zoneinfo/
│
├── tests/                          # Python tests (existing, from python-dateutil)
├── benchmarks/                     # Performance benchmarks (Python vs Rust)
├── .github/                        # CI workflows
├── LICENSE
└── Makefile
```

### Key Layout Decisions

- **No workspace Cargo.toml at root** — single crate, so `crates/dateutil-rs/Cargo.toml` is the only Cargo.toml.
- **`crates/dateutil-rs/`** — Rust source lives here, following `crates/` convention for Rust code.
- **`src/dateutil/`** — Original python-dateutil v2.9.0 reference code. Read-only; used for comparison tests, benchmarks, and as implementation reference. Not modified.
- **`python/dateutil_rs/`** — maturin mixed layout. Thin Python layer that re-exports from the Rust native module.
- **`pyproject.toml`** — Uses `maturin` as build backend with `manifest-path = "crates/dateutil-rs/Cargo.toml"`.

## Build Configuration

### crates/dateutil-rs/Cargo.toml

```toml
[package]
name = "dateutil-rs"
version = "0.1.0"
edition = "2021"

[lib]
name = "dateutil_rs"
# rlib only by default — maturin adds cdylib at build time.
# This avoids requiring Python interpreter linkage for `cargo test`.
crate-type = ["rlib"]

[dependencies]
chrono = "0.4"
pyo3 = { version = "0.24", features = ["extension-module"], optional = true }

[features]
default = []
python = ["pyo3"]
```

- `rlib` — Pure Rust library (no Python dependency). Default crate type.
- `cdylib` — Python extension module (via PyO3). Added automatically by maturin at build time.
- `#[cfg(feature = "python")]` gates all PyO3 bindings
- `cargo test` works without Python installed (rlib only, no cdylib linking)

### pyproject.toml (maturin settings)

```toml
[build-system]
requires = ["maturin>=1.0"]
build-backend = "maturin"

[tool.maturin]
manifest-path = "crates/dateutil-rs/Cargo.toml"
features = ["python"]
python-source = "python"
module-name = "dateutil_rs._native"
```

## Architecture Overview

### Source Python Modules → Target Rust Modules

| Python Module | Lines | Rust Module | Priority | Complexity |
|---|---|---|---|---|
| `dateutil.parser` (_parser.py + isoparser.py) | ~2,029 | `dateutil_rs::parser` | P0 | High |
| `dateutil.rrule` | ~1,737 | `dateutil_rs::rrule` | P1 | High |
| `dateutil.tz` (tz.py + _common.py + _factories.py) | ~2,348 | `dateutil_rs::tz` | P1 | High |
| `dateutil.relativedelta` | ~599 | `dateutil_rs::relativedelta` | P0 | Medium |
| `dateutil.easter` | ~89 | `dateutil_rs::easter` | P2 | Low |
| `dateutil.utils` | ~71 | `dateutil_rs::utils` | P2 | Low |
| `dateutil.zoneinfo` | ~167 | `dateutil_rs::zoneinfo` | P2 | Medium |
| `dateutil._common` (weekday) | ~43 | `dateutil_rs::common` | P0 | Low |

### Implementation Phases

**Phase 1 — Core Types & Foundations**
- `common::Weekday` (MO–SU with N-th occurrence support)
- `easter::easter()` (3 calendar methods). Handle year <= 0 explicitly (match Python's ValueError).
- `utils::within_delta()` only. `today()` and `default_tzinfo()` deferred to Phase 3 (depend on timezone types).
- PyO3 bindings for Phase 1 modules (incremental — each phase ships bindings)
- Rust project scaffolding (Cargo.toml, module structure, CI)
- Python-side benchmarks (extend existing pytest-benchmark to compare dateutil vs dateutil_rs). Criterion deferred to Phase 5.

**Phase 2 — Parser & RelativeDelta**
- `relativedelta::RelativeDelta` (relative/absolute date arithmetic)
- `parser::parse()` (generic date/time string parsing)
- `parser::isoparse()` (ISO-8601 strict parsing)
- `parser::ParserInfo` (customizable parsing rules)

**Phase 3 — Timezone Support**
- `tz::TzUtc`, `tz::TzOffset` (fixed offsets)
- `tz::TzFile` (POSIX tzfile binary format)
- `tz::TzStr`, `tz::TzRange` (TZ environment string)
- `tz::TzLocal` (system local timezone)
- `tz::gettz()` (convenience lookup)
- `utils::today()`, `utils::default_tzinfo()` (deferred from Phase 1, depend on tz types)
- `zoneinfo` (bundled IANA timezone database)

**Phase 4 — Recurrence Rules**
- `rrule::RRule` (RFC 5545 recurrence rules)
- `rrule::RRuleSet` (composite rule sets with exdates)
- `rrule::rrulestr()` (RFC string parsing)
- Frequency/weekday constants

**Phase 5 — Polish & Release**
Note: PyO3 bindings are built incrementally in Phases 1-4 (each phase ships its own bindings).
Phase 5 focuses on cross-platform polish and release readiness.
- Cross-platform wheel builds (manylinux, macOS, Windows) via GitHub Actions
- Full Python-side compatibility test suite (dateutil_rs vs dateutil reference)
- Criterion benchmarks for Rust-internal regression testing
- Documentation and README
- Publish to crates.io and PyPI

## Key Design Decisions

- **Chrono crate** for core date/time types (`NaiveDateTime`, `DateTime<Tz>`, etc.)
- **PyO3 + maturin** for Python bindings
- **chrono-tz** or **iana-time-zone** for timezone database
- Thread-safe by default (Rust ownership model replaces Python's lock-based caching)
- Iterator-based API for rrule (matching Python's iterable interface)
- `#[cfg(feature = "python")]` to gate PyO3 bindings — pure Rust usage without Python dependency

## Reference: Python dateutil Public API

### parser
- `parse(timestr, parserinfo=None, **kwargs)` → `DateTime`
- `isoparse(dt_str)` → `DateTime`
- `parserinfo` — customizable parsing config
- `ParserError` — parse failure exception

### relativedelta
- `relativedelta(dt1=None, dt2=None, years=0, months=0, ...)` — relative date offset
- Weekday constants: `MO`, `TU`, `WE`, `TH`, `FR`, `SA`, `SU`

### rrule
- `rrule(freq, dtstart, ...)` — recurrence rule
- `rruleset()` — composite rule set
- `rrulestr(s)` — parse RFC 5545 string
- Frequency: `YEARLY`, `MONTHLY`, `WEEKLY`, `DAILY`, `HOURLY`, `MINUTELY`, `SECONDLY`

### tz
- `tzutc()`, `tzoffset(name, offset)`, `tzlocal()`, `tzfile(path)`
- `tzrange(...)`, `tzstr(s)`, `tzical(s)`
- `gettz(name)` — timezone lookup
- `datetime_ambiguous(dt)`, `datetime_exists(dt)`, `resolve_imaginary(dt)`

### easter
- `easter(year, method=EASTER_WESTERN)`
- `EASTER_JULIAN`, `EASTER_ORTHODOX`, `EASTER_WESTERN`

### utils
- `today(tzinfo=None)`, `default_tzinfo(dt, tzinfo)`, `within_delta(dt1, dt2, delta)`

## Testing Strategy

- **Rust unit tests:** `cargo test --manifest-path crates/dateutil-rs/Cargo.toml` — Tests pure Rust logic without Python.
- **Python reference tests:** `PYTHONPATH=src uv run pytest tests/` — Tests the original python-dateutil code. Defines "correct behavior".
- **Python integration tests:** `uv run pytest` — After `maturin develop`, tests dateutil_rs against the same test expectations.
- **Benchmarks:** `uv run pytest benchmarks/ --benchmark-enable` — Python-side comparison (dateutil vs dateutil_rs). Primary performance measurement.

## Development Commands

- `cargo test --manifest-path crates/dateutil-rs/Cargo.toml` — Run Rust tests (no Python needed)
- `cargo clippy --manifest-path crates/dateutil-rs/Cargo.toml` — Rust linter
- `maturin develop --manifest-path crates/dateutil-rs/Cargo.toml -F python` — Build Python extension
- `PYTHONPATH=src uv run pytest tests/` — Run reference Python tests
- `uv run pytest benchmarks/ --benchmark-enable` — Run benchmarks
- `uv run ruff check src/ tests/` — Python linter

## Code Conventions

- **Conversation language**: Japanese
- **Code / comments / variable names**: English
- **Documentation / README / markdown**: English
- **Commit messages**: English
