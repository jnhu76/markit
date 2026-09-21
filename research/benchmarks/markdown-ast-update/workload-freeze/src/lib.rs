//! markit-mdbench-workload-freeze — CORRECTIVE-C real-workload payload
//! freeze (#35, primary G0 campaign).
//!
//! Authority chain:
//!
//! ```text
//! issue #35 (+ Corrective-1/2, Workload Construction Algorithm v1)
//!   -> CORRECTIVE-A semantic substrate (PR #37): lanes / profiler /
//!      transition oracle / payload lifecycle      [frozen]
//!   -> CORRECTIVE-B profile + select (PR #38): 36 physical files  [frozen]
//!   -> CORRECTIVE-C (this crate): A6 anchors + applicability,
//!      A7 FULL_READ / EDIT_WRITE / trace payloads, TRANSITION-REGISTRY-v1,
//!      A8 correctness-only harness dry-run, final coverage report
//!   -> CORE_REAL_WORKLOAD_FREEZE_PASS (candidate; human review owns it)
//! ```
//!
//! What this crate is:
//!
//! - a deterministic generator: the same inputs always produce
//!   byte-identical artifacts (checked by the `determinism` subcommand);
//! - a validator: every frozen payload is re-verified against the frozen
//!   TRANSITION-ORACLE-v1 and the frozen source identities;
//! - the harness dry-run adapter (G7): it dispatches frozen G0 payloads
//!   through the EXISTING runner correctness path (H0-H4), using the
//!   explicit correctness-only runner mode — no clock, no work counters,
//!   no timing value of any kind.
//!
//! What this crate is **not**:
//!
//! - not a new benchmark engine: the runner / oracle / CaseId / horse
//!   contracts are reused unchanged;
//! - not a workload-design stage: after this freeze, workload membership,
//!   edits, anchors and identities are closed (POST_HOC_* work cannot
//!   silently mutate them);
//! - not a performance instrument: no latency, allocation, or reuse fact
//!   is reachable from any code path in this crate.

pub mod applicability;
pub mod artifacts;
pub mod coverage;
pub mod dryrun;
pub mod editors;
pub mod fullread;
pub mod registry;
pub mod repair;
pub mod traces;

use serde::{Deserialize, Serialize};

/// Frozen generator identity for this corrective.
pub const CORRECTIVE_C_VERSION: &str = "CORRECTIVE-C-WORKLOAD-FREEZE-v1";
/// Schema tag of the generated transition-registry artifact.
pub const TRANSITION_REGISTRY_SCHEMA: &str = "transition-registry-v1";
/// Schema tag of the generated applicability-matrix artifact.
pub const APPLICABILITY_SCHEMA: &str = "applicability-matrix-v1";
/// Schema tag of the generated FULL_READ manifest.
pub const FULL_READ_SCHEMA: &str = "full-read-v1";
/// Schema tag of the generated trace manifest.
pub const TRACE_MANIFEST_SCHEMA: &str = "trace-manifest-v1";
/// Schema tag of the generated final coverage report.
pub const COVERAGE_REPORT_SCHEMA: &str = "coverage-final-v1";
/// Schema tag of the generated freeze receipt.
pub const FREEZE_RECEIPT_SCHEMA: &str = "freeze-receipt-v1";
/// Schema tag of the generated dry-run report.
pub const DRY_RUN_SCHEMA: &str = "dry-run-report-v1";

/// The frozen lane/authority identifiers this corrective binds to
/// (echoed from the frozen substrate; asserted, never redefined).
pub use markit_mdbench_semantics::{G0_GRAMMAR_ID, G1_GRAMMAR_ID, G2_GRAMMAR_ID};

/// Membership labels used in payload `memberships` and FULL_READ records.
pub const MEMBERSHIP_G0_PRIMARY: &str = "G0_PRIMARY";
pub const MEMBERSHIP_G1_TABLE_SEMANTIC: &str = "G1_TABLE_SEMANTIC";
/// Generator id used when deriving harness `CaseId`s for real payloads.
pub const CASE_GENERATOR_ID: &str = "CORRECTIVE-C-WORKLOAD-FREEZE-v1";

/// One selected physical file, as frozen by CORRECTIVE-B
/// (`workloads/selections/selected-files-v1.json`, schema
/// `selected-files-v1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedFile {
    /// Repository-relative key: `<source_id>/files/<upstream path>`.
    pub key: String,
    /// Acquisition source id (first path segment of `key`).
    pub source_id: String,
    /// Byte-verified source bytes.
    pub text: String,
    /// Frozen sha256 from the selection artifact.
    pub sha256: String,
    pub file_bytes: u64,
    pub memberships: Vec<String>,
    pub g0_strict_eligible: bool,
    pub g1_strict_eligible: bool,
}

