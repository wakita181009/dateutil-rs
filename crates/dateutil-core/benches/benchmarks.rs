use chrono::NaiveDate;
use criterion::{criterion_group, criterion_main, Criterion};
use dateutil::common::Weekday;
use dateutil::easter::{easter, EasterMethod};
use dateutil::parser;
use dateutil::parser::tokenizer;
use dateutil::relativedelta::RelativeDelta;
use dateutil::rrule::parse::rrulestr;
use dateutil::rrule::set::RRuleSet;
use dateutil::rrule::{Frequency, RRuleBuilder, Recurrence};
use std::hint::black_box;

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
            black_box(parser::parse(black_box("2024-01-15"), false, false, None, None).unwrap());
        })
    });

    c.bench_function("parse_datetime", |b| {
        b.iter(|| {
            black_box(
                parser::parse(black_box("2024-01-15 10:30:45"), false, false, None, None).unwrap(),
            );
        })
    });

    c.bench_function("parse_month_name", |b| {
        b.iter(|| {
            black_box(
                parser::parse(black_box("January 15, 2024"), false, false, None, None).unwrap(),
            );
        })
    });

    c.bench_function("parse_complex", |b| {
        b.iter(|| {
            black_box(
                parser::parse(
                    black_box("Monday, January 15, 2024 3:30:45.123456 PM UTC"),
                    false,
                    false,
                    None,
                    None,
                )
                .unwrap(),
            );
        })
    });

    c.bench_function("parse_worst_case", |b| {
        b.iter(|| {
            black_box(
                parser::parse_to_result(
                    black_box("Monday, January 15, 2024 3:30:45.123456 PM EST -05:00"),
                    false,
                    false,
                    None,
                )
                .unwrap(),
            );
        })
    });

    c.bench_function("parse_tz_positive_offset", |b| {
        b.iter(|| {
            black_box(
                parser::parse_to_result(black_box("2024-01-15 10:30:45+05:30"), false, false, None)
                    .unwrap(),
            );
        })
    });

    c.bench_function("parse_tz_negative_offset", |b| {
        b.iter(|| {
            black_box(
                parser::parse_to_result(black_box("2024-01-15 10:30:45-0800"), false, false, None)
                    .unwrap(),
            );
        })
    });

    c.bench_function("parse_ampm", |b| {
        b.iter(|| {
            black_box(
                parser::parse(
                    black_box("January 15, 2024 3:30 PM"),
                    false,
                    false,
                    None,
                    None,
                )
                .unwrap(),
            );
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
                black_box(parser::parse(black_box(input), false, false, None, None).unwrap());
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
            .years(1)
            .months(2)
            .days(3)
            .hours(4)
            .minutes(30)
            .seconds(15)
            .microseconds(500_000)
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

fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(h, mi, s)
        .unwrap()
}

fn bench_rrule_iter(c: &mut Criterion) {
    c.bench_function("rrule_daily_100", |b| {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .count(100)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all().unwrap());
        })
    });

    c.bench_function("rrule_weekly_52", |b| {
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .count(52)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all().unwrap());
        })
    });

    c.bench_function("rrule_monthly_24", |b| {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 15, 9, 0, 0))
            .count(24)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all().unwrap());
        })
    });

    c.bench_function("rrule_yearly_10", |b| {
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2020, 6, 15, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all().unwrap());
        })
    });

    c.bench_function("rrule_hourly_720", |b| {
        let rule = RRuleBuilder::new(Frequency::Hourly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(720)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all().unwrap());
        })
    });

    c.bench_function("rrule_daily_byweekday_mwf_52", |b| {
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .byweekday(vec![
                Weekday::new(0, None).unwrap(), // MO
                Weekday::new(2, None).unwrap(), // WE
                Weekday::new(4, None).unwrap(), // FR
            ])
            .count(52)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all().unwrap());
        })
    });

    c.bench_function("rrule_monthly_bymonthday_15_12", |b| {
        let rule = RRuleBuilder::new(Frequency::Monthly)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .bymonthday(vec![15])
            .count(12)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all().unwrap());
        })
    });

    c.bench_function("rrule_yearly_bymonth_interval2", |b| {
        let rule = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .bymonth(vec![1, 4, 7, 10])
            .interval(2)
            .count(20)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all().unwrap());
        })
    });
}

