# P0-02 Implementation Note — Incremental Block Index and Markdown L1 Core

Status: **complete** on `feat/p0-02-incremental-block-index` (2026-08-23).
Companion docs: `markdown-l1-semantic-contract.md` (normative dialect),
ADR-003/ADR-004, `performance-invariants.md`, issue #12 (audit R1–R11).

## What landed

- `docs/product/markdown-l1-semantic-contract.md` — written **before** the
  parser: block vocabulary, flat nesting rule, classification order,
  opener/continuation/termination semantics per construct, inline model,
  tiling invariant, restart/convergence semantics, `InternalBlockId`
  rules, and a 14-entry deviation table vs CommonMark 0.31.2 (D1–D14).
- `crates/markit-core/src/markdown/` — GPUI-independent, zero-dependency:
  - `lex.rs` — pure lexical seam (line classification shapes, backtick
    and emphasis delimiter runs). SIMD status: **DEFERRED — no profiler
    evidence yet**; the seam exists so a SIMD classifier is a measured
    replacement, not a default.
  - `block.rs`, `identity.rs`, `state.rs` — `BlockRecord` (source range,
    half-open line span, `state_before/after`, FNV-1a fingerprint, kind
    detail), `InternalBlockId` (monotonic counter, internal product
    identity, **not** plugin identity), `BlockParseState`
    (`Ground | InFence`).
  - `parser.rs` — restartable forward parser; the same code path serves
    full builds and island reparses.
  - `inline.rs` — source-referenced inline IR per independent run;
    single-pass scan (escapes, equal-length code spans, inline links
    with the no-nested-links rule), whitespace-only flanking for `*`
    with the intraword-`_` restriction (D11), CommonMark-style delimiter
    pairing minus the rule of 3 (D10), iterative frame-stack assembly
    (unbounded delimiter nesting cannot overflow the stack).
  - `incremental.rs` — island resynchronization: rewind to the latest
    safe boundary (extended one block back when the edit lands on a
    first line and could continue the paragraph/list/quote above),
    reparse forward, converge at `Ground` state exactly at the shifted
    start of the next survivor; kind-pairing by longest
    order-preserving subsequence with a documented linear fallback.
  - `mod.rs` — `MarkdownState` (`build`/`update`, query views
    `block_at_offset` / `blocks_in_range` / `blocks_in_lines` /
    `block_by_id` / `inline_ir`), `MarkdownWork` counters,
    fail-closed `MarkdownStateError`.

Representation is the prescribed one: a plain `Vec<BlockRecord>` tiling
the document (every byte belongs to exactly one block), binary-search
queries, local splice on edits. No interval trees, arenas, ropes, SIMD,
threads, or parser frameworks.

## Bugs the oracles caught during construction

The differential/golden batteries earned their keep — three real defects
were caught before merge, each invisible to spot-checks:

1. **Relative link-text base**: link children were parsed with a
   run-relative base, so blocks not at offset 0 (and survivors after a
   shift) carried wrong child ranges. Caught by the 10K-line
   inline-dense family, not by base-0 fixtures.
2. **Convergence without the state gate**: the position-equality check
   did not require `Ground` parser state, so a fence that had swallowed
   to EOF still "converged" against a survivor standing exactly at EOF.
3. **Paragraph backward dependency**: an edit landing on a block's first
   line can extend the *previous* block (continuation lines), so the
   restart must sometimes include it — the survivor below would
   otherwise keep a stale shape that a full rebuild would have merged.

## Structural measurements (counters, not milliseconds)

From `tests/markdown_incremental.rs` on the controlled families:

| scenario | 10K | 100K | 1M |
|---|---|---|---|
| local edit (5 families), `lines_scanned` | ≤ 6 | ≤ 6 | (see sparse) |
| local edit `dirty_regions` / `blocks_reparsed` | 1 / ≤2 | 1 / ≤2 | 1 / ≤2 |
| sparse two-location (line 10 + line 999 990) | — | — | 2 islands, ≤12 lines, middle never parsed |
| fence content edit (100K) | — | 1 island, ≤5 lines | — |
| deleted closing fence | honest rescan onward | honest rescan to EOF | — |

The sparse regression is permanent: two distant one-character edits on a
one-million-line document parse at most a dozen lines; the middle is
shifted as bookkeeping, never parsed.

## Known residuals (owned, not hidden)

- **Survivor range shift is O(blocks after the edit)** — the same
  accepted cost class as the P0-01 line index's suffix shift (ADR-003
  Notes); a buffer/index redesign is a real-workload decision.
  Deliberately not counted as `blocks_examined` scanning; *parsing* is
  what the counters bound.
- **Block granularity is the invalidation unit**: one unbroken
  100K-item list is a single block, and an edit inside it reparses the
  block. The L1 dialect (D7/D8) bounds list blocks by blank-line
  separation; a finer-grained list model would be a dialect change with
  its own contract update, not a silent optimization.
- **Fence semantics are spec-faithful**: unclosed fences propagate to
  end of document (issue #12 R7 supersedes the earlier "opener only if
  a close exists ahead" idea — wording repaired in ADR-004,
  `performance-invariants.md`, `architecture.md` §10 in this change).
- **Inline IR is stored per block eagerly** — compact, but a lazily
  materialized LOD (§11.6 model) is available if memory measurements
  ever demand it.
- **`blocks_reparsed == 1` is an observation, not a law** (issue #12
  R6): the invariant is the smallest semantically valid region, and the
  fence cases honestly exceed one block.

## Validation

- `cargo fmt --check`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings` — all clean;
  `#![forbid(unsafe_code)]` retained; crate remains dependency-free.
- Suites: 114 unit + 9 golden/identity/staleness + 8 differential and
  large-document tests, including per-edit incremental==rebuild equality
  over randomized Markdown-dense corpora (deterministic seeds) and the
  1M-line sparse regression.

## Issue #12 coverage

R5 (golden suite): landed per construct incl. malformed/CJK/emoji/EOF.
R6 (one-block is not a law): counters + wording repairs above. R7 (no
semantic bending for fences): contract §6.7 + doc repairs. R9 (internal
vs plugin identity): `InternalBlockId` docs + contract §10; nothing
extension-facing was exposed.
