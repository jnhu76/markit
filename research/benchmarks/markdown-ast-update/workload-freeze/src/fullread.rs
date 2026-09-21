//! A7 — the frozen FULL_READ-v1 manifest (task §37).
//!
//! Every final selected physical file appears, once per meaningful
//! grammar lane, with its frozen identity, eligibility and qualification
//! class. G0 strict FULL_READ cases are explicitly identifiable; G1/G2
//! records remain semantic/realism/deferred and are never marked
//! H0-H4-ready. No timing exists anywhere in this manifest.

use serde::{Deserialize, Serialize};

use crate::{SelectedFile, FULL_READ_SCHEMA};

/// Qualification class of one FULL_READ lane record (closed).
pub const QUAL_STRICT: &str = "strict";
pub const QUAL_SEMANTIC_ONLY: &str = "semantic_only";
pub const QUAL_REALISM_ONLY: &str = "realism_only";
pub const QUAL_DEFERRED: &str = "deferred";

/// One lane record inside a FULL_READ manifest row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullReadLane {
    pub lane_id: String,
    pub grammar_id: String,
    pub lane_valid: bool,
    pub strict_scope_clean: bool,
    pub qualification: String,
    /// Explicit campaign class: `G0_STRICT_FULL_READ` is the primary
    /// strict-performance surface; every other class is named and bounded.
    pub case_class: String,
}

/// One FULL_READ manifest row (canonical JSONL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullReadRecord {
    pub schema: String,
    pub generator_version: String,
    pub source_id: String,
    pub source_key: String,
    pub source_sha256: String,
    pub file_bytes: u64,
    pub acquisition_commit_sha: String,
    /// Logical CORRECTIVE-B memberships of the physical file.
    pub memberships: Vec<String>,
    pub lanes: Vec<FullReadLane>,
}

/// Build the FULL_READ manifest for the frozen selection. `commit_shas`
/// maps source id -> acquisition commit SHA (from source-lock.json).
pub fn build_full_read(
    files: &[SelectedFile],
    profiles: &std::collections::BTreeMap<(String, String), markit_mdbench_semantics::LaneProfile>,
    commit_shas: &std::collections::BTreeMap<String, String>,
) -> Vec<FullReadRecord> {
    let mut records = Vec::new();
    for file in files {
        let mut lanes = Vec::new();
        for (lane_id, grammar_id) in [
            ("G0", crate::G0_GRAMMAR_ID),
            ("G1", crate::G1_GRAMMAR_ID),
            ("G2", crate::G2_GRAMMAR_ID),
        ] {
            let (lane_valid, strict_clean) = if lane_id == "G2" {
                // G2 has no semantics yet: lane validity is undefined, so
                // the record carries the frozen deferred status instead
                // of an invented boolean.
                (false, false)
            } else if let Some(profile) = profiles.get(&(file.key.clone(), grammar_id.to_string()))
            {
                (profile.eligibility.lane_valid, profile.eligibility.strict_scope_clean)
            } else {
                (false, false)
            };
            let (qualification, case_class) = match lane_id {
                "G0" => {
                    if strict_clean {
                        (QUAL_STRICT, "G0_STRICT_FULL_READ")
                    } else {
                        (QUAL_REALISM_ONLY, "G0_REALISM_FULL_READ")
                    }
                }
                "G1" => {
                    if strict_clean {
                        (QUAL_SEMANTIC_ONLY, "G1_SEMANTIC_FULL_READ")
                    } else {
                        (QUAL_REALISM_ONLY, "G1_REALISM_FULL_READ")
                    }
                }
                _ => (QUAL_DEFERRED, "G2_DEFERRED_FULL_READ"),
            };
            lanes.push(FullReadLane {
                lane_id: lane_id.to_string(),
                grammar_id: grammar_id.to_string(),
                lane_valid,
                strict_scope_clean: strict_clean,
                qualification: qualification.to_string(),
                case_class: case_class.to_string(),
            });
        }
        records.push(FullReadRecord {
            schema: FULL_READ_SCHEMA.to_string(),
            generator_version: crate::CORRECTIVE_C_VERSION.to_string(),
            source_id: file.source_id.clone(),
            source_key: file.key.clone(),
            source_sha256: file.sha256.clone(),
            file_bytes: file.file_bytes,
            acquisition_commit_sha: commit_shas
                .get(&file.source_id)
                .cloned()
                .unwrap_or_default(),
            memberships: file.memberships.clone(),
            lanes,
        });
    }
    records
}

/// Count of G0 strict FULL_READ cases (the primary strict surface).
pub fn g0_strict_count(records: &[FullReadRecord]) -> usize {
    records
        .iter()
        .filter(|record| {
            record
                .lanes
                .iter()
                .any(|lane| lane.case_class == "G0_STRICT_FULL_READ")
        })
        .count()
}
