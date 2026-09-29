//! RQ8 sensitivity-freeze acceptance tests (#33; R0 §4.6).
//!
//! These prove the FREEZE contract only — the second frozen profile is
//! the single intentional difference from the primary, the sensitivity
//! manifest is self-consistent, the projected schedule is a
//! deterministic restriction of the frozen primary schedule, and the
//! sensitivity expectation builder enforces its own contract. They
//! contain no benchmark measurement and no performance claim.

use std::collections::BTreeSet;
use std::path::PathBuf;

use markit_mdbench_campaign::schedule::{schedule_from_jsonl, ScheduleRow};
use markit_mdbench_campaign::sensitivity::{
    project_schedule, sensitivity_spec_id_from_root, SensitivityManifest,
    SENSITIVITY_SCHEDULE_PATH,
};

fn benchmark_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn load_manifest() -> SensitivityManifest {
    SensitivityManifest::load(&benchmark_root()).expect("sensitivity manifest parses")
}

/// The workspace expresses BOTH frozen profiles exactly: the primary is
/// untouched (thin LTO etc.) and the second profile inherits release
/// with `lto = false` as the only difference.
#[test]
fn second_profile_is_single_factor_and_primary_untouched() {
    let cargo_toml = std::fs::read_to_string(benchmark_root().join("Cargo.toml")).unwrap();
    let workspace: toml::Value = toml::from_str(&cargo_toml).unwrap();
    let profiles = workspace.get("profile").expect("[profile] tables");
    let release = profiles.get("release").expect("[profile.release]");
    assert_eq!(
        release.get("opt-level").and_then(|v| v.as_integer()),
        Some(3)
    );
    assert_eq!(release.get("lto").and_then(|v| v.as_str()), Some("thin"));
    assert_eq!(
        release.get("codegen-units").and_then(|v| v.as_integer()),
        Some(1)
    );
    assert_eq!(
        release.get("incremental").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        release.get("panic").and_then(|v| v.as_str()),
        Some("unwind")
    );

    let second = profiles
        .get(markit_mdbench_runner::build_identity::SENSITIVITY_LTO_OFF_PROFILE_ID)
        .expect("the second frozen profile exists in the workspace");
    assert_eq!(
        second.get("inherits").and_then(|v| v.as_str()),
        Some("release"),
        "the second profile must inherit the primary"
    );
    assert_eq!(
        second.get("lto").and_then(|v| v.as_bool()),
        Some(false),
        "the ONLY intentional difference is lto = false"
    );
    for inherited in ["opt-level", "codegen-units", "incremental", "panic"] {
        assert!(
            second.get(inherited).is_none(),
            "the second profile must inherit {inherited} unchanged, not restate it"
        );
    }
}

/// The frozen profile manifest and environment record agree with the
/// workspace profile and the runner constants.
#[test]
fn profile_manifest_and_environment_agree() {
    let manifest_text =
        std::fs::read_to_string(benchmark_root().join("manifest/rq8-sensitivity-profile-v1.toml"))
            .unwrap();
    let manifest: toml::Value = toml::from_str(&manifest_text).unwrap();
    assert_eq!(
        manifest.get("profile_id").and_then(|v| v.as_str()),
        Some(markit_mdbench_runner::build_identity::SENSITIVITY_LTO_OFF_PROFILE_ID)
    );
    let profile = manifest.get("profile").expect("[profile]");
    assert_eq!(
        profile.get("opt_level").and_then(|v| v.as_integer()),
        Some(3)
    );
    assert_eq!(profile.get("lto").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        profile.get("codegen_units").and_then(|v| v.as_integer()),
        Some(1)
    );
    assert_eq!(
        profile.get("incremental").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        profile.get("panic").and_then(|v| v.as_str()),
        Some("unwind")
    );
    assert_eq!(
        profile.get("target_cpu").and_then(|v| v.as_str()),
        Some("default")
    );
    assert_eq!(profile.get("rustflags").and_then(|v| v.as_str()), Some(""));
    let difference = manifest.get("difference").expect("[difference]");
    assert_eq!(
        difference.get("dimension").and_then(|v| v.as_str()),
        Some("lto")
    );
    assert_eq!(
        difference.get("primary_value").and_then(|v| v.as_str()),
        Some("thin")
    );
    assert_eq!(
        difference.get("second_value").and_then(|v| v.as_str()),
        Some("false")
    );

    let environment =
        std::fs::read_to_string(benchmark_root().join("manifest/environment.toml")).unwrap();
    assert!(
        environment.contains(markit_mdbench_runner::build_identity::SENSITIVITY_LTO_OFF_PROFILE_ID),
        "environment.toml must record the frozen second profile"
    );
    assert!(
        !environment
            .to_string()
            .contains("second_profile = \"RESERVED"),
        "the reserved slot must be replaced by the frozen profile"
    );
}

