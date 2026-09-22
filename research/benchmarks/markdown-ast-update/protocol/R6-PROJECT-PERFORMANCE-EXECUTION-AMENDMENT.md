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

## MEASUREMENT-CORRECTIVE-1 constraints on #31 execution

`protocol/MEASUREMENT-CORRECTIVE-1.md` repairs the measurement substrate
(timer boundary, attribution schema v2, counter authority, FULL_READ parity).
Its rules bind every #31 phase and are restated here because #31 is the
execution issue they protect:

1. **Frozen workload primacy.** The materialized real workload
   (`workloads/` — selections, sources, manifests, payloads, receipts) is
   the authority for WHAT is measured. No #31 phase may reselect, extend, or
   regenerate the strict surface; the G0-strict file list lives in the
   frozen `full-read-manifest-v1.jsonl`, and its mechanism-neutral facts
   are re-derived only by the frozen profiler from hash-verified source
   bytes (`mdbench-corrective-c profile-export`) — the strict SET itself is
   never reselected.
2. **Timer boundary.** `T_native` contains update + native-sealing
   `complete()` only. Normalize/validate/checksum/oracle work is a
   post-timer export via `ResultChecksum`. Any #31 measurement that times
   oracle work is invalid and must be re-run.
3. **Counter authority.** Counters are cumulative actual work including
   discarded fallback/restart work. Never subtract discarded work; never
   present the incremental happy path without its fallback-inclusive
   cumulative counterpart (MQ3).
4. **Parse Amplification.** PA's numerator is the per-version unique unions
   (sum of Old union + Post union); its denominator is the logical edited
   bytes (R0 §10, unchanged). PA never derives from cumulative effort and
   never merges the Old and Post coordinate spaces.
5. **Qualification gate.** A promoted conclusion names its DQ(s) and the
   MQ(s) qualifying its evidence (R0 §0.1), its evidence class (#33 §1),
   and its metric families, each QUALIFIED under MEASUREMENT-CORRECTIVE-1
   §9. Until a CPU/memory instrumentor exists, conclusions must not lean
   on CPU_TIME, ALLOC_*, PEAK, or RETAINED (MQ7).
6. **Crossover discipline.** Crossover points (DQ7; measurement
   discipline MQ4) are located by observation first, then confirmed by
   controlled sweeps; intermediate scale points may be added only to
   resolve an observed crossover and are recorded as follow-up points,
   never silently.
7. **Clean parse + native-state construction parity.** Every horse must
   pass the 110/110 FULL_READ surface before any timing lane includes it;
   a horse that fails clean construction is not a performance subject.
8. **Gate sequencing.** MEASUREMENT_SUBSTRATE_PASS on this corrective
   unlocks ONLY `MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-1` —
   freezing session, iteration, horse order, case order, machine/profile,
   metric qualification, aggregation weights, quantile definition,
   failure handling, and raw-result identity. It does NOT confer
   `PRIMARY_PERFORMANCE_CAMPAIGN_READY` and does NOT authorize any
   primary timing run.

### DQ / MQ mapping for the #31 phases

Decision questions (DQ1-DQ7, R0 §0.1):

```text
DQ1 (local text edit: who is best)              -> P1/P2 (full parse + update latency, vs H0)
DQ2 (degradation as affected block grows)       -> P2/P3 (work counters vs timing, per affected-block size)
DQ3 (list/blockquote container edit winner)     -> P2 (per-edit-family latency + fallback-inclusive cumulative counters)
DQ4 (fence forward propagation degradation/why) -> P2/P4 (View D + fence-propagation family attribution)
DQ5 (who pays most on reference dependency change) -> P2 (reference-edit family; Old/Post unions)
DQ6 (small files: is H0 outright cheaper)       -> P1/P2 (small-N slice, vs H0)
DQ7 (crossovers across N/B/L/K/F regimes)       -> P2/P4 (crossover plots + controlled sweeps)
```

Measurement/attribution qualification questions (MQ1-MQ7, R0 §0.1):

```text
MQ1 -> P1/P2 (full parse + update latency, vs H0)
MQ2 -> P2/P3 (View E attribution; work counters vs timing)
MQ3 -> P2 (fallback-inclusive cumulative counters per edit family)
MQ4 -> P2/P4 (View D crossover plots + controlled sweeps)
MQ5 -> P2/P3 (unique vs cumulative inspection audit per horse)
MQ6 -> P1/P2 (in-timer vs post-timer split, enforced by the runner)
MQ7 -> every phase (qualification table is a precondition of writing)
```

These constraints amend #31's execution text where the two disagree; the
corrective file is the authority for the measurement substrate.
