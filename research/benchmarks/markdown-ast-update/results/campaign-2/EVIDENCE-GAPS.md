# EVIDENCE-GAPS — MARKIT-31 Full Evidence Campaign-2

```text
authority     3762b7a42e1c284a4c2c2e0ebac8496e70c63431
study         MARKIT-31-FULL-EVIDENCE-STUDY
campaign      MARKIT-31-FULL-EVIDENCE-CAMPAIGN-2
```

Absence of measurement is not absence of effect. This file records every
place where the campaign measured less than it would have liked, and every
structural confound a reader must know about before reading a table. None
of these is a blocker; none of them was used to change a result.

---

## 1. Not measured at all

```text
CPU_TIME                UNAVAILABLE — no per-case CPU-time instrumentation
                        exists in the frozen substrate; `task-clock` appears
                        only in the process-level `perf stat` evidence.
ALLOC_COUNT             UNAVAILABLE — no allocator instrumentation.
ALLOC_BYTES             UNAVAILABLE — as above.
PEAK_MEMORY             UNAVAILABLE as a per-case metric. The memory lane
                        is process RSS only (see §3).
RETAINED_MEMORY         UNAVAILABLE as a per-case metric.
```

The frozen metric-qualification table carries these as `UNAVAILABLE` into
`CampaignSpecId`; nothing in the primary tables prints a fabricated zero.

## 2. eBPF

```text
STATUS   EBPF_UNAVAILABLE
reason   /proc/sys/kernel/unprivileged_bpf_disabled = 2
         bpftrace : NOT INSTALLED
         bpftool  : NOT INSTALLED
         ulimit -l = 8192 KiB
```

No kernel setting was modified to obtain eBPF, and no eBPF evidence was
fabricated. Consequences:

- unexpected syscalls, scheduler switches, CPU migrations, page faults,
  `mmap`/`munmap`/`brk` and allocator-call counts were **not** collected as
  eBPF evidence;
- process-level `page-faults:u`, `context-switches:u`, `cpu-migrations:u`
  from `perf stat` are the only available substitutes, and they are
  process-level, not region-level;
- CPU migrations during the measured region are therefore **unobserved**.
  Affinity (`taskset -c 1`) makes migrations structurally unlikely but this
  is an argument, not a measurement.

## 3. Memory lane is descriptive only

```text
label     DESCRIPTIVE_PROCESS_MEMORY
method    /proc/self/status (VmRSS, VmHWM, VmSize) + /proc/self/statm,
          read in the process that owns the state, strictly BETWEEN
          operations, in a process-isolated invocation per mode.
```

RSS is a **process** observation. It is not an object size, it is not a
per-case allocation total, and it cannot separate the mechanism's retained
state from the harness's own working set. Peak (`VmHWM`) is monotone over
the process lifetime, so it describes the heaviest point reached, not the
state under test. No allocator interposition was used, so the lane cannot
report allocation counts either.

## 4. PMU counters are multiplexed estimates

Six hardware events were requested per `perf_event_open` group while this
Xeon exposes four generic programmable counters, so the kernel
time-multiplexes them and **scales** each count. Two consequences:

- absolute cycle counts are reported **raw** and are never converted into
  time;
- only within-slot horse-to-horse comparisons (identical case, source,
  edit, binary, CPU) are used.

A consistency check on one slot (128 KiB controlled update, 30
repetitions) gives `cycles / in-process elapsed` ≈ 4.4 GHz, above this
CPU's turbo ceiling (3.5 GHz), i.e. the counters over-count relative to the
frozen in-process timer. This is recorded rather than smoothed away.
`cache-misses` counts in the low single digits for several slots and
should be treated as unreliable at this resolution.

Also unavailable at `perf_event_paranoid = 2`:

```text
kernel-scope counters     DENIED (all events are `:u` scoped)
hardware breakpoints      not requested
uncore / NUMA counters    not requested
```

