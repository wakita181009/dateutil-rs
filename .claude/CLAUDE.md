# dateutil — The Fast Date Utility Library for Python & Rust

## Project Vision

Build the **definitive date utility library** — a drop-in replacement for python-dateutil with massive performance gains, usable both as a native Rust crate (`dateutil`) and as a Python package (`dateutil`) via PyO3/maturin.

### Versioning Strategy

| Version | Codename | Description |
|---------|----------|-------------|
| **0.x** | v0 | Full python-dateutil API compatibility. Direct Rust port of python-dateutil v2.9.0. Priority: correctness & compatibility. |
| **1.x** | v1 | Rust-optimized minimal core. Clean break from legacy. Priority: performance & simplicity. |

- `pip install dateutil==0.x` → drop-in replacement for python-dateutil
- `pip install dateutil==1.x` → blazing fast, streamlined API
- Rust users: `dateutil = "1"` on crates.io

## Directory Structure

```
dateutil-rs/                            # Repository
├── Cargo.toml                          # Workspace root
├── pyproject.toml                      # Python project config (maturin)
│
├── crates/
│   ├── dateutil/                       # v1: Pure Rust optimized core (crates.io)
│   │   ├── Cargo.toml                  # rlib only, no PyO3
│   │   └── src/
│   │       ├── lib.rs                  # Crate root, public API
│   │       ├── common.rs              # Weekday (MO-SU with N-th occurrence)
│   │       ├── easter.rs             # easter() — 3 calendar methods
│   │       ├── relativedelta.rs      # RelativeDelta (optimized)
│   │       ├── parser/
│   │       │   ├── mod.rs            # parse() — zero-copy tokenizer
│   │       │   └── isoparser.rs      # isoparse() — ISO-8601
│   │       ├── rrule/
│   │       │   ├── mod.rs            # RRule, RRuleSet, rrulestr()
│   │       │   └── iter.rs           # Buffer-reusing iterator
│   │       └── tz/
│   │           ├── mod.rs            # gettz(), Tz enum
│   │           ├── utc.rs            # TzUtc
│   │           ├── offset.rs         # TzOffset
│   │           ├── file.rs           # TzFile (TZif binary)
│   │           └── local.rs          # TzLocal
│   │
│   ├── dateutil-py/                    # PyO3 thin binding layer (PyPI)
│   │   ├── Cargo.toml                  # depends on dateutil + pyo3
│   │   └── src/
│   │       └── lib.rs                  # Python API ↔ Rust core conversion
│   │
│   └── dateutil-rs/                    # v0: Current code (python-dateutil compat)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
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
│   └── dateutil/                       # import dateutil (renamed from dateutil_rs)
│       ├── __init__.py                 # Re-exports from Rust native module
│       ├── py.typed                    # PEP 561 marker
│       ├── common.py
│       ├── easter.py
│       ├── parser.py
│       ├── relativedelta.py
│       ├── rrule.py
│       ├── tz.py
│       └── utils.py
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
| `dateutil` | v1 pure Rust optimized core | No | crates.io |
| `dateutil-py` | Thin PyO3 binding layer | Yes | PyPI (`dateutil`) |
| `dateutil-rs` | v0 python-dateutil compat (current code) | Yes | (v0 period only) |

### Migration Path

```
v0.x release:  dateutil-py  → wraps dateutil-rs (current code)
v1.0 develop:  dateutil crate built in parallel
v1.0 release:  dateutil-py  → wraps dateutil (new optimized core)
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

## v1 — Rust-Optimized Core (New)

### Design Principles

1. **Zero-copy where possible** — `&str` slices instead of `String` clones
2. **Buffer reuse** — pre-allocated buffers cleared and reused across iterations
3. **Compile-time optimization** — `phf` perfect hash maps, const evaluation
4. **Minimal allocations** — `SmallVec`, stack buffers, arena patterns
5. **No legacy baggage** — drop rarely-used features that add complexity

### v1 Feature Scope

