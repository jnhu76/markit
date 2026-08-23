//! Incremental-vs-full-rebuild differential oracle and large-document
//! structural work tests (contract §12).
//!
//! - The randomized differential applies thousands of edits over
//!   Markdown-dense alphabets (fences, markers, emphasis, CJK, emoji,
//!   backslashes) and asserts after **every** edit that the incrementally
//!   updated state equals a fresh full parse in every observable except
//!   identity: kinds, source ranges, line spans, restart states,
//!   fingerprints, kind detail, inline IR, and version.
//! - The large-document tests assert **algorithmic work** (structural
//!   counters), never milliseconds: a local edit must scan a bounded
//!   number of lines regardless of document size, and a sparse
//!   two-location transaction must never rescan the middle.
//!
//! State-maintenance work is counted separately from parsing: an edit
//! that changes lengths honestly rewrites the survivor records after it
//! (`survivor_blocks_shifted` / `survivor_inline_nodes_shifted`) and may
//! relocate records when an island changes the block count
//! (`block_records_moved`) — the same accepted cost class as the P0-01
//! line index's suffix shift (ADR-003 Notes). What must stay local is
//! *parsing*, and an equal-length edit must touch no survivor at all.

use markit_core::markdown::MarkdownState;
use markit_core::{ByteOffset, Document, EditTransaction, LineNumber, SourceRange, TextEdit};

/// SplitMix64 — deterministic, dependency-free randomness (same pattern
/// as the P0-01 batteries).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E3779B97F4A7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Markdown-dense alphabet: every block opener, every inline delimiter,
/// multi-byte scalars, and plain text.
const TOKENS: [&str; 16] = [
    "a", "x", "#", " ", "-", ">", "`", "*", "_", "[", "]", "(", ")", "\\", "中", "🙂",
];

fn random_tokens(rng: &mut Rng, max_len: usize) -> String {
    (0..rng.below(max_len + 1))
        .map(|_| TOKENS[rng.below(TOKENS.len())])
        .collect()
}

fn char_boundaries(text: &str) -> Vec<usize> {
    text.char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect()
}

/// Full-observable equality except identity (the oracle's rule: ids are
/// pairing decisions, not structure).
fn assert_matches_rebuild(doc: &Document, state: &MarkdownState, context: &str) {
    let rebuilt = MarkdownState::build(&doc.snapshot());
    assert_eq!(state.version(), rebuilt.version(), "{context}: version");
    assert_eq!(
        state.block_count(),
        rebuilt.block_count(),
        "{context}: block count ({})\ntext: {:?}",
        state
            .blocks()
            .iter()
            .map(|b| b.kind.name())
            .collect::<Vec<_>>()
            .join(","),
        doc_text(doc)
    );
    for (i, (incremental, fresh)) in state.blocks().iter().zip(rebuilt.blocks()).enumerate() {
        assert_eq!(incremental.kind, fresh.kind, "{context}: block {i} kind");
        assert_eq!(
            incremental.source_range, fresh.source_range,
            "{context}: block {i} range"
        );
        assert_eq!(
            incremental.line_span, fresh.line_span,
            "{context}: block {i} lines"
        );
        assert_eq!(
            incremental.state_before, fresh.state_before,
            "{context}: block {i} state_before"
        );
        assert_eq!(
            incremental.state_after, fresh.state_after,
            "{context}: block {i} state_after"
        );
        assert_eq!(
            incremental.fingerprint, fresh.fingerprint,
            "{context}: block {i} fingerprint"
        );
        assert_eq!(
            incremental.detail, fresh.detail,
            "{context}: block {i} detail"
        );
        assert_eq!(
            incremental.inline, fresh.inline,
            "{context}: block {i} inline IR"
        );
    }
}

fn doc_text(doc: &Document) -> String {
    let len = doc.snapshot().len_bytes();
    doc.slice(SourceRange::new(ByteOffset(0), ByteOffset(len)))
        .into_owned()
}

