# Markit documentation

Markit is currently in a **Markdown parser research phase**.

The previous research/architecture/ADR set is historical evidence. It must not be read as the current implementation plan.

## Active documents

```text
docs/
├── PRD.md                              # product requirements
├── README.md                           # this authority map
├── product/
│   ├── architecture.md                 # HOLD: invariants only; architecture not frozen
│   ├── mvp-v0.1.md                     # intended product scope
│   ├── print-browser-contract.md       # print/browser completeness contract
│   └── roadmap.md                      # current sequencing/status
├── research/
│   └── markdown-parser/README.md       # current parser research north star
└── archive/
    └── product-reset-2026-09-16/README.md
```

The active external research campaign is:

- Issue #19 — `Incremental Markdown parsing: locality, convergence, and invalidation`.

## Current authority order

```text
docs/PRD.md
    = what product we are trying to build

Issue #19 + generated experimental evidence
    = current Markdown parsing research authority

docs/product/architecture.md
    = HOLD / non-negotiable invariants only

docs/product/print-browser-contract.md
    = output completeness contract

docs/product/mvp-v0.1.md
    = intended V0.1 scope

docs/product/roadmap.md
    = work ordering
```

`architecture.md` is intentionally not a complete architecture. A real implementation architecture must wait for the Issue #19 parser verdict.

## Historical material

The complete pre-reset state is preserved at Git revision:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

That revision is the archive for:

- old ADRs;
- GPUI/PocketJS substrate decisions;
- A0-A4 experiments;
- previous performance/realtime execution model;
- old plugin compatibility design;
- old Markdown L1 semantic contract;
- P0-01/P0-02 implementation notes;
- previous issue backlog/platform matrix;
- old product architecture and roadmap;
- the pre-reset implementation itself.

Historical documents may be cited as evidence, but they do not regain authority unless a new evidence-backed decision explicitly re-adopts them.

## Current technical question

The provisional research north star is:

> **For lossless Markdown editing under arbitrary edits, how can Markit minimize reparse radius, tree reconstruction, memory movement, and downstream render invalidation while preserving correctness?**

Issue #19 must also decide whether that question should be `KEEP`, `REFINE`, or `REPLACE` after experimentation.

## Rule for new documents

During the parser campaign, avoid writing speculative architecture documents for UI, rendering, storage, CST/AST layout, scheduler topology, or plugin runtime.

Create a durable architectural document only when its decision has evidence and a clear authority boundary.

Until then, prefer:

```text
question -> experiment -> evidence -> verdict -> architecture
```

not:

```text
architecture -> implementation -> benchmark justification
```