fn bench_rrule_before_after(c: &mut Criterion) {
    let rule = RRuleBuilder::new(Frequency::Daily)
        .dtstart(dt(2020, 1, 1, 9, 0, 0))
        .count(365)
        .build()
        .unwrap();

    c.bench_function("rrule_before_mid", |b| {
        b.iter(|| {
            black_box(rule.before(black_box(dt(2020, 7, 1, 0, 0, 0)), false));
        })
    });

    c.bench_function("rrule_after_mid", |b| {
        b.iter(|| {
            black_box(rule.after(black_box(dt(2020, 7, 1, 0, 0, 0)), false));
        })
    });

    c.bench_function("rrule_between_quarter", |b| {
        b.iter(|| {
            black_box(rule.between(
                black_box(dt(2020, 4, 1, 0, 0, 0)),
                black_box(dt(2020, 6, 30, 0, 0, 0)),
                true,
            ));
        })
    });
}

fn bench_rruleset(c: &mut Criterion) {
    c.bench_function("rruleset_two_rules_merge", |b| {
        b.iter(|| {
            let mut rset = RRuleSet::new();
            let rule1 = RRuleBuilder::new(Frequency::Daily)
                .dtstart(dt(2020, 1, 1, 9, 0, 0))
                .count(30)
                .build()
                .unwrap();
            let rule2 = RRuleBuilder::new(Frequency::Daily)
                .dtstart(dt(2020, 1, 1, 14, 0, 0))
                .count(30)
                .build()
                .unwrap();
            rset.rrule(rule1);
            rset.rrule(rule2);
            black_box(rset.all().unwrap());
        })
    });

    c.bench_function("rruleset_with_exdates", |b| {
        b.iter(|| {
            let mut rset = RRuleSet::new();
            let rule = RRuleBuilder::new(Frequency::Daily)
                .dtstart(dt(2020, 1, 1, 9, 0, 0))
                .count(60)
                .build()
                .unwrap();
            rset.rrule(rule);
            // Exclude weekends (approx)
            for i in 0..9 {
                rset.exdate(dt(2020, 1, 4 + i * 7, 9, 0, 0));
                rset.exdate(dt(2020, 1, 5 + i * 7, 9, 0, 0));
            }
            black_box(rset.all().unwrap());
        })
    });

    c.bench_function("rruleset_with_exrule", |b| {
        b.iter(|| {
            let mut rset = RRuleSet::new();
            let rule = RRuleBuilder::new(Frequency::Daily)
                .dtstart(dt(2020, 1, 1, 9, 0, 0))
                .count(60)
                .build()
                .unwrap();
            rset.rrule(rule);
            let exrule = RRuleBuilder::new(Frequency::Daily)
                .dtstart(dt(2020, 1, 1, 9, 0, 0))
                .interval(7)
                .count(9)
                .build()
                .unwrap();
            rset.exrule(exrule);
            black_box(rset.all().unwrap());
        })
    });

    c.bench_function("rruleset_rdates_only_50", |b| {
        b.iter(|| {
            let mut rset = RRuleSet::new();
            for i in 0..50 {
                rset.rdate(dt(2020, 1, 1, 9, 0, 0) + chrono::Duration::days(i * 3));
            }
            black_box(rset.all().unwrap());
        })
    });
}

fn bench_rrulestr(c: &mut Criterion) {
    c.bench_function("rrulestr_simple_daily", |b| {
        b.iter(|| {
            black_box(
                rrulestr(
                    black_box("DTSTART:20200101T090000\nRRULE:FREQ=DAILY;COUNT=30"),
                    None,
                    false,
                    false,
                    false,
                )
                .unwrap(),
            );
        })
    });

    c.bench_function("rrulestr_weekly_byday", |b| {
        b.iter(|| {
            black_box(
                rrulestr(
                    black_box("DTSTART:20200101T090000\nRRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;COUNT=52"),
                    None,
                    false,
                    false,
                    false,
                )
                .unwrap(),
            );
        })
    });

    c.bench_function("rrulestr_monthly_complex", |b| {
        b.iter(|| {
            black_box(
                rrulestr(
                    black_box("DTSTART:20200101T090000\nRRULE:FREQ=MONTHLY;BYMONTHDAY=1,15;BYHOUR=9,17;COUNT=24"),
                    None, false, false, false,
                ).unwrap(),
            );
        })
    });

    c.bench_function("rrulestr_with_exdate", |b| {
        b.iter(|| {
            black_box(
                rrulestr(
                    black_box("DTSTART:20200101T090000\nRRULE:FREQ=DAILY;COUNT=10\nEXDATE:20200103T090000,20200107T090000"),
                    None, true, false, false,
                ).unwrap(),
            );
        })
    });

    c.bench_function("rrulestr_parse_and_collect", |b| {
        b.iter(|| {
            let result = rrulestr(
                black_box("DTSTART:20200101T090000\nRRULE:FREQ=DAILY;COUNT=100"),
                None,
                false,
                false,
                false,
            )
            .unwrap();
            black_box(result.all().unwrap());
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
    bench_rrule_iter,
    bench_rrule_before_after,
    bench_rruleset,
    bench_rrulestr,
);
criterion_main!(benches);
