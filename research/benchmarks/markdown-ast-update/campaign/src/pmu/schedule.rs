//! Deterministic PMU schedule (task §16/§17 of the PMU campaign order).
//!
//! Materialized from the FROZEN PMU seed and the FROZEN panel BEFORE any
//! horse observation, written to disk, and SHA256-bound; the runner
//! refuses to observe anything not present in the verified schedule.
//!
//! Ordering rules (frozen):
//!
//! - horse order within one `(panel cell, repetition)` block is the
//!   seeded base six-horse permutation (the runner's frozen
//!   `splitmix64-v1 + fisher-yates-lemire-rejection-v2` primitive over
//!   sorted horse labels, seeded by the PMU seed) rotated by
//!   `(cell ordinal + repetition ordinal) mod 6` — the same
//!   `seeded-base-permutation-rotate-v1` policy shape as the primary
//!   campaign, so identity order is never execution order;
//! - the flattened entry list (all cells × groups × repetitions ×
//!   horses) receives ONE final seeded shuffle with the SAME primitive
//!   plus a fixed derivation domain, so execution never runs "all H0,
//!   then all H1, …" globally either;
//! - every qualified observation is exactly one `case × horse × event
//!   group × repetition` (task §10): one schedule entry = one fresh
//!   process = one counted region.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::events;
use super::panel::{PanelCell, PanelManifest, PMU_SEED, PMU_STUDY_ID};

/// Schedule file schema id (first line of the file).
pub const PMU_SCHEDULE_SCHEMA: &str = "pmu-schedule-v1";

/// Fresh-process repetitions per (cell × horse × group) — frozen.
pub const PMU_REPETITIONS: u32 = 3;

/// Fixed derivation domain separating the entry-shuffle seed from the
/// horse-permutation seed (both derive from the PMU seed).
pub const ENTRY_SHUFFLE_SEED_DOMAIN: &str = "MARKIT-76-SIX-HORSE-PMU-ENTRY-SHUFFLE-v1";

/// Algorithm ids recorded in the header (reused runner primitives).
pub const ENTRY_SHUFFLE_ALGORITHM: &str = "splitmix64-v1+fisher-yates-lemire-rejection-v2";
pub const HORSE_ORDER_POLICY: &str = "seeded-base-permutation-rotate-v1";

/// One schedule entry: one qualified PMU observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PmuScheduleEntry {
    pub schema: String,
    pub entry_ordinal: u32,
    pub slot: String,
    pub case_id: String,
    pub surface: String,
    pub payload_id: String,
    pub frozen_regime: String,
    pub selection_class: String,
    pub selection_reason: String,
    pub horse: String,
    pub event_group: String,
    pub repetition: u32,
}

/// The schedule header record (first line).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PmuScheduleHeader {
    pub schema: String,
    pub study_id: String,
    pub panel_manifest_sha256: String,
    pub pmu_seed_hex: String,
    pub repetitions: u32,
    pub horse_order_policy: String,
    pub entry_shuffle_algorithm: String,
    pub entry_shuffle_seed: u64,
    /// SHA256 of the canonical entry lines (ordinal-sorted), excluding
    /// the header itself.
    pub schedule_sha256: String,
    pub expected_entries: u32,
}

/// Derive the entry-shuffle seed from the frozen PMU seed.
pub fn entry_shuffle_seed(pmu_seed: u64) -> u64 {
    // Same first-64-bits-big-endian derivation frame as the campaign
    // identity module, over this module's fixed domain.
    let material = format!("{ENTRY_SHUFFLE_SEED_DOMAIN}\n{pmu_seed:x}");
    let digest = crate::pmu::sha256_bytes(material.as_bytes());
    u64::from_be_bytes(digest[0..8].try_into().expect("8 bytes"))
}

/// Canonical entry serialization for hashing (one JSON line per entry,
/// ordinal-sorted; identity fields only — never measured values).
fn canonical_entry_bytes(entries: &[PmuScheduleEntry]) -> Vec<u8> {
    let mut sorted: Vec<&PmuScheduleEntry> = entries.iter().collect();
    sorted.sort_by_key(|entry| entry.entry_ordinal);
    let mut bytes = Vec::new();
    for entry in sorted {
        serde_json::to_writer(&mut bytes, entry).expect("entry serializes");
        bytes.push(b'\n');
    }
    bytes
}

