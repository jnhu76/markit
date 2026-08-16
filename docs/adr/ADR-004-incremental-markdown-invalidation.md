# ADR-004 — Incremental Markdown invalidation model

Status: accepted (A4-R2).

## Observed problem

A full-document Markdown re-parse per edit is O(N) and defeats the
viewport-bound editing model (the "one key → parse entire 1MB → rebuild
everything" trap).

## Causal evidence

A4-R2 built the L1 pipeline: `BlockIndex` (line→block map, incremental
rescan stopping at the first stable boundary, fence state carried) +
per-block styled runs + viewport slicing. 5000-edit randomized
differential vs the full-scan oracle: 0 failures.

## Scaling evidence

Local edits (paragraph, inline, heading, list, off-viewport): exactly 1
block re-parsed at 10K and 1M; edit time viewport-constant (~1.4–1.6 ms).
Structural fence-boundary edits invalidate honestly through the fence
cascade (1M: 30 197 lines, 68.9 ms) — recorded, owned, and to be bounded
by a product strategy (only treat ``` as an opener when a close exists
ahead).

## Alternatives considered

- Full re-parse with a reference parser (marked) as the production path:
  rejected for the hot path; used as an oracle only.
- Per-line parsing: rejected — block-level runs handle cross-line
  emphasis and match the block invalidation model.

## Decision

Document → Block Index → Incremental Parse → Affected Blocks → Styled
Runs → Visible Layout → DrawList, with change-range propagation at every
layer. Structural-edit cost is owned and documented, never benchmarked
away.
