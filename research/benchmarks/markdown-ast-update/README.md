# MARKIT Markdown Update Benchmark

Controlled Rust study of Markdown AST/CST update mechanisms.

## Status

```text
R0  methodology freeze                 PASS
R1  controlled Rust harness            PASS
R2  prior-art mechanism extraction     PASS
R3  grammar/corpus/mutation freeze     PASS
R4  H0 reference full rebuild          PASS
R5  H1-H4 correctness/parity           PASS — PR #30 merged

ACTIVE: #31 project-driven performance evaluation under #22
```

R5 established that H0-H4 solve the same BENCH-GRAMMAR-v1 problem under the
same normalized-result contract and that eager completion, correctness,
identity/reuse accounting, fallback semantics, source-inspection attribution,
and implementation parity are trustworthy.

**R5 did not produce a performance ranking.** Formal timing begins only after
the #31 project/eligibility/trace freeze gate.

## Mechanisms

```text
H0 FULL_REBUILD
H1 BLOCK_LOCAL_REPARSE
H2 FRAGMENT_REUSE
H3 OLD_TREE_SUBTREE_REUSE
H4 RESTART_CONVERGENCE
```

## Research authority

- umbrella issue: **#22 MARKIT-MARKDOWN-BENCHMARK-1**
- baseline methodology: `protocol/R0-METHODOLOGY.md`
- post-R5 execution amendment: `protocol/R6-PROJECT-PERFORMANCE-EXECUTION-AMENDMENT.md`
- active execution issue: **#31 project-driven performance evaluation**
- harness contract: `protocol/R1-HARNESS-CONTRACT.md`
- grammar: `grammar/BENCH-GRAMMAR-v1.md`
- normalized result: `grammar/NORMALIZED-RESULT-v1.md`
- R5 mechanism freeze: `protocol/R5-HORSE-CORRECTNESS-PARITY.md`
- R5 evidence: `protocol/R5-HORSES-STAGE-RECORD.md`
- repository ownership map: `STRUCTURE.md`

#31 is an execution refinement under #22; it does not independently override
frozen R0 contracts. The explicit R6 amendment changes only the post-R5 role
and ordering of real-project versus synthetic performance surfaces.

Frozen R0-R5 paths are intentionally retained. Do not reorganize historical
protocol/corpus/case files merely for aesthetics: citations and gate records
refer to them.

## Workspace map

```text
# Frozen research authority / controlled inputs
protocol/           methodology, amendments, stage contracts, evidence records
prior-art/          mechanism provenance and fidelity boundaries
grammar/            BENCH-GRAMMAR-v1 + normalized result contract
corpus/             frozen synthetic corpus definitions / receipts
mutations/          frozen mutation families
cases/              frozen case matrices
manifest/           environment/schema manifests where applicable

# Executable benchmark substrate
common/             source/edit/case/work/result substrate types
instrumentation/    timing / measurement support
oracle/             correctness normalization + validation
runner/             measurement process / lanes / isolation
corpusgen/          deterministic synthetic corpus construction
shared-grammar/     common BENCH-GRAMMAR-v1 parser primitives
mechanisms/         H0-H4 + null R1 mechanism crates
scripts/            verification and campaign scripts

# #31 project-driven measurement lifecycle
projects/           pinned project manifests, eligibility, profiles
traces/             deterministic real-project edit traces
results/            raw measurements + machine-readable summaries
analysis/           attribution/scaling/crossover/replication analysis
report/             later reviewed Weakness Map / paper-like outputs
```

The dependency direction is:

```text
projects + traces
      ↓
runner + H0-H4
      ↓
results/raw
      ↓
results/summaries
      ↓
analysis
      ↓
later report / Weakness Map
```

No analysis script may rewrite raw observations. No report table is an
authority substitute for the raw rows/manifests from which it was derived.

## #31 performance question

For each pinned real Markdown project, using the same eligible files and the
same deterministic edit traces:

> Which mechanism wins or loses, by how much, in which workload regime, and
> what mechanism-level work causes the difference?

Required headline data include:

```text
full parse/state construction:
  p50/p95 latency, MiB/s, files/s, CPU, allocations, peak/retained memory

incremental update:
  T_prepare/T_native/T_total p50/p95, speedup vs H0, CPU, allocations

attribution:
  source bytes inspected / PA
  blocks reparsed
  nodes rebuilt/reused
  metadata touched
  fallback
  restart/convergence distance
  mechanism-specific consultations where preregistered
```

A result such as "H3 is slow" is not a conclusion. It remains an observation
until timing, scaling, work counters, and a controlled explanation agree.

## Measurement discipline

The frozen R0 policy remains authoritative:

```text
3 independent sessions
10 warmups/session
30 measured iterations/session
recorded shuffled case order
p50 + p95
no p99
no outlier deletion
```

Timing and attribution lanes remain separate and are joined by stable case /
trace identity.

## Current next gate

#31 starts with project reconnaissance and eligibility/trace freeze under the
R6 execution amendment:

```text
PROJECT_CORPUS_FREEZE_PASS
```

No H0-H4 project timing is primary evidence before the project set, immutable
SHAs, BENCH-GRAMMAR-v1 eligibility policy, workload-profile schema,
deterministic trace-selection rules, and campaign identity are frozen.
