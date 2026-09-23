//! Campaign-2 identity derivation (task §7, §50).
//!
//! Rules inherited from Campaign-1 and kept:
//!
//! - every id is `hex(SHA256(<canonical byte string>))` over FROZEN
//!   inputs only — never over a measured value, timestamp, PID, or
//!   wall-clock fact;
//! - `CampaignSpecId` binds the campaign SPECIFICATION only;
//! - `RunId` additionally binds the exact executable bytes, so a rebuilt
//!   binary can never silently continue a run;
//! - no two raw observations may share an `ObservationId`.
//!
//! Campaign-2 additions (task §7, §50):
//!
//! - `StudyId` binds the study definition (authority SHA + the frozen
//!   sub-campaign set);
//! - `SubCampaignSpecId` binds ONE separately-freezable sub-campaign
//!   (a surface, a lane, or one controlled axis), so a partial rerun can
//!   never mix identities;
//! - every raw file carries `StudyId`, `CampaignSpecId`,
//!   `SubCampaignSpecId` and `RunId`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CAMPAIGN2_ID, ENVELOPE_SCHEMA_ID, STUDY_ID};

/// Domain string of the Campaign-2 root seed derivation.
pub const CAMPAIGN2_SEED_DOMAIN: &str = "MARKIT-31-FULL-EVIDENCE-CAMPAIGN-2-SEED-v1";
/// Domain of the per-(surface, session) case-order seed.
pub const SESSION_SEED_DOMAIN: &str = "MARKIT-31-CAMPAIGN-2-SESSION-SEED-v1";
/// Domain of the seeded base horse permutation seed.
pub const HORSE_BASE_SEED_DOMAIN: &str = "MARKIT-31-CAMPAIGN-2-HORSE-PERMUTATION-SEED-v1";
/// Algorithm tag of the 64-bit seed extraction (big-endian, frozen).
pub const SEED_ALGORITHM_ID: &str = "sha256-first64-bigendian-v1";

