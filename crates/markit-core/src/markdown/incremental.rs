//! Incremental block resynchronization (contract §9).
//!
//! Consumes the **canonical per-edit regions** of an [`EditResult`]
//! (each [`AppliedEdit`]'s line spans — never a covering range) and
//! rewrites only the affected part of the tiling stream:
//!
//! 1. **Rewind** to the latest safe restart boundary at or before the
//!    island's first affected old line (a block start whose
//!    `state_before` is `Ground`).
//! 2. **Reparse forward** over the new document with the same
//!    [`BlockParser`](crate::markdown::parser::BlockParser) the full
//!    build uses.
//! 3. **Converge** when the parser stands at `Ground` exactly at the
//!    (delta-shifted) start of the next surviving old block — a block
//!    whose old lines lie entirely after the island. Parsing is
//!    deterministic and depends only on state + suffix bytes, and the
//!    suffix bytes of survivors are untouched by construction, so
//!    keeping them equals what a full rebuild would produce. Convergence
//!    is parser state + positional alignment; fingerprints are never
//!    load-bearing here (contract §9.3).
//!
//! Islands stay sparse: two distant one-line edits are two islands with
//! their own restart/convergence points, and the stream between them is
//! shifted as bookkeeping, never parsed. Islands merge only when one
//! island's reparse actually consumed another island's region (semantic
//! overlap). If a reparse runs into an unclosed fence it honestly
//! continues to end of document (contract §6.7, issue #12) — visible in
//! `lines_scanned`/`convergence_line`, never hidden.
//!
//! ## Work discipline (the review's core finding)
//!
//! "Reparsed one block" must not quietly mean "rewrote every record".
//! The update therefore runs in two passes:
//!
//! - **Analysis** (read-only over the old stream): island boundaries,
//!   the reparse itself, identity pairing — no mutation.
//! - **Mutation**: survivors before an island are *never touched*
//!   (records are not even copied); a survivor segment is rewritten in
//!   place only when the edits before it moved bytes or lines — and
//!   that rewrite is counted (`survivor_blocks_shifted`,
//!   `survivor_inline_nodes_shifted`); the island splice overwrites in
//!   place when the block count is unchanged and otherwise splices,
//!   counting the records the `Vec` relocates
//!   (`block_records_moved`). An equal-length local edit shifts no
//!   survivor and moves no record at any document size.

use crate::change::AppliedEdit;
use crate::markdown::block::{BlockKind, BlockRecord};
use crate::markdown::identity::InternalBlockId;
use crate::markdown::inline;
use crate::markdown::parser::BlockParser;
use crate::markdown::MarkdownWork;
use crate::snapshot::DocumentSnapshot;

/// Above this `dead × fresh` product, island kind-pairing falls back
/// from the exact longest-common-subsequence alignment to bounded
/// common-prefix/suffix run pairing (contract §10 requires a
/// deterministic rule, not an unbounded DP on paste-scale islands —
/// and identity preservation is never worth unbounded work).
const LCS_LIMIT: usize = 1_000_000;

/// One island's analysis: what to splice where, with replacement
/// records already identified and inline-attached.
struct IslandPlan {
    /// Old-index range of dead records, `[restart, dead_end)`.
    restart: usize,
    dead_end: usize,
    /// Replacement records for the dead range.
    records: Vec<BlockRecord>,
}

