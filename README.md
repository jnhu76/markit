# Markit

**Markit** is an evidence-driven project exploring how to build a low-latency, Markdown-native desktop editor.

> Product principle: **Nothing gets between your input and the next frame.**  
> Research principle: **Control → Measure → Attribute → Scale → Intervene → Generalize → Optimize.**

## Project Status

Markit is currently in the **research and measurement phase**.

The project does **not** yet assume that any particular UI framework, text buffer, parser, layout system, renderer, or platform abstraction is the correct solution.

PocketJS is the intended product direction, but architecture changes must be justified by reproducible evidence.

## Current Goals

1. Build a reproducible editor performance lab.
2. Measure real interaction latency across representative editor architectures.
3. Locate bottlenecks using timeline traces, flame graphs, off-CPU analysis, allocation profiling, GPU/presentation traces, and hardware counters where useful.
4. Study scaling behavior with document size, changed region, visible region, line length, and block count.
5. Validate root causes with controlled interventions.
6. Compare how existing systems address verified bottlenecks.
7. Use the resulting design principles to evolve PocketJS and build Markit.

## Non-Goals — For Now

The early project is **not** trying to:

- build a full Typora clone immediately;
- prove PocketJS is the fastest UI runtime;
- prove Electron or JavaScript is inherently slow;
- create a GUI framework benchmark leaderboard;
- build a plugin marketplace;
- add AI features;
- implement every Unicode edge case in the first experiment;
- optimize before a bottleneck is measured.

## Repository Layout

```text
.
├── AGENTS.md
├── CONTRIBUTING.md
├── LICENSE
├── README.md
├── .editorconfig
├── .gitignore
├── bench/
│   └── README.md
├── docs/
│   ├── PRD.md
│   ├── README.md
│   └── adr/
│       └── README.md
├── profiles/
│   └── .gitkeep
└── results/
    ├── raw/
    │   └── .gitkeep
    └── summary/
        └── .gitkeep
```

The PRD lives at [docs/PRD.md](docs/PRD.md).

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

## Initial Experimental Scope

The first controlled baseline should deliberately stay simple:

- one platform;
- optimized/release builds with symbols;
- fixed hardware;
- fixed window and display configuration;
- fixed font and font size;
- ASCII/basic Latin Markdown;
- deterministic input sequence;
- 10 KB → 100 KB → 1 MB → 10 MB scaling families.

Unicode and platform complexity should be introduced incrementally after the baseline methodology is validated.

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

Examples:

- parser is not the bottleneck;
- FFI is not material;
- viewport virtualization does not help a workload;
- Rope does not improve end-to-end latency;
- GPU is not limiting.

## Documentation

Start with:

- `docs/PRD.md` — product/research requirements;
- `docs/adr/` — architectural decisions after evidence exists;
- `bench/README.md` — benchmark rules and workload contracts.

Do not turn unverified hypotheses into ADRs.

## License

Apache License 2.0. See [LICENSE](LICENSE).
