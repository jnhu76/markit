# Documentation

Markit's documentation is split into three layers so current product
architecture does not drift into historical research notes.

```text
docs/
├── PRD.md
├── adr/                  # mature evidence-backed architectural decisions
├── product/              # current product sources of truth
│   ├── architecture.md
│   ├── realtime-execution-model.md
│   ├── performance-invariants.md
│   ├── roadmap.md
│   ├── mvp-v0.1.md
│   ├── platform-capability-matrix.md
│   └── issue-backlog.md
└── research/             # historical A0–A4 evidence / experiments
```

## Current product reading order

1. `product/realtime-execution-model.md` — how Markit keeps interaction
   incremental, viewport-bounded, non-blocking, revision-safe, and
   demand-driven.
2. `product/architecture.md` — core/platform structure and how the
   execution model fits the Markdown pipeline.
3. `product/performance-invariants.md` — testable work-amplification,
   scheduling, cache, revision, and publication invariants.
4. `product/roadmap.md` — phase order and acceptance gates; every hot-path
   phase must preserve the execution laws.
5. `product/mvp-v0.1.md` — first shippable Windows scope.
6. `adr/ADR-008-direct-gpui-product-substrate.md` — why the product uses
   direct GPUI.

`PRD.md` contains the product direction header plus the historical
adversarial research audit. `research/` and `phase-a*` documents remain
valuable evidence but do not override current product documents.

## Documentation drift rule

When a cross-cutting hot-path execution rule changes, update the affected
sources of truth together:

```text
realtime-execution-model.md
architecture.md
performance-invariants.md
roadmap.md
AGENTS.md (when contributor rules change)
mvp/feature acceptance docs (when gates change)
```

Do not let roadmap, architecture, invariants, and agent instructions
silently describe different systems.

Only create an ADR after the underlying decision is mature enough to be
treated as an architectural commitment. Exact worker topology, numeric
frame budgets, buffer structures, cache sizes, and batching policies stay
evidence-driven until measured.

Performance documents should clearly distinguish:

- observation;
- measurement;
- inference;
- causal evidence;
- design decision.
