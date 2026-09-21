//! Work-counter schema (R0 §10, as corrected by MEASUREMENT-CORRECTIVE-1
//! = ATTRIBUTION-SCHEMA-v2) and the sink abstraction that keeps
//! attribution cost out of the timing lane.
//!
//! R1 materialized only the schema/interface. Counter *behavior* for
//! H1/H2/H3/H4 is not invented here; a mechanism reports a slot or
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
//! Counter authority (MEASUREMENT-CORRECTIVE-1 §14, frozen before any
//! primary performance result):
//!
//! > Attribution counters report actual cumulative mechanism work that
//! > occurred, including work later discarded by fallback/restart. They
//! > do not merely describe the final delivered tree.
//!
//! A regional parse whose result is later discarded still happened: its
//! blocks reparsed, native nodes constructed, source inspections, and
//! metadata work all stay in the cumulative counters. Work that never
//! happened is never charged.
//!
//! # Source-inspection facts and coordinate spaces
//!
//! Source-inspection facts are reported as raw half-open byte-range
//! EVENTS tagged with their SOURCE VERSION
//! (`record_source_inspection(version, start, end)`), never as final
//! counters. After a length-changing edit the OLD and POST source byte
//! offsets name DIFFERENT bytes, so the two coordinate spaces are never
//! unioned with each other (MEASUREMENT-CORRECTIVE-1 §18). The common
//! collector ([`CounterSink::finalize_derived`]) unions events
//! separately per version and derives:
//!
//! ```text
//! unique_old_source_intervals / unique_old_source_bytes
//! unique_post_source_intervals / unique_post_source_bytes
//! unique_source_intervals / unique_source_bytes
//!     the COMBINED primary unique-coverage quantity: the SUM of the
//!     two per-version unions — a byte touched in both versions counts
//!     once per version, never a cross-version offset union
//! source_bytes_inspected_total
//!     actual cumulative inspection EFFORT: every event contributes its
//!     full length, repeated scans included
//! ```
//!
//! Unique coverage answers "how much distinct source territory was
//! touched"; total inspection answers "how much source-reading work
//! actually occurred". The two are distinct quantities and neither
//! overloads the other (MEASUREMENT-CORRECTIVE-1 §19).
//!
//! Parse Amplification (R0 §10), when activated, must be computed by a
//! common analysis layer from these derived counters — its numerator is
//! the COMBINED primary quantity `unique_source_bytes` (old-union +
//! post-union bytes) over logical edited bytes; repeated effort is
//! reported separately as total-inspection amplification
//! (`source_bytes_inspected_total` / edited bytes). Never by a
//! mechanism/horse.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::observed::Observed;

/// Which source's byte coordinates an inspection event refers to
/// (MEASUREMENT-CORRECTIVE-1 §18). After a length-changing edit, OLD and
/// POST offsets identify different bytes and are never conflated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceVersion {
    /// The pre-edit (old/retained) source. Consultations of retained
    /// state against the old bytes.
    Old,
    /// The post-edit (current/live) source. Every reparse of the live
    /// document — including a FULL_PARSE of the case's source — reports
    /// `Post` by definition: the parsed bytes ARE the current source.
    Post,
}

