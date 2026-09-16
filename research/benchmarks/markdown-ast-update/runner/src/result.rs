//! Versioned raw result row (schema v1).
//!
//! The Rust model in this file is the single source of truth;
//! `protocol/result-schema-v1.json` is GENERATED from it (see the
//! `mdbench-gen-schema` binary) and `protocol/result-schema.md` documents
//! it. Raw rows preserve facts only: no winner, rank, score, weighted
//! metric, or speedup conclusion may ever be added here.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use markit_mdbench_common::CaseId;
use markit_mdbench_common::CorrectnessStatus;
use markit_mdbench_common::ExecutionStatus;
use markit_mdbench_common::MechanismId;
use markit_mdbench_common::Observed;
use markit_mdbench_common::OperationKind;
use markit_mdbench_common::PayloadShape;
use markit_mdbench_common::Seed;
use markit_mdbench_common::WorkCounters;
use markit_mdbench_instrumentation::LaneMeasurement;
use markit_mdbench_instrumentation::MemoryRecord;
use markit_mdbench_instrumentation::TimingRecord;

/// Version of the raw result row schema emitted by this runner.
pub const RESULT_SCHEMA_VERSION_V1: u16 = 1;

/// The frozen protocol generation these rows were produced under.
/// Points at `protocol/R0-METHODOLOGY.md` (R0 FROZEN, verdict PASS).
pub const PROTOCOL_VERSION: &str = "R0-FROZEN-V1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BuildIdentitySlot {
    pub runner_git_commit: String,
    pub rustc: String,
    pub target: String,
    pub build_profile_id: String,
    pub cargo_lock_sha256: String,
}

impl From<super::build_identity::BuildIdentityV1> for BuildIdentitySlot {
    fn from(v: super::build_identity::BuildIdentityV1) -> Self {
        Self {
            runner_git_commit: v.runner_git_commit,
            rustc: v.rustc,
            target: v.target,
            build_profile_id: v.build_profile_id,
            cargo_lock_sha256: v.cargo_lock_sha256,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PayloadMetaV1 {
    pub payload_id: String,
    pub shape: PayloadShape,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct EditMetaV1 {
    pub start_byte: Option<u64>,
    pub end_byte: Option<u64>,
    /// SHA256 of the exact inserted UTF-8 bytes (`null` when the
    /// operation carries no insertion, e.g. FULL_PARSE / DELETE).
    pub inserted_sha256: Option<String>,
}

/// Why an operation/edit combination was rejected: raw rows must never
/// serialize facts that contradict the operation contract shared with
/// `CaseKeyV1` (R1-CORRECTIVE-1, MAJOR-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMetaError {
    /// Operation carries no edit but an edit was supplied.
    UnexpectedEdit,
    /// Operation carries an edit but none was supplied.
    MissingEdit,
    /// Operation carries no inserted text but the edit does
    /// (e.g. a DELETE with a non-empty inserted string).
    UnexpectedInsertedText,
}

impl core::fmt::Display for EditMetaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EditMetaError::UnexpectedEdit => {
                f.write_str("operation carries no edit, but an edit was supplied")
            }
            EditMetaError::MissingEdit => {
                f.write_str("operation carries an edit, but no edit was supplied")
            }
            EditMetaError::UnexpectedInsertedText => {
                f.write_str("operation carries no inserted text, but the edit has inserted text")
            }
        }
    }
}

impl std::error::Error for EditMetaError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimingMetricsV1 {
    pub prepare_ns: Observed<u64>,
    pub native_ns: Observed<u64>,
    pub total_ns: Observed<u64>,
}

