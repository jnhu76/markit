//! MARKIT-31-ALGORITHM-IDENTITY-AUDIT-1 — H0 FULL_REBUILD identity audit.
//!
//! Audit authority: PR #44 merge `0bf678cc` (R7 freeze). This file adds
//! audit-only tests; no production algorithm code is changed.
//!
//! Audit questions answered here (task §7, §23, §24, §39 Property C/D,
//! §41, §42):
//!
//! - H0-ID-1 (§7 / Property D): H0's update result AND work counters are
//!   independent of the old state and of update history — no accidental
//!   or hidden reuse of old AST/blocks/payloads/cached output.
//! - H0-ID-2 (§7): every EDIT_WRITE update parses the COMPLETE post-edit
//!   source: the derived unique POST coverage is exactly the whole
//!   source (union = [0, len)).
//! - H0-ID-3 (§24): the pending result is complete before `complete()`;
//!   after completion the state answers pure queries with no parser work
//!   and no deferred mutation (repeated projections are identical).
//! - H0-ID-4 (§23): fresh-state isolation — independently constructed
//!   states produce identical update behavior; no cross-iteration global
//!   state exists (static scan: `diagnostics/tests/anti_cheat.rs` +
//!   grep evidence in the audit record).
//! - H0-ID-5 (§41/§42): UTF-8/CJK byte-coordinate correctness and
//!   sentinel boundaries (byte 0, EOF, empty document).
//! - H0-ID-6 (§28/§54): the H0 attribution profile — `nodes_reused` is
//!   the measured zero, metadata/fallback NotApplicable, gauges
//!   NotApplicable — on update AND full_parse.

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, ResultChecksum, Source,
    WorkCounters,
};
use markit_mdbench_full_rebuild::{FullRebuildMechanism, H0State};
use markit_mdbench_oracle::normalized::normalized_checksum;
use markit_mdbench_oracle::NormalizeV1;

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

/// Drive one EDIT_WRITE update through the real phase boundary with one
/// attribution sink; returns the sealed state and the finalized counters.
fn h0_update(
    old: &Source,
    post: &Source,
    edit: &CanonicalEdit,
    old_state: H0State,
) -> (H0State, WorkCounters) {
    let mech = FullRebuildMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let state = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let prepared = mech
            .prepare_update(old, post, edit, &old_state, &mut cx)
            .expect("prepare");
        let pending = mech
            .update(old, post, edit, old_state, prepared, &mut cx)
            .expect("update");
        let done = mech.complete(pending).expect("complete");
        // Derive on the SAME sink that collected the events (the
        // collector owns the event list).
        sink.finalize_derived();
        done.state
    };
    (state, counters)
}

fn h0_state_of(bytes: &[u8]) -> H0State {
    let mech = FullRebuildMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let pending = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        mech.full_parse(&source_of(bytes, 1), &mut cx)
            .expect("full_parse")
    };
    mech.complete(pending).expect("complete").state
}

/// H0-ID-1 + Property D: two DIFFERENT histories reaching the same post
/// source produce byte-identical results and identical work counters.
/// A hidden reuse of old structure could not pass the counter identity:
/// any avoided work would shrink the second history's counts.
#[test]
fn h0_result_and_counters_are_history_independent() {
    let post_b = b"same post source\n\nsecond para\n";
    let post = source_of(post_b, 9);

    // History A: a long document, then delete everything after the first
    // line region.
    let hist_a_old = b"same post source\n\nsecond para\n\nUNRELATED TAIL\n\nmore\n";
    let edit_a = CanonicalEdit::new(31, hist_a_old.len(), "").expect("edit");
    let (state_a, counters_a) = h0_update(
        &source_of(hist_a_old, 2),
        &post,
        &edit_a,
        h0_state_of(hist_a_old),
    );

    // History B: a one-paragraph document, then append the second para.
    let hist_b_old = b"same post source\n";
    let edit_b = CanonicalEdit::new(16, 16, "\nsecond para\n").expect("edit");
    let (state_b, counters_b) = h0_update(
        &source_of(hist_b_old, 3),
        &post,
        &edit_b,
        h0_state_of(hist_b_old),
    );

    let want = normalized_checksum(&markit_mdbench_full_rebuild::parse_document(post_b));
    assert_eq!(state_a.result_checksum(), want);
    assert_eq!(state_b.result_checksum(), want);
    assert_eq!(
        counters_a.blocks_reparsed, counters_b.blocks_reparsed,
        "work must not depend on history"
    );
    assert_eq!(counters_a.nodes_rebuilt, counters_b.nodes_rebuilt);
    assert_eq!(
        counters_a.source_bytes_inspected_total,
        counters_b.source_bytes_inspected_total
    );
    assert_eq!(
        counters_a.unique_post_source_bytes,
        counters_b.unique_post_source_bytes
    );
}