/// Materialize the full deterministic schedule from the frozen panel.
///
/// Block construction: for each repetition, for each event group, for
/// each panel cell (manifest order), expand the seeded rotated horse
/// order into one entry per horse. The flattened list then receives the
/// final seeded shuffle which assigns `entry_ordinal`s.
pub fn materialize(
    panel: &PanelManifest,
) -> Result<(PmuScheduleHeader, Vec<PmuScheduleEntry>), String> {
    let base = crate::schedule::base_horse_permutation(PMU_SEED);
    let shuffle_seed = entry_shuffle_seed(PMU_SEED);
    let mut blocks: Vec<PmuScheduleEntry> = Vec::new();
    for repetition in 0..PMU_REPETITIONS {
        for group in events::all_groups() {
            for (cell_ordinal, cell) in panel.cells.iter().enumerate() {
                let horse_order =
                    crate::schedule::horse_order_for(&base, cell_ordinal as u32, repetition);
                for horse in horse_order {
                    blocks.push(PmuScheduleEntry {
                        schema: PMU_SCHEDULE_SCHEMA.to_string(),
                        // temporary ordinal, reassigned after the shuffle
                        entry_ordinal: 0,
                        slot: cell.slot.clone(),
                        case_id: cell.case_id.clone(),
                        surface: cell.surface.clone(),
                        payload_id: cell.payload_id.clone(),
                        frozen_regime: cell.frozen_regime.clone(),
                        selection_class: cell.selection_class.clone(),
                        selection_reason: cell.selection_reason.clone(),
                        horse,
                        event_group: group.id.to_string(),
                        repetition,
                    });
                }
            }
        }
    }
    // Final seeded shuffle over the flattened entry list (the SAME frozen
    // primitive the primary campaign uses for case order).
    let mut rng = markit_mdbench_runner::SplitMix64V1::new(shuffle_seed);
    for i in (1..blocks.len()).rev() {
        let j = rng.next_below(i + 1);
        blocks.swap(i, j);
    }
    let mut entries = blocks;
    for (ordinal, entry) in entries.iter_mut().enumerate() {
        entry.entry_ordinal = ordinal as u32;
    }
    let schedule_sha256 = crate::sha256_hex(&canonical_entry_bytes(&entries));
    let header = PmuScheduleHeader {
        schema: PMU_SCHEDULE_SCHEMA.to_string(),
        study_id: PMU_STUDY_ID.to_string(),
        panel_manifest_sha256: super::panel::PMU_PANEL_MANIFEST_SHA256.to_string(),
        pmu_seed_hex: format!("{PMU_SEED:#x}"),
        repetitions: PMU_REPETITIONS,
        horse_order_policy: HORSE_ORDER_POLICY.to_string(),
        entry_shuffle_algorithm: ENTRY_SHUFFLE_ALGORITHM.to_string(),
        entry_shuffle_seed: shuffle_seed,
        schedule_sha256,
        expected_entries: entries.len() as u32,
    };
    Ok((header, entries))
}

/// Expected entry cardinality for the frozen panel (22 cells × 6 horses
/// × 8 groups × 3 repetitions).
pub fn expected_entries(panel_cell_count: usize) -> u32 {
    let groups = events::all_groups().count() as u32;
    (panel_cell_count as u32) * crate::HORSE_ROSTER.len() as u32 * groups * PMU_REPETITIONS
}

/// Serialize the schedule file: header line, then entry lines in ordinal
/// order.
pub fn schedule_file_bytes(header: &PmuScheduleHeader, entries: &[PmuScheduleEntry]) -> Vec<u8> {
    let mut bytes = Vec::new();
    serde_json::to_writer(&mut bytes, header).expect("header serializes");
    bytes.push(b'\n');
    let mut sorted: Vec<&PmuScheduleEntry> = entries.iter().collect();
    sorted.sort_by_key(|entry| entry.entry_ordinal);
    for entry in sorted {
        serde_json::to_writer(&mut bytes, entry).expect("entry serializes");
        bytes.push(b'\n');
    }
    bytes
}

