//! RRuleSet — composite recurrence sets with heap-merge iteration.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;

use chrono::NaiveDateTime;

use super::iter::RRuleIter;
use super::{Recurrence, RRule};

// ---------------------------------------------------------------------------
// RRuleSet
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RRuleSet {
    rrules: Vec<Arc<RRule>>,
    rdates: Vec<NaiveDateTime>,
    exrules: Vec<Arc<RRule>>,
    exdates: Vec<NaiveDateTime>,
}

impl RRuleSet {
    pub fn new() -> Self {
        Self {
            rrules: Vec::new(),
            rdates: Vec::new(),
            exrules: Vec::new(),
            exdates: Vec::new(),
        }
    }

    pub fn rrule(&mut self, rule: RRule) {
        self.rrules.push(Arc::new(rule));
    }

    pub fn rdate(&mut self, dt: NaiveDateTime) {
        self.rdates.push(dt);
    }

    pub fn exrule(&mut self, rule: RRule) {
        self.exrules.push(Arc::new(rule));
    }

    pub fn exdate(&mut self, dt: NaiveDateTime) {
        self.exdates.push(dt);
    }

}

impl Recurrence for RRuleSet {
    type Iter = RRuleSetIter;

    fn iter(&self) -> RRuleSetIter {
        RRuleSetIter::new(self)
    }

    fn is_finite(&self) -> bool {
        self.rrules.iter().all(|r| r.is_finite())
    }
}

impl Default for RRuleSet {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// IterSource — concrete enum instead of Box<dyn Iterator>
// ---------------------------------------------------------------------------

enum IterSource {
    Rule(Box<RRuleIter>),
    Dates(std::vec::IntoIter<NaiveDateTime>),
}

impl IterSource {
    #[inline]
    fn next(&mut self) -> Option<NaiveDateTime> {
        match self {
            IterSource::Rule(it) => it.next(),
            IterSource::Dates(it) => it.next(),
        }
    }
}

// ---------------------------------------------------------------------------
// HeapItem for min-heap merge
// ---------------------------------------------------------------------------

struct HeapItem {
    dt: NaiveDateTime,
    source: IterSource,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.dt == other.dt
    }
}

impl Eq for HeapItem {}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed for min-heap
        other.dt.cmp(&self.dt)
    }
}

// ---------------------------------------------------------------------------
// RRuleSetIter
// ---------------------------------------------------------------------------

pub struct RRuleSetIter {
    rheap: BinaryHeap<HeapItem>,
    exheap: BinaryHeap<HeapItem>,
    exdates: Vec<NaiveDateTime>,
    exdate_cursor: usize,
    last_dt: Option<NaiveDateTime>,
}

impl RRuleSetIter {
    fn new(set: &RRuleSet) -> Self {
        let mut rheap = BinaryHeap::new();
        let mut exheap = BinaryHeap::new();

        // Add rdate source
        let mut rdates = set.rdates.clone();
        rdates.sort();
        let mut rdate_iter = rdates.into_iter();
        if let Some(dt) = rdate_iter.next() {
            rheap.push(HeapItem {
                dt,
                source: IterSource::Dates(rdate_iter),
            });
        }

        // Add rrule sources
        for rule in &set.rrules {
            let mut rule_iter = RRuleIter::new(Arc::clone(rule));
            if let Some(dt) = rule_iter.next() {
                rheap.push(HeapItem {
                    dt,
                    source: IterSource::Rule(Box::new(rule_iter)),
                });
            }
        }

        // Add exrule sources
        for rule in &set.exrules {
            let mut rule_iter = RRuleIter::new(Arc::clone(rule));
            if let Some(dt) = rule_iter.next() {
                exheap.push(HeapItem {
                    dt,
                    source: IterSource::Rule(Box::new(rule_iter)),
                });
            }
        }

        // Sort exdates for cursor-based exclusion
        let mut exdates = set.exdates.clone();
        exdates.sort();

        RRuleSetIter {
            rheap,
            exheap,
            exdates,
            exdate_cursor: 0,
            last_dt: None,
        }
    }