fn assert_public_tiling(state: &MarkdownState, len: usize, context: &str) {
    let mut expected = 0usize;
    for (i, b) in state.blocks().iter().enumerate() {
        assert_eq!(
            b.source_range.start.as_usize(),
            expected,
            "{context}: block {i} start"
        );
        expected = b.source_range.end.as_usize();
    }
    assert_eq!(expected, len, "{context}: stream must end at EOF");
}

// ---------------------------------------------------------------------------
// Oracle A: randomized differential
// ---------------------------------------------------------------------------

fn differential_run(seed: u64, ops: usize, initial_len: usize) {
    let mut rng = Rng::new(seed);
    let mut text = random_tokens(&mut rng, initial_len);
    let mut doc = Document::new(&text);
    let mut state = MarkdownState::build(&doc.snapshot());

    for op in 0..ops {
        let bounds = char_boundaries(&text);
        let start_idx = rng.below(bounds.len());
        let end_idx = start_idx + rng.below(bounds.len() - start_idx);
        let (start, end) = (bounds[start_idx], bounds[end_idx]);
        let mut replacement = random_tokens(&mut rng, 4);
        if replacement.is_empty() && start == end {
            replacement = "x".to_string(); // never a no-op
        }

        let applied = EditTransaction::typing()
            .with_edit(TextEdit::replace(
                SourceRange::new(ByteOffset(start), ByteOffset(end)),
                replacement.clone(),
            ))
            .apply(&mut doc)
            .expect("random edit applies");
        state
            .update(&doc.snapshot(), &applied.result)
            .unwrap_or_else(|e| panic!("seed {seed} op {op}: update rejected: {e}"));

        text = format!("{}{}{}", &text[..start], replacement, &text[end..]);
        let context = format!("seed {seed} op {op}");
        assert_matches_rebuild(&doc, &state, &context);
        assert_public_tiling(&state, text.len(), &context);
        assert!(
            state.last_work().lines_scanned <= doc.line_count() as u64,
            "{context}: scanning cannot exceed the document"
        );
    }
}

#[test]
fn randomized_differential_small_documents() {
    for seed in 1..=8u64 {
        differential_run(seed, 300, 40);
    }
}

#[test]
fn randomized_differential_medium_documents() {
    for seed in 100..=101u64 {
        differential_run(seed, 400, 2000);
    }
}

#[test]
fn randomized_differential_multi_edit_transactions() {
    // Sparse transactions: two distant edits applied atomically must
    // stay two islands and stay differential-correct.
    for seed in 200..=203u64 {
        let mut rng = Rng::new(seed);
        let mut text = random_tokens(&mut rng, 800);
        let mut doc = Document::new(&text);
        let mut state = MarkdownState::build(&doc.snapshot());

        for op in 0..120 {
            let bounds = char_boundaries(&text);
            let a_idx = rng.below(bounds.len() / 2);
            let b_idx = bounds.len() / 2 + rng.below(bounds.len() - bounds.len() / 2);
            let a = bounds[a_idx];
            let a_end = bounds[a_idx + rng.below(3).min(bounds.len() - 1 - a_idx)];
            let b = bounds[b_idx];
            let b_end = bounds[b_idx + rng.below(3).min(bounds.len() - 1 - b_idx)];
            let mut text_a = random_tokens(&mut rng, 3);
            let mut text_b = random_tokens(&mut rng, 3);
            if text_a.is_empty() && a == a_end {
                text_a = "x".to_string();
            }
            if text_b.is_empty() && b == b_end {
                text_b = "y".to_string();
            }

            let applied = EditTransaction::typing()
                .with_edit(TextEdit::replace(
                    SourceRange::new(ByteOffset(a), ByteOffset(a_end)),
                    text_a.clone(),
                ))
                .with_edit(TextEdit::replace(
                    SourceRange::new(ByteOffset(b), ByteOffset(b_end)),
                    text_b.clone(),
                ))
                .apply(&mut doc)
                .expect("transaction applies");
            state.update(&doc.snapshot(), &applied.result).unwrap();

            text = format!(
                "{}{}{}{}{}",
                &text[..a],
                text_a,
                &text[a_end..b],
                text_b,
                &text[b_end..]
            );
            let context = format!("sparse seed {seed} op {op}");
            assert_matches_rebuild(&doc, &state, &context);
            assert_public_tiling(&state, text.len(), &context);
        }
    }
}

