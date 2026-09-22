//! markit-mdbench-campaign — the #31 primary performance campaign freeze
//! (MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-1).
//!
//! Authority chain (frozen before any primary timing):
//!
//! ```text
//! #22 mechanisms / runner / controlled protocol
//! #35 frozen primary real workload   (22 G0-strict FULL_READ files,
//!                                     362 G0_PRIMARY EDIT_WRITE cases)
//! #41 measurement substrate          (timer boundary, schema v2, 110/110
//!                                     + 1810/1810 parity)
//! THIS CRATE: campaign execution contract freeze (#31)
//!   session identity / iteration policy / case+horse ordering /
//!   machine + build profile / metric qualification / raw observation
//!   envelope / quantile + aggregation policy / failure policy /
//!   preflight / non-research smoke
//! human review -> PRIMARY_PERFORMANCE_CAMPAIGN_READY
//! separate task -> MARKIT-31-PRIMARY-PERFORMANCE-RUN-1 (real timing)
//! ```
//!
//! What this crate is:
//!
//! - an ORCHESTRATOR: it consumes the frozen workload manifests, the
//!   frozen campaign manifest / machine manifest / schedule, and
//!   dispatches H0-H4 through the EXISTING runner functions
//!   (`run_full_parse_timed` / `run_update_timed` / `run_*_attributed`,
//!   `build_initial_state`) — there is no second benchmark engine here;
//! - an identity layer: `CampaignSpecId` / `SessionId` / `RunId` /
//!   `ObservationId` and the campaign seed are derived deterministically
//!   from frozen inputs, never chosen after seeing timing;
//! - a fail-closed preflight: receipt hashes, machine match, build
//!   identity, affinity, cardinalities, duplicate observation guards.
//!
//! What this crate is NOT:
//!
//! - it does not reselect workload (#35 owns selection; #31 consumes);
//! - it does not change horse semantics, fallback/reuse policy, the
//!   timer boundary, or the result schema (Rust `runner/src/result.rs`
//!   stays the schema authority);
//! - it does not rank, score, or promote winners — raw observations
//!   carry facts only; interpretation belongs after measurement review;
//! - its statistical layer ([`stats`]) is exercised only on synthetic
//!   data in this freeze; no primary wall-clock number exists here.
//!
//! Primary trace semantics remain SINGLE_RESET (task §4): every measured
//! EDIT_WRITE iteration starts from the exact frozen pre-edit source and
//! a freshly constructed valid pre-edit horse state; no state is retained
//! across measured iterations.

pub mod execute;
pub mod finalize;
pub mod identity;
pub mod machine;
pub mod manifest;
pub mod preflight;
pub mod receipt;
pub mod schedule;
pub mod smoke;
pub mod stats;
pub mod workload;

use serde::{Deserialize, Serialize};

/// Identifier of the frozen campaign (matches the task/protocol name).
pub const CAMPAIGN_ID: &str = "MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-v1";

/// Schema tag of the campaign manifest
/// (`results/manifests/primary-performance-campaign-v1.toml`).
pub const CAMPAIGN_MANIFEST_SCHEMA: &str = "primary-performance-campaign-v1";

/// Schema tag of the machine manifest
/// (`results/manifests/primary-machine-v1.toml`).
pub const MACHINE_MANIFEST_SCHEMA: &str = "primary-machine-v1";

/// Schema tag of the schedule manifest
/// (`results/manifests/primary-schedule-v1.jsonl`).
pub const SCHEDULE_SCHEMA: &str = "primary-schedule-v1";

/// Schema tag of the campaign receipt
/// (`results/manifests/primary-campaign-receipt-v1.json`).
pub const CAMPAIGN_RECEIPT_SCHEMA: &str = "primary-campaign-receipt-v1";

/// Schema tag of the raw campaign observation envelope
/// (`protocol/campaign-observation-schema-v1.json` documents the Rust
/// model in [`execute::CampaignObservationV1`], which is the authority).
pub const ENVELOPE_SCHEMA_ID: &str = "campaign-observation-v1";

/// Measurement-substrate corrective this campaign is measured under.
pub const MEASUREMENT_CORRECTIVE_VERSION: &str = "MEASUREMENT-CORRECTIVE-1";

/// Frozen protocol generation of the result rows this campaign emits.
pub const PROTOCOL_VERSION: &str = markit_mdbench_runner::PROTOCOL_VERSION;

/// The grammar authority of the primary G0 surfaces (echoed from the
/// frozen substrate, asserted against the workload manifests — never
/// redefined here).
pub const G0_GRAMMAR_ID: &str = markit_mdbench_semantics::G0_GRAMMAR_ID;

/// One frozen roster entry: horse label + its mechanism id (echoed from
/// the mechanism crates and asserted against them at manifest verify —
/// never a second source of truth).
pub struct HorseEntry {
    pub id: &'static str,
    pub mechanism_id: &'static str,
}

/// The frozen horse roster (identity order H0-H4; per-case EXECUTION
/// order is a seeded rotation, see [`schedule`]).
pub const HORSE_ROSTER: [HorseEntry; 5] = [
    HorseEntry {
        id: "H0",
        mechanism_id: markit_mdbench_full_rebuild::H0_MECHANISM_ID,
    },
    HorseEntry {
        id: "H1",
        mechanism_id: markit_mdbench_block_local::H1_MECHANISM_ID,
    },
    HorseEntry {
        id: "H2",
        mechanism_id: markit_mdbench_fragment_reuse::H2_MECHANISM_ID,
    },
    HorseEntry {
        id: "H3",
        mechanism_id: markit_mdbench_old_tree_subtree_reuse::H3_MECHANISM_ID,
    },
    HorseEntry {
        id: "H4",
        mechanism_id: markit_mdbench_restart_convergence::H4_MECHANISM_ID,
    },
];

/// Horse labels in identity order.
pub const HORSE_IDS: [&str; 5] = ["H0", "H1", "H2", "H3", "H4"];

/// The two primary timing surfaces (task §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    CleanState,
    EditWrite,
}

impl Surface {
    pub fn as_str(self) -> &'static str {
        match self {
            Surface::CleanState => "clean_state",
            Surface::EditWrite => "edit_write",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "clean_state" => Ok(Surface::CleanState),
            "edit_write" => Ok(Surface::EditWrite),
            other => Err(format!(
                "unknown surface {other:?} (clean_state | edit_write)"
            )),
        }
    }

    /// Frozen case cardinality of this surface (22 / 362).
    pub fn frozen_case_count(self) -> usize {
        match self {
            Surface::CleanState => 22,
            Surface::EditWrite => 362,
        }
    }
}

/// Sample kinds carried by the raw observation envelope (task §17-§18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleKind {
    Warmup,
    Measured,
    Attribution,
}

impl SampleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SampleKind::Warmup => "warmup",
            SampleKind::Measured => "measured",
            SampleKind::Attribution => "attribution",
        }
    }
}

/// Frozen metric qualification table (task §19 / #41): the ONLY metrics
/// the primary campaign may report. Anything else must surface as
/// UNAVAILABLE — blanks are never printed as zero.
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

/// SHA256 of a byte slice as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest: [u8; 32] = hasher.finalize().into();
    markit_mdbench_common::to_lower_hex(&digest)
}

/// SHA256 of a file's bytes as lowercase hex.
pub fn sha256_file(path: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(sha256_hex(&bytes))
}
