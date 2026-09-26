//! I4 incremental-update correctness battery (task contract §25–§27/§33):
//! the public-API oracle over literal sources.
//!
//! For every successful update the gate is full normalized structural
//! equality against clean H0 parsing of the new source — never a hash — plus
//! every READY invariant of the returned state. Continuation readiness is
//! exercised by chains that keep updating the state the previous update
//! returned.

use markit_mdbench_common::{CanonicalEdit, NoopWorkSink, Source, SourceId};
use markit_mdbench_horse_a::{full_build, update, validate_ready, NormalizeV1, ReadyDocument};
use markit_mdbench_oracle::validate_normalized;
use markit_mdbench_shared_grammar::parse_full;

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

/// The frozen oracle (spec §19): full normalized structural equality.
fn assert_equals_h0(doc: &ReadyDocument, source: &Source) {
    validate_ready(doc).expect("READY invariants after the update");
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
    full_build(&source(1, text), &mut NoopWorkSink).expect("initial full build")
}

/// Run one update from a fresh initial state and return the READY result.
fn run_case(old_text: &str, start: usize, end: usize, inserted: &str) -> (ReadyDocument, Source) {
    let old_source = source(1, old_text);
    let old = full_build(&old_source, &mut NoopWorkSink).expect("initial full build");
    let edit = CanonicalEdit::new(start, end, inserted).expect("edit geometry");
    edit.validate_against(&old_source)
        .expect("edit vs old source");
    let post = edit.apply(&old_source, SourceId(2)).expect("edit applies");
    let next = update(old, &old_source, &post, &edit, &mut NoopWorkSink)
        .unwrap_or_else(|e| panic!("update must succeed for {old_text:?}: {e}"));
    assert_equals_h0(&next, &post);
    (next, post)
}

/// Continuation readiness (T7): keep updating the state the previous update
/// returned, checking the oracle after every step.
fn run_chain(old_text: &str, edits: &[(usize, usize, &str)]) -> (ReadyDocument, Source) {
    let mut current_source = source(1, old_text);
    let mut current_state = full_build(&current_source, &mut NoopWorkSink).expect("initial build");
    for (step, &(start, end, inserted)) in edits.iter().enumerate() {
        let edit = CanonicalEdit::new(start, end, inserted).expect("edit geometry");
        edit.validate_against(&current_source)
            .expect("edit vs current source");
        let post = edit
            .apply(&current_source, SourceId(2 + step as u64))
            .expect("edit applies");
        current_state = update(
            current_state,
            &current_source,
            &post,
            &edit,
            &mut NoopWorkSink,
        )
        .unwrap_or_else(|e| panic!("chain step must succeed: {e}"));
        assert_equals_h0(&current_state, &post);
        current_source = post;
    }
    (current_state, current_source)
}

#[test]
fn paragraph_edits_pass_the_oracle() {
    const S: &str = "alpha\n\nbeta\n\ngamma\n";
    // insertion / BOF insertion / EOF append / deletion / replacement /
    // length-preserving replacement / boundary insertion.
    run_case(S, 3, 3, "X");
    run_case(S, 0, 0, "\n");
    run_case(S, 19, 19, "Z");
    run_case(S, 12, 13, "");
    run_case(S, 13, 18, "GAMMA");
    run_case(S, 13, 13, "X");
    run_case(S, 0, 5, "one");
    run_case(S, 7, 11, "");
    run_case(S, 5, 7, "\n\n\n");
}

#[test]
fn definition_edits_pass_the_oracle_on_both_branches() {
    const S: &str = "[a]: /x\n\np1\n\np2\n\np3\n";
    // facts equal -> local path
    run_case(S, 1, 2, "A");
    run_case(S, 14, 14, "X");
    // facts change -> same-target full build
    run_case(S, 13, 13, "[b]: /y\n\n");
    run_case(S, 5, 7, "/zz");
    run_case(S, 0, 8, "");
    run_case(S, 0, 0, "[b]: /y\n\n");
}

#[test]
fn block_container_edits_pass_the_oracle() {
    // heading / quote / list / fence blocks, each one Owner.
    run_case("# Title\n\nbody\n", 3, 3, "!");
    run_case("> quoted\n\nnext\n", 3, 3, "X");
    run_case("- a\n- b\n\nnext\n", 3, 3, "X");
    run_case("```rust\ncode\n```\n\nplain\n", 9, 9, "Z");
    run_case("```\n\n```\n\nplain\n", 5, 5, "Z");
    // A fence whose body contains blank-looking lines: the paragraph after
    // it is a separate Owner and the edit inside it stays local.
    run_case("```\ncode\n\nmore\n```\n\nq\n\np\n\nr\n", 15, 15, "Z");
    run_case("```\ncode\n\nmore\n```\n\nq\n\np\n\nr\n", 26, 26, "Z");
    // A fence that runs to EOF swallows the whole retained suffix.
    run_case("```\ncode\n```\n\nq\n\np\n", 13, 13, "Z");
}

#[test]
fn unicode_cjk_and_emoji_edits_pass_the_oracle() {
    // Byte coordinates stay byte coordinates: every boundary below is a
    // UTF-8 char boundary, and no coordinate system is conflated.
    run_case("中文 🐉\n\n第二段\n\nthird\n", 0, 0, "🐉");
    run_case("中文 🐉\n\n第二段\n\nthird\n", 3, 3, "x");
    run_case("中文 🐉\n\n第二段\n\nthird\n", 3, 6, "");
    run_case("中文 🐉\n\n第二段\n\nthird\n", 7, 11, "dragon");
    run_case("中文 🐉\n\n第二段\n\nthird\n", 16, 16, "🐉");
    run_case("中文 🐉\n\n第二段\n\nthird\n", 23, 23, "!");
}