fn sha256_digest(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex(digest: &[u8; 32]) -> String {
    markit_mdbench_common::to_lower_hex(digest)
}

fn first64_big_endian(digest: &[u8; 32]) -> u64 {
    u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

/// The nine frozen Campaign-2 sub-campaigns (task §49 raw layout).
pub const SUB_CAMPAIGNS: [&str; 9] = [
    "construction",
    "resident_update",
    "lifecycle",
    "controlled_n",
    "controlled_b",
    "controlled_d_fence",
    "controlled_f_reference",
    "controlled_k_container",
    "profiling",
];

/// `StudyId`.
///
/// ```text
/// StudyId = hex(SHA256("MARKIT-31-FULL-EVIDENCE-STUDY\n"
///                     + authority_sha + "\n"
///                     + sub_campaign_1 + "\n" + ... ))
/// ```
///
/// Structural: it changes only if the authority revision or the frozen
/// sub-campaign set changes.
pub fn study_id(authority_sha: &str) -> String {
    let mut material = String::new();
    material.push_str(STUDY_ID);
    material.push('\n');
    material.push_str(authority_sha);
    for sub in SUB_CAMPAIGNS {
        material.push('\n');
        material.push_str(sub);
    }
    hex(&sha256_digest(material.as_bytes()))
}

/// `CampaignSpecId` = `hex(SHA256(canonical JSON of the frozen campaign
/// specification))`. The binding struct's field order IS the canonical
/// order (serde serializes struct fields in declaration order).
pub fn campaign_spec_id(binding: &crate::spec::Campaign2Spec) -> String {
    hex(&sha256_digest(&binding.canonical_bytes()))
}

/// `SubCampaignSpecId` = `hex(SHA256(CampaignSpecId || sub-campaign tag
/// || canonical JSON of the sub-campaign binding))`.
pub fn sub_campaign_spec_id(
    spec_id: &str,
    tag: &str,
    binding: &crate::spec::SubCampaignBinding,
) -> String {
    let mut material = Vec::new();
    material.extend_from_slice(spec_id.as_bytes());
    material.push(b'\n');
    material.extend_from_slice(tag.as_bytes());
    material.push(b'\n');
    material.extend_from_slice(&binding.canonical_bytes());
    hex(&sha256_digest(&material))
}

/// Campaign-2 root seed.
pub fn campaign_seed(authority_sha: &str) -> u64 {
    let material = format!("{CAMPAIGN2_SEED_DOMAIN}\n{CAMPAIGN2_ID}\n{authority_sha}");
    first64_big_endian(&sha256_digest(material.as_bytes()))
}

/// Deterministic per-(sub-campaign, session) case-order seed.
pub fn session_seed(root: u64, tag: &str, session_ordinal: u32) -> u64 {
    let material = format!("{SESSION_SEED_DOMAIN}\n{root:016x}\n{tag}\n{session_ordinal}");
    first64_big_endian(&sha256_digest(material.as_bytes()))
}

/// Seed of the base H0-H4 permutation.
pub fn horse_base_seed(root: u64) -> u64 {
    let material = format!("{HORSE_BASE_SEED_DOMAIN}\n{root:016x}");
    first64_big_endian(&sha256_digest(material.as_bytes()))
}

/// `SessionId` = `hex(SHA256(CampaignSpecId || tag || session || lane))`.
pub fn session_id(spec_id: &str, tag: &str, session_ordinal: u32, lane: &str) -> String {
    let material = format!("{spec_id}\n{tag}\n{session_ordinal}\n{lane}");
    hex(&sha256_digest(material.as_bytes()))
}

/// `RunId` = execution-time binding (task §7, §47). Field order frozen:
/// `StudyId`, `CampaignSpecId`, `SubCampaignSpecId`, runner commit,
/// machine manifest digest, rustc, target, build profile id, Cargo.lock
/// digest, executable SHA256.
pub fn run_id(
    study_id: &str,
    campaign_spec_id: &str,
    sub_campaign_spec_id: &str,
    runner_git_commit: &str,
    machine_manifest_sha256: &str,
    build_identity: &markit_mdbench_runner::BuildIdentityV1,
    executable_sha256: &str,
) -> String {
    let material = format!(
        "{study_id}\n{campaign_spec_id}\n{sub_campaign_spec_id}\n{runner_git_commit}\n\
         {machine_manifest_sha256}\n{}\n{}\n{}\n{}\n{executable_sha256}",
        build_identity.rustc,
        build_identity.target,
        build_identity.build_profile_id,
        build_identity.cargo_lock_sha256
    );
    hex(&sha256_digest(material.as_bytes()))
}

/// `ObservationId` = `hex(SHA256(immutable envelope identity fields))`.
#[allow(clippy::too_many_arguments)]
pub fn observation_id(
    run_id: &str,
    session_id: &str,
    surface: &str,
    case_id: &str,
    horse_id: &str,
    sample_kind: &str,
    iteration_ordinal: u32,
) -> String {
    let material = format!(
        "{run_id}\n{session_id}\n{surface}\n{case_id}\n{horse_id}\n{sample_kind}\n{iteration_ordinal}"
    );
    hex(&sha256_digest(material.as_bytes()))
}

/// SHA256 of a file's bytes as lowercase hex.
pub fn file_sha256(path: &std::path::Path) -> Result<String, String> {
    crate::sha256_file(path)
}

/// Serialized campaign-2 identity block carried by the freeze document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Campaign2Identity {
    pub study_id: String,
    pub campaign_spec_id: String,
    pub campaign_id: String,
    pub authority_sha: String,
    pub envelope_schema_id: String,
    pub seed_algorithm: String,
    pub campaign_seed: u64,
    pub sub_campaigns: Vec<SubCampaignIdentity>,
}

/// One sub-campaign's derived identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubCampaignIdentity {
    pub tag: String,
    pub sub_campaign_spec_id: String,
}

/// Envelope schema tag carried into the spec binding.
pub fn envelope_schema_id() -> &'static str {
    ENVELOPE_SCHEMA_ID
}
