//! Work-counter schema (R0 §10) and the sink abstraction that keeps
//! attribution cost out of the timing lane.
//!
//! R1 materializes only the schema/interface. Counter *behavior* for
//! H1/H2/H3/H4 must not be invented here; a mechanism reports a slot or
//! leaves it `Unknown`, and slots that cannot apply to the mechanism are
//! `NotApplicable`.
//!
//! Reporting semantics (R1-CORRECTIVE-1, MAJOR-1) — the method name states
//! the semantics, and the recording sink enforces it:
//!
//! ```text
//! add_*                  cumulative work; multiple calls ACCUMULATE
//!                        (never overwrite); add(0) is a measured zero
//! set_*                  single-valued gauges only (restart/convergence
//!                        distance); the last value wins
//! record_fallback_to_full / record_source_inspection
//!                        events; each call is one occurrence
//! ```
//!
//! Source-inspection facts are reported as raw half-open byte-range
//! EVENTS (`record_source_inspection`), never as final counters. The
//! common collector ([`CounterSink::finalize_derived`]) unions and
//! deduplicates them and derives
//! `unique_source_intervals_inspected` / `unique_source_bytes_inspected`
//! from one shared definition. Parse Amplification
//! (unique source bytes re-inspected / logical edited bytes), when it is
//! activated at a later stage, must be computed by this common layer from
//! those derived counters — never by a mechanism/horse.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::observed::Observed;

/// The R0 first-round work-counter slots, in frozen order.
///
/// `unique_source_intervals_inspected` / `unique_source_bytes_inspected`
/// are DERIVED slots: mechanisms never write them; they are computed by
/// [`CounterSink::finalize_derived`] from `record_source_inspection`
/// events. They stay `Unknown` for runs that did not complete (or did not
/// run attribution).
///
/// No `Default`: absent instrumentation must be spelled
/// [`WorkCounters::all_unknown`] or [`WorkCounters::all_not_applicable`]
/// explicitly, never implicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorkCounters {
    pub unique_source_intervals_inspected: Observed<u64>,
    pub unique_source_bytes_inspected: Observed<u64>,
    pub blocks_reparsed: Observed<u64>,
    pub nodes_rebuilt: Observed<u64>,
    pub nodes_reused: Observed<u64>,
    pub metadata_records_touched: Observed<u64>,
    pub restart_distance: Observed<u64>,
    pub convergence_distance: Observed<u64>,
    pub fallback_to_full_count: Observed<u64>,
}

impl WorkCounters {
    /// Every slot `Unknown`: the default for runs that did not collect
    /// attribution.
    pub fn all_unknown() -> Self {
        Self {
            unique_source_intervals_inspected: Observed::Unknown,
            unique_source_bytes_inspected: Observed::Unknown,
            blocks_reparsed: Observed::Unknown,
            nodes_rebuilt: Observed::Unknown,
            nodes_reused: Observed::Unknown,
            metadata_records_touched: Observed::Unknown,
            restart_distance: Observed::Unknown,
            convergence_distance: Observed::Unknown,
            fallback_to_full_count: Observed::Unknown,
        }
    }

    /// Every slot `NotApplicable`.
    pub fn all_not_applicable() -> Self {
        Self {
            unique_source_intervals_inspected: Observed::NotApplicable,
            unique_source_bytes_inspected: Observed::NotApplicable,
            blocks_reparsed: Observed::NotApplicable,
            nodes_rebuilt: Observed::NotApplicable,
            nodes_reused: Observed::NotApplicable,
            metadata_records_touched: Observed::NotApplicable,
            restart_distance: Observed::NotApplicable,
            convergence_distance: Observed::NotApplicable,
            fallback_to_full_count: Observed::NotApplicable,
        }
    }

    pub fn all_unknown_slots(&self) -> bool {
        *self == Self::all_unknown()
    }
}

