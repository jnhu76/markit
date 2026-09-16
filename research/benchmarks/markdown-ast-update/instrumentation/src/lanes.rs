//! Measurement lane separation (R1 task §8, R1-CORRECTIVE-1 MAJOR-2).
//!
//! Three structurally distinct lanes; never one run with everything
//! enabled presented as headline timing:
//!
//! ```text
//! T-LANE = timing            real timer, NoopWorkSink, no memory collector
//! M-LANE = memory/allocation per-case memory windows, no headline TimingRecord
//! A-LANE = attribution       work counters, no headline TimingRecord
//! ```

use std::cell::RefCell;

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

/// M-LANE record: per-case memory facts.
///
/// Peak memory and retained memory are DISTINCT R0 metrics and are
/// reported as logically separate slots — `peak_retained_bytes` was a
/// conflation and is gone (R1-CORRECTIVE-1, MAJOR-2):
///
/// - `allocated_bytes`: total bytes allocated during the case window;
/// - `allocation_count`: number of allocation events in the window;
/// - `peak_bytes`: maximum live bytes observed inside the window;
/// - `retained_bytes`: bytes still held when the window closed (the cost
///   of the new retained state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryRecord {
    pub allocated_bytes: Observed<u64>,
    pub allocation_count: Observed<u64>,
    pub peak_bytes: Observed<u64>,
    pub retained_bytes: Observed<u64>,
}

impl MemoryRecord {
    /// Placeholder for lanes run without an allocation instrumentor.
    pub fn unavailable() -> Self {
        Self {
            allocated_bytes: Observed::Unknown,
            allocation_count: Observed::Unknown,
            peak_bytes: Observed::Unknown,
            retained_bytes: Observed::Unknown,
        }
    }
}

/// Opaque per-case handle: opened by [`MemoryReporter::begin_case`],
/// consumed by [`MemoryReporter::end_case`]. The runner opens the window
/// before the mechanism runs and closes it strictly after the mechanism
/// finished — exactly one window per case run, so per-case deltas have a
/// frozen authority and allocations can never leak across cases.
pub struct CaseMemoryProbe {
    case_index: u64,
}

impl CaseMemoryProbe {
    /// Opaque handle constructor for [`MemoryReporter`] implementors.
    pub fn new(case_index: u64) -> Self {
        Self { case_index }
    }

    pub(crate) fn case_index(&self) -> u64 {
        self.case_index
    }
}

/// Per-case memory authority (R1-CORRECTIVE-1, MAJOR-2).
///
/// A reporter owns measurement WINDOWS, not a single post-hoc report:
///
/// ```text
/// begin_case()            open the per-case window (before the mechanism)
/// ... mechanism runs ...
/// end_case(probe)         close the window, derive the per-case delta
/// ```
///
/// R1 ships the lifecycle and the honest placeholder only; a real
/// allocator-backed reporter is a later infrastructure addition and must
/// never run inside the timing lane.
pub trait MemoryReporter {
    /// Open one per-case measurement window.
    fn begin_case(&self) -> CaseMemoryProbe;
    /// Close the window and produce the per-case delta record. Must be
    /// called for every opened probe — including failed runs — so the
    /// lane is preserved.
    fn end_case(&self, probe: CaseMemoryProbe) -> MemoryRecord;
}

/// Interface-only reporter: every slot `Unknown`, honestly.
pub struct NoMemoryReporter;

impl MemoryReporter for NoMemoryReporter {
    fn begin_case(&self) -> CaseMemoryProbe {
        CaseMemoryProbe::new(0)
    }

    fn end_case(&self, _probe: CaseMemoryProbe) -> MemoryRecord {
        MemoryRecord::unavailable()
    }
}

/// Deterministic, allocator-free per-case reporter — the `ManualClock`
/// analogue for the M-LANE: tests and harness validation inject exact
/// observations into the open window, and every value is attributable to
/// exactly one case. This is NOT a production instrumentor.
///
/// Observations made with no window open are dropped (a window bug must
/// not silently contaminate another case).
#[derive(Default)]
pub struct ManualMemoryReporter {
    state: RefCell<ManualState>,
}

#[derive(Default)]
struct ManualState {
    next_case: u64,
    open: Option<(u64, ManualWindow)>,
}

#[derive(Default)]
struct ManualWindow {
    allocated_bytes: u64,
    allocation_count: u64,
    overflow: bool,
    peak_bytes: Option<u64>,
    retained_bytes: Option<u64>,
}

