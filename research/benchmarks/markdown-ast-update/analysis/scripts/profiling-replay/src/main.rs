//! MARKIT-31 Stage-B profiling replay (task §36).
//!
//! Replays ONE frozen case x one horse with identical mechanism dispatch
//! and timing boundaries as the frozen primary campaign (the SAME match
//! pattern and the SAME public runner primitives the campaign calls).
//! Prints per-iteration timings as JSON lines on stdout. Diagnostic
//! only: NON_PRIMARY, no raw campaign output, no ObservationId, no
//! changes to any horse.

use std::path::PathBuf;

use markit_mdbench_block_local::BlockLocalMechanism;
use markit_mdbench_campaign::workload::{load_campaign_workload, CampaignWorkload};
use markit_mdbench_common::Source;
use markit_mdbench_fragment_reuse::FragmentReuseMechanism;
use markit_mdbench_full_rebuild::{parse_document, FullRebuildMechanism};
use markit_mdbench_instrumentation::{InstantClock};
use markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism;
use markit_mdbench_oracle::{validate_normalized, ReferenceOracle};
use markit_mdbench_restart_convergence::RestartConvergenceMechanism;
use markit_mdbench_runner::orchestrate::{
    build_initial_state, run_full_parse_timed, run_update_timed,
};
use serde_json::json;


fn timing_ns(report: &markit_mdbench_runner::orchestrate::RunReport)
    -> (Option<u64>, Option<u64>, Option<u64>)
{
    match &report.measurement {
        markit_mdbench_instrumentation::LaneMeasurement::Timing(t) => (
            t.prepare_ns.known_value(),
            t.native_ns.known_value(),
            t.total_ns.known_value(),
        ),
        _ => panic!("profiling replay expects the timing lane"),
    }
}

fn main() {
    let mut root = None;
    let mut case_arg = None;
    let mut horse = String::new();
    let mut warmup = 5usize;
    let mut iters = 20usize;
    let mut build_only = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--benchmark-root" => root = Some(PathBuf::from(args.next().expect("path"))),
            "--case" => case_arg = Some(args.next().expect("case")),
            "--horse" => horse = args.next().expect("horse"),
            "--warmup" => warmup = args.next().expect("n").parse().expect("n"),
            "--iters" => iters = args.next().expect("n").parse().expect("n"),
            "--build-only" => build_only = true,
            other => panic!("unknown arg {other}"),
        }
    }
    let root = root.expect("--benchmark-root");
    let case_id = case_arg.expect("--case");

    let workload: CampaignWorkload =
        load_campaign_workload(&root).expect("frozen workload fails closed");
    let clock = InstantClock::default();

    if let Some(edit_case) =
        workload.edit_write.iter().find(|c| c.case_id_hex == case_id)
    {
        let (pre, post) =
            markit_mdbench_campaign::workload::edit_case_sources(edit_case);
        // Correctness authority identical to the campaign: H0 clean parse
        // of the POST source, once, outside every timer.
        let reference = parse_document(post.as_bytes());
        validate_normalized(&reference, None).expect("reference gate");
        macro_rules! dispatch {
            ($mech:expr) => {{
                let mechanism = $mech;
                let loop_iters = if build_only { iters } else { warmup + iters };
                for iteration in 0..loop_iters {
                    // FRESH-STATE RULE (task §8): new pre-state per
                    // iteration, built outside every timer.
                    let old_state =
                        build_initial_state(&mechanism, &pre).expect("pre-state");
                    if build_only { continue; }
                    let report = run_update_timed(
                        &mechanism,
                        &pre,
                        &post,
                        &edit_case.edit,
                        old_state,
                        &clock,
                        &ReferenceOracle::new(reference.clone()),
                    );
                    let timing = timing_ns(&report);
                    if iteration >= warmup {
                        println!(
                            "{}",
                            json!({
                                "kind": "edit_write",
                                "case": case_id,
                                "horse": horse,
                                "iteration": iteration - warmup,
                                "prepare_ns": timing.0,
                                "native_ns": timing.1,
                                "total_ns": timing.2,
                            })
                        );
                    }
                }
            }};
        }
        match horse.as_str() {
            "H0" => dispatch!(FullRebuildMechanism::new()),
            "H1" => dispatch!(BlockLocalMechanism::new()),
            "H2" => dispatch!(FragmentReuseMechanism::new()),
            "H3" => dispatch!(OldTreeSubtreeReuseMechanism::new()),
            "H4" => dispatch!(RestartConvergenceMechanism::new()),
            other => panic!("unknown horse {other}"),
        }
        return;
    }

    let clean_case = workload
        .clean_state
        .iter()
        .find(|c| c.case_id_hex == case_id)
        .unwrap_or_else(|| panic!("case {case_id} not in frozen workload"));
    let source =
        Source::new(markit_mdbench_common::SourceId(0), clean_case.source_text.clone());
    let reference = parse_document(source.as_bytes());
    validate_normalized(&reference, None).expect("reference gate");
    macro_rules! dispatch_clean {
        ($mech:expr) => {{
            let mechanism = $mech;
            for iteration in 0..(warmup + iters) {
                let report = run_full_parse_timed(
                    &mechanism,
                    &source,
                    &clock,
                    &ReferenceOracle::new(reference.clone()),
                );
                let timing = timing_ns(&report);
                if iteration >= warmup {
                    println!(
                        "{}",
                        json!({
                            "kind": "clean_state",
                            "case": case_id,
                            "horse": horse,
                            "iteration": iteration - warmup,
                            "prepare_ns": timing.0,
                            "native_ns": timing.1,
                            "total_ns": timing.2,
                        })
                    );
                }
            }
        }};
    }
    match horse.as_str() {
        "H0" => dispatch_clean!(FullRebuildMechanism::new()),
        "H1" => dispatch_clean!(BlockLocalMechanism::new()),
        "H2" => dispatch_clean!(FragmentReuseMechanism::new()),
        "H3" => dispatch_clean!(OldTreeSubtreeReuseMechanism::new()),
        "H4" => dispatch_clean!(RestartConvergenceMechanism::new()),
        other => panic!("unknown horse {other}"),
    }
}