/// The frozen sensitivity manifest is self-consistent (matrix, horse
/// sets, case lists, cardinality arithmetic).
#[test]
fn sensitivity_manifest_self_consistent() {
    let manifest = load_manifest();
    manifest.verify().expect("the frozen manifest must verify");
}

/// The sensitivity spec id is deterministic, distinct from the primary,
/// and derived from the pinned primary authority.
#[test]
fn sensitivity_spec_id_deterministic_and_distinct() {
    let (spec_id, _) = sensitivity_spec_id_from_root(&benchmark_root())
        .expect("pinned primary artifacts verify against live state");
    let manifest = load_manifest();
    assert_ne!(
        spec_id, manifest.authority.primary_campaign_spec_id,
        "the sensitivity campaign must have its own spec identity"
    );
    // Re-derivation is byte-stable.
    let (again, _) = sensitivity_spec_id_from_root(&benchmark_root()).unwrap();
    assert_eq!(spec_id, again);
}

/// The projected schedule is a deterministic restriction of the frozen
/// primary schedule: same case set per (surface, session), order
/// ordinals preserved, horse orders are order-preserving subsets, the
/// on-disk freeze is byte-identical to a fresh projection.
#[test]
fn schedule_projection_deterministic_subset_of_primary() {
    let root = benchmark_root();
    let manifest = load_manifest();
    let (spec_id, _) = sensitivity_spec_id_from_root(&root).unwrap();
    let primary_bytes = std::fs::read(root.join("results/manifests/six-horse-schedule-v1.jsonl"))
        .expect("frozen primary schedule");
    let primary = schedule_from_jsonl(&primary_bytes).unwrap();

    let first = project_schedule(&primary, &manifest, &spec_id).unwrap();
    let second = project_schedule(&primary, &manifest, &spec_id).unwrap();
    assert_eq!(
        markit_mdbench_campaign::schedule::schedule_to_jsonl(&first).unwrap(),
        markit_mdbench_campaign::schedule::schedule_to_jsonl(&second).unwrap(),
        "projection must be byte-deterministic"
    );

    // Same case set per (surface, session) as the primary.
    use std::collections::BTreeMap;
    let mut primary_cases: BTreeMap<(String, u32), BTreeSet<&str>> = BTreeMap::new();
    for row in &primary {
        primary_cases
            .entry((row.surface.clone(), row.session_ordinal))
            .or_default()
            .insert(row.case_id.as_str());
    }
    let mut projected_cases: BTreeMap<(String, u32), BTreeSet<&str>> = BTreeMap::new();
    for row in &first {
        projected_cases
            .entry((row.surface.clone(), row.session_ordinal))
            .or_default()
            .insert(row.case_id.as_str());
    }
    let keys: BTreeSet<_> = primary_cases.keys().cloned().collect();
    for key in keys {
        assert_eq!(
            primary_cases.get(&key),
            projected_cases.get(&key),
            "projection must keep every primary case of {key:?}"
        );
    }

    // Per-row invariants: identity fields verbatim, ordinals preserved,
    // horse order = order-preserving subset of the primary horse order.
    let primary_by_key: BTreeMap<_, _> = primary
        .iter()
        .map(|row| {
            (
                (
                    row.surface.clone(),
                    row.session_ordinal,
                    row.case_id.clone(),
                ),
                row,
            )
        })
        .collect();
    let roster: BTreeSet<&str> = markit_mdbench_campaign::HORSE_IDS.iter().copied().collect();
    let full_roster_cases = manifest.full_roster_edit_cases();
    for row in &first {
        let source = primary_by_key
            .get(&(
                row.surface.clone(),
                row.session_ordinal,
                row.case_id.clone(),
            ))
            .expect("projected row exists in primary");
        assert_eq!(row.order_ordinal, source.order_ordinal);
        assert_eq!(row.payload_id, source.payload_id);
        assert_eq!(row.source_key, source.source_key);
        assert_eq!(row.trace_id, source.trace_id);
        let subset: Vec<&str> = source
            .horse_order
            .iter()
            .filter(|horse| row.horse_order.contains(horse))
            .map(|horse| horse.as_str())
            .collect();
        assert_eq!(
            subset,
            row.horse_order
                .iter()
                .map(|h| h.as_str())
                .collect::<Vec<_>>(),
            "horse order must be the primary order restricted, never reordered"
        );
        let unique: BTreeSet<&str> = row.horse_order.iter().map(|h| h.as_str()).collect();
        assert!(unique.iter().all(|horse| roster.contains(horse)));
        match row.surface.as_str() {
            "clean_state" => assert_eq!(unique.len(), 6, "clean_state keeps the full roster"),
            "edit_write" => {
                if full_roster_cases.contains(row.case_id.as_str()) {
                    assert_eq!(unique.len(), 6, "K2/K4 subset cases keep the full roster");
                } else {
                    assert_eq!(
                        unique,
                        manifest
                            .horses
                            .population
                            .iter()
                            .map(|h| h.as_str())
                            .collect::<BTreeSet<_>>(),
                        "population cases carry exactly the population horses"
                    );
                }
            }
            other => panic!("unexpected surface {other:?}"),
        }
    }

    // The on-disk frozen schedule is byte-identical to the projection.
    let on_disk = std::fs::read(root.join(SENSITIVITY_SCHEDULE_PATH)).unwrap();
    assert_eq!(
        on_disk,
        markit_mdbench_campaign::schedule::schedule_to_jsonl(&first).unwrap(),
        "the frozen schedule must equal a fresh projection"
    );
}

