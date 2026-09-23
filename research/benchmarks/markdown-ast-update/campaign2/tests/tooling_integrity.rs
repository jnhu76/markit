//! Campaign-2 tooling integrity tests.
//!
//! These prove the tooling — not the horses — is trustworthy:
//!
//! - the chained lifecycle step uses the SAME timer boundary and produces
//!   the SAME execution/correctness/checksum facts as the frozen
//!   `run_update_timed` for the same inputs;
//! - every controlled document is byte-exact and BENCH-GRAMMAR-v1
//!   admissible (H0 clean parse validates against NORMALIZED-RESULT-v1);
//! - every frozen lifecycle trace replays to its recorded post-source
//!   digests;
//! - identities are deterministic and sensitive to their bindings.

use markit_mdbench_campaign2::exec::{reference_for, run_update_chain_step, CaseSpec};
use markit_mdbench_campaign2::generators::{self, Axis};
use markit_mdbench_campaign2::lifecycle::{self, TracePlan};
use markit_mdbench_common::{Source, SourceId};
use markit_mdbench_instrumentation::InstantClock;
use markit_mdbench_oracle::ReferenceOracle;

fn controlled_case(axis: Axis, label: &str) -> generators::ControlledCase {
    generators::generate_cell(axis, label).expect("controlled cell generates")
}

#[test]
fn generators_are_byte_exact_and_admissible() {
    for axis in [Axis::N, Axis::B, Axis::D, Axis::F, Axis::K] {
        let cases = generators::generate_axis(axis).expect("axis generates");
        assert_eq!(cases.len(), axis.points().len());
        for case in &cases {
            generators::verify_case(case).expect("generator self-check");
            reference_for(&case.pre_source)
                .unwrap_or_else(|e| panic!("{} {}: pre inadmissible: {e}", axis.as_str(), case.axis_label));
            reference_for(&case.post_source)
                .unwrap_or_else(|e| panic!("{} {}: post inadmissible: {e}", axis.as_str(), case.axis_label));
        }
    }
}

#[test]
fn controlled_edits_are_fixed_size_across_each_axis() {
    // The task fixes "the same edit bytes" inside an axis. N/K replace by
    // an 8-byte insertion; B replaces by an 8-byte insertion; D removes
    // exactly three backticks; F removes exactly the 32-byte definition.
    let n = generators::generate_axis(Axis::N).unwrap();
    for case in &n {
        assert_eq!(case.post_source.len() as i64 - case.pre_source.len() as i64, 8);
    }
    let b = generators::generate_axis(Axis::B).unwrap();
    for case in &b {
        assert_eq!(case.post_source.len() as i64 - case.pre_source.len() as i64, 8);
    }
    let d = generators::generate_axis(Axis::D).unwrap();
    for case in &d {
        assert_eq!(case.pre_source.len() as i64 - case.post_source.len() as i64, 3);
        assert_eq!(case.edit_start, generators::C_D_EDIT_OFFSET as u64);
    }
    let f = generators::generate_axis(Axis::F).unwrap();
    for case in &f {
        assert_eq!(case.pre_source.len() as i64 - case.post_source.len() as i64, 32);
    }
    let k = generators::generate_axis(Axis::K).unwrap();
    for case in &k {
        assert_eq!(case.post_source.len() as i64 - case.pre_source.len() as i64, 8);
    }
}

#[test]
fn chain_step_matches_the_frozen_runner_facts() {
    // The chained lifecycle step is the only NEW timed path in Campaign-2.
    // It must agree with the frozen `run_update_timed` on every fact that
    // is not the wall-clock value itself.
    let case = controlled_case(Axis::N, "128KiB");
    let spec = CaseSpec::update(
        case.case_id,
        case.case_id_hex.clone(),
        case.cell_id.clone(),
        case.pre_source.clone(),
        case.post_source.clone(),
        case.edit.clone(),
    );
    let (pre, post) = spec.sources();
    let reference = reference_for(&case.post_source).expect("reference");
    let clock = InstantClock::new();
    let facts = spec.facts(markit_mdbench_full_rebuild::H0_MECHANISM_ID, 0);
    let build = markit_mdbench_runner::current_build_identity();
    let _ = facts;
    let _ = build;

    markit_mdbench_campaign2::with_horse!("H3", |mech| {
        let hook = ReferenceOracle::new(reference.clone());
        let frozen_state = markit_mdbench_runner::orchestrate::build_initial_state(&mech, &pre)
            .expect("fresh state for the frozen runner");
        let frozen = markit_mdbench_runner::orchestrate::run_update_timed(
            &mech,
            &pre,
            &post,
            &case.edit,
            frozen_state,
            &clock,
            &hook,
        );
        let chain_state = markit_mdbench_runner::orchestrate::build_initial_state(&mech, &pre)
            .expect("fresh state for the chained step");
        let hook2 = ReferenceOracle::new(reference.clone());
        let chained =
            run_update_chain_step(&mech, &pre, &post, &case.edit, chain_state, &clock, &hook2);
        assert_eq!(chained.report.execution_status, frozen.execution_status);
        assert_eq!(chained.report.correctness_status, frozen.correctness_status);
        assert_eq!(chained.report.result_checksum, frozen.result_checksum);
        assert!(chained.new_state.is_some(), "a passing step seals a new state");
        Ok::<(), String>(())
    })
    .expect("H3 exists");
}

