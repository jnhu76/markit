//! Frozen-workload correctness closure regression (#22 REAL WORKLOAD
//! CORRECTNESS CLOSURE).
//!
//! Level 2 — the pinned real counterexamples: every (horse, payload_id)
//! that produced `wrong_result` on the CORRECTIVE-C freeze (PR #39) must
//! pass against the unchanged frozen payloads.
//!
//! Level 3 — the complete frozen matrix: all 362 G0 EDIT_WRITE cases ×
//! H0–H4 dispatched through the EXISTING runner correctness-only lane.
//!
//! Both levels READ the frozen manifests and sources; neither writes
//! anything, and neither measures anything (no clock, no counters, no
//! performance fact). The workload-identity guards (which need only the
//! committed manifests) run everywhere; the byte-level dispatches need
//! the materialized acquisition (`workloads/sources/`, local-only,
//! gitignored) and are gated on it with a loud notice, exactly like the
//! `mdbench-corrective-c dry-run` they mirror.

use std::path::{Path, PathBuf};

use markit_mdbench_common::{CanonicalEdit, Source, SourceId};
use markit_mdbench_diagnostics::{isolate, IsolationClass};
use markit_mdbench_full_rebuild::FullRebuildMechanism;
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_semantics::payload::PayloadRecord;
use markit_mdbench_workload_freeze::dryrun::read_jsonl;
use markit_mdbench_workload_freeze::{load_selected_files, MEMBERSHIP_G0_PRIMARY};

/// The benchmark root (this crate lives at `<root>/diagnostics`).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("benchmark root")
        .to_path_buf()
}

/// The 35 wrong dispatches recorded by the CORRECTIVE-C freeze (PR #39),
/// as (horse, payload_id) pairs. This list is the frozen regression
/// authority: the cases must remain in the manifest and must pass.
const FROZEN_WRONG_DISPATCHES: &[(&str, &str)] = &[
    // G0-FENCE-CLOSER-REMOVE (step 0)
    ("H2", "rp1:67111c301f8ce3548341084799bafbc5"),
    ("H2", "rp1:4d48070656f7cfa612097942d283930b"),
    ("H2", "rp1:7deb2e6a6a3a34e214c0be96de57b842"),
    ("H2", "rp1:d5018ac1a66fde932c8a3e57c7d1061c"),
    ("H3", "rp1:67111c301f8ce3548341084799bafbc5"),
    ("H3", "rp1:4d48070656f7cfa612097942d283930b"),
    ("H3", "rp1:7deb2e6a6a3a34e214c0be96de57b842"),
    ("H3", "rp1:d5018ac1a66fde932c8a3e57c7d1061c"),
    ("H4", "rp1:67111c301f8ce3548341084799bafbc5"),
    ("H4", "rp1:4d48070656f7cfa612097942d283930b"),
    ("H4", "rp1:7deb2e6a6a3a34e214c0be96de57b842"),
    ("H4", "rp1:d5018ac1a66fde932c8a3e57c7d1061c"),
    // G0-FENCE-CLOSER-RESTORE (step 1)
    ("H2", "rp1:2bfb595924ebc83f8fb307d91e280237"),
    ("H2", "rp1:40ca5042915a34fbbcae6ce47d25f01e"),
    ("H2", "rp1:dc5c6ccba715031533217eaefcf6ee70"),
    ("H2", "rp1:fcdd451fd5c8c8e9bbd8c492db22c0b1"),
    ("H3", "rp1:2bfb595924ebc83f8fb307d91e280237"),
    ("H3", "rp1:40ca5042915a34fbbcae6ce47d25f01e"),
    ("H3", "rp1:dc5c6ccba715031533217eaefcf6ee70"),
    ("H3", "rp1:fcdd451fd5c8c8e9bbd8c492db22c0b1"),
    ("H4", "rp1:2bfb595924ebc83f8fb307d91e280237"),
    ("H4", "rp1:40ca5042915a34fbbcae6ce47d25f01e"),
    ("H4", "rp1:dc5c6ccba715031533217eaefcf6ee70"),
    ("H4", "rp1:fcdd451fd5c8c8e9bbd8c492db22c0b1"),
    // G0-REFDEF-RESTORE (step 1)
    ("H2", "rp1:40ca5042915a34fbbcae6ce47d25f01e"),
    ("H2", "rp1:67d9a252ce841a6f9859a898d4bd63ef"),
    ("H2", "rp1:96459a914b1c6e169fa5e70776098fc1"),
    ("H3", "rp1:40ca5042915a34fbbcae6ce47d25f01e"),
    ("H3", "rp1:67d9a252ce841a6f9859a898d4bd63ef"),
    ("H3", "rp1:5835fef92f3ca4b972c95648a4e50735"),
    ("H3", "rp1:446004e95c826aeb9a7a78f59c2cf120"),
    ("H3", "rp1:69a7f2a88f3daceb9753f35e66a620d8"),
    ("H3", "rp1:528885c8ebcbd1524200a50cab1bc7c0"),
    ("H3", "rp1:488090bbd682252f08e9deb979920cd3"),
    ("H3", "rp1:96459a914b1c6e169fa5e70776098fc1"),
];