/// The sensitivity expectation builder enforces its own contract:
/// horse-subset rows are accepted with exact subset cardinalities;
/// foreign horses, wrong sessions, and count mismatches are rejected.
#[test]
fn sensitivity_expectation_accepts_subsets_and_rejects_drift() {
    use markit_mdbench_campaign::finalize::expectation_from_sensitivity_schedule;
    let mk_row = |session: u32, case: &str, horses: &[&str]| ScheduleRow {
        schema: markit_mdbench_campaign::SCHEDULE_SCHEMA.to_string(),
        campaign_spec_id: "spec".to_string(),
        surface: "edit_write".to_string(),
        session_ordinal: session,
        order_ordinal: 0,
        case_id: case.to_string(),
        payload_id: "p".to_string(),
        source_key: "s".to_string(),
        trace_id: None,
        horse_order: horses.iter().map(|h| h.to_string()).collect(),
    };
    let rows = [
        mk_row(0, &"a".repeat(64), &["H0", "H4", "HorseA", "H1"]),
        mk_row(
            0,
            &"b".repeat(64),
            &["H0", "H1", "H2", "H3", "H4", "HorseA"],
        ),
    ];
    let refs: Vec<&ScheduleRow> = rows.iter().collect();
    let expectation = expectation_from_sensitivity_schedule(
        &refs,
        "spec",
        "run",
        "session",
        markit_mdbench_campaign::Surface::EditWrite,
        0,
        10,
        30,
        2,
        4 * 40 + 6 * 40,
    )
    .expect("valid subset schedule builds an expectation");
    assert_eq!(expectation.expected_rows(), 400);
    // Wrong session rejected.
    assert!(expectation_from_sensitivity_schedule(
        &refs,
        "spec",
        "run",
        "session",
        markit_mdbench_campaign::Surface::EditWrite,
        1,
        10,
        30,
        2,
        400
    )
    .is_err());
    // Wrong case count rejected.
    assert!(expectation_from_sensitivity_schedule(
        &refs,
        "spec",
        "run",
        "session",
        markit_mdbench_campaign::Surface::EditWrite,
        0,
        10,
        30,
        3,
        400
    )
    .is_err());
    // Wrong row cardinality rejected.
    assert!(expectation_from_sensitivity_schedule(
        &refs,
        "spec",
        "run",
        "session",
        markit_mdbench_campaign::Surface::EditWrite,
        0,
        10,
        30,
        2,
        399
    )
    .is_err());
    // Horse outside the frozen roster rejected.
    let bad = [mk_row(0, &"a".repeat(64), &["H0", "H9"])];
    let bad_refs: Vec<&ScheduleRow> = bad.iter().collect();
    assert!(expectation_from_sensitivity_schedule(
        &bad_refs,
        "spec",
        "run",
        "session",
        markit_mdbench_campaign::Surface::EditWrite,
        0,
        10,
        30,
        1,
        80
    )
    .is_err());
    // Empty horse order rejected.
    let empty = [mk_row(0, &"a".repeat(64), &[])];
    let empty_refs: Vec<&ScheduleRow> = empty.iter().collect();
    assert!(expectation_from_sensitivity_schedule(
        &empty_refs,
        "spec",
        "run",
        "session",
        markit_mdbench_campaign::Surface::EditWrite,
        0,
        10,
        30,
        1,
        0
    )
    .is_err());
}
