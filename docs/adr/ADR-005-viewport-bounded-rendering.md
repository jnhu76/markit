# ADR-005 — Viewport-bounded rendering invariant

Status: accepted (A3-G1, reaffirmed A4).

> Post-pivot note (2026-08-18): the invariant is implementation-neutral
> and is a core requirement for the GPUI product layer: only visible
> lines (+ overscan) are laid out and painted; the materialized GPUI
> element/shaped-text work must scale with the visible presentation, not
> document size; the idle editor must not request frames. The A4 "DrawList
> words identical" measurement was the PocketJS-side evidence for the same
> invariant.

## Observed problem

A2: GPUI shaped 18 081 lines per frame at 1M because the element was
sized to full content height (98% of the frame). A3 fixed the equivalent
on both substrates.

## Causal evidence

A3-G1: viewport-sized element → 1M frame 52.4 → 1.21 ms, lines shaped
18 081 → 25 (26 visible + 2 overscan), logical scroll extent preserved.

## Scaling evidence

10K→1M static redraw ≤ 1.8× (gate ≤ 2×). A4-R1: DrawList words identical
at 10K/100K/1M (3046); idle redraw ~0 frames/s.

## Decision

Frame work is viewport-bounded whenever semantics permit: visible range +
overscan only, DrawList sized by the visible presentation, demand
rendering on DrawList change. The document may be huge; the frame must
not be.
