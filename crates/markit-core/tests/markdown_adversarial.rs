//! Adversarial inline-input regressions (contract §11 D15/D16, §12).
//!
//! Characterization is **structural, not wall-clock**: node/depth counts
//! and the `inline_bytes_scanned` counter must stay within a constant
//! of the run length, and doubling the input must not change that.
//! These inputs once made the parser quadratic or exponentially
//! redundant (per-closer opener rescans, per-bracket suffix scans,
//! nested link-text re-parsing, unbounded tree depth); the bounds exist
//! so they stay linear — and so a regression to the old shapes fails
//! here instead of in a user's paragraph.

use markit_core::markdown::MarkdownState;
use markit_core::{ByteOffset, Document, EditTransaction, InlineNode, TextEdit};

/// Scanning steps of one full build over a single-paragraph document.
fn build_scanned(text: &str) -> (MarkdownState, u64) {
    let doc = Document::new(text);
    let state = MarkdownState::build(&doc.snapshot());
    let scanned = state.last_work().inline_bytes_scanned;
    (state, scanned)
}

fn first_run_nodes(state: &MarkdownState) -> &[InlineNode] {
    let block = state.blocks().next().expect("one paragraph");
    &state
        .inline_ir(block.id())
        .expect("paragraph has inline IR")
        .runs[0]
        .nodes
}

