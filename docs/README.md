# Markit documentation

Markit is in a **benchmark-first Markdown parser research phase**. There is no
production implementation in this repository and no frozen Markdown
architecture.

## Active documents

```text
docs/
├── PRD.md                              # product requirements (top product authority)
├── README.md                           # this authority map
├── product/
│   ├── architecture.md                 # HOLD: invariants only; architecture not frozen
│   ├── mvp-v0.1.md                     # intended product scope
│   ├── print-browser-contract.md       # print/browser completeness contract
│   └── roadmap.md                      # current sequencing/status
├── research/
│   ├── README.md                       # research campaign status map
│   └── repo-reset-inventory.md         # record of the experiment-first repo reset
└── archive/
    └── product-reset-2026-09-16/README.md
```

Research artifacts live outside `docs/`:

```text
research/experiments/experiment-0-parser-survey/   archived Experiment 0 (#19/#20)
research/benchmarks/markdown-ast-update/           active #22 benchmark area
```

## Current authority order

```text
docs/PRD.md + docs/product/**
    = what product we are trying to build (no implementation authority)

Issue #22 — MARKIT-MARKDOWN-BENCHMARK-1
    = the ONLY active Markdown parser research authority

research/experiments/experiment-0-parser-survey/
    = ARCHIVED Experiment 0 (#19 / PR #20) — historical evidence only

Issue #21 — MARKIT-MARKDOWN-ARCHITECTURE-1
    = SUPERSEDED / CLOSED — not a gate for anything

docs/product/architecture.md
    = HOLD / non-negotiable invariants only

docs/product/print-browser-contract.md
    = output completeness contract

docs/product/mvp-v0.1.md
    = intended V0.1 scope

docs/product/roadmap.md
    = work ordering
```

## Current research state

```text
Product requirements       = docs/product/**
Experiment 0 (#19/#20)     = archived historical evidence
Active research            = #22 standardized Markdown AST/CST update benchmark
Markit parser algorithm    = NOT YET DEFINED
Formal model               = NOT YET DEFINED
Architecture               = HOLD
Production implementation  = BLOCKED
```

`architecture.md` intentionally does not define the implementation
architecture. Its HOLD lifts only after the #22 chain completes:

```text
#22 benchmark → Weakness Map review
             → Markit algorithm campaign
             → formal/correctness review (as applicable)
             → architecture synthesis
```

## Historical material

The complete pre-reset state is preserved at Git revision:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

The post-reset implementation trees (apps, crates, mvp, bench, tools,
workloads, profiles, old results) were removed from the active tree by
MARKIT-EXPERIMENT-FIRST-REPO-RESET-1; Git history before that reset is their
archive. See `docs/research/repo-reset-inventory.md` for the full disposition
record.

Historical documents and code may be cited as evidence, but they do not regain
authority unless a new evidence-backed decision explicitly re-adopts them.

## Rule for new documents

Avoid writing speculative architecture documents for UI, rendering, storage,
CST/AST layout, scheduler topology, or plugin runtime.

Create a durable architectural document only when its decision has evidence
and a clear authority boundary. Until then:

```text
question -> experiment -> evidence -> verdict -> architecture
```

not:

```text
architecture -> implementation -> benchmark justification
```
