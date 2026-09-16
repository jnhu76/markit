//! Scenario runner: one (document, mutation) pair measured against the
//! current markit-core `MarkdownState` implementation.
//!
//! Per scenario:
//! - `full_us`  — median wall-clock of `MarkdownState::build` on the
//!   post-edit snapshot (the M0 control: what a full parse would cost);
//! - `inc_us`   — median wall-clock of `MarkdownState::update`;
//! - `MarkdownWork` counters (the implementation's own honest structural
//!   accounting: bytes/lines scanned, blocks reparsed, survivors shifted,
//!   records moved, convergence point);
//! - allocation count/bytes of both paths (counting global allocator);
//! - projection delta — the FakeProjectionConsumer of issue #19 §12,
//!   split by the run-1.1 influence schema into P_semantic (content
//!   changed → re-project) vs P_coordinate (pure offset movement).
//!   Comparison is by block KIND + raw SOURCE BYTES, never by parsed
//!   detail/inline IR. The prefix merge demands identical absolute
//!   ranges; the suffix merge is shift-tolerant (equal distance-to-EOF).
//! - influence vector (influence.rs) — derived ONLY from the observed
//!   counters of this run;
//! - oracle A (SELF_EQUIVALENCE) — incremental state == clean full
//!   rebuild on every observable field;
//! - oracle C (LOSSLESSNESS) — block ranges tile the source, every
//!   unclaimed gap is whitespace-only, all ranges are char boundaries.

use markit_core::markdown::{BlockKind, MarkdownState, MarkdownWork};
use markit_core::{ByteOffset, Document, EditTransaction, SourceRange, TextEdit};

use crate::alloc::{timed, AllocDelta};
use crate::influence::{self, InfluenceVector};
use crate::mutate::Case;
use crate::text::SurveyText;

#[derive(Clone, Copy, Debug)]
struct BlockSig {
    kind: BlockKind,
    start: usize,
    end: usize,
    line_start: usize,
    line_end: usize,
}

fn capture(state: &MarkdownState) -> Vec<BlockSig> {
    state
        .blocks()
        .map(|v| {
            let r = v.source_range();
            let ls = v.line_span();
            BlockSig {
                kind: v.kind(),
                start: r.start.as_usize(),
                end: r.end.as_usize(),
                line_start: ls.start.as_usize(),
                line_end: ls.end.as_usize(),
            }
        })
        .collect()
}

fn covering<'a>(blocks: &'a [BlockSig], at: usize) -> &'a BlockSig {
    blocks
        .iter()
        .find(|b| b.start <= at && at < b.end)
        .unwrap_or_else(|| blocks.last().expect("nonempty state"))
}

/// Stable prefix: identical absolute ranges + identical content.
/// Stable suffix: identical content at identical distance from EOF.
/// Returns (prefix-aligned, suffix-aligned, invalidated blocks, bytes).
/// Everything between is what a position-keyed renderer must re-project
/// (P_semantic); the suffix-aligned survivors are P_coordinate.
fn projection_delta(
    old: &[BlockSig],
    old_text: &str,
    new: &[BlockSig],
    new_text: &str,
) -> (usize, usize, u64, u64) {
    let sig_eq = |a: &BlockSig, b: &BlockSig| {
        a.kind == b.kind && old_text.get(a.start..a.end) == new_text.get(b.start..b.end)
    };
    let mut p = 0;
    while p < old.len()
        && p < new.len()
        && old[p].start == new[p].start
        && old[p].end == new[p].end
        && sig_eq(&old[p], &new[p])
    {
        p += 1;
    }
    let mut s = 0;
    while s < old.len() - p && s < new.len() - p {
        let o = &old[old.len() - 1 - s];
        let n = &new[new.len() - 1 - s];
        let o_dist = old_text.len() - o.end;
        let n_dist = new_text.len() - n.end;
        if o_dist == n_dist && sig_eq(o, n) {
            s += 1;
        } else {
            break;
        }
    }
    let inv = &new[p..new.len() - s];
    (
        p,
        s,
        inv.len() as u64,
        inv.iter().map(|b| (b.end - b.start) as u64).sum(),
    )
}

