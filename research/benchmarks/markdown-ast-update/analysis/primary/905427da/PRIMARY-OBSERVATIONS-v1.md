# PRIMARY-OBSERVATIONS-v1 — MARKIT-31 primary analysis Stage-A freeze

Status: **STAGE-A OBSERVATION FREEZE (task §32)**. Written after the
input-integrity gate, the full Stage-A summary, the regime views, and the
`campaign::stats` cross-check (CROSSCHECK_PASS, 1250 comparisons, 0
mismatches). After this file is hashed, primary summaries are NOT rewritten
to fit profiling stories; profiling may only add explanatory evidence.

```text
Authority merge   0bf678cc1505098e6afe26cb8ec54cda24115831 (PR #44)
CampaignSpecId    ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2
RunId             905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d
Executable        a3ef4e63a8548604fff991c5b981e3dfc856238fd06ead3e9192a5a3b6acd5bb
Input gate        PRIMARY_ANALYSIS_INPUT_PASS (00-input-integrity.log)
Cross-check       CROSSCHECK_PASS (03-crosscheck.log, §58 protocol)
Evidence levels   per §28; this freeze contains LEVEL 1-2 claims only
```

All numbers below are read from the committed CSV artifacts in this
directory (`project-macro.csv`, `family-macro.csv`, `case-weighted.csv`,
`regime-*.csv`, `attribution-by-horse-family.csv`, `session-stability.csv`).

---

## 1. Headline patterns

### O1 — EDIT_WRITE macro results (LEVEL 1)

```text
                    H1      H2      H3      H4
PROJECT_MACRO      1.059   1.368   1.447   2.342
FAMILY_MACRO       0.948   1.441   1.506   2.161
CASE geomean       0.989   1.587   1.702   2.688
CASE median        0.690   1.803   2.046   3.680
cases faster       27.6%   76.8%   77.6%   77.9%
```

H4 (restart-convergence) is the strongest mechanism on this real workload;
H1 (block-local-reparse) does NOT beat full rebuild on average (geomean
0.989, median case 0.690) — its macro-level parity hides a strongly
bimodal per-project distribution (O6).

### O2 — CLEAN_STATE: incremental state construction premium (LEVEL 1)

On all 22 G0-strict FULL_READ files (clean parse + native-state
construction), EVERY incremental horse is slower than H0's plain parse:

```text
PROJECT_MACRO clean_state: H1 0.754  H2 0.802  H3 0.797  H4 0.782
per-project range:         0.681–0.864 (no project, no horse reaches 1.0)
```

The premium (~15–32%) is the fixed cost of building each mechanism's
auxiliary state from scratch. It is the price an incremental mechanism
must amortize; it directly shapes the small-file crossover (O5).

### O3 — E6 REFERENCE_DEFINITION: every incremental horse loses (LEVEL 2)

```text
family-macro speedup:  H1 0.645  H2 0.857  H3 0.812  H4 0.648
H1 fallback rate:      1.000 events/case (block-local always falls back)
H4 convergence:        median 3225 bytes (vs 272 overall), nodes_reused median 0
H2/H3 (§18 check):     nodes_reused 56–66 BUT nodes_rebuilt 64–67 and
                       metadata_records_touched 27–53 (vs 14 for H2 overall):
                       structural reuse does NOT imply zero semantic
                       maintenance — consistent with the #40 corrective.
```

Reference-definition edits invalidate the definition environment; every
mechanism pays near-full-reparse cost, and the incremental ones pay it
PLUS their own overhead.

### O4 — Deep containers flip H2/H3/H4 below H0 (LEVEL 1, OBSERVED_NEEDS_CONTROLLED)

```text
max_container_depth >= 3 (45 cases):
  H1 0.961   H2 0.668   H3 0.644   H4 0.834   (depth 0: 1.72/1.95/4.08 for H2/H3/H4)
```

Container-heavy files reverse the ranking of the reuse mechanisms.
Observational only: the controlled MANY_BLOCKS/container sweep has not run.

### O5 — Small-file crossover (LEVEL 1, OBSERVED_CROSSOVER)

```text
file bin    H1      H2      H3      H4     (median case speedup)
<2KiB      0.571   0.670   0.644   0.834   ALL lose to H0
2-4KiB     0.639   1.647   1.811   3.030   H2/H3/H4 cross above 1.0
4-8KiB     0.684   2.299   2.428   4.308
>8KiB      3.198   2.563   2.927   5.472
```

H0 is outright cheaper on the smallest real files (DQ6 answer). The exact
thresholds are descriptive bins over the frozen population, not measured
knee points (NEEDS_CONTROLLED for precise placement).

### O6 — H1 is bimodal by project, driven by fallback (LEVEL 2)