/// The ATTRIBUTION-SCHEMA-v2 work-counter slots, in frozen order
/// (supersedes the v1 slot set; MEASUREMENT-CORRECTIVE-1 §18/§19 — the
/// v1 schema silently unioned OLD and POST source offsets and had no
/// repeated-effort quantity).
///
/// `unique_*_source_*` are DERIVED slots: mechanisms never write them;
/// they are computed by [`CounterSink::finalize_derived`] from
/// `record_source_inspection` events, per source version. They stay
/// `Unknown` for runs that did not complete (or did not run
/// attribution).
///
/// No `Default`: absent instrumentation must be spelled
/// [`WorkCounters::all_unknown`] or [`WorkCounters::all_not_applicable`]
/// explicitly, never implicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorkCounters {
    /// Derived, OLD version: distinct byte intervals/bytes of the OLD
    /// source inspected.
    pub unique_old_source_intervals: Observed<u64>,
    pub unique_old_source_bytes: Observed<u64>,
    /// Derived, POST version: distinct byte intervals/bytes of the POST
    /// source inspected.
    pub unique_post_source_intervals: Observed<u64>,
    pub unique_post_source_bytes: Observed<u64>,
    /// Derived COMBINED primary unique-coverage quantity: the SUM of the
    /// OLD-union bytes and the POST-union bytes. Never a cross-version
    /// offset union.
    pub unique_source_intervals: Observed<u64>,
    pub unique_source_bytes: Observed<u64>,
    /// Actual cumulative inspection EFFORT: the sum of every inspection
    /// event's length across both versions, repeated scans included.
    pub source_bytes_inspected_total: Observed<u64>,
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
            unique_old_source_intervals: Observed::Unknown,
            unique_old_source_bytes: Observed::Unknown,
            unique_post_source_intervals: Observed::Unknown,
            unique_post_source_bytes: Observed::Unknown,
            unique_source_intervals: Observed::Unknown,
            unique_source_bytes: Observed::Unknown,
            source_bytes_inspected_total: Observed::Unknown,
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
            unique_old_source_intervals: Observed::NotApplicable,
            unique_old_source_bytes: Observed::NotApplicable,
            unique_post_source_intervals: Observed::NotApplicable,
            unique_post_source_bytes: Observed::NotApplicable,
            unique_source_intervals: Observed::NotApplicable,
            unique_source_bytes: Observed::NotApplicable,
            source_bytes_inspected_total: Observed::NotApplicable,
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
/// to a mechanism (R1 law: `Known(0)`, `Unknown` and `NotApplicable` are
/// distinct — a slot with no referent for the mechanism is
/// `NotApplicable`, never a fabricated zero).
///
/// Slots NOT listed here (`blocks_reparsed`, `nodes_rebuilt`,
/// `nodes_reused`, the derived unique-source slots, and the
/// cumulative-inspection total) are meaningful for every mechanism that
/// parses and are always reported as measured values. `nodes_reused` was
/// briefly listed here when R4 introduced the enum; R4-H0-REFERENCE-
/// CORRECTIVE-1 §4 reclassified it: a full rebuild has the precise fact
/// `nodes_reused == 0` and must report it as a measured zero through the
/// ordinary cumulative path, not as `NotApplicable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotApplicableSlot {
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
    /// accumulate; they never overwrite. Discarded attempts accumulate
    /// too: work that happened is reported (MEASUREMENT-CORRECTIVE-1
    /// §14).
    fn add_blocks_reparsed(&mut self, _n: u64) {}
    /// Cumulative: nodes rebuilt. Multiple calls accumulate.
    fn add_nodes_rebuilt(&mut self, _n: u64) {}
    /// Cumulative: nodes reused. Multiple calls accumulate.
    fn add_nodes_reused(&mut self, _n: u64) {}
    /// Cumulative: metadata records touched. Multiple calls accumulate.
    /// One scan of a record is ONE charge: a later fallback must not
    /// charge the same scan again.
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
    /// end_byte)` OF THE NAMED SOURCE VERSION during the CURRENT phase
    /// (`prepare_update` or `update`/`full_parse`). Empty ranges
    /// (`start >= end`) are ignored. Overlapping/duplicate reports are
    /// unioned by the common collector within one version — they never
    /// double-count unique coverage; the cumulative-effort total counts
    /// every event (MEASUREMENT-CORRECTIVE-1 §18/§19).
    fn record_source_inspection(&mut self, _version: SourceVersion, _start_byte: u64, _end_byte: u64) {
    }
}

/// One recorded inspection event: `(source_version, start, end)`.
pub type InspectionEvent = (SourceVersion, u64, u64);