/// Accumulate `n` into a cumulative slot. The first add replaces
/// `Unknown`/`NotApplicable` with `Known(n)` (doing the work declares the
/// slot applicable); further adds accumulate. Overflow marks the slot
/// `Unknown` instead of lying with a wrapped number.
fn accumulate(slot: &mut Observed<u64>, n: u64) {
    *slot = match *slot {
        Observed::Known(v) => match v.checked_add(n) {
            Some(sum) => Observed::Known(sum),
            None => Observed::Unknown,
        },
        Observed::Unknown | Observed::NotApplicable => Observed::Known(n),
    };
}

/// Cumulative/event counter slots that can be intrinsically inapplicable
/// to a mechanism (R4, H0 attribution semantics: `Known(0)`, `Unknown`
/// and `NotApplicable` are distinct — a slot with no referent for the
/// mechanism is `NotApplicable`, never a fabricated zero).
///
/// Slots NOT listed here (`blocks_reparsed`, `nodes_rebuilt`, the
/// derived unique-source slots) are meaningful for every mechanism that
/// parses and are always reported as measured values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotApplicableSlot {
    /// `nodes_reused`: the mechanism has no reuse concept at all.
    NodesReused,
    /// `metadata_records_touched`: the mechanism maintains no
    /// metadata/range records.
    MetadataRecordsTouched,
    /// `fallback_to_full_count`: the mechanism has no degraded mode from
    /// which falling back would be an event.
    FallbackToFullCount,
}

/// Receives work-counter observations from a mechanism.
///
/// Every method has a no-op default: mechanisms written against this trait
/// compile against both a recording sink ([`CounterSink`], attribution
/// lane) and a discarding sink ([`NoopWorkSink`], timing lane), so the
/// same mechanism code runs in every lane.
pub trait WorkSink {
    /// Cumulative: blocks re-entered by a local reparse. Multiple calls
    /// accumulate; they never overwrite.
    fn add_blocks_reparsed(&mut self, _n: u64) {}
    /// Cumulative: nodes rebuilt. Multiple calls accumulate.
    fn add_nodes_rebuilt(&mut self, _n: u64) {}
    /// Cumulative: nodes reused. Multiple calls accumulate.
    fn add_nodes_reused(&mut self, _n: u64) {}
    /// Cumulative: metadata records touched. Multiple calls accumulate.
    fn add_metadata_records_touched(&mut self, _n: u64) {}
    /// Cumulative: full-rebuild fallbacks. Prefer
    /// [`WorkSink::record_fallback_to_full`]; multiple calls accumulate.
    fn add_fallback_to_full(&mut self, _n: u64) {}
    /// Event: one full-rebuild fallback occurred.
    fn record_fallback_to_full(&mut self) {
        self.add_fallback_to_full(1);
    }
    /// Gauge: distance restarted from. Single-valued; the last value
    /// wins.
    fn set_restart_distance(&mut self, _v: Observed<u64>) {}
    /// Gauge: distance until convergence. Single-valued; the last value
    /// wins.
    fn set_convergence_distance(&mut self, _v: Observed<u64>) {}
    /// Declaration (R4, additive): a cumulative/event slot is
    /// intrinsically inapplicable to this mechanism. This is a statement
    /// about the mechanism, not a measurement of zero.
    fn set_slot_not_applicable(&mut self, _slot: NotApplicableSlot) {}
    /// Event: the mechanism inspected source bytes `[start_byte,
    /// end_byte)` during the CURRENT phase (`prepare_update` or
    /// `update`/`full_parse`). Empty ranges (`start >= end`) are ignored.
    /// Overlapping/duplicate reports are unioned by the common collector —
    /// they never double-count.
    fn record_source_inspection(&mut self, _start_byte: u64, _end_byte: u64) {}
}

/// Attribution-lane sink: writes observations into a [`WorkCounters`] and
/// collects source-inspection events for the common union derivation.
pub struct CounterSink<'a> {
    counters: &'a mut WorkCounters,
    inspections: Vec<(u64, u64)>,
}

impl<'a> CounterSink<'a> {
    pub fn new(counters: &'a mut WorkCounters) -> Self {
        Self {
            counters,
            inspections: Vec::new(),
        }
    }