/// Load the frozen selected-files artifact and materialize + verify every
/// member against the acquisition sources. Fails closed on any hash
/// mismatch or missing file (G1 SOURCE_AUTHORITY).
///
/// When `workloads/selections/syntax-coverage-repair-v1.json` exists, the
/// deterministic SYNTAX_COVERAGE_SET repair (see [`repair`]) is applied on
/// top, fail-closed; generate / verify / dry-run therefore all observe the
/// same effective file list.
pub fn load_selected_files(benchmark_root: &std::path::Path) -> Result<Vec<SelectedFile>, String> {
    let selection_path = benchmark_root.join("workloads/selections/selected-files-v1.json");
    let raw = std::fs::read_to_string(&selection_path)
        .map_err(|e| format!("read {}: {e}", selection_path.display()))?;
    #[derive(Deserialize)]
    struct SelectionArtifact {
        #[allow(dead_code)]
        schema: String,
        members: std::collections::BTreeMap<String, SelectionMember>,
    }
    #[derive(Deserialize)]
    struct SelectionMember {
        sha256: String,
        file_bytes: u64,
        memberships: Vec<String>,
        g0_strict_eligible: bool,
        g1_strict_eligible: bool,
    }
    let artifact: SelectionArtifact =
        serde_json::from_str(&raw).map_err(|e| format!("parse selected-files-v1.json: {e}"))?;

    let mut files = Vec::new();
    for (key, member) in artifact.members {
        let (source_id, _rest) = key
            .split_once('/')
            .ok_or_else(|| format!("selected key {key:?} does not start with `<source_id>/`"))?;
        // The key IS the path below workloads/sources/: `<source_id>/files/...`
        let path = benchmark_root.join("workloads/sources").join(&key);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("materialized source missing: {}: {e}", path.display()))?;
        let sha = markit_mdbench_semantics::sha256_hex(text.as_bytes());
        if sha != member.sha256 {
            return Err(format!(
                "SOURCE_AUTHORITY failure: {key} bytes hash {sha} != frozen {}",
                member.sha256
            ));
        }
        if text.len() as u64 != member.file_bytes {
            return Err(format!(
                "SOURCE_AUTHORITY failure: {key} bytes {} != frozen {}",
                text.len(),
                member.file_bytes
            ));
        }
        files.push(SelectedFile {
            key: key.clone(),
            source_id: source_id.to_string(),
            text,
            sha256: member.sha256.clone(),
            file_bytes: member.file_bytes,
            memberships: member.memberships,
            g0_strict_eligible: member.g0_strict_eligible,
            g1_strict_eligible: member.g1_strict_eligible,
        });
    }
    // Deterministic order: by repository-relative key.
    files.sort_by(|a, b| a.key.cmp(&b.key));
    // Deterministic SYNTAX_COVERAGE_SET repair overlay (fail-closed).
    if let Some(repair) = repair::load_syntax_coverage_repair(benchmark_root)? {
        files = repair::apply_repair(benchmark_root, files, &repair)?;
    }
    Ok(files)
}

/// Acquisition commit SHA for a source id, read from the frozen
/// `source-lock.json` (schema 2).
pub fn acquisition_commit_sha(
    benchmark_root: &std::path::Path,
    source_id: &str,
) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Lock {
        sources: Vec<LockSource>,
    }
    #[derive(Deserialize)]
    struct LockSource {
        source_id: String,
        commit_sha: String,
    }
    let raw = std::fs::read_to_string(benchmark_root.join("workloads/source-lock.json"))
        .map_err(|e| format!("read source-lock.json: {e}"))?;
    let lock: Lock =
        serde_json::from_str(&raw).map_err(|e| format!("parse source-lock.json: {e}"))?;
    lock.sources
        .iter()
        .find(|s| s.source_id == source_id)
        .map(|s| s.commit_sha.clone())
        .ok_or_else(|| format!("source id {source_id:?} not in source-lock.json"))
}

/// Small deterministic JSONL writer helper (canonical JSON per line).
pub fn write_jsonl(path: &std::path::Path, lines: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    let mut out = String::new();
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    std::fs::write(path, out).map_err(|e| format!("write {}: {e}", path.display()))
}
