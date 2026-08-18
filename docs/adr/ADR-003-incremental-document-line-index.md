# ADR-003 — Incremental document + line indexing

Status: accepted (A3-P1, reaffirmed A4).

> Post-pivot note (2026-08-18): this decision is implementation-neutral
> and transfers directly to the Rust core (`markit-core` Document +
> LineIndex + explicit `EditResult.change` propagation). No PocketJS
> binding.

## Observed problem

A2: one 1M-character edit cost 224.8 ms; 94.6% of the turn was Markit's
per-edit full-document `lineStarts()` scan (O(N) per edit, counterfactual
CF=1 → 7.5 ms).

## Intervention / causal evidence

A3-P1 replaced the per-edit scan with an incrementally maintained
`LineIndex`: one full scan at load, local updates per edit (drop/add
newline entries in the changed range, shift the suffix by the length
delta). 1M edit: 224.8 → 9.9 ms; full scans per edit 1 → 0; 38 tests
incl. randomized differential vs the full-scan oracle.

## Scaling evidence

10K→1M amplification collapsed ~25× → ~1.1× (noise band). The remaining
term is the O(lines-after) suffix shift (position-dependent,
instrumented; begin-position 1M edits ~2.3 ms).

## Alternatives considered

- Keep full scans: rejected (measured root cause).
- Rope/piece-table buffer now: deferred — a real product workload
  decision, not a pre-emptive one.

## Decision

The document is a string + incremental LineIndex; explicit changed-range
propagation (`EditResult.change`) is the seam every layer consumes. A
buffer redesign may revisit this when the product workload demands it.
