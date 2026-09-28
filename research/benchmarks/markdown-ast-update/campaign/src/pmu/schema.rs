//! PMU observation schema (task §22 of the PMU campaign order): every
//! row binds the study identity, the frozen panel, the frozen schedule,
//! the driver binary, the primary mechanism authority, the host and
//! toolchain, the event group encodings, and the cell's preserved
//! selection identity.
//!
//! Facts only: raw counts, times, fractions, and correctness outcomes.
//! No interpretation, no ranking, no derived cache-miss taxonomy —
//! derivation happens only after the raw evidence is sealed.

use serde::{Deserialize, Serialize};

use super::events::EventDef;

/// Observation row schema id.
pub const PMU_OBSERVATION_SCHEMA: &str = "pmu-observation-v1";

/// Qualification verdicts (frozen before any horse observation).
pub const QUAL_QUALIFIED: &str = "QUALIFIED";
pub const QUAL_UNQUALIFIED_MULTIPLEXED: &str = "UNQUALIFIED_MULTIPLEXED";
pub const QUAL_COUNTER_UNAVAILABLE: &str = "COUNTER_UNAVAILABLE";
pub const QUAL_INVALID: &str = "INVALID";

/// A raw event count with its multiplexing bookkeeping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct EventCountV1 {
    pub alias: String,
    /// `perf_event_attr.type` of the exact encoding used.
    pub perf_type: u32,
    /// `perf_event_attr.config` of the exact encoding used.
    pub config: u64,
    pub raw_count: u64,
    pub time_enabled: u64,
    pub time_running: u64,
    /// `time_running / time_enabled`; `null` when `time_enabled == 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_fraction: Option<f64>,
}

impl EventCountV1 {
    pub fn from_reading(def: &EventDef, reading: super::perfcount::EventReading) -> Self {
        EventCountV1 {
            alias: def.alias.to_string(),
            perf_type: def.kind,
            config: def.config,
            raw_count: reading.value,
            time_enabled: reading.time_enabled,
            time_running: reading.time_running,
            running_fraction: reading.running_fraction(),
        }
    }
}

/// Correctness block (verified strictly OUTSIDE the counter window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CorrectnessV1 {
    pub execution_status: String,
    pub correctness_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_checksum: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
}

/// One PMU observation row (append/create-only evidence).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PmuObservationV1 {
    pub schema: String,
    pub pmu_study_id: String,
    pub observation_id: String,
    pub pmu_run_id: String,
    pub panel_manifest_sha256: String,
    pub pmu_schedule_sha256: String,
    pub driver_sha256: String,
    pub primary_mechanism_authority: String,
    pub entry_ordinal: u32,
    pub slot: String,
    pub case_id: String,
    pub surface: String,
    pub payload_id: String,
    pub frozen_regime: String,
    pub selection_class: String,
    pub selection_reason: String,
    pub horse: String,
    pub mechanism_id: String,
    pub event_group: String,
    pub repetition: u32,
    pub events: Vec<EventCountV1>,
    /// Frozen LOW_COUNT threshold: events with `raw_count` below this are
    /// flagged for INCONCLUSIVE metric derivation.
    pub low_count_events: Vec<String>,
    pub qualification: String,
    pub correctness: CorrectnessV1,
    pub host: HostIdentityV1,
    pub toolchain: ToolchainV1,
}

/// Host identity recorded per observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct HostIdentityV1 {
    pub hostname: String,
    pub kernel: String,
    pub cpu: String,
    pub perf_event_paranoid: String,
}

/// Toolchain identity recorded per observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ToolchainV1 {
    pub rustc: String,
    pub cargo: String,
    pub build_profile: String,
}

/// `ObservationId` = hex(SHA256(canonical line of immutable identity
/// fields)). No two PMU observations may share one, and the domain line
/// makes cross-study collisions structurally impossible.
pub fn pmu_observation_id(
    run_id: &str,
    entry_ordinal: u32,
    horse: &str,
    event_group: &str,
    repetition: u32,
) -> String {
    let material = format!(
        "{PMU_OBSERVATION_SCHEMA}\n{run_id}\n{entry_ordinal}\n{horse}\n{event_group}\n{repetition}"
    );
    crate::sha256_hex(material.as_bytes())
}

/// The PMU run identity: binds the executable, the frozen panel, the
/// frozen schedule, and the host. One collection = one run id.
pub fn pmu_run_id(
    driver_sha256: &str,
    panel_sha256: &str,
    schedule_sha256: &str,
    hostname: &str,
) -> String {
    let material = format!(
        "{PMU_OBSERVATION_SCHEMA}\n{}\n{driver_sha256}\n{panel_sha256}\n{schedule_sha256}\n{hostname}",
        super::panel::PMU_STUDY_ID
    );
    crate::sha256_hex(material.as_bytes())
}
