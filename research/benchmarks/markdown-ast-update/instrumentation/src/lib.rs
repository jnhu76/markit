//! markit-mdbench-instrumentation — timer / lane interfaces only.
//!
//! Timer placement belongs to the runner; mechanisms never receive a
//! timer or clock type from this crate. Lane records (T/M/A) are the
//! stable measurement vocabulary shared with the result schema.

pub mod clock;
pub mod lanes;
pub mod timer;

pub use clock::{Clock, InstantClock, ManualClock};
pub use lanes::{
    CaseMemoryProbe, Lane, LaneMeasurement, ManualMemoryReporter, MemoryRecord, MemoryReporter,
    NoMemoryReporter,
};
pub use timer::{PhaseGuard, TimingError, TimingRecord};

/// Serialized names of the three lanes (used in result rows).
pub const TIMING_LANE_NAME: &str = "timing";
pub const MEMORY_LANE_NAME: &str = "memory";
pub const ATTRIBUTION_LANE_NAME: &str = "attribution";
