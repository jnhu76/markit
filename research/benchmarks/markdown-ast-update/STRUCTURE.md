# Benchmark workspace structure

This file defines directory ownership for `research/benchmarks/markdown-ast-update/`.
It is an information-architecture contract, not a benchmark-result document.

## Principle

Preserve frozen R0-R5 paths. New #31 work follows a one-way research-data
lifecycle:

```text
frozen protocol + inputs
        ↓
project corpus + deterministic traces
        ↓
measurement runner / H0-H4
        ↓
raw observations
        ↓
derived summaries
        ↓
attribution / scaling analysis
        ↓
reviewed report / Weakness Map
```

A later layer may reference an earlier layer; it must not silently rewrite it.

## Directory ownership

### Frozen authority and controlled inputs

`protocol/`
: Methodology, contracts, stage records, and gate evidence. Historical stage
  files keep their existing paths.

`prior-art/`
: Source/provenance/fidelity records for mechanism extraction.

`grammar/`
: BENCH-GRAMMAR-v1 and normalized-result semantics.

`corpus/`
: Frozen synthetic corpus definitions and receipts. Synthetic corpora are
  controlled explanatory workloads, not the primary real-project headline.

`mutations/`
: Frozen mutation families / structural edit definitions.

`cases/`
: Frozen synthetic case matrices.

`manifest/`
: Shared environment/schema/version manifests where already defined.

### Executable substrate

`common/`
: Shared source/edit/case/status/work contracts. No horse-specific policy.

`instrumentation/`
: Measurement/instrumentation plumbing. Timing overhead and work attribution
  remain separable.

`oracle/`
: Normalization, validation, checksum, correctness authority.

`runner/`
: Process isolation, timer lanes, sampling/order execution, result emission.

`corpusgen/`
: Deterministic synthetic corpus generation.

`shared-grammar/`
: Shared BENCH-GRAMMAR-v1 parsing primitives whose sharing does not erase a
  mechanism-specific cost.

`mechanisms/`
: H0-H4 mechanism-owned state and policy plus the R1 null mechanism.

`scripts/`
: Verification, campaign orchestration, and reproducibility entrypoints.

### #31 real-project performance lifecycle

`projects/`
: Immutable project pins plus BENCH-GRAMMAR-v1 eligibility and workload
  profiles. Do not put benchmark timing here.

`traces/`
: Deterministic canonical edit traces derived from project structure. No timing
  or horse-specific selection is allowed here.

`results/`
: Machine-readable measurement outputs. `raw` is append-only evidence;
  `summaries` is derived. Human interpretation does not belong here.

`analysis/`
: Scripts and records that join timing/work lanes, analyze scaling/crossover,
  test attribution hypotheses, and perform replication/optimization
  sensitivity. Analysis must name its input result manifest/hash.

`report/`
: Reviewed human-facing findings: project tables, figures, Weakness Map, and
  eventual paper-like narrative. Reports cite result/analysis artifacts and
  never replace them as evidence.

## Naming and identity

Real-project measurement identity must carry enough frozen information to
reproduce the row, at minimum:

```text
project_id + project_commit_sha
file identity / source digest
trace identity / canonical edit
horse id
lane
session
iteration
runner commit
build profile
machine/environment manifest
```

Reuse stable CaseId/trace identity where the R1 contract already defines it.
Do not encode runtime timing values into identity.

## Raw-data rule

Raw observations are immutable evidence. Corrections create a new campaign or
superseding manifest; they do not edit historical rows in place.

Derived CSV/JSON summaries must be reproducible from raw rows by a checked-in
analysis command.

## What not to do

- Do not move R0-R5 frozen files merely to make the tree prettier.
- Do not place third-party project checkouts inside mechanism crates.
- Do not place timing numbers in `projects/` or `traces/`.
- Do not hand-edit generated summary tables to make report numbers nicer.
- Do not treat `report/` as the only copy of evidence.
- Do not add horse-specific project or edit selection.
- Do not start Markit production parser code inside this workspace.

## Current execution order

```text
#31 P0  projects/ + traces policy freeze
#31 P1  full parse/state construction -> results/
#31 P2  real-project edit measurements -> results/
#31 P3  mechanism attribution -> analysis/
#31 P4  controlled synthetic explanations -> analysis/
#31 P5  replication + optimization sensitivity -> analysis/ + report/
```
