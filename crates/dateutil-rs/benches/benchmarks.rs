use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use chrono::NaiveDate;
use dateutil_rs::common::Weekday;
use dateutil_rs::easter::{easter, EASTER_WESTERN};
use dateutil_rs::relativedelta::RelativeDelta;
use dateutil_rs::rrule::{self, RRuleBuilder, RRuleSet, rrulestr};

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

fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(h, mi, s)
        .unwrap()
}

fn bench_rrule_iter(c: &mut Criterion) {
    c.bench_function("v0_rrule_daily_100", |b| {
        let rule = RRuleBuilder::new(rrule::DAILY)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .count(100)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all());
        })
    });

    c.bench_function("v0_rrule_weekly_52", |b| {
        let rule = RRuleBuilder::new(rrule::WEEKLY)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .count(52)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all());
        })
    });

    c.bench_function("v0_rrule_monthly_24", |b| {
        let rule = RRuleBuilder::new(rrule::MONTHLY)
            .dtstart(dt(2020, 1, 15, 9, 0, 0))
            .count(24)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all());
        })
    });

    c.bench_function("v0_rrule_yearly_10", |b| {
        let rule = RRuleBuilder::new(rrule::YEARLY)
            .dtstart(dt(2020, 6, 15, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all());
        })
    });

    c.bench_function("v0_rrule_hourly_720", |b| {
        let rule = RRuleBuilder::new(rrule::HOURLY)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(720)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all());
        })
    });

    c.bench_function("v0_rrule_daily_byweekday_mwf_52", |b| {
        let rule = RRuleBuilder::new(rrule::DAILY)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .byweekday(vec![
                Weekday::new(0, None), // MO
                Weekday::new(2, None), // WE
                Weekday::new(4, None), // FR
            ])
            .count(52)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all());
        })
    });

    c.bench_function("v0_rrule_monthly_bymonthday_15_12", |b| {
        let rule = RRuleBuilder::new(rrule::MONTHLY)
            .dtstart(dt(2020, 1, 1, 9, 0, 0))
            .bymonthday(vec![15])
            .count(12)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all());
        })
    });

    c.bench_function("v0_rrule_yearly_bymonth_interval2", |b| {
        let rule = RRuleBuilder::new(rrule::YEARLY)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .bymonth(vec![1, 4, 7, 10])
            .interval(2)
            .count(20)
            .build()
            .unwrap();
        b.iter(|| {
            black_box(rule.all());
        })
    });
}

fn bench_rrule_before_after(c: &mut Criterion) {
    let rule = RRuleBuilder::new(rrule::DAILY)
        .dtstart(dt(2020, 1, 1, 9, 0, 0))
        .count(365)
        .build()
        .unwrap();

    c.bench_function("v0_rrule_before_mid", |b| {
        b.iter(|| {
            black_box(rule.before(black_box(dt(2020, 7, 1, 0, 0, 0)), false));
        })
    });

    c.bench_function("v0_rrule_after_mid", |b| {
        b.iter(|| {
            black_box(rule.after(black_box(dt(2020, 7, 1, 0, 0, 0)), false));
        })
    });

    c.bench_function("v0_rrule_between_quarter", |b| {
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
    c.bench_function("v0_rruleset_two_rules_merge", |b| {
        b.iter(|| {
            let mut rset = RRuleSet::new();
            let rule1 = RRuleBuilder::new(rrule::DAILY)
                .dtstart(dt(2020, 1, 1, 9, 0, 0))
                .count(30)
                .build()
                .unwrap();
            let rule2 = RRuleBuilder::new(rrule::DAILY)
                .dtstart(dt(2020, 1, 1, 14, 0, 0))
                .count(30)
                .build()
                .unwrap();
            rset.rrule(rule1);
            rset.rrule(rule2);
            black_box(rset.all());
        })
    });

    c.bench_function("v0_rruleset_with_exdates", |b| {
        b.iter(|| {
            let mut rset = RRuleSet::new();
            let rule = RRuleBuilder::new(rrule::DAILY)
                .dtstart(dt(2020, 1, 1, 9, 0, 0))
                .count(60)
                .build()
                .unwrap();
            rset.rrule(rule);
            for i in 0..9 {
                rset.exdate(dt(2020, 1, 4 + i * 7, 9, 0, 0));
                rset.exdate(dt(2020, 1, 5 + i * 7, 9, 0, 0));
            }
            black_box(rset.all());
        })
    });

    c.bench_function("v0_rruleset_with_exrule", |b| {
        b.iter(|| {
            let mut rset = RRuleSet::new();
            let rule = RRuleBuilder::new(rrule::DAILY)
                .dtstart(dt(2020, 1, 1, 9, 0, 0))
                .count(60)
                .build()
                .unwrap();
            rset.rrule(rule);
            let exrule = RRuleBuilder::new(rrule::DAILY)
                .dtstart(dt(2020, 1, 1, 9, 0, 0))
                .interval(7)
                .count(9)
                .build()
                .unwrap();
            rset.exrule(exrule);
            black_box(rset.all());
        })
    });

    c.bench_function("v0_rruleset_rdates_only_50", |b| {
        b.iter(|| {
            let mut rset = RRuleSet::new();
            for i in 0..50 {
                rset.rdate(dt(2020, 1, 1, 9, 0, 0) + chrono::Duration::days(i * 3));
            }
            black_box(rset.all());
        })
    });
}

fn bench_rrulestr(c: &mut Criterion) {
    c.bench_function("v0_rrulestr_simple_daily", |b| {
        b.iter(|| {
            black_box(
                rrulestr(
                    black_box("DTSTART:20200101T090000\nRRULE:FREQ=DAILY;COUNT=30"),
                    None, false, false, false,
                ).unwrap(),
            );
        })
    });

    c.bench_function("v0_rrulestr_weekly_byday", |b| {
        b.iter(|| {
            black_box(
                rrulestr(
                    black_box("DTSTART:20200101T090000\nRRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;COUNT=52"),
                    None, false, false, false,
                ).unwrap(),
            );
        })
    });

    c.bench_function("v0_rrulestr_monthly_complex", |b| {
        b.iter(|| {
            black_box(
                rrulestr(
                    black_box("DTSTART:20200101T090000\nRRULE:FREQ=MONTHLY;BYMONTHDAY=1,15;BYHOUR=9,17;COUNT=24"),
                    None, false, false, false,
                ).unwrap(),
            );
        })
    });

    c.bench_function("v0_rrulestr_with_exdate", |b| {
        b.iter(|| {
            black_box(
                rrulestr(
                    black_box("DTSTART:20200101T090000\nRRULE:FREQ=DAILY;COUNT=10\nEXDATE:20200103T090000,20200107T090000"),
                    None, true, false, false,
                ).unwrap(),
            );
        })
    });

    c.bench_function("v0_rrulestr_parse_and_collect", |b| {
        b.iter(|| {
            let result = rrulestr(
                black_box("DTSTART:20200101T090000\nRRULE:FREQ=DAILY;COUNT=100"),
                None, false, false, false,
            ).unwrap();
            black_box(result.all());
        })
    });
}

criterion_group!(
    benches,
    bench_easter,
    bench_weekday,
    bench_relativedelta,
    bench_rrule_iter,
    bench_rrule_before_after,
    bench_rruleset,
    bench_rrulestr,
);
criterion_main!(benches);