/// H0-ID-2: an update's unique POST coverage is exactly the complete
/// post source (the union of the block-pass lines and the inline regions
/// is [0, len)), and `nodes_reused` is the measured zero — never a
/// fabricated reuse of the dropped old state.
#[test]
fn h0_update_covers_the_complete_post_source() {
    let old_b = b"para one\n\npara two\n";
    let edit = CanonicalEdit::new(5, 8, "ONE").expect("edit");
    let post_b = b"para ONE\n\npara two\n";
    let (_state, counters) = h0_update(
        &source_of(old_b, 2),
        &source_of(post_b, 3),
        &edit,
        h0_state_of(old_b),
    );
    assert_eq!(
        counters.unique_post_source_bytes,
        Observed::Known(post_b.len() as u64)
    );
    assert_eq!(counters.unique_post_source_intervals, Observed::Known(1));
    assert_eq!(counters.unique_old_source_bytes, Observed::Known(0));
    assert_eq!(counters.nodes_reused, Observed::Known(0));
    // The full rebuild reparsed every block of the post source. The
    // frozen H0 counting rule (R5 §11.6): every normalized node minus
    // the Document root; blocks are the block-kind nodes among them.
    let doc = markit_mdbench_full_rebuild::parse_document(post_b);
    const INLINE: [markit_mdbench_oracle::NodeKind; 5] = [
        markit_mdbench_oracle::NodeKind::Text,
        markit_mdbench_oracle::NodeKind::Emphasis,
        markit_mdbench_oracle::NodeKind::CodeSpan,
        markit_mdbench_oracle::NodeKind::Link,
        markit_mdbench_oracle::NodeKind::ReferenceLink,
    ];
    fn count(n: &markit_mdbench_oracle::Node, nodes: &mut u64, blocks: &mut u64) {
        if n.kind != markit_mdbench_oracle::NodeKind::Document {
            *nodes += 1;
            if !INLINE.contains(&n.kind) {
                *blocks += 1;
            }
        }
        for c in &n.children {
            count(c, nodes, blocks);
        }
    }
    let mut nodes = 0u64;
    let mut blocks = 0u64;
    count(&doc.root, &mut nodes, &mut blocks);
    assert_eq!(counters.blocks_reparsed, Observed::Known(blocks));
    assert_eq!(counters.nodes_rebuilt, Observed::Known(nodes));
}

/// H0-ID-3 (§24): the attribution profile is complete and honest on both
/// operations; the gauges and the two intrinsic N/A slots are declared,
/// never fabricated.
#[test]
fn h0_attribution_profile_update_and_full_parse() {
    let old_b = b"a\n\nb\n";
    let edit = CanonicalEdit::new(0, 1, "c").expect("edit");
    let post_b = b"c\n\nb\n";
    let (_state, counters) = h0_update(
        &source_of(old_b, 2),
        &source_of(post_b, 3),
        &edit,
        h0_state_of(old_b),
    );
    assert_eq!(counters.nodes_reused, Observed::Known(0));
    assert_eq!(counters.metadata_records_touched, Observed::NotApplicable);
    assert_eq!(counters.fallback_to_full_count, Observed::NotApplicable);
    assert_eq!(counters.restart_distance, Observed::NotApplicable);
    assert_eq!(counters.convergence_distance, Observed::NotApplicable);

    // full_parse: same profile.
    let mech = FullRebuildMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let pending = mech
            .full_parse(&source_of(b"x\n", 4), &mut cx)
            .expect("full_parse");
        mech.complete(pending).expect("complete");
        sink.finalize_derived();
    }
    assert_eq!(counters.nodes_reused, Observed::Known(0));
    assert_eq!(counters.metadata_records_touched, Observed::NotApplicable);
    assert_eq!(counters.fallback_to_full_count, Observed::NotApplicable);
}

