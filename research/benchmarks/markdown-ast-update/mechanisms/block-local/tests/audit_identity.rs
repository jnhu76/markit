//! MARKIT-31-ALGORITHM-IDENTITY-AUDIT-1 — H1 BLOCK_LOCAL_REPARSE identity
//! audit. Audit authority: PR #44 merge `0bf678cc`. No production code is
//! changed; every expected value below is HAND-DERIVED from the frozen
//! mechanism rules (R5 freeze §6 + R5-CORRECTIVE-1 §5/§11.6 + H1-A/H1-B)
//! and from the shared grammar, not from the mechanism under test.
//!
//! Audit questions answered here:
//!
//! - H1-ID-1 (§8): a safe local edit is genuinely a LOCAL region reparse
//!   plus prefix pass-through and delta-shifted suffix reconstruction —
//!   exact `blocks_reparsed`/`nodes_rebuilt`/`nodes_reused` counts, sub-
//!   full source coverage, no OLD-source reads.
//! - H1-ID-2 (§9/§34): the F1–F6 fallback fires AT MOST ONCE per update;
//!   probe + fallback keeps exactly one fallback event; discarded region
//!   work stays in the cumulative counters (H1-B).
//! - H1-ID-3 (§10): the guard boundary is the frozen semantic threshold,
//!   tested from both sides (F4(a) terminator-at-edge vs terminator-
//!   beyond-edge; F6 fence-bounded-within-region vs fence-open-past-
//!   region-end).
//! - H1-ID-4 (§42): insertion-gap mapping (pure boundary insert).
//! - H1-ID-5 (§22/§39): BREAK→RESTORE round trip; fresh-state determinism
//!   (same edit from independently built states → identical counters).
//! - H1-ID-6 (§41): CJK/UTF-8 byte coordinates.
//! - H1-ID-7 (§29/§30): unique vs cumulative inspection diverge visibly
//!   on the probe+fallback path (same bytes inspected twice).
//! - H1-ID-8 (§24): completed-state purity (pure projection, reusable
//!   sealed state).
//!
//! Counting rule reminder (R5 §11.6): a plain paragraph = 1 skeleton +
//! 1 Text = 2 native nodes; a fence or definition = 1 native node; a
//! blank entry stores no syntax. On a fallback the DISCARDED region
//! reparse keeps its skeleton units in blocks_reparsed AND nodes_rebuilt
//! (H1-B), on top of the delivered full parse's counts.

use markit_mdbench_block_local::{BlockLocalMechanism, H1State};
use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, ResultChecksum, Source,
    SourceVersion, WorkCounters,
};
use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::normalized_checksum;
use markit_mdbench_oracle::NormalizeV1;

const DOC3: &[u8] = b"alpha one\n\nalpha two\n\nalpha three\n";

fn source_of(bytes: &[u8], id: u64) -> Source {
    Source::new(
        SourceId(id),
        String::from_utf8(bytes.to_vec()).expect("input is UTF-8"),
    )
}

/// Byte offset of `needle` inside `haystack` — keeps edit coordinates
/// honest without hand-counting bytes.
fn at(haystack: &[u8], needle: &str) -> usize {
    let n = needle.as_bytes();
    haystack
        .windows(n.len())
        .position(|w| w == n)
        .unwrap_or_else(|| panic!("needle {needle:?} not present"))
}

/// The consistent post source for an edit (the mechanism-under-test and
/// the oracle must see byte-identical inputs).
fn post_of(old: &[u8], edit: &CanonicalEdit) -> Vec<u8> {
    edit.apply(&source_of(old, 99), SourceId(100))
        .expect("post")
        .as_bytes()
        .to_vec()
}

fn h1_state_of(bytes: &[u8]) -> H1State {
    let mech = BlockLocalMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let pending = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        mech.full_parse(&source_of(bytes, 1), &mut cx)
            .expect("full_parse")
    };
    mech.complete(pending).expect("complete").state
}

