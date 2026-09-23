//! MARKIT-31-ALGORITHM-IDENTITY-AUDIT-1 — H3 OLD_TREE_SUBTREE_REUSE
//! identity audit. Audit authority: PR #44 merge `0bf678cc`. No
//! production code is changed. Expected values are HAND-DERIVED from the
//! frozen mechanism rules (R5 freeze §8 + §12 corrective + R5-CORRECTIVE-1
//! §8) and from the shared grammar.
//!
//! Audit questions answered here:
//!
//! - H3-ID-1 (§14): how the old tree participates — patch-path damage
//!   flags, line-aligned forward-cursor consultations, Arc-shared takes;
//!   exact counters + the patched-ancestry change flags are observable
//!   in the retained state.
//! - H3-ID-2 (§16): relocation — a prefix insertion shifts the patched
//!   tree's derived positions; the unchanged suffix is still taken at
//!   its NEW coordinates; no stale ranges survive (result == H0).
//! - H3-ID-3 (§17): same bytes, changed forward state — a destroyed
//!   fence closer turns the unchanged remainder into fence body; the
//!   change-flag/margin machinery refuses every stale candidate.
//! - H3-ID-4 (§15): anti-oracle — the reference-environment repair is
//!   computed from the mechanism's OWN assembled table (a definition
//!   edit rematerializes the affected retained payload); no H0/oracle
//!   result can be an input (static guards: diagnostics/anti_cheat.rs,
//!   Cargo isolation; this test pins the dynamic behavior).
//! - H3-ID-5: multi-entry deletion with clamped patch arithmetic —
//!   stale positions refuse naturally, result == H0.
//! - H3-ID-6 (§22/§39/§41/§24): BREAK→RESTORE, determinism, CJK,
//!   completed-state purity.
//!
//! Counting rule (R5 §11.6): a plain paragraph = 2 native nodes; a
//! fence/definition = 1; cursor consultations are counted per live
//! block-start line.

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, ResultChecksum, Source,
    WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_old_tree_subtree_reuse::{H3State, OldTreeSubtreeReuseMechanism};
use markit_mdbench_oracle::normalized::normalized_checksum;
use markit_mdbench_oracle::validate_root;
use markit_mdbench_oracle::NormalizeV1;

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

fn at(haystack: &[u8], needle: &str) -> usize {
    let n = needle.as_bytes();
    haystack
        .windows(n.len())
        .position(|w| w == n)
        .unwrap_or_else(|| panic!("needle {needle:?} not present"))
}

fn post_of(old: &[u8], edit: &CanonicalEdit) -> Vec<u8> {
    edit.apply(&source_of(old, 99), SourceId(100))
        .expect("post")
        .as_bytes()
        .to_vec()
}

fn h3_state_of(bytes: &[u8]) -> H3State {
    let mech = OldTreeSubtreeReuseMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let pending = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        mech.full_parse(&source_of(bytes, 1), &mut cx)
            .expect("full_parse")
    };
    mech.complete(pending).expect("complete").state
}

/// ONE sink across BOTH phases — exactly the runner's A-LANE shape
/// (`run_update_attributed`): prepare-phase OLD-source reads (the patch
/// margins) must reach the same inspection-event log the derived
/// counters are computed from. A per-phase sink would silently drop the
/// prepare events at `finalize_derived` (it overwrites the derived
/// slots from its own event log only). Also returns the prepared value
/// (cloned before `update` consumes it): the change flags live on the
/// PATCHED OLD tree, and flagged nodes are never taken, so the
/// assembled state carries none.
fn h3_update(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H3State,
) -> (
    H3State,
    markit_mdbench_old_tree_subtree_reuse::H3Prepared,
    WorkCounters,
) {
    let mech = OldTreeSubtreeReuseMechanism::new();
    let old = source_of(old, 2);
    let post = source_of(post, 3);
    let mut counters = WorkCounters::all_unknown();
    let (state, prepared_clone) = {
        let mut sink = CounterSink::new(&mut counters);
        let prepared = {
            let mut cx = MechanismContext::new(&mut sink);
            mech.prepare_update(&old, &post, edit, &old_state, &mut cx)
                .expect("prepare")
        };
        let prepared_clone = prepared.clone();
        let mut cx = MechanismContext::new(&mut sink);
        let pending = mech
            .update(&old, &post, edit, old_state, prepared, &mut cx)
            .expect("update");
        let done = mech.complete(pending).expect("complete");
        sink.finalize_derived();
        (done.state, prepared_clone)
    };
    (state, prepared_clone, counters)
}