/// Attribution-lane sink: writes observations into a [`WorkCounters`] and
/// collects source-inspection events for the common union derivation.
pub struct CounterSink<'a> {
    counters: &'a mut WorkCounters,
    inspections: Vec<InspectionEvent>,
}

impl<'a> CounterSink<'a> {
    pub fn new(counters: &'a mut WorkCounters) -> Self {
        Self {
            counters,
            inspections: Vec::new(),
        }
    }

    /// The recorded inspection events, unmerged and in arrival order.
    /// Read-only view for attribution assertions: a mechanism-work read
    /// must be visible as an EVENT (an exact
    /// `(source_version, start, end)` triple), which the derived union
    /// counters cannot distinguish once a parser's own per-line reports
    /// cover the same bytes.
    pub fn inspections(&self) -> &[InspectionEvent] {
        &self.inspections
    }

    /// Derive the `unique_*_source_*` slots and the cumulative-inspection
    /// total from the recorded inspection events.
    ///
    /// Per version: sort + merge overlapping/adjacent ranges, sum the
    /// merged lengths (the union of disjoint ranges inside
    /// `[0, u64::MAX]` can never overflow). The combined primary
    /// quantity is the SUM of the two per-version unions — never a
    /// cross-version union. The cumulative total sums every raw event
    /// length with checked arithmetic (overflow marks it `Unknown`
    /// instead of lying).
    ///
    /// Call exactly once per case, after the mechanism finished
    /// successfully and before the counters are read or serialized. A run
    /// that did not complete must NOT call this: underived slots stay
    /// `Unknown`, honestly. Zero recorded events derive `Known(0)` — an
    /// empty event stream IS the measured zero of an event counter.
    pub fn finalize_derived(&mut self) {
        let mut total: Option<u64> = Some(0);
        let derived_per_version = |events: Vec<(u64, u64)>, total: &mut Option<u64>| -> (u64, u64) {
            let mut events = events;
            events.sort_unstable();
            let mut merged: Vec<(u64, u64)> = Vec::with_capacity(events.len());
            for &(start, end) in &events {
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
            for &(start, end) in &events {
                // Cumulative effort: every event counts, repeats included.
                if let Some(t) = *total {
                    *total = t.checked_add(end - start);
                }
            }
            (merged.len() as u64, bytes)
        };

        let mut old_events = Vec::new();
        let mut post_events = Vec::new();
        for &(version, start, end) in &self.inspections {
            match version {
                SourceVersion::Old => old_events.push((start, end)),
                SourceVersion::Post => post_events.push((start, end)),
            }
        }
        let (old_intervals, old_bytes) = derived_per_version(old_events, &mut total);
        let (post_intervals, post_bytes) = derived_per_version(post_events, &mut total);
        let c = &mut self.counters;
        c.unique_old_source_intervals = Observed::Known(old_intervals);
        c.unique_old_source_bytes = Observed::Known(old_bytes);
        c.unique_post_source_intervals = Observed::Known(post_intervals);
        c.unique_post_source_bytes = Observed::Known(post_bytes);
        // The combined primary quantity: sum of the per-version unions.
        c.unique_source_intervals = Observed::Known(old_intervals + post_intervals);
        c.unique_source_bytes = Observed::Known(old_bytes + post_bytes);
        c.source_bytes_inspected_total = match total {
            Some(t) => Observed::Known(t),
            None => Observed::Unknown, // overflow: honest unknown, never wrapped
        };
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
            NotApplicableSlot::MetadataRecordsTouched => {
                self.counters.metadata_records_touched = Observed::NotApplicable;
            }
            NotApplicableSlot::FallbackToFullCount => {
                self.counters.fallback_to_full_count = Observed::NotApplicable;
            }
        }
    }
    fn record_source_inspection(&mut self, version: SourceVersion, start_byte: u64, end_byte: u64) {
        if end_byte > start_byte {
            self.inspections.push((version, start_byte, end_byte));
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
    fn nodes_reused_zero_is_measured_and_distinct_from_not_applicable() {
        // R4-H0-REFERENCE-CORRECTIVE-1 §4: a full rebuild has the precise
        // fact nodes_reused == 0 (it intentionally reuses no old parse
        // node) and reports it through the ordinary cumulative path.
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.add_nodes_reused(0);
        drop(sink);
        assert_eq!(counters.nodes_reused, Observed::Known(0));
        assert_ne!(counters.nodes_reused, Observed::NotApplicable);
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
        sink.record_source_inspection(SourceVersion::Post, 0, 10);
        sink.record_source_inspection(SourceVersion::Post, 5, 15); // overlaps
        sink.record_source_inspection(SourceVersion::Post, 5, 15); // duplicate
        sink.record_source_inspection(SourceVersion::Post, 20, 25); // disjoint
        sink.record_source_inspection(SourceVersion::Post, 30, 30); // empty: ignored
        sink.record_source_inspection(SourceVersion::Post, 40, 39); // inverted: ignored
        sink.finalize_derived();
        assert_eq!(counters.unique_post_source_intervals, Observed::Known(2));
        assert_eq!(counters.unique_post_source_bytes, Observed::Known(20));
        // Combined = old-union (0) + post-union (20).
        assert_eq!(counters.unique_source_bytes, Observed::Known(20));
        // Cumulative effort: every non-empty, well-formed event counts,
        // duplicates included: 10 + 10 + 10 + 5 = 35.
        assert_eq!(counters.source_bytes_inspected_total, Observed::Known(35));
    }

    #[test]
    fn old_and_post_spaces_never_union_into_each_other() {
        // MEASUREMENT-CORRECTIVE-1 §18: the same offsets in the two
        // versions name different bytes after a length-changing edit;
        // the collector must keep the unions apart.
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.record_source_inspection(SourceVersion::Old, 0, 10);
        sink.record_source_inspection(SourceVersion::Post, 0, 10);
        sink.record_source_inspection(SourceVersion::Old, 5, 15); // overlaps the old one only
        sink.finalize_derived();
        assert_eq!(counters.unique_old_source_bytes, Observed::Known(15));
        assert_eq!(counters.unique_post_source_bytes, Observed::Known(10));
        // Combined = 15 + 10 (sum of unions), NOT a 15-byte union.
        assert_eq!(counters.unique_source_bytes, Observed::Known(25));
        assert_eq!(counters.unique_source_intervals, Observed::Known(2));
        assert_eq!(counters.source_bytes_inspected_total, Observed::Known(30));
    }

    #[test]
    fn cumulative_inspection_total_counts_repeats_across_versions() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.record_source_inspection(SourceVersion::Old, 0, 100);
        sink.record_source_inspection(SourceVersion::Post, 0, 100);
        sink.record_source_inspection(SourceVersion::Post, 0, 100); // rescanned
        sink.finalize_derived();
        assert_eq!(counters.unique_old_source_bytes, Observed::Known(100));
        assert_eq!(counters.unique_post_source_bytes, Observed::Known(100));
        assert_eq!(counters.unique_source_bytes, Observed::Known(200));
        // 300 bytes of source-reading work actually occurred.
        assert_eq!(counters.source_bytes_inspected_total, Observed::Known(300));
    }

    #[test]
    fn inspection_total_overflow_is_honest_unknown() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.record_source_inspection(SourceVersion::Post, 0, u64::MAX);
        sink.record_source_inspection(SourceVersion::Post, u64::MAX - 1, u64::MAX);
        sink.finalize_derived();
        assert_eq!(counters.source_bytes_inspected_total, Observed::Unknown);
        // Unique coverage is still exact.
        assert_eq!(counters.unique_source_bytes, Observed::Known(u64::MAX));
    }

