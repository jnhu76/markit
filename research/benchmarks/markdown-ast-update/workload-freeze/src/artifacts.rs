//! Deterministic artifact writers + the freeze receipt.
//!
//! Every artifact is written in canonical JSON (sorted keys, no
//! whitespace, fixed record order), so two clean generation runs are
//! byte-identical (task §52). The freeze receipt pins the artifact
//! digests and the frozen authority identities.

use std::path::Path;

use crate::applicability::{FrozenWorkload, MatrixRow};
use crate::coverage::CoverageReport;
use crate::fullread::FullReadRecord;
use crate::traces::TraceRecord;
use crate::{CORRECTIVE_C_VERSION, FREEZE_RECEIPT_SCHEMA};

/// Canonical JSONL lines for the applicability matrix.
pub fn matrix_lines(rows: &[MatrixRow]) -> Vec<String> {
    rows.iter()
        .map(|row| markit_mdbench_semantics::canonical_json_line(row))
        .collect()
}

/// Canonical JSONL lines for the FULL_READ manifest.
pub fn full_read_lines(records: &[FullReadRecord]) -> Vec<String> {
    records
        .iter()
        .map(|record| markit_mdbench_semantics::canonical_json_line(record))
        .collect()
}

/// Canonical JSONL lines for the EDIT_WRITE manifest.
pub fn payload_lines(payloads: &[markit_mdbench_semantics::payload::PayloadRecord]) -> Vec<String> {
    payloads
        .iter()
        .map(|payload| markit_mdbench_semantics::canonical_json_line(payload))
        .collect()
}

/// Canonical JSONL lines for the trace manifest.
pub fn trace_lines(records: &[TraceRecord]) -> Vec<String> {
    records
        .iter()
        .map(|record| markit_mdbench_semantics::canonical_json_line(record))
        .collect()
}

/// The freeze receipt: artifact digests + counts + frozen identities.
pub fn freeze_receipt(
    benchmark_root: &Path,
    artifact_digests: &[(String, String)],
    workload: &FrozenWorkload,
    full_read: &[FullReadRecord],
    coverage: &CoverageReport,
) -> serde_json::Value {
    let selection_identity: serde_json::Value = {
        let raw = std::fs::read_to_string(
            benchmark_root.join("workloads/selections/selected-files-v1.json"),
        )
        .expect("selection artifact");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("selection JSON");
        value["identity"].clone()
    };
    serde_json::json!({
        "schema": FREEZE_RECEIPT_SCHEMA,
        "generator_version": CORRECTIVE_C_VERSION,
        "authority": {
            "issue": "#35 MARKIT-WORKLOAD-CONSTRUCTION",
            "corrective": "CORRECTIVE-C (task contract, supersedes the oversized draft)",
            "base_master_sha": "3a7d24e1ecffbe3f21c8091ed6dcdb6a5fa22473",
            "selection_identity": selection_identity,
            "lane_registry_version": markit_mdbench_semantics::LANE_REGISTRY_VERSION,
            "transition_oracle_version": markit_mdbench_semantics::TRANSITION_ORACLE_VERSION,
            "predicate_version": markit_mdbench_semantics::PREDICATE_VERSION,
            "payload_lifecycle_version": markit_mdbench_semantics::PAYLOAD_LIFECYCLE_VERSION,
            "transition_registry_version": crate::registry::TRANSITION_REGISTRY_VERSION,
            "profiler_version": markit_mdbench_semantics::PROFILER_VERSION,
        },
        "counts": {
            "final_physical_files": coverage.sources.final_physical_file_count,
            "full_read_records": full_read.len(),
            "edit_write_payloads": workload.payloads.len(),
            "break_restore_pairs": workload.break_restore_reports.len(),
            "applicable_cells": workload
                .rows
                .iter()
                .filter(|row| row.status == crate::applicability::STATUS_APPLICABLE)
                .count(),
            "matrix_rows": workload.rows.len(),
        },
        "artifact_sha256": artifact_digests
            .iter()
            .map(|(name, digest)| serde_json::json!({ "artifact": name, "sha256": digest }))
            .collect::<Vec<_>>(),
        "claim_boundary": "Workload construction freeze candidate for the primary G0 \
            campaign only. No H0-H4 timing was performed by this generator; the dry-run \
            is correctness-only. Broader grammar lanes (G1 non-table, G2 math) remain \
            explicitly deferred.",
    })
}

/// Write a string artifact and return its digest.
pub fn write_artifact(dir: &Path, name: &str, content: &str) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|error| format!("mkdir {}: {error}", dir.display()))?;
    let path = dir.join(name);
    std::fs::write(&path, content).map_err(|error| format!("write {}: {error}", path.display()))?;
    Ok(markit_mdbench_semantics::sha256_hex(content.as_bytes()))
}
