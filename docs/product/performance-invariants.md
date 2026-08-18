# Markit — Performance Invariants

These are the A1–A4 research results turned into architecture invariants
for the direct-GPUI product (ADR-008). They are **work-amplification
invariants** first — the product must not accidentally do O(document)
work in the hot paths — with loose guardrails (not hard ms SLAs; CI must
not flake on wall clocks). Evidence for each invariant lives in
`docs/research/` (historical A2–A4 measurements) and the ADRs.

## INV-01 — Normal single-character edits must not scan the entire document

The A2 root cause (per-edit `lineStarts()` full scan, 94.6% of the 1M
edit turn) is removed by the incremental LineIndex (A3-P1) and the
incremental Markdown BlockIndex (A4-R2). Guard: full-document scans per
edit == 0; blocks_reparsed for local edits == 1 at any document size
(measured 10K→1M).

## INV-02 — Normal frame work must be viewport-bounded

Only the visible range (+ overscan) is laid out and drawn (A3-G1 formula;
measured 25 lines shaped at 1M on the GPUI prototype). The view model
emits exactly the visible lines; nothing shapes or paints beyond them.

## INV-03 — Materialized presentation work scales with the visible presentation, not the document

The equivalent of the A4 "DrawList words identical at 10K/100K/1M (3046)"
measurement: materialized GPUI elements / shaped text / paint work must
scale with the visible presentation, not total document size. A document
can be huge; the frame must not be.

## INV-04 — An idle document must not continuously request frames

Demand rendering: the idle editor must not request frames while nothing
changed (measured ~0 frames/s, ~1% CPU in A3/A4). No animation/timer loop
may force frames while nothing changed.

## INV-05 — A local Markdown edit reparses the smallest semantically valid region

The BlockIndex rescan stops at the first stable boundary; local edits
reparse exactly one block (measured). Structural edits (fence boundaries)
may invalidate broadly — that cost is owned, documented, and bounded by a
product strategy (see Notes and `architecture.md` §8).

## INV-06 — Platform integration must not add unnecessary work to the per-edit hot path

Keyboard/mouse/scroll events cross the platform edge per tick;
everything else (clipboard, IME candidates, dialogs, fonts, files)
arrives through capability paths that never run inside the per-edit
path. GPUI itself sits under the editor, but Markit must not add
per-edit work (conversion, allocation, re-shaping) at the GPUI edge
beyond what the visible presentation requires.

## INV-07 — Performance measurement prioritizes p95/p99, not only averages

All A4 cells report p50/p95/max per tick. Product regression runs must
keep tail values visible.

## Notes

- INV-05's fence cascade is the known exception: a fence-boundary edit
  re-parsed up to 30K lines at 1M (68.9 ms, honest, A4 measurement). The
  product must bound fence recovery (only treat ``` as an opener when a
  matching close exists ahead) and re-measure before shipping L1.
- The model's O(lines-after) line-index suffix shift (up to ~2.3 ms at
  1M begin-position edits, A3 measurement) is instrumented and
  position-dependent; a buffer redesign is a real-workload decision, not
  a pre-emptive one.
- The A4-R1 reactive-identity finding (~90 native node recreations per
  edit from fresh visible-list objects) is a design warning for the GPUI
  layer: per-line presentation must derive statelessly from the model's
  visible range (see `issue-backlog.md` — viewport identity discipline).

## Regression battery

Core unit tests + invariants battery (to be re-expressed for the Rust
core; the A2–A4 battery `bench/run-a4.py` remains as historical tooling):

| check | invariant covered |
|-------|-------------------|
| full_document_scans == 0 per local edit | INV-01 |
| blocks_reparsed == 1 for local edits at 10K and 1M | INV-01/05 |
| materialized presentation work flat across sizes | INV-02/03 |
| op churn per edit viewport-bound | INV-02 |
| idle: no frame requests | INV-04 |
| structural edit radius recorded (bounded recovery to come) | INV-05 |
