//! I5 frontier tests (task contract §4–§6/§8/§22/§32; #59 §6.1/§9/§11): the
//! staging state and the prepared commit are distinct states with a real
//! ownership and failure boundary, the geometry route is recorded, and the
//! committed sentinel sites measure zero on the valid local path.

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};

use crate::full_build::full_build;
use crate::structural::{
    HorseAStructuralCountersV1, NoopHorseAStructuralSink, Observed, RecordingHorseAStructuralSink,
};
use crate::update::stage;

fn edit(start: usize, end: usize, inserted: &str) -> CanonicalEdit {
    CanonicalEdit::new(start, end, inserted).expect("edit geometry")
}

/// `stage → prepare → commit` is the frozen pipeline: staging is fallible
/// and leaves the old state owned and coherent; `prepare` consumes the old
/// state by ownership; `commit` is infallible and returns READY.
#[test]
fn the_frontier_pipeline_preserves_the_frozen_ownership_shape() {
    let old_source = Source::new(SourceId(1), "alpha\n\nbeta\n\ngamma\n");
    let old = full_build(
        &old_source,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("initial full build");

    let e = edit(3, 3, "X");
    let post = e.apply(&old_source, SourceId(2)).expect("edit applies");

    // Staging borrows: the old document is still owned by the caller
    // afterwards (here: still readable and complete).
    let staged = stage(
        &old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("the update stages");
    assert_eq!(old.source_len, old_source.len_bytes(), "old stays coherent");

    // Formation consumes the old document by ownership (it is moved in).
    let prepared = staged
        .prepare(old, &mut NoopHorseAStructuralSink)
        .expect("the bounded commit workspace prepares");

    // Crossing is infallible — no Result, no fallback branch.
    let next = prepared.commit(&mut NoopHorseAStructuralSink);
    assert_eq!(next.source_id, SourceId(2));
    assert_eq!(next.source_len, post.len_bytes());
}

/// The geometry/route record is frozen at its decision points and the
/// committed sentinel sites measure zero on the valid local path
/// (`Known(0)`, not `Unknown`).
#[test]
fn the_local_route_records_geometry_and_zero_sentinels() {
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
    let next = crate::update::update_with_structural(
        old,
        &old_source,
        &post,
        &e,
        &mut NoopWorkSink,
        &mut recording,
    )
    .expect("the update completes on the local path");

    let c: &HorseAStructuralCountersV1 = recording.counters();
    assert_eq!(c.schema, HorseAStructuralCountersV1::SCHEMA);

    // Frozen witness-shape geometry of this edit (hand-derived: the guard
    // Owner "beta" starts at 7, the damaged Owner "gamma" at 13, no
    // convergence before EOF — the damaged Owner's own boundary is the EOF
    // boundary and is never certified).
    assert_eq!(c.restart_old, Observed::known(7));
    assert_eq!(c.convergence_old, Observed::known(19));
    assert_eq!(c.convergence_new, Observed::known(20));
    assert_eq!(c.replace_lo, Observed::known(1));
    assert_eq!(c.replace_hi, Observed::known(3));
    assert_eq!(c.full_build_selected, Observed::known(0));
    assert_eq!(c.full_build_reason, Observed::known(0));

    // Scale diagnostics: 3 records before and after (one Owner replaced
    // by one Owner).
    assert_eq!(c.m_old, Observed::known(3));
    assert_eq!(c.m_new, Observed::known(3));

    // The committed sentinel sites executed and measured zero.
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

    // The returned state is a normal READY state (continuation-ready).
    assert_eq!(next.owners.records(), 3);
}
