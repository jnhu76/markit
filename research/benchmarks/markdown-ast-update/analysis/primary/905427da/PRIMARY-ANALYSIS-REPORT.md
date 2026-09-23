# PRIMARY-ANALYSIS-REPORT — MARKIT-31-PRIMARY-PERFORMANCE-ANALYSIS-1

Final Stage-A + Stage-B report for the frozen #44 primary performance
campaign. Authority chain: #35 -> #40 -> #41 -> #42 -> #43 -> #44 -> this
analysis. Companion matrices: `DQ-MATRIX.md`, `MQ-MATRIX.md`;
interpretation freeze: `PRIMARY-OBSERVATIONS-v1.md`
(SHA256 5cfb5c993d5aa212a01cad19b415e9d5cba10ff49eff684a03aa9af0b07f5d66).

## 1. Evidence identity

```text
authority merge SHA   0bf678cc1505098e6afe26cb8ec54cda24115831 (PR #44)
CampaignSpecId        ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2
RunId                 905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d
primary executable    a3ef4e63a8548604fff991c5b981e3dfc856238fd06ead3e9192a5a3b6acd5bb
machine               primary-e5-xeon2666v3-fedora44 (E5-2666 v3, cpu1-pinned)
analysis branch       research/31-primary-performance-analysis-1 @ 0bf678c + artifacts
```

## 2. Data integrity

`PRIMARY_ANALYSIS_INPUT_PASS` (logs/00-input-integrity.log): 8 raw JSONL +
8 receipts, 0 invalid markers; every file SHA256 matches both
`raw-inventory.txt` and its receipt; row_count == expected_rows ==
unique_observation_ids == actual rows per file; totals 1920 attribution +
230400 timing (57600 warmup + 172800 measured) = 232320; CampaignSpecId /
RunId distinct-count 1; all rows execution_status=pass &&
correctness_status=pass; T_total == T_prepare + T_native on every timing
row under exact schema semantics (`prepare_ns = NOT_APPLICABLE` is
whole-cell and occurs exactly on the 330 clean-state cells, where the
measured phase is by definition "clean parse + native-state construction"
and T_total == T_native); 5760 cells x exactly 30 measured; zero duplicate
observation ids; mechanism set == {H0..H4 frozen ids}.

Raw evidence untouched: analysis ran on a reflink copy
(`~/markit-analysis-input/`); nothing under `results/raw/` was opened for
write; `git status` of the primary-run worktree is unchanged.

## 3. Statistical contract

Implemented exactly per `campaign/src/stats.rs` (nearest-rank p50 rank 15,
p95 rank 29, n=30, no interpolation; no pooling; case estimate = median of
3 session values; paired per-session H0-relative speedup then geometric
mean; instability ratio > 1.5 diagnostic-only; PROJECT_MACRO hierarchy
case -> trace -> file -> project -> equal-weight; FAMILY_MACRO equal
weight). Cross-check §58: a standalone Rust binary calling
`campaign::stats` directly (10 predetermined CaseIds = SHA256-derived
subsample fixed before comparison, all 5 horses, all 3 sessions) agreed
with the Python Stage-A artifacts on 1250 comparisons, 0 mismatches
(integer quantiles exact; ratios within CSV 6-decimal formatting).
**CROSSCHECK_PASS** (logs/03-crosscheck.log).

## 4. CLEAN_STATE results (22 files x H0-H4)

Every incremental horse is SLOWER than H0 full parse on every file and
every project: PROJECT_MACRO H1 0.754, H2 0.802, H3 0.797, H4 0.782
(per-project 0.681-0.864; project-macro.csv). Semantic name applies: this
is clean parse + native-state construction, NOT raw parser speed. The
~15-32% premium is each mechanism's fixed state-construction cost
(weakness W2) and sets the amortization floor behind the small-file
crossover. T_prepare is NOT_APPLICABLE by schema on this surface; no
unavailable metric is reported as zero.

## 5. EDIT_WRITE results (362 cases x H0-H4)

```text
                    H1      H2      H3      H4
PROJECT_MACRO      1.059   1.368   1.447   2.342
FAMILY_MACRO       0.948   1.441   1.506   2.161
CASE geomean       0.989   1.587   1.702   2.688
CASE median        0.690   1.803   2.046   3.680
cases faster       27.6%   76.8%   77.6%   77.9%
median T_total p50 30.1us  13.7us  12.3us   6.8us   (H0 23.9us)
```

Family view (family-macro.csv): E1 H4 2.904; E2 H4 2.728; E3 H4 1.983;
E4 H1 0.878 / H4 1.972; E5 H4 2.847; E6 all horses < 1.0 (H4 0.648);
ATX_HEADING_TOGGLE (frozen family, 12 cases) H4 3.845. H1 is bimodal by
project (owasp 2.518 vs rust-rfcs 0.662) with fallback firing on 65% of
all cases.

## 6. Project/regime heterogeneity

Per-project and per-file tables (project-macro.csv, file-macro.csv) show
rank flips the macro cannot hide: H1 spans 0.61x-2.52x across projects;
H4 wins 9 of 10 projects (loses kubernetes-keps 0.808) and every file bin
above 2KiB. Regime flips: file size (<2KiB all horses < 1.0; crossover
inside 2-4KiB for H2/H3/H4; H1 only >8KiB), container depth (>=3 flips
H2/H3/H4 below H0), edit family (E6 below H0 everywhere). Descriptive bins
over the frozen population; knee points NEEDS_CONTROLLED.