## 5. Controlled-axis confounds (recorded, not hidden)

`N` is held fixed at 128 KiB for every axis except C-N, so the axis value
must come out of some other region. The absorbing region is listed here so
the confound cannot be mistaken for an effect:

```text
C-N   nothing absorbed — N is the variable. Padding blocks (128 B) are
      mechanically identical at every point; the target block index is
      blocks/2 so the relative edit position is ~0.5 at every point
      (0.623 at 512 B, converging to 0.500 at 16 MiB) — NOT exactly equal.
C-B   absorbed by the padding block count. Target block offset therefore
      moves (65 408 B at B=128 to 32 768 B at B=64 KiB), so "target
      relative location" is ~0.4995 at every point but the absolute
      position is not constant.
C-D   absorbed by the SUFFIX length after the reconvergence closer.
      Interior = D - 6 B, suffix = N - prefix - fence - D. The suffix is
      mechanically identical paragraph padding in every cell but its
      LENGTH changes with D, and its post-edit interpretation also flips.
C-F   nothing absorbed — F selects how many of 64 fixed slots carry a use.
      The definition-environment size, slot count, slot size and
      use-distance distribution are constant.
C-K   absorbed by the trailing padding. The edit sits on the container's
      FIRST content line, so its absolute offset moves by 2·K bytes
      (32 799 at K=0 to 32 823 at K=12) while its relative position moves
      from 0.250237 to 0.250420.
```

Additional known properties:

```text
C-D   the reconvergence closer is 5 backticks while the opener is 3, so
      pre-edit that line is ALSO a valid fence OPENER (any closer line is
      a valid opener — the grammar has no backtick line that is only one
      of the two). Pre-edit the region after it is therefore an unclosed
      fence body; post-edit it is ordinary content. The measured H4
      forward/convergence gauge is recorded per cell precisely because
      the nominal D is not the whole story.
C-B   B = 128 B is mechanically the same block size as the C-N padding
      unit, so C-B's smallest point and C-N's target block coincide in
      shape; they are different documents and are never pooled.
```

## 6. Workload coverage gaps

```text
real BREAK/RESTORE pair families available in the frozen #35 workload:
  G0-ATX-TO-PARAGRAPH  G0-BQ-NEST-LINE  G0-CODESPAN-DELIM-BREAK
  G0-EMPH-DELIM-BREAK  G0-FENCE-CLOSER-REMOVE  G0-LIST-ITEM-INDENT
  G0-REFDEF-REMOVE     (= 7 G0 families, all used)
  G1-TABLE-DELIM-BREAK (= G1; never dispatched into H0-H4)
```

The following frozen edit families have **no** exact
`base -> broken -> base` pair, so they have **no** real lifecycle trace:

```text
E1_LOCAL_TEXT            covered by the controlled L1/L2/L7 traces only
E2_PARAGRAPH_SPLIT_MERGE covered by the controlled L3 trace only
E5 LINK-DEST-BREAK       no real lifecycle trace (CODESPAN and EMPH
                         sub-families of E5 are covered)
```

Real lifecycle traces are therefore 7 (one per available G0 break
transition) plus the 7 controlled L1-L7 traces.

## 7. Lifecycle chain construction

Real lifecycle chains are `repeat(break, restore)` of one frozen pair.
That is a deterministic, grammar-valid, indefinitely repeatable chain in
which **every step is a real frozen payload**, but it is periodic: the
state alternates between exactly two documents. It cannot exercise
monotone document growth, and it cannot exercise a long aperiodic edit
history. The controlled L1-L7 traces are the aperiodic complement, and
they are synthetic 64 KiB documents, not real project files.

A lifecycle run of K = 128 is 128 chained edits on ONE state. The
checkpoint table is a cumulative sum of per-step p50s and is labelled
DERIVED — the campaign did **not** measure an enclosing wall-clock
interval for a whole lifecycle run.

## 8. Attribution coverage