/// Drive prepare + update with ONE attribution sink — exactly the
/// runner's A-LANE shape (`run_update_attributed`): any prepare-phase
/// OLD-source read must reach the same inspection-event log the derived
/// counters are computed from, otherwise `finalize_derived` (which
/// overwrites the derived slots from its own event log only) would
/// silently drop it. Returns the sealed state, the finalized counters,
/// and the raw inspection events (both phases).
fn h1_update(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H1State,
) -> (H1State, WorkCounters, Vec<(SourceVersion, u64, u64)>) {
    let mech = BlockLocalMechanism::new();
    let old = source_of(old, 2);
    let post = source_of(post, 3);
    let mut counters = WorkCounters::all_unknown();
    let (state, events) = {
        let mut sink = CounterSink::new(&mut counters);
        let prepared = {
            let mut cx = MechanismContext::new(&mut sink);
            mech.prepare_update(&old, &post, edit, &old_state, &mut cx)
                .expect("prepare")
        };
        let mut cx = MechanismContext::new(&mut sink);
        let pending = mech
            .update(&old, &post, edit, old_state, prepared, &mut cx)
            .expect("update");
        let done = mech.complete(pending).expect("complete");
        let events = sink.inspections().to_vec();
        assert!(
            events
                .iter()
                .all(|(v, _, _)| matches!(v, SourceVersion::Old | SourceVersion::Post)),
            "inspection events must carry a source version"
        );
        sink.finalize_derived();
        (done.state, events)
    };
    (state, counters, events)
}

fn assert_matches_h0(state: &H1State, post: &[u8], context: &str) {
    let want = normalized_checksum(&parse_document(post));
    assert_eq!(
        state.result_checksum(),
        want,
        "{context}: H1 update result must equal the clean authoritative parse"
    );
    let doc = state.normalize_v1();
    markit_mdbench_oracle::validate_root(&doc.root, Some(post))
        .unwrap_or_else(|e| panic!("{context}: retained state violates NORMALIZED-RESULT-v1: {e}"));
}

/// Union of one version's event ranges.
fn union_bytes(events: &[(SourceVersion, u64, u64)], version: SourceVersion) -> u64 {
    let mut ranges: Vec<(u64, u64)> = events
        .iter()
        .filter(|(v, _, _)| *v == version)
        .map(|&(_, s, e)| (s, e))
        .collect();
    ranges.sort_unstable();
    let mut total = 0u64;
    let mut cur: Option<(u64, u64)> = None;
    for (s, e) in ranges {
        match cur {
            Some((cs, ce)) if s <= ce => cur = Some((cs, ce.max(e))),
            _ => {
                if let Some((cs, ce)) = cur.take() {
                    total += ce - cs;
                }
                cur = Some((s, e));
            }
        }
    }
    if let Some((cs, ce)) = cur {
        total += ce - cs;
    }
    total
}

