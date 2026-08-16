# Markit Desktop — Performance Invariants

These are the A1–A4 research results turned into architecture invariants.
They are **work-amplification invariants** first — the product must not
accidentally do O(document) work in the hot paths — with loose guardrails
(not hard ms SLAs; CI must not flake on wall clocks). Evidence for each
invariant lives in `docs/phase-a4-final-research-closeout.md` and the
A2/A3 reports.

## INV-01 — Normal single-character edits must not scan the entire document

The A2 root cause (per-edit `lineStarts()` full scan, 94.6% of the 1M
edit turn) is removed by the incremental LineIndex (A3-P1) and the
incremental Markdown BlockIndex (A4-R2). Guard: full-document scans per
edit == 0; blocks_reparsed for local edits == 1 at any document size
(measured 10K→1M).

## INV-02 — Normal frame work must be viewport-bounded

Only the visible range (+ overscan) is laid out and drawn (A3-G1 formula;
measured 25 lines shaped at 1M). The view model emits exactly the visible
lines; nothing shapes or paints beyond them.

## INV-03 — DrawList size scales with the visible presentation, not the document

Measured: 3046 words at 10K and at 1M (A3/A4). A document can be huge;
the DrawList must not be.

## INV-04 — An idle document must not continuously redraw

Demand rendering: the host redraws only when the DrawList hash changes
(idle measured ~0 frames/s, ~1% CPU). No animation/timer loop may force
frames while nothing changed.

## INV-05 — A local Markdown edit reparses the smallest semantically valid region

The BlockIndex rescan stops at the first stable boundary; local edits
reparse exactly one block (measured). Structural edits (fence boundaries)
may invalidate broadly — that cost is owned, documented, and bounded by a
product strategy (see INV-07 note and `architecture.md` §10).

## INV-06 — Platform integration must not enter the hot editing path unnecessarily

Keyboard/mouse/scroll events cross the host/svc boundary per tick;
everything else (clipboard, IME candidates, dialogs, fonts, files)
arrives through capability providers that never run inside the per-edit
path. The caret-rect svc message is the only per-tick platform payload.

## INV-07 — Performance measurement prioritizes p95/p99, not only averages

All A4 cells report p50/p95/max per tick (see `bench/run-a4.py`).
Product regression runs must keep tail values visible.

## Notes

- INV-05's fence cascade is the known exception: a fence-boundary edit
  re-parsed up to 30K lines at 1M (68.9 ms, honest). The product must
  bound fence recovery (only treat ``` as an opener when a matching
  close exists ahead) and re-measure before shipping L1.
- The model's O(lines-after) line-index suffix shift (up to ~2.3 ms at
  1M begin-position edits) is instrumented and position-dependent; a
  buffer redesign is a real-workload decision, not a pre-emptive one.
- The Solid visible-list discipline (stable item identity + item-scoped
  memos, A4-R1) is a code-level invariant for the PocketJS UI layer:
  visible-list items must never be fresh objects per render, or every
  edit re-mounts the visible list (~90 node creations per edit).

## Regression battery

`bench/run-a4.py` cells map to invariants:

| cell | invariant checked |
|------|-------------------|
| r1-scale (base) | INV-01/02/03 (full scans == 0; words flat across sizes) |
| r1-ops | INV-02 (op churn per edit, viewport-bound) |
| r2-case m1–m4, m6 | INV-01/05 (blocks_reparsed == 1 at 10K and 1M) |
| r2-case m5 | INV-05 structural radius recorded (bounded recovery to come) |
