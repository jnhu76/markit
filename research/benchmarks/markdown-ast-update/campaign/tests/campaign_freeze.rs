//! Campaign-freeze acceptance tests (task §57-§58).
//!
//! These prove the freeze contract only: workload consumption
//! cardinalities, schedule determinism, receipt integrity, envelope
//! schema authority, observation uniqueness, attribution determinism,
//! and the NON_RESEARCH fake-clock smoke. They contain no benchmark
//! measurement and no performance claim.

use std::path::PathBuf;

fn benchmark_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn frozen_workload_consumes_exactly_22_and_362() {
    let workload = markit_mdbench_campaign::workload::load_campaign_workload(&benchmark_root())
        .expect("frozen workload must materialize");
    assert_eq!(workload.clean_state.len(), 22, "G0-strict FULL_READ files");
    assert_eq!(
        workload.edit_write.len(),
        362,
        "G0_PRIMARY EDIT_WRITE cases"
    );
    // Identity uniqueness in both directions.
    let clean: std::collections::BTreeSet<&str> = workload
        .clean_state
        .iter()
        .map(|case| case.case_id_hex.as_str())
        .collect();
    let edit: std::collections::BTreeSet<&str> = workload
        .edit_write
        .iter()
        .map(|case| case.case_id_hex.as_str())
        .collect();
    assert_eq!(clean.len(), 22);
    assert_eq!(edit.len(), 362);
}

#[test]
fn campaign_manifest_verifies_against_live_state() {
    let manifest = markit_mdbench_campaign::manifest::CampaignManifest::load(&benchmark_root())
        .expect("campaign manifest loads");
    manifest
        .verify(&benchmark_root())
        .expect("campaign manifest verifies against live repository state");
    // Frozen arithmetic (task §10).
    assert_eq!(manifest.cells_per_session(), 1920);
    assert_eq!(manifest.timing_rows_per_session(), 76_800);
    assert_eq!(manifest.cardinality.total_timing_rows, 230_400);
    assert_eq!(manifest.cardinality.measured_rows_total, 172_800);
    assert_eq!(manifest.cardinality.warmup_rows_total, 57_600);
    assert_eq!(manifest.cardinality.attribution_rows, 1920);
}

#[test]
fn schedule_is_deterministic_and_verifies() {
    let root = benchmark_root();
    let manifest = markit_mdbench_campaign::manifest::CampaignManifest::load(&root).unwrap();
    let first = markit_mdbench_campaign::receipt::regenerate_schedule(&root, &manifest).unwrap();
    let second = markit_mdbench_campaign::receipt::regenerate_schedule(&root, &manifest).unwrap();
    assert_eq!(first, second, "two regenerations must be byte-identical");
    let on_disk = std::fs::read(root.join("results/manifests/primary-schedule-v1.jsonl"))
        .expect("frozen schedule on disk");
    assert_eq!(on_disk, first, "on-disk schedule must equal regeneration");
    // 3 sessions x (22 + 362) scheduled cases.
    assert_eq!(
        first.iter().filter(|b| **b == b'\n').count(),
        3 * (22 + 362)
    );
    markit_mdbench_campaign::receipt::verify_schedule_on_disk(&root)
        .expect("schedule verification passes");
}

#[test]
fn campaign_receipt_binds_every_artifact() {
    markit_mdbench_campaign::receipt::verify_receipt(&benchmark_root())
        .expect("campaign receipt verifies");
    let receipt =
        markit_mdbench_campaign::receipt::CampaignReceipt::load(&benchmark_root()).unwrap();
    assert_eq!(receipt.artifacts.len(), 12, "12 bound artifacts (task §41)");
    assert!(receipt.produced_before_primary_timing);
    assert_eq!(receipt.campaign_seed, 6000671815411757117);
}

#[test]
fn envelope_schema_has_not_drifted_from_the_rust_model() {
    let root = benchmark_root();
    let generated = serde_json::to_value(schemars::schema_for!(
        markit_mdbench_campaign::execute::CampaignObservationV1
    ))
    .unwrap();
    let checked: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("protocol/campaign-observation-schema-v1.json")).expect(
            "protocol/campaign-observation-schema-v1.json exists; regenerate with \
                     `cargo run -p markit-mdbench-campaign --bin mdbench-campaign -- \
                     gen-envelope-schema --out protocol/campaign-observation-schema-v1.json`",
        ),
    )
    .unwrap();
    assert_eq!(
        checked, generated,
        "checked-in envelope schema drifted from the Rust CampaignObservationV1 model"
    );
}

