//! PMU diagnostic driver tests (#76 PMU campaign order).
//!
//! These tests independently establish invariants the oracle does not
//! expose (freeze discipline): frozen event-table shape, deterministic
//! schedule materialization from the frozen seed, panel-authority
//! verification (hash + class vocabulary), and byte-identity of the
//! filtered workload loader against the full frozen load.

use std::collections::BTreeSet;
use std::path::PathBuf;

use markit_mdbench_campaign::pmu::events;
use markit_mdbench_campaign::pmu::panel::{
    self, PanelCell, PanelManifest, SELECTION_POST_HOC, SELECTION_REPRESENTATIVE,
};
use markit_mdbench_campaign::pmu::schedule::{self, PMU_REPETITIONS};

fn benchmark_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A synthetic panel with the frozen shape (16 + 6 cells, valid classes,
/// unique slots/case ids) — NOT the authority file; used to test the
/// materializer and the verification rules with a locally known hash.
fn synthetic_panel() -> PanelManifest {
    let mut cells = Vec::new();
    for index in 0..16u32 {
        cells.push(PanelCell {
            slot: format!("SYN-R{index:02}"),
            case_id: format!("{index:064x}"),
            surface: if index % 2 == 0 {
                "clean_state"
            } else {
                "edit_write"
            }
            .to_string(),
            payload_id: format!("synthetic-r{index}"),
            frozen_regime: "SYNTHETIC_REGIME".to_string(),
            project: "synthetic".to_string(),
            file_bytes: 1024,
            selection_class: SELECTION_REPRESENTATIVE.to_string(),
            selection_reason: "synthetic representative".to_string(),
            trigger_statistic: None,
        });
    }
    for index in 0..6u32 {
        cells.push(PanelCell {
            slot: format!("SYN-P{index:02}"),
            case_id: format!("{:064x}", 0x1000 + index as u64),
            surface: "edit_write".to_string(),
            payload_id: format!("synthetic-p{index}"),
            frozen_regime: "SYNTHETIC_REGIME".to_string(),
            project: "synthetic".to_string(),
            file_bytes: 2048,
            selection_class: SELECTION_POST_HOC.to_string(),
            selection_reason: "synthetic post-hoc".to_string(),
            trigger_statistic: Some("synthetic trigger".to_string()),
        });
    }
    PanelManifest {
        artifact: "PMU-PANEL-MANIFEST-v1".to_string(),
        study: panel::PMU_STUDY_ID.to_string(),
        frozen_at_utc: "2026-09-29T00:00:00Z".to_string(),
        campaign_spec_id: "synthetic".to_string(),
        cells,
    }
}

fn write_panel_file(panel: &PanelManifest) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("markit-pmu-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("panel.json");
    let bytes = serde_json::to_vec(panel).expect("serialize panel");
    let sha = markit_mdbench_campaign::sha256_hex(&bytes);
    std::fs::write(&path, bytes).expect("write panel");
    (path, sha)
}

#[test]
fn event_table_shape_is_frozen() {
    events::table_invariants().expect("event table invariants");
    for group in events::all_groups() {
        if group.events[0].kind != events::PERF_TYPE_SOFTWARE {
            assert!(
                group.events.len() <= events::MAX_HARDWARE_GROUP_EVENTS,
                "hardware group {} is too large",
                group.id
            );
        }
    }
    assert_eq!(events::HARDWARE_GROUPS.len(), 7);
    assert_eq!(events::SOFTWARE_GROUPS.len(), 1);
    // Kernel-generic encodings stay exactly as frozen.
    assert_eq!(
        events::GROUP_B2[0].config,
        0,
        "L1-dcache-loads = L1D|READ|ACCESS"
    );
    assert_eq!(events::GROUP_B2[1].config, 1 << 16, "L1-dcache-load-misses");
    assert_eq!(events::GROUP_C1[0].config, 2, "LLC-loads = LL|READ|ACCESS");
    assert_eq!(
        events::GROUP_C2[1].config,
        3 | (1 << 16),
        "dTLB-load-misses = DTLB|READ|MISS"
    );
    assert_eq!(events::GROUP_C3[0].config, 4, "iTLB-loads");
    assert_eq!(
        events::GROUP_A1[1].config,
        1,
        "instructions = PERF_COUNT_HW_INSTRUCTIONS"
    );
    assert_eq!(events::GROUP_D1.len(), 4, "OS diagnostics group");
}

#[test]
fn schedule_is_deterministic_complete_and_self_verifying() {
    let panel = synthetic_panel();
    let (header_a, entries_a) = schedule::materialize(&panel).expect("materialize A");
    let (header_b, entries_b) = schedule::materialize(&panel).expect("materialize B");
    assert_eq!(
        header_a, header_b,
        "header must be a pure function of the panel"
    );
    assert_eq!(
        entries_a, entries_b,
        "entries must be a pure function of the panel"
    );

    let expected = schedule::expected_entries(22);
    assert_eq!(expected, 22 * 6 * 8 * PMU_REPETITIONS);
    assert_eq!(entries_a.len() as u32, expected);

    // Every (cell, group, repetition) block carries each horse exactly
    // once: block keys legitimately repeat (once per horse), so the
    // per-block horse SET is the invariant.
    let mut horses_per_block = std::collections::BTreeMap::new();
    for entry in &entries_a {
        let key = (
            entry.slot.clone(),
            entry.event_group.clone(),
            entry.repetition,
        );
        horses_per_block
            .entry(key)
            .or_insert_with(BTreeSet::new)
            .insert(entry.horse.clone());
    }
    assert_eq!(horses_per_block.len(), 22 * 8 * PMU_REPETITIONS as usize);
    for horses in horses_per_block.values() {
        assert_eq!(horses.len(), 6, "each block must cover all six horses");
    }
    // Global interleaving: the first 24 entries must not be all one horse.
    let first: BTreeSet<&str> = entries_a
        .iter()
        .take(24)
        .map(|e| e.horse.as_str())
        .collect();
    assert!(
        first.len() >= 4,
        "entry shuffle must interleave horses, got {first:?}"
    );

    // Round-trip through the file format and the verifier.
    let bytes = schedule::schedule_file_bytes(&header_a, &entries_a);
    let (parsed_header, parsed_entries) =
        schedule::verify_schedule_file(&bytes).expect("verify round-trip");
    assert_eq!(parsed_header, header_a);
    assert_eq!(parsed_entries, entries_a);
    schedule::entries_match_panel(&parsed_entries, &panel).expect("entries match panel");

    // Tamper detection: one flipped byte in the entry lines must fail.
    let mut tampered = bytes.clone();
    let last = tampered.len() - 3;
    tampered[last] ^= 0x01;
    assert!(schedule::verify_schedule_file(&tampered).is_err());

    // Missing entry detection: drop the last ENTRY line (cut at the
    // second-to-last newline so the line itself disappears).
    let mut truncated = bytes.clone();
    let last_newline = truncated
        .iter()
        .rposition(|b| *b == b'\n')
        .expect("newline");
    let cut = truncated[..last_newline]
        .iter()
        .rposition(|b| *b == b'\n')
        .expect("second-to-last newline");
    truncated.truncate(cut + 1);
    assert!(schedule::verify_schedule_file(&truncated).is_err());
}

#[test]
fn panel_verification_enforces_hash_and_shape() {
    let panel = synthetic_panel();
    let (path, sha) = write_panel_file(&panel);
    panel::load_verified_panel(&path, &sha).expect("synthetic panel with its own hash verifies");

    let wrong = panel::load_verified_panel(&path, &"0".repeat(64));
    assert!(wrong.is_err());
    let message = wrong.err().unwrap();
    assert!(message.contains("PMU_PANEL_AUTHORITY = FAIL"), "{message}");

    // Class-count drift must fail even under a matching re-hash.
    let mut mutated = synthetic_panel();
    mutated.cells.pop();
    let (path2, sha2) = write_panel_file(&mutated);
    let error = panel::load_verified_panel(&path2, &sha2).expect_err("shape drift fails");
    assert!(error.contains("representative"), "{error}");
}

#[test]
fn observation_ids_are_deterministic_and_domain_separated() {
    use markit_mdbench_campaign::pmu::schema;
    let a = schema::pmu_observation_id("run", 1, "H0", "A1", 0);
    let b = schema::pmu_observation_id("run", 1, "H0", "A1", 0);
    let c = schema::pmu_observation_id("run", 1, "H0", "A1", 1);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.len(), 64);
    let r1 = schema::pmu_run_id("driver", "panel", "schedule", "host");
    let r2 = schema::pmu_run_id("driver2", "panel", "schedule", "host");
    assert_ne!(r1, r2);
}

