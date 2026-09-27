//! I5 conformance battery (task contract §30–§31/§35; #59 §6/§9–§11/§20;
//! #60 §9.3/§9.3.1). T8–T18 pin the frontier and accounting contracts that
//! I3/I4 could not yet state:
//!
//! - T8  — the frontier is real: `commit` is infallible by type, staging
//!   failures leave the old state owned and reusable;
//! - T9  — the post-frontier region attempts zero allocations (denial
//!   probe under a test-only global allocator with a thread-local
//!   guard; the allocator still services requests for memory
//!   safety but records every attempt, and the region must record
//!   none);
//! - T10 — post-frontier accounting is scalar and event-bounded: a
//!   commit-only recording sink proves the frontier region charges
//!   no parse/fact/candidate work;
//! - T11 — local retirement touches exactly the detached middle;
//! - T12 — full-route retirement covers the complete old state
//!   (pinned in `i5_tests::retirement`);
//! - T13 — the decision-bearing ledger is exact on the #60 witness shape;
//! - T14 — `Unknown` (never measured) is distinct from `Known(0)`
//!   (measured zero) in both directions;
//! - T15 — no post-hoc attribution walk (source audit of the retirement
//!   and commit modules);
//! - T16 — the frontier signature structurally cannot re-observe the
//!   source (no work sink past the frontier);
//! - T17 — every forbidden sentinel measures `Known(0)` on the valid
//!   local path;
//! - T18 — the route decision has exactly two branches and no fallback.
//!
//! Every expected value is hand-derived from the frozen authorities and
//! the literal byte layout of the fixture — never read back from the
//! implementation.
//!
//! T9 allocates nothing on the guarded path; note the probe allocator is
//! the global allocator of this test binary and simply delegates to
//! `System` outside the guard.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};

use crate::full_build::full_build;
use crate::structural::{
    HorseAStructuralCountersV1, NoopHorseAStructuralSink, Observed, RecordingHorseAStructuralSink,
};
use crate::update::{stage, update_with_structural};

fn edit(start: usize, end: usize, inserted: &str) -> CanonicalEdit {
    CanonicalEdit::new(start, end, inserted).expect("edit geometry")
}

/// The #60 witness-shape fixture: three paragraph Owners, the edit at the
/// physical start of the third, local route, convergence at real EOF.
const WITNESS_SOURCE: &str = "alpha\n\nbeta\n\ngamma\n";