pub(crate) fn apply_edits(
    blocks: &mut Vec<BlockRecord>,
    next_id: &mut u64,
    snapshot: &DocumentSnapshot<'_>,
    edits: &[AppliedEdit],
    work: &mut MarkdownWork,
) {
    debug_assert!(
        edits
            .windows(2)
            .all(|w| { w[0].old_range.end.as_usize() <= w[1].old_range.start.as_usize() }),
        "canonical edits are ascending and non-overlapping"
    );

    // Per-edit byte/line deltas as true prefix sums: one binary search
    // per query (O(log E)), correct even when an island's reparse
    // overshoots past later islands. (The previous per-query
    // re-summation made multi-edit transactions O(blocks × edits).)
    let deltas = DeltaMap::new(edits);

    // -------- pass 1: analysis (read-only over `blocks`) ---------------
    let mut plans: Vec<IslandPlan> = Vec::new();
    let mut cursor = 0usize; // first old block not yet handled
    let mut restart_line_min: Option<u64> = None;
    let mut convergence_line_max: Option<u64> = None;

    for edit in edits {
        let island_start_line = edit.old_line_span.start.0;
        let island_end_line = edit.old_line_span.end.0;

        // First old block that is not entirely before the island.
        let first_touching = blocks.partition_point(|b| b.line_span.end.0 <= island_start_line);
        let r0 = first_touching.max(cursor);
        // Entirely consumed by a previous island's reparse: this edit's
        // bytes were already parsed as part of that island.
        if r0 >= blocks.len() || blocks[r0].line_span.start.0 >= island_end_line {
            continue;
        }
        // Rewind to a safe restart boundary (today every record starts
        // Ground; the loop keeps the contract's rule honest if that ever
        // changes).
        let mut restart = r0;
        while restart > cursor && !blocks[restart].state_before.is_ground() {
            restart -= 1;
        }
        // Backward dependency: an edit landing exactly on a block's
        // first line can extend the *previous* block — a plain or
        // continued line continues a paragraph, list item, or quote
        // above it. The reparse must then include that block, or the
        // survivor below it would keep a stale shape (the full rebuild
        // would have merged the lines into it).
        if restart > cursor
            && blocks[restart].line_span.start.0 == island_start_line
            && matches!(
                blocks[restart - 1].kind,
                BlockKind::Paragraph
                    | BlockKind::BlockQuote
                    | BlockKind::UnorderedList
                    | BlockKind::OrderedList
            )
        {
            restart -= 1;
        }
        // Dead: blocks intersecting the island's old lines.
        let mut dead_end =
            restart + blocks[restart..].partition_point(|b| b.line_span.start.0 < island_end_line);
        work.blocks_examined += (dead_end - restart) as u64;

        let restart_new_line = (blocks[restart].line_span.start.0 as i64
            + deltas.line_before(blocks[restart].line_span.start.0))
        .max(0) as usize;
        restart_line_min = Some(
            restart_line_min.map_or(restart_new_line as u64, |m| m.min(restart_new_line as u64)),
        );

        let mut parser = BlockParser::new(snapshot, restart_new_line);
        let mut parsed = Vec::new();
        let converged_line;
        loop {
            // Subsume survivors the reparse already passed: their target
            // position lies strictly behind the parser.
            while dead_end < blocks.len() {
                let target = (blocks[dead_end].source_range.start.as_usize() as i64
                    + deltas.byte_before(blocks[dead_end].source_range.start.as_usize()))
                .max(0) as usize;
                if target < parser.current_offset() {
                    dead_end += 1;
                    work.blocks_examined += 1;
                } else {
                    break;
                }
            }
            // Convergence: Ground state + exact position match with the
            // next survivor (contract §9.2 step 3).
            if dead_end < blocks.len() {
                let target = (blocks[dead_end].source_range.start.as_usize() as i64
                    + deltas.byte_before(blocks[dead_end].source_range.start.as_usize()))
                .max(0) as usize;
                if target == parser.current_offset() && parser.state().is_ground() {
                    converged_line = parser.current_line() as u64;
                    break;
                }
            }
            if parser.at_eof() {
                // Honest propagation to end of document: every remaining
                // old block is dead.
                work.blocks_examined += (blocks.len() - dead_end) as u64;
                dead_end = blocks.len();
                converged_line = snapshot.line_count() as u64;
                break;
            }
            parsed.push(parser.next_block().expect("not at eof"));
        }
        convergence_line_max =
            Some(convergence_line_max.map_or(converged_line, |m| m.max(converged_line)));
        work.lines_scanned += parser.lines_scanned();
        work.bytes_scanned += parser.bytes_scanned();

        // Identity pairing (contract §10): greedy-by-kind would lose
        // obvious pairs on splits/merges, so islands are paired by an
        // order-preserving longest common subsequence over kinds.
        let dead = &blocks[restart..dead_end];
        let fresh_kinds: Vec<BlockKind> = parsed.iter().map(|b| b.kind()).collect();
        let dead_kinds: Vec<BlockKind> = dead.iter().map(|b| b.kind).collect();
        let pairs = pair_kinds(&dead_kinds, &fresh_kinds);
        work.blocks_reparsed += parsed.len() as u64;
        work.blocks_reused += pairs.len() as u64;
        work.blocks_created += (parsed.len() - pairs.len()) as u64;
        work.blocks_removed += (dead.len() - pairs.len()) as u64;
        work.dirty_regions += 1;

        let pair_of_fresh: Vec<Option<InternalBlockId>> = {
            let mut assigned = vec![None; parsed.len()];
            for &(dead_idx, fresh_idx) in &pairs {
                assigned[fresh_idx] = Some(dead[dead_idx].id);
            }
            assigned
        };
        let mut records = Vec::with_capacity(parsed.len());
        for (block, reused) in parsed.into_iter().zip(pair_of_fresh) {
            let id = reused.unwrap_or_else(|| InternalBlockId::mint(next_id));
            let mut record = block.into_record(id);
            if inline::attach_inline(&mut record, snapshot, work) {
                work.inline_blocks_reparsed += 1;
            }
            records.push(record);
        }
        plans.push(IslandPlan {
            restart,
            dead_end,
            records,
        });
        cursor = dead_end;
    }

    // -------- pass 2: mutation -----------------------------------------
    // Index translation from old to live positions: splices only affect
    // indices after the splice point, so live = old + idx_shift holds
    // uniformly for everything not yet handled.
    let mut idx_shift: isize = 0;
    let mut cursor_old = 0usize;
    for plan in plans {
        // Survivors between the previous island and this one. Their
        // applicable delta is uniform (no edit ends inside the segment),
        // so one probe decides: zero-delta segments are left untouched —
        // not copied, not shifted, not even examined.
        let live_lo = cursor_old.wrapping_add_signed(idx_shift);
        let live_hi = plan.restart.wrapping_add_signed(idx_shift);
        shift_survivors(&mut blocks[live_lo..live_hi], &deltas, work);

        // The island splice. Equal block count overwrites in place (no
        // record moves); otherwise splice and count the tail records the
        // Vec physically relocates.
        let dead_len = plan.dead_end - plan.restart;
        let fresh_len = plan.records.len();
        if fresh_len == dead_len {
            blocks[live_hi..live_hi + dead_len]
                .iter_mut()
                .zip(plan.records)
                .for_each(|(slot, record)| *slot = record);
        } else {
            let tail_moved = blocks.len() - (live_hi + dead_len);
            blocks.splice(live_hi..live_hi + dead_len, plan.records);
            work.block_records_moved += tail_moved as u64;
        }
        idx_shift += fresh_len as isize - dead_len as isize;
        cursor_old = plan.dead_end;
    }

    // Tail survivors after the last island.
    let live_lo = cursor_old.wrapping_add_signed(idx_shift);
    shift_survivors(&mut blocks[live_lo..], &deltas, work);

    work.restart_line = restart_line_min.unwrap_or(0);
    work.convergence_line = convergence_line_max.unwrap_or(snapshot.line_count() as u64);
}

