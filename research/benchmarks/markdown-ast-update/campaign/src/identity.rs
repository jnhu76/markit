//! Campaign identity: seed derivation and the four stable ids (task §13,
//! §22).
//!
//! Design rules (frozen BEFORE any timing):
//!
//! - every id is `hex(SHA256(<canonical byte string>))` over frozen
//!   inputs only — never over a measured value, timestamp, PID, or
//!   wall-clock fact;
//! - byte order for 64-bit seed extraction is EXPLICIT and frozen:
//!   **big-endian** (the first eight bytes of the SHA256 digest read as a
//!   big-endian u64);
//! - `CampaignSpecId` binds the campaign SPECIFICATION only. It must NOT
//!   depend on measured values (task §22), so the binding struct below
//!   carries policies and digests, never observations;
//! - `RunId` is the execution-time binding (approved runner commit +
//!   machine manifest digest + build identity + the SHA256 of the exact
//!   executable producing the rows); all sessions of one primary
//!   campaign share the same RunId inputs;
//! - `ObservationId` is derived from immutable envelope identity fields;
//!   no two raw observations may share one (enforced by schedule
//!   verification and preflight).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CAMPAIGN_ID, ENVELOPE_SCHEMA_ID, MEASUREMENT_CORRECTIVE_VERSION};

/// Domain + algorithm id of the campaign seed derivation (task §13).
pub const CAMPAIGN_SEED_ALGORITHM_ID: &str = "sha256-first64-bigendian-v1";

/// Domain of the per-(surface, session) case-order seed derivation.
pub const SESSION_SEED_DOMAIN: &str = "MARKIT-31-CAMPAIGN-SESSION-SEED-v1";

/// Domain of the seeded base horse permutation seed (task §15).
pub const HORSE_BASE_SEED_DOMAIN: &str = "MARKIT-31-CAMPAIGN-HORSE-PERMUTATION-SEED-v1";

/// Derivation string frozen by task §13:
///
/// ```text
/// campaign_seed = first 64 bits (big-endian) of
///     SHA256("MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-v1\n" + base_authority_sha)
/// ```
///
/// where `base_authority_sha` is the campaign base authority merge SHA
/// (PR #41 MEASUREMENT-SUBSTRATE-CORRECTIVE merge,
/// `911788a2ad5fcb5b939d8bc3576d3aea77f7df62`, or a descendant recorded
/// in the campaign manifest). The seed is chosen before any timing
/// exists and is never regenerated after results are seen.
pub fn campaign_seed(base_authority_sha: &str) -> u64 {
    let mut material = String::with_capacity(CAMPAIGN_ID.len() + 1 + base_authority_sha.len());
    material.push_str(CAMPAIGN_ID);
    material.push('\n');
    material.push_str(base_authority_sha);
    first64_big_endian(&sha256_digest(material.as_bytes()))
}

/// Session case-order seed: distinct deterministic derivation per
/// `surface × session_ordinal` from the campaign root seed (task §14).
pub fn session_seed(root: u64, surface: &str, session_ordinal: u32) -> u64 {
    let material = format!("{SESSION_SEED_DOMAIN}\n{root:016x}\n{surface}\n{session_ordinal}");
    first64_big_endian(&sha256_digest(material.as_bytes()))
}

/// Seed of the base H0-H4 permutation (task §15 step 1).
pub fn horse_base_seed(root: u64) -> u64 {
    let material = format!("{HORSE_BASE_SEED_DOMAIN}\n{root:016x}");
    first64_big_endian(&sha256_digest(material.as_bytes()))
}

fn sha256_digest(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn first64_big_endian(digest: &[u8; 32]) -> u64 {
    // Explicitly documented frozen byte order: big-endian.
    u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

/// Canonical-JSON-ish binding of the campaign specification. Field order
/// is the declaration order (serde serializes struct fields in order),
/// so the SHA256 over the serialized form is deterministic.
///
/// Binds exactly the task §22 list: workload freeze receipt digest,
/// grammar/result authority, mechanism set, measurement-corrective
/// version, result schema version, sampling policy, campaign seed, order
/// algorithms, build profile, machine manifest digest, metric
/// qualification, quantile policy, aggregation policy, failure policy,
/// raw-envelope schema. It must NOT depend on measured values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignSpecBinding {
    pub campaign_id: String,
    pub base_authority_sha: String,
    pub workload_freeze_receipt_sha256: String,
    pub grammar_authority: String,
    pub result_protocol_version: String,
    pub mechanism_set: Vec<String>,
    pub measurement_corrective_version: String,
    pub result_schema_version: u16,
    pub envelope_schema_id: String,
    pub envelope_schema_sha256: String,
    pub session_count: u32,
    pub warmup_iterations: u32,
    pub measured_iterations: u32,
    pub campaign_seed: u64,
    pub campaign_seed_algorithm: String,
    pub case_order_algorithm: String,
    pub horse_order_policy: String,
    pub session_seed_domain: String,
    pub horse_base_seed_domain: String,
    pub build_profile_id: String,
    pub machine_manifest_sha256: String,
    pub metric_qualification: Vec<(String, String)>,
    pub quantile_policy: Vec<String>,
    pub aggregation_policy: Vec<String>,
    pub failure_policy: Vec<String>,
}

impl CampaignSpecBinding {
    /// Canonical serialized form (stable field order, no whitespace).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("campaign spec binding is JSON-serializable")
    }
}