/// The PMU driver's filtered loader must materialize a panel case
/// byte-identically to the full frozen load (task §4: source/edit
/// identity preserved). Heavy: performs the full hash-verified load.
#[test]
fn filtered_loader_matches_full_load() {
    let root = benchmark_root();
    let full = markit_mdbench_campaign::workload::load_campaign_workload(&root)
        .expect("full frozen workload loads");
    let clean_target = full
        .clean_state
        .first()
        .expect("a clean case")
        .payload_id
        .clone();
    let edit_target = full
        .edit_write
        .iter()
        .find(|case| case.trace_id.len() > 3)
        .expect("an edit case")
        .payload_id
        .clone();
    let mut filter = BTreeSet::new();
    filter.insert(clean_target.clone());
    filter.insert(edit_target.clone());
    let filtered =
        markit_mdbench_campaign::workload::load_campaign_workload_filtered(&root, &filter)
            .expect("filtered load");
    assert_eq!(filtered.clean_state.len(), 1);
    assert_eq!(filtered.edit_write.len(), 1);
    let fc = &filtered.clean_state[0];
    let fullc = &full.clean_state[0];
    assert_eq!(fc.case_id_hex, fullc.case_id_hex);
    assert_eq!(fc.payload_id, fullc.payload_id);
    assert_eq!(fc.source_key, fullc.source_key);
    assert_eq!(fc.source_id, fullc.source_id);
    assert_eq!(fc.file_bytes, fullc.file_bytes);
    assert_eq!(fc.source_sha256, fullc.source_sha256);
    assert_eq!(fc.source_text, fullc.source_text);
    let full_edit = full
        .edit_write
        .iter()
        .find(|case| case.payload_id == edit_target)
        .expect("target edit case in full load");
    // Field-by-field identity (EditWriteCase is not PartialEq; the
    // comparison must cover every identity-bearing field).
    let a = &filtered.edit_write[0];
    assert_eq!(a.case_id_hex, full_edit.case_id_hex);
    assert_eq!(a.payload_id, full_edit.payload_id);
    assert_eq!(a.trace_id, full_edit.trace_id);
    assert_eq!(a.source_id, full_edit.source_id);
    assert_eq!(a.source_path, full_edit.source_path);
    assert_eq!(a.edit_family, full_edit.edit_family);
    assert_eq!(a.expected_transition, full_edit.expected_transition);
    assert_eq!(a.pre_source_text, full_edit.pre_source_text);
    assert_eq!(a.post_source_text, full_edit.post_source_text);
    assert_eq!(format!("{:?}", a.edit), format!("{:?}", full_edit.edit));
    assert_eq!(a.pre_len_bytes, full_edit.pre_len_bytes);
    assert_eq!(a.logical_edited_bytes, full_edit.logical_edited_bytes);
}
