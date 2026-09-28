//! Frozen PMU panel authority (task §3/§4 of the PMU campaign order).
//!
//! The panel was frozen by RESULT ANALYSIS (Stages A+E) and lives at
//! `results/diagnostics/pmu-v1/panel/PMU-PANEL-MANIFEST-v1.json` on the
//! formal host. The driver NEVER regenerates, extends, or reselects it:
//! it parses the file, verifies its SHA256 against the baked constant,
//! and preserves every cell's identity fields verbatim into every
//! observation.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Study identity (never the primary campaign identity).
pub const PMU_STUDY_ID: &str = "MARKIT-76-SIX-HORSE-PMU-EXPLANATION-v1";

/// The frozen PMU horse-order seed (binds the panel manifest content).
pub const PMU_SEED: u64 = 0x9cb79023813dcf7b;

/// The COMPLETE 64-hex SHA256 of the frozen panel manifest — authority
/// is the full hash, never an abbreviation.
pub const PMU_PANEL_MANIFEST_SHA256: &str =
    "409b689c33547b396e792847d4f35b9a1e4a53365a7ae6a9ea1b06e34511a15a";

/// The primary mechanism source authority this study observes (the six
/// horse mechanism implementations remain bound to this revision).
pub const PRIMARY_MECHANISM_AUTHORITY: &str = "df1955c9faf06a36bb3fc7d6453d720dfcdc8f0f";

/// The only selection classes the frozen panel may carry.
pub const SELECTION_REPRESENTATIVE: &str = "REPRESENTATIVE_PRE_OUTCOME";
pub const SELECTION_POST_HOC: &str = "POST_HOC_ANOMALY";

/// One panel cell as frozen by RESULT ANALYSIS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PanelCell {
    pub slot: String,
    pub case_id: String,
    pub surface: String,
    pub payload_id: String,
    pub frozen_regime: String,
    pub project: String,
    pub file_bytes: u64,
    /// Serialized as `SELECTION_CLASS` in the frozen artifact.
    #[serde(rename = "SELECTION_CLASS")]
    pub selection_class: String,
    #[serde(default)]
    pub selection_reason: String,
    #[serde(default)]
    pub trigger_statistic: Option<String>,
}

/// The panel manifest (only the fields the driver needs; the byte-hash
/// check above pins the file, so unknown extra fields cannot change
/// meaning).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PanelManifest {
    pub artifact: String,
    pub study: String,
    pub frozen_at_utc: String,
    pub campaign_spec_id: String,
    pub cells: Vec<PanelCell>,
}

/// The expected frozen panel shape (16 + 6 = 22 cells, classes intact).
pub const EXPECTED_REPRESENTATIVE_CELLS: usize = 16;
pub const EXPECTED_POST_HOC_CELLS: usize = 6;

/// Load and verify the frozen panel: byte hash against the baked
/// constant, study identity, cell counts, class vocabulary, cell-slot
/// and case-id uniqueness, surface vocabulary.
pub fn load_verified_panel(path: &Path, expected_sha256: &str) -> Result<PanelManifest, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read panel {}: {e}", path.display()))?;
    let actual = crate::sha256_hex(&bytes);
    if actual != expected_sha256 {
        return Err(format!(
            "PMU_PANEL_AUTHORITY = FAIL: panel {} sha256 {actual} != frozen {expected_sha256}",
            path.display()
        ));
    }
    let manifest: PanelManifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse panel {}: {e}", path.display()))?;
    if manifest.study != PMU_STUDY_ID {
        return Err(format!(
            "panel study {:?} != {PMU_STUDY_ID}",
            manifest.study
        ));
    }
    let representative = manifest
        .cells
        .iter()
        .filter(|c| c.selection_class == SELECTION_REPRESENTATIVE)
        .count();
    let post_hoc = manifest
        .cells
        .iter()
        .filter(|c| c.selection_class == SELECTION_POST_HOC)
        .count();
    if representative != EXPECTED_REPRESENTATIVE_CELLS || post_hoc != EXPECTED_POST_HOC_CELLS {
        return Err(format!(
            "panel cells {representative} representative + {post_hoc} post-hoc != frozen \
             {EXPECTED_REPRESENTATIVE_CELLS} + {EXPECTED_POST_HOC_CELLS}"
        ));
    }
    for cell in &manifest.cells {
        if cell.selection_class != SELECTION_REPRESENTATIVE
            && cell.selection_class != SELECTION_POST_HOC
        {
            return Err(format!(
                "cell {} carries unknown selection class {:?}",
                cell.slot, cell.selection_class
            ));
        }
        crate::Surface::parse(&cell.surface)
            .map_err(|e| format!("cell {} surface: {e}", cell.slot))?;
        if cell.post_hoc() && cell.trigger_statistic.is_none() {
            return Err(format!(
                "post-hoc cell {} lacks its trigger statistic",
                cell.slot
            ));
        }
    }
    let slots: BTreeSet<&str> = manifest.cells.iter().map(|c| c.slot.as_str()).collect();
    if slots.len() != manifest.cells.len() {
        return Err("duplicate panel cell slots".to_string());
    }
    let case_surfaces: BTreeSet<(&str, &str)> = manifest
        .cells
        .iter()
        .map(|c| (c.case_id.as_str(), c.surface.as_str()))
        .collect();
    if case_surfaces.len() != manifest.cells.len() {
        return Err("duplicate (case_id, surface) panel cells".to_string());
    }
    Ok(manifest)
}

impl PanelCell {
    pub fn is_representative(&self) -> bool {
        self.selection_class == SELECTION_REPRESENTATIVE
    }

    pub fn post_hoc(&self) -> bool {
        self.selection_class == SELECTION_POST_HOC
    }
}