/// Same run, without the prepared damage map (for tests that do not
/// inspect change flags).
fn h3_run(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H3State,
) -> (H3State, WorkCounters) {
    let (state, _, counters) = h3_update(old, post, edit, old_state);
    (state, counters)
}

fn assert_matches_h0(state: &H3State, post: &[u8], context: &str) {
    let want = normalized_checksum(&parse_document(post));
    assert_eq!(
        state.result_checksum(),
        want,
        "{context}: result must equal H0"
    );
    let doc = state.normalize_v1();
    validate_root(&doc.root, Some(post))
        .unwrap_or_else(|e| panic!("{context}: retained state violates NORMALIZED-RESULT-v1: {e}"));
}

/// H3-ID-1: exact counters + observable change flags for a local edit
/// ("two" -> "TWO" in p2 of `para one / para two / para three`).
///
/// Hand derivation (spans exclude the terminating LF): p1 = [0,8),
/// p2 = [10,18), p3 = [20,30); edit es=15, ee=18, delta=0.
/// Patch: scanned = 3; first hit = p2 (patch_node -> 1 patched node,
/// changed); prev margin reads the OLD separation [8,15) = 7 bytes,
/// finds 2 LFs (blank line) -> p1 NOT marked; next margin reads the
/// POST separation [18,20) = 2 LFs -> p3 NOT marked.
/// Forward cursor: take {p1} at 0 (extension stops at the changed p2),
/// take {p3} at 20; p2' fresh. reused = 4, rebuilt = 2, blocks = 1.
///
/// POST-source coverage is the FULL document (31/31), and that is the
/// derived value, not a discrepancy: the hook fires at every line start
/// (0, 8, 9, 10, 19, 20, 30) and each consult's live-side paragraph
/// margin reports `[prev_line_start - 1, pos - 1)` (R5-CORRECTIVE-2
/// closure, pass or fail). The blank-line consults reach back over the
/// neighboring lines — consult(8) reads [0,7) (p1's own content) and
/// consult(9) reads [0,8) — so taken bytes are re-read by the margin
/// machinery even when the take itself adds zero parser reads. This is
/// the H3 attribution profile: structure reuse with line-granular
/// consult reads. OLD coverage = [8,15) exactly (the prev margin).
#[test]
fn h3_exact_reuse_and_change_flags() {
    let old_b = b"para one\n\npara two\n\npara three\n";
    let es = at(old_b, "two");
    let edit = CanonicalEdit::new(es, es + 3, "TWO").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, prepared, counters) = h3_update(old_b, &post_b, &edit, h3_state_of(old_b));

    assert_matches_h0(&state, &post_b, "exact reuse");
    assert_eq!(counters.blocks_reparsed, Observed::Known(1), "p2' fresh");
    assert_eq!(counters.nodes_rebuilt, Observed::Known(2));
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(4),
        "takes p1 and p3 share whole Arc subtrees"
    );
    assert_eq!(counters.fallback_to_full_count, Observed::NotApplicable);
    assert_eq!(counters.restart_distance, Observed::NotApplicable);
    assert_eq!(
        counters.unique_old_source_bytes,
        Observed::Known(7),
        "the prev-continuation margin reads the OLD separation bytes [8,15)"
    );
    assert_eq!(
        counters.unique_post_source_bytes,
        Observed::Known(31),
        "full POST coverage: consult-time margins read one line back at \
         every line start, overlapping taken content (see doc comment)"
    );
    // metadata: prepare = 3 scanned + 1 patched; update = 7 hook
    // consultations (line starts 0,8,9,10,19,20,30) + 2 take slots.
    assert_eq!(counters.metadata_records_touched, Observed::Known(13));
    // The damage map is observable on the prepared tree: the linear scan
    // touched all 3 entries, and exactly the patched p2 ancestry carries
    // the changed flag (both margins saw blank-line separations).
    assert_eq!(prepared.scanned_entries, 3);
    assert_eq!(prepared.patched_nodes, 1);
    assert_eq!(prepared.tree.changed_count(), 1, "only p2's patched node");
    assert_eq!(
        state.tree().changed_count(),
        0,
        "flagged nodes are never taken, so the assembled tree carries no flags"
    );
}

