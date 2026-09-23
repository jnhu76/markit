//! MARKIT-31-ALGORITHM-IDENTITY-AUDIT-1 — H2 FRAGMENT_REUSE identity
//! audit. Audit authority: PR #44 merge `0bf678cc`. No production code is
//! changed. Expected values are HAND-DERIVED from the frozen mechanism
//! rules (R5 freeze §7 + §12 corrective + R5-CORRECTIVE-1 §8) and from
//! the shared grammar.
//!
//! Audit questions answered here:
//!
//! - H2-ID-1 (§11): WHAT is reused is observable — a taken run shares
//!   whole old BLOCK subtrees at fragment-window-aligned positions; the
//!   exact `nodes_reused`/`nodes_rebuilt`/`blocks_reparsed` counts are
//!   hand-derived, and the reused interior is never scanned (no
//!   inspection event overlaps it).
//! - H2-ID-2 (§12): context sensitivity — identical bytes under changed
//!   forward state (unclosed fence) are NOT reused; result == H0.
//! - H2-ID-3 (§13): semantic rematerialization — `nodes_reused` (structure)
//!   is distinct from semantic work avoided: when the definition
//!   environment changes, a taken member's PAYLOAD is re-scanned
//!   (rebuilt) while its reference-insensitive co-members stay reused;
//!   the counters show exactly that split.
//! - H2-ID-4 (§10): the frozen minGap = 128 boundary tested from both
//!   sides (fragment exists at es=128, absent at es=127).
//! - H2-ID-5 (§22/§39): BREAK→RESTORE + fresh-state determinism.
//! - H2-ID-6 (§41): CJK byte coordinates.
//! - H2-ID-7 (§24): completed-state purity.
//!
//! Counting rule (R5 §11.6): one stored node = 1 block structure unit +
//! its inline forest. A plain paragraph = 2; a fence/definition = 1.

use markit_mdbench_common::source::SourceId;
use markit_mdbench_common::{
    CanonicalEdit, CounterSink, Mechanism, MechanismContext, Observed, ResultChecksum, Source,
    SourceVersion, WorkCounters,
};
use markit_mdbench_fragment_reuse::{FragmentReuseMechanism, H2State, MIN_GAP};
use markit_mdbench_full_rebuild::parse_document;
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

fn h2_state_of(bytes: &[u8]) -> H2State {
    let mech = FragmentReuseMechanism::new();
    let mut counters = WorkCounters::all_unknown();
    let pending = {
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        mech.full_parse(&source_of(bytes, 1), &mut cx)
            .expect("full_parse")
    };
    mech.complete(pending).expect("complete").state
}

fn h2_update(
    old: &[u8],
    post: &[u8],
    edit: &CanonicalEdit,
    old_state: H2State,
) -> (H2State, WorkCounters, Vec<(SourceVersion, u64, u64)>) {
    let mech = FragmentReuseMechanism::new();
    let old = source_of(old, 2);
    let post = source_of(post, 3);
    let mut counters = WorkCounters::all_unknown();
    let update_events;
    let state = {
        let prepared = {
            let mut prepare_sink = CounterSink::new(&mut counters);
            let mut pcx = MechanismContext::new(&mut prepare_sink);
            mech.prepare_update(&old, &post, edit, &old_state, &mut pcx)
                .expect("prepare")
        };
        let mut sink = CounterSink::new(&mut counters);
        let mut cx = MechanismContext::new(&mut sink);
        let pending = mech
            .update(&old, &post, edit, old_state, prepared, &mut cx)
            .expect("update");
        let done = mech.complete(pending).expect("complete");
        update_events = sink.inspections().to_vec();
        sink.finalize_derived();
        done.state
    };
    (state, counters, update_events)
}

