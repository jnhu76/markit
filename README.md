# Markit

**Markit** is a low-latency, Markdown-native desktop editor built in Rust
directly on GPUI.

> Product principle: **Nothing gets between your input and the next frame.**  
> Execution principle: **Never do work the user cannot observe.**  
> Research principle: **Control → Measure → Attribute → Scale → Intervene → Generalize → Optimize.**

The execution principle means:

```text
if it did not change -> do not recompute it
if it is not visible -> defer it
if the interaction/frame budget is exhausted -> yield
if work is stale -> cancel or reject it
if derived state is incompatible -> do not publish it
```

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
foundation**.

The research-established design principles — incremental line indexing,
block-granular Markdown invalidation, viewport-bounded rendering, and
explicit changed-range propagation — are now combined with a real-time
editor execution model:

```text
change
  -> precise dirty propagation
  -> revision-aware derived work
  -> user-observable priority
  -> viewport / Document LOD
  -> coherent publication
  -> frame-budgeted, demand-driven presentation
```

The target scaling law for normal local edits is:

```text
work ~= changed semantic region + visible presentation + bounded overhead
```

rather than work proportional to the total document.

## Current Goals

1. Select and pin a GPUI baseline suitable for Windows development and
   observable demand-driven scheduling (roadmap G0).
2. Build the framework-independent Rust core (`markit-core`): Document,
   LineIndex, Selection, EditTransaction, Commands, BlockIndex, Markdown
   L1, explicit dirty/revision semantics.
3. Build the Windows editor MVP on direct GPUI with viewport-bounded,
   non-blocking presentation: window, editing, IME, clipboard, files, CJK,
   undo, large-document stability, cooperative/deferred work seams.
4. Instrument work amplification, frame work, yields, queue/stale-result
   behavior, cache invalidation, and interaction tails so the real-time
   model is testable rather than aspirational.
5. Keep the evidence-before-architecture discipline: measure before tuning
   exact budgets/data structures/worker topology, and never trade
   correctness for a benchmark.

## Real-time execution model

Markit borrows real-time techniques commonly seen in game engines and
streaming renderers, without becoming a game engine:

- dirty flags / precise dependency invalidation;
- bounded per-frame/cooperative work;
- user-observable priority;
- cancellable or stale-result-safe jobs;
- viewport culling / Document LOD;
- coherent versioned publication;
- explicit cache keys and invalidation;
- demand rendering — no permanent idle tick.

Markstream (`Simon-He95/markstream-vue`) is an explicit reference for the
**discipline** of incremental/adaptive streaming Markdown rendering. Markit
does not adopt Vue/DOM or copy Markstream's numeric frame budgets.

See [docs/product/realtime-execution-model.md](docs/product/realtime-execution-model.md).

## Non-Goals — For Now

- building a full Typora clone immediately;
- proving GPUI is the fastest UI runtime;
- implementing Linux/macOS product support before the Windows foundation
  is adequate;
- introducing a new large framework abstraction;
- building an ECS/archetype/game-engine framework;
- adding a permanent 60 Hz update loop to a demand-driven editor;
- adding plugins, AI features, or a marketplace;
- tuning worker counts, frame budgets, cache sizes, or buffer structures
  before the product workload measures the need.

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
│   ├── adr/          # evidence-backed architectural decisions
│   ├── product/      # current architecture, execution model, roadmap, MVP, invariants
│   └── research/     # historical research record (A0–A4, PocketJS-era)
├── mvp/
│   └── gpui/         # GPUI Windows feasibility prototype (gpui 0.2.2, not the product baseline)
├── workloads/        # shared benchmark corpora
├── profiles/
└── results/          # benchmark results (raw + summaries)
```

The PRD lives at [docs/PRD.md](docs/PRD.md).

The product architecture lives at
[docs/product/architecture.md](docs/product/architecture.md), with the
real-time execution model at
[docs/product/realtime-execution-model.md](docs/product/realtime-execution-model.md).

The current substrate decision is
[ADR-008](docs/adr/ADR-008-direct-gpui-product-substrate.md).

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
- p99 (when sample size supports it)
- max
- long-frame counts

For real-time execution also record, where relevant:

- changed bytes / lines / blocks;
- blocks rescanned / reparsed;
- visible / near / far materialization;
- layout / shaping work;
- frame-work duration and yields/budget overruns;
- queue depth and priority inversion;
- cancelled/stale-result counts;
- cache hit/miss and memory bounds;
- scroll drift / layout jumps.

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

Reference systems (Zed, Markstream, game engines, other editors) provide
hypotheses and vocabulary, not proof of Markit's bottleneck or tuning
parameters.

## Documentation

Start with:

- `docs/product/realtime-execution-model.md` — hot-path execution contract;
- `docs/product/architecture.md` — current product/core/platform architecture;
- `docs/product/performance-invariants.md` — testable work/scheduling invariants;
- `docs/product/roadmap.md` — implementation order and phase gates;
- `docs/product/mvp-v0.1.md` — first shippable product scope;
- `docs/PRD.md` — product/research requirements and historical adversarial audit;
- `docs/adr/ADR-008-direct-gpui-product-substrate.md` — product substrate decision;
- `docs/research/README.md` — how to read the historical A0–A4 evidence.

Do not turn unverified implementation choices into ADRs. The high-level
execution laws are product design constraints; exact worker topology,
numeric budgets, batching algorithms, buffer structures, and cache
policies remain evidence-driven.

## License

Apache License 2.0. See [LICENSE](LICENSE).
