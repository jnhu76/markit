# PROFILING-RECORD-v1 — Stage-B profiling results (NON_PRIMARY, POST_HOC)

Status: `PROFILING_FOLLOWUP / NON_PRIMARY / POST_HOC_CASE_SELECTION`
(task §34, §47-§48). Nothing here modifies or replaces the frozen primary
result. Tools: perf 7.2.5 (perf_event_paranoid=2, user-space `:u` events
only, no kernel settings changed). bpftrace: **UNAVAILABLE**
(`kernel.unprivileged_bpf_disabled = 2`, no root assumed) — §42/§43
dynamic-system questions were therefore NOT answered.

Replay binary: `analysis/scripts/profiling-replay` (same mechanism crates
and same runner primitives as the campaign, analysis commit 0bf678c).
**PROFILING_BINARY != PRIMARY_BINARY** (separate diagnostic ELF, rustc
1.98.1 vs primary rustc 1.97.1); source semantics identical (task §35B).

Environment note (recorded, §37): the replay host-regime ran ~1.1-1.3x
slower than the primary sessions (schedutil frequency regime, turbo
policy unchanged and never modified). Replay ABSOLUTE times are therefore
not comparable to primary absolute times; per-slot WINNER/LOSER structure
reproduces the primary in every slot (replay-vs-primary.csv), which is
the fidelity check this layer relies on.

## Per-slot results (QUESTION -> RESULT -> INTERPRETATION)

Common methodology note: whole-process profiles are dominated by
frozen-workload materialization verification (sha256_hex), fresh-state
construction (shared-grammar parse + allocator churn) and oracle
validation, exactly as designed; mechanism-edit subtrees are a minority
of samples. Conclusions below are restricted to what survives that
dilution. Cycles-differential per-edit values
(`perf-stat-edit-region.csv`) are RETAINED AS RAW BUT DEEMED UNRELIABLE:
the in-loop fresh-state build inflates the subtracted baseline in a
regime-dependent way (P1: H0 589.5k vs H1 575.9k "edit cycles" contradicts
the reproducible 2.3x replay timing advantage), so NO cycle-count claim is
made. Raw perf stat files are committed for inspection.

| Slot | Question | Evidence | Result | Interpretation (bounded) |
| ---- | -------- | -------- | ------ | ------------------------ |
| P1 | H1 winner: where does residual go? | replay ratios; perf stat IPC/brm/cm; record P1-H1 | replay speedup 1.98 (primary 2.57); IPC 1.85 vs H0 2.14; brm 2.39% vs 1.78%; alloc family ~25% of process cycles | H1's win is consistent with genuinely avoided reparse; no cache-stall pathology (cache-miss rate < 0.2%). Allocation churn is the visible residual, not parsing. |
| P2 | H1 loser: pure fallback cost? | record P2-H1; attribution: fallback=1, inspected 14530B > H0 13511B | IPC 1.89 vs H0 1.94; brm 2.85% vs 2.21%; sha256+parse symbols dominate load; H1 replay 1.5x slower than H0 | Supports: fallback path re-parses everything PLUS block-scan/probe overhead; loss is real added work, not a timing artifact. |
| P3 | H3 prepare outlier | record P3-H3; old_tree symbols | `old_tree_subtree_reuse::{build_tnode, assemble_level, shift_forest, Cursor::consult}` visible with allocation-coupled frames (`in_place_collect`, `drop_glue`, `Arc::drop_slow`) | H3's ~1575ns median prepare (vs H2 82ns) is old-tree index construction; allocation-heavy. PROFILE_SUPPORTED interpretation of R2. |
| P4 | E6 semantic rematerialization (H3/H2) | perf stat brm: H3 3.72% / H2 3.46% (vs 0.4-1.8% other slots); attribution inspected ~11100B ≈ H0 | branch-miss rate roughly DOUBLES on E6 consultation vs all other slots; inspected bytes show near-full traversal despite structure reuse | Supports §18 reading: definition-environment invalidation forces broad re-consultation; pointer-heavy consultation (branchy), not bulk parsing. Scaling still NEEDS_CONTROLLED. |
| P5 | E6 anchor H2 | attribution: rebuilt 64/130 vs 6-27 typical | H2 rebuilds ~half the tree on this case | Same mechanism family as P4; consistent. |
| P6/P7 | CLEAN_STATE premium (H2/H4) | record P7-H4; replay-vs-primary 1.13/1.09 | clean build is parse-dominated (`memchr_lf`, `BlockScanner::run`, `scan_region_with_sink`) + oracle walk + allocation churn; H2 4817 vs H4 3825 build-only cycles/iter (unreliable diff, direction only) | The premium is consistent with building each mechanism's auxiliary state on top of the same parse; no single hotspot dominates. LEVEL 0 descriptive; attribution needs a dedicated controlled design (see plan C5). |
| P8 | H4 deep-container latency | perf stat P8: IPC 2.43, brm 0.81%, cmr 0.11% — cleanest profile of all slots; restart 1018 / conv 1173 of 1632B | H4 depth-3+ cost tracks convergence/restart distance over a small file; no memory stall, no fallback | Supports convergence-distance explanation for the depth-bin flip (R5); causal scaling NEEDS_CONTROLLED. |
| P9 | H4 winner sanity | replay speedup 3.38 (primary 4.25); inspected 271B vs H0 5849B | direction and magnitude class reproduce | The primary H4 win is consistent with avoided work; no measurement anomaly. |

## Evidence-level summary

```text
R2 (H3 prepare)                 LEVEL 3 — PROFILE_SUPPORTED
R3/E6 consultation profile      LEVEL 3 — PROFILE_SUPPORTED (mechanism only)
R1/R5/R6                        replay + counters consistent; LEVEL 2 unchanged
allocator-churn observation     PROFILE_ONLY_ALLOCATOR_EVIDENCE (task §45):
                                does NOT promote ALLOC_COUNT/ALLOC_BYTES to
                                primary-qualified metrics
IPC / branch-miss / cache-miss  exploratory ratios only (task §39); not
                                frozen campaign metrics
bpftrace                        UNAVAILABLE (unprivileged BPF disabled)
```

Raw artifacts: `perf-stat/` (stat CSVs), `replay-raw/` (30-iteration
replay JSONL), `replay-vs-primary.csv`, `perf-stat-edit-region.csv`
(unreliable, retained). External (not committed, preserved off-repo):
perf.data files with SHA256 in
`~/markit-analysis-logs/profiling-perf-data/perf-data-hashes.txt`; text
`perf report --stdio` extracts copied to `perf-report/`.
