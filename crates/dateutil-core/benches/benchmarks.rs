use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use dateutil_core::common::Weekday;
use dateutil_core::easter::{easter, EasterMethod};
use dateutil_core::parser;
use dateutil_core::parser::tokenizer;
use dateutil_core::relativedelta::RelativeDelta;

fn bench_tokenizer(c: &mut Criterion) {
    c.bench_function("tokenize_simple_date", |b| {
        b.iter(|| {
            black_box(tokenizer::tokenize(black_box("2024-01-15")));
        })
    });

    c.bench_function("tokenize_datetime", |b| {
        b.iter(|| {
            black_box(tokenizer::tokenize(black_box("2024-01-15 10:30:45")));
        })
    });

    c.bench_function("tokenize_complex", |b| {
        b.iter(|| {
            black_box(tokenizer::tokenize(black_box(
                "Monday, January 15, 2024 3:30:45.123456 PM UTC+05:30",
            )));
        })
    });

    c.bench_function("tokenize_tz_offset", |b| {
        b.iter(|| {
            black_box(tokenizer::tokenize(black_box("2024-01-15T10:30:45+05:30")));
        })
    });
}

fn bench_parser(c: &mut Criterion) {
    c.bench_function("parse_iso_date", |b| {
        b.iter(|| {
            black_box(parser::parse(black_box("2024-01-15"), false, false).unwrap());
        })
    });

    c.bench_function("parse_datetime", |b| {
        b.iter(|| {
            black_box(parser::parse(black_box("2024-01-15 10:30:45"), false, false).unwrap());
        })
    });

    c.bench_function("parse_month_name", |b| {
        b.iter(|| {
            black_box(parser::parse(black_box("January 15, 2024"), false, false).unwrap());
        })
    });

    c.bench_function("parse_complex", |b| {
        b.iter(|| {
            black_box(
                parser::parse(black_box("Monday, January 15, 2024 3:30:45.123456 PM UTC"), false, false)
                    .unwrap(),
            );
        })
    });

    c.bench_function("parse_worst_case", |b| {
        b.iter(|| {
            black_box(
                parser::parse_to_result(
                    black_box("Monday, January 15, 2024 3:30:45.123456 PM EST -05:00"),
                    false, false,
                ).unwrap(),
            );
        })
    });

    c.bench_function("parse_tz_positive_offset", |b| {
        b.iter(|| {
            black_box(
                parser::parse_to_result(black_box("2024-01-15 10:30:45+05:30"), false, false)
                    .unwrap(),
            );
        })
    });

    c.bench_function("parse_tz_negative_offset", |b| {
        b.iter(|| {
            black_box(
                parser::parse_to_result(black_box("2024-01-15 10:30:45-0800"), false, false)
                    .unwrap(),
            );
        })
    });

    c.bench_function("parse_ampm", |b| {
        b.iter(|| {
            black_box(parser::parse(black_box("January 15, 2024 3:30 PM"), false, false).unwrap());
        })
    });

    c.bench_function("isoparse_full", |b| {
        b.iter(|| {
            black_box(parser::isoparse(black_box("2024-01-15T10:30:45.123456")).unwrap());
        })
    });

    c.bench_function("isoparse_compact", |b| {
        b.iter(|| {
            black_box(parser::isoparse(black_box("20240115T103045")).unwrap());
        })
    });

    c.bench_function("isoparse_date_only", |b| {
        b.iter(|| {
            black_box(parser::isoparse(black_box("2024-01-15")).unwrap());
        })
    });
}

fn bench_parser_throughput(c: &mut Criterion) {
    let inputs = [
        "2024-01-15",
        "2024-01-15 10:30:45",
        "January 15, 2024",
        "Monday, January 15, 2024 3:30 PM",
        "15 Jan 2024 10:30:45 UTC",
        "01/15/2024",
        "2024-01-15T10:30:45+05:30",
        "March 3, 2025 2:15:30.500 PM",
    ];

    c.bench_function("parse_throughput_8_inputs", |b| {
        b.iter(|| {
            for input in &inputs {
                black_box(parser::parse(black_box(input), false, false).unwrap());
            }
        })
    });

    let iso_inputs = [
        "2024-01-15",
        "2024-01-15T10:30:45",
        "2024-01-15T10:30:45.123456",
        "20240115T103045",
        "20240115",
        "2024-01",
        "2024-01-15T10:30:45Z",
        "2024-01-15T10:30:45+05:30",
    ];

    c.bench_function("isoparse_throughput_8_inputs", |b| {
        b.iter(|| {
            for input in &iso_inputs {
                black_box(parser::isoparse(black_box(input)).unwrap());
            }
        })
    });
}

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

criterion_group!(
    benches,
    bench_tokenizer,
    bench_parser,
    bench_parser_throughput,
    bench_easter,
    bench_weekday,
    bench_relativedelta,
);
criterion_main!(benches);
