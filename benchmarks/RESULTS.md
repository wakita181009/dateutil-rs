# Benchmark Results

> Last updated: 2026-04-07 | Commit: `914821d`

## Environment

- **Python:** 3.13.12
- **Platform:** macOS 26.3.1, Apple Silicon (arm64)
- **Tool:** pytest-benchmark 5.2.3
- **Build:** `maturin develop --release`

## Summary

Benchmarks compare two implementations side-by-side:

| Variant | Description |
|---------|-------------|
| **original** | `python-dateutil` from PyPI (v2.9.0.post0) |
| **rust** | Rust implementation via PyO3 (`dateutil_rs`) |

### Implementation Status

| Module | Rust Status | Speedup vs original |
|--------|-------------|---------------------|
| easter | Implemented | **3.2x – 6.2x** |
| relativedelta | Implemented | **3.5x – 18.7x** |
| parser (parse) | Implemented | **1.3x – 3.5x** |
| parser (isoparse) | Implemented | **5.1x – 23.5x** |
| rrule | Implemented | **1.7x – 9.1x** |
| tz | Implemented | **1.0x – 3.4x** (varies) |

---

## Easter

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| single (Western) | 0.51 µs | 0.13 µs | **3.9x** |
| single (Orthodox) | 0.35 µs | 0.11 µs | **3.2x** |
| single (Julian) | 0.29 µs | 0.06 µs | **4.9x** |
| range 1000 years (Western) | 437.88 µs | 70.46 µs | **6.2x** |
| range 500 years × 3 methods | 567.99 µs | 110.33 µs | **5.2x** |

## RelativeDelta

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| create simple | 0.94 µs | 0.19 µs | **4.9x** |
| create complex | 0.91 µs | 0.26 µs | **3.5x** |
| create absolute | 1.04 µs | 0.20 µs | **5.2x** |
| create weekday | 0.91 µs | 0.13 µs | **7.0x** |
| add months | 1.62 µs | 0.13 µs | **12.7x** |
| add complex | 1.67 µs | 0.14 µs | **12.3x** |
| add weekday | 2.46 µs | 0.50 µs | **4.9x** |
| subtract | 3.03 µs | 0.18 µs | **16.5x** |
| multiply | 1.49 µs | 0.08 µs | **18.7x** |
| diff dates | 2.81 µs | 0.33 µs | **8.6x** |
| diff datetimes | 2.72 µs | 0.26 µs | **10.5x** |
| sequential add (×12) | 19.41 µs | 1.70 µs | **11.4x** |
| month-end overflow | 1.74 µs | 0.13 µs | **13.3x** |

## Parser — parse()

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| simple date | 8.78 µs | 5.96 µs | **1.5x** |
| datetime with time | 14.00 µs | 6.12 µs | **2.3x** |
| datetime with tz | 17.99 µs | 6.52 µs | **2.8x** |
| American format | 14.83 µs | 6.34 µs | **2.3x** |
| European format | 9.01 µs | 5.56 µs | **1.6x** |
| with microseconds | 15.69 µs | 5.89 µs | **2.7x** |
| fuzzy parsing | 29.42 µs | 8.45 µs | **3.5x** |
| relative with default | 6.56 µs | 5.13 µs | **1.3x** |
| 10 various formats | 165.30 µs | 63.13 µs | **2.6x** |

## Parser — isoparse()

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| isoparse date | 0.81 µs | 0.08 µs | **10.7x** |
| isoparse datetime | 2.06 µs | 0.10 µs | **20.9x** |
| isoparse datetime+tz | 3.11 µs | 0.61 µs | **5.1x** |
| isoparse datetime+UTC | 2.32 µs | 0.39 µs | **6.0x** |
| isoparse compact | 1.98 µs | 0.15 µs | **13.4x** |
| isoparse with µs | 2.67 µs | 0.11 µs | **23.5x** |
| isoparse various | 23.10 µs | 3.10 µs | **7.4x** |

## RRule

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| daily 100 | 105.63 µs | 63.11 µs | **1.7x** |
| daily interval=3 | 129.37 µs | 72.78 µs | **1.8x** |
| weekly 52 | 105.33 µs | 34.44 µs | **3.1x** |
| weekly byday interval=2 | 137.45 µs | 49.78 µs | **2.8x** |
| monthly 120 | 448.46 µs | 114.12 µs | **3.9x** |
| monthly byday | 19.56 µs | 11.13 µs | **1.8x** |
| monthly bymonthday | 130.80 µs | 45.09 µs | **2.9x** |
| yearly 100 | 2,144.58 µs | 235.02 µs | **9.1x** |
| yearly bymonth | 669.32 µs | 81.82 µs | **8.2x** |
| hourly 1000 | 1,269.24 µs | 645.47 µs | **2.0x** |
| rruleset union | 129.28 µs | 69.38 µs | **1.9x** |
| rruleset exdate | 146.35 µs | 70.43 µs | **2.1x** |
| rruleset exrule | 624.26 µs | 220.76 µs | **2.8x** |
| rrulestr simple | 3.14 µs | 0.53 µs | **6.0x** |
| rrulestr complex | 6.60 µs | 0.90 µs | **7.4x** |
| rrulestr with dtstart | 15.61 µs | 0.78 µs | **20.0x** |

## Timezone

| Benchmark | original | rust | Speedup |
|-----------|----------|------|---------|
| tzutc create | 0.06 µs | 0.07 µs | 1.0x |
| tzoffset create | 0.43 µs | 0.56 µs | 0.8x |
| tzlocal create | 0.49 µs | 0.23 µs | **2.1x** |
| gettz UTC | 0.33 µs | 21.77 µs | 0.02x (*) |
| gettz named | 0.31 µs | 23.54 µs | 0.01x (*) |
| gettz offset | 22.57 µs | 5.40 µs | **4.2x** |
| convert UTC→JST | 1.44 µs | 1.43 µs | 1.0x |
| convert UTC→Eastern | 1.66 µs | 1.47 µs | **1.1x** |
| convert chain | 7.16 µs | 5.33 µs | **1.3x** |
| localize naive | 0.08 µs | 0.08 µs | 1.0x |
| datetime_exists | 3.22 µs | 2.05 µs | **1.6x** |
| datetime_ambiguous | 1.07 µs | 0.65 µs | **1.6x** |
| resolve_imaginary | 6.71 µs | 2.11 µs | **3.2x** |
| gettz various (×10) | 786.67 µs | 231.92 µs | **3.4x** |

(*) python-dateutil caches `gettz()` results via `_TzFactory`. Single repeated lookups appear faster in the original because the factory returns a cached singleton. The Rust implementation does not cache yet; each call performs a fresh filesystem lookup.

---

## How to Reproduce

```bash
# Install dependencies
uv pip install python-dateutil

# Build Rust extension (release mode)
maturin develop --release

# Run benchmarks
make bench

# Save results as JSON
make bench-save
```
