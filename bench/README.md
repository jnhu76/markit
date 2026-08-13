# Markit Performance Lab

This directory will contain the reproducible benchmark harness, corpora, workload definitions, and experiment metadata.

## Research Sequence

```text
Control
→ Measure
→ Attribute
→ Scale
→ Intervene
→ Generalize
→ Optimize
```

## Initial Baseline

Start deliberately small:

- one OS;
- one hardware configuration;
- one font;
- one window/display setup;
- U0 ASCII/basic Latin;
- deterministic input;
- controlled document-size scaling.

Only add new dimensions after the previous experiment is understood.

## Planned Corpus Axes

Control independently where practical:

```text
N  total bytes
B  block count
L  line/paragraph length
V  visible region
Δ  changed region
U  Unicode complexity
```

## Unicode Ladder

```text
U0 basic Latin / ASCII
U1 simplified Chinese
U2 mixed CJK / Latin
U3 combining marks / basic emoji
U4 fallback / ZWJ
U5 complex scripts / bidi
```

## Workload Levels

Cross-system comparisons should record semantic equivalence.

Suggested levels:

```text
L0 plain text edit
L1 Markdown parse
L2 syntax/projection update
L3 user-visible Markdown editing
L4 realistic product configuration
```

## Core Workloads

Planned workload families:

- open;
- typing;
- delete;
- paste;
- scrolling;
- resize/reflow;
- structural Markdown edits;
- selection;
- IME after Unicode/IME expansion.

## Metrics

Primary:

```text
interaction-to-present latency
```

Report:

- p50;
- p95;
- p99;
- max;
- long-frame counts.

Where useful also collect:

- CPU sampling profiles;
- off-CPU traces;
- allocation/RSS;
- GPU/presentation traces;
- PMU counters.

## Important

Profiling runs are primarily for attribution.

Uninstrumented or minimally instrumented runs should be used for final latency comparisons when profiling overhead is material.