/// `CampaignSpecId` = `hex(SHA256(canonical_bytes(binding)))`.
pub fn campaign_spec_id(binding: &CampaignSpecBinding) -> String {
    let digest = sha256_digest(&binding.canonical_bytes());
    markit_mdbench_common::to_lower_hex(&digest)
}

/// `SessionId` = `hex(SHA256(CampaignSpecId || surface || session_ordinal))`
/// over the canonical line form below (attribution rows use the literal
/// surface tag + `"attribution"`; see `execute`).
pub fn session_id(spec_id: &str, surface: &str, session_ordinal: u32) -> String {
    let material = format!("{spec_id}\n{surface}\n{session_ordinal}");
    markit_mdbench_common::to_lower_hex(&sha256_digest(material.as_bytes()))
}

/// `RunId` = execution-time binding of spec + approved runner commit +
/// machine manifest digest + build identity + the SHA256 of the exact
/// executable that produces the rows (task §22, §47).
///
/// Field order (frozen): `CampaignSpecId`, runner commit, machine
/// manifest digest, rustc, target, build profile id, `Cargo.lock`
/// digest, executable SHA256.
///
/// The executable digest is a BINDING, not a record: R7 §12 requires all
/// sessions of one primary campaign to run the same binary, and the
/// build identity alone cannot prove that (an identical commit /
/// toolchain / profile / lockfile can rebuild to different bytes). A
/// rebuilt binary therefore changes the `RunId` and can never silently
/// continue an existing campaign run.
pub fn run_id(
    spec_id: &str,
    runner_git_commit: &str,
    machine_manifest_sha256: &str,
    build_identity: &markit_mdbench_runner::BuildIdentityV1,
    executable_sha256: &str,
) -> String {
    let material = format!(
        "{spec_id}\n{runner_git_commit}\n{machine_manifest_sha256}\n{}\n{}\n{}\n{}\n{executable_sha256}",
        build_identity.rustc,
        build_identity.target,
        build_identity.build_profile_id,
        build_identity.cargo_lock_sha256
    );
    markit_mdbench_common::to_lower_hex(&sha256_digest(material.as_bytes()))
}

/// SHA256 (lowercase hex) of the currently running executable.
///
/// Fails closed: a primary session must never collect data under an
/// unknown binary identity, so an unresolvable or unreadable
/// `current_exe()` is an error, never `"unknown"`.
pub fn current_executable_sha256() -> Result<String, String> {
    let path = std::env::current_exe()
        .map_err(|e| format!("resolve current executable for the RunId binding: {e}"))?;
    crate::sha256_file(&path)
        .map_err(|e| format!("hash current executable {}: {e}", path.display()))
}

/// `ObservationId` = `hex(SHA256(canonical line of immutable envelope
/// identity fields))`. No two raw observations may share one.
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
    markit_mdbench_common::to_lower_hex(&sha256_digest(material.as_bytes()))
}

/// Convenience: the current measurement-corrective tag carried into the
/// spec binding (kept next to the constant so they cannot drift).
pub fn measurement_corrective_version() -> &'static str {
    MEASUREMENT_CORRECTIVE_VERSION
}

