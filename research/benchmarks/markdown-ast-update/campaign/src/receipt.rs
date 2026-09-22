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
    "results/manifests/primary-performance-campaign-v1.toml",
    "results/manifests/primary-machine-v1.toml",
    "results/manifests/primary-schedule-v1.jsonl",
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
        campaign_seed: manifest.seed.value,
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
        campaign_seed: manifest.seed.value,
        produced_before_primary_timing: true,
        artifacts,
    })
}

/// Verify the checked-in campaign receipt (task §47): every bound hash
/// matches, the spec id recomputes, and the bound schedule is exactly
/// the schedule regenerate produces.
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
    for bound in &receipt.artifacts {
        match sha256_file(&benchmark_root.join(&bound.artifact)) {
            Ok(actual) if actual == bound.sha256 => {}
            Ok(actual) => blockers.push(format!(
                "bound artifact {} hash {actual} != receipt {}",
                bound.artifact, bound.sha256
            )),
            Err(error) => blockers.push(format!("bound artifact {}: {error}", bound.artifact)),
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
        manifest.seed.value,
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
    crate::schedule::verify_schedule(
        &rows,
        &spec_id,
        manifest.sessions.count,
        manifest.seed.value,
        &inputs,
    )
}

/// The canonical path of the campaign manifest (re-exported for the
/// CLI).
pub fn campaign_manifest_path() -> &'static str {
    CAMPAIGN_MANIFEST_PATH
}
