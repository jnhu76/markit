# Contributing to Markit

Markit is currently research-heavy. Contributions should optimize for **reproducibility and evidence**, not only implementation speed.

## Before Opening a Performance PR

Please include:

- the problem being investigated;
- the workload;
- the environment;
- baseline measurements;
- profiling or attribution evidence;
- scaling behavior when relevant;
- the proposed mechanism;
- before/after measurements;
- correctness impact;
- known limitations.

## Performance PR Template

```text
Problem:
Workload:
Environment:
Baseline:
Attribution:
Scaling:
Hypothesis:
Intervention:
Change:
Before:
After:
Correctness:
Trade-offs:
Limitations:
```

## Benchmark Changes

Benchmark changes should be reviewed as carefully as product code.

Changing:

- corpus;
- workload semantics;
- timing boundaries;
- profiler configuration;
- warm-up;
- cache state;

can invalidate historical comparisons.

Version benchmark definitions when semantics change.

## Architecture Changes

Architecture decisions should reference measured evidence.

If a decision is not yet evidence-backed, keep it as a hypothesis or experiment instead of an ADR.

## Scope

During the research phase, avoid unrelated feature expansion.

Especially defer:

- plugins;
- AI features;
- sync;
- collaboration;
- IDE features;
- broad UI polish.

The immediate goal is to understand and reduce interaction latency without destroying correctness.