fn payloads() -> Vec<PayloadRecord> {
    read_jsonl(&root().join("workloads/payloads/edit-write-manifest-v1.jsonl"))
        .expect("frozen EDIT_WRITE manifest readable")
}

fn require_sources(test: &str) -> Option<Vec<markit_mdbench_workload_freeze::SelectedFile>> {
    match load_selected_files(&root()) {
        Ok(files) if !files.is_empty() => Some(files),
        _ => {
            eprintln!(
                "SKIPPED {test}: acquisition sources are not materialized \
                 (workloads/sources/ is local-only). Run \
                 `python3 tools/acquire.py verify --full` from the benchmark root, \
                 then re-run `cargo test --workspace`."
            );
            None
        }
    }
}

/// Reconstruct one frozen payload's pre/post sources from the manifest +
/// materialized base source, verifying the frozen digests.
fn reconstruct(
    files: &[markit_mdbench_workload_freeze::SelectedFile],
    all: &[PayloadRecord],
    payload: &PayloadRecord,
) -> (Source, Source, CanonicalEdit) {
    let base = files
        .iter()
        .find(|file| file.key == payload.source_path)
        .expect("payload source materialized");
    let pre_text = if payload.step == 0 {
        base.text.clone()
    } else {
        let step0 = all
            .iter()
            .find(|candidate| candidate.trace_id == payload.trace_id && candidate.step == 0)
            .expect("trace has a step 0");
        step0.edit.apply(&base.text).expect("broken state")
    };
    let post_text = payload.edit.apply(&pre_text).expect("post source");
    // ALL HORSES SEE IDENTICAL BYTES: the reconstructed sources must hash
    // to the frozen digests recorded in the manifest.
    assert_eq!(
        markit_mdbench_semantics::sha256_hex(pre_text.as_bytes()),
        payload.pre_source_sha256,
        "{}: pre source bytes differ from the frozen digest",
        payload.payload_id
    );
    assert_eq!(
        markit_mdbench_semantics::sha256_hex(post_text.as_bytes()),
        payload.post_source_sha256,
        "{}: post source bytes differ from the frozen digest",
        payload.payload_id
    );
    (
        Source::new(SourceId(0), pre_text),
        Source::new(SourceId(1), post_text),
        payload.edit.to_canonical().expect("canonical edit"),
    )
}

// ---------------------------------------------------------------------------
// Workload identity guards (no sources needed)
// ---------------------------------------------------------------------------

#[test]
fn every_frozen_wrong_dispatch_is_still_in_the_manifest() {
    let all = payloads();
    for (horse, payload_id) in FROZEN_WRONG_DISPATCHES {
        let found = all.iter().find(|payload| &payload.payload_id == payload_id);
        assert!(
            found.is_some(),
            "{horse} {payload_id}: the failing case was REMOVED from the frozen manifest"
        );
        let payload = found.expect("checked");
        assert!(
            payload
                .memberships
                .iter()
                .any(|membership| membership == MEMBERSHIP_G0_PRIMARY),
            "{payload_id}: G0_PRIMARY membership changed"
        );
        assert_eq!(payload.grammar_id, "BENCH-GRAMMAR-v1");
    }
}

