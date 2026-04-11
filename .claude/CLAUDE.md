# dateutil — The Fast Date Utility Library for Python & Rust

## Project Vision

Build the **definitive date utility library** — a performance-focused alternative to python-dateutil, usable both as a native Rust crate (`dateutil-core`) and as a Python package (`python-dateutil-rs`) via PyO3/maturin.

**Design philosophy**: Cover 95%+ of real-world usage with maximum performance. Intentionally excludes rarely-used features (fuzzy parsing, POSIX TZ strings, etc.) in favor of a clean, fast API.

## Directory Structure

```
dateutil-rs/                            # Repository
├── Cargo.toml                          # Workspace root
├── Cargo.lock                          # Workspace lockfile
├── pyproject.toml                      # Python project config (maturin)
│
├── crates/
│   ├── dateutil-core/                  # Pure Rust optimized core (crates.io)
│   │   ├── Cargo.toml                  # rlib only, no PyO3
│   │   └── src/
│   │       ├── lib.rs                  # Crate root, public API
│   │       ├── common.rs              # Weekday (MO-SU with N-th occurrence)
│   │       ├── easter.rs             # easter() — 3 calendar methods
│   │       ├── error.rs              # Shared error types
│   │       ├── relativedelta.rs      # RelativeDelta (optimized)
│   │       ├── parser.rs             # parse() entry point
│   │       ├── parser/
│   │       │   ├── tokenizer.rs      # Zero-copy tokenizer
│   │       │   ├── parserinfo.rs     # Customizable parser info
│   │       │   └── isoparser.rs      # isoparse() — ISO-8601
│   │       ├── rrule.rs              # RRule entry point
│   │       ├── rrule/
│   │       │   ├── iter.rs           # Buffer-reusing iterator
│   │       │   ├── parse.rs          # rrulestr() RFC string parsing
│   │       │   └── set.rs            # RRuleSet
│   │       ├── tz.rs                 # Timezone module entry point
│   │       └── tz/
│   │           ├── utc.rs            # TzUtc — UTC timezone
│   │           ├── offset.rs         # TzOffset — fixed-offset timezone
│   │           ├── file.rs           # TzFile — TZif binary timezone
│   │           └── local.rs          # TzLocal — system local timezone
│   │
│   └── dateutil-py/                    # PyO3 binding layer → PyPI package
│       ├── Cargo.toml                  # depends on dateutil-core + pyo3
│       └── src/
│           ├── lib.rs                  # Module registration
│           ├── py.rs                   # Binding root + #[pymodule]
│           └── py/
│               ├── common.rs          # Weekday bindings
│               ├── conv.rs            # Shared conversion utilities
│               ├── easter.rs          # Easter bindings
│               ├── parser.rs          # Parser bindings
│               ├── relativedelta.rs   # RelativeDelta bindings
│               ├── rrule.rs           # RRule/RRuleSet bindings
│               └── tz.rs             # Timezone bindings
│
├── python/                             # Python package (maturin mixed layout)
│   └── dateutil_rs/                    # import dateutil_rs
│       ├── __init__.py                 # Re-exports from Rust native module
│       ├── _native.pyi                # Type stubs for native module
│       ├── py.typed                    # PEP 561 marker
│       └── parser.py                  # parserinfo (Python subclass support)
│
├── tests/                              # Python tests
├── benchmarks/                         # Performance benchmarks
├── .github/                            # CI workflows (ci.yml, publish.yml)
├── LICENSE
└── Makefile
```

### Crate Roles

| Crate | Purpose | PyO3 | Publish To |
|-------|---------|------|------------|
| `dateutil-core` | Pure Rust optimized core | No | crates.io |
| `dateutil-py` | PyO3 binding layer | Yes | PyPI (`python-dateutil-rs`) |

## Feature Scope

```
Included (covers 95%+ of real-world usage):
  ✅ parse(timestr)        — date/time string parsing (zero-copy tokenizer)
  ✅ isoparse(dt_str)      — ISO-8601 strict parsing
  ✅ parse_to_dict(timestr) — returns parsed fields as dict
  ✅ parserinfo            — customizable parser lookup tables
  ✅ relativedelta          — relative date arithmetic
  ✅ rrule / rruleset       — RFC 5545 recurrence rules
  ✅ rrulestr(s)            — RFC string parsing
  ✅ rrule __getitem__      — indexing and slicing support
  ✅ rrule count()          — total occurrence count
  ✅ rrule __contains__     — membership test
  ✅ easter(year)           — Easter date calculation
  ✅ Weekday (MO–SU)        — weekday constants with N-th occurrence
  ✅ gettz(name)            — timezone lookup (cached)
  ✅ tzutc / tzoffset       — UTC and fixed-offset timezones
  ✅ tzfile                 — TZif binary timezone files
  ✅ tzlocal                — system local timezone
  ✅ datetime_exists / datetime_ambiguous / resolve_imaginary

Excluded (legacy / low usage):
  ❌ parser fuzzy mode          — low precision, ambiguous results
  ❌ tzrange / tzstr            — POSIX TZ strings (IANA names suffice)
  ❌ tzical                     — iCalendar VTIMEZONE (rrulestr covers RFC 5545)
  ❌ parser timezone resolution — Python-specific tzinfos callback
  ❌ utils (within_delta, today, default_tzinfo) — trivial to implement in Python
  ❌ isoparser class            — isoparse() function suffices
  ❌ rrule xafter/replace       — use iter/list/between instead
```

