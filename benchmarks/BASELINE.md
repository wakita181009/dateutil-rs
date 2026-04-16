# python-dateutil Baseline Benchmarks

> Last updated: 2026-04-16 | python-dateutil v2.9.0.post0

## Environment

- **Python:** 3.13.12
- **Platform:** macOS 26.3.0, Apple Silicon (arm64)
- **Tool:** pytest-benchmark 5.2.3
- **Package:** python-dateutil 2.9.0.post0 (PyPI)

---

## Easter

| Benchmark | Min | Mean | Median | StdDev |
|-----------|----:|-----:|-------:|-------:|
| single (Western) | 375.0 ns | 499.1 ns | 459.0 ns | 362.1 ns |
| single (Orthodox) | 322.2 ns | 352.0 ns | 341.7 ns | 71.8 ns |
| single (Julian) | 273.1 ns | 291.5 ns | 287.1 ns | 40.3 ns |
| range 1000 years (Western) | 415.9 us | 434.9 us | 430.3 us | 14.4 us |
| range 500 years x 3 methods | 543.5 us | 565.7 us | 561.4 us | 16.5 us |

## RelativeDelta

| Benchmark | Min | Mean | Median | StdDev |
|-----------|----:|-----:|-------:|-------:|
| create simple | 791.0 ns | 954.6 ns | 917.0 ns | 421.3 ns |
| create complex | 840.3 ns | 904.7 ns | 888.8 ns | 151.6 ns |
| create absolute | 933.2 ns | 1,006.8 ns | 983.4 ns | 189.4 ns |
| create weekday | 835.4 ns | 888.1 ns | 866.6 ns | 99.8 ns |
| add months | 1.458 us | 1.604 us | 1.583 us | 0.400 us |
| add complex | 1.458 us | 1.647 us | 1.584 us | 0.568 us |
| add weekday | 1.542 us | 1.787 us | 1.750 us | 0.557 us |
| subtract | 2.708 us | 2.975 us | 2.916 us | 0.748 us |
| multiply | 1.333 us | 1.478 us | 1.458 us | 0.513 us |
| diff dates | 2.541 us | 2.763 us | 2.667 us | 0.712 us |
| diff datetimes | 2.458 us | 2.698 us | 2.625 us | 1.151 us |
| sequential add (x12) | 17.541 us | 18.723 us | 18.250 us | 2.022 us |
| month-end overflow | 1.417 us | 1.606 us | 1.542 us | 0.533 us |

## Parser -- parse()

| Benchmark | Min | Mean | Median | StdDev |
|-----------|----:|-----:|-------:|-------:|
| simple date | 8.000 us | 8.621 us | 8.292 us | 1.944 us |
| datetime with time | 13.000 us | 13.865 us | 13.500 us | 2.026 us |
| datetime with tz | 16.750 us | 17.698 us | 17.375 us | 2.351 us |
| American format | 13.250 us | 14.222 us | 13.833 us | 2.224 us |
| European format | 8.125 us | 8.875 us | 8.583 us | 1.763 us |
| with microseconds | 14.625 us | 15.482 us | 15.208 us | 1.710 us |
| fuzzy parsing | 27.500 us | 29.403 us | 28.625 us | 2.991 us |
| relative with default | 6.083 us | 6.492 us | 6.375 us | 1.332 us |
| 10 various formats | 155.167 us | 164.489 us | 160.958 us | 13.236 us |

## Parser -- isoparse()

| Benchmark | Min | Mean | Median | StdDev |
|-----------|----:|-----:|-------:|-------:|
| date | 708.0 ns | 818.4 ns | 792.0 ns | 271.1 ns |
| datetime | 1.833 us | 2.074 us | 2.041 us | 0.592 us |
| datetime+tz | 2.833 us | 3.087 us | 3.000 us | 0.738 us |
| datetime+UTC | 2.125 us | 2.333 us | 2.291 us | 0.574 us |
| compact | 1.791 us | 1.970 us | 1.917 us | 0.469 us |
| with microseconds | 2.416 us | 2.634 us | 2.542 us | 0.716 us |
| 10 various ISO strings | 21.917 us | 23.198 us | 22.625 us | 2.558 us |