// ---------------------------------------------------------------------------
// Controlled scaling: large documents, structural assertions
// ---------------------------------------------------------------------------

fn paragraph_family(lines: usize) -> String {
    (0..lines / 2)
        .map(|i| format!("paragraph {i}\n\n"))
        .collect()
}

fn heading_family(lines: usize) -> String {
    (0..lines / 2)
        .map(|i| format!("# heading {i}\n\n"))
        .collect()
}

fn list_family(lines: usize) -> String {
    // Three-item lists separated by blank lines: each block is bounded.
    (0..lines / 4)
        .flat_map(|g| (0..3).map(move |i| format!("- group {g} item {i}\n")))
        .chain(std::iter::once(String::new()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn inline_family(lines: usize) -> String {
    (0..lines / 2)
        .map(|i| format!("plain *em{i}* `code{i}` [link{i}](u{i}) tail\n\n"))
        .collect()
}

fn fence_family(lines: usize) -> String {
    // One fenced block per 4 lines: opener, two content lines, closer.
    (0..lines / 4)
        .map(|i| format!("```\ncontent {i} a\ncontent {i} b\n```\n"))
        .collect()
}

fn offset_of_line(text: &str, line: usize) -> usize {
    text.split('\n').take(line).map(|l| l.len() + 1).sum()
}

type FamilyFn = fn(usize) -> String;

fn apply_insert(
    doc: &mut Document,
    state: &mut MarkdownState,
    at: usize,
    what: &str,
) -> markit_core::markdown::MarkdownWork {
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(at), what))
        .apply(doc)
        .expect("edit applies");
    state
        .update(&doc.snapshot(), &applied.result)
        .expect("update");
    state.last_work()
}

/// A local edit on a huge document parses a bounded number of lines,
/// independent of document size (INV: work ~= Δ region + bounded
/// overhead; `blocks_reparsed == 1` is the expected case, not a law).
#[test]
fn local_edits_scan_constant_lines_across_sizes() {
    let families: [(&str, FamilyFn); 5] = [
        ("paragraphs", paragraph_family),
        ("headings", heading_family),
        ("lists", list_family),
        ("inline", inline_family),
        ("fences", fence_family),
    ];
    // 10k and 100k keep the matrix fast; the 1M case rides the sparse
    // regression below.
    for size in [10_000usize, 100_000] {
        for (name, family) in families {
            let text = family(size);
            let mut doc = Document::new(&text);
            let mut state = MarkdownState::build(&doc.snapshot());

            // Edit roughly the middle block's first line — for the
            // fence family that is the *content* line after the opener,
            // so the edit stays inside the block (editing an opener
            // line inverts open/close parity and honestly rescans to
            // EOF, covered by fence_delimiter_edit_propagates_to_eof).
            let mid_line = match name {
                "fences" => doc.line_count() / 2 + 1,
                _ => doc.line_count() / 2,
            };
            let line_len = doc.line_str(LineNumber(mid_line)).len();
            let at = offset_of_line(&text, mid_line) + line_len / 2;
            let work = apply_insert(&mut doc, &mut state, at, "X");

            assert_eq!(work.dirty_regions, 1, "{name}/{size}: one island");
            assert!(
                work.lines_scanned <= 6,
                "{name}/{size}: local edit scanned {} lines",
                work.lines_scanned
            );
            assert!(
                work.blocks_reparsed <= 2,
                "{name}/{size}: reparsed {} blocks",
                work.blocks_reparsed
            );
            assert!(
                work.convergence_line < doc.line_count() as u64,
                "{name}/{size}: converged before EOF"
            );
            assert_matches_rebuild(&doc, &state, &format!("{name}/{size}"));
        }
    }
}

/// The permanent sparse regression: 1M lines, edits at line 10 and line
/// 999990 — the middle million lines are never parsed.
#[test]
fn sparse_two_location_edit_on_one_million_lines() {
    let text = paragraph_family(1_000_000);
    let mut doc = Document::new(&text);
    let mut state = MarkdownState::build(&doc.snapshot());
    assert!(state.block_count() > 900_000);

    let id_far = state.blocks().iter().rev().nth(10).unwrap().id;
    let id_middle = state.blocks()[state.block_count() / 2].id;

    let at_a = offset_of_line(&text, 10) + 4;
    let at_b = offset_of_line(&text, 999_990) + 4;
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(at_a), "X"))
        .with_edit(TextEdit::insert(ByteOffset(at_b), "Y"))
        .apply(&mut doc)
        .expect("sparse transaction applies");
    state
        .update(&doc.snapshot(), &applied.result)
        .expect("update");

    let work = state.last_work();
    assert_eq!(
        work.dirty_regions, 2,
        "two semantic islands, never one span"
    );
    assert!(
        work.lines_scanned <= 12,
        "parsed {} lines for two one-character edits",
        work.lines_scanned
    );
    assert_eq!(work.blocks_reparsed, 2, "two paragraphs reparsed");
    assert_eq!(work.blocks_reused, 2);
    assert_eq!(work.blocks_created, 0);
    assert!(
        work.convergence_line < doc.line_count() as u64,
        "both islands converged before EOF"
    );
    // Honest state-maintenance accounting: inserting bytes rewrites the
    // survivor records after the first island (roughly everything up to
    // the second one), each island keeps the block count so no record is
    // physically relocated. Parsing stayed local; bookkeeping is
    // reported, not hidden.
    assert!(
        work.survivor_blocks_shifted > 400_000,
        "insertions shift the tail in place: {} survivors rewritten",
        work.survivor_blocks_shifted
    );
    assert_eq!(
        work.block_records_moved, 0,
        "equal-count islands splice nothing"
    );
    // The middle and the far end kept their identity.
    assert_eq!(
        state.block_by_id(id_middle).map(|b| b.kind),
        Some(markit_core::BlockKind::Paragraph),
        "middle block untouched"
    );
    assert!(state.block_by_id(id_far).is_some(), "far end untouched");
    // And the stream is still exactly what a full rebuild produces.
    assert_matches_rebuild(&doc, &state, "1m-sparse");
}

