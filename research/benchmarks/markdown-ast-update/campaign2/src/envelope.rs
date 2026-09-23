//! The Campaign-2 raw observation envelope (task §49-§50).
//!
//! A narrow identity wrapper around ONE existing schema-v2 result row. The
//! core row is NOT overloaded with campaign scheduling facts, and banned
//! analysis fields (winner / rank / speedup / score / interpretation) are
//! structurally absent.
//!
//! Optional blocks are omitted entirely when not applicable, so a
//! construction row can never look like a lifecycle row.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use markit_mdbench_runner::ResultRowV1;

/// Frozen schema tag.
pub const SCHEMA: &str = "campaign2-observation-v1";

/// Sample kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SampleKind2 {
    Warmup,
    Measured,
    Attribution,
    /// Lifecycle: every edit is recorded, so every step is a `Measured`
    /// row; `Lifecycle` is reserved for cumulative checkpoint rows.
    LifecycleStep,
    LifecycleCheckpoint,
    /// Descriptive process-memory observation (never primary timing).
    Memory,
}

impl SampleKind2 {
    pub fn as_str(self) -> &'static str {
        match self {
            SampleKind2::Warmup => "warmup",
            SampleKind2::Measured => "measured",
            SampleKind2::Attribution => "attribution",
            SampleKind2::LifecycleStep => "lifecycle_step",
            SampleKind2::LifecycleCheckpoint => "lifecycle_checkpoint",
            SampleKind2::Memory => "memory",
        }
    }
}

/// Controlled-cell identity (Surface D).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CellIdentityV1 {
    pub axis: String,
    pub axis_point_index: u32,
    pub axis_label: String,
    pub axis_value: u64,
    pub cell_id: String,
    pub generator_id: String,
    pub generator_version: String,
}

/// Lifecycle identity (Surface C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LifecycleIdentityV1 {
    pub trace_id: String,
    pub family: String,
    pub chain_construction: String,
    pub step: u32,
    pub step_count: u32,
    /// Which independent lifecycle repetition of this (trace, horse) this
    /// step belongs to.
    pub rep: u32,
    pub checkpoint: Option<u32>,
    pub cumulative_edits: u32,
    pub step_label: String,
    pub transition_label: String,
}

/// Descriptive state-representation counts (task §29).
///
/// These are descriptive evidence about state cost. A count the mechanism
/// does not expose is `UNAVAILABLE` — never a fabricated zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateReprV1 {
    /// Retained top-level block/skeleton units in the sealed state.
    pub retained_blocks: String,
    /// Retained checkpoints where the mechanism has them.
    pub checkpoints: String,
    /// Fragment / run metadata entries where the mechanism has them.
    pub fragment_metadata_entries: String,
    /// Old-tree index entries where the mechanism has them.
    pub old_tree_index_entries: String,
    /// Source bytes held by the sealed state.
    pub retained_source_bytes: String,
    /// How the counts were obtained (pure post-timer export).
    pub provenance: String,
}

impl StateReprV1 {
    pub fn unavailable(provenance: &str) -> Self {
        Self {
            retained_blocks: "UNAVAILABLE".to_string(),
            checkpoints: "UNAVAILABLE".to_string(),
            fragment_metadata_entries: "UNAVAILABLE".to_string(),
            old_tree_index_entries: "UNAVAILABLE".to_string(),
            retained_source_bytes: "UNAVAILABLE".to_string(),
            provenance: provenance.to_string(),
        }
    }
}

/// One Campaign-2 raw observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Campaign2ObservationV1 {
    pub schema: String,
    pub study_id: String,
    pub campaign_spec_id: String,
    pub sub_campaign_spec_id: String,
    pub run_id: String,
    pub evidence_class: String,
    pub surface: String,
    pub lane: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_ordinal: Option<u32>,
    pub case_order_ordinal: u32,
    pub horse_order_ordinal: u32,
    pub horse_id: String,
    pub sample_kind: String,
    pub iteration_ordinal: u32,
    /// Unique across the whole campaign (duplicate-guarded).
    pub observation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<CellIdentityV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<LifecycleIdentityV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_repr: Option<StateReprV1>,
    /// The existing raw result row (schema v2). Rust authority:
    /// `runner/src/result.rs`.
    pub result_row_v2: ResultRowV1,
}

/// Envelope identity context stamped into every observation of one
/// execution.
#[derive(Debug, Clone)]
pub struct ExecutionIdentity2 {
    pub study_id: String,
    pub campaign_spec_id: String,
    pub sub_campaign_spec_id: String,
    pub run_id: String,
    pub evidence_class: &'static str,
    pub machine_environment_ref: String,
    pub provenance: &'static str,
    pub non_research: bool,
}

/// Where observations are appended (create/append-only).
pub trait ObservationSink2 {
    fn append(&mut self, observation: &Campaign2ObservationV1) -> std::io::Result<()>;
}

impl<W: std::io::Write> ObservationSink2 for W {
    fn append(&mut self, observation: &Campaign2ObservationV1) -> std::io::Result<()> {
        let line = serde_json::to_string(observation)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.write_all(line.as_bytes())?;
        self.write_all(b"\n")
    }
}