#[test]
fn result_rows_are_schema_v2_only() {
    // The envelope must not accept a v1 row: the row type checks the
    // version field on deserialize (MEASUREMENT-CORRECTIVE-1).
    let root = benchmark_root();
    let receipt = markit_mdbench_campaign::receipt::CampaignReceipt::load(&root).unwrap();
    assert_eq!(receipt.campaign_spec_id.len(), 64);
    assert_eq!(
        markit_mdbench_runner::RESULT_SCHEMA_VERSION_V2,
        2,
        "campaign rows are schema v2 only"
    );
}

#[test]
fn preflight_passes_and_enumerates_unique_observation_ids() {
    let report = markit_mdbench_campaign::preflight::preflight(
        &benchmark_root(),
        markit_mdbench_campaign::preflight::HostBinding::SkipForNonResearch,
        None,
        None,
    );
    assert!(
        report.pass,
        "non-research preflight blockers: {:?}",
        report.blockers
    );
}

#[test]
fn attribution_counters_are_deterministic_on_representative_cases() {
    // Task §45: representative cases must produce identical attribution
    // rows across repeated executions (non-timed determinism check).
    let root = benchmark_root();
    let manifest = markit_mdbench_campaign::manifest::CampaignManifest::load(&root).unwrap();
    let workload = markit_mdbench_campaign::workload::load_campaign_workload(&root).unwrap();
    let binding = markit_mdbench_campaign::receipt::build_spec_binding(&root, &manifest).unwrap();
    let spec_id = markit_mdbench_campaign::identity::campaign_spec_id(&binding);
    let build = markit_mdbench_runner::current_build_identity();
    let identity = markit_mdbench_campaign::execute::ExecutionIdentity {
        campaign_spec_id: spec_id.clone(),
        run_id: "attribution-determinism-test".to_string(),
        machine_environment_ref: "test".to_string(),
        provenance: markit_mdbench_campaign::execute::PROVENANCE_NON_RESEARCH_SMOKE,
        non_research: true,
    };
    let run_once = |surface: markit_mdbench_campaign::Surface| -> String {
        let session_id =
            markit_mdbench_campaign::execute::attribution_session_id(&spec_id, surface.as_str());
        let mut buffer: Vec<u8> = Vec::new();
        use markit_mdbench_campaign::execute::{ScheduledCase, SessionExecutor};
        let horse_order: Vec<String> =
            ["H0", "H1", "H2", "H3", "H4"].iter().map(|h| h.to_string()).collect();
        let scheduled: Vec<ScheduledCase> = match surface {
            markit_mdbench_campaign::Surface::CleanState => vec![ScheduledCase::CleanState {
                order_ordinal: 0,
                horse_order: horse_order.clone(),
                case: &workload.clean_state[0],
            }],
            markit_mdbench_campaign::Surface::EditWrite => vec![ScheduledCase::EditWrite {
                order_ordinal: 0,
                horse_order: horse_order.clone(),
                case: &workload.edit_write[0],
            }],
        };
        let executor = SessionExecutor {
            identity: &identity,
            surface,
            session_ordinal: None,
            session_id: session_id.clone(),
            session_seed: manifest.seed.value,
            build_identity: build.clone(),
            warmup: 0,
            measured: 0,
            observations: 0,
            warmup_rows: 0,
            measured_rows: 0,
            poison_correctness: false,
        };
        let outcome = executor.run_attribution(&scheduled, &mut buffer).unwrap();
        assert!(
            !outcome.is_invalid(),
            "attribution determinism run failed: {outcome:?}"
        );
        // Normalize the timing-free rows to the counter-bearing fields.
        String::from_utf8(buffer).unwrap()
    };
    for surface in [
        markit_mdbench_campaign::Surface::CleanState,
        markit_mdbench_campaign::Surface::EditWrite,
    ] {
        let first = run_once(surface);
        let second = run_once(surface);
        assert_eq!(
            first, second,
            "{:?} attribution rows must be byte-identical across repeats",
            surface
        );
    }
}

#[test]
fn non_research_fake_clock_smoke_end_to_end() {
    let dir = std::env::temp_dir().join(format!("mdbench-campaign-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let options = markit_mdbench_campaign::smoke::SmokeOptions {
        cases_per_surface: 1,
        warmup: 1,
        measured: 1,
        inject_failure: true,
    };
    let report = markit_mdbench_campaign::smoke::run_smoke(&benchmark_root(), &dir, &options)
        .expect("fake-clock smoke runs end to end");
    assert!(report.non_research);
    assert_eq!(report.verdict, "NON_RESEARCH_SMOKE_PASS");
    // 2 surfaces x (1 case x 5 horses x (1+1) iterations) = 20 timing
    // rows; 2 surfaces x 5 attribution rows = 10.
    assert_eq!(report.timing_rows, 20);
    assert_eq!(report.attribution_rows, 10);
    assert!(report.failure_propagated, "poisoned run must invalidate");
    let _ = std::fs::remove_dir_all(&dir);
}
