//! Work-counter schema (R0 §10) and the sink abstraction that keeps
//! attribution cost out of the timing lane.
//!
//! R1 materializes only the schema/interface. Counter *behavior* for
//! H1/H2/H3/H4 must not be invented here; a mechanism reports a slot or
//! leaves it `Unknown`, and slots that cannot apply to the mechanism are
//! `NotApplicable`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::observed::Observed;

/// The R0 first-round work-counter slots, in frozen order.
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

/// Receives work-counter observations from a mechanism.
///
/// Every method has a no-op default: mechanisms written against this trait
/// compile against both a recording sink ([`CounterSink`], attribution
/// lane) and a discarding sink ([`NoopWorkSink`], timing lane), so the
/// same mechanism code runs in every lane.
pub trait WorkSink {
    fn set_unique_source_intervals_inspected(&mut self, _v: Observed<u64>) {}
    fn set_unique_source_bytes_inspected(&mut self, _v: Observed<u64>) {}
    fn set_blocks_reparsed(&mut self, _v: Observed<u64>) {}
    fn set_nodes_rebuilt(&mut self, _v: Observed<u64>) {}
    fn set_nodes_reused(&mut self, _v: Observed<u64>) {}
    fn set_metadata_records_touched(&mut self, _v: Observed<u64>) {}
    fn set_restart_distance(&mut self, _v: Observed<u64>) {}
    fn set_convergence_distance(&mut self, _v: Observed<u64>) {}
    fn set_fallback_to_full_count(&mut self, _v: Observed<u64>) {}
}

/// Attribution-lane sink: writes observations into a [`WorkCounters`].
pub struct CounterSink<'a> {
    counters: &'a mut WorkCounters,
}

impl<'a> CounterSink<'a> {
    pub fn new(counters: &'a mut WorkCounters) -> Self {
        Self { counters }
    }
}

impl WorkSink for CounterSink<'_> {
    fn set_unique_source_intervals_inspected(&mut self, v: Observed<u64>) {
        self.counters.unique_source_intervals_inspected = v;
    }
    fn set_unique_source_bytes_inspected(&mut self, v: Observed<u64>) {
        self.counters.unique_source_bytes_inspected = v;
    }
    fn set_blocks_reparsed(&mut self, v: Observed<u64>) {
        self.counters.blocks_reparsed = v;
    }
    fn set_nodes_rebuilt(&mut self, v: Observed<u64>) {
        self.counters.nodes_rebuilt = v;
    }
    fn set_nodes_reused(&mut self, v: Observed<u64>) {
        self.counters.nodes_reused = v;
    }
    fn set_metadata_records_touched(&mut self, v: Observed<u64>) {
        self.counters.metadata_records_touched = v;
    }
    fn set_restart_distance(&mut self, v: Observed<u64>) {
        self.counters.restart_distance = v;
    }
    fn set_convergence_distance(&mut self, v: Observed<u64>) {
        self.counters.convergence_distance = v;
    }
    fn set_fallback_to_full_count(&mut self, v: Observed<u64>) {
        self.counters.fallback_to_full_count = v;
    }
}

/// Timing-lane sink: discards everything, adding no attribution cost.
pub struct NoopWorkSink;

impl WorkSink for NoopWorkSink {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_sink_records_and_noop_discards() {
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        sink.set_blocks_reparsed(Observed::Known(0));
        sink.set_restart_distance(Observed::NotApplicable);
        assert_eq!(counters.blocks_reparsed, Observed::Known(0));
        assert_eq!(counters.restart_distance, Observed::NotApplicable);
        assert_eq!(counters.unique_source_bytes_inspected, Observed::Unknown);

        let mut noop = NoopWorkSink;
        noop.set_blocks_reparsed(Observed::Known(7)); // compiles, discards
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
