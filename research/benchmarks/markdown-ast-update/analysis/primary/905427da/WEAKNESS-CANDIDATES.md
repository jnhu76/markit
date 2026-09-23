# WEAKNESS-CANDIDATES — MARKIT-31 primary analysis

Input to the final Weakness Map (#33 authority — NOT the Weakness Map).
Each candidate: mechanism weakness observed on the frozen primary real
workload, with evidence level per §28 and counterexamples kept visible.

## W1 — H1 (block-local reparse): fallback cliff on real workloads

```text
regime          large/complex affected regions: E6 (1.0 fallback/case),
                ATX (1.0), E4 (0.795), overall 0.646/case
symptom         slower than H0 exactly when fallback fires
timing          H1 family macro 0.948; per-project 0.611-2.518
work            fallback cases inspect MORE source than H0 (P2: 14530B
                vs 13511B) — full reparse PLUS block-scan overhead
profile         P2: no cache/IPC pathology; real added work
counterexample  block-fragmented files (owasp 2.52x, crafting 1.98x)
                never fall back and win big
confidence      HIGH (LEVEL 2; timing + counters agree)
controlled?     YES — C-FULL-1 (HUGE_BLOCK) to place the boundary
```

## W2 — ALL incremental horses: clean-state construction premium

```text
regime          clean parse + native-state construction (all 22 files)
symptom         0.68-0.86x vs H0 plain parse, uniformly, every project
timing          PROJECT_MACRO clean_state: H1 0.754 H2 0.802 H3 0.797 H4 0.782
work            aux-state build on top of identical parse
profile         P6/P7: parse-dominated + allocation churn; no single hotspot
counterexample  none within the surface (that IS the finding)
confidence      HIGH (LEVEL 1)
controlled?     YES — C-SMALL-1 / C-CLEAN-1 to decompose the premium
```

## W3 — E6 reference-definition dependency: environment invalidation

```text
regime          E6 REFERENCE_DEFINITION (20 cases), all file sizes
symptom         every incremental horse loses to H0 (H1 0.645, H2 0.857,
                H3 0.812, H4 0.648)
timing          case medians in regime-e6-reference.csv
work            H1 full fallback; H2/H3 structural reuse with half-tree
                rematerialization + metadata multiplier; H4 convergence
                to file end (median 3225B), nodes_reused 0
profile         P4/P5: branch-miss ~doubles; inspection ~= H0 despite reuse
counterexample  none observed in primary population
confidence      HIGH on the pattern (LEVEL 2 + profile support)
controlled?     YES — C-REF-1 (REFERENCE_FANOUT); must separate
                definition-distance from definition-count
```

## W4 — H4 (restart-convergence): convergence-distance degradation

```text
regime          E4 fence propagation (median conv 880B vs 272 overall);
                E6 (3225B); depth>=3 containers (P8: 1173B of 1632B file)
symptom         latency rises with convergence distance; Spearman 0.667 (E4)
timing          E4 family macro 1.972 (still > H0); E6 0.648 (below H0)
work            restart/convergence counters only exist on H4 schema
profile         P8: convergence-tracking, cache-clean, no stalls
counterexample  P9-class cases (conv ~130B) reach 4-5x speedups
confidence      MEDIUM-HIGH (LEVEL 3 pattern; causal scaling unproven)
controlled?     YES — C-FENCE-1 (FENCE_HEAVY propagation-length sweep)
```

## W5 — Container depth flip for reuse mechanisms

```text
regime          max_container_depth >= 3 (45 cases)
symptom         H2 0.668 / H3 0.644 / H4 0.834 (vs 1.7-4.1 at depth 0)
timing          regime-container-depth.csv
work            P8 counters: restart/conv span most of file
profile         P8 clean; no stall pathology
counterexample  depth 0-2 cases dominate the macro wins
confidence      MEDIUM (LEVEL 1 observed; n=45, NEEDS_CONTROLLED)
controlled?     YES — C-CONT-1 (MANY_BLOCKS/container depth)
```

## W6 — H3 prepare-phase index construction

```text
regime          all EDIT_WRITE (prepare median 1575ns vs H2 82ns / H0 36ns)
symptom         constant per-edit tax erodes H3's native advantage
timing          O8 in PRIMARY-OBSERVATIONS-v1
work            old-tree index build (build_tnode/assemble_level/shift_forest)
profile         P3: allocation-coupled index construction (LEVEL 3 support)
counterexample  none (constant cost by design)
confidence      MEDIUM-HIGH (LEVEL 3 for interpretation)
controlled?     maybe — optimization-sensitive check belongs to #33, not
                this campaign; record only
```

## Kept-visible strengths (not weaknesses; guard against overfitting)

```text
S1  H4 wins 77.9% of all real cases, median case 3.68x — the strongest
    mechanism overall; its weaknesses (W3/W4) are regime-specific.
S2  H2/H3 deliver 1.3-1.6x macro with the lowest inspection effort;
    their losses concentrate exactly where validity/environments break.
S3  H0 is the correct baseline below ~2KiB and on E6 — the "trivial"
    mechanism remains the right answer in real regimes.
```