```
Included (covers 95%+ of real-world usage):
  ✅ parse(timestr)        — date/time string parsing (zero-copy tokenizer)
  ✅ isoparse(dt_str)      — ISO-8601 strict parsing
  ✅ relativedelta          — relative date arithmetic
  ✅ rrule / rruleset       — RFC 5545 recurrence rules
  ✅ rrulestr(s)            — RFC string parsing
  ✅ gettz(name)            — timezone lookup (IANA names)
  ✅ tzutc / tzoffset       — UTC and fixed-offset timezones
  ✅ tzfile                 — TZif binary timezone files
  ✅ tzlocal                — system local timezone
  ✅ easter(year)           — Easter date calculation
  ✅ Weekday (MO–SU)        — weekday constants with N-th occurrence

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
- `IterInfo` holds `Arc<RRule>` reference instead of cloning entire struct
- Pre-allocated year/month mask buffers reused via `clear()` + refill
- `dayset()` returns `Range<usize>` instead of `Vec<Option<usize>>`
- Batch generation with capacity-hinted output buffers

**Timezone:**
- `TtInfo` abbreviation as `SmallStr<8>` or interned string (no heap per-transition)
- `tzname()` returns `&str` instead of `String`
- Path lookup uses stack-allocated `[u8; 256]` buffer
- `gettz()` cache unchanged (already optimal with `OnceLock<RwLock<HashMap>>`)

**General:**
- `SmallVec<[T; N]>` for small, bounded collections
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

### v1 Implementation Phases

**Phase 1 — Scaffold & Small Modules**
- Workspace setup, `dateutil` crate skeleton
- `common::Weekday`, `easter::easter()`
- `relativedelta::RelativeDelta` (optimized from scratch)
- Criterion benchmark harness

**Phase 2 — Parser**
- Zero-copy tokenizer
- `phf`-based lookup tables
- `parse()` and `isoparse()`
- Benchmark comparison vs v0

**Phase 3 — RRule**
- Buffer-reusing `IterInfo`
- `RRule`, `RRuleSet`, `rrulestr()`
- Iterator with pre-allocated output

**Phase 4 — Timezone**
- `TzFile` with `SmallStr` abbreviations
- `gettz()` with borrowed API
- `TzUtc`, `TzOffset`, `TzLocal`

**Phase 5 — PyO3 Bindings & Release**
- `dateutil-py` crate wrapping `dateutil` core
- Python `dateutil/` package pointing to new backend
- Full test suite passing
- Publish dateutil 1.0 to PyPI and crates.io

## Build Configuration

### Workspace Cargo.toml (root)

```toml
[workspace]
members = [
    "crates/dateutil",
    "crates/dateutil-py",
    "crates/dateutil-rs",
]
resolver = "2"
```

### crates/dateutil/Cargo.toml (v1 core)

```toml
[package]
name = "dateutil"
version = "0.1.0"
edition = "2021"

[lib]
name = "dateutil"
crate-type = ["rlib"]

[dependencies]
chrono = "0.4"
thiserror = "2"
phf = { version = "0.11", features = ["macros"] }
smallvec = "1"

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
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
dateutil = { path = "../dateutil" }
pyo3 = { version = "0.28", features = ["extension-module", "chrono"], optional = true }

[features]
default = []
python = ["pyo3"]
```

### crates/dateutil-rs/Cargo.toml (v0 — unchanged)

```toml
[package]
name = "dateutil-rs"
version = "0.0.8"
edition = "2021"

[lib]
name = "dateutil_rs"
crate-type = ["rlib"]

[dependencies]
chrono = "0.4"
thiserror = "2"
pyo3 = { version = "0.28", features = ["extension-module", "chrono"], optional = true }

[features]
default = []
python = ["pyo3"]
```

### pyproject.toml

```toml
[build-system]
requires = ["maturin>=1.0"]
build-backend = "maturin"

[tool.maturin]
manifest-path = "crates/dateutil-py/Cargo.toml"
features = ["python"]
python-source = "python"
module-name = "dateutil._native"
```

## Testing Strategy

- **v1 Rust unit tests:** `cargo test -p dateutil` — Tests pure Rust core without Python.
- **v0 Rust unit tests:** `cargo test -p dateutil-rs` — Tests v0 Rust logic.
- **Rust benchmarks:** `cargo bench -p dateutil` — Criterion benchmarks for v1 core.
- **Python reference tests:** `uv run pytest tests/` — Tests against python-dateutil. Defines "correct behavior".
- **Python integration tests:** `uv run pytest` — After `maturin develop`, tests dateutil package.
- **Benchmarks:** `uv run pytest benchmarks/ --benchmark-enable` — Python-side comparison.

## Development Commands

### v1 Core (dateutil crate)
- `cargo test -p dateutil` — Run v1 Rust tests
- `cargo clippy -p dateutil` — Lint v1 code
- `cargo bench -p dateutil` — Run Criterion benchmarks

### v0 Compat (dateutil-rs crate)
- `cargo test -p dateutil-rs` — Run v0 Rust tests
- `cargo clippy -p dateutil-rs` — Lint v0 code

### Python
- `maturin develop --manifest-path crates/dateutil-py/Cargo.toml -F python` — Build Python extension
- `uv run pytest tests/` — Run reference Python tests
- `uv run pytest benchmarks/ --benchmark-enable` — Run benchmarks
- `uv run ruff check tests/ python/` — Python linter

### Workspace-wide
- `cargo test --workspace` — Run all Rust tests
- `cargo clippy --workspace` — Lint all crates

## Code Conventions

- **Conversation language**: Japanese
- **Code / comments / variable names**: English
- **Documentation / README / markdown**: English
- **Commit messages**: English
