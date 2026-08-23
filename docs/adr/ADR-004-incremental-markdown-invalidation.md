# ADR-004 — Incremental Markdown invalidation model

Status: accepted (A4-R2).

> Post-pivot note (2026-08-18): the incremental pipeline (Document →
> Block Index → Incremental Parse → Affected Blocks → Styled Runs →
> Visible Layout → presentation) transfers to the Rust core. In the
> direct-GPUI architecture the presentation tail is GPUI elements/shaped
> text instead of the PocketJS DrawList; the change-range propagation and
> block-granular invalidation principles are unchanged.

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
cascade (1M: 30 197 lines, 68.9 ms) — recorded, owned, and reported
through structural counters.

> P0-02 qualification (2026-08-23, issue #12 R6/R7): "exactly 1 block"
> is an observation of ordinary measured local edits, not a law — the
> requirement is the *smallest semantically valid region*. And fence
> recovery must **not** be bounded by changing Markdown semantics: the
> Markit L1 dialect (`docs/product/markdown-l1-semantic-contract.md`
> §6.7) keeps CommonMark fence behavior, so an unclosed fence honestly
> swallows to end of document and the counters say so. Any future bound
> must be a semantics-preserving, evidence-driven product decision
> (for example a presentation-level deferral), never a parser rule that
> renders differently than the spec.

## Alternatives considered

- Full re-parse with a reference parser (marked) as the production path:
  rejected for the hot path; used as an oracle only.
- Per-line parsing: rejected — block-level runs handle cross-line
  emphasis and match the block invalidation model.

## Decision

Document → Block Index → Incremental Parse → Affected Blocks → Styled
Runs → Visible Layout → presentation (GPUI elements / shaped text in the
direct-GPUI architecture), with change-range propagation at every layer.
Structural-edit cost is owned and documented, never benchmarked away.
