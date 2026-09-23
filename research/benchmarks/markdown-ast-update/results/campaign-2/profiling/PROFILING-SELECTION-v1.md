# PROFILING-SELECTION-v1 — Campaign-2 matched profiling slots

```text
evidence class   PROFILE_PERF
label            POST_HOC
                 NON_PRIMARY
authority        3762b7a42e1c284a4c2c2e0ebac8496e70c63431
```

Written **after** the primary/controlled collection and **before** any
`perf` run, from the mechanical summary tables only
(`results/campaign-2/summary/*.csv`). Every value quoted here is a p50
that already exists in those tables; nothing new was measured to choose a
slot.

Selection rule: one slot per question, each slot a SINGLE case/step that
is profiled identically for H0-H4 (task §34 — same case, source, edit,
replay structure, profiling binary and CPU). The horses in a slot are
never a mixed pair such as `H3 case A` vs `H0 case B`.

Profiling is **not** primary timing: the numbers below are p50s in
microseconds from the primary tables and are quoted only so the reader can
see why the slot was chosen.

| # | Question (task §36) | Surface / cell | Case index or step | Primary p50 (µs) H0 / H1 / H2 / H3 / H4 | Why this slot |
|---|---|---|---|---|---|
| P01 | strongest H4 real-workload win | resident-update | index 67 (E5 `G0-CODESPAN-DELIM-RESTORE`, rust-book appendix-05-editions) | 57.5 / 61.2 / 10.1 / 7.8 / 4.0 | H4 14.2× over H0; the largest real-workload gap |
| P02 | H4 real-workload loss (counterexample to the dominant pattern) | resident-update | index 194 (E3 `G0-LIST-ITEM-INDENT`, rust-book appendix-06-translation) | 17.4 / 32.5 / 17.4 / 16.1 / 37.1 | H4 is 2.1× SLOWER than H0 — the dominant pattern inverts |
| P03 | H1 fallback loss | resident-update | index 285 (E2 `G0-PARAGRAPH-MERGE`, rust-rfcs 0404-change-prefer-dynamic) | 23.4 / 54.7 / 42.2 / 44.1 / 39.0 | H1 0.44× H0, the largest H1 relative loss |
| P04 | H2/H3 loss on a delimiter transition | resident-update | index 21 (E5 `G0-CODESPAN-DELIM-RESTORE`, crafting-interpreters ch13) | 16.6 / 29.1 / 15.5 / 13.3 / 32.1 | H2 and H3 both beat H0 here while H1 and H4 lose — the opposite of P01 on the same transition family |
| P05 | construction premium | construction | owasp-cheatsheets Email_Validation (largest FULL_READ file) | 53.9 / 79.5 / 74.3 / 73.8 / 80.8 | every incremental horse carries a ~1.4-1.5× construction premium over H0 |
| P06 | small-file resident-update / small-document regime | controlled C-N `512B` | cell `C2-N-512B` | 1.93 / 1.26 / 2.25 / 2.10 / 0.99 | the only regime where H2/H3 lose to H0 outright |
| P07 | long fence propagation | controlled C-D `64KiB` | cell `C2-D-64KiB` | 508 / 678 / 19 966 / 19 601 / 499 | 20-40× loss for H2/H3; H0 and H4 are the only viable paths |
| P08 | fence case where H0 wins | controlled C-D `64B` | cell `C2-D-64B` | 780 / 1032 / 16 432 / 16 601 / 985 | shortest propagation span: H0 is the fastest horse — the span itself is not the cost driver |
| P09 | reference fanout | controlled C-F `64` | cell `C2-F-64` | 878 / 1202 / 4 060 / 119 152 / 1275 | H2 grows monotonically with F; H3 is flat and ~100× slower |
| P10 | deep container | controlled C-K `12` | cell `C2-K-12` | 843 / 542 / 706 / 626 / 406 | container depth 12; H1/H2/H3 cluster while H4 keeps a ~2× lead |
| P11 | affected-block-size stress | controlled C-B `64KiB` | cell `C2-B-64KiB` | 406 / 294 / 262 / 286 / 171 | target block is half the document; H1/H2/H3 converge |
| P12 | lifecycle late step | lifecycle `C2-L1-REPEATED-LOCAL-TEXT` | step 127 (K = 128) | (per-step p50s in `summary/lifecycle-step-p50.csv`) | 128 chained edits on one state — does history change the cost |

## Execution

```bash
PERF_STAT_REPETITIONS=10 ./results/campaign-2/logs/run-profiling.sh . <slots.tsv> <subdir>
```

For each slot and each horse:

1. a region-scoped replay (`mdbench-replay`, PMU counters enabled only
   across the measured region),
2. a `--setup-only` companion run with an EMPTY measured region, which is
   the independent validation of any whole-process number,
3. `perf stat` over the same replay command with
   `cycles:u,instructions:u,branches:u,branch-misses:u,cache-references:u,cache-misses:u,task-clock,page-faults,context-switches,cpu-migrations`,
   repeated 10 times with the horse order rotated one position per
   repetition.

`cycle`-level comparisons are only ever made **between horses inside one
slot**. `perf record` is run on the slots whose primary gap the region
counters do not explain.

## Known limitation of this lane (recorded, not hidden)

The PMU counters are read through `perf_event_open` at
`perf_event_paranoid = 2`, so they are user-space-scoped multiplexed
estimates (six hardware events share the four generic counters). A
cycle-vs-elapsed consistency check on a 128 KiB update gives an implied
frequency above the CPU's turbo ceiling, so **absolute cycle counts are
reported raw and are never converted into time**; only within-slot
horse-to-horse comparisons are used. See `EVIDENCE-GAPS.md`.
