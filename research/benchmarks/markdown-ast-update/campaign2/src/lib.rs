//! markit-mdbench-campaign2 — MARKIT-31 FULL EVIDENCE CAMPAIGN-2.
//!
//! This crate is CAMPAIGN TOOLING. It adds no mechanism semantics and
//! changes no production horse: H0-H4 are consumed exactly as frozen at
//! authority SHA `3762b7a42e1c284a4c2c2e0ebac8496e70c63431`, through the
//! existing `markit_mdbench_runner` phase functions.
//!
//! Four evidence surfaces (task §13-§25):
//!
//! ```text
//! A CONSTRUCTION      source -> valid native state            (primary timing)
//! B RESIDENT_UPDATE   state -> one isolated edit -> state     (primary timing)
//! C LIFECYCLE         build once -> edit1..editK              (primary lifecycle)
//! D CONTROLLED        N / B / D-fence / F-reference / K-depth (controlled)
//! ```
//!
//! Campaign-1 identities (`CampaignSpecId ad45c7cb…`, `RunId 905427da…`)
//! are historical evidence and are never reused, appended to, or
//! rewritten. Campaign-2 derives NEW identities from NEW frozen inputs.
//!
//! Campaign-1's interpretation rules R1-R4 (PR #46 audit) are frozen into
//! the Campaign-2 metadata: see [`interpretation`].

pub mod controlled;
pub mod envelope;
pub mod exec;
pub mod finalize;
pub mod generators;
pub mod identity;
pub mod interpretation;
pub mod lifecycle;
pub mod machine;
pub mod memory;
pub mod receipt;
pub mod schedule;
pub mod spec;
pub mod store;

use std::path::Path;

/// Study identifier (task §7). Structural, not hashed.
pub const STUDY_ID: &str = "MARKIT-31-FULL-EVIDENCE-STUDY";

/// Campaign-2 identifier.
pub const CAMPAIGN2_ID: &str = "MARKIT-31-FULL-EVIDENCE-CAMPAIGN-2";

/// Envelope schema tag of the Campaign-2 raw observation.
pub const ENVELOPE_SCHEMA_ID: &str = "campaign2-observation-v1";

/// Result-row schema version consumed from the frozen runner.
pub const RESULT_SCHEMA_VERSION: u16 = markit_mdbench_runner::RESULT_SCHEMA_VERSION_V2;

/// The four Campaign-2 surfaces (task §13-§25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface2 {
    /// A — source -> valid native state.
    Construction,
    /// B — valid state -> one isolated edit -> valid state (SINGLE_RESET).
    ResidentUpdate,
    /// C — build once, then K chained edits on the SAME state.
    Lifecycle,
    /// D — controlled regime cells.
    Controlled,
}

impl Surface2 {
    pub fn as_str(self) -> &'static str {
        match self {
            Surface2::Construction => "construction",
            Surface2::ResidentUpdate => "resident_update",
            Surface2::Lifecycle => "lifecycle",
            Surface2::Controlled => "controlled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "construction" => Ok(Surface2::Construction),
            "resident_update" => Ok(Surface2::ResidentUpdate),
            "lifecycle" => Ok(Surface2::Lifecycle),
            "controlled" => Ok(Surface2::Controlled),
            other => Err(format!(
                "unknown campaign-2 surface {other:?} \
                 (construction | resident_update | lifecycle | controlled)"
            )),
        }
    }
}

/// The five evidence classes (task §54). Every artifact belongs to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EvidenceClass {
    PrimaryTiming,
    PrimaryWork,
    PrimaryLifecycle,
    ControlledTiming,
    ControlledWork,
    ProfilePerf,
    ProfileEbpf,
    ProfileAllocator,
    DescriptiveMemory,
    EnvironmentTelemetry,
    DerivedMechanical,
}

