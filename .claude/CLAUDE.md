# dateutil — The Fast Date Utility Library for Python & Rust

## Project Vision

Build the **definitive date utility library** — a drop-in replacement for python-dateutil with massive performance gains, usable both as a native Rust crate and as a Python package (`python-dateutil-rs`) via PyO3/maturin.

### Versioning Strategy

| Version | Codename | Description |
|---------|----------|-------------|
| **0.x** | v0 | Full python-dateutil API compatibility. Direct Rust port of python-dateutil v2.9.0. Priority: correctness & compatibility. |
| **1.x** | v1 | Rust-optimized minimal core. Clean break from legacy. Priority: performance & simplicity. |

- `pip install python-dateutil-rs==0.x` → drop-in replacement for python-dateutil
- `pip install python-dateutil-rs==1.x` → blazing fast, streamlined API
- Rust users: `dateutil-core` on crates.io

## Directory Structure

```
dateutil-rs/                            # Repository
├── Cargo.toml                          # Workspace root
├── Cargo.lock                          # Workspace lockfile
├── pyproject.toml                      # Python project config (maturin)
│
├── crates/
│   ├── dateutil-core/                  # v1: Pure Rust optimized core (crates.io)
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
│   │       │   └── isoparser.rs      # isoparse() — ISO-8601
│   │       ├── rrule.rs              # RRule entry point
│   │       ├── rrule/
│   │       │   ├── iter.rs           # Buffer-reusing iterator
│   │       │   ├── parse.rs          # rrulestr() RFC string parsing
│   │       │   └── set.rs            # RRuleSet
│   │       └── (tz/ planned)
│   │
│   ├── dateutil-py/                    # PyO3 thin binding layer
│   │   ├── Cargo.toml                  # depends on dateutil-core + pyo3
│   │   └── src/
│   │       ├── lib.rs                  # Module registration
│   │       ├── py.rs                   # Binding root
│   │       └── py/
│   │           ├── common.rs          # Weekday bindings
│   │           ├── easter.rs          # Easter bindings
│   │           ├── parser.rs          # Parser bindings
│   │           ├── relativedelta.rs   # RelativeDelta bindings
│   │           └── rrule.rs           # RRule/RRuleSet bindings
│   │
│   └── dateutil-rs/                    # v0: python-dateutil compat
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # Crate root + unified #[pymodule]
│           ├── common.rs
│           ├── easter.rs
│           ├── utils.rs
│           ├── relativedelta.rs
│           ├── parser/
│           │   ├── mod.rs
│           │   └── isoparser.rs
│           ├── rrule/
│           │   ├── mod.rs
│           │   └── iter.rs
│           └── tz/
│               ├── mod.rs
│               ├── utc.rs
│               ├── offset.rs
│               ├── file.rs
│               ├── local.rs
│               └── range.rs
│
├── python/                             # Python package (maturin mixed layout)
│   └── dateutil_rs/                    # import dateutil_rs
│       ├── __init__.py                 # Re-exports from Rust native module
│       ├── _native.pyi                # Type stubs for v0 native module
│       ├── py.typed                    # PEP 561 marker
│       ├── common.py
│       ├── easter.py
│       ├── parser.py
│       ├── relativedelta.py
│       ├── rrule.py
│       ├── tz.py
│       ├── utils.py
│       └── v1/                         # v1 optimized API
│           ├── __init__.py
│           ├── _native.pyi            # Type stubs for v1 native module
│           ├── py.typed
│           ├── common.py
│           ├── easter.py
│           ├── parser.py
│           ├── relativedelta.py
│           └── rrule.py
│
├── tests/                              # Python tests (from python-dateutil)
├── benchmarks/                         # Performance benchmarks
├── .github/                            # CI workflows
├── LICENSE
└── Makefile
```

### Crate Roles