/// Build the old READY state for [`WITNESS_SOURCE`].
fn witness_old() -> crate::state::ReadyDocument {
    let old_source = Source::new(SourceId(1), WITNESS_SOURCE);
    full_build(
        &old_source,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("initial full build")
}

/// Run the witness edit through the full recording pipeline.
fn witness_update() -> (
    crate::state::ReadyDocument,
    HorseAStructuralCountersV1,
    Source,
) {
    let old_source = Source::new(SourceId(1), WITNESS_SOURCE);
    let old = witness_old();
    let e = edit(13, 13, "X");
    let post = e.apply(&old_source, SourceId(2)).expect("edit applies");
    let mut recording = RecordingHorseAStructuralSink::new();
    let next = update_with_structural(
        old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut recording,
    )
    .expect("the update completes on the local path");
    (next, recording.into_counters(), post)
}

/// A field that was never charged or was measured at zero — i.e. evidence
/// of absence, never a positive quantity.
fn untouched(o: &Observed) -> bool {
    matches!(o, Observed::Unknown | Observed::Known(0))
}

// ---------------------------------------------------------------------------
// T9 — the allocation-denial probe. The guarded region (frontier crossing)
// must attempt zero allocations: post-frontier work is ownership moves,
// relinks, aggregate recomputation and fixed-size scalar writes only
// (#59 §9.2, §11: drops only, no allocation, no panic).
// ---------------------------------------------------------------------------

thread_local! {
    static DENY_ALLOC: Cell<bool> = const { Cell::new(false) };
    static ALLOC_ATTEMPTS: Cell<u64> = const { Cell::new(0) };
}

struct FrontierAllocProbe;

unsafe impl GlobalAlloc for FrontierAllocProbe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if DENY_ALLOC.with(Cell::get) {
            ALLOC_ATTEMPTS.with(|c| c.set(c.get() + 1));
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static FRONTIER_ALLOC_PROBE: FrontierAllocProbe = FrontierAllocProbe;

/// Guard RAII: the flag is always restored, even on panic.
struct DenyGuard;

impl DenyGuard {
    fn deny() -> Self {
        DENY_ALLOC.with(|d| d.set(true));
        DenyGuard
    }
}

impl Drop for DenyGuard {
    fn drop(&mut self) {
        DENY_ALLOC.with(|d| d.set(false));
    }
}

/// T8: the frontier is real. `commit` is infallible by type (the binding
/// below is a `ReadyDocument`, not a `Result`), and a staging failure
/// leaves the old state owned, coherent and reusable.
#[test]
fn t08_frontier_is_infallible_and_stage_failure_leaves_old_owned() {
    let old_source = Source::new(SourceId(1), WITNESS_SOURCE);
    let old = witness_old();

    // A post source that is not `edit.apply(old_source)` must refuse at
    // staging — before any ownership transfer.
    let e = edit(13, 13, "X");
    let bogus_post = Source::new(SourceId(2), WITNESS_SOURCE); // wrong length
    let staged = stage(
        &old,
        &old_source,
        &bogus_post,
        &e,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    );
    assert!(
        matches!(
            staged,
            Err(crate::update::UpdateError::InvalidAssociation { .. })
        ),
        "a non-canonical post source must be refused at staging"
    );

    // The old state is still owned by the caller and still works: the
    // same pipeline succeeds against the true post source.
    let post = e.apply(&old_source, SourceId(2)).expect("edit applies");
    let prepared = stage(
        &old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("the corrected staging succeeds")
    .prepare(old, &mut NoopHorseAStructuralSink)
        .expect("the bounded commit workspace prepares");
    let next: crate::state::ReadyDocument = prepared.commit(&mut NoopHorseAStructuralSink);
    assert_eq!(next.source_id, SourceId(2));
    assert_eq!(next.owners.records(), 3);
}

/// T9: the frontier-crossing region attempts zero allocations. Staging and
/// formation (which materialize fresh Owners) run outside the guard; only
/// `commit` runs inside it.
#[test]
fn t09_post_frontier_region_attempts_zero_allocations() {
    let old_source = Source::new(SourceId(1), WITNESS_SOURCE);
    let old = witness_old();
    let e = edit(13, 13, "X");
    let post = e.apply(&old_source, SourceId(2)).expect("edit applies");

    let prepared = stage(
        &old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("staging succeeds")
    .prepare(old, &mut NoopHorseAStructuralSink)
        .expect("the bounded commit workspace prepares");

    ALLOC_ATTEMPTS.with(|c| c.set(0));
    let mut recording = RecordingHorseAStructuralSink::new();
    {
        let _guard = DenyGuard::deny();
        let next: crate::state::ReadyDocument = prepared.commit(&mut recording);
        assert_eq!(next.owners.records(), 3);
        assert_eq!(next.source_len, post.len_bytes());
    }

    let attempts = ALLOC_ATTEMPTS.with(Cell::get);
    assert_eq!(
        attempts, 0,
        "the post-frontier region attempted {attempts} allocation(s); \
         post-frontier work must be ownership moves, relinks, aggregate \
         recomputation and fixed-size scalar writes only"
    );
}

/// T10 + T11: a commit-only recording sink shows exactly what the frontier
/// region charged. Nothing pre-frontier (locate, safe predecessor, cursor,
/// candidate, facts, fresh materialization, certificate writes) appears;
/// the splice and retirement work does; the sentinel sites on this path
/// measure `Known(0)`.
#[test]
fn t10_t11_commit_only_ledger_is_event_bounded_and_retirement_exact() {
    let old_source = Source::new(SourceId(1), WITNESS_SOURCE);
    let old = witness_old();
    let e = edit(13, 13, "X");
    let post = e.apply(&old_source, SourceId(2)).expect("edit applies");

    // Everything up to the frontier records into a sink that is then
    // dropped; `commit` records alone into the audited sink.
    let prepared = stage(
        &old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("staging succeeds")
    .prepare(old, &mut NoopHorseAStructuralSink)
        .expect("the bounded commit workspace prepares");

    let mut commit_only = RecordingHorseAStructuralSink::new();
    let next = prepared.commit(&mut commit_only);
    let c = commit_only.into_counters();

    // — Pre-frontier work must not reappear past the frontier. —
    assert!(untouched(&c.restart_old));
    assert!(untouched(&c.convergence_old));
    assert!(untouched(&c.convergence_new));
    assert!(untouched(&c.replace_lo));
    assert!(untouched(&c.replace_hi));
    assert!(untouched(&c.full_build_selected));
    assert!(untouched(&c.full_build_reason));
    assert!(untouched(&c.m_old));
    assert!(untouched(&c.h_old));
    assert!(untouched(&c.m_new));
    assert!(untouched(&c.locate_node_visits));
    assert!(untouched(&c.safe_predecessor_node_visits));
    assert!(untouched(&c.cursor_node_visits));
    assert!(untouched(&c.fact_range_node_visits));
    assert!(untouched(&c.bulk_build_node_visits));
    assert!(untouched(&c.certificate_reads));
    assert!(untouched(&c.certificate_writes));
    assert!(untouched(&c.candidate_checks));
    assert!(untouched(&c.cursor_advances));
    assert!(untouched(&c.owners_created));
    assert!(untouched(&c.fresh_payload_nodes_final));
    assert!(untouched(&c.fresh_payload_nodes_temporary));
    assert!(untouched(&c.old_fact_owner_visits));
    assert!(untouched(&c.old_facts_extracted));
    assert!(untouched(&c.new_facts_extracted));
    assert!(untouched(&c.facts_compared));
    assert!(untouched(&c.fact_compare_bytes));
    assert!(untouched(&c.reftable_entries_visited));
    // Sentinel sites that live in staging were not executed here.
    assert!(untouched(&c.forbidden_global_fact_recollection));
    assert!(untouched(&c.forbidden_unaffected_payload_inspections));
    assert!(untouched(&c.forbidden_unaffected_certificate_writes));

    // — The structural splice happened here and only here. —
    assert!(matches!(c.split_node_visits, Observed::Known(v) if v > 0));
    assert!(matches!(c.pivot_extract_node_visits, Observed::Known(v) if v > 0));
    assert!(matches!(c.join_node_visits, Observed::Known(v) if v > 0));
    assert!(matches!(c.sequence_link_writes, Observed::Known(v) if v > 0));
    assert!(matches!(c.aggregate_reads, Observed::Known(v) if v > 0));
    assert!(matches!(c.aggregate_writes, Observed::Known(v) if v > 0));
    assert!(matches!(c.owners_removed, Observed::Known(v) if v > 0));

    // T11: retirement charged exactly the detached middle (Δ_old = 2
    // records, P_removed = 4 payload nodes) — never the retained prefix
    // (alpha) or the fresh suffix owners.
    assert_eq!(c.retire_node_visits, Observed::known(2));
    assert_eq!(c.payload_nodes_retired, Observed::known(4));
    assert_eq!(c.retirement_frames_entered, Observed::known(6));
    assert_eq!(c.max_retirement_depth, Observed::known(2));

    // The commit-side sentinel sites executed and measured zero.
    assert_eq!(
        c.forbidden_prefix_sequential_enumeration,
        Observed::known(0)
    );
    assert_eq!(
        c.forbidden_suffix_sequential_enumeration,
        Observed::known(0)
    );
    assert_eq!(c.forbidden_unaffected_coordinate_writes, Observed::known(0));
    assert_eq!(c.forbidden_unaffected_old_retirement, Observed::known(0));
    assert_eq!(c.forbidden_attribution_tree_walk, Observed::known(0));

    // H_new is the one post-frontier diagnostic.
    assert_eq!(c.h_new, Observed::known(2));
    assert_eq!(next.owners.records(), 3);
}

/// T13: the decision-bearing ledger is exact on the witness shape. The
/// numbers below are hand-derived from #60 §9.3/§9.3.1/§11.1 and the byte
/// layout of the fixture, not observed:
///
/// - `certificate_reads` = 1: the safe-predecessor final-selection
///   inspection; the support-touch validation of the selected certificate
///   is fused with that one inspection, and no candidate predicate
///   evaluation executes on this path (the single certified candidate's
///   mapped image lies past the only fresh root-blank barrier, so the
///   parse converges at real EOF — see the assertion comment below);
/// - `certificate_writes` = 1: the interior root-blank barrier between the
///   two fresh replacement owners; the EOF convergence installs nothing;
/// - retirement: Δ_old = 2 records, P_removed = 4 payload nodes, frames =
///   2 + 4 = 6, gauge = max(H_detached = 2, D_payload = 2) = 2;
/// - fresh materialization: Δ_new = 2 owners, 4 payload nodes, both
///   surviving (no transient discards in this realization);
/// - facts: both sides extract zero definition entries and the comparison
///   executes and measures zero (applied-but-empty is `Known(0)`).
#[test]
fn t13_decision_bearing_ledger_is_exact_on_the_witness_shape() {
    let (next, c, _post) = witness_update();
    assert_eq!(c.schema, HorseAStructuralCountersV1::SCHEMA);

    // Geometry / route (#60 §9.3 shape, adapted to this fixture: the
    // replacement reaches real EOF, so convergence is L_old/L_new).
    assert_eq!(c.restart_old, Observed::known(7));
    assert_eq!(c.convergence_old, Observed::known(19));
    assert_eq!(c.convergence_new, Observed::known(20));
    assert_eq!(c.replace_lo, Observed::known(1));
    assert_eq!(c.replace_hi, Observed::known(3));
    assert_eq!(c.full_build_selected, Observed::known(0));
    assert_eq!(c.full_build_reason, Observed::known(0));

    // Scale/height diagnostics: 3 balanced records on both sides.
    assert_eq!(c.m_old, Observed::known(3));
    assert_eq!(c.h_old, Observed::known(2));
    assert_eq!(c.m_new, Observed::known(3));
    assert_eq!(c.h_new, Observed::known(2));

    // Candidate/certificate work. Hand-derived for this fixture: the edit
    // inserts one byte (delta = +1), so the single certified candidate
    // (beta's boundary at 13) maps to new-side image 14, which lies past
    // the only root-blank barrier of the fresh region (13). The parser
    // therefore never reaches the candidate's mapped cut before real EOF,
    // so NO predicate evaluation executes and `candidate_checks` stays
    // `Unknown` — never-measured evidence, not measured zero (T14). One
    // candidate is found by the seek (advance), and the one certificate
    // read is the safe-predecessor final-selection inspection (the
    // support-touch validation of the selected certificate is fused with
    // that one inspection; the EOF upper boundary carries no certificate).
    assert!(matches!(c.candidate_checks, Observed::Unknown));
    assert_eq!(c.certificate_reads, Observed::known(1));
    assert_eq!(c.certificate_writes, Observed::known(1));
    assert_eq!(c.cursor_advances, Observed::known(1));

    // Fresh materialization (§9.3: bulk_build visits = Δ_new).
    assert_eq!(c.bulk_build_node_visits, Observed::known(2));
    assert_eq!(c.owners_created, Observed::known(2));
    assert_eq!(c.owners_removed, Observed::known(2));
    assert_eq!(c.fresh_payload_nodes_final, Observed::known(4));
    assert_eq!(c.fresh_payload_nodes_temporary, Observed::known(4));

    // Facts: applied-but-empty comparison is measured zero, not Unknown.
    assert_eq!(c.old_fact_owner_visits, Observed::known(2));
    assert_eq!(c.old_facts_extracted, Observed::known(0));
    assert_eq!(c.new_facts_extracted, Observed::known(0));
    assert_eq!(c.facts_compared, Observed::known(0));
    assert_eq!(c.fact_compare_bytes, Observed::known(0));
    assert_eq!(c.reftable_entries_visited, Observed::known(0));

    // Retirement (§11.1 separated quantities).
    assert_eq!(c.retire_node_visits, Observed::known(2));
    assert_eq!(c.payload_nodes_retired, Observed::known(4));
    assert_eq!(c.retirement_frames_entered, Observed::known(6));
    assert_eq!(c.max_retirement_depth, Observed::known(2));

    // The measured-but-not-exactly-derived operator fields must all have
    // executed (the path exercised them); their exact per-operator values
    // are pinned against synthetic fixtures in `i5_tests::primitives`.
    for (name, field) in [
        ("locate", &c.locate_node_visits),
        ("safe_predecessor", &c.safe_predecessor_node_visits),
        ("cursor", &c.cursor_node_visits),
        ("fact_range", &c.fact_range_node_visits),
        ("split", &c.split_node_visits),
        ("pivot_extract", &c.pivot_extract_node_visits),
        ("join", &c.join_node_visits),
        ("link_writes", &c.sequence_link_writes),
        ("aggregate_reads", &c.aggregate_reads),
        ("aggregate_writes", &c.aggregate_writes),
    ] {
        assert!(
            matches!(field, Observed::Known(v) if *v > 0),
            "{name} executed on this path but was not measured positive: {field:?}"
        );
    }

    // The returned state is a normal READY state.
    assert_eq!(next.owners.records(), 3);
}

/// T14: `Unknown` (never measured) and `Known(0)` (measured zero) are
/// distinct kinds of evidence, in both directions.
#[test]
fn t14_unknown_and_known_zero_are_distinct_evidence() {
    // Direction 1: a fresh record has never measured anything — every
    // field is Unknown, including fields a valid run would measure as 0.
    let fresh = HorseAStructuralCountersV1::new();
    assert!(matches!(fresh.candidate_checks, Observed::Unknown));
    assert!(matches!(fresh.facts_compared, Observed::Unknown));
    assert!(matches!(
        fresh.forbidden_attribution_tree_walk,
        Observed::Unknown
    ));
    assert!(matches!(fresh.retire_node_visits, Observed::Unknown));

    // Direction 2: on the witness run the fact comparison site executed
    // and measured zero — `Known(0)`, never `Unknown` (#59 §10.2: the
    // applied-but-empty comparison is evidence of absence).
    let (_next, c, _post) = witness_update();
    assert_eq!(c.facts_compared, Observed::known(0));
    assert_eq!(c.fact_compare_bytes, Observed::known(0));
    assert_eq!(c.old_facts_extracted, Observed::known(0));

    // And the staging-side sentinel that did execute is `Known(0)` while
    // the commit-side sentinels that did not run in the T14-adjacent
    // commit-only view stay Unknown — the same "no occurrence" fact is
    // recorded differently depending on whether the defended site ran.
    // (Concretely: the full route never executes the local-route
    // retirement site, so its sentinel stays Unknown there — see
    // `i5_tests::retirement::the_full_route_retires_the_complete_old_document`
    // and the T17 assertion below for the local-route complement.)
}

/// T17: every forbidden sentinel measures `Known(0)` on the valid local
/// path — each defended site executed and measured zero occurrences.
#[test]
fn t17_all_forbidden_sentinels_measure_zero_on_the_valid_local_path() {
    let (_next, c, _post) = witness_update();
    assert_eq!(
        c.forbidden_prefix_sequential_enumeration,
        Observed::known(0)
    );
    assert_eq!(
        c.forbidden_suffix_sequential_enumeration,
        Observed::known(0)
    );
    assert_eq!(
        c.forbidden_unaffected_payload_inspections,
        Observed::known(0)
    );
    assert_eq!(c.forbidden_unaffected_coordinate_writes, Observed::known(0));
    assert_eq!(
        c.forbidden_unaffected_certificate_writes,
        Observed::known(0)
    );
    assert_eq!(c.forbidden_global_fact_recollection, Observed::known(0));
    assert_eq!(c.forbidden_unaffected_old_retirement, Observed::known(0));
    assert_eq!(c.forbidden_attribution_tree_walk, Observed::known(0));
}

/// T15: no post-hoc attribution walk. Source audit of the two post-frontier
/// modules: retirement charges only its own frame ledger (no node visits,
/// no link writes, no rotations, no aggregate work, no recomputes), and the
/// commit module contains no candidate/fact/cursor charging at all.
#[test]
fn t15_no_posthoc_attribution_walk_source_audit() {
    let retirement = include_str!("../retirement.rs");
    for banned in [
        "node_visit(",
        "link_writes(",
        "rotations(",
        "aggregate_reads(",
        "aggregate_writes(",
        "recompute(",
        "candidate_check(",
        "cursor_advance(",
        "certificate_read(",
        "certificate_write(",
    ] {
        assert!(
            !retirement.contains(banned),
            "retirement.rs must not charge `{banned}`: retirement performs \
             no relinks, no recomputes, and no candidate/certificate work"
        );
    }
    // The only sink methods retirement may call are the two frame
    // charges (plus the plain drops it performs without any sink).
    assert!(retirement.contains("retire_avl_frame("));
    assert!(retirement.contains("retire_payload_frame("));

    let prepared = include_str!("../prepared.rs");
    for banned in [
        "candidate_check(",
        "cursor_advance(",
        "old_fact_owner_visit(",
        "facts_extracted_old(",
        "facts_extracted_new(",
        "facts_compared(",
        "fact_compare_bytes(",
        "owners_created(",
        "fresh_payload_node_created(",
        "reftable_entries_visited(",
    ] {
        assert!(
            !prepared.contains(banned),
            "the commit module must not charge pre-frontier work `{banned}`"
        );
    }
}

/// T16: the frontier signature structurally cannot re-observe the source.
/// `PreparedCommit::commit` takes no work sink and no source — parser
/// observation is a staging phase, and nothing past the frontier can run
/// the parser or read the document text.
#[test]
fn t16_frontier_signature_cannot_reobserve_the_source() {
    let prepared = include_str!("../prepared.rs");
    const SIGNATURE: &str =
        "pub(crate) fn commit(self, sink: &mut dyn HorseAStructuralSink) -> ReadyDocument {";
    assert!(
        prepared.contains(SIGNATURE),
        "the frontier-crossing signature changed; it must take no work sink \
         and no source — post-frontier code cannot re-observe the document"
    );
    assert!(
        !prepared.contains("WorkSink"),
        "the commit module must not depend on any work sink"
    );
}

/// T18: the route decision has exactly the two frozen branches and no
/// fallback: facts preserved → local splice (`Known(0)`); facts differ →
/// same-target full build (`Known(1)`, reason facts-differ). There is no
/// third branch and no budget/cost selector.
#[test]
fn t18_route_decision_has_exactly_two_branches() {
    // Branch 1: the witness edit preserves facts → local splice.
    let (next_local, c_local, post_local) = witness_update();
    assert_eq!(c_local.full_build_selected, Observed::known(0));
    assert_eq!(c_local.full_build_reason, Observed::known(0));
    assert_eq!(next_local.owners.records(), 3);

    // Branch 2: appending a reference definition changes the new-side fact
    // set → same-target full build (W-A2 preserved, not repaired).
    let old_source = Source::new(SourceId(1), WITNESS_SOURCE);
    let old = witness_old();
    let e = edit(19, 19, "\n\n[ref]: /u");
    let post = e.apply(&old_source, SourceId(2)).expect("edit applies");
    let mut recording = RecordingHorseAStructuralSink::new();
    let next_full = update_with_structural(
        old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut recording,
    )
    .expect("the full route completes");
    let c_full = recording.into_counters();
    assert_eq!(c_full.full_build_selected, Observed::known(1));
    assert_eq!(c_full.full_build_reason, Observed::known(1));
    assert_eq!(next_full.owners.records(), 4);
    assert_eq!(next_full.source_len, post.len_bytes());

    // Both routes produce the same normalized result as a clean parse of
    // the post source (the oracle gate is exercised in full by the public
    // I5 conformance battery in `tests/`).
    let _ = post_local;
}
