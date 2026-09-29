//! Campaign spec binding + campaign receipt (task §22, §41, §58).
//!
//! The receipt is produced BEFORE timing and SHA256-binds every frozen
//! artifact the campaign consumes. Changing one bound artifact after
//! the freeze invalidates the `CampaignSpecId`.

use serde::{Deserialize, Serialize};

use crate::identity::CampaignSpecBinding;
use crate::manifest::{
    CampaignManifest, CAMPAIGN_MANIFEST_PATH, ENVELOPE_SCHEMA_PATH, MACHINE_MANIFEST_PATH,
    SCHEDULE_MANIFEST_PATH,
};
use crate::{sha256_file, CAMPAIGN_RECEIPT_SCHEMA};

/// Every artifact the campaign receipt SHA256-binds (task §41).
///
/// `Cargo.toml` and `rust-toolchain.toml` are bound because they DEFINE
/// the frozen `release-primary-v1` build profile and the pinned
/// toolchain: binding only `Cargo.lock` would let a post-freeze profile
/// edit (opt-level/lto/codegen-units) pass unnoticed.
pub const BOUND_ARTIFACTS: [&str; 14] = [
    "results/manifests/six-horse-performance-campaign-v1.toml",
    "results/manifests/six-horse-machine-v1.toml",
    "results/manifests/six-horse-schedule-v1.jsonl",
    "protocol/campaign-observation-schema-v1.json",
    "workloads/payloads/freeze-receipt-v1.json",
    "workloads/payloads/full-read-manifest-v1.jsonl",
    "workloads/payloads/edit-write-manifest-v1.jsonl",
    "workloads/payloads/transition-registry-v1.json",
    "workloads/profiles/strict-surface-profile-v1.jsonl",
    "protocol/result-schema-v2.json",
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
    "manifest/environment.toml",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundArtifact {
    pub artifact: String,
    pub sha256: String,
}

/// The campaign freeze receipt (task §41). Produced before timing;
/// never overwritten after finalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignReceipt {
    pub schema: String,
    pub campaign_spec_id: String,
    pub campaign_seed: u64,
    /// The receipt is produced BEFORE any primary timing (task §41).
    pub produced_before_primary_timing: bool,
    pub artifacts: Vec<BoundArtifact>,
}

impl CampaignReceipt {
    pub fn load(benchmark_root: &std::path::Path) -> Result<Self, String> {
        let path = benchmark_root.join(crate::manifest::CAMPAIGN_RECEIPT_PATH);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
    }
}

/// Assemble the campaign spec binding from LIVE repository state (task
/// §22): manifest + machine manifest digest + the frozen constants.
/// Binds policies and digests only — never a measured value.
pub fn build_spec_binding(
    benchmark_root: &std::path::Path,
    manifest: &CampaignManifest,
) -> Result<CampaignSpecBinding, String> {
    let machine_manifest_sha256 = sha256_file(&benchmark_root.join(MACHINE_MANIFEST_PATH))?;
    let envelope_schema_sha256 = sha256_file(&benchmark_root.join(ENVELOPE_SCHEMA_PATH))?;
    let workload_freeze_receipt_sha256 =
        sha256_file(&benchmark_root.join("workloads/payloads/freeze-receipt-v1.json"))?;
    Ok(CampaignSpecBinding {
        campaign_id: manifest.campaign_id.clone(),
        base_authority_sha: manifest.base_authority_sha.clone(),
        workload_freeze_receipt_sha256,
        grammar_authority: crate::G0_GRAMMAR_ID.to_string(),
        result_protocol_version: crate::PROTOCOL_VERSION.to_string(),
        mechanism_set: manifest
            .horses
            .iter()
            .map(|horse| horse.mechanism_id.clone())
            .collect(),
        measurement_corrective_version: crate::MEASUREMENT_CORRECTIVE_VERSION.to_string(),
        result_schema_version: markit_mdbench_runner::RESULT_SCHEMA_VERSION_V2,
        envelope_schema_id: crate::ENVELOPE_SCHEMA_ID.to_string(),
        envelope_schema_sha256,
        session_count: manifest.sessions.count,
        warmup_iterations: manifest.sessions.warmup_iterations,
        measured_iterations: manifest.sessions.measured_iterations,
        campaign_seed: crate::manifest::parse_seed_value(&manifest.seed.value)?,
        campaign_seed_algorithm: crate::identity::CAMPAIGN_SEED_ALGORITHM_ID.to_string(),
        case_order_algorithm: markit_mdbench_runner::SHUFFLE_ALGORITHM_ID.to_string(),
        horse_order_policy: crate::schedule::HORSE_ORDER_POLICY_ID.to_string(),
        session_seed_domain: crate::identity::SESSION_SEED_DOMAIN.to_string(),
        horse_base_seed_domain: crate::identity::HORSE_BASE_SEED_DOMAIN.to_string(),
        build_profile_id: markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID
            .to_string(),
        machine_manifest_sha256,
        metric_qualification: manifest
            .metrics
            .iter()
            .map(|metric| (metric.name.clone(), metric.status.clone()))
            .collect(),
        quantile_policy: vec![
            format!(
                "nearest-rank-n{}-p50rank{}-p95rank{}",
                manifest.quantiles.sample_size,
                manifest.quantiles.p50_rank,
                manifest.quantiles.p95_rank
            ),
            "1-indexed".to_string(),
            "no-interpolation".to_string(),
            "no-pooling-across-sessions".to_string(),
        ],
        aggregation_policy: manifest.aggregation.clone(),
        failure_policy: manifest.failure_policy.clone(),
    })
}

