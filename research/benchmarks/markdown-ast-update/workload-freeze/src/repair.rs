//! SYNTAX_COVERAGE_SET deterministic repair (CORRECTIVE-C, PR #39).
//!
//! The A6 applicability matrix found one required BREAK-side transition
//! (`G0-BQ-NEST-LINE`, E3_CONTAINER_DEPTH) with no applicable cell in any
//! of the 36 selected files. Under the Workload Construction Algorithm v1
//! Stage-A section 8.2 stop rule, the remediation is a deterministic
//! extension of SYNTAX_COVERAGE_SET ONLY (never another set, never a
//! selected-file replacement, never a candidate-universe change): add the
//! lexicographically-first G0-strict-clean universe candidate that
//! carries real blockquote anchors.
//!
//! The decision itself is FROZEN in the committed artifact
//! `workloads/selections/syntax-coverage-repair-v1.json` (evidence rows
//! from the committed CORRECTIVE-B profile artifact, tie-break trace,
//! selection identity digests). This module only APPLIES it, fail-closed:
//!
//! - the base selection identity must still match the repair's recorded
//!   identity (otherwise the repair's basis is gone);
//! - the added file's materialized bytes must hash to the sha256 the
//!   repair recorded from the universe evidence;
//! - the added file must not already be selected.

/// Schema tag of the committed repair artifact.
pub const SYNTAX_COVERAGE_REPAIR_SCHEMA: &str = "syntax-coverage-repair-v1";

use crate::SelectedFile;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RepairCandidate {
    pub key: String,
    #[allow(dead_code)]
    pub source_id: String,
    #[allow(dead_code)]
    pub domain: String,
    pub sha256: String,
    #[allow(dead_code)]
    pub file_bytes: u64,
    #[allow(dead_code)]
    pub g0_block_quote_anchors: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RepairBasis {
    #[allow(dead_code)]
    pub uncovered_cell: serde_json::Value,
    #[serde(default)]
    selection_identity: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RepairArtifact {
    #[allow(dead_code)]
    pub schema: String,
    pub basis: RepairBasis,
    pub evaluated_candidates: Vec<RepairCandidate>,
    pub selected_key: String,
    pub membership_added: String,
}

/// Load the committed repair artifact; `Ok(None)` when no repair is
/// recorded (the plain frozen selection then applies unchanged).
pub fn load_syntax_coverage_repair(
    benchmark_root: &std::path::Path,
) -> Result<Option<RepairArtifact>, String> {
    let path = benchmark_root.join("workloads/selections/syntax-coverage-repair-v1.json");
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read {}: {error}", path.display())),
    };
    let artifact: RepairArtifact = serde_json::from_str(&raw)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    if artifact.schema != SYNTAX_COVERAGE_REPAIR_SCHEMA {
        return Err(format!(
            "repair artifact schema {} != {}",
            artifact.schema, SYNTAX_COVERAGE_REPAIR_SCHEMA
        ));
    }
    Ok(Some(artifact))
}

/// Apply the repair overlay to the loaded selected files, fail-closed.
pub fn apply_repair(
    benchmark_root: &std::path::Path,
    mut files: Vec<SelectedFile>,
    repair: &RepairArtifact,
) -> Result<Vec<SelectedFile>, String> {
    // The repair's basis is the frozen selection identity; fail closed
    // when the base selection no longer matches it.
    let selection_path = benchmark_root
        .join("workloads/selections/selected-files-v1.json");
    let raw = std::fs::read_to_string(&selection_path)
        .map_err(|error| format!("read selected-files-v1.json: {error}"))?;
    let base: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("parse selected-files-v1.json: {error}"))?;
    let base_identity = base
        .get("identity")
        .and_then(|value| value.as_object())
        .ok_or("selected-files-v1.json has no identity object")?;
    for (field, recorded) in &repair.basis.selection_identity {
        let current = base_identity
            .get(field)
            .ok_or_else(|| format!("repair identity field {field} missing from selection"))?;
        if current != recorded {
            return Err(format!(
                "REPAIR_AUTHORITY failure: selection identity field {field} changed \
                 since the repair was recorded ({current} != {recorded})"
            ));
        }
    }

    if files
        .iter()
        .any(|file| file.key == repair.selected_key)
    {
        return Err(format!(
            "repair selected key {} is already selected",
            repair.selected_key
        ));
    }
    // The repair may ONLY extend SYNTAX_COVERAGE_SET. A tampered or
    // mis-recorded membership label that would silently re-label the
    // added file into any other logical set is a hard error (review
    // Q29: this value is otherwise applied verbatim).
    if repair.membership_added != "syntax_coverage" {
        return Err(format!(
            "REPAIR_AUTHORITY failure: membership_added {:?} != \"syntax_coverage\"; \
             the repair may only extend SYNTAX_COVERAGE_SET",
            repair.membership_added
        ));
    }
    let candidate = repair
        .evaluated_candidates
        .iter()
        .find(|candidate| candidate.key == repair.selected_key)
        .ok_or_else(|| "repair selected_key missing from evaluated_candidates".to_string())?;

    let (source_id, _rest) = repair.selected_key.split_once('/').ok_or_else(|| {
        format!(
            "repair key {:?} does not start with `<source_id>/`",
            repair.selected_key
        )
    })?;
    let path = benchmark_root
        .join("workloads/sources")
        .join(&repair.selected_key);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("repair source missing: {}: {error}", path.display()))?;
    let sha = markit_mdbench_semantics::sha256_hex(text.as_bytes());
    if sha != candidate.sha256 {
        return Err(format!(
            "REPAIR_AUTHORITY failure: {} bytes hash {sha} != universe evidence {}",
            repair.selected_key, candidate.sha256
        ));
    }

    let file_bytes = text.as_bytes().len() as u64;
    files.push(SelectedFile {
        key: repair.selected_key.clone(),
        source_id: source_id.to_string(),
        text,
        sha256: candidate.sha256.clone(),
        file_bytes,
        memberships: vec![repair.membership_added.clone()],
        g0_strict_eligible: true,
        g1_strict_eligible: true,
    });
    files.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(files)
}
