# Existing code baseline

Status: **EXPERIMENTAL / REFERENCE**

This document records how to treat the current source tree during the Markit parser-research phase.

The purpose is to prevent two opposite mistakes:

1. assuming the old code is the new architecture because it already exists;
2. deleting/restructuring useful experimental code before Issue #19 can use it as a comparison baseline.

## Baseline revision

The exact pre-reset repository is preserved at:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

The current branch may add research/documentation markers, but the existing implementation should remain behaviorally intact until the parser experiment decides what is worth reusing.

## Current source areas

### `crates/markit-core`

Contains the existing document/change/index/Markdown experiments.

Current status:

```text
REFERENCE BASELINE FOR ISSUE #19
```

Its Markdown implementation is useful because the new parser experiment needs a concrete prior implementation to compare against.

Do not call its current data structures or parser boundaries product architecture.

Do not optimize it before the comparison corpus exists unless the change is required purely for instrumentation and is isolated from the baseline result.

### `apps/markit`

Contains the existing desktop/GPUI-facing experiment.

Current status:

```text
FROZEN UI REFERENCE
```

UI architecture is not under active development during Issue #19.

Do not use this app as evidence that GPUI is the future required backend.

### `mvp/gpui`

Historical feasibility prototype.

Current status:

```text
ARCHIVED EXPERIMENT
```

Retained only for reproducibility/reference.

### `bench/`, `results/`, `profiles/`, `tools/`

Historical experiment infrastructure and evidence.

Current status:

```text
REFERENCE / REUSE CANDIDATES
```

Useful harness patterns may be reused for Issue #19, but prior workload assumptions must not be imported without review.

New parser experiments should prefer a clearly separated harness/corpus so old UI/runtime benchmarks and new parser evidence cannot be confused.

## Change policy during Issue #19

Allowed:

- add isolated research harnesses;
- add mutation corpora;
- add adapters needed to measure existing parsers;
- add instrumentation that does not silently change the measured semantics;
- add clean full-parse correctness oracles;
- record reproducible benchmark results.

Avoid:

- production parser replacement;
- UI redesign;
- GPUI architecture work;
- broad renaming/moving of current source directories;
- performance tuning of the baseline before measurement;
- changing old parser semantics to make incremental results look better;
- deleting old implementations before their comparison role is complete.

## Post-experiment classification

After Issue #19 receives human review, each relevant component must be classified:

```text
ADOPT
ADAPT
REPLACE
DELETE
```

Examples:

```text
Document/change model         -> ?
Line/position index           -> ?
Existing Markdown parser      -> ?
Incremental invalidation      -> ?
Existing semantic structures  -> ?
GPUI app shell                -> ?
Benchmark harness pieces      -> ?
```

No answer is predetermined.

## Architecture rule

Code layout is not architecture authority during this phase.

The next architecture must be derived from:

```text
Issue #19 evidence
    -> parser direction
    -> research-question revision
    -> representation/invalidation contracts
    -> architecture
```

not from the fact that a Rust module or dependency already exists in the repository.
