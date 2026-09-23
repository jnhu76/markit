# GPT-6 FULL EVIDENCE HANDOFF — MARKIT-31 Campaign-2

```text
This document contains NO persuasive conclusion, NO ranking, NO winner,
NO design recommendation, and NO Weakness Map. It is the complete,
non-interpretive description of the evidence that exists, how it was
produced, and what it does not cover. Interpretation is yours.
```

```text
authority merge SHA   3762b7a42e1c284a4c2c2e0ebac8496e70c63431
StudyId               cf151a01b1a347210320d6d2cf920c3de2310612cd8f4aa2f5aadb01d4b19856
CampaignSpecId        bd89908c51b0724073bdb090c7b54a380e08bfc2a4240df4b296b26e47800990
```

---

## 0. Read in this order

```text
1. CAMPAIGN-2-FREEZE.md      the execution contract, frozen before any row
2. EVIDENCE-GAPS.md          every limitation and confound
3. CAMPAIGN-2-CLOSURE.md     identities, counts, hash closure, verdict
4. summary/*.csv             the mechanical tables (§6 below)
5. profiling/**              the profiling lane (§8 below)
6. raw JSONL + receipts      the primary evidence itself (§5 below)
```

---

## 1. Authority chain

```text
issue #22 MARKIT-MARKDOWN-BENCHMARK-1        umbrella research authority
#35 CORRECTIVE-C workload freeze             the ONLY real workload
                                             (never re-selected here)
#41 MEASUREMENT-CORRECTIVE-1                 timer boundary / schema v2 /
                                             110+1810 parity
PR #46 audit (algorithm + counter identity)  PASS, 0 P0, 0 P1
                                             -> interpretation rules R1-R4
PR #45                                       CLOSED / SUPERSEDED, NOT used
Campaign-1 (#31 primary run)                 historical evidence only,
                                             identities untouched
THIS campaign (#31 full evidence campaign-2) the evidence described here
```

Campaign-1 identities, unchanged and not reused:

```text
CampaignSpecId  ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2
RunId           905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d
```

## 2. H0-H4 as measured (nothing here is a claim about quality)

```text
H0  FULL_REBUILD                 h0-full-rebuild
    clean full parse of the source; no reuse of previous state.
H1  BLOCK_LOCAL_REPARSE          block-local-reparse-h1
    retains a top-level block tiling + a definitions array; reparses the
    damaged block region and reuses undamaged tiling entries.
H2  FRAGMENT_REUSE               fragment-reuse-h2
    retains a fragment tree; reuses fragments that a content-addressed
    comparison accepts.
H3  OLD_TREE_SUBTREE_REUSE       old-tree-subtree-reuse-h3
    retains the whole old normalized tree; reuses subtrees whose spans it
    can re-vouch.
H4  RESTART_CONVERGENCE          restart-convergence-h4
    retains top-level blocks with base offsets plus checkpoints; restarts
    at the edit and converges onto a reusable suffix.
```

Identical across the five: the same BENCH-GRAMMAR-v1 input language, the
same normalized result vocabulary, the same oracle, the same timer
boundary, the same work-counter sink.

## 3. Interpretation rules frozen into CampaignSpecId (task §6)

```text
R1  blocks_reparsed = fresh block-structure / skeleton units.
    NOT top-level blocks. NOT bytes reparsed.
R2  H3: unique source inspection = parser work + reuse-vouch / consultation
    margin reads. Full source coverage does NOT imply full reparse.
R3  H4 convergence_distance: read WITH reuse.
      nodes_reused > 0  -> forward distance to accepted suffix reuse
      nodes_reused == 0 -> forward parse distance to EOF
    Never interpret the gauge alone.
R4  H1 full_parse fallback slot Unknown = not-applicable-by-phase.
    Not update fallback evidence.
```

These are constraints on reading, not findings.

## 4. What was executed

