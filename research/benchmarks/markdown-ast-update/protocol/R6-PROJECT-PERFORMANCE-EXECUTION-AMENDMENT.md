# R6 Project-Performance Execution Amendment

Status: **FROZEN FOR #31 EXECUTION**

Authority chain:

```text
#22 MARKIT-MARKDOWN-BENCHMARK-1
  -> protocol/R0-METHODOLOGY.md (baseline methodology)
  -> this amendment (post-R5 execution refinement)
  -> #31 project-driven performance evaluation
```

This file is a narrow post-R5 amendment. It does **not** reopen H0-H4 mechanism
semantics, correctness contracts, timer boundaries, sampling rules, parity,
compiler/build policy, work-counter semantics, or the R0 conclusion ladder.

## Why this amendment exists

R0 §9 originally assigned real Markdown the role of realism/sanity corpus while
freezing synthetic shapes as the first-round controlled payload surface. After
R5 correctness/parity closure, the next performance campaign is intentionally
reordered so that **pinned real projects become the primary realism/performance
surface**, while the frozen synthetic corpus becomes the controlled explanatory
surface used to isolate scaling signatures and mechanism causes.

This role change is explicit here; README files and issue text do not silently
override R0.

## Retained R0 authority

Unchanged:

```text
same Rust substrate
same H0-H4 mechanism identities
same BENCH-GRAMMAR-v1 semantics
same normalized result contract
same canonical UTF-8 edit contract
same T_prepare / T_native / T_total boundaries
same T/M/A lane separation
same implementation-parity / optimization ban
same work-counter and PA semantics
same sampling policy (3 sessions, 10 warmups, 30 measured, p50+p95)
same no-outlier-deletion rule
same attribution ladder and optimization-sensitivity requirement
```

No project measurement may weaken the correctness oracle or compare different
payload/edit semantics across horses.

## Changed execution role

For R6-R11 execution under #31:

```text
PRIMARY REALISM/PERFORMANCE SURFACE
  pinned real Markdown projects
  -> frozen BENCH-GRAMMAR-v1 eligibility policy
  -> same eligible files for H0-H4
  -> same deterministic canonical edit traces for H0-H4

CONTROLLED EXPLANATORY SURFACE
  frozen synthetic shapes/cases
  -> used after a real-project observation identifies a candidate dimension
  -> isolates N/B/L/D/K/F scaling, crossover, and mechanism attribution
```

A real project is not automatically eligible. Project/file coverage, exclusion
reasons, and eligibility-by-file/by-byte must be published before timing.
Unsupported syntax may not be silently discarded.

## R6-R11 execution mapping

The existing ROADMAP stage meanings remain useful, but #31 refines their
execution order as follows:

```text
#31 P0  project reconnaissance + eligibility/trace freeze
#31 P1  R6 full parse / retained-state construction surface
#31 P2  R7/R8 real-project edit performance (arbitrary + structural families)
#31 P3  R11 attribution on material winners/losers
#31 P4  R10 controlled synthetic scaling targeted by P1-P3 observations
#31 P5  R11 optimization-sensitivity / residual attribution close-out
```

R9 representation/query measurements remain an orthogonal required surface and
may run alongside P1-P3 where their inputs are already frozen. Final replication
and final report/Weakness Map authority remain governed by the later ROADMAP
stages; #31 may produce candidate analysis, not silently declare the final
Markit algorithm.

## Data-layout refinement

For #31 the repository layout is frozen as:

```text
projects/             project pins, eligibility, workload profiles
traces/               deterministic canonical edit traces
results/raw/          immutable runner observations
results/summaries/    machine-derived summaries
results/manifests/    campaign/environment/input identity
analysis/             attribution/scaling/crossover scripts, tables, figures
report/               later reviewed synthesis only
```

This intentionally places generated figures under `analysis/` rather than
`results/`; this is the one pre-measurement path refinement allowed by #31.

## Hard stop before timing

Project timing may start only after the project set, immutable SHAs,
eligibility policy, workload profile schema, trace selection rule/seed, and
campaign manifest identity are frozen and reviewed.

Gate:

```text
PROJECT_CORPUS_FREEZE_PASS
```

Until that gate, project discovery/profile work is setup evidence only, not a
performance result.
