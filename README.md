# python-dateutil-rs

[![PyPI](https://img.shields.io/pypi/v/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![Python](https://img.shields.io/pypi/pyversions/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![License](https://img.shields.io/pypi/l/python-dateutil-rs.svg?style=flat-square)](https://pypi.org/project/python-dateutil-rs/)
[![CI](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/wakita181009/dateutil-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/wakita181009/dateutil-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/wakita181009/dateutil-rs)

A high-performance Rust-backed port of [python-dateutil](https://github.com/dateutil/dateutil) (v2.9.0).

> **Status:** Python-only phase — the full python-dateutil API is implemented in pure Python and passes the original test suite. Rust (PyO3/maturin) integration is the next milestone.

## Features

- **Drop-in replacement** for `python-dateutil` — same API, same behavior
- **Full module coverage:** parser, relativedelta, rrule, tz, easter, utils
- **Comprehensive test suite** inherited from the original project
- **Benchmark infrastructure** for side-by-side performance comparison (original vs local)
- **Python 3.10–3.14** supported on Linux and macOS

## Installation

```bash
pip install python-dateutil-rs
```

## Usage

```python
from dateutil.parser import parse
from dateutil.relativedelta import relativedelta
from dateutil.rrule import rrule, MONTHLY
from dateutil.tz import gettz, tzutc
from dateutil.easter import easter

# Parse date strings
dt = parse("2024-01-15T10:30:00+09:00")

# Relative deltas
next_month = dt + relativedelta(months=+1)

# Recurrence rules
monthly = rrule(MONTHLY, count=5, dtstart=parse("2024-01-01"))

# Timezones
tokyo = gettz("Asia/Tokyo")
utc = tzutc()

# Easter
easter_date = easter(2024)
```

## Development

### Prerequisites

- Python 3.10+
- [uv](https://github.com/astral-sh/uv) (recommended) or pip

### Setup

```bash
git clone https://github.com/wakita181009/dateutil-rs.git
cd dateutil-rs
uv sync --extra dev
```

### Running Tests

```bash
# Run the test suite
uv run pytest tests/ -x -q

# Run with coverage
uv run pytest tests/ --cov=dateutil
```

### Linting

```bash
uv run ruff check src/ tests/
uv run ruff format --check src/ tests/
```

### Benchmarks

Benchmarks compare the original `python-dateutil` (from PyPI) against the local implementation side-by-side using pytest-benchmark.

```bash
# Install the original python-dateutil for comparison
uv pip install python-dateutil

# Run benchmarks
make bench

# Run and save results as JSON
make bench-save
```

## Project Structure

```
dateutil-rs/
├── src/dateutil/          # Python implementation (dateutil API)
│   ├── parser/            # Date/time string parsing + ISO-8601
│   ├── tz/                # Timezone support (tzfile, tzstr, tzlocal, ...)
│   ├── rrule.py           # Recurrence rules (RFC 5545)
│   ├── relativedelta.py   # Relative date arithmetic
│   ├── easter.py          # Easter date calculations
│   └── utils.py           # Utility functions
├── tests/                 # Test suite (~13k lines)
├── benchmarks/            # pytest-benchmark comparisons
├── .github/workflows/     # CI (lint + test matrix)
├── pyproject.toml
├── Makefile
└── LICENSE
```

## Roadmap

1. **~~Python-only phase~~** — Pure Python port with full test coverage ✅
2. **Rust core** — Rewrite performance-critical modules in Rust (`crates/dateutil-rs/`)
3. **PyO3 bindings** — Expose Rust implementation as a Python extension module via maturin
4. **Hybrid package** — Python fallback with Rust acceleration where available
5. **Release** — Publish to crates.io and PyPI with pre-built wheels (manylinux, macOS, Windows)

## License

[MIT](LICENSE)