/// ORACLE-C (LOSSLESSNESS): block ranges must ascend without overlap,
/// sit on char boundaries, and leave no gap containing non-whitespace
/// bytes. Returns the total unclaimed gap bytes and an error on any
/// violation. Lossless here means "the representation accounts for every
/// source byte": reconstruction is the original because ranges point
/// into it, and no content can hide outside a block.
fn oracle_lossless(state: &MarkdownState, text: &str) -> Result<u64, String> {
    let mut cursor = 0usize;
    let mut gap = 0u64;
    for v in state.blocks() {
        let r = v.source_range();
        let s = r.start.as_usize();
        let e = r.end.as_usize();
        if e < s {
            return Err(format!("inverted range {s}..{e}"));
        }
        if text.get(s..e).is_none() {
            return Err(format!("range {s}..{e} not on char boundaries"));
        }
        if s < cursor {
            return Err(format!("overlap at {s} (cursor {cursor})"));
        }
        gap += (s - cursor) as u64;
        if !text[cursor..s].bytes().all(is_ws_byte) {
            return Err(format!(
                "non-whitespace gap {cursor}..{s}: {:?}",
                text.get(cursor..s).map(|g| g.chars().take(24).collect::<String>())
            ));
        }
        cursor = e;
    }
    gap += (text.len() - cursor) as u64;
    if !text[cursor..].bytes().all(is_ws_byte) {
        return Err(format!("non-whitespace tail gap at {cursor}"));
    }
    Ok(gap)
}

fn is_ws_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

#[derive(Clone, Debug)]
pub struct Row {
    pub case_id: String,
    pub family: String,
    pub construct: String,
    pub predicted: String,
    pub corpus: String,
    pub position: String,
    pub doc_bytes: u64,
    pub doc_lines: u64,
    pub doc_blocks: u64,
    pub changed_bytes: u64,
    pub changed_lines: u64,
    pub full_us: f64,
    pub inc_us: f64,
    pub bytes_scanned: u64,
    pub lines_scanned: u64,
    pub blocks_reparsed: u64,
    pub blocks_reused: u64,
    pub blocks_created: u64,
    pub blocks_removed: u64,
    pub blocks_examined: u64,
    pub dirty_regions: u64,
    pub restart_line: u64,
    pub convergence_line: u64,
    pub inline_blocks_reparsed: u64,
    pub inline_bytes_scanned: u64,
    pub survivor_blocks_shifted: u64,
    pub survivor_inline_nodes_shifted: u64,
    pub block_records_moved: u64,
    pub proj_blocks_invalidated: u64,
    pub proj_bytes_invalidated: u64,
    pub alloc_count_inc: u64,
    pub alloc_bytes_inc: u64,
    pub alloc_count_full: u64,
    pub alloc_bytes_full: u64,
    pub oracle_ok: bool,
    // --- run-1.1 additions (observation schema) ---
    pub edit_start: u64,
    pub edit_end: u64,
    pub cover_old_kind: String,
    pub cover_old_lines: u64,
    pub cover_new_kind: String,
    pub prefix_aligned: u64,
    pub suffix_aligned: u64,
    pub gap_bytes_unclaimed: u64,
    pub lossless_ok: bool,
    pub syntax_class: &'static str,
    pub semantic_class: &'static str,
    pub representation_class: &'static str,
    pub provider: &'static str,
    pub note: String,
}

fn median_f64(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn median_alloc(v: &mut Vec<AllocDelta>) -> AllocDelta {
    v.sort_by_key(|d| d.count);
    v[v.len() / 2]
}

pub fn position_name(frac: f64) -> &'static str {
    match frac {
        f if f == 0.0 => "bof",
        f if f == 0.25 => "q1",
        f if f == 0.5 => "mid",
        f if f == 0.75 => "q3",
        f if f == 1.0 => "eof",
        _ => "target",
    }
}

/// Owned data captured on the final measured iteration. Snapshots borrow
/// the document, so nothing borrowed may cross iterations.
struct Finals {
    work: MarkdownWork,
    changed_bytes: u64,
    changed_lines: u64,
    doc_bytes: u64,
    doc_lines: u64,
    doc_blocks: u64,
    oracle_ok: bool,
    lossless_ok: bool,
    gap_bytes: u64,
    proj_blocks: u64,
    proj_bytes: u64,
    prefix_aligned: u64,
    suffix_aligned: u64,
    new_pbs: Vec<BlockSig>,
}

