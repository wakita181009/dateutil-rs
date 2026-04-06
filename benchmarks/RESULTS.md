# Benchmark Results

> Last updated: 2026-04-07 | Commit: `fb59aed7`

## Environment

- **Python:** 3.13.12
- **Platform:** macOS 26.3.1, Apple Silicon (arm64)
- **Tool:** pytest-benchmark 5.2.3

## Summary

Benchmarks compare three implementations side-by-side:

| Variant | Description |
|---------|-------------|
| **original** | `python-dateutil` from PyPI (v2.9.0.post0) |
| **local** | Pure Python port in `src/dateutil/` |
| **rust** | Rust implementation via PyO3 (`dateutil_rs`) |

### Implementation Status

| Module | Rust Status | Speedup vs original |
|--------|-------------|---------------------|
| easter | Implemented | **3.7x – 6.2x** |
| parser | Not yet | — |
| relativedelta | Not yet | — |
| rrule | Not yet | — |
| tz | Not yet | — |

---

## Easter

| Benchmark | original | local | rust | Speedup |
|-----------|----------|-------|------|---------|
| single (Western) | 0.49 µs | 0.43 µs | 0.11 µs | **4.4x** |
| single (Orthodox) | 0.39 µs | 0.34 µs | 0.11 µs | **3.7x** |
| single (Julian) | 0.29 µs | 0.29 µs | 0.06 µs | **4.9x** |
| range 1000 years (Western) | 423.92 µs | 424.89 µs | 68.32 µs | **6.2x** |
| range 500 years × 3 methods | 558.51 µs | 560.31 µs | 107.97 µs | **5.2x** |

## Parser

| Benchmark | original | local | Ratio |
|-----------|----------|-------|-------|
| simple date | 8.47 µs | 8.33 µs | 1.0x |
| datetime with time | 13.74 µs | 13.84 µs | 1.0x |
| datetime with tz | 17.60 µs | 17.88 µs | 1.0x |
| American format | 14.00 µs | 14.17 µs | 1.0x |
| European format | 8.59 µs | 8.69 µs | 1.0x |
| fuzzy parsing | 29.09 µs | 28.77 µs | 1.0x |
| relative with default | 6.54 µs | 6.41 µs | 1.0x |
| with microseconds | 15.45 µs | 15.35 µs | 1.0x |
| 10 various formats | 163.52 µs | 160.68 µs | 1.0x |
| isoparse date | 0.81 µs | 0.81 µs | 1.0x |
| isoparse datetime | 2.04 µs | 2.04 µs | 1.0x |
| isoparse datetime+tz | 3.02 µs | 3.08 µs | 1.0x |
| isoparse datetime+UTC | 2.29 µs | 2.30 µs | 1.0x |
| isoparse compact | 1.97 µs | 1.94 µs | 1.0x |
| isoparse with µs | 2.61 µs | 2.61 µs | 1.0x |
| isoparse various | 23.45 µs | 23.19 µs | 1.0x |

## RelativeDelta

| Benchmark | original | local | Ratio |
|-----------|----------|-------|-------|
| create simple | 0.93 µs | 0.93 µs | 1.0x |
| create complex | 0.91 µs | 0.90 µs | 1.0x |
| create absolute | 1.05 µs | 1.00 µs | 1.1x |
| create weekday | 0.89 µs | 0.88 µs | 1.0x |
| add months | 1.57 µs | 1.58 µs | 1.0x |
| add complex | 1.62 µs | 1.64 µs | 1.0x |
| add weekday | 1.77 µs | 1.77 µs | 1.0x |
| subtract | 2.93 µs | 2.94 µs | 1.0x |
| multiply | 1.44 µs | 1.45 µs | 1.0x |
| diff dates | 2.71 µs | 2.69 µs | 1.0x |
| diff datetimes | 2.68 µs | 2.66 µs | 1.0x |
| sequential add (×10) | 18.49 µs | 18.46 µs | 1.0x |
| month-end overflow | 1.58 µs | 1.57 µs | 1.0x |

## RRule

| Benchmark | original | local | Ratio |
|-----------|----------|-------|-------|
| daily 100 | 102.19 µs | 103.90 µs | 1.0x |
| daily interval=3 | 110.28 µs | 111.69 µs | 1.0x |
| weekly 52 | 103.78 µs | 104.21 µs | 1.0x |
| weekly byday interval=2 | 133.11 µs | 132.32 µs | 1.0x |
| monthly 120 | 432.43 µs | 437.79 µs | 1.0x |
| monthly byday | 18.04 µs | 17.66 µs | 1.0x |
| monthly bymonthday | 128.95 µs | 129.33 µs | 1.0x |
| yearly 100 | 2,121.93 µs | 2,076.06 µs | 1.0x |
| yearly bymonth | 661.77 µs | 662.37 µs | 1.0x |
| hourly 1000 | 1,257.83 µs | 1,257.12 µs | 1.0x |
| rruleset union | 128.49 µs | 128.36 µs | 1.0x |
| rruleset exdate | 136.24 µs | 135.42 µs | 1.0x |
| rruleset exrule | 614.90 µs | 618.74 µs | 1.0x |
| rrulestr simple | 2.99 µs | 2.96 µs | 1.0x |
| rrulestr complex | 6.53 µs | 6.43 µs | 1.0x |
| rrulestr with dtstart | 15.81 µs | 14.98 µs | 1.1x |

## Timezone

| Benchmark | original | local | Ratio |
|-----------|----------|-------|-------|
| tzutc create | 0.06 µs | 0.06 µs | 1.0x |
| tzoffset create | 0.42 µs | 0.42 µs | 1.0x |
| tzlocal create | 0.47 µs | 0.47 µs | 1.0x |
| gettz UTC | 0.30 µs | 0.31 µs | 1.0x |
| gettz named | 0.30 µs | 0.31 µs | 1.0x |
| gettz offset | 21.60 µs | 64.03 µs | 0.3x |
| convert UTC→JST | 1.39 µs | 1.38 µs | 1.0x |
| convert UTC→Eastern | 1.66 µs | 1.66 µs | 1.0x |
| convert chain | 7.15 µs | 6.81 µs | 1.0x |
| localize naive | 0.07 µs | 0.07 µs | 1.0x |
| datetime_exists | 3.34 µs | 3.16 µs | 1.1x |
| datetime_ambiguous | 1.02 µs | 1.03 µs | 1.0x |
| resolve_imaginary | 6.59 µs | 6.48 µs | 1.0x |
| gettz various (×10) | 728.30 µs | 719.95 µs | 1.0x |

---

## How to Reproduce

```bash
# Install dependencies
uv pip install python-dateutil

# Build Rust extension
maturin develop --manifest-path crates/dateutil-rs/Cargo.toml -F python

# Run benchmarks
make bench

# Save results as JSON
make bench-save
```