impl From<TimingRecord> for TimingMetricsV1 {
    fn from(r: TimingRecord) -> Self {
        Self {
            prepare_ns: r.prepare_ns,
            native_ns: r.native_ns,
            total_ns: r.total_ns,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryMetricsV1 {
    pub allocated_bytes: Observed<u64>,
    pub allocation_count: Observed<u64>,
    /// Maximum live bytes observed during the case window. Distinct from
    /// `retained_bytes` — the two are never conflated
    /// (R1-CORRECTIVE-1, MAJOR-2).
    pub peak_bytes: Observed<u64>,
    /// Bytes still held when the case window closed (cost of the new
    /// retained state).
    pub retained_bytes: Observed<u64>,
}

impl From<MemoryRecord> for MemoryMetricsV1 {
    fn from(r: MemoryRecord) -> Self {
        Self {
            allocated_bytes: r.allocated_bytes,
            allocation_count: r.allocation_count,
            peak_bytes: r.peak_bytes,
            retained_bytes: r.retained_bytes,
        }
    }
}

/// Tagged lane payload:
///
/// ```json
/// { "lane": "timing", "metrics": { "prepare_ns": 11, "native_ns": 23, "total_ns": 34 } }
/// ```
///
/// Exactly one lane per row. Metrics shapes are structurally exclusive
/// (`deny_unknown_fields`), so a timing row carrying attribution keys is
/// invalid and rejected on deserialize.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "lane", content = "metrics", rename_all = "snake_case")]
pub enum MeasurementV1 {
    Timing(TimingMetricsV1),
    Memory(MemoryMetricsV1),
    Attribution(WorkCounters),
}

impl From<LaneMeasurement> for MeasurementV1 {
    fn from(m: LaneMeasurement) -> Self {
        match m {
            LaneMeasurement::Timing(t) => MeasurementV1::Timing(t.into()),
            LaneMeasurement::Memory(m) => MeasurementV1::Memory(m.into()),
            LaneMeasurement::Attribution(w) => MeasurementV1::Attribution(w),
        }
    }
}

/// One raw result row: all facts of one case run in one lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ResultRowV1 {
    pub schema_version: u16,
    pub protocol_version: String,
    pub build_identity: BuildIdentitySlot,
    pub case_id: String,
    pub seed: u64,
    pub mechanism_id: String,
    pub operation: OperationKind,
    pub payload: PayloadMetaV1,
    pub edit: EditMetaV1,
    pub execution_status: ExecutionStatus,
    pub correctness_status: CorrectnessStatus,
    /// Hex checksum of the completed result (`null` when the run did not
    /// complete). A correctness fact, never a performance claim.
    pub result_checksum: Option<String>,
    /// Reference to the environment record, e.g.
    /// `manifest/environment.toml#<environment_id>`.
    pub environment_ref: String,
    /// Provenance tag; smoke/validation rows MUST carry
    /// `R1_SMOKE_ONLY/NON_RESEARCH_RESULT`.
    pub provenance_ref: String,
    pub measurement: MeasurementV1,
}

/// Logical facts identifying the case a report belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFacts {
    pub case_id: CaseId,
    pub seed: Seed,
    pub mechanism_id: MechanismId,
    pub operation: OperationKind,
    pub payload: PayloadMetaV1,
    pub edit: EditMetaV1,
}

/// Assemble the raw row. Pure function of facts + report + build
/// identity; no timing, clock, or randomness involved.
pub fn assemble_row(
    facts: &CaseFacts,
    report: &super::orchestrate::RunReport,
    build_identity: &super::build_identity::BuildIdentityV1,
    environment_ref: &str,
    provenance_ref: &str,
) -> ResultRowV1 {
    ResultRowV1 {
        schema_version: RESULT_SCHEMA_VERSION_V1,
        protocol_version: PROTOCOL_VERSION.to_string(),
        build_identity: build_identity.clone().into(),
        case_id: facts.case_id.hex(),
        seed: facts.seed.0,
        mechanism_id: facts.mechanism_id.0.clone(),
        operation: facts.operation,
        payload: facts.payload.clone(),
        edit: facts.edit.clone(),
        execution_status: report.execution_status,
        correctness_status: report.correctness_status,
        result_checksum: report.result_checksum.map(|c| format!("{c:016x}")),
        environment_ref: environment_ref.to_string(),
        provenance_ref: provenance_ref.to_string(),
        measurement: report.measurement.clone().into(),
    }
}

/// `EditMetaV1` from the operation + its optional canonical edit, using
/// the SAME operation contract as `CaseKeyV1` — raw-result metadata and
/// case identity can never disagree (R1-CORRECTIVE-1, MAJOR-4).
///
/// Invariants (inconsistent combinations are rejected, not serialized):
///
/// ```text
/// FULL_PARSE / QUERY:            no range, inserted_sha256 = null
/// DELETE:                        range present, inserted_sha256 = null
/// INSERT / REPLACE_EQ / REPLACE_GROW / REPLACE_SHRINK / STRUCTURAL_EDIT:
///                                range present, inserted_sha256 present
///                                (empty insertions hash the empty string,
///                                exactly like CaseKeyV1)
/// ```
pub fn edit_meta(
    operation: OperationKind,
    edit: Option<&markit_mdbench_common::CanonicalEdit>,
) -> Result<EditMetaV1, EditMetaError> {
    match (operation.has_edit(), edit) {
        (false, None) => Ok(EditMetaV1 {
            start_byte: None,
            end_byte: None,
            inserted_sha256: None,
        }),
        (false, Some(_)) => Err(EditMetaError::UnexpectedEdit),
        (true, None) => Err(EditMetaError::MissingEdit),
        (true, Some(e)) => {
            let inserted_sha256 = if operation.has_inserted_text() {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(e.inserted_text().as_bytes());
                let digest: [u8; 32] = hasher.finalize().into();
                Some(markit_mdbench_common::to_lower_hex(&digest))
            } else {
                // DELETE (or any future non-inserting edit): the canonical
                // form carries no inserted bytes.
                if !e.inserted_text().is_empty() {
                    return Err(EditMetaError::UnexpectedInsertedText);
                }
                None
            };
            Ok(EditMetaV1 {
                start_byte: Some(e.start_byte()),
                end_byte: Some(e.end_byte()),
                inserted_sha256,
            })
        }
    }
}
