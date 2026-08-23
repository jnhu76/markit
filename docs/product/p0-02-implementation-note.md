# P0-02 Implementation Note — Incremental Block Index and Markdown L1 Core

Status: **complete** on `feat/p0-02-incremental-block-index` (2026-08-23;
review round 1 fixes applied same day). Companion docs:
`markdown-l1-semantic-contract.md` (normative dialect), ADR-003/ADR-004,
`performance-invariants.md`, issue #12 (audit R1–R11).

## What landed

- `docs/product/markdown-l1-semantic-contract.md` — written **before**
  the parser: block vocabulary, flat nesting rule, classification order,
  opener/continuation/termination semantics per construct, inline model,
  tiling invariant, restart/convergence semantics, `InternalBlockId`
  rules, and a 16-entry deviation table vs CommonMark 0.31.2 (D1–D16).
- `crates/markit-core/src/markdown/` — GPUI-independent, zero-dependency:
  - `lex.rs` — pure lexical seam (line classification shapes, backtick
    and emphasis delimiter runs). SIMD status: **DEFERRED — no profiler
    evidence yet**; the seam exists so a SIMD classifier is a measured
    replacement, not a default.
  - `block.rs`, `identity.rs`, `state.rs` — `BlockRecord` (source range,
    half-open line span, `state_before/after`, FNV-1a fingerprint, kind
    detail), `InternalBlockId` (monotonic counter, internal product
    identity, **not** plugin identity), `BlockParseState`
    (`Ground | InFence`). All three record types are crate-private
    (contract §8): consumers read `BlockView` query views.
  - `parser.rs` — restartable forward parser; the same code path serves
    full builds and island reparses. Classification text drops one
    pre-terminator `\r` (CRLF-aware, bytes verbatim).
  - `inline.rs` — source-referenced inline IR per independent run;
    single-pass scan (escapes, equal-length code spans, inline links
    with the no-nested-links rule), whitespace-only flanking for `*`
    with the intraword-`_` restriction (D11), CommonMark-style delimiter
    pairing minus the rule of 3 (D10) over per-character opener stacks
    (linear), iterative frame-stack assembly, and adversarial bounds
    (D15/D16): nesting-depth cap, link-work budget, no-`]` watermark.
  - `incremental.rs` — island resynchronization in two passes: a
    read-only **analysis** pass (rewind with the backward-dependency
    rule, reparse, converge) and a **mutation** pass that splices in
    place — prefix survivors untouched, zero-delta survivor segments
    untouched, nonzero-delta segments rewritten in place once, and all
    of it counted. Old→new deltas are true prefix sums; large-island
    identity pairing falls back to bounded prefix/suffix runs.
  - `mod.rs` — `MarkdownState` (`build`/`update`, query views
    `block_at_offset` / `blocks_in_range` / `blocks_in_lines` /
    `block_by_id` / `inline_ir` over `BlockView`), `MarkdownWork`
    counters, fail-closed `MarkdownStateError`.

Representation is the prescribed one: a plain `Vec<BlockRecord>` tiling
the document (every byte belongs to exactly one block), binary-search
queries, local splice on edits. No interval trees, arenas, ropes, SIMD,
threads, or parser frameworks.

## Review round 1 (PR #14, 2026-08-23)

The review's central finding — *"reparsing one block" quietly meant
"rewriting every record"* — and its five companion findings were all
addressed on the branch:

1. **No whole-index rebuild per edit.** The updater no longer copies the
   record stream: analysis is read-only; mutation splices the island
   (equal-count islands overwrite in place, so they move no records) and
   rewrites survivor segments in place only when the applicable
   byte/line delta is nonzero. An equal-length local edit touches no
   survivor and moves no record, at any document size (regression at
   10K/100K).
2. **Truthful state-maintenance counters.** `survivor_blocks_shifted`,
   `survivor_inline_nodes_shifted`, `block_records_moved`, and
   `inline_bytes_scanned` now exist alongside the parse counters; the 1M
   sparse regression reports the honest ~500K-record tail rewrite of an
   inserting edit instead of hiding it outside "scanning".
3. **Real prefix sums** for `byte_before`/`line_before`: one binary
   search per query, not O(log E + E) re-summation (which made
   multi-edit transactions O(blocks × edits)).
4. **Genuinely bounded large-island pairing**: common prefix/suffix run
   matching, O(dead + fresh); the previous "linear" heuristic contained
   per-step `contains` scans and could degrade to O(dead × fresh).
   Identity preservation never outranks responsiveness.
5. **CRLF semantics, bytes preserved** (Windows-first): classification
   text drops one pre-terminator `\r`; ranges, fingerprints, runs, and
   saved files keep `\r\n` verbatim. Golden fixtures, a differential
   alphabet token, and a 10K/100K scaling family cover it.
6. **Narrowed public surface**: `BlockView` query views; `BlockRecord`,
   `BlockParseState`, `BlockFingerprint` are crate-private, pinned by
   compile-fail doctests.
7. **Adversarial inline bounds** (D15/D16): linear emphasis pairing,
   O(1) `prev_char`, bracket watermark, per-run link budget, nesting
   cap; `tests/markdown_adversarial.rs` pins linear structural growth.
   No SIMD — still no profiler evidence.

## Bugs the oracles caught during construction

The differential/golden batteries earned their keep — seven real defects
were caught before merge, most invisible to spot-checks:

1. **Relative link-text base**: link children were parsed with a
   run-relative base, so blocks not at offset 0 (and survivors after a
   shift) carried wrong child ranges. Caught by the 10K-line
   inline-dense family, not by base-0 fixtures.
2. **Convergence without the state gate**: the position-equality check
   did not require `Ground` parser state, so a fence that had swallowed
   to EOF still "converged" against a survivor standing exactly at EOF.
3. **Paragraph backward dependency**: an edit landing on a block's first
   line can extend the *previous* block (continuation lines), so the
   restart must sometimes include it.
4. **Phantom final blank**: deleting the final newline let the empty
   final-line block converge at EOF and survive, where a rebuild
   produces no such block (caught when `\r\n` tokens made final-newline
   deletions reachable).
5. **Blank-run backward dependency**: an edit on a block's first line
   must also rewind into a preceding Blank run — runs are maximal, and
   new blank lines must merge with the neighbor above.
6. **Duplicated final empty line**: a reparse ending in a blank run has
   already consumed the final empty line; converging against the empty
   survivor duplicated it.
7. **Line-delta undercount** (P0-01): exclusive-end line spans cannot
   express a replacement ending in a terminator; `AppliedEdit` now
   carries an exact `line_delta` (newlines in minus newlines out,
   debug-asserted to sum to the document delta).

## Structural measurements (counters, not milliseconds)

Parse work vs state-maintenance work, reported separately — the review's
rule: *"semantically, how much changed? plus, mechanically, how much
data did we touch?"* From `tests/markdown_incremental.rs`:

| scenario | parse work | state maintenance |
|---|---|---|
| local edit, 5 families × {10K, 100K} | 1 island, ≤ 6 lines, ≤ 2 blocks | shifted = moved = **0** |
| equal-length local edit, 10K/100K | 1 island, ≤ 6 lines, ≤ 2 blocks | shifted = moved = inline-shifted = **0** |
| sparse 2-edit insert, 1M lines | 2 islands, ≤ 12 lines, 4 blocks | ~500K survivors shifted in place (honest, counted), moved = 0 |
| fence content edit, 100K | 1 island, ≤ 5 lines | shifted = moved = 0 |
| deleted closing fence | honest rescan to EOF (counters say so) | island replaces the swallowed tail |
| adversarial inline (brackets/closers/links) | `inline_bytes_scanned` ≤ C·len, linear under doubling; depth ≤ 256 | — |

The sparse regression is permanent: two distant one-character edits on a
one-million-line document parse at most a dozen lines.

## Known residuals (owned, not hidden)

- **Inserting/deleting edits rewrite the survivor records after them**
  (`O(blocks after the edit)` in-place shifts, `O(tail)` record moves
  when an island changes the block count) — inherent to absolute ranges
  in a `Vec`, the same accepted cost class as the P0-01 line index's
  suffix shift (ADR-003 Notes), now counted per edit. A
  relative/offset-encoded representation is a real-workload decision,
  not a pre-emptive one; equal-length edits already touch nothing.
- **Block granularity is the invalidation unit**: one unbroken 100K-item
  list is a single block, and an edit inside it reparses the block. The
  L1 dialect (D7/D8) bounds list blocks by blank-line separation; a
  finer-grained list model would be a dialect change with its own
  contract update.
- **First-line edits reparse one boundary neighbor** (the blank/quote/
  list/paragraph above) because runs are maximal — `blocks_reparsed`
  reports it honestly; it is never a document-scale cost.
- **Fence semantics are spec-faithful**: unclosed fences propagate to
  end of document (issue #12 R7 supersedes the earlier "opener only if
  a close exists ahead" idea — wording repaired in ADR-004,
  `performance-invariants.md`, `architecture.md` §10).
- **Adversarial degradation is bounded, not avoided**: past D15/D16
  bounds, brackets/delimiters stay literal text — deterministic, so
  incremental parses equal rebuilds; the caps exist because unbounded
  trees/scans crash or stall an editor on hostile input.
- **Inline IR is stored per block eagerly** — compact, but a lazily
  materialized LOD (§11.6 model) is available if memory measurements
  ever demand it.
- **`blocks_reparsed == 1` is an observation, not a law** (issue #12
  R6): the invariant is the smallest semantically valid region, and the
  fence cases honestly exceed one block.

## Validation

- `cargo fmt --check`, `cargo test --workspace` (debug and release),
  `cargo clippy --workspace --all-targets -- -D warnings` — all clean;
  `#![forbid(unsafe_code)]` retained; crate remains dependency-free.
- Suites: 124 unit (including the full-record in-crate differential
  oracle) + 11 golden/identity/staleness/CRLF + 9 differential and
  large-document tests (per-edit incremental == rebuild over randomized
  Markdown-dense corpora with mixed line endings, deterministic seeds,
  the 1M-line sparse regression, and the equal-length zero-touch
  regression) + 5 adversarial-inline regressions + 7 public-surface.

## Issue #12 coverage

R5 (golden suite): landed per construct incl. malformed/CJK/emoji/EOF/
CRLF. R6 (one-block is not a law): counters + wording repairs. R7 (no
semantic bending for fences): contract §6.7 + doc repairs. R9 (internal
vs plugin identity): `InternalBlockId` docs + contract §10; nothing
extension-facing was exposed.