/// H1-ID-1: hand-derived exact counters for the safe local edit
/// "two" -> "TWO" inside p2.
///
/// Hand derivation (frozen rules; DOC3 spans: p1=[0,9] blank=[9,11)
/// p2=[11,20] blank=[20,22) p3=[22,33] tail=[33,34)):
/// - damage scan hits p2 only; region = [rs=9, re_new=20) — p1's LF plus
///   the edited block, right edge = next entry start, delta 0;
/// - the region reparse sees TWO empty leading lines (the separator LFs)
///   and ONE paragraph;
/// - F4(a): the region's last line terminator (LF@20) lies exactly AT
///   the right edge -> pass; both continuation pairs see 2 LFs -> pass;
/// - prefix p1 MOVED (reused = 2); suffix = blank + p3 + tail is
///   delta-shift RECONSTRUCTION (rebuilt 2); region block fresh
///   (rebuilt 2);
/// - blocks_reparsed = 1, nodes_rebuilt = 4, nodes_reused = 2,
///   fallback = Known(0);
/// - post-side union = separator + region + guard bytes [9,22) = 13
///   (sub-full); OLD-side union = 0 (no old-source reads on this path).
#[test]
fn h1_safe_local_edit_exact_counters_and_subfull_coverage() {
    let es = at(DOC3, "two");
    let edit = CanonicalEdit::new(es, es + 3, "TWO").expect("edit");
    let post_b = post_of(DOC3, &edit);
    let (state, counters, events) = h1_update(DOC3, &post_b, &edit, h1_state_of(DOC3));

    assert_matches_h0(&state, &post_b, "safe local edit");
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(1),
        "region reparse = 1 block"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(4),
        "region block (2) + suffix reconstruction p3 (2)"
    );
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(2),
        "prefix p1 MOVED through (1 skeleton + 1 Text); blanks store no syntax"
    );
    assert_eq!(counters.fallback_to_full_count, Observed::Known(0));
    assert_eq!(
        counters.unique_old_source_bytes,
        Observed::Known(0),
        "the H1 update path reads no OLD source bytes"
    );
    let post_union = union_bytes(&events, SourceVersion::Post);
    assert_eq!(
        post_union, 13,
        "post coverage = [9,22): separators + region + guard separation reads"
    );
    assert!(
        post_union < post_b.len() as u64,
        "sub-full coverage (W1 law)"
    );
    assert_eq!(
        counters.unique_post_source_bytes,
        Observed::Known(post_union),
        "derived union matches the event union"
    );
}

/// H1-ID-2 + H1-ID-7 (§9/§34/§29/§30): a guard probe followed by the
/// total fallback records EXACTLY ONE fallback event; the discarded
/// region's skeleton work stays in the cumulative counters (H1-B); the
/// same post bytes are inspected by BOTH the discarded region reparse
/// and the delivered full parse, so cumulative inspection strictly
/// exceeds unique coverage — the S30 divergence microcase.
///
/// Case: delete ONE LF of the blank separator between p2 and p3. The
/// region [20,21) is empty of blocks; the right-edge guard finds the
/// post line paragraph-continuing and fires. The delivered full parse
/// has TWO blocks (p1 and the merged "alpha two alpha three" paragraph).
#[test]
fn h1_probe_plus_fallback_counts_fallback_once_and_keeps_divergence() {
    // The separator blank line between "alpha two" and "alpha three":
    // delete its LF (the blank line's own terminator, NOT a paragraph
    // byte — the byte right after "alpha two"'s LF).
    let cut = at(DOC3, "alpha two") + "alpha two".len() + 1;
    let edit = CanonicalEdit::new(cut, cut + 1, "").expect("edit");
    let post_b = post_of(DOC3, &edit);
    let (state, counters, events) = h1_update(DOC3, &post_b, &edit, h1_state_of(DOC3));

    assert_matches_h0(&state, &post_b, "probe+fallback");
    assert_eq!(
        counters.fallback_to_full_count,
        Observed::Known(1),
        "exactly one fallback event per update, even after the probe reparse"
    );
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(2),
        "discarded region produced 0 blocks; the full parse rebuilt 2 paragraphs \
         (the last two MERGE — that merge is what the guard caught)"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(4),
        "the full parse's 2 paragraphs x (1 skeleton + 1 Text); the discarded \
         region produced 0 skeleton units"
    );
    assert_eq!(counters.nodes_reused, Observed::Known(0));
    assert_eq!(
        counters.unique_post_source_bytes,
        Observed::Known(post_b.len() as u64),
        "the fallback's full parse covers every post byte"
    );
    assert!(
        counters.source_bytes_inspected_total.known_value().unwrap()
            > counters.unique_post_source_bytes.known_value().unwrap(),
        "cumulative inspection > unique coverage: the probed region was read twice"
    );
    let post_union = union_bytes(&events, SourceVersion::Post);
    assert_eq!(post_union, post_b.len() as u64);
    assert_eq!(counters.unique_old_source_bytes, Observed::Known(0));
}

