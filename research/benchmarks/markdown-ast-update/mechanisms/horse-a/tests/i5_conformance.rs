//! I5 public-API conformance battery (task contract §30–§31/§35): C1/C2/C3
//! over the frozen correctness gates, exercised through BOTH structural
//! lanes (the no-op lane `update` and the recording lane
//! `update_with_structural` run the identical algorithm and must produce
//! identical results).
//!
//! - C1 — the incremental result equals the clean H0 parse of the post
//!   source (full normalized structural equality, never a hash), and the
//!   two lanes agree exactly;
//! - C2 — every returned state satisfies every READY invariant;
//! - C3 — continuation readiness: chains local→local, local→full,
//!   full→local, full→full keep producing correct READY states, and the
//!   correctness-only restore probe (a fresh same-target full build of
//!   the post source) agrees with the updated state.

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};
use markit_mdbench_horse_a::{
    full_build, update, update_with_structural, validate_ready, NoopHorseAStructuralSink,
    NormalizeV1, ReadyDocument, RecordingHorseAStructuralSink,
};
use markit_mdbench_oracle::validate_normalized;
use markit_mdbench_shared_grammar::parse_full;

/// The battery document: heading, paragraph, two reference definitions, a
/// list, a closing paragraph — enough topology that edits exercise local
/// routes with non-empty equal fact sets, local routes at certified
/// boundaries, and the full route on fact-changing edits.
const DOC: &str = concat!(
    "# Title\n\n",          // 0..9
    "See [a][x] here.\n\n", // 9..27
    "[x]: /x\n\n",          // 27..36
    "[y]: /y\n\n",          // 36..45
    "- one\n",              // 45..51
    "- two\n",              // 51..57
    "\n",                   // 57..58
    "End.\n",               // 58..63
);

/// The H0 clean-parse reference (the correctness-lane authority).
fn h0(text: &str) -> markit_mdbench_oracle::normalized::NormalizedDocument {
    let bytes = text.as_bytes();
    let mut noop = NoopWorkSink;
    let document = parse_full(bytes, &mut noop);
    if !bytes.is_empty() {
        validate_normalized(&document, Some(bytes))
            .expect("H0 result violates NORMALIZED-RESULT-v1");
    }
    document
}

/// The frozen oracle gate (spec §19): READY invariants + full normalized
/// structural equality against the clean parse.
fn assert_equals_h0(doc: &ReadyDocument, source: &Source) {
    validate_ready(doc).expect("READY invariants (C2)");
    let exported = doc.normalize_v1();
    if !source.as_str().is_empty() {
        validate_normalized(&exported, Some(source.as_bytes()))
            .expect("the updated export violates NORMALIZED-RESULT-v1");
    }
    assert_eq!(
        exported,
        h0(source.as_str()),
        "normalize(Horse-A update) != normalize(H0 clean parse) for {:?}",
        source.as_str()
    );
}

fn source(id: u64, text: &str) -> Source {
    Source::new(SourceId(id), text)
}

fn build(text: &str) -> ReadyDocument {
    full_build(
        &source(1, text),
        &mut NoopWorkSink,
        &mut NoopHorseAStructuralSink,
    )
    .expect("initial full build")
}

fn edit(start: usize, end: usize, inserted: &str) -> CanonicalEdit {
    CanonicalEdit::new(start, end, inserted).expect("edit geometry")
}

/// Run one update through the recording lane and through the no-op lane
/// from identical fresh states; gate both against H0 and assert the lanes
/// agree byte-for-byte on the normalized export.
fn assert_update_both_lanes(
    old_text: &str,
    start: usize,
    end: usize,
    inserted: &str,
) -> (ReadyDocument, Source) {
    let old_source = source(1, old_text);
    let e = edit(start, end, inserted);
    let post = e.apply(&old_source, SourceId(2)).expect("edit applies");

    let recording_lane = {
        let old = build(old_text);
        update_with_structural(
            old,
            &old_source,
            &post,
            &e,
            &mut NoopWorkSink,
            &mut RecordingHorseAStructuralSink::new(),
        )
        .unwrap_or_else(|err| panic!("recording-lane update failed for {old_text:?}: {err}"))
    };
    let noop_lane = {
        let old = build(old_text);
        update(old, &old_source, &post, &e, &mut NoopWorkSink)
            .unwrap_or_else(|err| panic!("no-op-lane update failed for {old_text:?}: {err}"))
    };

    assert_equals_h0(&recording_lane, &post);
    assert_equals_h0(&noop_lane, &post);
    assert_eq!(
        recording_lane.normalize_v1(),
        noop_lane.normalize_v1(),
        "the recording and no-op structural lanes must produce identical results"
    );
    (recording_lane, post)
}