/// H3-ID-2 (§16): relocation — a prefix insertion is a GAP edit (no
/// entry span intersects the zero-width edit at 0): the following
/// entry's gap absorbs the delta, so the UNCHANGED suffix aligns at its
/// NEW coordinates. The unchanged suffix is taken there; no stale
/// ranges survive (result == H0, and the normalized children
/// non-overlapping and strictly increasing).
///
/// The taken suffix is p2 ONLY: the gap edit's next-separation is the
/// empty range [6,6) — the margin cannot observe a blank line there,
/// so it conservatively marks p1 changed (patched) and p1' is reparsed.
/// Refusing a neighbor whose separation is unobservable is the
/// conservative direction of the frozen continuation rule (R5 §8).
#[test]
fn h3_prefix_insertion_relocates_the_suffix() {
    let old_b = b"para one\n\npara two\n";
    let edit = CanonicalEdit::new(0, 0, "HEAD\n\n").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, prepared, counters) = h3_update(old_b, &post_b, &edit, h3_state_of(old_b));

    assert_matches_h0(&state, &post_b, "prefix insertion");
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(2),
        "HEAD para fresh + p1' (margin-refused) fresh"
    );
    assert_eq!(counters.nodes_rebuilt, Observed::Known(4));
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(2),
        "p2 taken at its shifted NEW position 16"
    );
    // The gap edit left exactly one conservative damage flag on p1.
    assert_eq!(prepared.tree.changed_count(), 1, "p1 margin-marked");
    // Derived positions carry no stale offsets: the H0 equality above
    // compares absolute spans; here the walk additionally proves the
    // normalized children are ordered and non-overlapping (blank-line
    // gaps between blocks are not part of any block span).
    let doc = state.normalize_v1();
    let mut prev_end = 0u64;
    for child in &doc.root.children {
        assert!(child.start as u64 >= prev_end, "children must not overlap");
        assert!(child.end > child.start, "spans must be non-empty");
        prev_end = child.end as u64;
    }
    assert!(
        doc.root
            .children
            .iter()
            .any(|c| post_b[c.start..c.end] == *b"para two"),
        "the relocated para two exists under its new coordinates"
    );
}

/// H3-ID-3 (§17): same bytes, changed forward state. Destroying a fence
/// closer turns the UNCHANGED remainder of the document into fence
/// body: the cursor never consults inside the fence, every stale
/// candidate is refused, and the delivered tree is the fence-to-EOF.
#[test]
fn h3_fence_closer_removal_refuses_all_reuse() {
    let old_b = b"```go\nbody\n```\n\nunchanged tail para\n\nend\n";
    // The SECOND fence run (the closer).
    let first = at(old_b, "```");
    let second = old_b[first + 3..]
        .windows(3)
        .position(|w| w == b"```")
        .map(|p| p + first + 3)
        .expect("closer present");
    let es = second;
    let edit = CanonicalEdit::new(es, es + 3, "").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, prepared, counters) = h3_update(old_b, &post_b, &edit, h3_state_of(old_b));

    assert_matches_h0(&state, &post_b, "fence closer removal");
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(0),
        "unchanged bytes under changed forward state are never taken"
    );
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(1),
        "the EOF fence"
    );
    assert_eq!(counters.nodes_rebuilt, Observed::Known(1));
    // The patch flags exactly the fence ancestry (patch_node). The
    // next-entry margin reads the POST separation [11,13) = "\n\n"
    // (2 LFs) and correctly does NOT mark the tail — the tail's refusal
    // comes from the cursor: every consult inside the would-be fence
    // body carries the live fence ContextKey, which can never match a
    // top-level retained entry.
    assert_eq!(prepared.tree.changed_count(), 1, "only the patched fence");
}

