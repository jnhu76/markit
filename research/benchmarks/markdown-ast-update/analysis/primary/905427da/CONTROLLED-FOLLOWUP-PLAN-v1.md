# CONTROLLED-FOLLOWUP-PLAN-v1 — MARKIT-31

Status: PROPOSAL ONLY (task §49-§50). This plan is the next experimental
gate; nothing in it has been executed. Every sweep follows the frozen
#22/#31 protocol family (same correctness gates, same measurement model,
30 measured samples per cell x 3 sessions, paired H0-relative effects).

## C-FULL-1 — HUGE_BLOCK (serves DQ2, W1, F1)

```text
hypothesis        H1's fallback boundary is a block-size/complexity knee,
                  not a file-size knee
primary obs       H1 loses iff fallback fires; fallback iff affected region
                  exceeds block-local capacity (0.646/case overall)
attribution       fallback cases inspect > H0 (reparse + scan overhead)
profiling         P2 record: no stall pathology — real added work
independent var   affected-block byte size B (controlled synthetic blocks)
held constant     edit length, position, grammar lane, horse set, file N
shape             sawtooth of single-block documents, B in {512B, 1K, 2K,
                  4K, 8K, 16K, 32K, 64K}
expected signal   H1 latency = H0 + constant for B above the knee
                  (fallback), sublinear below it
falsifier         H1 stays below H0 latency as B grows past the knee
DQ/MQ served      DQ2, DQ7, W1
```

## C-FENCE-1 — FENCE_HEAVY propagation (serves DQ4, W4, F3)

```text
hypothesis        H4 E4 degradation is driven by convergence distance
primary obs       E4 Spearman(lat, conv) = 0.667; median conv 880B
attribution       convergence_distance grows toward document end
profiling         P8: clean convergence-tracking profile
independent var   fence-forward propagation length D (edit near head of a
                  long fenced tail)
held constant     edit bytes, fence count, document size N
shape             D in {64B, 256B, 1K, 4K, 16K, 64K} at fixed N=32K
expected signal   H4 latency ~ linear in D while H2/H3 stay flat; H1 flat
                  unless fence density forces fallback
falsifier         H4 latency flat while D grows 1000x
DQ/MQ served      DQ4, DQ7, W4
```

## C-REF-1 — REFERENCE_FANOUT (serves DQ5, W3, F2)

```text
hypothesis        E6 losses are driven by definition-to-use distance and
                  definition count (environment invalidation), not by the
                  edit's local size
primary obs       all horses < H0 on E6; H2/H3 rematerialize ~half the tree;
                  H4 converges 3225B median
attribution       metadata multiplier + near-full inspection with reuse
profiling         P4/P5: branch-miss doubling on consultation
independent var   F: definition count and def-use distance (two arms)
held constant     edit length at a use site, document size
shape             definitions in {1, 4, 16, 64}; distance in {100B, 1K, 8K}
expected signal   losses grow with F and distance for reuse horses; H0 flat
falsifier         E6 cost independent of F and def-use distance
DQ/MQ served      DQ5, DQ7, W3
```

## C-CONT-1 — MANY_BLOCKS / container depth (serves DQ3, W5, F7)

```text
hypothesis        deep-container validity repair, not size, flips H2/H3/H4
primary obs       depth>=3: H2 0.668, H3 0.644, H4 0.834 (n=45 real cases)
attribution       restart/conv spanning the file on deep cases (P8)
profiling         P8: no stall; work is traversal/repair
independent var   container depth K in {0,1,2,3,4,6,8} and sibling count L
held constant     document bytes, edit size at a leaf
shape             nested lists/blockquotes at each K
expected signal   ranking flip reproduced at a specific K; H4 flat in K
                  until conv distance couples to K
falsifier         no ranking change across K at fixed bytes
DQ/MQ served      DQ3, DQ7, W5
```

## C-SMALL-1 — document-size N sweep (serves DQ6, DQ7, W2, F4/F5)

```text
hypothesis        the all-horses lose crossover below ~2KiB is set by the
                  CLEAN_STATE amortization base, not by edit cost
primary obs       <2KiB: H1 0.571, H2 0.670, H3 0.644, H4 0.834;
                  crossover inside 2-4KiB for H2/H3/H4
attribution       O2 premium is a fixed aux-state build cost
profiling         P6/P7: premium = parse + aux build + churn
independent var   document size N in {256B, 512B, 1K, 2K, 4K, 8K, 16K, 32K}
held constant     one edit family (E1), one edit size, block structure
shape             geometric ladder, 3 sessions, full H0-H4
expected signal   per-horse crossover N* where case speedup = 1.0; N*
                  tracks the horse's clean-state premium ratio
falsifier         crossover absent or uncorrelated with the premium
DQ/MQ served      DQ6, DQ7, W2
```

## C-CLEAN-1 — clean-state premium decomposition (serves W2, F4)

```text
hypothesis        the premium is dominated by aux-state construction, not
                  by a slower parse
primary obs       0.68-0.86x on all 22 files
attribution       T_prepare=N/A schema: parse+native are fused on this
                  surface; cannot split from primary data
profiling         P6/P7: parse-dominated, no single hotspot
independent var   surface arm: FULL_PARSE-only vs full clean-state replay
                  (diagnostic replay tooling, NON_PRIMARY)
held constant     same 22 frozen files, same horses
shape             two-arm comparison, 30x3 samples
expected signal   premium concentrates in the native-state phase
falsifier         premium present already in FULL_PARSE-only arm
DQ/MQ served      W2, MQ1 residual
note              diagnostic layer, not a new primary surface; keeps
                  primary qualification set unchanged
```

Sequencing: C-FULL-1 and C-FENCE-1 first (highest materiality per
FOLLOWUP-CANDIDATES), then C-REF-1, C-CONT-1, C-SMALL-1, C-CLEAN-1.
Each sweep freezes its spec + receipt chain like the #44 campaign before
execution; none may reuse the primary RunId.