fn assert_matches_h0(state: &H2State, post: &[u8], context: &str) {
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

/// Whether any POST-version inspection event overlaps `[s, e)` — the
/// "reused interior was never scanned" probe.
fn post_events_overlap(events: &[(SourceVersion, u64, u64)], s: u64, e: u64) -> bool {
    events
        .iter()
        .any(|&(v, es, ee)| v == SourceVersion::Post && es < e && ee > s)
}

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

/// A paragraph of `n` 'x' bytes followed by its LF.
fn xpara(n: usize) -> String {
    format!("{}\n", "x".repeat(n))
}

/// H2-ID-1: exact reuse on a far-from-the-edit suffix.
///
/// Document (hand-indexed): p1 = "para one" [0,8] blank [8,10) p2 =
/// "para two" [10,18] blank [18,20) big = 130 x's [20,150] blank
/// [150,152) after = "after tail" [152,163] tail LF. len = 164.
/// Edit "one"->"ONE" (es=5, delta 0).
///
/// Fragment table: es=5 < MIN_GAP -> NO left fragment; old.len - ee = 156
/// is at least 128 -> right fragment [8,164) to_old=0, open_start.
/// Safe windows: left_window_end = p1.start = 0 (byte 14 is in p2 -> the
/// adjacent block p2 is excluded; the window degenerates to empty on the
/// left side). right_window_start = end of the block containing ee=8 --
/// byte 8 is in the blank gap after p1, so the first block AT/after the
/// gap (p2) is excluded entirely -> window starts at p2.end = 18.
///
/// Consultations: the run {big, after} is taken at pos=20 (line-aligned,
/// ctx [] vouched, inside the window, ref-free): reused = 2 + 2 = 4.
/// p1 [0,8] is BEFORE the fragment (no left fragment) -> refused; p2 is
/// excluded by the right window -> reparsed.
/// blocks_reparsed = 2 (p1', p2'); nodes_rebuilt = 4; nodes_reused = 4.
#[test]
fn h2_exact_reuse_of_the_stable_suffix_run() {
    let mut doc = String::new();
    doc.push_str("para one\n\npara two\n\n");
    doc.push_str(&xpara(130));
    doc.push('\n');
    doc.push_str("after tail\n");
    let old_b = doc.as_bytes();
    assert_eq!(old_b.len(), 163);

    let es = at(old_b, "one");
    let edit = CanonicalEdit::new(es, es + 3, "ONE").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters, events) = h2_update(old_b, &post_b, &edit, h2_state_of(old_b));

    assert_matches_h0(&state, &post_b, "exact reuse");
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(2),
        "p1' + p2' fresh"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(4),
        "2 fresh paras x 2"
    );
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(4),
        "taken run big(2) + after(2) shares whole Arc subtrees"
    );
    assert_eq!(counters.fallback_to_full_count, Observed::NotApplicable);
    assert_eq!(counters.restart_distance, Observed::NotApplicable);
    // The taken run's interior [20,150) is never scanned by parser or
    // cursor. (The line immediately before the splice point — inside the
    // run's final member — IS read by the documented consult-time
    // paragraph margin and reported as an inspection event,
    // R5-CORRECTIVE-2; that read is attribution-visible mechanism work,
    // not parser work.)
    assert!(
        !post_events_overlap(&events, 20, 150),
        "no inspection event may overlap the reused interior [20,150)"
    );
    for &(_v, s, e) in events.iter().filter(|(v, _, _)| *v == SourceVersion::Post) {
        if s < 163 && e > 20 {
            assert!(
                s >= 150,
                "overlaps inside the run are confined to the margin-read tail; got ({s},{e})"
            );
        }
    }
    // Sub-full coverage (W1 law).
    let post_union = union_bytes(&events, SourceVersion::Post);
    assert!(post_union < post_b.len() as u64);
}

/// H2-ID-2 (§12): identical bytes under changed forward state are NOT
/// reused. Inserting an unclosed fence opener before "para two" turns
/// the unchanged remainder of the document into fence body: no block
/// start ever aligns, no take happens (nodes_reused == 0), and the
/// result equals H0 (one fence to EOF).
#[test]
fn h2_context_change_refuses_reuse_and_stays_correct() {
    let mut doc = String::new();
    doc.push_str("para one\n\npara two\n\n");
    doc.push_str(&xpara(130));
    doc.push('\n');
    doc.push_str("after tail\n");
    let old_b = doc.as_bytes();

    // Insert the fence opener at the start of the "para two" line.
    let es = at(old_b, "para two");
    let edit = CanonicalEdit::new(es, es, "```\n").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters, _) = h2_update(old_b, &post_b, &edit, h2_state_of(old_b));

    assert_matches_h0(&state, &post_b, "fence context change");
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(0),
        "unchanged bytes inside the new fence are fence body, not blocks"
    );
    // Everything after the opener is ONE fence (fresh).
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(2),
        "p1' + the EOF fence"
    );
}