/// C1 + C2: the edit battery. Every mutation gates against the clean H0
/// parse through both lanes, and every returned state satisfies the READY
/// invariants (checked inside `assert_equals_h0`).
#[test]
fn c1_c2_edit_battery_matches_clean_parse_through_both_lanes() {
    // Local route, text inside the closing paragraph.
    assert_update_both_lanes(DOC, 60, 60, "really ");
    // Local route, word replaced inside a list item.
    assert_update_both_lanes(DOC, 47, 50, "ONE");
    // Local route, word deleted inside a list item.
    assert_update_both_lanes(DOC, 47, 50, "");
    // Local route from the BOF authority (insert before the heading).
    assert_update_both_lanes(DOC, 0, 0, "Intro\n\n");
    // Local route at a certified boundary (zero-length at a blank line).
    assert_update_both_lanes(DOC, 35, 35, "");
    // Full route: changing a definition destination changes the fact set.
    assert_update_both_lanes(DOC, 32, 34, "/z");
    // Full route: a new definition appended at EOF changes the fact set.
    assert_update_both_lanes(DOC, DOC.len(), DOC.len(), "\n[z]: /z");
    // Local route spanning the whole closing region (delete the tail
    // paragraph and its separator).
    assert_update_both_lanes(DOC, 57, DOC.len(), "");
}

/// C3: continuation readiness. Chains keep updating the state the previous
/// step returned; every step gates against H0; the final state agrees with
/// the correctness-only restore probe (a fresh same-target full build of
/// the final post source).
#[test]
fn c3_continuation_chains_and_restore_probe() {
    let restore_probe = |state: &ReadyDocument, post: &Source| {
        let fresh = build(post.as_str());
        assert_eq!(
            state.normalize_v1(),
            fresh.normalize_v1(),
            "the restored (fresh full-build) state must equal the updated state"
        );
    };

    // local → local: two text edits, both on the local route.
    {
        let (state1, post1) = assert_update_both_lanes(DOC, 47, 50, "ONE");
        let e2 = edit(60, 60, "!");
        let post2 = e2.apply(&post1, SourceId(3)).expect("edit applies");
        let state2 =
            update(state1, &post1, &post2, &e2, &mut NoopWorkSink).expect("local→local step 2");
        assert_equals_h0(&state2, &post2);
        restore_probe(&state2, &post2);
    }

    // local → full: a text edit, then a definition change (facts differ).
    {
        let (state1, post1) = assert_update_both_lanes(DOC, 47, 50, "ONE");
        let e2 = edit(32, 34, "/z");
        let post2 = e2.apply(&post1, SourceId(3)).expect("edit applies");
        let state2 =
            update(state1, &post1, &post2, &e2, &mut NoopWorkSink).expect("local→full step 2");
        assert_equals_h0(&state2, &post2);
        restore_probe(&state2, &post2);
    }

    // full → local: a definition change, then a text edit.
    {
        let (state1, post1) = assert_update_both_lanes(DOC, 32, 34, "/z");
        let e2 = edit(60, 60, "!");
        let post2 = e2.apply(&post1, SourceId(3)).expect("edit applies");
        let state2 =
            update(state1, &post1, &post2, &e2, &mut NoopWorkSink).expect("full→local step 2");
        assert_equals_h0(&state2, &post2);
        restore_probe(&state2, &post2);
    }

    // full → full: two definition changes.
    {
        let (state1, post1) = assert_update_both_lanes(DOC, 32, 34, "/z");
        let e2 = edit(41, 43, "/w"); // "[y]: /y" -> "[y]: /w"
        let post2 = e2.apply(&post1, SourceId(3)).expect("edit applies");
        let state2 =
            update(state1, &post1, &post2, &e2, &mut NoopWorkSink).expect("full→full step 2");
        assert_equals_h0(&state2, &post2);
        restore_probe(&state2, &post2);
    }
}
