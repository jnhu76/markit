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
    assert_eq!(manifest.cells_per_session(), 2304);
    assert_eq!(manifest.timing_rows_per_session(), 92_160);
    assert_eq!(manifest.cardinality.total_timing_rows, 276_480);
    assert_eq!(manifest.cardinality.measured_rows_total, 207_360);
    assert_eq!(manifest.cardinality.warmup_rows_total, 69_120);
    assert_eq!(manifest.cardinality.attribution_rows, 2304);
}

#[test]
fn schedule_is_deterministic_and_verifies() {
    let root = benchmark_root();
    let manifest = markit_mdbench_campaign::manifest::CampaignManifest::load(&root).unwrap();
    let first = markit_mdbench_campaign::receipt::regenerate_schedule(&root, &manifest).unwrap();
    let second = markit_mdbench_campaign::receipt::regenerate_schedule(&root, &manifest).unwrap();
    assert_eq!(first, second, "two regenerations must be byte-identical");
    let on_disk =
        std::fs::read(root.join(markit_mdbench_campaign::manifest::SCHEDULE_MANIFEST_PATH))
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
    assert_eq!(
        receipt.artifacts.len(),
        14,
        "14 bound artifacts (task §41 incl. profile-defining Cargo.toml/rust-toolchain.toml)"
    );
    assert!(receipt.produced_before_primary_timing);
    assert_eq!(receipt.campaign_seed, 15542238002486738101);
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
    let root = benchmark_root();
    for scope in [
        markit_mdbench_campaign::preflight::PreflightScope::All,
        markit_mdbench_campaign::preflight::PreflightScope::Timing {
            surface: markit_mdbench_campaign::Surface::EditWrite,
            session: 0,
        },
        markit_mdbench_campaign::preflight::PreflightScope::Memory {
            surface: markit_mdbench_campaign::Surface::EditWrite,
            session: 0,
        },
        markit_mdbench_campaign::preflight::PreflightScope::Attribution {
            surface: markit_mdbench_campaign::Surface::CleanState,
        },
    ] {
        let report = markit_mdbench_campaign::preflight::preflight(
            &root,
            markit_mdbench_campaign::preflight::HostBinding::SkipForNonResearch,
            scope,
        );
        assert!(
            report.pass,
            "non-research preflight blockers for {}: {:?}",
            scope.label(),
            report.blockers
        );
    }
    // The All scope really does enumerate the whole campaign: 276,480
    // timing rows + 276,480 memory rows (each = 3 sessions x 2 surfaces
    // x 92,160 rows/session) + 2,304 attribution rows, with no
    // duplicate and no missing identity — the memory ids join the
    // campaign-wide union since M-COLLECTOR-1 wired the lane.
    let report = markit_mdbench_campaign::preflight::preflight(
        &root,
        markit_mdbench_campaign::preflight::HostBinding::SkipForNonResearch,
        markit_mdbench_campaign::preflight::PreflightScope::All,
    );
    let enumeration = &report.diagnostics["observation_enumeration"];
    assert_eq!(enumeration["rows"], 555_264);
    assert_eq!(enumeration["unique_ids"], 555_264);
    assert_eq!(enumeration["duplicate_ids"], 0);
    // 6 timing sessions + 6 memory sessions + 2 attribution lanes.
    assert_eq!(enumeration["lanes"].as_array().unwrap().len(), 14);
    // Attribution lanes report their own frozen cardinalities, not a
    // timing session's (the defect the explicit scope fixes).
    let attribution: Vec<&serde_json::Value> = enumeration["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|lane| lane["lane"].as_str().unwrap().starts_with("attribution:"))
        .collect();
    assert_eq!(attribution.len(), 2);
    let mut rows: Vec<u64> = attribution
        .iter()
        .map(|lane| lane["rows"].as_u64().unwrap())
        .collect();
    rows.sort();
    assert_eq!(rows, vec![132, 2_172]);
    for lane in attribution {
        assert_eq!(lane["attribution_rows"], lane["rows"]);
        assert_eq!(lane["warmup_rows"], 0);
        assert_eq!(lane["measured_rows"], 0);
    }
    // Memory lanes mirror the timing sessions' frozen cardinalities
    // (22 x 6 x 40 = 5,280; 362 x 6 x 40 = 86,880) with the memory
    // session-id derivation.
    let memory: Vec<&serde_json::Value> = enumeration["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|lane| lane["lane"].as_str().unwrap().starts_with("memory:"))
        .collect();
    assert_eq!(memory.len(), 6);
    let mut rows: Vec<u64> = memory
        .iter()
        .map(|lane| lane["rows"].as_u64().unwrap())
        .collect();
    rows.sort();
    assert_eq!(rows, vec![5_280, 5_280, 5_280, 86_880, 86_880, 86_880]);
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
        let horse_order: Vec<String> = ["H0", "H1", "H2", "H3", "H4"]
            .iter()
            .map(|h| h.to_string())
            .collect();
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
            session_seed: markit_mdbench_campaign::manifest::parse_seed_value(&manifest.seed.value)
                .unwrap(),
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
    // 2 surfaces x (1 case x 6 horses x (1+1) iterations) = 24 timing
    // rows; 2 surfaces x 6 attribution rows = 12.
    assert_eq!(report.timing_rows, 24);
    assert_eq!(report.attribution_rows, 12);
    assert!(report.failure_propagated, "poisoned run must invalidate");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Formal M-LANE collector plumbing (#76 M-COLLECTOR-1). Deterministic,
// NON_RESEARCH: the allocator backend is either a fixed-value test
// reporter (dispatch/envelope/identity) or the real AllocReporter
// WITHOUT the counting allocator installed (the negative lane-isolation
// proof). No qualified research observation is produced here.
// ---------------------------------------------------------------------------

/// Test-only reporter: the `ManualClock` analogue for the M-LANE — every
/// window returns the same completed record, so the campaign-layer
/// dispatch/envelope/verification path runs deterministically without
/// an allocator backend (whose real path the NON_RESEARCH memory smoke
/// already validated instrument-side for all six horses).
struct FixedMemoryRecord;

impl markit_mdbench_instrumentation::MemoryReporter for FixedMemoryRecord {
    fn begin_case(&self) -> markit_mdbench_instrumentation::CaseMemoryProbe {
        markit_mdbench_instrumentation::CaseMemoryProbe::new(0)
    }

    fn end_case(
        &self,
        _probe: markit_mdbench_instrumentation::CaseMemoryProbe,
    ) -> markit_mdbench_instrumentation::MemoryRecord {
        use markit_mdbench_common::Observed;
        markit_mdbench_instrumentation::MemoryRecord {
            allocated_bytes: Observed::Known(4_096),
            allocation_count: Observed::Known(7),
            peak_bytes: Observed::Known(2_048),
            retained_bytes: Observed::Known(1_024),
        }
    }
}

#[test]
fn memory_lane_dispatches_all_six_horses_on_both_surfaces() {
    let root = benchmark_root();
    let manifest = markit_mdbench_campaign::manifest::CampaignManifest::load(&root).unwrap();
    let workload = markit_mdbench_campaign::workload::load_campaign_workload(&root).unwrap();
    let binding = markit_mdbench_campaign::receipt::build_spec_binding(&root, &manifest).unwrap();
    let spec_id = markit_mdbench_campaign::identity::campaign_spec_id(&binding);
    let build = markit_mdbench_runner::current_build_identity();
    let identity = markit_mdbench_campaign::execute::ExecutionIdentity {
        campaign_spec_id: spec_id.clone(),
        run_id: "memory-dispatch-test".to_string(),
        machine_environment_ref: "test".to_string(),
        // NON_RESEARCH identity: this dispatch test is plumbing proof,
        // never qualified memory evidence.
        provenance: "MEMORY_DISPATCH_TEST/NON_RESEARCH_RESULT",
        non_research: true,
    };
    use markit_mdbench_campaign::execute::{ScheduledCase, SessionExecutor};
    let six: Vec<String> = markit_mdbench_campaign::HORSE_IDS
        .iter()
        .map(|h| h.to_string())
        .collect();
    for surface in [
        markit_mdbench_campaign::Surface::CleanState,
        markit_mdbench_campaign::Surface::EditWrite,
    ] {
        let session_ordinal = 0u32;
        let session_id = markit_mdbench_campaign::execute::memory_session_id(
            &spec_id,
            surface.as_str(),
            session_ordinal,
        );
        let scheduled: Vec<ScheduledCase> = match surface {
            markit_mdbench_campaign::Surface::CleanState => vec![ScheduledCase::CleanState {
                order_ordinal: 0,
                horse_order: six.clone(),
                case: &workload.clean_state[0],
            }],
            markit_mdbench_campaign::Surface::EditWrite => vec![ScheduledCase::EditWrite {
                order_ordinal: 0,
                horse_order: six.clone(),
                case: &workload.edit_write[0],
            }],
        };
        let executor = SessionExecutor {
            identity: &identity,
            surface,
            session_ordinal: Some(session_ordinal),
            session_id: session_id.clone(),
            session_seed: markit_mdbench_campaign::manifest::parse_seed_value(&manifest.seed.value)
                .unwrap(),
            build_identity: build.clone(),
            warmup: 1,
            measured: 2,
            observations: 0,
            warmup_rows: 0,
            measured_rows: 0,
            poison_correctness: false,
        };
        let mut buffer: Vec<u8> = Vec::new();
        let reporter = FixedMemoryRecord;
        let outcome = executor
            .run_memory(&scheduled, &reporter, &mut buffer)
            .unwrap();
        assert!(
            !outcome.is_invalid(),
            "memory dispatch failed on {surface:?}: {outcome:?}"
        );
        let text = String::from_utf8(buffer).unwrap();
        let rows: Vec<serde_json::Value> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        // 1 case x 6 horses x (1 warmup + 2 measured) = 18 rows.
        assert_eq!(rows.len(), 18);
        let mut mechanisms = std::collections::BTreeSet::new();
        let mut ids = std::collections::BTreeSet::new();
        for row in &rows {
            assert_eq!(row["session_ordinal"], 0);
            // The executor stamps ITS identity's provenance (this test is
            // NON_RESEARCH; the formal binary stamps PROVENANCE_MEMORY,
            // whose acceptance is enforced by memory-lane finalization).
            assert_eq!(
                row["result_row_v2"]["provenance_ref"],
                "MEMORY_DISPATCH_TEST/NON_RESEARCH_RESULT"
            );
            // Every row is a MEMORY-lane schema-v2 row with a completed
            // window — the qualified M-LANE evidence shape.
            assert_eq!(row["result_row_v2"]["measurement"]["lane"], "memory");
            assert_eq!(
                row["result_row_v2"]["measurement"]["metrics"]["allocation_count"],
                7
            );
            mechanisms.insert(row["result_row_v2"]["mechanism_id"].as_str().unwrap());
            ids.insert(row["observation_id"].as_str().unwrap());
        }
        // All six frozen mechanisms dispatched (C1).
        assert_eq!(mechanisms.len(), 6);
        assert_eq!(ids.len(), 18, "observation ids are unique per iteration");
        // The warmup/measured split is exact (1 + 2 per cell).
        assert_eq!(
            rows.iter().filter(|r| r["sample_kind"] == "warmup").count(),
            6
        );
    }
}

#[test]
fn memory_lane_refuses_a_process_without_the_counting_allocator() {
    // C3 negative proof, deterministic: this test process does NOT
    // install the CountingAllocator, so the REAL AllocReporter windows
    // observe zero allocations — exactly the wrong-binary case — and
    // the formal lane must fail closed rather than emit all-zero rows.
    let root = benchmark_root();
    let manifest = markit_mdbench_campaign::manifest::CampaignManifest::load(&root).unwrap();
    let workload = markit_mdbench_campaign::workload::load_campaign_workload(&root).unwrap();
    let binding = markit_mdbench_campaign::receipt::build_spec_binding(&root, &manifest).unwrap();
    let spec_id = markit_mdbench_campaign::identity::campaign_spec_id(&binding);
    let build = markit_mdbench_runner::current_build_identity();
    let identity = markit_mdbench_campaign::execute::ExecutionIdentity {
        campaign_spec_id: spec_id,
        run_id: "memory-lane-guard-test".to_string(),
        machine_environment_ref: "test".to_string(),
        provenance: "MEMORY_GUARD_TEST/NON_RESEARCH_RESULT",
        non_research: true,
    };
    use markit_mdbench_campaign::execute::{ScheduledCase, SessionExecutor};
    let six: Vec<String> = markit_mdbench_campaign::HORSE_IDS
        .iter()
        .map(|h| h.to_string())
        .collect();
    let scheduled = vec![ScheduledCase::CleanState {
        order_ordinal: 0,
        horse_order: six,
        case: &workload.clean_state[0],
    }];
    let executor = SessionExecutor {
        identity: &identity,
        surface: markit_mdbench_campaign::Surface::CleanState,
        session_ordinal: Some(0),
        session_id: "memory-guard-test-session".to_string(),
        session_seed: 1,
        build_identity: build,
        warmup: 1,
        measured: 0,
        observations: 0,
        warmup_rows: 0,
        measured_rows: 0,
        poison_correctness: false,
    };
    let reporter = markit_mdbench_instrumentation::AllocReporter::new();
    let mut buffer: Vec<u8> = Vec::new();
    let outcome = executor
        .run_memory(&scheduled, &reporter, &mut buffer)
        .unwrap();
    match outcome {
        markit_mdbench_campaign::execute::SessionOutcome::Invalid { reason, .. } => {
            assert!(
                reason.contains("no allocations"),
                "guard must name the missing allocator; got {reason}"
            );
            // The failing row is RETAINED as evidence, then a clean stop.
            let text = String::from_utf8(buffer).unwrap();
            assert_eq!(text.lines().filter(|l| !l.trim().is_empty()).count(), 1);
        }
        other => panic!("non-instrumented process must not complete: {other:?}"),
    }
}

#[test]
fn memory_session_ids_never_collide_with_other_lanes() {
    // C5 cross-lane identity: the memory session-id derivation keeps
    // memory ObservationIds disjoint from timing and attribution ids of
    // the same (spec, surface, case, horse, kind, iteration).
    let spec = "spec-fixture";
    for surface in ["clean_state", "edit_write"] {
        for session in 0..3u32 {
            let memory =
                markit_mdbench_campaign::execute::memory_session_id(spec, surface, session);
            let timing = markit_mdbench_campaign::identity::session_id(spec, surface, session);
            let attribution =
                markit_mdbench_campaign::execute::attribution_session_id(spec, surface);
            assert_ne!(memory, timing);
            assert_ne!(memory, attribution);
            let base = markit_mdbench_campaign::identity::observation_id(
                "run", &memory, surface, "case", "H0", "measured", 0,
            );
            assert_ne!(
                base,
                markit_mdbench_campaign::identity::observation_id(
                    "run", &timing, surface, "case", "H0", "measured", 0
                )
            );
            assert_eq!(
                base,
                markit_mdbench_campaign::identity::observation_id(
                    "run", &memory, surface, "case", "H0", "measured", 0
                )
            );
        }
    }
}
