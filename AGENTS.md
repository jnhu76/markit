# AGENTS.md

This file defines working rules for humans and AI coding agents contributing to Markit.

## 1. Mission

Markit is an evidence-driven low-latency Markdown editor project, built
in Rust directly on GPUI.

The project is not primarily about implementing features quickly. It is about discovering which work actually causes editor latency, deriving reusable system principles, and applying them to the Markit editor core and its GPUI integration.

Core principles:

> **Nothing gets between your input and the next frame.**

> **Never do work the user cannot observe.**

> **Control → Measure → Attribute → Scale → Intervene → Generalize → Optimize.**

The second rule expands to:

```text
if it did not change -> do not recompute it
if it is not visible -> defer it
if the interaction/frame budget is exhausted -> yield
if work is stale -> cancel or reject it
if derived state is incompatible -> do not publish it
```

## 2. Current Phase

Assume the project is in the **product foundation / GPUI architecture phase**.

The substrate decision is made (direct GPUI, ADR-008). The high-level
editor execution laws are also fixed by
`docs/product/realtime-execution-model.md` and the accepted incremental /
viewport ADRs:

```text
explicit changed-range propagation
smallest semantically valid invalidation
viewport-bounded presentation
priority by user observability
bounded/cooperative deferrable work
revision-safe cancellation / stale-result rejection
coherent publication
precise cache invalidation
no permanent idle update loop
```

What is **not** fixed yet is the detailed implementation: buffer data
structure, worker topology, numeric frame budget, batching policy, cache
sizes, scheduler data structure, or rich-block machinery.

Do not prematurely implement a large editor/game-engine framework.

Do not assume that:

- Rope is the correct buffer;
- Piece Tree is the correct buffer;
- Tree-sitter is required;
- incremental parsing is the dominant bottleneck just because incremental
  invalidation is an architectural invariant;
- viewport virtualization is the dominant bottleneck just because
  viewport-bounded presentation is required;
- a thread pool is always better than cooperative work;
- a fixed 6 ms / 8 ms frame budget copied from another project is correct;
- ECS/archetypes are useful for Markit;
- Rust/native code is automatically faster;
- JavaScript/native FFI is a bottleneck;
- Electron is inherently the problem;
- GPUI's current prototype version is the product baseline.

Treat those as hypotheses until measured.

## 3. Evidence Before Architecture

Before proposing a performance-driven architectural change beyond the
accepted execution laws, provide:

1. the workload that exposes the problem;
2. the end-to-end metric that regresses;
3. attribution evidence;
4. scaling behavior;
5. a root-cause hypothesis;
6. an intervention or differential experiment;
7. before/after measurements;
8. correctness and complexity trade-offs.

Do not justify a rewrite with phrases such as:

- “this should be faster”;
- “native is faster”;
- “Zed does it this way”;
- “Markstream does it this way”;
- “game engines do it this way”;
- “modern editors use this data structure”;
- “this avoids overhead in theory”.

External systems are references, not proofs.

## 4. Performance Investigation Order

Use this order:

```text
Define semantic workload
        ↓
Control variables
        ↓
Validate measurement
        ↓
Measure end-to-end latency
        ↓
Attribute CPU / off-CPU / GPU time
        ↓
Study scaling
        ↓
Form root-cause hypothesis
        ↓
Intervene
        ↓
Re-measure
        ↓
Validate on another workload/platform when relevant
        ↓
Extract design principle
```

Do not skip directly from flame graph to optimization.

## 5. Benchmark Integrity

Keep latency runs and profiling runs conceptually separate.

Profilers and tracing can change timing.

For benchmark changes:

- use optimized/release builds;
- retain symbols where profiling needs them;
- record toolchain and commit SHA;
- record corpus/workload version;
- record hardware and OS;
- record display configuration;
- record profiler configuration;
- quantify profiler overhead where possible.

Never compare timestamps with different semantics as if they were equivalent.

For presentation timing, distinguish where possible:

```text
input
CPU frame preparation
GPU submission
presentation API completion
compositor/display visibility
```

If a platform only provides a proxy, label it as a proxy.

## 6. Workload Equivalence

Two editors are comparable only if they perform semantically comparable work.

Do not compare:

- plain-text editing in one system;
- live Markdown projection + spellcheck + plugins in another;

and call the difference “framework performance”.

When adding cross-system benchmarks, document the semantic workload level.