/// Verify the #35 workload freeze receipt's internal artifact hashes
/// (task §58: `WORKLOAD_IDENTITY_CHANGED = NO` must be re-checkable).
pub fn verify_workload_freeze_receipt(benchmark_root: &std::path::Path) -> Result<(), Vec<String>> {
    let payload_dir = benchmark_root.join("workloads/payloads");
    let receipt_path = payload_dir.join("freeze-receipt-v1.json");
    let text = std::fs::read_to_string(&receipt_path)
        .map_err(|e| vec![format!("read {}: {e}", receipt_path.display())])?;
    #[derive(Deserialize)]
    struct FreezeReceipt {
        artifact_sha256: Vec<BoundArtifact>,
    }
    let receipt: FreezeReceipt =
        serde_json::from_str(&text).map_err(|e| vec![format!("parse freeze receipt: {e}")])?;
    let mut blockers = Vec::new();
    for bound in &receipt.artifact_sha256 {
        let digest = sha256_file(&payload_dir.join(&bound.artifact));
        match digest {
            Ok(actual) if actual == bound.sha256 => {}
            Ok(actual) => blockers.push(format!(
                "workload artifact {} hash {actual} != frozen {} (WORKLOAD_IDENTITY_CHANGED)",
                bound.artifact, bound.sha256
            )),
            Err(error) => blockers.push(format!("workload artifact {}: {error}", bound.artifact)),
        }
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

/// Generate the campaign receipt from live state (freeze action; run
/// once, before timing). Requires all bound artifacts to exist.
pub fn generate_receipt(benchmark_root: &std::path::Path) -> Result<CampaignReceipt, String> {
    let manifest = CampaignManifest::load(benchmark_root)?;
    let binding = build_spec_binding(benchmark_root, &manifest)?;
    let spec_id = crate::identity::campaign_spec_id(&binding);
    let mut artifacts = Vec::new();
    for artifact in BOUND_ARTIFACTS {
        let digest = sha256_file(&benchmark_root.join(artifact))?;
        artifacts.push(BoundArtifact {
            artifact: artifact.to_string(),
            sha256: digest,
        });
    }
    Ok(CampaignReceipt {
        schema: CAMPAIGN_RECEIPT_SCHEMA.to_string(),
        campaign_spec_id: spec_id,
        campaign_seed: crate::manifest::parse_seed_value(&manifest.seed.value)?,
        produced_before_primary_timing: true,
        artifacts,
    })
}

/// Path of the reviewed primary-receipt supersession record (below the
/// benchmark root). Absent = no supersession is authorized at all.
pub const RECEIPT_SUPERSESSION_PATH: &str =
    "results/manifests/sensitivity/primary-receipt-supersession-v1.json";

/// The ONLY artifacts a supersession record may EVER name: workspace
/// evolution files the frozen primary receipt cannot anticipate (they
/// define profiles/toolchains for FUTURE authorized work). Everything
/// else the receipt binds — schedule, campaign/machine manifests,
/// workload, schemas, lockfile — is a replication input and is NOT
/// supersedeable; a record naming such an artifact is rejected outright
/// (#33 RQ8 pre-run review P2-1).
pub const SUPERSESSIONABLE_ARTIFACTS: [&str; 2] = ["Cargo.toml", "manifest/environment.toml"];

/// An authorized, reviewed workspace supersession of one or more
/// receipt-bound artifacts (#33 RQ8 second-profile addition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSupersession {
    pub schema: String,
    pub reason: String,
    pub date: String,
    /// artifact path -> the FROZEN RECEIPT hash it superseded (lineage
    /// proof: the record is made against the sealed receipt, never an
    /// arbitrary earlier state).
    pub superseded_artifacts: std::collections::BTreeMap<String, String>,
    /// artifact path -> the hash that now authoritatively replaces it.
    pub replacement_hashes: std::collections::BTreeMap<String, String>,
    pub note: String,
}

impl ReceiptSupersession {
    pub fn load(benchmark_root: &std::path::Path) -> Result<Option<Self>, String> {
        let path = benchmark_root.join(RECEIPT_SUPERSESSION_PATH);
        if !path.exists() {
            return Ok(None);
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let record: ReceiptSupersession =
            serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        if record.schema != "primary-receipt-supersession-v1" {
            return Err(format!(
                "supersession schema {:?} != primary-receipt-supersession-v1",
                record.schema
            ));
        }
        if record.superseded_artifacts.keys().collect::<Vec<_>>()
            != record.replacement_hashes.keys().collect::<Vec<_>>()
        {
            return Err(
                "supersession superseded_artifacts and replacement_hashes must name the same set"
                    .to_string(),
            );
        }
        validate_supersession_scope(&record)?;
        Ok(Some(record))
    }

    /// A record may only supersede workspace-evolution artifacts —
    /// never replication inputs (schedule, manifests, workload,
    /// schemas, lockfile).
    fn validate_supersession_scope(record: &ReceiptSupersession) -> Result<(), String> {
        for artifact in record.superseded_artifacts.keys() {
            if !SUPERSESSIONABLE_ARTIFACTS.contains(&artifact.as_str()) {
                return Err(format!(
                "supersession names {artifact:?}, which is a replication input — only {:?} may ever be superseded",
                SUPERSESSIONABLE_ARTIFACTS
            ));
            }
        }
        Ok(())
    }

    /// Pure acceptance predicate: `live` replaces `receipt_hash` for
    /// `artifact` only when this record names the artifact, quotes the
    /// SAME frozen receipt hash, and pins the exact live replacement.
    pub fn accepts(&self, artifact: &str, receipt_hash: &str, live_hash: &str) -> bool {
        self.superseded_artifacts
            .get(artifact)
            .map(|superseded| superseded == receipt_hash)
            .unwrap_or(false)
            && self
                .replacement_hashes
                .get(artifact)
                .map(|replacement| replacement == live_hash)
                .unwrap_or(false)
    }
}

/// Verify the checked-in campaign receipt (task §47): every bound hash
/// matches, the spec id recomputes, and the bound schedule is exactly
/// the schedule regenerate produces.
///
/// AUTHORIZED WORKSPACE SUPERSESSION (#33 RQ8): a bound artifact whose
/// live hash differs from the receipt is accepted ONLY when a reviewed
/// supersession record (`RECEIPT_SUPERSESSION_PATH`) names that
/// artifact, quotes the artifact's FROZEN RECEIPT hash (lineage proof
/// against the sealed receipt), and pins the exact live replacement.
/// The receipt file itself is never rewritten. Any other drift — or a
/// malformed/stale supersession record — fails closed exactly as
/// before. The one authorized supersession to date is the second
/// frozen profile addition (Cargo.toml + manifest/environment.toml;
/// see protocol/implementation-parity.md "Second frozen profile").
pub fn verify_receipt(benchmark_root: &std::path::Path) -> Result<(), Vec<String>> {
    let mut blockers = Vec::new();
    let receipt = match CampaignReceipt::load(benchmark_root) {
        Ok(receipt) => receipt,
        Err(error) => return Err(vec![error]),
    };
    if receipt.schema != CAMPAIGN_RECEIPT_SCHEMA {
        blockers.push(format!(
            "receipt schema {:?} != {CAMPAIGN_RECEIPT_SCHEMA}",
            receipt.schema
        ));
    }
    if !receipt.produced_before_primary_timing {
        blockers.push("receipt must be produced before primary timing".to_string());
    }
    let supersession = match ReceiptSupersession::load(benchmark_root) {
        Ok(record) => record,
        Err(error) => {
            blockers.push(error);
            None
        }
    };
    for bound in &receipt.artifacts {
        match sha256_file(&benchmark_root.join(&bound.artifact)) {
            Ok(actual) if actual == bound.sha256 => {}
            Ok(actual) => {
                let authorized = supersession
                    .as_ref()
                    .map(|record| record.accepts(&bound.artifact, &bound.sha256, &actual))
                    .unwrap_or(false);
                if !authorized {
                    blockers.push(format!(
                        "bound artifact {} hash {actual} != receipt {} (no authorized supersession)",
                        bound.artifact, bound.sha256
                    ));
                }
            }
            Err(error) => blockers.push(format!("bound artifact {}: {error}", bound.artifact)),
        }
    }
    if let Some(record) = &supersession {
        // A superseded artifact that no longer drifts is a stale record:
        // the supersession must describe the PRESENT authorized state.
        for artifact in record.superseded_artifacts.keys() {
            let frozen = receipt
                .artifacts
                .iter()
                .find(|bound| bound.artifact == *artifact)
                .map(|bound| bound.sha256.as_str());
            match (frozen, sha256_file(&benchmark_root.join(artifact)).ok()) {
                (Some(_), Some(live)) if record.accepts(artifact, frozen.unwrap_or_default(), &live) => {}
                (None, _) => blockers.push(format!(
                    "supersession names {artifact:?}, which the receipt does not bind"
                )),
                _ => blockers.push(format!(
                    "supersession record for {artifact} does not describe the live state — remove or update it via review"
                )),
            }
        }
    }
    let declared: Vec<&str> = receipt
        .artifacts
        .iter()
        .map(|a| a.artifact.as_str())
        .collect();
    for expected in BOUND_ARTIFACTS {
        if !declared.contains(&expected) {
            blockers.push(format!("receipt does not bind {expected}"));
        }
    }
    // Recompute the spec id from live state.
    let manifest = match CampaignManifest::load(benchmark_root) {
        Ok(manifest) => manifest,
        Err(error) => {
            blockers.push(error);
            return Err(blockers);
        }
    };
    match build_spec_binding(benchmark_root, &manifest) {
        Ok(binding) => {
            let recomputed = crate::identity::campaign_spec_id(&binding);
            if recomputed != receipt.campaign_spec_id {
                blockers.push(format!(
                    "campaign spec id {} != recomputed {recomputed} (a bound input changed after freeze)",
                    receipt.campaign_spec_id
                ));
            }
            if binding.campaign_seed != receipt.campaign_seed {
                blockers.push(format!(
                    "campaign seed {} != receipt {}",
                    binding.campaign_seed, receipt.campaign_seed
                ));
            }
        }
        Err(error) => blockers.push(error),
    }
    // The bound schedule must equal a fresh generation byte-for-byte.
    match regenerate_schedule(benchmark_root, &manifest) {
        Ok(expected) => {
            let on_disk = std::fs::read(benchmark_root.join(SCHEDULE_MANIFEST_PATH))
                .map_err(|e| format!("read schedule: {e}"));
            match on_disk {
                Ok(bytes) => {
                    if bytes != expected {
                        blockers.push(
                            "schedule on disk differs from deterministic regeneration".to_string(),
                        );
                    }
                }
                Err(error) => blockers.push(error),
            }
        }
        Err(error) => blockers.push(format!("schedule regeneration: {error}")),
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

/// Deterministically regenerate the schedule bytes from the frozen
/// inputs (manifest + consumed workload). Running this twice must be
/// byte-identical (task §48).
pub fn regenerate_schedule(
    benchmark_root: &std::path::Path,
    manifest: &CampaignManifest,
) -> Result<Vec<u8>, String> {
    let workload = crate::workload::load_campaign_workload(benchmark_root)?;
    let binding = build_spec_binding(benchmark_root, manifest)?;
    let spec_id = crate::identity::campaign_spec_id(&binding);
    let inputs = crate::workload::schedule_inputs(&workload);
    let rows = crate::schedule::generate_schedule(
        &spec_id,
        manifest.sessions.count,
        crate::manifest::parse_seed_value(&manifest.seed.value)?,
        &inputs,
    )?;
    crate::schedule::schedule_to_jsonl(&rows)
}

/// Convenience: verify the on-disk schedule (used by preflight and the
/// schedule-verify subcommand).
pub fn verify_schedule_on_disk(benchmark_root: &std::path::Path) -> Result<(), Vec<String>> {
    let manifest = match CampaignManifest::load(benchmark_root) {
        Ok(manifest) => manifest,
        Err(error) => return Err(vec![error]),
    };
    let bytes = match std::fs::read(benchmark_root.join(SCHEDULE_MANIFEST_PATH)) {
        Ok(bytes) => bytes,
        Err(e) => return Err(vec![format!("read schedule: {e}")]),
    };
    let rows = match crate::schedule::schedule_from_jsonl(&bytes) {
        Ok(rows) => rows,
        Err(error) => return Err(vec![error]),
    };
    let workload = match crate::workload::load_campaign_workload(benchmark_root) {
        Ok(workload) => workload,
        Err(error) => return Err(vec![error]),
    };
    let binding = match build_spec_binding(benchmark_root, &manifest) {
        Ok(binding) => binding,
        Err(error) => return Err(vec![error]),
    };
    let spec_id = crate::identity::campaign_spec_id(&binding);
    let inputs = crate::workload::schedule_inputs(&workload);
    let seed = match crate::manifest::parse_seed_value(&manifest.seed.value) {
        Ok(seed) => seed,
        Err(error) => return Err(vec![error]),
    };
    crate::schedule::verify_schedule(&rows, &spec_id, manifest.sessions.count, seed, &inputs)
}

/// The canonical path of the campaign manifest (re-exported for the
/// CLI).
pub fn campaign_manifest_path() -> &'static str {
    CAMPAIGN_MANIFEST_PATH
}

#[cfg(test)]
mod supersession_tests {
    use super::ReceiptSupersession;
    use std::collections::BTreeMap;

    fn record(superseded: &[(&str, &str)], replacement: &[(&str, &str)]) -> ReceiptSupersession {
        ReceiptSupersession {
            schema: "primary-receipt-supersession-v1".to_string(),
            reason: "test".to_string(),
            date: "2026-09-29".to_string(),
            superseded_artifacts: BTreeMap::from_iter(
                superseded
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            ),
            replacement_hashes: BTreeMap::from_iter(
                replacement
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            ),
            note: String::new(),
        }
    }

    #[test]
    fn supersession_never_authorizes_replication_inputs() {
        let mut rec = record(
            &[("Cargo.toml", "frozen-sha")],
            &[("Cargo.toml", "new-sha")],
        );
        // A record that also names the frozen schedule is rejected even
        // with perfect lineage: replication inputs are not supersedeable.
        rec.superseded_artifacts.insert(
            "results/manifests/six-horse-schedule-v1.jsonl".to_string(),
            "x".to_string(),
        );
        rec.replacement_hashes.insert(
            "results/manifests/six-horse-schedule-v1.jsonl".to_string(),
            "y".to_string(),
        );
        assert!(validate_supersession_scope(&rec).is_err());
        let ok = record(&[("Cargo.toml", "f")], &[("Cargo.toml", "n")]);
        assert!(validate_supersession_scope(&ok).is_ok());
    }

    #[test]
    fn supersession_accepts_only_exact_lineage_and_replacement() {
        let rec = record(
            &[("Cargo.toml", "frozen-sha")],
            &[("Cargo.toml", "new-sha")],
        );
        // Exact lineage + exact replacement: accepted.
        assert!(rec.accepts("Cargo.toml", "frozen-sha", "new-sha"));
        // Wrong lineage (the record must quote the RECEIPT's hash).
        assert!(!rec.accepts("Cargo.toml", "other-old", "new-sha"));
        // Wrong replacement (live drift beyond the reviewed record).
        assert!(!rec.accepts("Cargo.toml", "frozen-sha", "drifted-sha"));
        // Artifact not named by the record.
        assert!(!rec.accepts("Cargo.lock", "frozen-sha", "new-sha"));
    }
}