    /// Derive the `unique_source_*` slots from the recorded inspection
    /// events (sort + merge overlapping/adjacent ranges, sum the merged
    /// lengths). The union of disjoint ranges inside `[0, u64::MAX]` can
    /// never exceed `u64::MAX` bytes, so the derivation cannot overflow.
    ///
    /// Call exactly once per case, after the mechanism finished
    /// successfully and before the counters are read or serialized. A run
    /// that did not complete must NOT call this: underived slots stay
    /// `Unknown`, honestly. Zero recorded events derive `Known(0)` — an
    /// empty event stream IS the measured zero of an event counter.
    pub fn finalize_derived(&mut self) {
        self.inspections.sort_unstable();
        let mut merged: Vec<(u64, u64)> = Vec::with_capacity(self.inspections.len());
        for &(start, end) in &self.inspections {
            match merged.last_mut() {
                // `start <= last.end` merges overlaps AND touching ranges:
                // the union of the inspected byte SET, never a double count.
                Some(last) if start <= last.1 => {
                    last.1 = last.1.max(end);
                }
                _ => merged.push((start, end)),
            }
        }
        let mut bytes: u64 = 0;
        for &(start, end) in &merged {
            bytes += end - start;
        }
        self.counters.unique_source_intervals_inspected = Observed::Known(merged.len() as u64);
        self.counters.unique_source_bytes_inspected = Observed::Known(bytes);
    }
}

impl WorkSink for CounterSink<'_> {
    fn add_blocks_reparsed(&mut self, n: u64) {
        accumulate(&mut self.counters.blocks_reparsed, n);
    }
    fn add_nodes_rebuilt(&mut self, n: u64) {
        accumulate(&mut self.counters.nodes_rebuilt, n);
    }
    fn add_nodes_reused(&mut self, n: u64) {
        accumulate(&mut self.counters.nodes_reused, n);
    }
    fn add_metadata_records_touched(&mut self, n: u64) {
        accumulate(&mut self.counters.metadata_records_touched, n);
    }
    fn add_fallback_to_full(&mut self, n: u64) {
        accumulate(&mut self.counters.fallback_to_full_count, n);
    }
    fn set_restart_distance(&mut self, v: Observed<u64>) {
        self.counters.restart_distance = v;
    }
    fn set_convergence_distance(&mut self, v: Observed<u64>) {
        self.counters.convergence_distance = v;
    }
    fn set_slot_not_applicable(&mut self, slot: NotApplicableSlot) {
        match slot {
            NotApplicableSlot::NodesReused => {
                self.counters.nodes_reused = Observed::NotApplicable;
            }
            NotApplicableSlot::MetadataRecordsTouched => {
                self.counters.metadata_records_touched = Observed::NotApplicable;
            }
            NotApplicableSlot::FallbackToFullCount => {
                self.counters.fallback_to_full_count = Observed::NotApplicable;
            }
        }
    }
    fn record_source_inspection(&mut self, start_byte: u64, end_byte: u64) {
        if end_byte > start_byte {
            self.inspections.push((start_byte, end_byte));
        }
    }
}

/// Timing-lane sink: discards everything, adding no attribution cost.
pub struct NoopWorkSink;