Suggested levels:

```text
L0 plain text edit
L1 Markdown parse
L2 syntax/projection update
L3 user-visible Markdown editing
L4 realistic default product configuration
```

## 7. Controlled Scaling

Do not use only arbitrary real-world documents to infer complexity.

Maintain controlled corpus families that independently vary:

- total bytes `N`;
- block count `B`;
- line length `L`;
- visible region `V`;
- changed region `Δ`;
- Markdown structural density.

A local edit whose cost grows with total document size is a strong signal, not an automatic diagnosis.

For normal local edits, Markit's target law is:

```text
work ~= Δ semantic region + V presentation + bounded overhead
```

If a normal edit starts scaling with `N`, instrument the fan-out before
changing data structures.

## 8. Unicode Policy

Unicode correctness is an architectural requirement, but Unicode complexity is introduced gradually in performance research.

Suggested ladder:

```text
U0 basic Latin / ASCII
U1 simplified Chinese
U2 mixed CJK / Latin
U3 combining marks / basic emoji
U4 fallback / ZWJ
U5 complex scripts / bidi
```

Early performance experiments may use U0.

However, production-facing core APIs must not rely on ASCII-only assumptions.

Avoid ambiguous `charOffset`-style APIs.

Keep room to distinguish:

- bytes;
- Unicode scalars;
- grapheme boundaries;
- logical positions;
- display positions;
- platform UTF-16 coordinates when required.

## 9. Multiplatform Architecture

Follow a KMP-like design philosophy, not Kotlin-specific implementation rules:

> share policy/state where possible; isolate platform mechanisms where necessary.

Prefer explicit interfaces/traits for platform services such as:

- text shaping;
- font resolution;
- IME;
- clipboard;
- input/window integration;
- presentation;
- filesystem;
- clocks;
- profiler hooks.

Do not force all platforms into a lowest-common-denominator implementation.

Platform-specific fast paths are allowed when the common semantic contract remains intact.

Replaceability is valuable because it supports both portability and causal experiments.

## 10. GPUI Product Policy

GPUI is the chosen UI/platform substrate (ADR-008). Product work must
follow these rules:

1. GPUI is the chosen UI/platform substrate.

2. `markit-core` must remain independent from GPUI wherever this is
   semantically useful.

3. Do not put document ownership into the GPUI element tree.

4. Do not use GPUI entities as the canonical Markdown document model.

5. Platform integration belongs at the GPUI/platform edge:

   ```text
   window
   IME
   clipboard
   keyboard
   pointer
   native text
   file dialogs
   presentation
   frame request / platform timing hooks
   ```

6. Editor policy belongs in Markit:

   ```text
   document
   selection
   commands
   undo
   Markdown
   incremental invalidation
   dirty/revision model
   viewport / LOD model
   scheduling priority semantics
   coherent publication rules
   ```

7. GPUI-specific code should not leak unnecessarily into editor
   algorithms.

8. Zed is a REFERENCE IMPLEMENTATION, not proof.

9. Markstream is a REFERENCE for streaming scheduling discipline, not a
   dependency and not proof that its batch sizes/budgets are correct for
   Markit.

Maintain the current evidence-before-architecture discipline: a change to
the GPUI integration must correspond to a measured bottleneck, and
baseline selection (roadmap G0) requires dedicated validation, not the
assumption that "newer is better".

## 11. Real-time Editor Execution Rules

Read `docs/product/realtime-execution-model.md` before changing any editor
hot path.

### 11.1 Dirty propagation

Do not use one undifferentiated "document changed" path when downstream
work differs semantically.

Preserve enough information to distinguish changes such as:

```text
Append
LocalEdit
Delete
Paste
StructuralEdit
ReplaceDocument
ViewportMove
Theme/StyleChange
```

The concrete Rust enum/flags are implementation choices; precise
invalidation is not.

### 11.2 User-observable priority

Use this semantic order unless a correctness dependency requires otherwise:

```text
current interaction
  > visible viewport
  > near viewport / likely next interaction
  > distant/background work
```

FIFO completion is not a product goal.

### 11.3 Bounded/cooperative work

Deferrable work must be chunkable, resumable, or moved off the critical
UI path. When the calibrated interaction/frame budget is exhausted, yield.

Do not copy a numeric budget from Markstream, a game engine, Zed, or a
blog. Calibrate it on real Markit Windows/GPUI presentation evidence.