/// H1-ID-2b (§9/§34): the F1 definition fallback fires exactly once for
/// a definition-bearing document even when the edit is far from the
/// definition (W2 law). The discarded region's 1 paragraph skeleton is
/// kept (H1-B) on top of the full parse's counts.
#[test]
fn h1_f1_definition_fallback_exactly_once() {
    let old_b = b"para one\n\npara two\n\n[a]: /url\n";
    let es = at(old_b, "two");
    let edit = CanonicalEdit::new(es, es + 3, "TWO").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters, _) = h1_update(old_b, &post_b, &edit, h1_state_of(old_b));

    assert_matches_h0(&state, &post_b, "F1 fallback");
    assert_eq!(counters.fallback_to_full_count, Observed::Known(1));
    // Discarded region probe: 1 paragraph skeleton. Delivered full parse:
    // 2 paragraphs + 1 definition = 3 block units.
    assert_eq!(counters.blocks_reparsed, Observed::Known(4));
    // Full parse native nodes: 2 paras x 2 + 1 def = 5; + discarded 1.
    assert_eq!(counters.nodes_rebuilt, Observed::Known(6));
    assert_eq!(counters.nodes_reused, Observed::Known(0));
}

/// H1-ID-3a (§10): the F4(a) terminator boundary — a region whose last
/// line's terminator lies BEYOND the region edge fires the guard (the
/// clean parse merges the line into the prefix paragraph, which the
/// region parser cannot see). The pass side (terminator exactly AT the
/// edge) is pinned by h1_safe_local_edit_exact_counters.
#[test]
fn h1_f4a_terminator_beyond_the_cut_falls_back() {
    let old_b = b"p1\n# H\n\nafter\n";
    let es = at(old_b, "# H");
    let edit = CanonicalEdit::new(es, es + 3, "text").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters, _) = h1_update(old_b, &post_b, &edit, h1_state_of(old_b));

    assert_matches_h0(&state, &post_b, "F4(a) beyond-edge");
    // post: ONE merged paragraph "p1\ntext" + "after".
    assert_eq!(counters.fallback_to_full_count, Observed::Known(1));
    // Discarded region probe: 1 paragraph skeleton. Full parse: 2 paras.
    assert_eq!(counters.blocks_reparsed, Observed::Known(3));
    // Discarded: 1 (skeleton only). Full: 2 x 2 = 4.
    assert_eq!(counters.nodes_rebuilt, Observed::Known(5));
    assert_eq!(counters.nodes_reused, Observed::Known(0));
}

/// H1-ID-3b (§10): the F6 fence boundary from both sides.
/// Side 1: an edit inside a CLOSED fence reparses the fence locally (the
/// fence is bounded within the region — no fallback).
/// Side 2: an edit that destroys the closer leaves the region parse with
/// an open fence BEFORE the document end -> F6 fires, total fallback.
#[test]
fn h1_f6_fence_boundary_both_sides() {
    // Side 1: local fence reparse. old: fence=[0,15) blank=[15,17)
    // after=[17,22] tail=[22,23). Insert X after "body1" (byte 11).
    let old_b = b"```go\nbody1\n```\n\nafter\n";
    let es = at(old_b, "body1") + 5;
    let edit = CanonicalEdit::new(es, es, "X").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters, _) = h1_update(old_b, &post_b, &edit, h1_state_of(old_b));
    assert_matches_h0(&state, &post_b, "F6 bounded side");
    assert_eq!(counters.fallback_to_full_count, Observed::Known(0));
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(1),
        "the fence block"
    );
    // Fresh fence (1) + suffix "after" reconstruction (2).
    assert_eq!(counters.nodes_rebuilt, Observed::Known(3));
    assert_eq!(counters.nodes_reused, Observed::Known(0), "prefix is empty");

    // Side 2: the closer is destroyed -> the region ends inside an open
    // fence with bytes beyond the region -> F6.
    let old_b = b"```go\nbody1\n```\n\nafter\n";
    let es = at(old_b, "body1") + 5;
    let ee = at(old_b, "after") - 1; // through the closer line's LF
    let edit = CanonicalEdit::new(es, ee, "X").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters, _) = h1_update(old_b, &post_b, &edit, h1_state_of(old_b));
    assert_matches_h0(&state, &post_b, "F6 unbounded side");
    assert_eq!(counters.fallback_to_full_count, Observed::Known(1));
    // Discarded region probe: 1 fence skeleton; delivered: 1 fence (EOF).
    assert_eq!(counters.blocks_reparsed, Observed::Known(2));
    assert_eq!(counters.nodes_rebuilt, Observed::Known(2));
    assert_eq!(counters.nodes_reused, Observed::Known(0));
}