| Crate | Purpose | PyO3 | Publish To |
|-------|---------|------|------------|
| `dateutil-core` | v1 pure Rust optimized core | No | crates.io |
| `dateutil-py` | Thin PyO3 binding layer for v1 | Yes | (via dateutil-rs) |
| `dateutil-rs` | v0 python-dateutil compat + unified native module | Yes | PyPI (`python-dateutil-rs`) |

### Unified Native Module

Both v0 and v1 are compiled into a single `_native` shared library. The `dateutil-rs` crate depends on `dateutil-py` (via feature flag `v1`) and registers both module trees under one `#[pymodule]`:

```
dateutil_rs._native       → v0 API (parser, rrule, tz, etc.)
dateutil_rs.v1._native    → v1 API (parse, rrule, rruleset, etc.)
```

### Migration Path

```
v0.x release:  dateutil-rs  → v0 compat (current)
               dateutil-py  → v1 bindings (embedded in dateutil-rs via feature flag)
v1.0 release:  dateutil-py  → wraps dateutil-core (standalone)
v0.x EOL:      dateutil-rs crate archived
```

## v0 — python-dateutil Compatibility (Current)

Full python-dateutil v2.9.0 API compatibility. All modules implemented:

| Python Module | Rust Module | Status | Speedup |
|---|---|---|---|
| `dateutil.parser` | `dateutil_rs::parser` | ✅ | 1.3x–23.5x |
| `dateutil.rrule` | `dateutil_rs::rrule` | ✅ | 1.7x–9.1x |
| `dateutil.tz` | `dateutil_rs::tz` | ✅ | 1.0x–94.3x |
| `dateutil.relativedelta` | `dateutil_rs::relativedelta` | ✅ | 3.5x–18.7x |
| `dateutil.easter` | `dateutil_rs::easter` | ✅ | 3.2x–6.2x |
| `dateutil.utils` | `dateutil_rs::utils` | ✅ (partial) | — |
| `dateutil._common` | `dateutil_rs::common` | ✅ | — |

## v1 — Rust-Optimized Core

### Design Principles

1. **Zero-copy where possible** — `&str` slices instead of `String` clones
2. **Buffer reuse** — pre-allocated buffers cleared and reused across iterations
3. **Compile-time optimization** — `phf` perfect hash maps, const evaluation
4. **Minimal allocations** — `SmallVec`, stack buffers, bitflags
5. **No legacy baggage** — drop rarely-used features that add complexity

### v1 Implementation Status

| Module | Rust Crate | PyO3 Bindings | Status |
|--------|-----------|---------------|--------|
| `common` (Weekday) | `dateutil_core::common` | `dateutil_py::common` | ✅ Complete |
| `easter` | `dateutil_core::easter` | `dateutil_py::easter` | ✅ Complete |
| `relativedelta` | `dateutil_core::relativedelta` | `dateutil_py::relativedelta` | ✅ Complete |
| `parser` | `dateutil_core::parser` | `dateutil_py::parser` | ✅ Complete |
| `rrule` | `dateutil_core::rrule` | `dateutil_py::rrule` | ✅ Complete |
| `tz` | — | — | ❌ Not yet |

### v1 Feature Scope

```
Included (covers 95%+ of real-world usage):
  ✅ parse(timestr)        — date/time string parsing (zero-copy tokenizer)
  ✅ isoparse(dt_str)      — ISO-8601 strict parsing
  ✅ relativedelta          — relative date arithmetic
  ✅ rrule / rruleset       — RFC 5545 recurrence rules
  ✅ rrulestr(s)            — RFC string parsing
  ✅ easter(year)           — Easter date calculation
  ✅ Weekday (MO–SU)        — weekday constants with N-th occurrence
  🔲 gettz(name)            — timezone lookup (planned)
  🔲 tzutc / tzoffset       — UTC and fixed-offset timezones (planned)
  🔲 tzfile                 — TZif binary timezone files (planned)
  🔲 tzlocal                — system local timezone (planned)

Excluded (legacy / low usage):
  ❌ parserinfo customization  — complex, rarely used
  ❌ parser fuzzy mode          — low precision, ambiguous results
  ❌ tzrange / tzstr            — POSIX TZ strings (IANA names suffice)
  ❌ tzical                     — iCalendar VTIMEZONE (rrulestr covers RFC 5545)
  ❌ parser timezone resolution — Python-specific tzinfos callback
```