### 11.4 Revision-safe jobs

Every deferred/background result that can race with edits must be able to
prove that it is valid for the current revision/dependencies.

When newer work supersedes old work:

```text
reuse if proven compatible
otherwise cancel if worthwhile
otherwise reject result at commit
```

Never allow stale output to overwrite a newer presentation.

### 11.5 Coherent publication

The visible frame may reuse older artifacts only when compatible. Do not
silently publish mixtures such as:

```text
AST v103 + layout v102 + highlight v099
```

unless compatibility is explicitly proven.

"Snapshot" means a coherent version boundary; it does not mean copying the
whole document.

### 11.6 Viewport / Document LOD

Derived-state materialization follows observability. Far content may keep
only lightweight metadata/extent; near content may be prefetched; visible
content gets exact layout/shaping; presented content gets render state.

The concrete representation is open. Scroll drift and layout jumps are
correctness/performance metrics, not acceptable hidden costs.

### 11.7 Cache discipline

A cache must define:

- key;
- dependencies;
- invalidation;
- revision compatibility;
- memory bound / eviction;
- instrumentation seam.

Do not add an opaque cache whose correctness depends on "usually stale is
fine".

### 11.8 Game-engine analogy limits

Borrow:

```text
dirty flags
frame budgets
job priority/cancellation
visibility culling
LOD
coherent frame publication
caches
```

Do not introduce by default:

```text
ECS
archetypes
a permanent 60 Hz tick
a scene graph as canonical document state
a generic engine framework
```

Markit is demand-driven: idle means almost no work.

## 12. Reference Host

A deterministic/headless host may be created for:

- correctness;
- algorithmic scaling;
- dirty propagation;
- revision/cancellation ordering;
- controlled interventions;
- reproducible core tests.

It is not evidence of real desktop interaction latency.

Real OS hosts are required for claims involving:

- input delivery;
- IME;
- fonts;
- scheduling;
- compositor behavior;
- GPU/presentation;
- numeric frame-budget calibration.

## 13. Correctness

Never trade away correctness to win a benchmark.

Do not silently disable:

- Unicode correctness;
- IME behavior;
- Markdown semantics;
- font fallback;
- required rendering behavior;
- revision compatibility;
- scroll/layout stability;

unless the experiment explicitly describes that subsystem as the intervention.

Such experiments are diagnostic, not product benchmarks.

## 14. Code Changes

Keep patches small and hypothesis-driven.

Prefer:

- instrumentation before optimization;
- replaceable subsystems;
- stable benchmark seams;
- explicit changed-range propagation;
- explicit revision/dependency identity;
- measurable invariants;
- bounded work queues;
- stale-result tests;
- demand-driven frame requests.

Avoid speculative framework-wide rewrites.

Before adding a new hot-path feature, answer:

```text
what becomes dirty?
what is the invalidation radius?
what must finish before the next visible frame?
what can be deferred?
what makes old work stale?
what is cached/reused?
how is the result measured?
```

## 15. Documentation Rules

Place the PRD in `docs/`.

Place architectural decisions in `docs/adr/` only after sufficient evidence exists.

Current product design lives in `docs/product/`. When a cross-cutting
hot-path execution rule changes, update **all affected sources of truth in
the same change**:

```text
docs/product/realtime-execution-model.md
+ docs/product/architecture.md
+ docs/product/performance-invariants.md
+ docs/product/roadmap.md
+ AGENTS.md when contributor rules change
+ MVP/feature acceptance docs when gates change
```

Do not allow architecture, roadmap, invariants, and agent instructions to
describe different execution models.

For performance findings, record:

- question;
- setup;
- workload;
- measurement;
- profile;
- scaling;
- intervention;
- conclusion;
- limitations.

Distinguish observation from inference.

## 16. Definition of Done for Performance Work

A performance task is not done because:

- a microbenchmark improved;
- a flame graph looks narrower;
- CPU usage fell;
- code became more “native”;
- all queued background work completed sooner.

It is done when the relevant end-to-end metric is re-measured and the result is documented.

For real-time scheduling work, also verify that the intervention did not
merely move cost into:

```text
long frames
stale work
queue growth
scroll drift
cache memory
revision races
```

## 17. When Unsure

Prefer:

```text
instrument → measure → compare
```

over:

```text
rewrite → hope → benchmark
```
