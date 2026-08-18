# docs/research/ — Historical research record

This directory holds the research evidence that produced the current
Markit architecture.

## What lives here

The A0–A4 phase reports and their benchmark artifacts are **historical
research records**:

- `docs/phase-a0-windows-feasibility.md`
- `docs/phase-a1-pocketjs-windows.md`
- `docs/phase-a1-pocketjs-mvp-status.md`
- `docs/phase-a2-causal-decomposition.md`
- `docs/phase-a3-intervention-validation.md`
- `docs/phase-a4-final-research-closeout.md`
- `docs/Markit — GPUI 与 PocketJS Benchmark /mechanism-comparison.md`
- `docs/Markit_Phase0_GPUI_PocketJS_实验设计_v0.1.docx`
- `results/` raw and summary benchmark data
- `bench/run-a2.py`, `run-a3.py`, `run-a4.py`, `parse-*.py` (the A2–A4
  experiment drivers; historical tooling for the PocketJS-era battery)

## Reading rules

- A0–A4 record the experiments that led to the current architecture.
- Their measurements remain valid **within their original setup**
  (same machine, same workload contract, same instrumentation semantics).
- Their **product-foundation recommendations may be superseded**.
- Do not edit historical numbers. Evidence remains evidence.

## Product-foundation status

The A1–A4 experiments compared PocketJS and GPUI. A4 selected PocketJS as
the product foundation at that time (ADR-001).

On 2026-08-18 Markit pivoted: **Rust + direct GPUI is now the product
foundation** (see [ADR-008](../adr/ADR-008-direct-gpui-product-substrate.md)).
PocketJS is historical research context, not a Markit dependency.

The design principles the experiments established — incremental line
index, block-granular Markdown invalidation, viewport-bounded rendering,
explicit changed-range propagation, command/shortcut and IME composition
models — remain part of the architecture and are carried into the Rust
core (see `pocketjs-mvp-knowledge-transfer.md` and `docs/adr/`).