## Key Optimizations

**Parser:**
- Zero-copy tokenizer operating on `&str` slices (`&input[start..end]`)
- `phf` crate for compile-time perfect hash lookup tables (weekdays, months, hms, ampm)
- Eliminate `VecDeque<String>` token buffer → index-based scanning
- `ParserResult` uses stack-allocated fields only

**RRule:**
- Pre-allocated year/month mask buffers reused via `clear()` + refill
- `dayset()` returns `Range<usize>` instead of `Vec<Option<usize>>`
- Batch generation with capacity-hinted output buffers
- Bitflag-based filter optimization

**General:**
- `SmallVec<[T; N]>` for small, bounded collections
- `bitflags` for efficient set operations
- Strategic `#[inline]` on hot-path functions
- Criterion benchmarks integrated in the crate for regression testing

## Measured Performance (2026-04-11, vs python-dateutil)

| Module | Speedup |
|--------|---------|
| Parser (parse) | **19.5x–36.0x** |
| Parser (isoparse) | **13.0x–38.4x** |
| RRule | **5.9x–63.7x** |
| Timezone | **1.0x–896.7x** ¹ |
| RelativeDelta | **2.0x–28.1x** |
| Easter | **5.0x–7.3x** |

¹ Excludes `tzlocal()` which reads `/etc/localtime` on every call without caching.

## Build Configuration

### Workspace Cargo.toml (root)

```toml
[workspace]
members = ["crates/dateutil-core", "crates/dateutil-py"]
resolver = "2"
```

### crates/dateutil-core/Cargo.toml

```toml
[package]
name = "dateutil-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "dateutil_core"
crate-type = ["rlib"]

[dependencies]
bitflags = "2"
chrono = "0.4"
phf = { version = "0.13", features = ["macros"] }
smallvec = "1.15"
thiserror = "2"
iana-time-zone = "0.1"

[dev-dependencies]
criterion = { version = "0.8", features = ["html_reports"] }
```

### crates/dateutil-py/Cargo.toml

```toml
[package]
name = "dateutil-py"
version = "0.1.0"
edition = "2021"

[lib]
name = "dateutil_py"
crate-type = ["rlib"]

[dependencies]
dateutil-core = { path = "../dateutil-core" }
chrono = "0.4"
pyo3 = { version = "0.28", features = ["extension-module", "chrono"], optional = true }

[features]
default = []
python = ["pyo3"]
```

### pyproject.toml

```toml
[build-system]
requires = ["maturin>=1.13"]
build-backend = "maturin"

[tool.maturin]
manifest-path = "crates/dateutil-py/Cargo.toml"
features = ["python"]
python-source = "python"
module-name = "dateutil_rs._native"
```

## Testing Strategy

- **Rust unit tests:** `cargo test -p dateutil-core` — Tests pure Rust core.
- **Rust benchmarks:** `cargo bench -p dateutil-core` — Criterion benchmarks.
- **Python integration tests:** `uv run pytest tests/` — Tests against python-dateutil.
- **Benchmarks:** `uv run pytest benchmarks/ --benchmark-enable` — Python-side comparison.

## Development Commands

### Rust (dateutil-core crate)
- `cargo test -p dateutil-core` — Run Rust tests
- `cargo clippy -p dateutil-core` — Lint code
- `cargo bench -p dateutil-core` — Run Criterion benchmarks

### Python
- `maturin develop -F python` — Build Python extension (dev)
- `maturin develop --release` — Build Python extension (release)
- `uv run pytest tests/` — Run Python tests
- `uv run pytest benchmarks/ --benchmark-enable` — Run benchmarks
- `uv run ruff check tests/ python/` — Python linter
- `uv run mypy python/` — Type checking

### Workspace-wide
- `cargo test --workspace` — Run all Rust tests
- `cargo clippy --workspace` — Lint all crates

## Code Conventions

- **Conversation language**: Japanese
- **Code / comments / variable names**: English
- **Documentation / README / markdown**: English
- **Commit messages**: English