```text
Surface A CONSTRUCTION     22 G0-strict FULL_READ files x H0-H4 x 3 sessions
                           x (10 warmup + 30 measured)
Surface B RESIDENT_UPDATE  362 G0_PRIMARY cases x H0-H4 x 3 sessions
                           x (10 warmup + 30 measured), SINGLE_RESET
Surface C LIFECYCLE        14 traces x H0-H4 x 3 sessions x 30 repetitions,
                           128 chained edits per run on ONE state,
                           verified after EVERY step
Surface D CONTROLLED       43 cells (N 11, B 9, D 7, F 8, K 8) x H0-H4
                           x 3 sessions x (10 warmup + 30 measured)
work counters              construction 110, resident-update 1810,
                           controlled 215, lifecycle 26,880 rows
memory                     DESCRIPTIVE_PROCESS_MEMORY, process-isolated
profiling                  12 matched slots, perf stat (10 rotated
                           repetitions) + region-scoped PMU counters +
                           perf record on 4 slots
```

Timer boundaries, exactly:

```text
construction   source resident -> start -> clean parse + native construction
               + seal -> usable -> stop
               (fs IO, oracle, normalize, checksum, serialization excluded)
resident_update / controlled
               fresh pre-edit state built OUTSIDE the timer -> start ->
               prepare_update -> update -> complete/seal -> black_box ->
               stop -> oracle outside
               T_total = T_prepare + T_native (arithmetic)
lifecycle      state built once per (trace, horse, rep), untimed; then per
               step the resident_update boundaries, NO reconstruction
```

## 5. Raw inventory and schemas

Raw root: `results/campaign-2/` (in the worktree), plus byte-identical
copies B and C (see §9).

```text
34 raw JSONL observation files, 1,091,615 rows, 2,431,080,966 bytes
274 raw artifacts in total, 2,494,862,669 bytes
```

Per-file SHA256, byte size, row count, lane, sub_campaign_spec_id, run_id,
StudyId and CampaignSpecId: `manifests/raw-inventory-v1.json` and
`.txt`. One receipt per raw file in `receipts/`.

Envelope (`campaign2-observation-v1`): a narrow identity wrapper around one
existing schema-v2 result row.

```text
schema, study_id, campaign_spec_id, sub_campaign_spec_id, run_id,
evidence_class, surface, lane, session_id?, session_ordinal?,
case_order_ordinal, horse_order_ordinal, horse_id, sample_kind,
iteration_ordinal, observation_id,
cell?                   (controlled only: axis, point index, label, value,
                         cell_id, generator id + version)
lifecycle?              (lifecycle only: trace_id, family, chain construction,
                         step, step_count, rep, checkpoint?,
                         cumulative_edits, step_label, transition_label)
state_repr?             (retained_blocks, checkpoints,
                         fragment_metadata_entries, old_tree_index_entries,
                         retained_source_bytes, provenance; each field is a
                         number or the literal "UNAVAILABLE")
result_row_v2           (the frozen schema-v2 row; Rust authority
                         runner/src/result.rs — unmodified)
```

Result-row schema v2 is unchanged from the frozen substrate. The
attribution payload is ATTRIBUTION-SCHEMA-v2 with `Observed<u64>` slots
representing `Known(n)` as a number and the distinctions `"UNKNOWN"` /
`"NOT_APPLICABLE"` as literal strings — `Known(0)` is never conflated with
either.

Evidence classes (task §54), never mixed:

```text
PRIMARY_TIMING / PRIMARY_WORK / PRIMARY_LIFECYCLE
CONTROLLED_TIMING / CONTROLLED_WORK
PROFILE_PERF / PROFILE_EBPF (empty) / PROFILE_ALLOCATOR (empty)
DESCRIPTIVE_MEMORY / ENVIRONMENT_TELEMETRY / DERIVED_MECHANICAL
```

## 6. Mechanical summary tables (DERIVED_MECHANICAL)

Reproducible by `tools/summarize.py` from raw. No analysis, no ranking.