    fn excluded(&mut self, dt: NaiveDateTime) -> bool {
        // Check exdates via cursor
        while self.exdate_cursor < self.exdates.len() && self.exdates[self.exdate_cursor] < dt {
            self.exdate_cursor += 1;
        }
        if self.exdate_cursor < self.exdates.len() && self.exdates[self.exdate_cursor] == dt {
            return true;
        }

        // Check exrules via heap
        while let Some(exitem) = self.exheap.peek() {
            if exitem.dt < dt {
                let mut exitem = self.exheap.pop().unwrap();
                if let Some(next_dt) = exitem.source.next() {
                    exitem.dt = next_dt;
                    self.exheap.push(exitem);
                }
            } else {
                break;
            }
        }

        if self.exheap.peek().is_some_and(|ex| ex.dt == dt) {
            // Advance all matching exclusion sources
            while self.exheap.peek().is_some_and(|ex| ex.dt == dt) {
                let mut exitem = self.exheap.pop().unwrap();
                if let Some(next_dt) = exitem.source.next() {
                    exitem.dt = next_dt;
                    self.exheap.push(exitem);
                }
            }
            return true;
        }

        false
    }
}

impl Iterator for RRuleSetIter {
    type Item = NaiveDateTime;

    fn next(&mut self) -> Option<NaiveDateTime> {
        while let Some(mut ritem) = self.rheap.pop() {
            let dt = ritem.dt;

            // Advance this source
            if let Some(next_dt) = ritem.source.next() {
                ritem.dt = next_dt;
                self.rheap.push(ritem);
            }

            // Skip duplicates
            if self.last_dt == Some(dt) {
                continue;
            }

            // Check exclusions
            if self.excluded(dt) {
                self.last_dt = Some(dt);
                continue;
            }

            self.last_dt = Some(dt);
            return Some(dt);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use chrono::NaiveDateTime;

    use crate::rrule::{Frequency, Recurrence, RRuleBuilder};
    use super::RRuleSet;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    // -----------------------------------------------------------------
    // Multiple rrules merge
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_multiple_rrules() {
        let mut rset = RRuleSet::new();
        let rule1 = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        let rule2 = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 2, 12, 0, 0))
            .count(3)
            .build()
            .unwrap();
        rset.rrule(rule1);
        rset.rrule(rule2);
        let results = rset.all();
        // Merged in order, no duplicates
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 2, 0, 0, 0),
                dt(2020, 1, 2, 12, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 3, 12, 0, 0),
                dt(2020, 1, 4, 12, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------
    // Deduplication of overlapping dates
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_dedup() {
        let mut rset = RRuleSet::new();
        let rule1 = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        let rule2 = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        rset.rrule(rule1);
        rset.rrule(rule2);
        let results = rset.all();
        // Same dates from both rules — should be deduplicated
        assert_eq!(results.len(), 3);
    }

    // -----------------------------------------------------------------
    // rdate + exdate overlap (same date in both)
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_rdate_and_exdate_same() {
        let mut rset = RRuleSet::new();
        rset.rdate(dt(2020, 1, 1, 0, 0, 0));
        rset.rdate(dt(2020, 1, 2, 0, 0, 0));
        rset.exdate(dt(2020, 1, 1, 0, 0, 0));
        let results = rset.all();
        assert_eq!(results, vec![dt(2020, 1, 2, 0, 0, 0)]);
    }

    // -----------------------------------------------------------------
    // Multiple exdates
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_multiple_exdates() {
        let mut rset = RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        rset.rrule(rule);
        rset.exdate(dt(2020, 1, 2, 0, 0, 0));
        rset.exdate(dt(2020, 1, 4, 0, 0, 0));
        let results = rset.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------
    // exdate + exrule combined
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_exdate_and_exrule_combined() {
        let mut rset = RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(7)
            .build()
            .unwrap();
        rset.rrule(rule);
        // Exclude odd days via exrule
        let exrule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .interval(2)
            .count(4)
            .build()
            .unwrap();
        rset.exrule(exrule); // excludes Jan 1, 3, 5, 7
        rset.exdate(dt(2020, 1, 2, 0, 0, 0)); // also exclude Jan 2
        let results = rset.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 6, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------
    // exrule removes all occurrences
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_exrule_removes_all() {
        let mut rset = RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(3)
            .build()
            .unwrap();
        rset.rrule(rule.clone());
        rset.exrule(rule);
        let results = rset.all();
        assert!(results.is_empty());
    }

    // -----------------------------------------------------------------
    // before / after / between on RRuleSet
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_before() {
        let mut rset = RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        rset.rrule(rule);
        assert_eq!(
            rset.before(dt(2020, 1, 5, 0, 0, 0), false),
            Some(dt(2020, 1, 4, 0, 0, 0))
        );
        assert_eq!(
            rset.before(dt(2020, 1, 5, 0, 0, 0), true),
            Some(dt(2020, 1, 5, 0, 0, 0))
        );
    }

    #[test]
    fn test_rruleset_after() {
        let mut rset = RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        rset.rrule(rule);
        assert_eq!(
            rset.after(dt(2020, 1, 5, 0, 0, 0), false),
            Some(dt(2020, 1, 6, 0, 0, 0))
        );
        assert_eq!(
            rset.after(dt(2020, 1, 5, 0, 0, 0), true),
            Some(dt(2020, 1, 5, 0, 0, 0))
        );
    }

    #[test]
    fn test_rruleset_between() {
        let mut rset = RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        rset.rrule(rule);
        let results = rset.between(
            dt(2020, 1, 3, 0, 0, 0),
            dt(2020, 1, 6, 0, 0, 0),
            true,
        );
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
                dt(2020, 1, 6, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn test_rruleset_between_exclusive() {
        let mut rset = RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(10)
            .build()
            .unwrap();
        rset.rrule(rule);
        let results = rset.between(
            dt(2020, 1, 3, 0, 0, 0),
            dt(2020, 1, 6, 0, 0, 0),
            false,
        );
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 4, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------
    // is_finite
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_is_finite() {
        let mut rset = RRuleSet::new();
        let finite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        rset.rrule(finite);
        assert!(rset.is_finite());
    }

    #[test]
    fn test_rruleset_is_infinite() {
        let mut rset = RRuleSet::new();
        let infinite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .build()
            .unwrap();
        rset.rrule(infinite);
        assert!(!rset.is_finite());
    }

    #[test]
    fn test_rruleset_mixed_finite_infinite() {
        let mut rset = RRuleSet::new();
        let finite = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        let infinite = RRuleBuilder::new(Frequency::Yearly)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .build()
            .unwrap();
        rset.rrule(finite);
        rset.rrule(infinite);
        assert!(!rset.is_finite());
    }

    // -----------------------------------------------------------------
    // Empty set
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_empty() {
        let rset = RRuleSet::new();
        assert!(rset.is_finite());
        let results = rset.all();
        assert!(results.is_empty());
    }

    // -----------------------------------------------------------------
    // rdates only (no rrules)
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_rdates_only() {
        let mut rset = RRuleSet::new();
        rset.rdate(dt(2020, 3, 15, 0, 0, 0));
        rset.rdate(dt(2020, 1, 1, 0, 0, 0));
        rset.rdate(dt(2020, 6, 30, 0, 0, 0));
        let results = rset.all();
        // Should be sorted
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 3, 15, 0, 0, 0),
                dt(2020, 6, 30, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------
    // exdates in reverse order
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_exdates_reverse_order() {
        let mut rset = RRuleSet::new();
        let rule = RRuleBuilder::new(Frequency::Daily)
            .dtstart(dt(2020, 1, 1, 0, 0, 0))
            .count(5)
            .build()
            .unwrap();
        rset.rrule(rule);
        // Add exdates in reverse order
        rset.exdate(dt(2020, 1, 4, 0, 0, 0));
        rset.exdate(dt(2020, 1, 2, 0, 0, 0));
        let results = rset.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 1, 0, 0, 0),
                dt(2020, 1, 3, 0, 0, 0),
                dt(2020, 1, 5, 0, 0, 0),
            ]
        );
    }

    // -----------------------------------------------------------------
    // rrule + rdate interleaved
    // -----------------------------------------------------------------

    #[test]
    fn test_rruleset_interleaved() {
        let mut rset = RRuleSet::new();
        // Weekly on Mondays
        let rule = RRuleBuilder::new(Frequency::Weekly)
            .dtstart(dt(2020, 1, 6, 9, 0, 0)) // Monday
            .count(3)
            .build()
            .unwrap();
        rset.rrule(rule);
        // Add rdates on Wednesdays
        rset.rdate(dt(2020, 1, 8, 9, 0, 0));
        rset.rdate(dt(2020, 1, 15, 9, 0, 0));
        let results = rset.all();
        assert_eq!(
            results,
            vec![
                dt(2020, 1, 6, 9, 0, 0),  // Mon (rule)
                dt(2020, 1, 8, 9, 0, 0),  // Wed (rdate)
                dt(2020, 1, 13, 9, 0, 0), // Mon (rule)
                dt(2020, 1, 15, 9, 0, 0), // Wed (rdate)
                dt(2020, 1, 20, 9, 0, 0), // Mon (rule)
            ]
        );
    }
}