/// H2-ID-3 (§13): `nodes_reused` (structure) is distinct from semantic
/// work avoided. A definition edit changes the document-global
/// environment; a TAKEN member whose bytes contain '[' but carried no
/// resolved reference (has_ref == false) must have its payload
/// RE-MATERIALIZED against the rebuilt table (1 + its inline forest
/// counts as rebuilt), while its reference-insensitive co-member stays
/// reused (structure shared, zero source reads).
///
/// Hand derivation: lit = "see [a] and [b]" [0,15] (all literal -> one
/// Text), blank, big1 = 130 x's [17,147], blank line, def =
/// "[a]: /url" [149,158]. (The blank line matters: a def DIRECTLY after
/// the big paragraph trips the left-edge continuation margin -- the
/// edit would reach into the boundary line -- and the whole left
/// fragment is conservatively dropped; that guard behavior is audited
/// separately in the gate suites.)
/// Edit /url -> /new at es=154 >= MIN_GAP: the LEFT fragment [0,154)
/// survives (to_old 0, open_end); no right fragment (1-byte tail).
/// definition_changing = the damaged def -> TRUE.
/// Take at pos 0: window (0, left_window_end=149): run {lit, big1}
/// vouched (has_ref false everywhere), reused = 2 + 2 = 4. The def line
/// itself parses fresh (its line start 149 is not < win_end 149... the
/// window check refuses it) -> table = [a -> /new] != retained ->
/// environment CHANGED. Assembly: lit contains '[' -> payload
/// RE-MATERIALIZED (rebuilt = 1 + 1 Text; subtracted members = 2; kept
/// = 0); big1 keeps its Arc. nodes_reused = 4 - 2 = 2; nodes_rebuilt =
/// fresh def 1 + remat 2 = 3; blocks_reparsed = 1 (only the def was
/// reparsed).
#[test]
fn h2_rematerialization_splits_structure_reuse_from_semantic_rebuild() {
    let mut doc = String::new();
    doc.push_str("see [a] and [b]\n\n");
    doc.push_str(&xpara(130));
    doc.push('\n');
    doc.push_str("[a]: /url\n");
    let old_b = doc.as_bytes();
    assert_eq!(at(old_b, "[a]: /url"), 149);

    let es = at(old_b, "/url");
    let edit = CanonicalEdit::new(es, es + 4, "/new").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, counters, _) = h2_update(old_b, &post_b, &edit, h2_state_of(old_b));

    assert_matches_h0(&state, &post_b, "def-environment change");
    // lit's native nodes (2) left the reused column: its PAYLOAD was
    // rebuilt; big1 (2) stays reused with zero source reads.
    assert_eq!(
        counters.nodes_reused,
        Observed::Known(2),
        "structure reuse = the reference-insensitive co-member only"
    );
    assert_eq!(
        counters.nodes_rebuilt,
        Observed::Known(3),
        "fresh def (1) + rematerialized lit payload (1 node + 1 Text)"
    );
    assert_eq!(
        counters.blocks_reparsed,
        Observed::Known(1),
        "only the definition was reparsed; lit/big1 structure was shared"
    );
}

/// H2-ID-4 (§10): the frozen minGap = 128 fragment boundary from both
/// sides. Two documents identical except one byte: at es=128 the left
/// fragment [0,128) exists and para 1 is taken (reused = 2); at es=127
/// no fragment exists and the same prefix is reparsed (reused = 0).
#[test]
fn h2_mingap_boundary_127_vs_128() {
    assert_eq!(MIN_GAP, 128, "the frozen @lezer default");
    // es = 128: para1 = 121 x's -> para1 [0,121] blank [121,123) para2
    // starts at 123; insert at byte 128 (inside para2).
    let doc_a = format!(
        "{}\n\n{}\n\n{}\n",
        "x".repeat(121),
        "y".repeat(60),
        "z".repeat(60)
    );
    let old_a = doc_a.as_bytes().to_vec();
    let es_a = 128usize;
    let edit_a = CanonicalEdit::new(es_a, es_a, "Z").expect("edit");
    let post_a = post_of(&old_a, &edit_a);
    let (state_a, counters_a, _) = h2_update(&old_a, &post_a, &edit_a, h2_state_of(&old_a));
    assert_matches_h0(&state_a, &post_a, "minGap side A (es=128, fragment exists)");
    assert_eq!(
        counters_a.nodes_reused,
        Observed::Known(2),
        "the left fragment survives at es = MIN_GAP: para1 is taken"
    );

    // es = 127: para1 = 120 x's -> para2 starts at 122; insert at 127.
    let doc_b = format!(
        "{}\n\n{}\n\n{}\n",
        "x".repeat(120),
        "y".repeat(60),
        "z".repeat(60)
    );
    let old_b = doc_b.as_bytes().to_vec();
    let es_b = 127usize;
    let edit_b = CanonicalEdit::new(es_b, es_b, "Z").expect("edit");
    let post_b = post_of(&old_b, &edit_b);
    let (state_b, counters_b, _) = h2_update(&old_b, &post_b, &edit_b, h2_state_of(&old_b));
    assert_matches_h0(&state_b, &post_b, "minGap side B (es=127, no fragment)");
    assert_eq!(
        counters_b.nodes_reused,
        Observed::Known(0),
        "below MIN_GAP no left fragment survives: the prefix is reparsed"
    );
}