impl EvidenceClass {
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceClass::PrimaryTiming => "PRIMARY_TIMING",
            EvidenceClass::PrimaryWork => "PRIMARY_WORK",
            EvidenceClass::PrimaryLifecycle => "PRIMARY_LIFECYCLE",
            EvidenceClass::ControlledTiming => "CONTROLLED_TIMING",
            EvidenceClass::ControlledWork => "CONTROLLED_WORK",
            EvidenceClass::ProfilePerf => "PROFILE_PERF",
            EvidenceClass::ProfileEbpf => "PROFILE_EBPF",
            EvidenceClass::ProfileAllocator => "PROFILE_ALLOCATOR",
            EvidenceClass::DescriptiveMemory => "DESCRIPTIVE_MEMORY",
            EvidenceClass::EnvironmentTelemetry => "ENVIRONMENT_TELEMETRY",
            EvidenceClass::DerivedMechanical => "DERIVED_MECHANICAL",
        }
    }
}

/// Frozen horse roster (identity order). Mechanism ids are echoed from
/// the mechanism crates and asserted at manifest verification — never a
/// second source of truth.
pub const HORSE_IDS: [&str; 5] = ["H0", "H1", "H2", "H3", "H4"];

/// Mechanism id for a horse label.
pub fn horse_mechanism_id(horse_id: &str) -> Result<&'static str, String> {
    match horse_id {
        "H0" => Ok(markit_mdbench_full_rebuild::H0_MECHANISM_ID),
        "H1" => Ok(markit_mdbench_block_local::H1_MECHANISM_ID),
        "H2" => Ok(markit_mdbench_fragment_reuse::H2_MECHANISM_ID),
        "H3" => Ok(markit_mdbench_old_tree_subtree_reuse::H3_MECHANISM_ID),
        "H4" => Ok(markit_mdbench_restart_convergence::H4_MECHANISM_ID),
        other => Err(format!("unknown horse {other:?}")),
    }
}

/// SHA256 (lowercase hex) of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest: [u8; 32] = hasher.finalize().into();
    markit_mdbench_common::to_lower_hex(&digest)
}

/// SHA256 (lowercase hex) of a file's bytes.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

/// SHA256 (lowercase hex) of the currently running executable. Fails
/// closed: campaign evidence never carries an unknown binary identity.
pub fn current_executable_sha256() -> Result<String, String> {
    let path = std::env::current_exe()
        .map_err(|e| format!("resolve current executable for the RunId binding: {e}"))?;
    sha256_file(&path).map_err(|e| format!("hash current executable {}: {e}", path.display()))
}

/// Frozen metric qualification carried into the Campaign-2 spec. Anything
/// outside this table must surface as `UNAVAILABLE` (never a printed 0).
pub const METRIC_QUALIFICATION: [(&str, &str); 8] = [
    ("LATENCY", "QUALIFIED"),
    ("WORK_COUNTERS", "QUALIFIED"),
    ("PARSE_AMPLIFICATION", "QUALIFIED"),
    ("CPU_TIME", "UNAVAILABLE"),
    ("ALLOC_COUNT", "UNAVAILABLE"),
    ("ALLOC_BYTES", "UNAVAILABLE"),
    ("PEAK_MEMORY", "UNAVAILABLE"),
    ("RETAINED_MEMORY", "UNAVAILABLE"),
];

/// Paths of the frozen #35 workload artifacts this campaign consumes.
pub struct WorkloadPaths {
    pub root: std::path::PathBuf,
}

impl WorkloadPaths {
    /// Path of one frozen payload artifact.
    pub fn payload(&self, name: &str) -> std::path::PathBuf {
        self.root.join("workloads/payloads").join(name)
    }
}

/// The frozen #35 workload artifact root.
pub fn manifest_paths(benchmark_root: &Path) -> WorkloadPaths {
    WorkloadPaths {
        root: benchmark_root.to_path_buf(),
    }
}

/// Load the frozen Campaign-2 identity block written by `spec-bind`.
pub fn load_frozen_identity(benchmark_root: &Path) -> Result<identity::Campaign2Identity, String> {
    let path = store::campaign_root(benchmark_root).join("manifests/campaign-2-identity-v1.json");
    let bytes = std::fs::read(&path).map_err(|e| {
        format!(
            "read frozen Campaign-2 identity {}: {e} (run `spec-bind` first)",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {e}", path.display()))
}