## RRule

| Benchmark | Min | Mean | Median | StdDev |
|-----------|----:|-----:|-------:|-------:|
| daily 100 | 98.208 us | 102.758 us | 100.834 us | 4.595 us |
| daily interval=3 | 105.625 us | 110.861 us | 108.625 us | 5.540 us |
| weekly 52 | 99.625 us | 105.085 us | 102.667 us | 6.075 us |
| weekly byday interval=2 | 126.750 us | 134.541 us | 131.167 us | 8.779 us |
| monthly 120 | 416.209 us | 442.554 us | 436.750 us | 20.892 us |
| monthly byday | 16.916 us | 18.236 us | 17.417 us | 57.847 us |
| monthly bymonthday | 122.875 us | 129.946 us | 126.709 us | 8.183 us |
| yearly 100 | 2,009.9 us | 2,098.0 us | 2,090.3 us | 45.3 us |
| yearly bymonth | 638.875 us | 664.231 us | 658.125 us | 17.456 us |
| hourly 1000 | 1,202.2 us | 1,263.0 us | 1,256.4 us | 37.7 us |
| rruleset union | 122.291 us | 130.011 us | 126.750 us | 8.183 us |
| rruleset exdate | 130.041 us | 137.812 us | 134.916 us | 8.035 us |
| rruleset exrule | 587.542 us | 616.209 us | 609.667 us | 20.884 us |
| rrulestr simple | 2.750 us | 3.039 us | 2.958 us | 0.952 us |
| rrulestr complex | 6.000 us | 6.552 us | 6.375 us | 1.285 us |
| rrulestr with dtstart | 14.375 us | 15.442 us | 14.958 us | 2.631 us |

## Timezone

| Benchmark | Min | Mean | Median | StdDev |
|-----------|----:|-----:|-------:|-------:|
| tzutc create | 52.5 ns | 55.7 ns | 54.6 ns | 8.6 ns |
| tzoffset create | 292.0 ns | 430.4 ns | 417.0 ns | 264.8 ns |
| tzlocal create | 375.0 ns | 479.7 ns | 458.0 ns | 412.5 ns |
| gettz UTC | 250.0 ns | 312.9 ns | 292.0 ns | 85.4 ns |
| gettz named | 208.0 ns | 325.4 ns | 292.0 ns | 652.9 ns |
| gettz offset | 18.375 us | 21.838 us | 19.125 us | 10.380 us |
| convert UTC->JST | 1.208 us | 1.400 us | 1.334 us | 0.568 us |
| convert UTC->Eastern | 1.458 us | 1.671 us | 1.625 us | 0.579 us |
| convert chain | 6.583 us | 7.030 us | 6.875 us | 1.101 us |
| localize naive | 70.8 ns | 75.0 ns | 73.3 ns | 11.6 ns |
| datetime_exists | 2.958 us | 3.233 us | 3.166 us | 0.658 us |
| datetime_ambiguous | 916.0 ns | 1,042.8 ns | 1,000.0 ns | 330.9 ns |
| resolve_imaginary | 6.383 us | 7.030 us | 6.875 us | 1.101 us |
| gettz various (x10) | 690.084 us | 737.814 us | 720.979 us | 52.398 us |

---

## How to Reproduce

```bash
# Install python-dateutil
uv pip install python-dateutil

# Build Rust extension (needed for conftest fixture parametrization)
maturin develop --release

# Run python-dateutil only benchmarks
uv run pytest benchmarks/ --benchmark-enable --benchmark-only \
    --benchmark-sort=fullname --benchmark-group-by=func \
    -k "python-dateutil"
```