#[test]
fn chained_state_advances_across_steps() {
    // Two chained steps must behave like two independent (pre, post) pairs
    // — this is what makes K = 128 a valid lifecycle rather than 128
    // restarts.
    let plan = lifecycle::controlled_plans()
        .into_iter()
        .find(|p| p.family() == "L1")
        .expect("L1 plan exists");
    let initial = plan.initial_source();
    let mut current = initial.clone();
    let mut steps = Vec::new();
    for step in 0..4u32 {
        let (edit, _, _) = plan.next_edit(step, &current).expect("trace edit");
        let pre = Source::new(SourceId(0), current.clone());
        let post = edit.apply(&pre, SourceId(1)).expect("apply").as_str().to_string();
        steps.push((pre, Source::new(SourceId(1), post.clone()), edit, current.clone()));
        current = post;
    }
    let clock = InstantClock::new();
    markit_mdbench_campaign2::with_horse!("H1", |mech| {
        let mut state = markit_mdbench_runner::orchestrate::build_initial_state(
            &mech,
            &Source::new(SourceId(9), initial.clone()),
        )
        .expect("initial state");
        for (pre, post, edit, expected_pre) in &steps {
            assert_eq!(pre.as_str(), expected_pre.as_str(), "chain pre-source");
            let reference = reference_for(post.as_str()).expect("reference");
            let hook = ReferenceOracle::new(reference);
            let outcome = run_update_chain_step(&mech, pre, post, edit, state, &clock, &hook);
            assert_eq!(
                outcome.report.correctness_status,
                markit_mdbench_common::CorrectnessStatus::Pass,
                "step correctness"
            );
            state = outcome.new_state.expect("sealed state");
        }
        Ok::<(), String>(())
    })
    .expect("H1 exists");
}

#[test]
fn lifecycle_traces_replay_to_their_frozen_digests() {
    for plan in lifecycle::controlled_plans() {
        let trace = lifecycle::materialize(plan.as_ref()).expect("materialize");
        let mut current = plan.initial_source();
        assert_eq!(
            markit_mdbench_campaign2::sha256_hex(current.as_bytes()),
            trace.initial_source_sha256,
            "{}: initial digest",
            trace.trace_id
        );
        for step in &trace.steps {
            let edit = step.edit();
            let pre = Source::new(SourceId(0), current.clone());
            let post = edit.apply(&pre, SourceId(1)).expect("apply").as_str().to_string();
            assert_eq!(
                markit_mdbench_campaign2::sha256_hex(post.as_bytes()),
                step.post_source_sha256,
                "{} step {}: post digest",
                trace.trace_id,
                step.step
            );
            let expected = lifecycle::expected_checksum(&post).expect("expected checksum");
            assert_eq!(
                format!("{expected:016x}"),
                step.expected_checksum,
                "{} step {}: expected checksum",
                trace.trace_id,
                step.step
            );
            current = post;
        }
        assert_eq!(trace.steps.len() as u32, lifecycle::TRACE_STEPS);
    }
}

#[test]
fn identities_are_deterministic_and_binding_sensitive() {
    let study = markit_mdbench_campaign2::identity::study_id("aaaa");
    assert_eq!(study, markit_mdbench_campaign2::identity::study_id("aaaa"));
    assert_ne!(study, markit_mdbench_campaign2::identity::study_id("bbbb"));

    let seed = markit_mdbench_campaign2::identity::campaign_seed("aaaa");
    assert_eq!(seed, markit_mdbench_campaign2::identity::campaign_seed("aaaa"));
    assert_ne!(seed, markit_mdbench_campaign2::identity::campaign_seed("bbbb"));
    assert_ne!(
        markit_mdbench_campaign2::identity::session_seed(seed, "lifecycle", 0),
        markit_mdbench_campaign2::identity::session_seed(seed, "lifecycle", 1)
    );
    assert_ne!(
        markit_mdbench_campaign2::identity::session_seed(seed, "lifecycle", 0),
        markit_mdbench_campaign2::identity::session_seed(seed, "construction", 0)
    );

    let a = markit_mdbench_campaign2::identity::observation_id("r", "s", "lifecycle", "c", "H1", "lifecycle_step", 0);
    let b = markit_mdbench_campaign2::identity::observation_id("r", "s", "lifecycle", "c", "H1", "lifecycle_step", 1);
    assert_ne!(a, b);
}

#[test]
fn axis_point_tables_are_aligned_and_frozen() {
    for axis in [Axis::N, Axis::B, Axis::D, Axis::F, Axis::K] {
        assert_eq!(axis.points().len(), axis.point_labels().len());
    }
    assert_eq!(Axis::N.points().len(), 11);
    assert_eq!(Axis::B.points().len(), 9);
    assert_eq!(Axis::D.points().len(), 7);
    assert_eq!(Axis::F.points().len(), 8);
    assert_eq!(Axis::K.points().len(), 8);
    assert_eq!(markit_mdbench_campaign2::controlled::total_cells(), 43);
}