impl WorkSink for NoopWorkSink {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cumulative_adds_accumulate_and_never_overwrite() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.add_blocks_reparsed(2);
        sink.add_blocks_reparsed(3);
        sink.add_fallback_to_full(1);
        sink.record_fallback_to_full(); // event sugar: +1
        drop(sink);
        assert_eq!(counters.blocks_reparsed, Observed::Known(5));
        assert_eq!(counters.fallback_to_full_count, Observed::Known(2));
        assert_eq!(counters.nodes_rebuilt, Observed::Unknown);
    }

    #[test]
    fn add_zero_is_a_measured_zero_not_unknown() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.add_blocks_reparsed(0);
        drop(sink);
        assert_eq!(counters.blocks_reparsed, Observed::Known(0));
        assert_eq!(counters.nodes_rebuilt, Observed::Unknown);
    }

    #[test]
    fn gauges_are_last_value_wins() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.set_restart_distance(Observed::Known(10));
        sink.set_restart_distance(Observed::NotApplicable);
        sink.set_convergence_distance(Observed::Known(4));
        drop(sink);
        assert_eq!(counters.restart_distance, Observed::NotApplicable);
        assert_eq!(counters.convergence_distance, Observed::Known(4));
    }

    #[test]
    fn cumulative_overflow_becomes_unknown_not_wrapped() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.add_nodes_rebuilt(u64::MAX);
        sink.add_nodes_rebuilt(1);
        drop(sink);
        assert_eq!(counters.nodes_rebuilt, Observed::Unknown);
    }

    #[test]
    fn overlapping_inspections_are_unioned_not_double_counted() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.record_source_inspection(0, 10);
        sink.record_source_inspection(5, 15); // overlaps
        sink.record_source_inspection(5, 15); // duplicate
        sink.record_source_inspection(20, 25); // disjoint
        sink.record_source_inspection(30, 30); // empty: ignored
        sink.record_source_inspection(40, 39); // inverted: ignored
        sink.finalize_derived();
        assert_eq!(
            counters.unique_source_intervals_inspected,
            Observed::Known(2)
        );
        assert_eq!(counters.unique_source_bytes_inspected, Observed::Known(20));
    }

    #[test]
    fn touching_inspections_merge_into_one_interval() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.record_source_inspection(0, 5);
        sink.record_source_inspection(5, 10);
        sink.record_source_inspection(9, 12);
        sink.finalize_derived();
        assert_eq!(
            counters.unique_source_intervals_inspected,
            Observed::Known(1)
        );
        assert_eq!(counters.unique_source_bytes_inspected, Observed::Known(12));
    }

    #[test]
    fn no_inspections_derive_measured_zero_and_unfinalized_stays_unknown() {
        let mut counters = WorkCounters::all_unknown();
        {
            let mut sink = CounterSink::new(&mut counters);
            sink.add_blocks_reparsed(1);
        }
        // Without finalize the derived slots stay Unknown...
        assert_eq!(counters.unique_source_bytes_inspected, Observed::Unknown);
        // ...an empty event stream IS the measured zero.
        {
            let mut sink = CounterSink::new(&mut counters);
            sink.finalize_derived();
        }
        assert_eq!(counters.unique_source_bytes_inspected, Observed::Known(0));
        assert_eq!(
            counters.unique_source_intervals_inspected,
            Observed::Known(0)
        );
    }
    #[test]
    fn full_domain_union_is_represented_exactly() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.record_source_inspection(0, u64::MAX);
        sink.record_source_inspection(1 << 40, 1 << 41); // inside: merged away
        sink.finalize_derived();
        assert_eq!(
            counters.unique_source_intervals_inspected,
            Observed::Known(1)
        );
        assert_eq!(
            counters.unique_source_bytes_inspected,
            Observed::Known(u64::MAX)
        );
    }

    #[test]
    fn noop_sink_discards_everything() {
        let mut noop = NoopWorkSink;
        noop.add_blocks_reparsed(7);
        noop.record_fallback_to_full();
        noop.set_restart_distance(Observed::Known(1));
        noop.record_source_inspection(0, 100);
    }

    #[test]
    fn serde_shape_is_snake_case_and_strict() {
        let mut counters = WorkCounters::all_unknown();
        counters.blocks_reparsed = Observed::Known(3);
        let json = serde_json::to_value(&counters).unwrap();
        assert_eq!(json["blocks_reparsed"], 3);
        assert_eq!(json["nodes_rebuilt"], "UNKNOWN");
        let back: WorkCounters = serde_json::from_value(json).unwrap();
        assert_eq!(back, counters);
        // Lane exclusivity at the schema level: unknown keys are rejected.
        let mut bad = serde_json::to_value(WorkCounters::all_unknown()).unwrap();
        bad["prepare_ns"] = serde_json::json!(11);
        let res: Result<WorkCounters, _> = serde_json::from_value(bad);
        assert!(res.is_err(), "attribution metrics must reject timing keys");
    }
}