/// H2-ID-5 (§22/§39 Properties A+C): BREAK→RESTORE round trip and
/// fresh-state determinism on the fence-closer class (the reference
/// environment flips and flips back).
#[test]
fn h2_break_restore_and_determinism() {
    let mut doc = String::new();
    doc.push_str("```go\nbody\n```\n\nuse [a]\n\n");
    doc.push_str(&xpara(130));
    let old_b = doc.as_bytes();
    assert!(old_b.len() > 130);

    // BREAK: delete the closing fence (3 bytes) -> the rest of the
    // document becomes fence body; [a] leaves the table.
    fn find_from(h: &[u8], n: &str, from: usize) -> usize {
        h.windows(n.len())
            .skip(from)
            .position(|w| w == n.as_bytes())
            .map(|p| p + from)
            .expect("needle")
    }
    let es = find_from(old_b, "```", at(old_b, "body") + 4);
    let break_edit = CanonicalEdit::new(es, es + 3, "").expect("edit");
    let broken = post_of(old_b, &break_edit);
    let (broken_state, _, _) = h2_update(old_b, &broken, &break_edit, h2_state_of(old_b));
    assert_matches_h0(&broken_state, &broken, "after BREAK");

    // RESTORE: put the closer back.
    let restore_edit = CanonicalEdit::new(es, es, "```").expect("edit");
    let (restored, restore_counters, _) = h2_update(&broken, old_b, &restore_edit, broken_state);
    assert_matches_h0(&restored, old_b, "after RESTORE");

    // Property C: same edit from an independently built state -> same
    // deterministic counters.
    let fresh = h2_state_of(&broken);
    let (again, again_counters, _) = h2_update(&broken, old_b, &restore_edit, fresh);
    assert_matches_h0(&again, old_b, "fresh-state repeat");
    assert_eq!(
        restore_counters, again_counters,
        "all counters deterministic"
    );
}

/// H2-ID-6 (§41): CJK bytes across fragment-mapped coordinates.
#[test]
fn h2_cjk_byte_coordinates() {
    let old_b = "中文第一段\n\n中文第二段\n\n第三段在远处\n".as_bytes();
    let es = at(old_b, "第一段");
    let edit = CanonicalEdit::new(es, es + 3, "ＸＹ").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, _, _) = h2_update(old_b, &post_b, &edit, h2_state_of(old_b));
    assert_matches_h0(&state, &post_b, "CJK edit");
}

/// H2-ID-7 (§24): completed-state purity.
#[test]
fn h2_completed_state_pure_query() {
    let mut doc = String::new();
    doc.push_str("para one\n\npara two\n\n");
    doc.push_str(&xpara(130));
    doc.push('\n');
    doc.push_str("after tail\n");
    let old_b = doc.as_bytes();
    let es = at(old_b, "one");
    let edit = CanonicalEdit::new(es, es + 3, "ONE").expect("edit");
    let post_b = post_of(old_b, &edit);
    let (state, _, _) = h2_update(old_b, &post_b, &edit, h2_state_of(old_b));
    let d1 = state.normalize_v1();
    let d2 = state.normalize_v1();
    assert_eq!(d1, d2, "projection must be pure");
    assert_eq!(state.result_checksum(), state.result_checksum());
}
