//! markit-mdbench-profile-select — CORRECTIVE-B (#35): profile + select.
//!
//! Authority chain:
//!
//! ```text
//! PR #34 merged acquisition authority (3,970 candidates, frozen lock)
//!   -> CORRECTIVE-A semantic substrate (lanes / profiler / oracle)
//!   -> MARKIT-WORKLOAD-PR-SETTLEMENT-AND-CORRECTIVE-B-1 (this task)
//!   -> selections/SELECTION-CONTRACT-v1.md + selection-config-v1.json
//!   -> this crate
//! ```
//!
//! What this crate does:
//!
//! - [`universe`] — materialization verification of every candidate against
//!   the merged PR #34 manifests (identity + sha256 + byte length);
//! - [`rows`] — run REAL-MARKDOWN-PROFILER-v1 over the whole universe under
//!   the declared lanes and reduce each profile to a selection-grade
//!   [`CandidateRow`] (facts stay lane-scoped; nothing is collapsed across
//!   lanes);
//! - [`stats`] / [`bias`] — distributions and the candidate-vs-eligible
//!   eligibility-bias report;
//! - [`redundancy`] — exact byte duplicates plus an inspectable
//!   shingle/MinHash near-duplicate analysis (diagnostic only);
//! - [`selection`] — the frozen SELECTION-CONTRACT-v1 selectors:
//!   REPRESENTATIVE_SET, EXTREMAL_SET, SYNTAX_COVERAGE_SET,
//!   FULL_DOCUMENT_SET, with a complete deterministic trace;
//! - [`coverage`] — retention report data and UNCOVERED-WORKLOAD-SPACE;
//! - [`spotcheck`] — deterministic span-vs-bytes validation samples.
//!
//! What this crate is **not**:
//!
//! - it never measures anything and never names a horse: no timing, no
//!   allocation, no reuse/propagation counter, no restart/convergence
//!   fact, and no dependency on `mechanisms/*`, `instrumentation` or
//!   `runner`;
//! - it does not emit final canonical edits, BREAK/RESTORE payloads,
//!   `payload_id` or `CaseId` values (CORRECTIVE-C owns those);
//! - it does not grant any workload freeze.
//!
//! Every selection decision is machine-derived and reproducible: the same
//! committed inputs produce byte-identical artifacts.

pub mod artifacts;
pub mod bias;
pub mod coverage;
pub mod domains;
pub mod redundancy;
pub mod rows;
pub mod selection;
pub mod spotcheck;
pub mod stats;
pub mod universe;

use serde::{Deserialize, Serialize};

/// Frozen CORRECTIVE-B tool identity (artifact `generator_version`).
pub const CORRECTIVE_B_VERSION: &str = "CORRECTIVE-B-PROFILE-SELECT-v1";
/// Frozen candidate-row record tag.
pub const CANDIDATE_ROW_SCHEMA: &str = "candidate-row-v1";
/// Frozen universe-verification artifact tag.
pub const VERIFICATION_SCHEMA: &str = "candidate-universe-verification-v1";
/// Frozen distributions artifact tag.
pub const DISTRIBUTIONS_SCHEMA: &str = "distributions-v1";
/// Frozen eligibility-bias artifact tag.
pub const ELIGIBILITY_BIAS_SCHEMA: &str = "eligibility-bias-v1";
/// Frozen redundancy artifact tag.
pub const REDUNDANCY_SCHEMA: &str = "redundancy-v1";
/// Frozen domain-strata input tag.
pub const DOMAIN_STRATA_SCHEMA: &str = "domain-strata-v1";
/// Frozen selection-config tag.
pub const SELECTION_CONFIG_SCHEMA: &str = "selection-config-v1";
/// Frozen representative-set artifact tag.
pub const REPRESENTATIVE_SET_SCHEMA: &str = "representative-set-v1";
/// Frozen extremal-set artifact tag.
pub const EXTREMAL_SET_SCHEMA: &str = "extremal-set-v1";
/// Frozen syntax-coverage-set artifact tag.
pub const SYNTAX_COVERAGE_SET_SCHEMA: &str = "syntax-coverage-set-v1";
/// Frozen full-document-set artifact tag.
pub const FULL_DOCUMENT_SET_SCHEMA: &str = "full-document-set-v1";
/// Frozen selection-trace record tag.
pub const SELECTION_TRACE_SCHEMA: &str = "selection-trace-v1";
/// Frozen selected-files artifact tag.
pub const SELECTED_FILES_SCHEMA: &str = "selected-files-v1";
/// Frozen coverage artifact tag.
pub const COVERAGE_SCHEMA: &str = "coverage-v1";
/// Frozen spot-check artifact tag.
pub const SPOT_CHECK_SCHEMA: &str = "spot-check-v1";

/// The lanes every candidate is profiled under (CORRECTIVE-B §10).
pub const PROFILED_GRAMMAR_IDS: [&str; 3] = [
    markit_mdbench_semantics::G0_GRAMMAR_ID,
    markit_mdbench_semantics::G1_GRAMMAR_ID,
    markit_mdbench_semantics::G2_GRAMMAR_ID,
];