/// H3-ID-4 (§15): anti-oracle, dynamic half — a definition edit changes
/// the environment; the taken member with reference-bearing bytes is
/// re-materialized against the mechanism's OWN rebuilt table (its
/// payload rebuilt, structure shared). The static half (no H0/oracle
/// dependency in lib code) is enforced by diagnostics/anti_cheat.rs and
/// the Cargo dependency graph.
#[test]
fn h3_definition_environment_repair_uses_the_own_table() {
    let mut doc = String::new();
    doc.push_str("see [a] and [b]\n\n");
    doc.push_str(&format!("{}\n", "x".repeat(130)));
    doc.push('\n');
    doc.push_str("[a]: /url\n");
    let old_b = doc.as_bytes();
    assert_eq!(at(old_b, "[a]: /url"), 149);

    let es = at(old_b, "/url");
    let edit = CanonicalEdit::new(es, es + 4, "/new").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters) = h3_run(old_b, &post_b, &edit, h3_state_of(old_b));

    assert_matches_h0(&state, &post_b, "def environment repair");
    // Same split as H2 (identical repair, patched-tree cursor): lit's
    // payload rebuilt (2), big1 kept (2); the def reparsed fresh (1).
    assert_eq!(counters.nodes_reused, Observed::Known(2));
    assert_eq!(counters.nodes_rebuilt, Observed::Known(3));
    assert_eq!(counters.blocks_reparsed, Observed::Known(1));
}

/// H3-ID-5: multi-entry deletion — the first hit's size clamps at zero,
/// the residual moves to the last hit, and every stale/changed position
/// refuses naturally; the delivered tree is the single surviving block.
#[test]
fn h3_multi_entry_deletion_clamps_and_reparses() {
    let old_b = b"aaa\n\nbbb\n\nccc\n";
    let edit = CanonicalEdit::new(0, 10, "").expect("edit");
    let post_b = post_of(old_b, &edit);
    assert_eq!(post_b, b"ccc\n");
    let (state, counters) = h3_run(old_b, &post_b, &edit, h3_state_of(old_b));

    assert_matches_h0(&state, &post_b, "multi-entry deletion");
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(0),
        "clamped/stale positions refuse; ccc was margin-marked changed"
    );
    assert_eq!(counters.blocks_reparsed, Observed::Known(1));
    assert_eq!(counters.nodes_rebuilt, Observed::Known(2));
}

/// H3-ID-6a (§22/§39): BREAK→RESTORE round trip + fresh-state determinism.
#[test]
fn h3_break_restore_and_determinism() {
    let old_b = b"first para\n\nsecond para\n\nthird para\n";
    let es = at(old_b, "second");
    let ee = at(old_b, "third");
    let break_edit = CanonicalEdit::new(es, ee, "").expect("edit");
    let broken = post_of(old_b, &break_edit);
    let (broken_state, _) = h3_run(old_b, &broken, &break_edit, h3_state_of(old_b));
    assert_matches_h0(&broken_state, &broken, "after BREAK");

    let restore_edit = CanonicalEdit::new(es, es, "second para\n\n").expect("edit");
    let (restored, restore_counters) = h3_run(&broken, old_b, &restore_edit, broken_state);
    assert_matches_h0(&restored, old_b, "after RESTORE");

    let fresh = h3_state_of(&broken);
    let (again, again_counters) = h3_run(&broken, old_b, &restore_edit, fresh);
    assert_matches_h0(&again, old_b, "fresh-state repeat");
    assert_eq!(
        restore_counters, again_counters,
        "all counters deterministic"
    );
}

/// H3-ID-6b (§41): CJK byte coordinates through the patched tree.
#[test]
fn h3_cjk_byte_coordinates() {
    let old_b = "中文第一段\n\n中文第二段\n\n第三段在远处\n".as_bytes();
    let es = at(old_b, "第一段");
    let edit = CanonicalEdit::new(es, es + 3, "ＸＹ").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, _) = h3_run(old_b, &post_b, &edit, h3_state_of(old_b));
    assert_matches_h0(&state, &post_b, "CJK edit");
}

/// H3-ID-6c (§24): completed-state purity.
#[test]
fn h3_completed_state_pure_query() {
    let old_b = b"para one\n\npara two\n\npara three\n";
    let es = at(old_b, "two");
    let edit = CanonicalEdit::new(es, es + 3, "TWO").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, _) = h3_run(old_b, &post_b, &edit, h3_state_of(old_b));
    assert_eq!(state.normalize_v1(), state.normalize_v1());
    assert_eq!(state.result_checksum(), state.result_checksum());
}