/// Rewrites a survivor segment in place when (and only when) the edits
/// before it moved bytes or lines. The segment's delta is uniform — no
/// canonical edit ends inside a survivor segment — so one probe decides
/// for the whole segment.
fn shift_survivors(segment: &mut [BlockRecord], deltas: &DeltaMap, work: &mut MarkdownWork) {
    let Some(first) = segment.first() else {
        return;
    };
    let byte_delta = deltas.byte_before(first.source_range.start.as_usize());
    let line_delta = deltas.line_before(first.line_span.start.0);
    if byte_delta == 0 && line_delta == 0 {
        return;
    }
    for block in segment {
        block.shift_in_place(byte_delta, line_delta);
        work.blocks_examined += 1;
        work.survivor_blocks_shifted += 1;
        work.survivor_inline_nodes_shifted += block.inline.node_count() as u64;
    }
}

/// Old→new coordinate deltas as prefix sums: `byte_before(p)` is the
/// summed byte delta of all edits whose old range ends at or before
/// `p`, in one binary search.
struct DeltaMap {
    byte_ends: Vec<usize>,
    byte_prefix: Vec<i64>,
    line_ends: Vec<usize>,
    line_prefix: Vec<i64>,
}

impl DeltaMap {
    fn new(edits: &[AppliedEdit]) -> Self {
        let mut byte_ends = Vec::with_capacity(edits.len());
        let mut line_ends = Vec::with_capacity(edits.len());
        let mut byte_prefix = Vec::with_capacity(edits.len() + 1);
        let mut line_prefix = Vec::with_capacity(edits.len() + 1);
        byte_prefix.push(0i64);
        line_prefix.push(0i64);
        let (mut bytes, mut lines) = (0i64, 0i64);
        for edit in edits {
            bytes += edit.byte_delta;
            lines += (edit.new_line_span.end.0 as i64 - edit.new_line_span.start.0 as i64)
                - (edit.old_line_span.end.0 as i64 - edit.old_line_span.start.0 as i64);
            byte_prefix.push(bytes);
            line_prefix.push(lines);
            byte_ends.push(edit.old_range.end.as_usize());
            line_ends.push(edit.old_line_span.end.0);
        }
        Self {
            byte_ends,
            byte_prefix,
            line_ends,
            line_prefix,
        }
    }