impl ManualMemoryReporter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attribute ONE allocation of `bytes` to the currently open window.
    pub fn observe_allocation(&self, bytes: u64) {
        let mut state = self.state.borrow_mut();
        if let Some((_, window)) = state.open.as_mut() {
            window.allocation_count += 1;
            match window.allocated_bytes.checked_add(bytes) {
                Some(sum) => window.allocated_bytes = sum,
                None => window.overflow = true,
            }
        }
    }

    /// Observe a running peak (the maximum wins) in the open window.
    pub fn observe_peak(&self, bytes: u64) {
        let mut state = self.state.borrow_mut();
        if let Some((_, window)) = state.open.as_mut() {
            window.peak_bytes = Some(window.peak_bytes.unwrap_or(0).max(bytes));
        }
    }

    /// Set the retained-bytes gauge for the open window (last value wins).
    pub fn observe_retained(&self, bytes: u64) {
        let mut state = self.state.borrow_mut();
        if let Some((_, window)) = state.open.as_mut() {
            window.retained_bytes = Some(bytes);
        }
    }
}

impl MemoryReporter for ManualMemoryReporter {
    fn begin_case(&self) -> CaseMemoryProbe {
        let mut state = self.state.borrow_mut();
        // An unclosed previous window is dropped: windows never merge.
        state.next_case += 1;
        state.open = Some((state.next_case, ManualWindow::default()));
        CaseMemoryProbe::new(state.next_case)
    }

    fn end_case(&self, probe: CaseMemoryProbe) -> MemoryRecord {
        let mut state = self.state.borrow_mut();
        let window = match state.open.take() {
            Some((index, window)) => {
                debug_assert_eq!(
                    index,
                    probe.case_index(),
                    "end_case must close the window opened by begin_case"
                );
                Some(window)
            }
            None => None,
        };
        match window {
            None => MemoryRecord::unavailable(),
            Some(window) => MemoryRecord {
                allocated_bytes: if window.overflow {
                    Observed::Unknown
                } else {
                    Observed::Known(window.allocated_bytes)
                },
                allocation_count: Observed::Known(window.allocation_count),
                peak_bytes: window
                    .peak_bytes
                    .map(Observed::Known)
                    .unwrap_or(Observed::Unknown),
                retained_bytes: window
                    .retained_bytes
                    .map(Observed::Known)
                    .unwrap_or(Observed::Unknown),
            },
        }
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
                    assert!(json["Memory"].get("peak_bytes").is_some());
                    assert!(json["Memory"].get("retained_bytes").is_some());
                    assert!(json["Memory"].get("peak_retained_bytes").is_none());
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

    #[test]
    fn manual_reporter_scopes_values_to_one_case_never_leaks() {
        let reporter = ManualMemoryReporter::new();

        // Case 1: two allocations, a peak, a retained gauge.
        let probe = reporter.begin_case();
        reporter.observe_allocation(60);
        reporter.observe_allocation(40);
        reporter.observe_peak(70);
        reporter.observe_retained(30);
        let case1 = reporter.end_case(probe);
        assert_eq!(case1.allocated_bytes, Observed::Known(100));
        assert_eq!(case1.allocation_count, Observed::Known(2));
        assert_eq!(case1.peak_bytes, Observed::Known(70));
        assert_eq!(case1.retained_bytes, Observed::Known(30));

        // Case 2 starts from zero: nothing of case 1 leaks in...
        let probe = reporter.begin_case();
        reporter.observe_allocation(7);
        reporter.observe_peak(7);
        let case2 = reporter.end_case(probe);
        assert_eq!(case2.allocated_bytes, Observed::Known(7));
        assert_eq!(case2.allocation_count, Observed::Known(1));
        assert_eq!(case2.peak_bytes, Observed::Known(7));
        assert_eq!(case2.retained_bytes, Observed::Unknown);
        assert_ne!(case1, case2);

        // ...and observations with no open window go nowhere.
        reporter.observe_allocation(999);
        let probe = reporter.begin_case();
        let case3 = reporter.end_case(probe);
        assert_eq!(case3.allocated_bytes, Observed::Known(0));
        assert_eq!(case3.allocation_count, Observed::Known(0));
        assert_eq!(case3.peak_bytes, Observed::Unknown);
    }

    #[test]
    fn allocation_overflow_is_honest_unknown() {
        let reporter = ManualMemoryReporter::new();
        let probe = reporter.begin_case();
        reporter.observe_allocation(u64::MAX);
        reporter.observe_allocation(1);
        let record = reporter.end_case(probe);
        assert_eq!(record.allocated_bytes, Observed::Unknown);
        assert_eq!(record.allocation_count, Observed::Known(2));
    }
}