### v1 Key Optimizations

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

### v1 Target Performance

| Module | v0 Speedup (vs Python) | v1 Target |
|--------|------------------------|-----------|
| Parser | 1.3x–23.5x | 5x–50x |
| RRule | 1.7x–9.1x | 5x–15x |
| Timezone | 1.0x–94.3x | 2x–100x |
| RelativeDelta | 3.5x–18.7x | 10x–30x |
| Easter | 3.2x–6.2x | 5x–10x |

### v1 Remaining Phases

**Phase 4 — Timezone**
- `TzFile` with optimized abbreviations
- `gettz()` with borrowed API
- `TzUtc`, `TzOffset`, `TzLocal`

**Phase 5 — PyO3 Bindings & Release**
- Full test suite passing for v1
- Publish dateutil-core to crates.io
- Publish python-dateutil-rs 1.0 to PyPI

## Build Configuration

### Workspace Cargo.toml (root)

```toml
[workspace]
members = ["crates/dateutil-core", "crates/dateutil-py", "crates/dateutil-rs"]
resolver = "2"
```

### crates/dateutil-core/Cargo.toml (v1 core)

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

[dev-dependencies]
criterion = { version = "0.8", features = ["html_reports"] }
```

### crates/dateutil-py/Cargo.toml (PyO3 bindings)

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

### crates/dateutil-rs/Cargo.toml (v0 + unified module)

```toml
[package]
name = "dateutil-rs"
version = "0.0.12"
edition = "2021"

[lib]
name = "dateutil_rs"
crate-type = ["rlib"]

[dependencies]
chrono = "0.4"
thiserror = "2"
pyo3 = { version = "0.28", features = ["extension-module", "chrono"], optional = true }
dateutil-py = { path = "../dateutil-py", optional = true }

[features]
default = []
python = ["pyo3"]
v1 = ["dateutil-py/python"]
```

### pyproject.toml

```toml
[build-system]
requires = ["maturin>=1.0"]
build-backend = "maturin"

[tool.maturin]
manifest-path = "crates/dateutil-rs/Cargo.toml"
features = ["python", "v1"]
python-source = "python"
module-name = "dateutil_rs._native"
```

## Testing Strategy

- **v1 Rust unit tests:** `cargo test -p dateutil-core` — Tests pure Rust core without Python.
- **v0 Rust unit tests:** `cargo test -p dateutil-rs` — Tests v0 Rust logic.
- **Rust benchmarks:** `cargo bench -p dateutil-core` — Criterion benchmarks for v1 core.
- **Python reference tests:** `uv run pytest tests/` — Tests against python-dateutil. Defines "correct behavior".
- **Python integration tests:** `uv run pytest` — After `maturin develop`, tests dateutil package.
- **Benchmarks:** `uv run pytest benchmarks/ --benchmark-enable` — Python-side comparison.

## Development Commands

### v1 Core (dateutil-core crate)
- `cargo test -p dateutil-core` — Run v1 Rust tests
- `cargo clippy -p dateutil-core` — Lint v1 code
- `cargo bench -p dateutil-core` — Run Criterion benchmarks

### v0 Compat (dateutil-rs crate)
- `cargo test -p dateutil-rs` — Run v0 Rust tests
- `cargo clippy -p dateutil-rs` — Lint v0 code

### Python
- `maturin develop -F python -F v1` — Build Python extension (v0 + v1)
- `uv run pytest tests/` — Run reference Python tests
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