/// H1-ID-4 (§42): insertion-gap mapping — a pure insert at the exact
/// boundary between two blocks maps the region to the inserted bytes
/// themselves; no fallback; the prefix/suffix split follows the gap.
#[test]
fn h1_insertion_gap_mapping_pure_insert() {
    // Insert a new paragraph at the exact start of p2 (byte 11).
    let edit = CanonicalEdit::new(11, 11, "inserted\n\n").expect("edit");
    let post_b = post_of(DOC3, &edit);
    let (state, counters, _) = h1_update(DOC3, &post_b, &edit, h1_state_of(DOC3));

    assert_matches_h0(&state, &post_b, "gap insert");
    assert_eq!(counters.fallback_to_full_count, Observed::Known(0));
    // No entry strictly overlaps the empty range -> insertion-gap
    // mapping: after = p2's index; region = [rs=11, re_new=21) — exactly
    // the inserted bytes. ONE new block reparsed.
    assert_eq!(counters.blocks_reparsed, Observed::Known(1));
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(2),
        "prefix = [p1, blank]; only p1 stores syntax"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(6),
        "region block (2) + suffix reconstruction p2 AND p3 (4)"
    );
    assert_eq!(
        counters.metadata_records_touched,
        Observed::Known(12),
        "6 scanned + 2 region fill + 4 suffix (R5 §6 counter applicability)"
    );
}

/// H1-ID-5a (§22/§39 Property A): BREAK→RESTORE round trip — every step
/// equals the clean H0 parse, including all retained semantic state.
#[test]
fn h1_break_restore_round_trip() {
    let clean = b"first para\n\nsecond para\n\nthird para\n";
    // BREAK: delete the whole second paragraph INCLUDING its LF.
    let es = at(clean, "second");
    let ee = at(clean, "third");
    let break_edit = CanonicalEdit::new(es, ee, "").expect("edit");
    let broken = post_of(clean, &break_edit);
    // RESTORE: put the paragraph AND its trailing blank line back at the
    // same position (exactly reconstructing `clean`).
    let restore_edit = CanonicalEdit::new(es, es, "second para\n\n").expect("edit");

    let mech = BlockLocalMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let state = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let pending = mech
            .full_parse(&source_of(clean, 1), &mut cx)
            .expect("parse");
        sink.finalize_derived();
        mech.complete(pending).expect("complete").state
    };
    assert_matches_h0(&state, clean, "clean baseline");

    let (broken_state, broken_counters, _) = h1_update(clean, &broken, &break_edit, state);
    assert_matches_h0(&broken_state, &broken, "after BREAK");
    assert_eq!(broken_counters.fallback_to_full_count, Observed::Known(0));

    let (restored, restore_counters, _) = h1_update(&broken, clean, &restore_edit, broken_state);
    assert_matches_h0(&restored, clean, "after RESTORE");
    assert_eq!(restored.source_len_bytes(), clean.len());

    // Property C: the same edit from a freshly built state produces the
    // same deterministic counters.
    let fresh = h1_state_of(&broken);
    let (again, again_counters, _) = h1_update(&broken, clean, &restore_edit, fresh);
    assert_matches_h0(&again, clean, "fresh-state repeat");
    assert_eq!(
        restore_counters.fallback_to_full_count,
        again_counters.fallback_to_full_count
    );
    assert_eq!(
        restore_counters.blocks_reparsed,
        again_counters.blocks_reparsed
    );
    assert_eq!(restore_counters.nodes_rebuilt, again_counters.nodes_rebuilt);
    assert_eq!(restore_counters.nodes_reused, again_counters.nodes_reused);
    assert_eq!(
        restore_counters.metadata_records_touched,
        again_counters.metadata_records_touched
    );
    assert_eq!(
        restore_counters.source_bytes_inspected_total,
        again_counters.source_bytes_inspected_total
    );
}