/// An equal-length local edit (no byte/line delta) rebinds one block
/// and touches no survivor record and moves nothing, at any document
/// size — the "did the editor secretly rewrite the index" regression.
#[test]
fn equal_length_local_edit_touches_no_survivors() {
    for size in [10_000usize, 100_000] {
        let text = inline_family(size);
        let mut doc = Document::new(&text);
        let mut state = MarkdownState::build(&doc.snapshot());

        let mid_line = doc.line_count() / 2;
        let line_text = doc.line_str(LineNumber(mid_line)).into_owned();
        let at = offset_of_line(&text, mid_line) + line_text.len() / 2;
        // Replace one character with another: zero byte delta, zero
        // line delta.
        let applied = EditTransaction::typing()
            .with_edit(TextEdit::replace(
                SourceRange::new(ByteOffset(at), ByteOffset(at + 1)),
                "Z",
            ))
            .apply(&mut doc)
            .expect("replace applies");
        state
            .update(&doc.snapshot(), &applied.result)
            .expect("update");

        let work = state.last_work();
        assert_eq!(work.dirty_regions, 1, "{size}: one island");
        assert!(
            work.lines_scanned <= 6,
            "{size}: scanned {} lines",
            work.lines_scanned
        );
        assert!(
            work.blocks_reparsed <= 2,
            "{size}: reparsed {} blocks",
            work.blocks_reparsed
        );
        assert_eq!(
            work.survivor_blocks_shifted, 0,
            "{size}: equal-length edits shift no survivor"
        );
        assert_eq!(
            work.block_records_moved, 0,
            "{size}: equal-length edits move no record"
        );
        assert!(
            work.survivor_inline_nodes_shifted == 0,
            "{size}: no inline IR rewritten"
        );
        assert_matches_rebuild(&doc, &state, &format!("equal-length/{size}"));
    }
}

