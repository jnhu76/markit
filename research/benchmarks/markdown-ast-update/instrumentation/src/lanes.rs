//! Measurement lane separation (R1 task §8).
//!
//! Three structurally distinct lanes; never one run with everything
//! enabled presented as headline timing:
//!
//! ```text
//! T-LANE = timing            real timer, NoopWorkSink, no memory collector
//! M-LANE = memory/allocation memory interface, no headline TimingRecord
//! A-LANE = attribution       work counters, no headline TimingRecord
//! ```

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use markit_mdbench_common::Observed;
use markit_mdbench_common::WorkCounters;

use crate::timer::TimingRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    Timing,
    Memory,
    Attribution,
}

impl Lane {
    pub fn name(self) -> &'static str {
        match self {
            Lane::Timing => crate::TIMING_LANE_NAME,
            Lane::Memory => crate::MEMORY_LANE_NAME,
            Lane::Attribution => crate::ATTRIBUTION_LANE_NAME,
        }
    }
}

/// M-LANE record. R1 materializes the interface/schema only — there is no
/// production allocation instrumentor yet, so reports may be `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryRecord {
    pub allocated_bytes: Observed<u64>,
    pub allocation_count: Observed<u64>,
    pub peak_retained_bytes: Observed<u64>,
}

impl MemoryRecord {
    /// Placeholder for lanes run without an allocation instrumentor.
    pub fn unavailable() -> Self {
        Self {
            allocated_bytes: Observed::Unknown,
            allocation_count: Observed::Unknown,
            peak_retained_bytes: Observed::Unknown,
        }
    }
}

/// Produces a memory report for one case run. R1 ships only
/// [`NoMemoryReporter`]; a real allocator-backed reporter is a later
/// infrastructure addition and must never run inside the timing lane.
pub trait MemoryReporter {
    fn report(&self) -> MemoryRecord;
}

/// Interface-only reporter: every slot `Unknown`, honestly.
pub struct NoMemoryReporter;

impl MemoryReporter for NoMemoryReporter {
    fn report(&self) -> MemoryRecord {
        MemoryRecord::unavailable()
    }
}

/// One lane's measurement payload for a case run. Exactly one variant is
/// ever produced per run — this is the structural lane separation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneMeasurement {
    Timing(TimingRecord),
    Memory(MemoryRecord),
    Attribution(WorkCounters),
}

impl LaneMeasurement {
    pub fn lane(&self) -> Lane {
        match self {
            LaneMeasurement::Timing(_) => Lane::Timing,
            LaneMeasurement::Memory(_) => Lane::Memory,
            LaneMeasurement::Attribution(_) => Lane::Attribution,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lane_payloads_serialize_exclusively() {
        // External tag: {"<Variant>": {<payload>}}. The row-level
        // snake_case lane tag lives in runner::MeasurementV1.
        let cases = [
            LaneMeasurement::Timing(TimingRecord::update(11, 23).unwrap()),
            LaneMeasurement::Memory(MemoryRecord::unavailable()),
            LaneMeasurement::Attribution(WorkCounters::all_unknown()),
        ];
        for m in cases {
            let json = serde_json::to_value(&m).unwrap();
            match &m {
                LaneMeasurement::Timing(t) => {
                    assert!(json["Timing"].get("prepare_ns").is_some());
                    assert!(json["Timing"].get("allocated_bytes").is_none());
                    assert!(json["Timing"].get("blocks_reparsed").is_none());
                    assert_eq!(&json["Timing"], &serde_json::to_value(t).unwrap());
                }
                LaneMeasurement::Memory(mem) => {
                    assert!(json["Memory"].get("allocated_bytes").is_some());
                    assert!(json["Memory"].get("prepare_ns").is_none());
                    assert_eq!(&json["Memory"], &serde_json::to_value(mem).unwrap());
                }
                LaneMeasurement::Attribution(w) => {
                    assert!(json["Attribution"].get("blocks_reparsed").is_some());
                    assert!(json["Attribution"].get("prepare_ns").is_none());
                    assert_eq!(&json["Attribution"], &serde_json::to_value(w).unwrap());
                }
            }
        }
    }
}