/// Runs one scenario to completion. `Err(reason)` means the scenario was
/// skipped (anchor not found, edit rejected) — a harness issue, printed
/// by the driver and never silently dropped.
pub fn run_case(
    corpus: &str,
    frac: f64,
    case: &Case,
    doc_text: &str,
    warm: usize,
    iters: usize,
) -> Result<Row, String> {
    let survey = SurveyText::new(doc_text.to_string());
    let edit: TextEdit = (case.build)(&survey, frac)
        .ok_or_else(|| format!("no anchor for {} at frac {frac}", case.id))?;
    let edit_start = edit.range.start.as_usize();
    let edit_end = edit.range.end.as_usize();
    // Old-state lines the edit starts and ends on.
    let edit_start_line = influence::line_of_byte(&survey.lines, edit_start);
    let edit_end_line = influence::line_of_byte(&survey.lines, edit_end.max(edit_start));
    // Provider candidate: edit point on a mermaid fence opener or body.
    let mermaid_candidate = match survey.roles.get(edit_start_line) {
        Some(crate::text::Role::FenceBody { mermaid: true }) => true,
        Some(crate::text::Role::FenceOpen { info: (s, e) }) => survey
            .text
            .get(*s..*e)
            .map(|i| i.trim() == "mermaid")
            .unwrap_or(false),
        _ => false,
    };

    // Pre-edit reference state (outside all timing).
    let old_doc = Document::new(doc_text.to_string());
    let old_pbs = capture(&MarkdownState::build(&old_doc.snapshot()));
    drop(old_doc);

    let mut inc_us: Vec<f64> = Vec::with_capacity(iters);
    let mut full_us: Vec<f64> = Vec::with_capacity(iters);
    let mut inc_alloc: Vec<AllocDelta> = Vec::with_capacity(iters);
    let mut full_alloc: Vec<AllocDelta> = Vec::with_capacity(iters);
    let mut finals: Option<Finals> = None;

    let last_it = warm + iters - 1;
    for it in 0..=last_it {
        let mut doc = Document::new(doc_text.to_string());
        let snap0 = doc.snapshot();
        let mut state = MarkdownState::build(&snap0);

        let tx = if case.paste {
            EditTransaction::paste()
        } else {
            EditTransaction::typing()
        };
        let applied = tx
            .with_edit(edit.clone())
            .apply(&mut doc)
            .map_err(|e| format!("apply failed for {}: {e}", case.id))?;
        let snap1 = doc.snapshot();

        let (res, us, al) = timed(|| state.update(&snap1, &applied.result));
        res.map_err(|e| format!("update failed for {}: {e:?}", case.id))?;
        let (fresh, fus, fal) = timed(|| MarkdownState::build(&snap1));
        if it >= warm {
            inc_us.push(us);
            full_us.push(fus);
            inc_alloc.push(al);
            full_alloc.push(fal);
        }
        if it == last_it {
            // Oracle A (SELF_EQUIVALENCE): incremental == clean full
            // rebuild, observable surface only (identity may differ by
            // contract).
            let oracle_ok = state.block_count() == fresh.block_count()
                && state.blocks().zip(fresh.blocks()).all(|(a, b)| {
                    a.kind() == b.kind()
                        && a.source_range() == b.source_range()
                        && a.line_span() == b.line_span()
                        && a.detail() == b.detail()
                        && a.inline() == b.inline()
                });
            // Oracle C (LOSSLESSNESS): coverage of the post-edit source.
            let new_whole = snap1.slice(SourceRange::new(
                ByteOffset(0),
                ByteOffset(snap1.len_bytes()),
            ));
            let lossless = oracle_lossless(&state, &new_whole);
            let new_pbs = capture(&state);
            let (prefix_aligned, suffix_aligned, proj_blocks, proj_bytes) =
                projection_delta(&old_pbs, doc_text, &new_pbs, &new_whole);
            finals = Some(Finals {
                work: state.last_work(),
                changed_bytes: applied.result.work.changed_bytes,
                changed_lines: applied.result.work.changed_lines,
                doc_bytes: snap1.len_bytes() as u64,
                doc_lines: snap1.line_count() as u64,
                doc_blocks: state.block_count() as u64,
                oracle_ok,
                lossless_ok: lossless.is_ok(),
                gap_bytes: lossless.unwrap_or(0),
                proj_blocks,
                proj_bytes,
                prefix_aligned: prefix_aligned as u64,
                suffix_aligned: suffix_aligned as u64,
                new_pbs,
            });
        }
    }

    let f = finals.expect("measured iterations must capture finals");
    let ia = median_alloc(&mut inc_alloc);
    let fa = median_alloc(&mut full_alloc);
    let w = f.work;

    // Influence vector — from observed counters only.
    let cover_old = covering(&old_pbs, edit_start);
    let cover_new_at = edit_start + edit.new_text.len();
    let cover_new = covering(&f.new_pbs, cover_new_at);
    let observed = influence::Observed {
        edit_start: edit_start,
        edit_end_line,
        changed_lines: f.changed_lines,
        doc_bytes: f.doc_bytes,
        doc_blocks: f.doc_blocks,
        bytes_scanned: w.bytes_scanned,
        blocks_reparsed: w.blocks_reparsed,
        blocks_created: w.blocks_created,
        blocks_removed: w.blocks_removed,
        survivor_blocks_shifted: w.survivor_blocks_shifted,
        survivor_inline_nodes_shifted: w.survivor_inline_nodes_shifted,
        block_records_moved: w.block_records_moved,
        restart_line: w.restart_line,
        convergence_line: w.convergence_line,
        cover_kind: cover_old.kind,
        cover_end_line: cover_old.line_end,
    };
    let vector: InfluenceVector = influence::classify(&observed, mermaid_candidate);

    Ok(Row {
        case_id: case.id.to_string(),
        family: case.family.to_string(),
        construct: case.construct.to_string(),
        predicted: case.predicted.to_string(),
        corpus: corpus.to_string(),
        position: case
            .pos
            .map(|s| s.to_string())
            .unwrap_or_else(|| position_name(frac).to_string()),
        doc_bytes: f.doc_bytes,
        doc_lines: f.doc_lines,
        doc_blocks: f.doc_blocks,
        changed_bytes: f.changed_bytes,
        changed_lines: f.changed_lines,
        full_us: median_f64(&mut full_us),
        inc_us: median_f64(&mut inc_us),
        bytes_scanned: w.bytes_scanned,
        lines_scanned: w.lines_scanned,
        blocks_reparsed: w.blocks_reparsed,
        blocks_reused: w.blocks_reused,
        blocks_created: w.blocks_created,
        blocks_removed: w.blocks_removed,
        blocks_examined: w.blocks_examined,
        dirty_regions: w.dirty_regions,
        restart_line: w.restart_line,
        convergence_line: w.convergence_line,
        inline_blocks_reparsed: w.inline_blocks_reparsed,
        inline_bytes_scanned: w.inline_bytes_scanned,
        survivor_blocks_shifted: w.survivor_blocks_shifted,
        survivor_inline_nodes_shifted: w.survivor_inline_nodes_shifted,
        block_records_moved: w.block_records_moved,
        proj_blocks_invalidated: f.proj_blocks,
        proj_bytes_invalidated: f.proj_bytes,
        alloc_count_inc: ia.count,
        alloc_bytes_inc: ia.bytes,
        alloc_count_full: fa.count,
        alloc_bytes_full: fa.bytes,
        oracle_ok: f.oracle_ok,
        edit_start: edit_start as u64,
        edit_end: edit_end as u64,
        cover_old_kind: cover_old.kind.name().to_string(),
        cover_old_lines: cover_old.line_end.saturating_sub(cover_old.line_start) as u64,
        cover_new_kind: cover_new.kind.name().to_string(),
        prefix_aligned: f.prefix_aligned as u64,
        suffix_aligned: f.suffix_aligned as u64,
        gap_bytes_unclaimed: f.gap_bytes,
        lossless_ok: f.lossless_ok,
        syntax_class: vector.syntax.name(),
        semantic_class: vector.semantic.name(),
        representation_class: vector.representation.name(),
        provider: vector.downstream.p_provider.name(),
        note: String::new(),
    })
}