```text
summary/construction-case-horse.csv              110 rows
summary/resident-update-case-horse.csv         1,810 rows
summary/controlled-{N,B,D,F,K}-cell-horse.csv    43 cells
summary/lifecycle-step-p50.csv                26,880 rows
summary/lifecycle-cumulative-derived-p50.csv  26,880 rows  DERIVED
summary/lifecycle-checkpoint-derived-p50.csv     560 rows  DERIVED
summary/attribution-{construction,resident-update}.csv
summary/attribution-controlled-{N,B,D,F,K}.csv
summary/attribution-lifecycle.csv              8,960 rows
```

Columns that matter:

```text
session{0,1,2}_p50_ns            nearest-rank rank 15 of the 30 measured
session{0,1,2}_p95_ns            nearest-rank rank 29 of the 30 measured
case_estimate_p50_ns             median of the three session p50s
h0_relative_speedup_geomean      geometric mean of per-session
                                 p50(H0)/p50(Hx)  (>1 means faster than H0)
session_p50_max_over_min         stability diagnostic; present only when
                                 the ratio exceeds 1.5
```

Sessions are never pooled into n = 90. Lifecycle cumulative and checkpoint
values are DERIVED sums of per-step p50s, not a measured enclosing
wall-clock trace. `iteration_ordinal` on the lifecycle surface is
`rep * 128 + step`.

Counters present in the attribution CSVs (with their frozen R1-R4 reading):
`blocks_reparsed`, `nodes_rebuilt`, `nodes_reused`,
`metadata_records_touched`, `unique_old_source_bytes`,
`unique_post_source_bytes`, `source_bytes_inspected_total`,
`parse_amplification_*`, H1 fallback slots, H4 restart and convergence
distance slots. Lifecycle attribution carries the same counters per
(trace, step, horse) and a `state_repr` block per step.

## 7. Controlled geometry (what varies and what absorbs it)

```text
C-N  N = 512B,1K,2K,4K,8K,16K,32K,64K,128K,1M,16M
     fixed: 128 B affected block, LOCAL_TEXT +8 B, target ~middle,
            depth 0, no refs, no fences. Nothing absorbed.
C-B  N = 128 KiB; B = 128B..64KiB; padding absorbs N-B.
C-D  N = 128 KiB; D = 64B..64KiB; suffix absorbs.
     D = pre-edit distance from the edited closer to the reconvergence
     closer. Edit = 3 backtick bytes at a FIXED offset for every D.
C-F  N = 128 KiB; F = 0..64 over 64 fixed 64-byte slots; nothing absorbed.
     Edit = delete the 32-byte definition line.
C-K  N = 128 KiB; K = 0..12 blockquote depth; suffix absorbs.
     Sibling width 62 B and sibling count 256 both constant.
```

Full per-cell geometry:
`manifests/controlled-cells-v1.txt`. Entanglements and their consequences
are listed in `EVIDENCE-GAPS.md` §5 (including the C-D structural note
that any fence closer line is also a valid opener in this grammar).

## 8. Profiling methodology and raw data

```text
selection      profiling/PROFILING-SELECTION-v1.md  (POST_HOC, NON_PRIMARY,
               written AFTER primary collection and BEFORE any perf run)
capability     profiling/PERF-CAPABILITY-AUDIT-v1.txt
perf stat      profiling/perf-stat/slot-P*-perf-stat.txt
               events cycles:u,instructions:u,branches:u,branch-misses:u,
               cache-references:u,cache-misses:u,task-clock,page-faults,
               context-switches,cpu-migrations
               10 repetitions per horse per slot, horse order rotated
region scope   profiling/perf-stat/slot-P*-region.jsonl
               (per-repetition counters read through perf_event_open with
                exclude_kernel, baseline sampled WHILE DISABLED)
setup-only     profiling/perf-stat/slot-P*-setuponly.jsonl
               (the measured region is empty — the validating companion)
perf record    profiling/perf-record/slot-P*-{H0..H4}-report.txt
               cycles:u @999 Hz, call-graph dwarf, release binary
```