fn max_depth(nodes: &[InlineNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            InlineNode::Emphasis { children, .. }
            | InlineNode::Strong { children, .. }
            | InlineNode::Link { children, .. } => 1 + max_depth(children),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn many_unmatched_brackets_scan_linearly() {
    // No `]` anywhere: the first attempt scans the suffix once, the
    // bracket watermark makes every later `[` free. The naive parser
    // rescanned the suffix per bracket (O(n²)).
    let (n, scanned_n) = {
        let text = "[".repeat(20_000);
        let (_, scanned) = build_scanned(&text);
        (text.len(), scanned)
    };
    let (m, scanned_m) = {
        let text = "[".repeat(40_000);
        let (_, scanned) = build_scanned(&text);
        (text.len(), scanned)
    };
    assert!(scanned_n <= 4 * n as u64, "n: {scanned_n} for {n} bytes");
    assert!(scanned_m <= 4 * m as u64, "m: {scanned_m} for {m} bytes");
    assert!(
        scanned_m <= 3 * scanned_n,
        "doubling the input must not triple the work: {scanned_n} -> {scanned_m}"
    );
}

#[test]
fn closer_heavy_delimiter_runs_pair_linearly() {
    // Every `*` is a closer with no opener anywhere: pairing must not
    // rescan earlier delimiters per closer (the old backward search
    // was quadratic here).
    let make = |n: usize| format!("{}\n", "a* ".repeat(n));
    let small = make(5_000);
    let big = make(20_000);
    let (_, scanned_small) = build_scanned(&small);
    let (state, scanned_big) = build_scanned(&big);
    assert!(
        scanned_big <= 4 * big.len() as u64,
        "{} bytes scanned {scanned_big}",
        big.len()
    );
    // 4x input, proportionally bounded work; everything stays literal.
    assert!(scanned_big <= 4 * scanned_small + 64);
    assert_eq!(first_run_nodes(&state).len(), 1, "all literal text");
}

#[test]
fn deep_emphasis_nesting_is_capped_and_small() {
    // 2n stars around one character nest ~n/2 levels deep. The cap
    // (D15) keeps the assembled tree at bounded depth with bounded
    // node count regardless of n; delimiters beyond the cap are
    // literal text. Without it, drop/shift/equality on the tree would
    // recurse as deep as the input.
    for n in [2_000usize, 20_000, 200_000] {
        let text = format!("{}x{}\n", "*".repeat(n), "*".repeat(n));
        let (state, scanned) = build_scanned(&text);
        let nodes = first_run_nodes(&state);
        assert_eq!(max_depth(nodes), 256, "n={n}: depth hits the cap exactly");
        assert!(
            scanned <= 4 * text.len() as u64,
            "n={n}: {scanned} for {} bytes",
            text.len()
        );
    }
}

#[test]
fn deeply_nested_link_attempts_are_budgeted() {
    // Each wrapper's link text contains a link, so every outer attempt
    // fails after parsing its inner text — the textbook exponential
    // re-parse. The work budget (D16) caps total scanning at a constant
    // multiple of the run length; degradation is total but
    // deterministic (incremental and rebuild agree, pinned below).
    let mut text = String::from("a");
    for _ in 0..4_000 {
        text = format!("[{text}](u)");
    }
    let (state, scanned) = build_scanned(&text);
    // ~2 × budget (64·len + 4096) plus the overshoot of in-flight
    // charges that straddle exhaustion.
    assert!(
        scanned <= 150 * text.len() as u64 + 16_384,
        "len {}: scanned {scanned}",
        text.len()
    );
    assert!(max_depth(first_run_nodes(&state)) <= 256);

    // A single character edit inside the adversarial paragraph still
    // keeps incremental == rebuild (the bounds are deterministic
    // functions of the run text).
    let mut doc = Document::new(&text);
    let mut doc_state = MarkdownState::build(&doc.snapshot());
    let applied = EditTransaction::typing()
        .with_edit(TextEdit::insert(ByteOffset(text.len() / 2), "X"))
        .apply(&mut doc)
        .expect("edit applies");
    doc_state
        .update(&doc.snapshot(), &applied.result)
        .expect("update");
    let rebuilt = MarkdownState::build(&doc.snapshot());
    assert_eq!(doc_state.block_count(), rebuilt.block_count());
    let a = doc_state.blocks().next().unwrap();
    let b = rebuilt.blocks().next().unwrap();
    assert_eq!(a.kind(), b.kind());
    assert_eq!(a.source_range(), b.source_range());
    assert_eq!(
        doc_state.inline_ir(a.id()).unwrap().runs[0].nodes,
        rebuilt.inline_ir(b.id()).unwrap().runs[0].nodes,
        "bounded degradation is deterministic"
    );
}

#[test]
fn very_long_paragraphs_scan_linearly() {
    // Mixed CJK/emoji/inline content at paragraph scale.
    let unit = "段落 *强调* `代码` [链接](u) 🙂 ";
    for reps in [200usize, 2_000] {
        let text = format!("{}\n", unit.repeat(reps));
        let (_, scanned) = build_scanned(&text);
        assert!(
            scanned <= 2 * text.len() as u64 + 16,
            "reps={reps}: {scanned} for {} bytes",
            text.len()
        );
    }
}

#[test]
fn unmatched_backtick_runs_scan_linearly() {
    // Adversarial input: backtick runs of increasing length with no
    // matching closer. The old find_backtick_string rescanned the suffix
    // for each opener (O(n²) total). BacktickIndex pre-scans all runs
    // once, so each lookup is O(log n) and total work is O(n).
    //
    // Build: `x ``x ```x ````x ... up to k runs.
    // Text length is O(k²), so we double k and check work stays
    // proportional to text length (not to k² or worse).
    fn build_adversarial(k: usize) -> String {
        let mut text = String::new();
        for i in 1..=k {
            text.push_str(&"`".repeat(i));
            text.push('x');
        }
        text.push('\n');
        text
    }

    let (small_len, scanned_small) = {
        let text = build_adversarial(200);
        let (_, scanned) = build_scanned(&text);
        (text.len(), scanned)
    };
    let (big_len, scanned_big) = {
        let text = build_adversarial(800);
        let (_, scanned) = build_scanned(&text);
        (text.len(), scanned)
    };
    // Work must be proportional to text length (not k²).
    // 800/200 = 4x k, but text ratio is ~16x (O(k²)).
    // If work were quadratic in text length, big would be ~256x small.
    // Linear means big ≈ 16x small (matching the text size ratio).
    let text_ratio = big_len as f64 / small_len as f64;
    let work_ratio = scanned_big as f64 / scanned_small as f64;
    assert!(
        work_ratio <= text_ratio * 2.0,
        "work must scale with text length: text_ratio={text_ratio:.1}x, work_ratio={work_ratio:.1}x (small={scanned_small}/{small_len}, big={scanned_big}/{big_len})"
    );
    // Absolute bound: scanned <= 2 * text_len + overhead.
    assert!(
        scanned_big <= 2 * big_len as u64 + 256,
        "big: {scanned_big} for {big_len} bytes"
    );
}
