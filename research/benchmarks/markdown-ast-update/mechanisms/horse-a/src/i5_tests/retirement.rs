//! I5 explicit-attribution retirement tests (task contract §9–§11/§22/§34;
//! #59 §11, §11.1, §20). The retirement walk must charge exactly one AVL
//! frame per retired record, one payload frame per destroyed normalized
//! payload node, and deepen the depth gauge per frame kind in its own
//! recursion space — with no recomputes, no link writes, and no aggregate
//! reads anywhere in retirement.
//!
//! Fixture: `"alpha\n\nbeta\n\ngamma\n"` — the #60 witness shape. Each of
//! the three Owners is a paragraph whose normalized payload is the
//! paragraph node plus its text child (2 payload nodes, depth 2). The
//! local fixture replaces ranks 1..3 (beta, gamma): Δ_old = 2 retired
//! records, P_removed = 4 destroyed payload nodes, D_payload = 2. A
//! 2-record subtree has height 2 regardless of shape, so the frozen
//! witness gauge bound max(H_detached, D_payload) = max(2, 2) = 2 is
//! shape-independent here.

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};

use crate::full_build::full_build;
use crate::structural::{
    HorseAStructuralCountersV1, NoopHorseAStructuralSink, Observed, RecordingHorseAStructuralSink,
};
use crate::update::update_with_structural;

fn edit(start: usize, end: usize, inserted: &str) -> CanonicalEdit {
    CanonicalEdit::new(start, end, inserted).expect("edit geometry")
}

#[test]
fn the_local_route_retires_exactly_the_detached_middle() {
    let old_source = Source::new(SourceId(1), "alpha\n\nbeta\n\ngamma\n");
    let old = full_build(
        &old_source,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("initial full build");

    let e = edit(13, 13, "X"); // insert at the physical start of "gamma"
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

    let c: &HorseAStructuralCountersV1 = recording.counters();

    // The drop walk visited exactly the detached records (Δ_old = 2) and
    // destroyed exactly their payload nodes (2 records × 2 nodes = 4).
    assert_eq!(c.retire_node_visits, Observed::known(2));
    assert_eq!(c.payload_nodes_retired, Observed::known(4));

    // One recursion frame per retired AVL record AND per destroyed payload
    // node: Δ_old + P_removed = 6 (§11.1).
    assert_eq!(c.retirement_frames_entered, Observed::known(6));

    // The depth gauge is a max over the two separate recursion spaces:
    // the AVL space reaches H_detached = 2 and the payload space reaches
    // D_payload = 2, so the gauge is 2 — never 4 (the spaces do not add).
    assert_eq!(c.max_retirement_depth, Observed::known(2));

    // Retirement performs NO recomputes, NO link writes, and NO aggregate
    // reads: nothing detached is relinked, nothing dropped is rewritten.
    // The fixture's splice/relink work is charged elsewhere and is not
    // asserted here; the retirement-only guarantee is that the fields
    // above are the *complete* retirement ledger and the retained state
    // after commit is a normal READY state.
    assert_eq!(next.owners.records(), 3);
}

#[test]
fn the_full_route_retires_the_complete_old_document() {
    // Appending a reference definition changes the new-side fact set
    // (W-A2 preserved: facts differ → same-target full build), so the
    // complete old representation retires: all 3 records. (The source is
    // 19 bytes; the edit inserts at EOF, after "gamma\n".)
    let old_source = Source::new(SourceId(1), "alpha\n\nbeta\n\ngamma\n");
    let old = full_build(
        &old_source,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("initial full build");

    let e = edit(19, 19, "\n\n[ref]: /u");
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
    .expect("the update completes");

    let c: &HorseAStructuralCountersV1 = recording.counters();

    // The conservative route was taken and the whole old state retired.
    assert_eq!(c.full_build_selected, Observed::known(1));
    assert_eq!(c.owners_removed, Observed::known(3));
    assert_eq!(c.retire_node_visits, Observed::known(3));
    assert_eq!(c.payload_nodes_retired, Observed::known(6));

    // Frames: 3 AVL records + 6 payload nodes = 9; gauge: the old tree of
    // 3 records has height 2 and the payload depth is 2 → max = 2.
    assert_eq!(c.retirement_frames_entered, Observed::known(9));
    assert_eq!(c.max_retirement_depth, Observed::known(2));

    // The result is READY and correct in shape (3 old + 1 new definition).
    assert_eq!(next.owners.records(), 4);
}