/// Parse + verify a schedule file: header schema/study, entry-lines hash
/// equality with the header, expected count, ordinal completeness,
/// identity-key uniqueness, and per-entry structural validity.
pub fn verify_schedule_file(
    bytes: &[u8],
) -> Result<(PmuScheduleHeader, Vec<PmuScheduleEntry>), String> {
    let mut lines = bytes.split(|b| *b == b'\n').filter(|l| !l.is_empty());
    let header_line = lines.next().ok_or("empty schedule file")?;
    let header: PmuScheduleHeader =
        serde_json::from_slice(header_line).map_err(|e| format!("schedule header: {e}"))?;
    let mut entries = Vec::new();
    for line in lines {
        let entry: PmuScheduleEntry =
            serde_json::from_slice(line).map_err(|e| format!("schedule entry: {e}"))?;
        entries.push(entry);
    }
    if header.schema != PMU_SCHEDULE_SCHEMA {
        return Err(format!(
            "schedule schema {:?} != {PMU_SCHEDULE_SCHEMA}",
            header.schema
        ));
    }
    if header.study_id != PMU_STUDY_ID {
        return Err(format!(
            "schedule study {:?} != {PMU_STUDY_ID}",
            header.study_id
        ));
    }
    let actual_hash = crate::sha256_hex(&canonical_entry_bytes(&entries));
    if actual_hash != header.schedule_sha256 {
        return Err(format!(
            "schedule entry-lines sha256 {actual_hash} != header {}",
            header.schedule_sha256
        ));
    }
    if entries.len() as u32 != header.expected_entries {
        return Err(format!(
            "schedule holds {} entries, header expects {}",
            entries.len(),
            header.expected_entries
        ));
    }
    let mut ordinals = BTreeSet::new();
    let mut identity_keys = BTreeSet::new();
    let mut horses = BTreeSet::new();
    for entry in &entries {
        if !ordinals.insert(entry.entry_ordinal) {
            return Err(format!("duplicate entry ordinal {}", entry.entry_ordinal));
        }
        let horse_ok = crate::HORSE_ROSTER.iter().any(|h| h.id == entry.horse);
        if !horse_ok {
            return Err(format!(
                "entry {} carries unknown horse {:?}",
                entry.entry_ordinal, entry.horse
            ));
        }
        horses.insert(entry.horse.clone());
        events::group_by_id(&entry.event_group)?;
        crate::Surface::parse(&entry.surface)
            .map_err(|e| format!("entry {} surface: {e}", entry.entry_ordinal))?;
        if entry.repetition >= PMU_REPETITIONS {
            return Err(format!(
                "entry {} repetition out of range",
                entry.entry_ordinal
            ));
        }
        if !identity_keys.insert((
            entry.case_id.clone(),
            entry.surface.clone(),
            entry.horse.clone(),
            entry.event_group.clone(),
            entry.repetition,
        )) {
            return Err(format!(
                "duplicate observation identity at entry {}",
                entry.entry_ordinal
            ));
        }
    }
    if ordinals.len() != entries.len() {
        return Err("entry ordinals are not a complete 0..n sequence".to_string());
    }
    if horses.len() != crate::HORSE_ROSTER.len() {
        return Err("schedule does not cover every horse".to_string());
    }
    Ok((header, entries))
}

/// Look up one entry by ordinal.
pub fn entry_by_ordinal(
    entries: &[PmuScheduleEntry],
    ordinal: u32,
) -> Result<&PmuScheduleEntry, String> {
    entries
        .iter()
        .find(|entry| entry.entry_ordinal == ordinal)
        .ok_or_else(|| format!("schedule has no entry ordinal {ordinal}"))
}

/// The panel cells referenced by the schedule must match the panel
/// verbatim (task §4: no reselection, no label drift).
pub fn entries_match_panel(
    entries: &[PmuScheduleEntry],
    panel: &PanelManifest,
) -> Result<(), String> {
    let by_slot: std::collections::BTreeMap<&str, &PanelCell> =
        panel.cells.iter().map(|c| (c.slot.as_str(), c)).collect();
    for entry in entries {
        let Some(cell) = by_slot.get(entry.slot.as_str()) else {
            return Err(format!(
                "entry {} references unknown panel slot {}",
                entry.entry_ordinal, entry.slot
            ));
        };
        if cell.case_id != entry.case_id
            || cell.surface != entry.surface
            || cell.payload_id != entry.payload_id
            || cell.frozen_regime != entry.frozen_regime
            || cell.selection_class != entry.selection_class
        {
            return Err(format!(
                "entry {} identity drift against panel cell {}",
                entry.entry_ordinal, entry.slot
            ));
        }
    }
    Ok(())
}
