use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use dateutil_rs::common::Weekday;
use dateutil_rs::easter::{easter, EASTER_WESTERN};
use dateutil_rs::relativedelta::RelativeDelta;

fn bench_easter(c: &mut Criterion) {
    c.bench_function("v0_easter_western_1000_years", |b| {
        b.iter(|| {
            for year in 1000..2000 {
                black_box(easter(black_box(year), EASTER_WESTERN).unwrap());
            }
        })
    });

    c.bench_function("v0_easter_western_single", |b| {
        b.iter(|| {
            black_box(easter(black_box(2024), EASTER_WESTERN).unwrap());
        })
    });
}

fn bench_weekday(c: &mut Criterion) {
    c.bench_function("v0_weekday_create_and_display", |b| {
        b.iter(|| {
            for i in 0..7u8 {
                let wd = Weekday::new(black_box(i), Some(2));
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

    c.bench_function("v0_relativedelta_add_months", |b| {
        let delta = RelativeDelta::new(
            0, 3, 0.0, 0, 0.0, 0.0, 0.0, 0.0, None, None, None, None, None, None, None, None,
            None, None,
        )
        .unwrap();
        b.iter(|| {
            black_box(delta.add_to_naive_datetime(black_box(base)));
        })
    });

    c.bench_function("v0_relativedelta_add_complex", |b| {
        let wd = Weekday::new(4, Some(2)); // 2nd Friday
        let delta = RelativeDelta::new(
            1, 2, 3.0, 0, 4.0, 30.0, 15.0, 500_000.0, None, None, None, Some(wd), None, None,
            None, None, None, None,
        )
        .unwrap();
        b.iter(|| {
            black_box(delta.add_to_naive_datetime(black_box(base)));
        })
    });

    c.bench_function("v0_relativedelta_from_diff", |b| {
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
