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

#35 workload construction              COMPLETE — PR #39 merged
#22 real-workload correctness closure  PR #40 current review
#22 measurement substrate corrective   COMPLETE on PR #40 head (this branch)
#31 performance                        NOT STARTED
```

For workload definition and usage, see `WORKLOAD.md`.

R5 established that H0-H4 solve the same BENCH-GRAMMAR-v1 problem under the
same normalized-result contract and that eager completion, correctness,
identity/reuse accounting, fallback semantics, source-inspection attribution,
and implementation parity are trustworthy.

**R5 did not produce a performance ranking.** Formal timing begins only after
the #31 project/eligibility/trace freeze gate.

MEASUREMENT-CORRECTIVE-1 (`protocol/MEASUREMENT-CORRECTIVE-1.md`) then
repaired the measurement substrate itself: the timer boundary (native-sealing
`complete()` inside timing; normalize/validate/checksum strictly post-timer
via `ResultChecksum`), attribution schema v2 (Old/Post source-inspection
coordinate spaces, unique-vs-cumulative effort, explicit PA definition),
cumulative counter authority including discarded fallback/restart work, the
H1/H4 counter repairs with H2/H3 audits, and FULL_READ extended to
**clean parse + native-state construction parity on all five horses
(110/110)**. The canonical research question, the decision questions
DQ1-DQ7, and the measurement/attribution qualification questions MQ1-MQ7
are frozen in `protocol/R0-METHODOLOGY.md` §0.1; the
metric qualification table (what may and may not be concluded in this
environment) is in `protocol/MEASUREMENT-CORRECTIVE-1.md` §9. No formal
timing, no ranking, and no primary measurement were produced by the
corrective.

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
- real-workload authority amendment: `protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md`
- measurement substrate corrective: `protocol/MEASUREMENT-CORRECTIVE-1.md`
- primary campaign freeze (#31):
  `protocol/R7-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-v1.md`
  (+ `results/manifests/primary-*.toml|jsonl|json`,
  `protocol/campaign-observation-schema-v1.json`, the `campaign` crate)
- Stage A workload construction: **#35** + `workloads/`
- post-freeze performance execution: **#31** (campaign contract frozen;
  primary timing NOT started)
- harness contract: `protocol/R1-HARNESS-CONTRACT.md`
- grammar: `grammar/BENCH-GRAMMAR-v1.md`
- normalized result: `grammar/NORMALIZED-RESULT-v1.md`
- grammar lanes (G0/G1/G2): `grammar/GRAMMAR-LANES-v1.md`
- transition oracle: `grammar/TRANSITION-ORACLE-v1.md`
- profiler contract: `workloads/profiles/PROFILER-CONTRACT-v1.md`
- payload lifecycle: `workloads/payloads/PAYLOAD-LIFECYCLE-v1.md`
- semantic pilot: `workloads/pilots/README.md`
- CORRECTIVE-A adversarial review record:
  `workloads/REVIEW-CORRECTIVE-A-ADVERSARIAL-v1.md`
- R5 mechanism freeze: `protocol/R5-HORSE-CORRECTNESS-PARITY.md`
- R5 evidence: `protocol/R5-HORSES-STAGE-RECORD.md`
- repository ownership map: `STRUCTURE.md`

#31 is an execution refinement under #22; it does not independently override
frozen R0 contracts. The explicit R6 amendments change only the post-R5 role
and ordering of real-project versus synthetic performance surfaces, and the
Stage A workload-construction gates.

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
semantics/          Stage A semantic substrate: grammar lanes, profiler,
                    transition oracle, payload lifecycle, semantic pilot
scripts/            verification and campaign scripts

# #35 Stage A workload construction (COMPLETE — see WORKLOAD.md)
workloads/          acquisition provenance/lock/inventory (PR #34), profiler
                    contract + schemas, frozen selections, frozen payload
                    artifacts (transition registry, applicability matrix,
                    FULL_READ/EDIT_WRITE/trace manifests, coverage, freeze
                    receipt), payload lifecycle + schemas, semantic pilot
diagnostics/        post-freeze correctness diagnosis only (mdbench-diverge);
                    owns no workload identity, imported by no mechanism

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

Stage A ran as three correctives under #35; all three are complete and the
frozen workload exists (`WORKLOAD.md`). No H0-H4 research timing is authorized
yet.

```text
CORRECTIVE-A  semantic substrate          lanes / profiler / oracle / lifecycle
              -> closed G2, G3, G5, G6 substrate contracts
CORRECTIVE-B  profile + select            all candidates, bias, four sets
              -> G1, G4
CORRECTIVE-C  payload freeze              anchors, edits, BREAK/RESTORE, traces
              -> G6, G7 — PR #39 merged

#22 correctness closure (H2/H3/H4)        PR #40
              -> frozen G0 matrix 362 x H0-H4 = 1810/1810 PASS
              -> awaiting human correctness review

then #31 P0 -> project/eligibility/trace freeze -> H0-H4 timing
```

`PROJECT_CORPUS_FREEZE_PASS` is superseded as a Stage A execution gate by the
#35 gate set (`protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` §2).
`CORE_REAL_WORKLOAD_FREEZE_PASS` for the primary G0 lane is a
candidate awaiting human review; every performance measurement remains
outstanding and belongs to #31.
