# Benchmark workspace structure

This file defines directory ownership for `research/benchmarks/markdown-ast-update/`.
It is an information-architecture contract, not a benchmark-result document.

## Principle

Preserve frozen R0-R5 paths. Post-R5 #31 work follows the explicit execution
refinement in `protocol/R6-PROJECT-PERFORMANCE-EXECUTION-AMENDMENT.md` and a
one-way research-data lifecycle:

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
later reviewed report / Weakness Map
```

A later layer may reference an earlier layer; it must not silently rewrite it.

## Directory ownership

### Frozen authority and controlled inputs

`protocol/`
: Methodology, explicit amendments, contracts, stage records, and gate evidence.
  Historical stage files keep their existing paths.

`prior-art/`
: Source/provenance/fidelity records for mechanism extraction.

`grammar/`
: BENCH-GRAMMAR-v1 and normalized-result semantics, plus the CORRECTIVE-A
  grammar-lane registry (`GRAMMAR-LANES-v1.md`, generated
  `grammar-lanes-v1.json`) and the transition-oracle contract
  (`TRANSITION-ORACLE-v1.md`).

`workloads/`
: Stage-A real-Markdown workload construction substrate (#35). Owned layers:
  acquisition provenance/lock/inventory (`sources/`, `licenses/`,
  `acquisition-config.json`, `tools/`, `manifests/` — merged acquisition
  authority from PR #34); profiler contract and schemas (`profiles/`);
  payload lifecycle contract and schemas (`payloads/`);
  contract-validation pilot (`pilots/`); deferred attribution contracts
  (`attribution/`); the CORRECTIVE-A adversarial review record
  (`REVIEW-CORRECTIVE-A-ADVERSARIAL-v1.md`); CORRECTIVE-B analysis
  artifacts (`analysis/`: domain strata input, materialization
  verification, eligibility bias, redundancy, uncovered space) and
  selection artifacts (`selections/`: frozen contract + config, the four
  logical sets, selection trace, selected files, coverage report); the
  real-vs-scaling workload boundary contract
  (`PERFORMANCE-SURFACE-BOUNDARY-v1.md`: surfaces R/S/X, N/B/L/K/F
  definitions, PERFORMANCE_MATRIX_FREEZE reservation).
  `sources/` stores byte-exact upstream Markdown snapshots pinned by
  immutable commit SHAs with per-file hashes; `manifests/` records
  candidate-universe membership and provenance; `_cache/` is a
  rebuildable, gitignored acquisition cache that is not benchmark
  authority; universe-scale derived data (`real-profile-v1.jsonl`,
  `candidate-rows-v1.jsonl`) is gitignored and hash-bound the same way.
  No timing, no final edit payloads, and no mechanism facts belong here.
  See `workloads/README.md` and `workloads/ACQUISITION-REPORT-1.md`.

`corpus/`
: Frozen synthetic corpus definitions and receipts. Under the R6 execution
  amendment, these are the controlled explanatory/scaling surface after
  real-project observations expose a candidate dimension.

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

`semantics/`
: CORRECTIVE-A semantic substrate: grammar-lane registry, REAL-MARKDOWN
  profiler, transition oracle, payload lifecycle, and the semantic pilot
  driver. Depends on `common/`, `oracle/`, and `shared-grammar/`; it must not
  depend on `mechanisms/`, `instrumentation/`, or `runner/`, and it contains
  no mechanism, timing, or selection policy.

`profile-select/`
: CORRECTIVE-B candidate-universe profiling and deterministic workload
  selection: materialization verification, universe profiling under the
  frozen lanes, distributions/eligibility-bias/redundancy analysis, the
  four logical set selectors with a complete trace, coverage retention
  and validation spot checks. Depends on `semantics/` only (plus
  serde/sha2); it must never depend on `mechanisms/`, `instrumentation/`,
  or `runner/`, emits no timing/work/reuse fact, and produces no final
  edit payloads (CORRECTIVE-C owns those).

`scripts/`
: Verification, campaign orchestration, and reproducibility entrypoints.

### #31 real-project performance lifecycle

`projects/`
: Project pins plus BENCH-GRAMMAR-v1 eligibility and workload profiles. Once a
  project manifest is frozen for a campaign, its SHA/eligibility inputs are
  immutable for that campaign. Do not put benchmark timing here.

`traces/`
: Deterministic canonical edit traces derived from project structure. No timing
  or horse-specific selection is allowed here.

`results/`
: Machine-readable measurement outputs. `raw` is immutable evidence;
  `summaries` is derived; `manifests` pins campaign identity. Human
  interpretation does not belong here.

`analysis/`
: Scripts and records that join timing/work lanes, analyze scaling/crossover,
  test attribution hypotheses, and perform optimization sensitivity. Analysis
  must name its input result manifest/hash. Generated figures belong here.

`report/`
: Reserved for later reviewed synthesis under the final ROADMAP report stage.
  It may consume reviewed result/analysis artifacts but does not own raw
  evidence and is not the place for tentative #31 observations.

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
- Do not let #31 issue text silently override frozen protocol; protocol changes
  require an explicit amendment under `protocol/`.

## Current execution order

```text
#35 CORRECTIVE-A  semantic substrate (lanes/profiler/oracle/lifecycle)  <- NOW
#35 CORRECTIVE-B  profile all candidates, publish bias, select sets
#35 CORRECTIVE-C  payload freeze, BREAK/RESTORE, traces, harness dry-run
#31 P0  project reconnaissance + eligibility/trace policy freeze
#31 P1  R6 full parse/state construction -> results/
#31 P2  R7/R8 real-project edit measurements -> results/
#31 P3  R11 mechanism attribution -> analysis/
#31 P4  R10 targeted synthetic scaling explanations -> analysis/
#31 P5  R11 optimization-sensitivity / residual attribution -> analysis/
```

CORRECTIVE-A grants no freeze: `REAL_WORKLOAD_FREEZE_PASS`,
`CORE_REAL_WORKLOAD_FREEZE_PASS`, and `PROJECT_CORPUS_FREEZE_PASS` remain
ungranted, and no H0-H4 timing may run before the #35 §10 gates pass.

R9 representation/query remains a required orthogonal surface as specified by
the ROADMAP. Final report/Weakness Map authority remains with the later final
synthesis stage; #31 does not move that authority into `report/` early.
