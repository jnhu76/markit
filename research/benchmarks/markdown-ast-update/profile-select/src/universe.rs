//! Candidate-universe loading and materialization verification (§8-§9).
//!
//! The input universe is exactly the merged PR #34 selected snapshot. This
//! module never adds, removes, rewrites or truncates candidates; it only
//! checks that the locally materialized bytes match the frozen authority
//! and records the outcome.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{sha256_hex, CandidateIdentity, VERIFICATION_SCHEMA};

/// One `sources/<id>/SOURCE.json` file entry.
#[derive(Debug, Clone, Deserialize)]
struct SourceFileEntry {
    snapshot_path: String,
    #[serde(default)]
    upstream_path: Option<String>,
    sha256: String,
    bytes: u64,
    #[serde(default)]
    git_blob_sha1: Option<String>,
}

/// One `sources/<id>/SOURCE.json`.
#[derive(Debug, Clone, Deserialize)]
struct SourceManifest {
    source_id: String,
    #[serde(default)]
    files: Vec<SourceFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationEntry {
    pub source_id: String,
    pub path: String,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
    pub expected_bytes: u64,
    pub actual_bytes: Option<u64>,
    pub materialized: bool,
    pub hash_match: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationSummary {
    pub expected_files: u64,
    pub materialized_files: u64,
    pub missing_files: u64,
    pub hash_mismatches: u64,
    pub byte_total_expected: u64,
    pub byte_total_materialized: u64,
    pub per_source: BTreeMap<String, PerSourceVerification>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PerSourceVerification {
    pub expected: u64,
    pub materialized: u64,
    pub missing: u64,
    pub hash_mismatches: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationArtifact {
    pub schema: String,
    pub campaign_id: String,
    pub acquisition_commit_note: String,
    pub entries: Vec<VerificationEntry>,
    pub summary: VerificationSummary,
}

/// Load the frozen per-source manifests from `workloads/sources/*/SOURCE.json`.
///
/// The candidate list is derived from the SOURCE manifests themselves (the
/// acquisition authority), and cross-checked against the derived
/// `candidate-universe-v1.json` source set: every source in the derived
/// manifest must have a SOURCE.json, and no extra SOURCE.json may exist.
pub fn load_identities(
    workloads_root: &Path,
) -> Result<(Vec<CandidateIdentity>, BTreeMap<String, u64>), String> {
    let universe: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(workloads_root.join("manifests/candidate-universe-v1.json"))
            .map_err(|error| format!("candidate-universe-v1.json: {error}"))?,
    )
    .map_err(|error| format!("candidate-universe-v1.json: {error}"))?;

    let campaign_id = universe["campaign_id"]
        .as_str()
        .unwrap_or("MARKIT-REAL-WORKLOAD-ACQUISITION-1")
        .to_string();
    if campaign_id != "MARKIT-REAL-WORKLOAD-ACQUISITION-1" {
        return Err(format!("unexpected campaign_id {campaign_id}"));
    }

    let mut declared: BTreeMap<String, ()> = BTreeMap::new();
    for source in universe["candidates"].as_array().unwrap_or(&Vec::new()) {
        if let Some(source_id) = source["source_id"].as_str() {
            declared.insert(source_id.to_string(), ());
        }
    }

    let domains = crate::domains::load_mapping(workloads_root)?;

    let sources_dir = workloads_root.join("sources");
    let mut source_ids: Vec<String> = Vec::new();
    for entry in fs::read_dir(&sources_dir).map_err(|error| format!("sources/: {error}"))? {
        let entry = entry.map_err(|error| format!("sources/: {error}"))?;
        if entry.path().join("SOURCE.json").is_file() {
            source_ids.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    source_ids.sort();

    for source_id in &source_ids {
        if !declared.contains_key(source_id) {
            return Err(format!(
                "SOURCE.json exists for undeclared source {source_id} (not in candidate-universe)"
            ));
        }
    }
    for source_id in declared.keys() {
        if !source_ids.contains(source_id) {
            return Err(format!(
                "declared source {source_id} has no sources/{source_id}/SOURCE.json"
            ));
        }
    }

    let mut identities = Vec::new();
    let mut per_source_counts: BTreeMap<String, u64> = BTreeMap::new();
    for source_id in &source_ids {
        let path = sources_dir.join(source_id).join("SOURCE.json");
        let manifest: SourceManifest = serde_json::from_str(
            &fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?,
        )
        .map_err(|error| format!("{}: {error}", path.display()))?;
        if manifest.source_id != *source_id {
            return Err(format!(
                "{}: source_id mismatch ({})",
                path.display(),
                manifest.source_id
            ));
        }
        let count = manifest.files.len() as u64;
        per_source_counts.insert(source_id.clone(), count);
        for file in manifest.files {
            let domain = domains
                .stratum_for(source_id)
                .ok_or_else(|| format!("no domain stratum for source {source_id}"))?
                .to_string();
            identities.push(CandidateIdentity {
                source_id: source_id.clone(),
                snapshot_path: file.snapshot_path,
                upstream_path: file.upstream_path.unwrap_or_default(),
                sha256: file.sha256,
                bytes: file.bytes,
                git_blob_sha1: file.git_blob_sha1.unwrap_or_default(),
                domain,
            });
        }
    }

    // Deterministic global order: lexical (source_id, snapshot_path).
    identities
        .sort_by(|a, b| (&a.source_id, &a.snapshot_path).cmp(&(&b.source_id, &b.snapshot_path)));
    Ok((identities, per_source_counts))
}

/// Materialized byte location for one candidate:
/// `workloads/sources/<id>/<snapshot_path>`.
pub fn materialized_path(workloads_root: &Path, identity: &CandidateIdentity) -> PathBuf {
    workloads_root
        .join("sources")
        .join(&identity.source_id)
        .join(&identity.snapshot_path)
}

/// Verify every candidate's materialized bytes against the frozen
/// manifest identity. Any hash mismatch is a hard failure (§9).
pub fn verify_universe(
    workloads_root: &Path,
    identities: &[CandidateIdentity],
) -> Result<VerificationArtifact, String> {
    let mut entries = Vec::with_capacity(identities.len());
    let mut per_source: BTreeMap<String, PerSourceVerification> = BTreeMap::new();

    for identity in identities {
        let path = materialized_path(workloads_root, identity);
        let mut entry = VerificationEntry {
            source_id: identity.source_id.clone(),
            path: identity.snapshot_path.clone(),
            expected_sha256: identity.sha256.clone(),
            actual_sha256: None,
            expected_bytes: identity.bytes,
            actual_bytes: None,
            materialized: false,
            hash_match: false,
        };
        match fs::read(&path) {
            Ok(bytes) => {
                entry.materialized = true;
                entry.actual_bytes = Some(bytes.len() as u64);
                let actual = sha256_hex(&bytes);
                entry.hash_match = actual == identity.sha256;
                entry.actual_sha256 = Some(actual);
                if bytes.len() as u64 != identity.bytes {
                    entry.hash_match = false;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("{}: {error}", path.display()));
            }
        }
        let stats = per_source.entry(identity.source_id.clone()).or_default();
        stats.expected += 1;
        stats.bytes += identity.bytes;
        if entry.materialized {
            stats.materialized += 1;
        } else {
            stats.missing += 1;
        }
        if entry.materialized && !entry.hash_match {
            stats.hash_mismatches += 1;
        }
        entries.push(entry);
    }

    let summary = VerificationSummary {
        expected_files: entries.len() as u64,
        materialized_files: entries.iter().filter(|entry| entry.materialized).count() as u64,
        missing_files: entries.iter().filter(|entry| !entry.materialized).count() as u64,
        hash_mismatches: entries
            .iter()
            .filter(|entry| entry.materialized && !entry.hash_match)
            .count() as u64,
        byte_total_expected: identities.iter().map(|identity| identity.bytes).sum(),
        byte_total_materialized: entries.iter().filter_map(|entry| entry.actual_bytes).sum(),
        per_source,
    };

    Ok(VerificationArtifact {
        schema: VERIFICATION_SCHEMA.to_string(),
        campaign_id: "MARKIT-REAL-WORKLOAD-ACQUISITION-1".to_string(),
        acquisition_commit_note: "input universe = merged PR #34 selected snapshot (3,970 files / 66,391,919 bytes / 20 sources); this verification is CORRECTIVE-B §9".to_string(),
        entries,
        summary,
    })
}
