# Markit

**Markit** is a low-latency, Markdown-native desktop editor built in Rust
directly on GPUI.

> Product principle: **Nothing gets between your input and the next frame.**  
> Research principle: **Control → Measure → Attribute → Scale → Intervene → Generalize → Optimize.**

## Project Status

Markit is in the **product foundation / GPUI architecture phase**.

The chosen product substrate is **Rust + direct GPUI** (architecture
decision ADR-008):

```text
Markit
  =
Rust editor core (markit-core)
  +
direct GPUI desktop UI/platform integration
```

Windows is the first product platform.

The project began as an evidence-driven comparison of editor
architectures, including PocketJS and GPUI (A0–A4). That research remains
in the repository as historical evidence (`docs/research/`, `docs/phase-a*`,
`results/`), but **PocketJS is no longer a Markit dependency or product
foundation**. The design principles the research established — incremental
line indexing, block-granular Markdown invalidation, viewport-bounded
rendering, explicit changed-range propagation — are carried into the Rust
core.

## Current Goals

1. Select and pin a GPUI baseline suitable for Windows development (roadmap G0).
2. Build the framework-independent Rust core (`markit-core`): Document,
   LineIndex, Selection, EditTransaction, Commands, BlockIndex, Markdown L1.
3. Build the Windows editor MVP on direct GPUI: window, editing, IME,
   clipboard, files, CJK, undo, large-document stability.
4. Keep the evidence-before-architecture discipline: measure before
   optimizing, and never trade correctness for a benchmark.

## Non-Goals — For Now

- building a full Typora clone immediately;
- proving GPUI is the fastest UI runtime;
- implementing Linux/macOS product support before the Windows foundation
  is adequate;
- introducing a new large framework abstraction;
- adding plugins, AI features, or a marketplace;
- optimizing before a bottleneck is measured.

## Repository Layout

```text
.
├── AGENTS.md
├── CONTRIBUTING.md
├── LICENSE
├── README.md
├── bench/            # benchmark harness + experiment drivers (A2–A4 historical battery)
├── docs/
│   ├── PRD.md
│   ├── adr/          # architectural decisions (ADR-008 = direct GPUI substrate)
│   ├── product/      # current architecture, roadmap, MVP, backlog, invariants
│   └── research/     # historical research record (A0–A4, PocketJS-era)
├── mvp/
│   └── gpui/         # GPUI Windows feasibility prototype (gpui 0.2.2, not the product baseline)
├── workloads/        # shared benchmark corpora
├── profiles/
└── results/          # benchmark results (raw + summaries)
```

The PRD lives at [docs/PRD.md](docs/PRD.md).

The current architecture decision is [ADR-008](docs/adr/ADR-008-direct-gpui-product-substrate.md).

The GPUI feasibility prototype lives under [mvp/gpui](mvp/gpui/README.md).

## Research Workflow

Every performance investigation should follow this order:

```text
Question
  ↓
Controlled workload
  ↓
Measurement validation
  ↓
Latency measurement
  ↓
Attribution
  ↓
Scaling experiment
  ↓
Root-cause hypothesis
  ↓
Controlled intervention
  ↓
Cross-workload / cross-platform validation
  ↓
Design principle
  ↓
Implementation
  ↓
Re-measurement
```

A flame graph is evidence for **where CPU time is spent**. It is not, by itself, proof of causality.

## Performance Metrics

Primary metric:

```text
interaction-to-present latency
```

Report distributions, not only averages:

- p50
- p95
- p99
- max
- long-frame counts

Where supported, also collect:

- CPU profiles;
- off-CPU/blocking traces;
- allocations and RSS;
- GPU/presentation timing;
- cycles / instructions / IPC;
- cache and branch behavior;
- page faults and context switches.

## Evidence Rules

Performance claims should include enough metadata to reproduce the result:

- commit SHA;
- OS and version;
- hardware;
- compiler/toolchain;
- build configuration;
- display configuration;
- corpus hash/version;
- workload version;
- profiler configuration.

Negative results are valid results.

## Documentation

Start with:

- `docs/PRD.md` — product/research requirements;
- `docs/adr/ADR-008-direct-gpui-product-substrate.md` — the current product substrate decision;
- `docs/product/` — current architecture, roadmap, MVP, invariants;
- `docs/research/README.md` — how to read the historical A0–A4 evidence.

Do not turn unverified hypotheses into ADRs.

## License

Apache License 2.0. See [LICENSE](LICENSE).