    #[test]
    fn touching_inspections_merge_into_one_interval() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.record_source_inspection(SourceVersion::Post, 0, 5);
        sink.record_source_inspection(SourceVersion::Post, 5, 10);
        sink.record_source_inspection(SourceVersion::Post, 9, 12);
        sink.finalize_derived();
        assert_eq!(counters.unique_post_source_intervals, Observed::Known(1));
        assert_eq!(counters.unique_post_source_bytes, Observed::Known(12));
    }

    #[test]
    fn no_inspections_derive_measured_zero_and_unfinalized_stays_unknown() {
        let mut counters = WorkCounters::all_unknown();
        {
            let mut sink = CounterSink::new(&mut counters);
            sink.add_blocks_reparsed(1);
        }
        // Without finalize the derived slots stay Unknown...
        assert_eq!(counters.unique_source_bytes, Observed::Unknown);
        // ...an empty event stream IS the measured zero.
        {
            let mut sink = CounterSink::new(&mut counters);
            sink.finalize_derived();
        }
        assert_eq!(counters.unique_source_bytes, Observed::Known(0));
        assert_eq!(counters.unique_old_source_bytes, Observed::Known(0));
        assert_eq!(counters.unique_post_source_bytes, Observed::Known(0));
        assert_eq!(counters.source_bytes_inspected_total, Observed::Known(0));
        assert_eq!(counters.unique_source_intervals, Observed::Known(0));
    }

    #[test]
    fn full_domain_union_is_represented_exactly() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.record_source_inspection(SourceVersion::Post, 0, u64::MAX);
        sink.record_source_inspection(SourceVersion::Post, 1 << 40, 1 << 41); // inside: merged away
        sink.finalize_derived();
        assert_eq!(counters.unique_post_source_intervals, Observed::Known(1));
        assert_eq!(counters.unique_post_source_bytes, Observed::Known(u64::MAX));
    }

    #[test]
    fn noop_sink_discards_everything() {
        let mut noop = NoopWorkSink;
        noop.add_blocks_reparsed(7);
        noop.record_fallback_to_full();
        noop.set_restart_distance(Observed::Known(1));
        noop.record_source_inspection(SourceVersion::Old, 0, 100);
    }

    #[test]
    fn serde_shape_is_snake_case_and_strict() {
        let mut counters = WorkCounters::all_unknown();
        counters.blocks_reparsed = Observed::Known(3);
        let json = serde_json::to_value(&counters).unwrap();
        assert_eq!(json["blocks_reparsed"], 3);
        assert_eq!(json["nodes_rebuilt"], "UNKNOWN");
        assert_eq!(json["source_bytes_inspected_total"], "UNKNOWN");
        let back: WorkCounters = serde_json::from_value(json).unwrap();
        assert_eq!(back, counters);
        // Lane exclusivity at the schema level: unknown keys are rejected.
        let mut bad = serde_json::to_value(WorkCounters::all_unknown()).unwrap();
        bad["prepare_ns"] = serde_json::json!(11);
        let res: Result<WorkCounters, _> = serde_json::from_value(bad);
        assert!(res.is_err(), "attribution metrics must reject timing keys");
        // The v1 slot names are gone, not aliased: a v1 payload must be
        // rejected, not silently reinterpreted (schema v2).
        let v1 = serde_json::json!({
            "unique_source_intervals_inspected": 3,
            "unique_source_bytes_inspected": 30,
            "blocks_reparsed": 0,
            "nodes_rebuilt": 0,
            "nodes_reused": 0,
            "metadata_records_touched": 0,
            "restart_distance": "UNKNOWN",
            "convergence_distance": "UNKNOWN",
            "fallback_to_full_count": 0
        });
        let res: Result<WorkCounters, _> = serde_json::from_value(v1);
        assert!(res.is_err(), "v1 attribution payloads must not deserialize as v2");
    }
}