## 7. Work attribution

Median per-case source inspection (H0 whole file, PA ~2030) vs H2 170 /
H3 215 / H4 129 (attribution-by-horse-family.csv). Locality claims hold
directionally with re-visitation 1.9-2.6x (unique vs cumulative kept
separate per §16/§17). Fallback counter exists only on the H1 schema
(0.646/case); restart/convergence only on H4 (medians 111B/272B overall;
E4 880B; E6 3225B). E6 shows structural reuse WITH ~half-tree
rematerialization and metadata multiplier — the §18/#40 semantic caveat
is visible in the counters, not assumed.

## 8. DQ1-DQ7

Full matrix: `DQ-MATRIX.md`. States: DQ1 ANSWERED_PRIMARY (H4 best in 9/10
projects; H1 wins only fragmented-file projects); DQ2 OBSERVED_NEEDS_CONTROLLED;
DQ3 ANSWERED_PRIMARY (population) + flip NEEDS_CONTROLLED; DQ4
ANSWERED_PROFILE_SUPPORTED; DQ5 ANSWERED_PROFILE_SUPPORTED; DQ6
ANSWERED_PRIMARY (OBSERVED_CROSSOVER); DQ7 OBSERVED_NEEDS_CONTROLLED
(five candidate crossovers, none single-row).

## 9. MQ1-MQ7

Full matrix: `MQ-MATRIX.md`. Headlines: H2/H3/H4 beat H0 end-to-end on
T_total; H1 does not. Wins are explained by avoided inspection; fallback
and convergence are the loss mechanisms; locality credible with 1.9-2.6x
re-visitation; verification cost excluded by contract; qualification set
unchanged (LATENCY / WORK_COUNTERS / PARSE_AMPLIFICATION only).

## 10. Session instability

82 of 1920 cells flagged (4.3%), all edit_write, spread evenly
(H0 17, H1 17, H2 13, H3 16, H4 19). No deletion/downweight/rerun —
flag-only per contract. No horse-specific stability pathology.

## 11. Profiling follow-up (Stage B, NON_PRIMARY)

`profiling/PROFILING-RECORD-v1.md`. perf 7.2.5 user-space only; bpftrace
UNAVAILABLE (unprivileged BPF disabled). Replay (same runner primitives,
fresh-state per iteration, complete() boundary, cpu1-pinned) reproduced
every slot's winner/loser structure; replay absolute times are ~1.1-1.3x
primary (frequency regime) and are never compared absolutely. Findings:
H3 prepare = old-tree index construction (allocation-coupled); E6
consultation doubles branch-miss rate; allocator churn ~25-30% of replay
process cycles (PROFILE_ONLY); no cache/IPC pathology anywhere.
Cycles-differential per-edit values retained but deemed UNRELIABLE
(design note in the record). perf.data kept externally, hashes committed.

## 12. Remaining unexplained observations

FOLLOWUP-CANDIDATES.md (F1-F7): H1's fallback boundary vs file structure;
E6 environment-cost decomposition; H4 convergence scaling; CLEAN_STATE
premium decomposition; H1 small-file knee; allocator sensitivity;
deep-container flip confirmation.

## 13. Controlled follow-up proposals

CONTROLLED-FOLLOWUP-PLAN-v1.md: C-FULL-1 (HUGE_BLOCK), C-FENCE-1
(FENCE_HEAVY), C-REF-1 (REFERENCE_FANOUT), C-CONT-1 (MANY_BLOCKS/depth),
C-SMALL-1 (N ladder), C-CLEAN-1 (premium decomposition, diagnostic arm).
None executed here. CONTROLLED_FOLLOWUP = NOT_STARTED.

## 14. Evidence limitations

```text
- G0 strict grammar only (BENCH-GRAMMAR-v1); no G1/CommonMark claims
- single machine, single frozen compiler/profile, single-worker mechanisms
- p95 at n=30 is a descriptive tail indicator
- CPU_TIME / allocation / memory metrics UNAVAILABLE in the primary lane
- attribution is one untimed observation per case x horse
- fallback / restart counters are horse-schema-specific (no cross-horse
  comparison of absent counters)
- perf/eBPF follow-ups are post-hoc explanatory data, never primary metrics
- controlled causal sweeps NOT executed; nothing here is LEVEL 4
- descriptive bins are population quantiles, not measured knee points
```

## Verdict

```text
PRIMARY_PERFORMANCE_ANALYSIS_PASS
PRIMARY_RAW_EVIDENCE      = UNCHANGED
PRIMARY_STATISTICS        = COMPLETE (CROSSCHECK_PASS)
PRIMARY_ATTRIBUTION_JOIN  = COMPLETE
DQ_MATRIX                 = COMPLETE
MQ_MATRIX                 = COMPLETE
PROFILE_FOLLOWUP          = COMPLETE (perf) / eBPF UNAVAILABLE
CONTROLLED_FOLLOWUP       = NOT_STARTED
FINAL_WEAKNESS_MAP        = NOT_STARTED (#33 authority)
```
