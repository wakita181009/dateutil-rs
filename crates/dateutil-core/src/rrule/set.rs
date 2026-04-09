//! RRuleSet — composite recurrence sets with heap-merge iteration.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;

use chrono::NaiveDateTime;

use super::iter::RRuleIter;
use super::RRule;

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

    pub fn is_finite(&self) -> bool {
        self.rrules.iter().all(|r| r.is_finite())
    }

    /// Collect all occurrences.
    ///
    /// # Panics
    ///
    /// Panics if any component rrule is not finite (i.e., neither `count` nor `until` is set).
    pub fn all(&self) -> Vec<NaiveDateTime> {
        assert!(
            self.is_finite(),
            "all() called on infinite RRuleSet (all rrules must have count or until)"
        );
        self.iter().collect()
    }

    pub fn iter(&self) -> RRuleSetIter {
        RRuleSetIter::new(self)
    }

    pub fn before(&self, dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
        let mut last = None;
        for i in self.iter() {
            if (inc && i > dt) || (!inc && i >= dt) {
                break;
            }
            last = Some(i);
        }
        last
    }

    pub fn after(&self, dt: NaiveDateTime, inc: bool) -> Option<NaiveDateTime> {
        self.iter().find(|&i| if inc { i >= dt } else { i > dt })
    }

    pub fn between(
        &self,
        after: NaiveDateTime,
        before: NaiveDateTime,
        inc: bool,
    ) -> Vec<NaiveDateTime> {
        let mut result = Vec::new();
        for i in self.iter() {
            let past_end = if inc { i > before } else { i >= before };
            if past_end {
                break;
            }
            let in_range = if inc { i >= after } else { i > after };
            if in_range {
                result.push(i);
            }
        }
        result
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

    fn is_excluded(&mut self, dt: NaiveDateTime) -> bool {
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
            if self.is_excluded(dt) {
                self.last_dt = Some(dt);
                continue;
            }

            self.last_dt = Some(dt);
            return Some(dt);
        }
        None
    }
}