Matched rule honoured: inside a slot, every horse replays the SAME case,
source, edit, binary and CPU. No `H3 case A` vs `H0 case B` comparison
exists anywhere in the profiling lane.

Known limitation: at `perf_event_paranoid = 2` the PMU counters are
user-scoped and multiplexed; absolute cycle counts are raw and are never
converted to time (see `EVIDENCE-GAPS.md` §4).

## 9. Raw copies

```text
COPY A  worktree: .../markdown-ast-update/results/campaign-2   (primary)
COPY B  /dev/shm/campaign-2-copy-b         independent filesystem (tmpfs)
COPY C  /home/jnhu/markit-campaign-2-raw   durable, outside any worktree
```

`tools/verify-copies.py`: 274/274 files byte-identical on all three copies,
0 mismatches, `SECOND_COPY_VERIFIED = YES`. Copy B is volatile (tmpfs);
copy C shares the btrfs data volume with copy A. No off-volume durable copy
exists (the two spare physical disks could not be mounted without
interactive authorization).

## 10. Failed / excluded evidence

```text
failed formal lanes        none
invalid raw markers        none
correctness failures       0
deleted / imputed / retried rows  none
excluded from headline     the pilot (NON_RESEARCH, results/campaign-2/pilot/)
                           perf record outputs (never in a primary table)
                           the smoke row set (logs/00-runner-smoke-validation.log)
```

## 11. Evidence gaps (full detail in EVIDENCE-GAPS.md)

```text
EBPF                UNAVAILABLE (unprivileged_bpf_disabled=2, no bpftrace)
CPU_TIME            UNAVAILABLE
ALLOC_COUNT/BYTES   UNAVAILABLE
PEAK/RETAINED MEM   UNAVAILABLE as per-case metrics; RSS is descriptive only
kernel-scope PMU    DENIED by perf_event_paranoid=2
cache-misses        low single digits; unreliable at this resolution
real lifecycle      only 7 G0 break transitions have exact pairs;
                    E1_LOCAL_TEXT, E2_PARAGRAPH_SPLIT_MERGE and
                    E5 LINK-DEST-BREAK have NO real lifecycle trace
lifecycle chain     real chains are periodic repeat(break,restore)
lifecycle counters  covered by the dedicated lifecycle attribution lane at
                    every step of every trace; the lane runs once per
                    (trace, step, horse, session) rather than once per
                    timing repetition, because the counters are
                    deterministic mechanism facts and the TIMING
                    variability lives in the separate PRIMARY_LIFECYCLE
                    lane
state_repr          exported at EVERY step in the lifecycle attribution
                    lane (26,880 exports) and only at the LAST step of
                    each chain in the lifecycle timing lane; per horse the
                    exposed fields differ and the rest are the literal
                    string UNAVAILABLE
memory lane         process RSS only; not object size; no allocator data
frequency           schedutil; recorded per session, not proven identical
```

## 12. Questions for the fresh-context review

```text
Q1   What is C_h, the construction cost of each mechanism?
Q2   What is U_h, the resident-state update cost by real workload regime?
Q3   How does long-lived state alter cost over K edits?
Q4   When does construction overhead amortize?
Q5   Where are the real N/B/D/F/K crossovers?
Q6   Which wins/losses are explained by avoided parsing,
     consultation/vouching, metadata work, semantic rematerialization,
     fallback, restart/convergence, allocation, or CPU effects?
Q7   Which primary observations are supported by matched perf evidence?
Q8   Does lifecycle history change mechanism ranking?
Q9   What state/machinery appears unnecessary?
Q10  What should the final Markit design preserve/remove/combine?
```

Answer them from `summary/*`, the raw JSONL and `profiling/*` — not from
this document, which deliberately states no result.

```text
FINAL_RESEARCH_ANALYSIS = NOT_STARTED
READY_FOR_GPT6          = YES
```
