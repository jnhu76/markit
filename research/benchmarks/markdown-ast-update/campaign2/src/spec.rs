//! The frozen Campaign-2 specification (task §7, §44).
//!
//! `CampaignSpecId` hashes THIS structure's canonical JSON. It binds
//! policies and digests only — never a measured value, so the spec id is
//! fixed before the first formal row and could not have been chosen after
//! seeing timing.

use serde::{Deserialize, Serialize};

use crate::{CAMPAIGN2_ID, ENVELOPE_SCHEMA_ID, METRIC_QUALIFICATION, STUDY_ID};

/// Timer boundaries (task §13-§15). Recorded so a reader can see, from
/// the spec alone, what was inside each timer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
 #[serde(deny_unknown_fields)]
pub struct TimerBoundaries {
    /// Surface A: source resident -> start -> clean parse -> native state
    /// construction -> seal -> usable -> stop. (Excludes filesystem IO,
    /// oracle validation, normalize, checksum, report serialization.)
    pub construction: String,
    /// Surface B: fresh pre-edit state built OUTSIDE the timer; then
    /// start -> prepare_update -> update -> seal -> stop; oracle outside.
    pub resident_update: String,
    /// Surface C: build once (untimed), then per step the same
    /// `T_prepare`/`T_native` boundaries as Surface B, with NO state
    /// reconstruction between edits.
    pub lifecycle: String,
    /// Surface D: identical to Surface B.
    pub controlled: String,
}

/// Sampling policy (task §18, §26).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplingPolicy {
    pub session_count: u32,
    pub warmup_iterations: u32,
    pub measured_iterations: u32,
    /// Lifecycle repetition count, frozen BEFORE the first formal
    /// lifecycle row (task §18). Never changed after seeing a result.
    pub lifecycle_repetitions: u32,
    /// Controlled-surface repetition count, frozen BEFORE the first
    /// formal controlled row (task §26).
    pub controlled_repetitions: u32,
    pub warmup_applies_to: Vec<String>,
    pub note: String,
}

/// Statistics policy (task §45-§46).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsPolicy {
    pub quantile: String,
    pub p50: String,
    pub p95: String,
    pub pooling: String,
    pub case_estimate: String,
    pub h0_relative: String,
    pub lifecycle: String,
    pub instability_flag: String,
    pub instability_action: String,
}

/// Correctness oracle (task §12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectnessPolicy {
    pub oracle: String,
    pub reference: String,
    pub outside_timer: bool,
    pub lifecycle_verification: String,
    pub failure_action: String,
}

/// Failure policy (task §48).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailurePolicy {
    pub on_lane_failure: String,
    pub on_code_change: String,
    pub forbidden: Vec<String>,
    pub performance_driven_adaptation: String,
}

/// Profiling policy (task §31, §34-§41).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilingPolicy {
    pub primary_timing_clean: bool,
    pub forbidden_during_primary: Vec<String>,
    pub matched_rule: String,
    pub perf_stat_repetitions: u32,
    pub perf_stat_rotation: String,
    pub perf_record_selection: String,
    pub ebpf_role: String,
    pub allocator_label: String,
    pub region_isolation: String,
}

/// CPU binding (task §11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuBindingPolicy {
    pub selected_cpu: u32,
    pub selected_core_id: u32,
    pub selected_thread_siblings: String,
    pub selected_numa_node: u32,
    pub policy: String,
}

/// The frozen Campaign-2 campaign specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign2Spec {
    pub study_id: String,
    pub campaign_id: String,
    pub schema: String,
    pub authority_sha: String,
    pub horse_set: Vec<String>,
    pub horse_mechanism_ids: Vec<String>,
    pub real_workload_identity: RealWorkloadIdentity,
    pub controlled_generator_identity: ControlledGeneratorIdentity,
    pub timer_boundaries: TimerBoundaries,
    pub sampling: SamplingPolicy,
    pub cpu_binding: CpuBindingPolicy,
    pub statistics: StatisticsPolicy,
    pub correctness: CorrectnessPolicy,
    pub failure: FailurePolicy,
    pub profiling: ProfilingPolicy,
    pub interpretation_rules: Vec<String>,
    pub envelope_schema_id: String,
    pub envelope_schema_sha256: String,
    pub metric_qualification: Vec<(String, String)>,
    pub order_algorithms: Vec<String>,
    pub base_authority_sha: String,
}

impl Campaign2Spec {
    /// Canonical serialized form (stable field order, no whitespace).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("campaign-2 spec is JSON-serializable")
    }

    pub fn study_id(&self) -> &str {
        &self.study_id
    }
}

/// Identity of the frozen real workload consumed by Campaign-2 (task §8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealWorkloadIdentity {
    /// The #35 freeze this campaign consumes; never re-selected.
    pub source: String,
    pub full_read_manifest: String,
    pub full_read_manifest_sha256: String,
    pub edit_write_manifest: String,
    pub edit_write_manifest_sha256: String,
    pub trace_manifest: String,
    pub trace_manifest_sha256: String,
    pub freeze_receipt: String,
    pub freeze_receipt_sha256: String,
    pub full_read_qualified_cases: u64,
    pub edit_write_qualified_cases: u64,
    pub trace_records: u64,
    pub break_restore_pairs: u64,
    pub projects: u64,
    pub edit_families: u64,
    pub reselection_allowed: bool,
}

/// Identity of the controlled workload generators (task §20-§25).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlledGeneratorIdentity {
    pub generator_id: String,
    pub generator_version: String,
    pub axes: Vec<String>,
    pub n_scale_points: Vec<String>,
    pub b_scale_points: Vec<String>,
    pub d_scale_points: Vec<String>,
    pub f_scale_points: Vec<String>,
    pub k_scale_points: Vec<String>,
    pub fixed_n_bytes: u64,
    pub note: String,
}

/// One sub-campaign's frozen binding. Its canonical JSON (prefixed by the
/// campaign spec id and the tag) produces the `SubCampaignSpecId`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubCampaignBinding {
    pub tag: String,
    pub surface: String,
    pub evidence_classes: Vec<String>,
    pub lanes: Vec<String>,
    pub case_cardinality: u64,
    pub session_count: u32,
    pub warmup_iterations: u32,
    pub measured_iterations: u32,
    /// Axis values for controlled sub-campaigns; empty otherwise.
    pub axis_values: Vec<String>,
    pub raw_path: String,
    pub note: String,
}

impl SubCampaignBinding {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("sub-campaign binding is JSON-serializable")
    }
}

/// The frozen campaign-2 identifier string.
pub fn campaign_id() -> &'static str {
    CAMPAIGN2_ID
}

/// The frozen study identifier string.
pub fn study_id_tag() -> &'static str {
    STUDY_ID
}

/// The frozen envelope schema tag.
pub fn envelope_schema_tag() -> &'static str {
    ENVELOPE_SCHEMA_ID
}

/// The frozen metric qualification table.
pub fn metric_qualification() -> Vec<(String, String)> {
    METRIC_QUALIFICATION
        .iter()
        .map(|(m, q)| (m.to_string(), q.to_string()))
        .collect()
}