/// Fence delimiter edits propagate honestly: closing-fence deletion
/// rescans to end of document, and the counters say so.
#[test]
fn fence_delimiter_edit_propagates_to_eof() {
    for size in [10_000usize, 100_000] {
        let text = fence_family(size);
        let mut doc = Document::new(&text);
        let mut state = MarkdownState::build(&doc.snapshot());

        // Delete the FIRST fence's closing delimiter: the fence now
        // swallows everything to EOF (contract §6.7, issue #12).
        let closer_line = 3; // opener, content, content, closer
        let closer_at = offset_of_line(&text, closer_line);
        let applied = EditTransaction::typing()
            .with_edit(TextEdit::delete(SourceRange::new(
                ByteOffset(closer_at),
                ByteOffset(closer_at + 3),
            )))
            .apply(&mut doc)
            .expect("delete applies");
        state
            .update(&doc.snapshot(), &applied.result)
            .expect("update");

        let work = state.last_work();
        assert_eq!(work.dirty_regions, 1);
        assert!(
            work.lines_scanned >= doc.line_count() as u64 / 2,
            "{size}: fence propagation must rescan onward, got {}",
            work.lines_scanned
        );
        assert_eq!(work.convergence_line, doc.line_count() as u64);
        assert_matches_rebuild(&doc, &state, &format!("fence-delim/{size}"));
    }
}

/// Fence CONTENT edits stay local: the fence block is reparsed (restart
/// at its opener), nothing beyond the fence is touched.
#[test]
fn fence_content_edit_stays_local() {
    let text = fence_family(100_000);
    let mut doc = Document::new(&text);
    let mut state = MarkdownState::build(&doc.snapshot());
    let content_at = offset_of_line(&text, 1) + 3; // inside first content line
    let work = apply_insert(&mut doc, &mut state, content_at, "X");
    assert_eq!(work.dirty_regions, 1);
    assert!(work.lines_scanned <= 5, "fence block lines only");
    assert_eq!(work.blocks_reparsed, 1);
    assert_matches_rebuild(&doc, &state, "fence-content");
}

/// Whole-document replacement keeps ids where the structure matches
/// (pairing sees the entire document as one island).
#[test]
fn replace_document_pairs_identical_structure() {
    let text = paragraph_family(2_000);
    let mut doc = Document::new(&text);
    let mut state = MarkdownState::build(&doc.snapshot());
    let ids_before: Vec<_> = state.blocks().iter().map(|b| b.id).collect();

    let result = doc.replace_all(&text); // same text, new revision
    state
        .update(&doc.snapshot(), &result)
        .expect("reload update");
    let ids_after: Vec<_> = state.blocks().iter().map(|b| b.id).collect();
    assert_eq!(ids_before, ids_after, "identical reload keeps every id");
    assert_matches_rebuild(&doc, &state, "replace-all");

    // A structurally different document keeps only what pairs: the
    // blank tail pairs, the headings are all newcomers.
    let other = "# different\n\n".repeat(50);
    let result = doc.replace_all(&other);
    state
        .update(&doc.snapshot(), &result)
        .expect("replace update");
    assert_matches_rebuild(&doc, &state, "replace-different");
    let work = state.last_work();
    assert!(work.blocks_reused >= 1, "blanks still pair");
    assert!(work.blocks_created >= 1, "headings are new");
}