/// Envelope schema tag carried into the spec binding.
pub fn envelope_schema_id() -> &'static str {
    ENVELOPE_SCHEMA_ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_seed_is_documented_derivation() {
        // SHA256("MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-v1\n911788a...")
        // first 64 bits big-endian. Pinned by recomputation, not by a
        // magic constant: this test proves the derivation is stable for
        // the frozen base authority SHA.
        let a = campaign_seed("911788a2ad5fcb5b939d8bc3576d3aea77f7df62");
        let b = campaign_seed("911788a2ad5fcb5b939d8bc3576d3aea77f7df62");
        assert_eq!(a, b);
        // Independent recomputation through the raw primitives.
        let material =
            "MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-v1\n911788a2ad5fcb5b939d8bc3576d3aea77f7df62";
        let digest = sha256_digest(material.as_bytes());
        assert_eq!(a, first64_big_endian(&digest));
        // A different base authority must move the seed.
        assert_ne!(a, campaign_seed("0000000000000000000000000000000000000000"));
    }

    #[test]
    fn session_and_horse_seeds_are_distinct_per_inputs() {
        let root = campaign_seed("911788a2ad5fcb5b939d8bc3576d3aea77f7df62");
        let s0 = session_seed(root, "edit_write", 0);
        let s1 = session_seed(root, "edit_write", 1);
        let s0c = session_seed(root, "clean_state", 0);
        assert_ne!(s0, s1);
        assert_ne!(s0, s0c);
        assert_eq!(s0, session_seed(root, "edit_write", 0));
        let h = horse_base_seed(root);
        assert_ne!(h, s0);
        assert_ne!(h, s1);
        assert_ne!(h, s0c);
    }

    #[test]
    fn observation_ids_are_unique_per_immutable_fields() {
        let base = observation_id("r", "s", "edit_write", "c", "H1", "measured", 0);
        assert_eq!(
            base,
            observation_id("r", "s", "edit_write", "c", "H1", "measured", 0)
        );
        assert_ne!(
            base,
            observation_id("r", "s", "edit_write", "c", "H1", "measured", 1)
        );
        assert_ne!(
            base,
            observation_id("r", "s", "edit_write", "c", "H1", "warmup", 0)
        );
        assert_ne!(
            base,
            observation_id("r", "s", "edit_write", "c", "H2", "measured", 0)
        );
        assert_ne!(
            base,
            observation_id("r", "s", "edit_write", "d", "H1", "measured", 0)
        );
        assert_ne!(
            base,
            observation_id("r", "t", "edit_write", "c", "H1", "measured", 0)
        );
    }

    #[test]
    fn spec_id_is_deterministic_and_sensitive_to_binding() {
        let binding = sample_binding();
        assert_eq!(campaign_spec_id(&binding), campaign_spec_id(&binding));
        let mut other = binding.clone();
        other.campaign_seed += 1;
        assert_ne!(campaign_spec_id(&binding), campaign_spec_id(&other));
        let mut other = binding.clone();
        other.metric_qualification[0].1 = "UNAVAILABLE".to_string();
        assert_ne!(campaign_spec_id(&binding), campaign_spec_id(&other));
    }

    #[test]
    fn run_id_is_deterministic_and_binds_the_executable_digest() {
        let build = markit_mdbench_runner::current_build_identity();
        let base = run_id("spec", "commit", "machine", &build, &"ab".repeat(32));
        assert_eq!(
            base,
            run_id("spec", "commit", "machine", &build, &"ab".repeat(32))
        );
        // A rebuilt binary (same commit/toolchain/profile/lockfile,
        // different bytes) MUST move the RunId: it is a binding, not a
        // record.
        assert_ne!(
            base,
            run_id("spec", "commit", "machine", &build, &"cd".repeat(32))
        );
        // Prior inputs stay binding too.
        assert_ne!(
            base,
            run_id("other", "commit", "machine", &build, &"ab".repeat(32))
        );
        assert_ne!(
            base,
            run_id("spec", "other", "machine", &build, &"ab".repeat(32))
        );
        assert_ne!(
            base,
            run_id("spec", "commit", "other", &build, &"ab".repeat(32))
        );
        let mut other_build = build.clone();
        other_build.rustc = "rustc-other".to_string();
        assert_ne!(
            base,
            run_id("spec", "commit", "machine", &other_build, &"ab".repeat(32))
        );
    }

    #[test]
    fn current_executable_digest_is_available_and_stable() {
        let first = current_executable_sha256().expect("test binary is resolvable");
        assert_eq!(first.len(), 64);
        assert_eq!(first, current_executable_sha256().unwrap());
    }

    fn sample_binding() -> CampaignSpecBinding {
        CampaignSpecBinding {
            campaign_id: crate::CAMPAIGN_ID.to_string(),
            base_authority_sha: "911788a2ad5fcb5b939d8bc3576d3aea77f7df62".to_string(),
            workload_freeze_receipt_sha256: "ab".repeat(32),
            grammar_authority: crate::G0_GRAMMAR_ID.to_string(),
            result_protocol_version: crate::PROTOCOL_VERSION.to_string(),
            mechanism_set: crate::HORSE_ROSTER
                .iter()
                .map(|h| h.mechanism_id.to_string())
                .collect(),
            measurement_corrective_version: MEASUREMENT_CORRECTIVE_VERSION.to_string(),
            result_schema_version: markit_mdbench_runner::RESULT_SCHEMA_VERSION_V2,
            envelope_schema_id: ENVELOPE_SCHEMA_ID.to_string(),
            envelope_schema_sha256: "cd".repeat(32),
            session_count: 3,
            warmup_iterations: 10,
            measured_iterations: 30,
            campaign_seed: 42,
            campaign_seed_algorithm: CAMPAIGN_SEED_ALGORITHM_ID.to_string(),
            case_order_algorithm: markit_mdbench_runner::SHUFFLE_ALGORITHM_ID.to_string(),
            horse_order_policy: crate::schedule::HORSE_ORDER_POLICY_ID.to_string(),
            session_seed_domain: SESSION_SEED_DOMAIN.to_string(),
            horse_base_seed_domain: HORSE_BASE_SEED_DOMAIN.to_string(),
            build_profile_id: markit_mdbench_runner::build_identity::RELEASE_PRIMARY_PROFILE_ID
                .to_string(),
            machine_manifest_sha256: "ef".repeat(32),
            metric_qualification: crate::METRIC_QUALIFICATION
                .iter()
                .map(|(m, q)| (m.to_string(), q.to_string()))
                .collect(),
            quantile_policy: vec!["nearest-rank".to_string()],
            aggregation_policy: vec!["project-macro".to_string()],
            failure_policy: vec!["fail-closed".to_string()],
        }
    }
}