#[test]
fn tiny_and_degenerate_documents_pass_the_oracle() {
    run_case("", 0, 0, "hello\n");
    run_case("hello\n", 0, 6, "");
    run_case("hello\n", 0, 6, "\n\n");
    run_case("\n\n", 0, 0, "hi");
    run_case("\n\n\n\n", 0, 0, "x\n");
    run_case("x\n", 0, 2, "\n\n\n\n");
    run_case("a", 0, 1, "b");
    run_case("   \n", 0, 0, "text");
    run_case("one line", 0, 0, "# ");
    run_case("one line", 8, 8, "\n");
}

#[test]
fn edits_that_merge_or_split_paragraphs_pass_the_oracle() {
    const S: &str = "a\n\nb\n\nc\n\nd\n";
    run_case(S, 2, 3, ""); // delete one blank line: "a" + "b" merge
    run_case(S, 3, 3, "\n"); // insert a blank line: "b" splits off
    run_case(S, 1, 5, "\n\n"); // collapse a two-block span into one boundary
    run_case(S, 0, 11, "one\n\ntwo\n");
    run_case(S, 2, 9, "\n");
}

#[test]
fn the_c3_restore_probe_recovers_the_original_document() {
    // C3: delete exactly the inserted bytes on the SAME returned state and
    // recover the clean pre-source result.
    let old_text = "[a]: /x\n\np1\n\np2\n\np3\n";
    let old_source = source(1, old_text);
    let old = full_build(&old_source, &mut NoopWorkSink).expect("initial build");

    let insertion = CanonicalEdit::new(3, 3, "xxxx").expect("edit");
    let middle = insertion
        .apply(&old_source, SourceId(2))
        .expect("first edit applies");
    let state1 =
        update(old, &old_source, &middle, &insertion, &mut NoopWorkSink).expect("insertion update");
    assert_equals_h0(&state1, &middle);

    let restore = CanonicalEdit::new(3, 7, "").expect("restore edit");
    restore.validate_against(&middle).expect("restore is valid");
    let restored = restore
        .apply(&middle, SourceId(3))
        .expect("restore applies");
    assert_eq!(restored.as_str(), old_text);
    let state2 =
        update(state1, &middle, &restored, &restore, &mut NoopWorkSink).expect("restore update");
    assert_equals_h0(&state2, &restored);
}

#[test]
fn a_chain_of_updates_keeps_the_returned_state_usable() {
    // Successive single-character insertions inside one paragraph, each
    // applied to the state the previous update returned.
    run_chain(
        "alpha\n\nbeta\n\ngamma\n",
        &[
            (3, 3, "1"),
            (4, 4, "2"),
            (5, 5, "3"),
            (6, 6, "4"),
            (7, 7, "5"),
            (8, 8, "6"),
        ],
    );
    // A chain that crosses block boundaries: split a paragraph, insert a
    // new block, then edit inside the newly inserted block.
    run_chain(
        "a\n\nb\n\nc\n",
        &[
            (3, 3, "\n"),
            (5, 5, "X\n\nY"),
            (10, 10, "Z"),
            (0, 0, "start\n\n"),
        ],
    );
    // A chain of definition edits (both branches interleaved).
    run_chain(
        "[a]: /x\n\np\n\nq\n",
        &[
            (1, 2, "A"),
            (9, 9, "X"),
            (0, 0, "[b]: /y\n\n"),
            (14, 14, "Y"),
        ],
    );
}

#[test]
fn independent_edits_from_the_same_state_agree_with_h0() {
    let base_text = "[a]: /x\n\nalpha\n\nbeta\n\ngamma\n";
    let base_source = source(1, base_text);
    let base = build(base_text);
    let cases: &[(usize, usize, &str)] = &[
        (0, 0, "\n"),
        (3, 3, "Q"),
        (10, 13, ""),
        (18, 18, "Z"),
        (9, 12, "ALPHA"),
        (28, 28, "\n\ntail\n"),
    ];
    for &(start, end, inserted) in cases {
        let edit = CanonicalEdit::new(start, end, inserted).expect("edit geometry");
        let post = edit.apply(&base_source, SourceId(2)).expect("edit applies");
        let next = update(base.clone(), &base_source, &post, &edit, &mut NoopWorkSink)
            .unwrap_or_else(|e| panic!("update must succeed: {e}"));
        assert_equals_h0(&next, &post);
    }
}

#[test]
fn a_long_edit_loop_stays_oracle_correct() {
    // A deterministic scripted loop against a larger document: every step
    // must return a state that is correct AND usable for the next step.
    let mut text = String::new();
    for i in 0..24 {
        text.push_str(&format!("paragraph {i}\n\n"));
    }
    let mut current_source = source(1, &text);
    let mut current_state = build(&text);

    // Rewrite the first paragraph's first character, then keep appending
    // inside the first paragraph, then append and delete a tail run.
    let scripted: &[(usize, usize, &str)] = &[
        (0, 1, "P"),
        (1, 1, "a"),
        (2, 2, "r"),
        (3, 3, "a"),
        (4, 4, "!"),
        (240, 240, "TAIL"),
        (244, 254, ""),
    ];
    for (step, &(start, end, inserted)) in scripted.iter().enumerate() {
        let edit = CanonicalEdit::new(start, end, inserted).expect("edit geometry");
        edit.validate_against(&current_source)
            .expect("edit vs current source");
        let post = edit
            .apply(&current_source, SourceId(2 + step as u64))
            .expect("edit applies");
        current_state = update(
            current_state,
            &current_source,
            &post,
            &edit,
            &mut NoopWorkSink,
        )
        .unwrap_or_else(|e| panic!("loop step must succeed: {e}"));
        assert_equals_h0(&current_state, &post);
        current_source = post;
    }
}