/// H0-ID-4 (§24/Property C): after `complete()` the state answers pure
/// queries with no parser work and no deferred mutation: repeated
/// projections are identical, the checksum is stable, and the completed
/// state feeds a second update (no hidden once-init).
#[test]
fn h0_completed_state_is_pure_and_reusable() {
    let src = b"first\n\nsecond *emph* para\n";
    let state = h0_state_of(src);

    let doc1 = state.normalize_v1();
    let doc2 = state.normalize_v1();
    assert_eq!(doc1, doc2, "projection must be pure (no deferred mutation)");
    assert_eq!(state.result_checksum(), state.result_checksum());
    assert!(state.node_count() > 1);
    assert_eq!(state.node_count(), {
        fn count(n: &markit_mdbench_oracle::Node) -> usize {
            1 + n.children.iter().map(count).sum::<usize>()
        }
        count(&doc1.root)
    });
    assert_eq!(state.source_len_bytes, src.len());

    // The completed state is a valid input to the next update.
    let edit = CanonicalEdit::new(6, 6, "X").expect("edit");
    let post_b = b"first X\n\nsecond *emph* para\n";
    let (next, counters) = h0_update(&source_of(src, 2), &source_of(post_b, 3), &edit, state);
    let want = normalized_checksum(&markit_mdbench_full_rebuild::parse_document(post_b));
    assert_eq!(next.result_checksum(), want);
    assert_eq!(counters.nodes_reused, Observed::Known(0));
}

/// H0-ID-5 (§41/§42): UTF-8/CJK coordinates and sentinel boundaries.
#[test]
fn h0_utf8_and_boundary_edits() {
    // 3-byte CJK characters; edit boundaries between multibyte chars.
    let old_b = "中文段落一号\n\n第二段\n".as_bytes();
    // Insert one ASCII byte after the first CJK char (byte 3).
    let edit = CanonicalEdit::new(3, 3, "X").expect("edit");
    let post_b = edit.apply(&source_of(old_b, 1), SourceId(2)).expect("post");
    let (state, _) = h0_update(&source_of(old_b, 3), &post_b, &edit, h0_state_of(old_b));
    let want = normalized_checksum(&markit_mdbench_full_rebuild::parse_document(
        post_b.as_bytes(),
    ));
    assert_eq!(state.result_checksum(), want);
    assert_eq!(state.source_len_bytes, post_b.len_bytes());

    // Edit at byte 0 and at EOF.
    for (es, ee, ins) in [
        (0usize, 0usize, "HEAD "),
        (old_b.len(), old_b.len(), "\nTAIL"),
    ] {
        let edit = CanonicalEdit::new(es, ee, ins).expect("edit");
        let post_b = edit.apply(&source_of(old_b, 1), SourceId(2)).expect("post");
        let (state, counters) = h0_update(&source_of(old_b, 3), &post_b, &edit, h0_state_of(old_b));
        let want = normalized_checksum(&markit_mdbench_full_rebuild::parse_document(
            post_b.as_bytes(),
        ));
        assert_eq!(state.result_checksum(), want);
        assert_eq!(
            counters.unique_post_source_bytes,
            Observed::Known(post_b.len_bytes() as u64)
        );
    }

    // Empty document: full parse and an update from the empty state.
    let empty = h0_state_of(b"");
    assert_eq!(empty.source_len_bytes, 0);
    assert!(empty.document.root.children.is_empty());
    let edit = CanonicalEdit::new(0, 0, "born\n").expect("edit");
    let post_b = b"born\n";
    let (state, counters) = h0_update(&source_of(b"", 3), &source_of(post_b, 4), &edit, empty);
    let want = normalized_checksum(&markit_mdbench_full_rebuild::parse_document(post_b));
    assert_eq!(state.result_checksum(), want);
    assert_eq!(
        counters.unique_post_source_bytes,
        Observed::Known(post_b.len() as u64)
    );

    // An invalid byte boundary is rejected by the correct layer (common
    // edit validation), never silently accepted by the mechanism.
    let bad = CanonicalEdit::new(1, 1, "x").expect("range");
    assert!(bad
        .validate_against(&source_of("中\n".as_bytes(), 5))
        .is_err());
}