/// One candidate file of the frozen universe (manifest identity only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateIdentity {
    pub source_id: String,
    /// Path inside `sources/<source_id>/` as recorded by acquisition
    /// (`snapshot_path`, always `files/…`).
    pub snapshot_path: String,
    pub upstream_path: String,
    pub sha256: String,
    pub bytes: u64,
    pub git_blob_sha1: String,
    pub domain: String,
}

/// Per-lane reduced eligibility/coverage summary of one candidate.
///
/// All numbers are lane-scoped facts copied from the CORRECTIVE-A profile;
/// nothing here mixes lanes. `syntax_counts` maps
/// `grammar_id -> kind -> status-class -> count`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LaneRow {
    pub grammar_id: String,
    pub lane_valid: bool,
    pub strict_scope_clean: bool,
    /// Host-context scope blockers by syntax kind (count).
    pub blocker_kinds: std::collections::BTreeMap<String, u64>,
    pub block_count: u64,
    pub largest_block_bytes: u64,
    pub max_container_depth: u32,
    pub fence_density_per_kib: f64,
    pub code_occupancy: f64,
    pub fenced_code_content_bytes: u64,
    pub reference_definition_count: u64,
    pub reference_use_count: u64,
    pub reference_density_per_kib: f64,
    pub table_count: u64,
    pub max_table_columns: u64,
    pub max_table_rows_including_header: u64,
    /// Recognized + host-context + strict-lane-coverage facts per kind.
    pub strict_kinds: std::collections::BTreeMap<String, u64>,
    /// Recognized facts with grade `contract_declared_not_qualified`.
    pub declared_kinds: std::collections::BTreeMap<String, u64>,
    /// Host-context candidate facts (`not_recognized`) per kind.
    pub candidate_kinds: std::collections::BTreeMap<String, u64>,
    /// Non-host-context facts per kind (inside fence/code/raw regions).
    pub nonhost_kinds: std::collections::BTreeMap<String, u64>,
    /// Host-context `ambiguous` facts per kind (e.g. math under G1).
    pub ambiguous_kinds: std::collections::BTreeMap<String, u64>,
    /// Host-context `unknown` facts per kind.
    pub unknown_kinds: std::collections::BTreeMap<String, u64>,
    /// True when any recognized table detail row carries an alignment
    /// marker (`:` in the delimiter row).
    pub table_alignment_marker: bool,
    /// True when a table probe candidate was rejected by a frozen lane
    /// rule (the escape-aware GFM cell count disagreement).
    pub table_candidate_rejected: bool,
    /// True when a recognized strict inline fact (code span, emphasis,
    /// link) lies inside a recognized table's span under this lane.
    pub inline_inside_table: bool,
    /// Total facts of this lane in the full profile record.
    pub fact_count: u64,
}

/// Selection-grade reduction of one candidate's full profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateRow {
    pub schema: String,
    pub identity: CandidateIdentity,
    pub source_sha256: String,
    pub file_bytes: u64,
    pub line_count: u64,
    pub cjk_byte_share: f64,
    pub newline_form: String,
    /// Profiling failed for this file (e.g. not valid UTF-8); lanes are
    /// empty and the file is ineligible everywhere. Never silently
    /// dropped.
    pub profile_failure: Option<String>,
    pub lanes: Vec<LaneRow>,
    /// Materialization verification outcome for this candidate.
    pub materialized: bool,
    pub hash_match: bool,
}

impl CandidateRow {
    /// The G0 lane row, if profiling succeeded.
    pub fn g0(&self) -> Option<&LaneRow> {
        self.lanes
            .iter()
            .find(|lane| lane.grammar_id == markit_mdbench_semantics::G0_GRAMMAR_ID)
    }

    /// The G1 lane row, if present.
    pub fn g1(&self) -> Option<&LaneRow> {
        self.lanes
            .iter()
            .find(|lane| lane.grammar_id == markit_mdbench_semantics::G1_GRAMMAR_ID)
    }

    /// Strict representative eligibility: G0 `strict_scope_clean` with a
    /// successful profile and verified materialization.
    pub fn g0_strict_eligible(&self) -> bool {
        self.materialized
            && self.hash_match
            && self.profile_failure.is_none()
            && self.g0().map(|lane| lane.strict_scope_clean).unwrap_or(false)
    }

    /// Frozen lexical candidate order: `(source_id, snapshot_path)`.
    pub fn lexical_key(&self) -> (&str, &str) {
        (&self.identity.source_id, &self.identity.snapshot_path)
    }
}

/// Canonical JSON (sorted keys, no whitespace) — same discipline as the
/// CORRECTIVE-A artifacts.
pub fn canonical_json<T: Serialize>(value: &T) -> String {
    markit_mdbench_semantics::canonical_json(value)
}

/// SHA-256 hex of arbitrary bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    markit_mdbench_semantics::sha256_hex(bytes)
}
