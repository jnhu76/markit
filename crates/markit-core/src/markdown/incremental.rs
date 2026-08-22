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

use crate::change::AppliedEdit;
use crate::markdown::block::{BlockKind, BlockRecord};
use crate::markdown::identity::InternalBlockId;
use crate::markdown::inline;
use crate::markdown::parser::BlockParser;
use crate::markdown::MarkdownWork;
use crate::snapshot::DocumentSnapshot;

/// Above this `dead × fresh` product, island kind-pairing falls back
/// from the exact longest-common-subsequence alignment to a linear
/// heuristic (contract §10 requires a deterministic rule, not an
/// unbounded DP on paste-scale islands).
const LCS_LIMIT: usize = 1_000_000;

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

    // Per-edit byte/line deltas and prefix sums, so any old position can
    // ask "by how much have the edits entirely before me moved me?" in
    // one binary search — correct even when an island's reparse
    // overshoots past later islands.
    let byte_deltas: Vec<i64> = edits.iter().map(|e| e.byte_delta).collect();
    let line_deltas: Vec<i64> = edits
        .iter()
        .map(|e| {
            (e.new_line_span.end.0 as i64 - e.new_line_span.start.0 as i64)
                - (e.old_line_span.end.0 as i64 - e.old_line_span.start.0 as i64)
        })
        .collect();
    let ends: Vec<usize> = edits.iter().map(|e| e.old_range.end.as_usize()).collect();
    let line_ends: Vec<usize> = edits.iter().map(|e| e.old_line_span.end.0).collect();
    let byte_before = |old_offset: usize| -> i64 {
        let count = ends.partition_point(|&end| end <= old_offset);
        byte_deltas[..count].iter().sum()
    };
    let line_before = |old_line: usize| -> i64 {
        let count = line_ends.partition_point(|&end| end <= old_line);
        line_deltas[..count].iter().sum()
    };

    let mut new_blocks: Vec<BlockRecord> = Vec::with_capacity(blocks.len());
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

        // Survivors between the previous island and this one: shifted,
        // never parsed.
        for block in &blocks[cursor..restart] {
            let byte_delta = byte_before(block.source_range.start.as_usize());
            let line_delta = line_before(block.line_span.start.0);
            new_blocks.push(block.shifted(byte_delta, line_delta));
            work.blocks_examined += 1;
        }

        let restart_new_line = (blocks[restart].line_span.start.0 as i64
            + line_before(blocks[restart].line_span.start.0))
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
                    + byte_before(blocks[dead_end].source_range.start.as_usize()))
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
                    + byte_before(blocks[dead_end].source_range.start.as_usize()))
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
        for (block, reused) in parsed.into_iter().zip(pair_of_fresh) {
            let id = reused.unwrap_or_else(|| InternalBlockId::mint(next_id));
            let mut record = block.into_record(id);
            if inline::attach_inline(&mut record, snapshot) {
                work.inline_blocks_reparsed += 1;
            }
            new_blocks.push(record);
        }
        cursor = dead_end;
    }

    // Tail survivors after the last island.
    for block in &blocks[cursor..] {
        let byte_delta = byte_before(block.source_range.start.as_usize());
        let line_delta = line_before(block.line_span.start.0);
        new_blocks.push(block.shifted(byte_delta, line_delta));
        work.blocks_examined += 1;
    }

    work.restart_line = restart_line_min.unwrap_or(0);
    work.convergence_line = convergence_line_max.unwrap_or(snapshot.line_count() as u64);
    *blocks = new_blocks;
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
        return pair_kinds_heuristic(dead, fresh);
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

/// Linear fallback for islands beyond [`LCS_LIMIT`]: two pointers; on
/// mismatch advance the side whose head does not reappear in the other's
/// remainder (ties advance dead). Deterministic, O(n + m).
fn pair_kinds_heuristic(dead: &[BlockKind], fresh: &[BlockKind]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < dead.len() && j < fresh.len() {
        if dead[i] == fresh[j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if dead[i + 1..].contains(&fresh[j]) && !fresh[j + 1..].contains(&dead[i]) {
            i += 1;
        } else if !dead[i + 1..].contains(&fresh[j]) && fresh[j + 1..].contains(&dead[i]) {
            j += 1;
        } else {
            i += 1;
        }
    }
    pairs
}