```text
H1 per-project macro:  owasp 2.518  crafting 1.979  d2l 1.610  otel 1.586
                       vs  rust-rfcs 0.662  rust-book 0.621  myst 0.611
fallback mean/case:    0.646 overall; 1.000 on E6 and ATX; 0.795 on E4
H1 >8KiB wins come ONLY from crafting-interpreters + d2l-en files.
```

H1 wins where files are block-fragmented (small affected blocks), loses
where its block-local capacity is exceeded and it falls back to full
reparse plus dispatch overhead.

### O7 — E4 fence propagation (LEVEL 2; scaling NEEDS_CONTROLLED)

```text
H1 E4 family macro 0.878 (fallback 0.795/case).
H4 E4: Spearman(latency, convergence_distance) = 0.667 (n=44);
       median convergence 880 bytes (vs 272 overall).
```

A real-workload fence-forward-propagation penalty exists for H1 (fallback)
and H4 (convergence scan). Causal scaling confirmation requires the
controlled FENCE_HEAVY sweep.

### O8 — Prepare vs native split (LEVEL 1)

```text
median T_prepare / T_native (edit_write):
  H0   36 ns  / 23843 ns
  H1   44 ns  / 30076 ns
  H2   82 ns  / 13626 ns
  H3 1575 ns  / 10662 ns   <-- prepare outlier
  H4  257 ns  /  6481 ns
```

H3's prepare phase (old-tree/subtree preparation) costs ~20-40x the other
horses' prepare; it is part of why H3 trails H4 despite similar native
work. Candidate for profiling (P3).

---

## 2. Candidate crossovers (all OBSERVED_*; thresholds NEEDS_CONTROLLED)

```text
C1  file size x {H2,H3,H4}:  crossover between 2KiB and 4KiB (O5)
C2  file size x H1:          crossover only >8KiB AND project-dependent (O5/O6)
C3  container depth:         H2/H3/H4 drop below 1.0 at depth >= 3 (O4)
C4  edit family:             E6 below 1.0 for ALL incremental horses at any size (O3)
C5  CLEAN_STATE premium:     all horses below 1.0 everywhere (O2) — bounds
                             how small an amortization base can be
```

## 3. Candidate cliffs

```text
X1  H1 fallback cliff:     block-local capacity exceeded -> full reparse +
                           overhead -> strictly worse than H0 (O6/O7)
X2  H4 convergence cliff:  convergence distance approaches file size on E6
                           (median 3225 B) -> H4 degrades to ~H0 cost + overhead (O3)
X3  H2/H3 semantic cliff:  definition-environment invalidation forces node
                           rematerialization + metadata maintenance (O3, §18)
```

## 4. Unexplained residuals (profiling targets)

```text
R1  H1 winner-vs-loser file structure: same family, same size bin can sit on
    opposite sides of 1.0 (owasp vs rust-rfcs). Which byte/block structure
    decides fallback?                     -> P1, P2
R2  H3 prepare cost (1575 ns median): what runs in prepare? -> P3
R3  E6 x H2/H3: is the residual latency in definition-environment rebuild,
    range/metadata maintenance, or node rematerialization? -> P4, P5
R4  CLEAN_STATE premium: where do the extra ~20-30% go for each horse? -> P6, P7
R5  H4 at depth>=3: latency elevation beyond convergence distance? -> P8
R6  H4 best-case magnitude: is the 3-5x win purely avoided parsing, or also
    cheaper state construction? (sanity counterexample) -> P9
```

## 5. Session stability

```text
SESSION_UNSTABLE cells (max/min session p50 > 1.5): 82 of 1920 (4.3%),
all edit_write; by horse: H0 17, H1 17, H2 13, H3 16, H4 19.
No session was deleted, downweighted, or rerun (contract). The flag
distribution shows no horse-specific stability pathology.
```

## 6. Profiling candidates (selection finalised in PROFILING-SELECTION-v1.md)

```text
P1  H1 x owasp-cheatsheets winner case   (R1)
P2  H1 x rust-rfcs loser case            (R1, counterexample to P1)
P3  H3 x ordinary E5 case, prepare phase (R2)
P4  H3 x E6 case                         (R3)
P5  H2 x E6 case                         (R3)
P6  CLEAN_STATE x H2, one large file     (R4)
P7  CLEAN_STATE x H4, same file          (R4)
P8  H4 x depth>=3 case                   (R5)
P9  H4 x E5 winner case (counterexample) (R6)
```

## 7. What this freeze does NOT claim

```text
- no causal scaling claims (LEVEL 4 requires controlled sweeps)
- no winner declaration beyond the frozen aggregates above
- no Clean-State "parser speed" interpretation (it is parse + native-state
  construction by definition)
- no primary-qualified CPU/allocation/memory claims (UNAVAILABLE by contract)
```