    fn byte_before(&self, old_offset: usize) -> i64 {
        self.byte_prefix[self.byte_ends.partition_point(|&end| end <= old_offset)]
    }

    fn line_before(&self, old_line: usize) -> i64 {
        self.line_prefix[self.line_ends.partition_point(|&end| end <= old_line)]
    }
}

/// Order-preserving alignment of dead and fresh kind sequences that
/// pairs the maximum number of equal kinds (deterministic: ties prefer
/// the earliest dead block). Pairs keep ids; everything else mints or
/// retires (contract §10 B/C).
fn pair_kinds(dead: &[BlockKind], fresh: &[BlockKind]) -> Vec<(usize, usize)> {
    if dead.is_empty() || fresh.is_empty() {
        return Vec::new();
    }
    if dead.len() * fresh.len() > LCS_LIMIT {
        return pair_kinds_runs(dead, fresh);
    }
    let n = dead.len();
    let m = fresh.len();
    // dp[i][j]: alignment size of dead[i..] × fresh[j..].
    let mut dp = vec![0u32; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * (m + 1) + j] = if dead[i] == fresh[j] {
                dp[(i + 1) * (m + 1) + j + 1] + 1
            } else {
                dp[(i + 1) * (m + 1) + j].max(dp[i * (m + 1) + j + 1])
            };
        }
    }
    let mut pairs = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if dead[i] == fresh[j] && dp[(i + 1) * (m + 1) + j + 1] + 1 == dp[i * (m + 1) + j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if dp[(i + 1) * (m + 1) + j] >= dp[i * (m + 1) + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

/// Bounded fallback for islands beyond [`LCS_LIMIT`] (paste-scale
/// restructures): pair the unambiguous common prefix and suffix runs
/// positionally; the ambiguous middle mints fresh ids (contract §10 C).
/// Genuinely O(dead + fresh) — the previous "linear" fallback contained
/// per-step `contains` scans over the remainders and could degrade to
/// O(dead × fresh), the very blowup it existed to avoid. Identity is a
/// UX optimization; responsiveness owns the budget.
fn pair_kinds_runs(dead: &[BlockKind], fresh: &[BlockKind]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let prefix = dead
        .iter()
        .zip(fresh.iter())
        .take_while(|(a, b)| a == b)
        .count();
    pairs.extend((0..prefix).map(|i| (i, i)));
    let max_tail = (dead.len() - prefix).min(fresh.len() - prefix);
    let mut suffix = 0usize;
    while suffix < max_tail && dead[dead.len() - 1 - suffix] == fresh[fresh.len() - 1 - suffix] {
        suffix += 1;
    }
    pairs.extend((0..suffix).map(|k| (dead.len() - suffix + k, fresh.len() - suffix + k)));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_sum_deltas_match_per_edit_summation() {
        use crate::change::ChangeKind;
        use crate::position::{ByteOffset, LineNumber, SourceRange};

        let span = |a: usize, b: usize| LineNumber(a)..LineNumber(b);
        let range = |a: usize, b: usize| SourceRange::new(ByteOffset(a), ByteOffset(b));
        // Three edits with mixed deltas, ascending in old coordinates.
        let edits = vec![
            AppliedEdit {
                kind: ChangeKind::Insert,
                old_range: range(5, 5),
                new_range: range(5, 8),
                old_line_span: span(0, 1),
                new_line_span: span(0, 1),
                byte_delta: 3,
            },
            AppliedEdit {
                kind: ChangeKind::Delete,
                old_range: range(20, 24),
                new_range: range(23, 23),
                old_line_span: span(2, 3),
                new_line_span: span(2, 3),
                byte_delta: -4,
            },
            AppliedEdit {
                kind: ChangeKind::Insert,
                old_range: range(30, 30),
                new_range: range(26, 32),
                old_line_span: span(4, 4),
                new_line_span: span(4, 7),
                byte_delta: 6,
            },
        ];
        let deltas = DeltaMap::new(&edits);
        // Brute-force reference: sum deltas of edits ending <= position.
        let expect = |offset: usize, ends: &[usize], ds: &[i64]| -> i64 {
            ends.iter()
                .zip(ds)
                .take_while(|(end, _)| **end <= offset)
                .map(|(_, d)| *d)
                .sum()
        };
        let byte_ds = [3i64, -4, 6];
        let line_ds = [0i64, 0, 3];
        for probe in [0usize, 4, 5, 6, 19, 20, 24, 25, 29, 30, 100] {
            assert_eq!(
                deltas.byte_before(probe),
                expect(probe, &[5, 24, 30], &byte_ds),
                "byte_before({probe})"
            );
        }
        for probe in [0usize, 1, 2, 3, 4, 5, 100] {
            assert_eq!(
                deltas.line_before(probe),
                expect(probe, &[1, 3, 4], &line_ds),
                "line_before({probe})"
            );
        }
    }

    #[test]
    fn bounded_fallback_pairs_prefix_and_suffix_runs() {
        use BlockKind::*;
        // Shared head, differing middle, shared tail.
        let dead = [Blank, Paragraph, Heading, Blank, Paragraph];
        let fresh = [Blank, Paragraph, Paragraph, Blank, Paragraph];
        let pairs = pair_kinds_runs(&dead, &fresh);
        // Prefix [Blank, Paragraph] pairs; suffix [Blank, Paragraph]
        // pairs; the Heading→Paragraph middle mints.
        assert_eq!(pairs, vec![(0, 0), (1, 1), (3, 3), (4, 4)]);
        // Order-preserving by construction.
        let mut sorted = pairs.clone();
        sorted.sort();
        assert_eq!(sorted, pairs);
    }
}