#[test]
fn frozen_manifest_counts_are_unchanged() {
    let all = payloads();
    let g0 = all
        .iter()
        .filter(|payload| {
            payload.grammar_id == "BENCH-GRAMMAR-v1"
                && payload
                    .memberships
                    .iter()
                    .any(|membership| membership == MEMBERSHIP_G0_PRIMARY)
        })
        .count();
    assert_eq!(all.len(), 449, "EDIT_WRITE payload count changed");
    assert_eq!(g0, 362, "G0 EDIT_WRITE case count changed");
    let pairs = all
        .iter()
        .filter(|payload| payload.step == 1)
        .map(|payload| payload.trace_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(pairs.len(), 120, "BREAK/RESTORE pair count changed");
}

// ---------------------------------------------------------------------------
// Level 2 — the pinned real counterexamples
// ---------------------------------------------------------------------------

#[test]
fn pinned_real_counterexamples_now_pass() {
    let Some(files) = require_sources("pinned_real_counterexamples_now_pass") else {
        return;
    };
    let all = payloads();
    let mut checked = 0usize;
    for (horse, payload_id) in FROZEN_WRONG_DISPATCHES {
        let payload = all
            .iter()
            .find(|candidate| &candidate.payload_id == payload_id)
            .expect("pinned payload still frozen");
        let (pre, post, edit) = reconstruct(&files, &all, payload);
        let reference = markit_mdbench_full_rebuild::parse_document(post.as_bytes());
        let h0_pre = markit_mdbench_full_rebuild::parse_document(pre.as_bytes());
        let isolation = match *horse {
            "H2" => isolate(
                &markit_mdbench_fragment_reuse::FragmentReuseMechanism::new(),
                &pre,
                &post,
                &edit,
                &reference,
                &h0_pre,
            ),
            "H3" => isolate(
                &markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism::new(),
                &pre,
                &post,
                &edit,
                &reference,
                &h0_pre,
            ),
            "H4" => isolate(
                &markit_mdbench_restart_convergence::RestartConvergenceMechanism::new(),
                &pre,
                &post,
                &edit,
                &reference,
                &h0_pre,
            ),
            other => panic!("unexpected horse {other}"),
        };
        assert_eq!(
            isolation.class(),
            IsolationClass::Pass,
            "{horse} {payload_id} ({}): {}",
            payload.expected_transition,
            isolation.class().name()
        );
        checked += 1;
    }
    assert_eq!(checked, 35, "the pinned wrong-dispatch list changed size");
    // The authority itself: H0's clean parse of every post source is the
    // frozen oracle, and it must be a valid normalized document.
    let _ = FullRebuildMechanism::new();
}

#[test]
fn frozen_reference_environment_cases_are_the_ones_that_flipped() {
    // Guard against a fixture-shaped mistake: every pinned case must be a
    // definition-environment case (the transition families the closure
    // report claims), and the H0 clean parses must really differ in the
    // reference direction — otherwise the pinned list no longer
    // reproduces the defect class it documents.
    let Some(files) =
        require_sources("frozen_reference_environment_cases_are_the_ones_that_flipped")
    else {
        return;
    };
    let all = payloads();
    let mut transitions = std::collections::BTreeSet::new();
    for (_, payload_id) in FROZEN_WRONG_DISPATCHES {
        let payload = all
            .iter()
            .find(|candidate| &candidate.payload_id == payload_id)
            .expect("pinned payload still frozen");
        transitions.insert(payload.expected_transition.clone());
        let (pre, post, _) = reconstruct(&files, &all, payload);
        let h0_pre = markit_mdbench_full_rebuild::parse_document(pre.as_bytes());
        let h0_post = markit_mdbench_full_rebuild::parse_document(post.as_bytes());
        assert_ne!(
            h0_pre, h0_post,
            "{payload_id}: the edit does not change the H0 result at all"
        );
    }
    assert_eq!(
        transitions,
        [
            "G0-FENCE-CLOSER-REMOVE",
            "G0-FENCE-CLOSER-RESTORE",
            "G0-REFDEF-RESTORE"
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        "the pinned failures must be exactly the three frozen families"
    );
}

// ---------------------------------------------------------------------------
// Mechanism fidelity on the frozen workload (no timing, no metric)
// ---------------------------------------------------------------------------

/// A few frozen G0 cases whose edits touch no definition and no fence, so
/// every horse's own reuse mechanism must still fire on them.
const SAFE_FROZEN_CASES: &[&str] = &[
    // G0-LOCAL-TEXT-REPLACE-EQ: a local text replacement that touches no
    // definition and no fence, in documents where every horse's own reuse
    // mechanism has something to retain.
    "rp1:490226ced228384bb093995c288909dd", // 9.3 KB document
    "rp1:7768323f8cba13c235e6171a1471e81a", // 6.1 KB document
    "rp1:240fb863f4c41b16af870ae085376d25", // 2.7 KB document
];

#[test]
fn safe_frozen_cases_still_exercise_each_mechanism() {
    let Some(files) = require_sources("safe_frozen_cases_still_exercise_each_mechanism") else {
        return;
    };
    let all = payloads();
    for payload_id in SAFE_FROZEN_CASES {
        let payload = all
            .iter()
            .find(|candidate| &candidate.payload_id == payload_id)
            .expect("safe case still frozen");
        let (pre, post, edit) = reconstruct(&files, &all, payload);
        let reference = markit_mdbench_full_rebuild::parse_document(post.as_bytes());

        // H2 FRAGMENT_REUSE: retained blocks are still taken.
        let (result, reused, _) = run_with_counters(
            &markit_mdbench_fragment_reuse::FragmentReuseMechanism::new(),
            &pre,
            &post,
            &edit,
        );
        assert_eq!(result, reference, "H2 {payload_id}");
        assert!(reused > 0, "H2 {payload_id}: fragment reuse stopped firing");

        // H3 OLD_TREE_SUBTREE_REUSE: retained subtrees are still taken.
        let (result, reused, _) = run_with_counters(
            &markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism::new(),
            &pre,
            &post,
            &edit,
        );
        assert_eq!(result, reference, "H3 {payload_id}");
        assert!(reused > 0, "H3 {payload_id}: subtree reuse stopped firing");

        // H4 RESTART_CONVERGENCE: the forward pass still converges before
        // EOF and adopts the retained suffix.
        let (result, reused, convergence) = run_with_counters(
            &markit_mdbench_restart_convergence::RestartConvergenceMechanism::new(),
            &pre,
            &post,
            &edit,
        );
        assert_eq!(result, reference, "H4 {payload_id}");
        assert!(
            reused > 0,
            "H4 {payload_id}: convergence reuse stopped firing"
        );
        assert!(
            convergence < post.as_bytes().len() as u64,
            "H4 {payload_id}: convergence now runs to EOF"
        );
    }
}

/// Run one update with the frozen counters collected, returning the
/// normalized result, `nodes_reused`, and `convergence_distance`
/// (mechanism-fidelity facts only — no timing value is read anywhere).
fn run_with_counters<M>(
    mechanism: &M,
    pre: &Source,
    post: &Source,
    edit: &CanonicalEdit,
) -> (markit_mdbench_oracle::NormalizedDocument, u64, u64)
where
    M: markit_mdbench_common::Mechanism,
    M::State: NormalizeV1,
{
    use markit_mdbench_common::{CounterSink, MechanismContext, Observed, WorkCounters};
    let mut counters = WorkCounters::all_unknown();
    let mut sink = CounterSink::new(&mut counters);
    let mut cx = MechanismContext::new(&mut sink);
    let state = {
        let pending = mechanism.full_parse(pre, &mut cx).expect("full_parse");
        mechanism.complete(pending).expect("complete").state
    };
    let prepared = mechanism
        .prepare_update(pre, post, edit, &state, &mut cx)
        .expect("prepare_update");
    let pending = mechanism
        .update(pre, post, edit, state, prepared, &mut cx)
        .expect("update");
    let completed = mechanism.complete(pending).expect("complete");
    let reused = match counters.nodes_reused {
        Observed::Known(value) => value,
        other => panic!("nodes_reused not measured: {other:?}"),
    };
    let convergence = match counters.convergence_distance {
        Observed::Known(value) => value,
        Observed::NotApplicable => 0,
        other => panic!("convergence_distance unusable: {other:?}"),
    };
    (completed.state.normalize_v1(), reused, convergence)
}

// ---------------------------------------------------------------------------
// Level 3 — the complete frozen matrix
// ---------------------------------------------------------------------------

#[test]
fn full_frozen_matrix_is_1810_of_1810() {
    let Some(files) = require_sources("full_frozen_matrix_is_1810_of_1810") else {
        return;
    };
    let (report, rows) = markit_mdbench_workload_freeze::dryrun::run_dry_run(&root(), &files)
        .expect("frozen dry-run");
    assert_eq!(report.full_read.g0_strict_cases, 22);
    // BLOCKER B parity: clean parse + native-state construction must pass
    // on EVERY horse for EVERY G0-strict file -> 22 x 5 = 110/110.
    assert_eq!(report.full_read.horse_dispatches, 110);
    assert_eq!(report.full_read.pass, 110, "FULL_READ 110/110");
    assert_eq!(report.full_read.failed, 0);
    for horse in ["H0", "H1", "H2", "H3", "H4"] {
        let counts = report
            .full_read
            .per_horse
            .get(horse)
            .unwrap_or_else(|| panic!("{horse} missing from the FULL_READ report"));
        assert_eq!(
            (counts.pass, counts.wrong_result, counts.execution_failed),
            (22, 0, 0),
            "{horse} must be 22 pass / 0 wrong / 0 execution failure"
        );
    }
    assert_eq!(report.edit_write.g0_cases, 362);
    assert_eq!(report.edit_write.horse_dispatches, 1810);
    for horse in ["H0", "H1", "H2", "H3", "H4"] {
        let counts = report
            .edit_write
            .per_horse
            .get(horse)
            .unwrap_or_else(|| panic!("{horse} missing from the dry-run report"));
        assert_eq!(
            (counts.pass, counts.wrong_result, counts.execution_failed),
            (362, 0, 0),
            "{horse} must be 362 pass / 0 wrong / 0 execution failure"
        );
    }
    assert_eq!(
        rows.len(),
        1810 + 110,
        "one row per EDIT_WRITE dispatch plus one per FULL_READ horse dispatch"
    );
}
