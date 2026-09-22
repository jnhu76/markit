//! DOMAIN-STRATA-v1 — frozen source-to-domain mapping (CORRECTIVE-B §17).
//!
//! The mapping is a committed INPUT, frozen before any selection runs. It
//! classifies each acquisition source by project purpose only — never by
//! parser behavior, eligibility or any performance fact. Sources whose
//! authoritative upstream book is not Markdown keep that caveat here so
//! selection can never count them as book-workload coverage.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::DOMAIN_STRATA_SCHEMA;

/// The closed domain-stratum vocabulary (CORRECTIVE-B §17).
pub const DOMAIN_STRATA: [&str; 9] = [
    "BOOK_TUTORIAL",
    "API_TECHNICAL_DOC",
    "STANDARD_SPECIFICATION",
    "RFC_PROPOSAL_DESIGN",
    "LARGE_GUIDE_REFERENCE",
    "SECURITY_OPERATIONAL_DOC",
    "ML_SCIENTIFIC_TECHNICAL",
    "CJK_TECHNICAL",
    "OTHER_DECLARED",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStratumEntry {
    pub source_id: String,
    pub stratum: String,
    pub rationale: String,
    /// True when the acquisition role may be counted as coverage for the
    /// stratum during selection (mirrors `counts_as_role_coverage`).
    pub counts_as_role_coverage: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_caveat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStrata {
    pub schema: String,
    pub sources: Vec<DomainStratumEntry>,
}

impl DomainStrata {
    pub fn stratum_for(&self, source_id: &str) -> Option<&str> {
        self.sources
            .iter()
            .find(|entry| entry.source_id == source_id)
            .map(|entry| entry.stratum.as_str())
    }

    /// Validate: closed vocabulary, no duplicates, strata with no member
    /// are reported (a missing stratum is an honest uncovered-space fact,
    /// not an error).
    pub fn validate(&self) -> Result<Vec<String>, String> {
        let mut seen = BTreeMap::new();
        for entry in &self.sources {
            if !DOMAIN_STRATA.contains(&entry.stratum.as_str()) {
                return Err(format!(
                    "source {} uses undeclared stratum {}",
                    entry.source_id, entry.stratum
                ));
            }
            if seen.insert(&entry.source_id, &entry.stratum).is_some() {
                return Err(format!("duplicate domain entry for {}", entry.source_id));
            }
        }
        let members: BTreeMap<&str, usize> =
            self.sources.iter().fold(BTreeMap::new(), |mut acc, entry| {
                *acc.entry(entry.stratum.as_str()).or_insert(0) += 1;
                acc
            });
        let empty: Vec<String> = DOMAIN_STRATA
            .iter()
            .filter(|stratum| !members.contains_key(*stratum))
            .map(|stratum| stratum.to_string())
            .collect();
        Ok(empty)
    }
}

/// Load the committed mapping from `workloads/analysis/domain-strata-v1.json`.
pub fn load_mapping(workloads_root: &Path) -> Result<DomainStrata, String> {
    let path = workloads_root.join("analysis/domain-strata-v1.json");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let strata: DomainStrata =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    if strata.schema != DOMAIN_STRATA_SCHEMA {
        return Err(format!(
            "{}: schema {} != {DOMAIN_STRATA_SCHEMA}",
            path.display(),
            strata.schema
        ));
    }
    strata.validate()?;
    Ok(strata)
}
