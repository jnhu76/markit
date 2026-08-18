# AGENTS.md

This file defines working rules for humans and AI coding agents contributing to Markit.

## 1. Mission

Markit is an evidence-driven low-latency Markdown editor project, built
in Rust directly on GPUI.

The project is not primarily about implementing features quickly. It is about discovering which work actually causes editor latency, deriving reusable system principles, and applying them to the Markit editor core and its GPUI integration.

Core principles:

> **Nothing gets between your input and the next frame.**

> **Control → Measure → Attribute → Scale → Intervene → Generalize → Optimize.**

## 2. Current Phase

Assume the project is in the **product foundation / GPUI architecture phase**.

The substrate decision is made (direct GPUI, ADR-008); the editor
architecture is not. Do not prematurely implement a full editor
architecture.

Do not assume that:

- Rope is the correct buffer;
- Piece Tree is the correct buffer;
- Tree-sitter is required;
- incremental parsing is the dominant optimization;
- viewport virtualization is the dominant optimization;
- Rust/native code is automatically faster;
- JavaScript/native FFI is a bottleneck;
- Electron is inherently the problem;
- GPUI's current prototype version is the product baseline.

Treat these as hypotheses until measured.

## 3. Evidence Before Architecture

Before proposing a performance-driven architectural change, provide:

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
   ```

6. Editor policy belongs in Markit:

   ```text
   document
   selection
   commands
   undo
   Markdown
   incremental invalidation
   viewport model
   ```

7. GPUI-specific code should not leak unnecessarily into editor
   algorithms.

8. Zed is a REFERENCE IMPLEMENTATION, not proof.

   Do not write: "Zed does X, therefore Markit must do X."

Maintain the current evidence-before-architecture discipline: a change to
the GPUI integration must correspond to a measured bottleneck, and
baseline selection (roadmap G0) requires dedicated validation, not the
assumption that "newer is better".

## 11. Reference Host

A deterministic/headless host may be created for:

- correctness;
- algorithmic scaling;
- controlled interventions;
- reproducible core tests.

It is not evidence of real desktop interaction latency.

Real OS hosts are required for claims involving:

- input delivery;
- IME;
- fonts;
- scheduling;
- compositor behavior;
- GPU/presentation.

## 12. Correctness

Never trade away correctness to win a benchmark.

Do not silently disable:

- Unicode correctness;
- IME behavior;
- Markdown semantics;
- font fallback;
- required rendering behavior;

unless the experiment explicitly describes that subsystem as the intervention.

Such experiments are diagnostic, not product benchmarks.

## 13. Code Changes

Keep patches small and hypothesis-driven.

Prefer:

- instrumentation before optimization;
- replaceable subsystems;
- stable benchmark seams;
- explicit changed-range propagation;
- measurable invariants.

Avoid speculative framework-wide rewrites.

## 14. Documentation Rules

Place the PRD in `docs/`.

Place architectural decisions in `docs/adr/` only after sufficient evidence exists.

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

## 15. Definition of Done for Performance Work

A performance task is not done because:

- a microbenchmark improved;
- a flame graph looks narrower;
- CPU usage fell;
- code became more “native”.

It is done when the relevant end-to-end metric is re-measured and the result is documented.

## 16. When Unsure

Prefer:

```text
instrument → measure → compare
```

over:

```text
rewrite → hope → benchmark
```