The attribution lane covers construction (22 × 5), resident-update
(362 × 5) and the controlled cells (43 × 5). It does **not** cover the
lifecycle surface, so the per-step work counters (blocks reparsed, nodes
rebuilt/reused, metadata records touched, unique/old/post bytes inspected,
H1 fallback, H4 restart/convergence distances) for the chained lifecycle
edits are **not** in the raw evidence. Lifecycle evidence is timing plus
the final sealed state-representation export.

State-representation counts (task §29) are exported only at the END of a
lifecycle chain, and only for the fields each horse actually exposes; a
field a horse does not expose is the literal string `UNAVAILABLE`:

```text
H0  retained_blocks, retained_source_bytes
H1  retained_blocks (tiling entries), fragment_metadata_entries
    (definitions), retained_source_bytes
H2  retained_blocks (tree nodes), fragment_metadata_entries
H3  retained_blocks (tree nodes), old_tree_index_entries
H4  retained_blocks, checkpoints, fragment_metadata_entries (defs),
    retained_source_bytes
UNAVAILABLE per horse: the fields not listed above
```

## 9. Interpretation rules that constrain what the counters mean

Inherited from the PR #46 audit and frozen into `CampaignSpecId`:

```text
R1  blocks_reparsed counts fresh block-structure / skeleton units, NOT
    top-level blocks and NOT bytes reparsed.
R2  for H3, unique source inspection = parser work + reuse-vouch /
    consultation margin reads, so full source coverage does NOT imply
    full reparse.
R3  H4 convergence_distance must be read together with reuse: with
    nodes_reused > 0 it is the forward distance to accepted suffix reuse;
    with nodes_reused == 0 it is the forward parse distance to EOF. The
    gauge alone is meaningless.
R4  an `Unknown` on the H1 full_parse fallback slot is
    not-applicable-by-phase, NOT update fallback evidence.
```

## 10. Sampling and scale gaps

```text
Scales executed                every frozen point of every axis
                               (11 + 9 + 7 + 8 + 8 = 43 cells)
Scales not executed            none
Failed formal lanes            none
Lifecycle repetitions          30 (chosen from the pilot's cost data and
                               frozen before the first formal row)
Controlled repetitions         30 measured + 10 warmup (uniform, frozen)
Sessions                       3 for every surface; sessions are never
                               pooled into n = 90
```

`session p50 max/min > 1.5` is computed for every cell and travels with
the summary tables. Where it fires, the cell is FLAGGED only: nothing was
deleted, downweighted or rerun.

## 11. Machine and environment

```text
governor        schedutil — frequency is policy-controlled, not
                tool-controlled. The campaign records this host as
                controlled-by-policy-record, not controlled-by-tooling.
turbo           intel_pstate no_turbo=0 (ENABLED), inspected only.
                No campaign step disables or enables turbo.
SMT             enabled; affinity pins to one logical CPU of one physical
                core, so the SMT sibling is idle but not reserved.
NUMA            single node; no cross-node effect is measurable here.
MemTotal        diagnostic only (per the #43 corrective); never a gate.
frequency       per-session `scaling_cur_freq` is recorded in the
                environment telemetry, but the campaign cannot prove the
                frequency was identical across sessions.
```

The host is recorded as the SAME machine class as Campaign-1
(`primary-e5-xeon2666v3-fedora44`, CPU 1, core 1, siblings 1,11).
Many other processes run on this host; the campaign did not fence them
off, only pinned itself to one CPU.

## 12. Evidence that is deliberately absent

```text
No ranking, winner, speedup conclusion or design recommendation appears
in any raw file, receipt, summary table or this register. Those are
analysis claims and belong to the deferred fresh-context review
(`GPT6-FULL-EVIDENCE-HANDOFF.md`).
Campaign-1 raw and Campaign-1 identities were not read as authority and
were not modified.
PR #45 was not used as scientific authority.
```
