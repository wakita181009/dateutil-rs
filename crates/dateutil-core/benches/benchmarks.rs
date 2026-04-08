use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use dateutil_core::common::Weekday;
use dateutil_core::easter::{easter, EasterMethod};
use dateutil_core::relativedelta::RelativeDelta;

fn bench_easter(c: &mut Criterion) {
    c.bench_function("easter_western_1000_years", |b| {
        b.iter(|| {
            for year in 1000..2000 {
                black_box(easter(black_box(year), EasterMethod::Western).unwrap());
            }
        })
    });

    c.bench_function("easter_western_single", |b| {
        b.iter(|| {
            black_box(easter(black_box(2024), EasterMethod::Western).unwrap());
        })
    });
}

fn bench_weekday(c: &mut Criterion) {
    c.bench_function("weekday_create_and_display", |b| {
        b.iter(|| {
            for i in 0..7u8 {
                let wd = Weekday::new(black_box(i), Some(2)).unwrap();
                black_box(wd.to_string());
            }
        })
    });
}

fn bench_relativedelta(c: &mut Criterion) {
    let base = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();

    c.bench_function("relativedelta_add_months", |b| {
        let delta = RelativeDelta::builder().months(3).build().unwrap();
        b.iter(|| {
            black_box(delta.add_to_naive_datetime(black_box(base)));
        })
    });

    c.bench_function("relativedelta_add_complex", |b| {
        let wd = Weekday::new(4, Some(2)).unwrap();
        let delta = RelativeDelta::builder()
            .years(1).months(2).days(3)
            .hours(4).minutes(30).seconds(15).microseconds(500_000)
            .weekday(wd)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(delta.add_to_naive_datetime(black_box(base)));
        })
    });

    c.bench_function("relativedelta_from_diff", |b| {
        let dt1 = chrono::NaiveDate::from_ymd_opt(2025, 6, 15)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap();
        let dt2 = chrono::NaiveDate::from_ymd_opt(2023, 3, 10)
            .unwrap()
            .and_hms_opt(8, 15, 0)
            .unwrap();
        b.iter(|| {
            black_box(RelativeDelta::from_diff(black_box(dt1), black_box(dt2)));
        })
    });
}

criterion_group!(benches, bench_easter, bench_weekday, bench_relativedelta);
criterion_main!(benches);
