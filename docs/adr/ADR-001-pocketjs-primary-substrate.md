# ADR-001 — PocketJS as the primary UI/runtime substrate

Status: **superseded** (2026-08-18).
Superseded by: [ADR-008-direct-gpui-product-substrate.md](ADR-008-direct-gpui-product-substrate.md).

> Historical decision record.
>
> This ADR reflects the architecture decision at the time it was written:
> A4 selected PocketJS as the primary product foundation. Markit later
> pivoted to direct GPUI as the product foundation (ADR-008). The
> measurements and reasoning below remain part of the evidence base and
> are not retroactively edited.

## Observed problem

Markit needs a three-platform desktop Markdown editor substrate. A1–A3
compared PocketJS and GPUI on parity MVPs; A3 measured GPUI lower on
active-edit latency (~1.2 vs ~9.9 ms at 1M), startup (~336 vs ~750 ms)
and memory (~39 vs ~236 MiB WS). A4-R decomposed the PocketJS residual:
~7.4 ms of the ~8.5 ms edit turn was Solid reactive reconstruction
(Markit's usage), fixed in Markit code to ~1.0–1.5 ms; the PocketJS lower
stack is ~0.3 ms.

## Attribution evidence

`docs/phase-a2-causal-decomposition.md` (scan root cause, counterfactual
CF=1), `docs/phase-a3-intervention-validation.md` (both root causes
removed, A2 predictions reproduced), `docs/phase-a4-final-research-closeout.md`
(R1 counterfactual decomposition + stable-item intervention, pixel/word
identical).

## Scaling evidence

PocketJS edit cost is viewport-constant (10K→1M ≈ 1.1×, A3) and local
invalidation holds for Markdown L1 edits at any size (A4-R2: 1 block at
10K and 1M). Remaining position-dependent term is Markit's O(lines-after)
suffix shift (~2.3 ms at 1M begin, instrumented).

## Alternatives considered

- GPUI as the product backend: faster on measured latency/startup/memory,
  but native Rust iteration for product logic; kept as reference oracle.
- Electron/WebView: excluded by product requirements (native behavior,
  memory, startup).

## Trade-offs

- Accepts ~1.0–1.5 ms viewport-constant Solid re-render per edit (bounded,
  addressable by the Incremental View Model) and ~230 MiB baseline
  runtime memory, for guest-side product logic and one DrawList contract
  across three platforms.
- GPUI's advantages remain documented; the decision is architecture
  control, not a benchmark win.

## Decision

PocketJS is the primary product foundation; GPUI is frozen as a
reference/performance oracle. Every measured bottleneck so far was
Markit-owned and fixed in Markit code; `vendor/pocketjs` unchanged across
A2–A4.