/// H1-ID-5b (§23): interleaved fresh states never interfere — two
/// independently built states updated in alternation produce identical
/// results and counters (no shared mutable document state).
#[test]
fn h1_fresh_state_isolation_under_interleaving() {
    let old_a = b"doc A one\n\ndoc A two\n";
    let old_b = b"doc B one\n\ndoc B two\n\nx\n";
    let edit_a = CanonicalEdit::new(at(old_a, "two"), at(old_a, "two") + 3, "TWO").expect("edit");
    let edit_b = CanonicalEdit::new(at(old_b, "two"), at(old_b, "two") + 3, "TWO").expect("edit");
    let post_a = post_of(old_a, &edit_a);
    let post_b = post_of(old_b, &edit_b);

    let state_a1 = h1_state_of(old_a);
    let state_b1 = h1_state_of(old_b);
    let (out_a1, c_a1, _) = h1_update(old_a, &post_a, &edit_a, state_a1);
    let state_a2 = h1_state_of(old_a);
    let (out_b1, _c_b1, _) = h1_update(old_b, &post_b, &edit_b, state_b1);
    let (out_a2, c_a2, _) = h1_update(old_a, &post_a, &edit_a, state_a2);

    assert_eq!(out_a1.result_checksum(), out_a2.result_checksum());
    assert_eq!(c_a1, c_a2, "counters deterministic across interleaving");
    assert_matches_h0(&out_b1, &post_b, "interleaved B");
}

/// H1-ID-6 (§41): CJK bytes — a replacement edit inside a CJK paragraph
/// keeps every derived span on char boundaries and equals H0.
#[test]
fn h1_cjk_byte_coordinates() {
    let old_b = "中文第一段\n\n中文第二段\n".as_bytes();
    let es = at(old_b, "第一段");
    let edit = CanonicalEdit::new(es, es + 3, "ＸＹ").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters, _) = h1_update(old_b, &post_b, &edit, h1_state_of(old_b));
    assert_matches_h0(&state, &post_b, "CJK edit");
    assert_eq!(counters.fallback_to_full_count, Observed::Known(0));
    assert!(counters.unique_post_source_bytes.known_value().unwrap() < post_b.len() as u64);
}

/// H1-ID-8 (§24): after `complete()` the retained state answers pure
/// queries; the projection is stable and the completed state feeds the
/// next update.
#[test]
fn h1_completed_state_pure_query() {
    let es = at(DOC3, "two");
    let edit = CanonicalEdit::new(es, es + 3, "TWO").expect("edit");
    let post_b = post_of(DOC3, &edit);
    let (state, _, _) = h1_update(DOC3, &post_b, &edit, h1_state_of(DOC3));
    let d1 = state.normalize_v1();
    let d2 = state.normalize_v1();
    assert_eq!(d1, d2, "normalize_v1 must be pure over the retained state");
    assert_eq!(state.result_checksum(), state.result_checksum());

    // The completed state is consumable by a further update.
    let next_es = at(&post_b, "alpha");
    let next_edit = CanonicalEdit::new(next_es, next_es + 5, "ALPHA").expect("edit");
    let next_post = post_of(&post_b, &next_edit);
    let (next, counters, _) = h1_update(&post_b, &next_post, &next_edit, state);
    assert_matches_h0(&next, &next_post, "second update from completed state");
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(0),
        "the edit hits the FIRST entry (p1), so no prefix exists to reuse"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(6),
        "region block (2) + suffix reconstruction p2'/p3 (4)"
    );
}
