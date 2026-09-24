# H4-LARGE-N-CAUSE-REPORT — H4 large-N update cost attribution (Issue #50)

```text
result class:      POST_HOC_EXPLANATORY / H4_ONLY / RESIDENT_SINGLE_RESET
authority base:    master @ 334eea6201fc0258e35a7c5b21feb722641ddcbd
diagnostic implementation / analysis executable authority:
                   6b2c2a4 (branch research/50-h4-large-n-cause-1; the
                   producing binaries are frozen under bin/ with SHA256s
                   in receipts/FROZEN-EXECUTABLES.txt and in every raw
                   receipt row)
results commit:    45ad068 (raw evidence, first report, receipts)
corrective commit: THIS commit — derived PMU root summary fixed and the
                   microarchitectural attribution narrowed; raw evidence
                   untouched, nothing re-collected
current PR head:   Draft PR #51, branch research/50-h4-large-n-cause-1
                   (moves with documentation commits; it is NOT the
                   provenance of any measurement)
collection:        attempt 5 (attempts 1-4 archived under attempt-*-superseded/,
                   each with the defect that forced the re-run)
H4_LARGE_N_CAUSE_RESULT = PASS
```

## 0. Question and method (one paragraph)

Campaign 2 measured that H4's resident update grows from ~86 µs at
N = 128 KiB to ~25.0 ms at N = 16 MiB (×290 for ×128 blocks) while a
single block is edited. This campaign determines WHICH operations cause
that growth with parser work held fixed. The method: a diagnostic copy
of the frozen A0 H4 mechanism (`h4diag/`), proven byte/clock/counter
equivalent to the original on all 8 cells before timing; five separate
measurement lanes (uninstrumented timing, mutually exclusive phases,
work counters, counting allocator, single-factor ablations); PMU
counters and sampling scoped to the resident update; kernel tracepoint
and eBPF probes for scheduler/fault/allocator suspects. All
correctness work stays strictly outside every timer.

## 1. Answer in one paragraph

**The parser is not the cause; the retained representation is.** A0
inspects a constant 283 source bytes and rebuilds 2 nodes in every
update at every N (parser work fixed by construction and verified).
But the update also walks and re-materializes O(M) retained state:
every one of the M top-level blocks has its pair→slot entry rebuilt, a
checkpoint record created, its `ContextKey` cloned, its `Arc` handle
cloned and (next update) released; the retained prefix and suffix
forests are re-walked to reassemble the block list; the (empty)
definition table is re-collected by walking all M skeletons and
re-compared against the retained table; the whole old state is
retired. Those five O(M) streams explain ~96% of U at 16 MiB. The
executed instruction count stays linear (constant ~260–275
instructions per block at every N); the super-linear latency growth
(×36 for ×16 over 1→16 MiB) is strongly associated with a
**cache-capacity / memory-hierarchy amplification**: at the same
2–4 MiB regime transition where the ~100–150 MB resident representation
stops fitting in the LLC, per-block LLC misses rise from ≈0 to 9.4 and
cycles/instruction rise from 1.10 to 2.57, while the competing
scheduler/fault/kernel suspects are excluded (§5). What these counters
do NOT resolve is the precise stall-level partition of the excess
cycles (§9); that is recorded as microarchitecturally unresolved in §7.

## 2. Workload and invariants (frozen)

128-byte paragraph blocks (126 content bytes + LF + LF), N ∈
{128 KiB … 16 MiB}, edit = zero-length 8-byte insert `"zzzzzzzz"` at
`target·128 + 63` (block middle), depth 0, no references/fences. Cells
reuse the Campaign-2 controlled-N construction; the three overlapping
points are byte-identical to `generate_cell(Axis::N, …)` (verified in
`cells.jsonl`). Per-cell pre-timing gates, all PASS at all N
(`preflight.jsonl`, `equivalence.jsonl`):

| gate | value |
|---|---|
| fresh blocks / fresh nodes | 1 / 2 at every N |
| restart distance (local) | 63 B |
| forward distance (local) | 136 B |
| actual suffix take | YES (1) |
| convergence before EOF | YES |
| correctness | `normalize(H4 result) == normalize(H0 clean full parse(post))` — PASS, 8/8 |
| equivalence to original A0 | checksums AND work counters identical, 8/8 cells |

## 3. Lane results

### 3.1 U_PLAIN (uninstrumented A0, original mechanism + runner boundary)

3 sessions × (10 warmup + 30 measured), balanced randomized order with
duplicate A0 control labels, pinned to CPU 1. Case estimate = median
of session p50s:

| N | U_PLAIN p50 | growth |
|---|---|---|
| 128 KiB | 86.3 µs | ×1 |
| 256 KiB | 169.7 µs | ×1.97 |
| 512 KiB | 343.4 µs | ×3.98 |
| 1 MiB | 693.9 µs | ×8.04 |
| 2 MiB | 1.539 ms | ×17.8 |
| 4 MiB | 4.717 ms | ×54.6 |
| 8 MiB | 12.10 ms | ×140.2 |
| 16 MiB | 24.99 ms | ×289.6 |

128 KiB→1 MiB is linear (×8.0 for ×8). 1→16 MiB is ×36.0 for ×16 —
the nonlinearity under diagnosis. Conservative A/A noise (p95 of
per-round |Δ| between the two identical A0 labels, floored at 5%):
9.9% (16 MiB) to 21.5% (128 KiB); the A0-dup control's measured effect
stays inside ±2.1% everywhere, so the p95 threshold is conservative.

### 3.2 U_PHASE (mutually exclusive phases; PHASE-MAP.md frozen first)

At 16 MiB (p50s; full table in `u-phase-summary.csv`, derived
`derived-level-16mib.csv`, `derived-growth-1-to-16.csv`):

| phase | p50 @16 MiB | share | share of 1→16 increment | class |
|---|---|---|---|---|
| P6 pairs→slots + checkpoints | 7.02 ms | 26.6% | 28.2% | **DOMINANT** |
| P5 fresh materialization + suffix assembly | 5.67 ms | 21.5% | 22.8% | **DOMINANT** |
| P3 prefix pair assembly | 5.31 ms | 20.2% | 21.2% | **DOMINANT** |
| P7 seal + retirement | 3.26 ms | 12.4% | 12.9% | MATERIAL |
| P1 prepare: damage scan + restart selection | 3.07 ms | 11.6% | 12.4% | MATERIAL |
| P4 definition collect + table compare | 2.27 ms | 8.6% | 9.1% | MATERIAL |
| P2 forward parse + convergence | 4.0 µs | 0.02% | 0.01% | NEGLIGIBLE |
| closure residual | 2.6 µs | 0.01% | — | NEGLIGIBLE |

The phases close: Σ(disjoint) accounts for 99.99% of U_PHASE at every
cell. `PHASE_PERTURBATION = NONE`: U_PHASE/U_PLAIN overhead is
+2.7%…+10.1% per cell, below each cell's max(5%, conservative A/A
noise) threshold (median ratios in `u-phase-vs-plain.csv`).

### 3.3 W_COUNTERS (measured visits, not final-state metadata)

All linear in M, all 8 cells, exact formulas verified
(`work-counters.csv`, `work-counters-per-block.csv`):

```text
damage_records_visited      = M          definition_nodes_visited = M
prefix_slots_visited        = M/2        suffix_slots_visited     = M/2 − 1
slots_created               = M          checkpoint_records_created = M
checkpoint_key_clones       = M          arc_handles_cloned/released = M
metadata_records_touched    = 3.5·M + 4            (3 588 → 458 756)
nodes_reused                = 2·M − 2              (2 046 → 262 142)
source_bytes_inspected_total = 283  CONSTANT   blocks_reparsed = 1
nodes_rebuilt               = 2  CONSTANT
```

These are the Campaign-2 reconciliation quantities: the 3.5·M+4
metadata touches and 2·M−2 reused nodes the earlier campaign reported
as growth are here attributed to the P1/P3/P4/P6 loops above, not to
parsing.

One counter-lane-only observation (also visible in `perf record`):
computing `nodes_reused` requires walking the retained inline forest —
an O(M) tree walk that A0 performs inside the frozen measured region
even though the timing sink discards the value. It is part of A0's
real cost (≈21% of sampled cycles at 16 MiB), reported here because
the frozen counter contract places it there.

### 3.4 A_ALLOCATOR (diagnostic-only counting allocator; never mixed with U_PLAIN)

Median p50s across 3 sessions (`allocator.csv`,
`allocator-summary.csv`):

| quantity | 1 MiB | 16 MiB |
|---|---|---|
| alloc calls / realloc calls per update | 13 / 11 | 13 / 15 |
| fresh requested bytes per update | 0.66 MB | 10.49 MB |
| realloc churn (new requested) per update | 1.31 MB | 20.97 MB |
| peak live requested (incl. retained B0) | 85.8 MB | 369.9 MB |

The per-block byte traffic is concentrated exactly in the dominant
phases: prefix assembly ≈ 128 B × M, fresh materialization ≈ 128 B × M
(one new block + the edited block's payload), slots + checkpoints ≈
128 B × M — i.e. **3 × 128 B × M fresh allocations plus a realloc
doubling chain (~2× the largest Vec)** ≈ 52 MB of allocator traffic
for one 16 MiB update. Retirement allocates nothing (refcount
decrements); `final_payload_destructions = 1`.

### 3.5 Ablations (single-factor, interleaved with A0 + A0-dup in one balanced schedule)

| ablation | what it removes | effect (median across sessions) | verdict |
|---|---|---|---|
| **Adefs** | definition traversal + table compare when maintained state proves `definition_count == 0` | −8.8% at 16 MiB (−4.2% at 128 KiB, growing ~linearly) | consistent in 3/3 sessions at every cell; at 16 MiB just under the 9.9% conservative threshold (`CONSISTENT_BELOW_THRESHOLD`) — matches P4's 8.6% share |
| **Adrop** | O(M) retirement via ownership-correct deferral (move, no pre-clone) | −12.4% at 16 MiB (`REACHES_THRESHOLD`) — **but** `T_drain` = 19.07 ms at 16 MiB (0.65 ms at 1 MiB), so DERIVED(U_deferred + T_drain) ≈ 41 ms ≫ 24.99 ms | retirement is real, non-boundary-artifact O(M) work; deferral only moves it out of the timer and makes it more expensive (cache-cold bulk drop) |
| **Acapacity** | Vec growth/reallocation via exact-capacity preallocation | −1%…−4% | consistent but always below threshold — the 21 MB realloc churn costs only ~2-4% of U |

The A0-duplicate control stays inside ±2.1% with INCONSISTENT
direction across sessions — i.e. the schedule's noise floor behaves
like noise.

## 4. PMU evidence (`perf/`, issue §12–§13)

**perf stat** — 5 groups × 8 cells × {user, root}, 15 rounds each,
window opened/closed by `--delay=-1 --control=fifo` around the resident
update only. **Every group in every invocation reported
running/enabled = 100.00%** (≥ 0.99 required): no multiplexing, no
`PMU_GROUP_UNRELIABLE` (`perf/stat-reliability.txt`). The derived
summary `perf/stat-summary.csv` is produced deterministically from the
raw `perf/stat/*.json` by `perf/derive-stat-summary.py`, regression-
tested by `perf/test-derive-stat-summary.py` (raw root counters
non-zero ⇒ derived root summary non-zero; known raw→summary numeric
case). CORRECTIVE: the summary first shipped in 45ad068 carried all-zero
root rows — its inline generator matched events only under their
user-mode perf names (`cycles:u`), silently defaulting the root files'
plain names (`cycles`) to 0. The raw root files were always valid; only
the derived summary was wrong, and it has been regenerated from the
unchanged raw evidence. An earlier all-LLC+TLB single group WAS
multiplexed (49–75%) and was split into C1/C2 as the record shows.

The table quotes the user-privilege rows. The root (kernel-inclusive)
rows corroborate them at every cell — at 16 MiB: 91.0 M vs 88.9 M
cycles/update (+2.4%, the kernel-side context-switch/fault cost), CPI
2.579 vs 2.567, LLC-miss/block 9.44 vs 9.38 — with the regime transition
at exactly the same cells.

| N | cycles/upd | instr/upd | cyc/blk | CPI | LLC-miss/blk | dTLB-miss/blk |
|---|---|---|---|---|---|---|
| 128 KiB | 0.31 M | 0.28 M | 302 | 1.10 | 0.001 | 0.04 |
| 1 MiB | 2.41 M | 2.15 M | 294 | 1.12 | 0.007 | 1.33 |
| 2 MiB | 5.37 M | 4.28 M | 328 | 1.26 | 0.41 | 1.42 |
| 4 MiB | 17.1 M | 8.54 M | 521 | 2.00 | 4.57 | 1.41 |
| 8 MiB | 41.5 M | 17.1 M | 634 | 2.44 | 7.66 | 1.48 |
| 16 MiB | 88.9 M | 34.6 M | 678 | 2.57 | 9.38 | 1.50 |

Instructions per update grow ×16.1 for ×16 M (linear; per-block
constant). Cycles per update grow ×36.9. LLC-load-misses per update
grow ×22 789 (54 → 1.23 M). Page faults grow ×16 (7 → 119, all minor);
context switches: 0 in user-mode counters, 15–37 per 15-update window
in root mode (≈1–2.5 per update); migrations 0. The knee is at the
2–4 MiB cells, exactly where the ~1 KB/block resident representation
exceeds L2/LLC.

**perf record** (1 MiB ×300 / 16 MiB ×20 windows, `-F 4000 --call-graph
dwarf`, `perf/stat` scope rule applies; percentages QUALITATIVE): at
16 MiB, sampled cycles sit in `update` self 37.8%, the retained-forest
node count feeding `nodes_reused` (`map_fold`) 21.1%, `prepare_update`
(damage scan/restart) 11.6%, old `Vec<DiagBlockSlot>` drop glue 10.5%,
`collect_defs_skel` 8.2%, `skel_count` 5.9%, `realloc`+`memmove` 3.8%.
The hotspot set is the O(M) representation loops of §3.2 — no parse
symbol appears.

## 5. eBPF / kernel-trace evidence (`ebpf/`, issue §14–§16)

`EBPF = COMPLETE`, `EBPF_ATTEMPT = AVAILABLE`, `EBPF_PERTURBED = NO`
(with_BPF/without_BPF median native ratio = 1.0040; traced/untraced
software-event run = 0.988). bpftrace 0.24.2 + bpftool 7.6.0 installed
from Fedora repos **after** all timed lanes finished; every load ran
under authorized sudo; `unprivileged_bpf_disabled=2` and
`perf_event_paranoid=2` were **not changed**. Probe results: 0 thread
migrations, 73 switches per batch (scheduler excluded); all faults
minor, 0 major (fault storms excluded); 31 mmap / 4 109 brk / 0 mremap
per batch (allocator address-space churn excluded — the heap grows by
brk a few times per update). Full record: `ebpf/summary.md`.

## 6. Microkernels

`MICROKERNELS = NOT_NEEDED` — the integrated evidence (phase
decomposition + counters + targeted ablations + PMU + sampling) leaves
no specific causal ambiguity that a microkernel would resolve
(`selection-notes/07-microkernels.md` records the per-candidate
decision).

## 7. Causal classification (issue §19)

DOMINANT (share ≥ 15% at 16 MiB AND ≥ 10% of the 1→16 increment, each
backed by counter growth and phase evidence):

- **P6 pairs→slots/checkpoint re-materialization** — 26.6% share,
  28.2% of increment; M slots + M checkpoints + M ContextKey clones
  counted and ~10.5 MB allocated per 16 MiB update.
- **P5 fresh materialization + suffix assembly** — 21.5%, 22.8%;
  the O(M) suffix re-walk/re-take dominates, fresh-region cost is O(1).
- **P3 prefix pair assembly** — 20.2%, 21.2%; M/2 prefix slots
  re-walked and re-paired per edit.

MATERIAL (share ≥ 8% or ablation-confirmed):

- **P7 seal + retirement** — 12.4%; Adrop REACHES_THRESHOLD (−12.4%)
  and proves the cost is real work (deferral + drain is 64% more
  expensive in total).
- **P1 damage scan + restart selection** — 11.6%; M-record reverse
  scan counted (`damage_records_visited = M`).
- **P4 definition traversal + table compare** — 8.6%; Adefs −8.8%
  consistent in 3/3 sessions at every cell.

SECONDARY:

- **Vec capacity churn** (Acapacity −1…−4%; ~2-4% of U).

NEGLIGIBLE:

- **P2 forward parse + convergence** (0.02% — parser work fixed:
  283 bytes, 1 block, 2 nodes at every N).
- **page faults / scheduler / address-space churn** (eBPF lane).
- **closure residual** (0.01%).

REPRESENTATION_CAUSAL_UNRESOLVED: **none.** Every phase ≥ 2% has
counter evidence + phase evidence, and P4/P7 additionally have targeted
ablations; the phase closure residual is 0.01%.

MICROARCHITECTURAL_UNRESOLVED: **the precise decomposition of the
excess CPI / memory-stall cycles.** The collected PMU groups establish
*that* the working set leaving the cache coincides with the CPI rise
(§4, §9), but they do not independently partition the excess cycles
into LLC-latency, memory-level-parallelism, prefetch,
downstream-memory, TLB, allocator or other backend-stall components.
That partition is UNRESOLVED; it was not needed for the
representation-level classification above, which rests on the phase,
counter and ablation evidence only.

## 8. Fundamental vs accidental (issue §20)

| cost | class | evidence |
|---|---|---|
| reparse 1 block + 2 nodes, inspect ~283 B | FUNDAMENTAL | P2 = 0.02% of U; counters constant |
| convergence predicate over the edit's block neighbourhood | FUNDAMENTAL | O(1) counted checks |
| correctness proof (normalize == H0, checksum) | FUNDAMENTAL but OUTSIDE timing | frozen boundary; strictly post-timer |
| M× slot/checkpoint/ContextKey re-materialization | ACCIDENTAL (representation) | P6 + counters + allocator bytes |
| retained prefix/suffix re-walk to re-pair blocks | ACCIDENTAL (representation) | P3 + suffix half of P5 |
| empty-definition re-traversal per edit | ACCIDENTAL (maintained-state waste) | P4 + Adefs |
| O(M) old-state retirement per edit | ACCIDENTAL (no cross-update structure sharing) | P7 + Adrop/T_drain |
| O(M) damage scan | ACCIDENTAL (no offset index) | P1 + `damage_records_visited = M` |
| O(M) `nodes_reused` counter walk inside the measured region | ACCIDENTAL (attribution inside the hot path) | perf record 21% sampled cycles |
| ~52 MB allocator traffic per 16 MiB update | ACCIDENTAL (consequence of the above) | allocator lane |

About 96% of U at 16 MiB is accidental representation tax.

## 9. The 1→16 MiB nonlinearity explained (issue §21)

M grows ×16; U grows ×36. Decomposition of the increment (medians):

1. **Work is linear.** Instructions/update ×16.1; all counted event
   streams are exact linear forms in M (§3.3). The linear fit
   T = α + β·M on the 128 KiB–1 MiB cells has r² ≥ 0.9998 for every
   phase.
2. **The constant per block inflates.** cycles/block rises 294 → 678
   because the resident representation (~M × ~1 KB incl. payload,
   slots, checkpoints, skeletons) leaves the LLC: LLC-misses/block
   0.007 → 9.38, CPI 1.12 → 2.57.
3. **Excess cycles account for the gap — in aggregate only.** Linear-fit
   extrapolation predicts 11.1 ms at 16 MiB; measured 25.0 ms; gap
   13.9 ms. The total-excess-cycle model — actual cycles minus
   instructions × baseline CPI (88.9 M − 34.6 M × 1.10 ≈ 50.8 M cycles
   ≈ 14.3 ms at the measured 3.56 GHz) — accounts for the same gap
   within 3%. This is NOT a quantitative closure of the cache cause:
   both quantities measure the same excess relative to small-N
   behaviour (one in wall-clock, one in cycles). The agreement
   establishes that the added latency is delivered as added cycles per
   unit of work — a memory-hierarchy slowdown, not extra executed work
   and not a timing artifact — and the coincident sharp rise of LLC
   misses per block at the same regime transition (§4) strongly
   supports the cache/memory hierarchy as its source. The stall-level
   composition of those 50.8 M cycles is not partitioned by the
   collected counters.

```text
ALGORITHMIC_WORK_RESULT:
The amount of representation work remains linear in M.
There is no evidence of a second super-linear algorithmic work term
within the measured regime.

MICROARCHITECTURAL_RESULT:
The large-N nonlinear latency amplification is strongly associated with
the working set leaving cache: CPI and LLC misses per block rise
sharply at the same 2–4 MiB regime transition, while competing
scheduler/fault/kernel explanations are not supported (§4–§5).

PRECISE_STALL_DECOMPOSITION:
UNRESOLVED. The current PMU data does not independently partition
excess cycles into LLC-latency, memory-level-parallelism, prefetch,
downstream-memory, TLB, allocator or other backend-stall components.
```

Consequently no quantitative stall-attribution residual is claimed.
The nonlinearity is a cache/memory-hierarchy amplification of a linear
amount of representation work — not an algorithmic surprise and not
parser work — but the exact share of the excess cycles attributable
specifically to LLC misses (as opposed to other memory-hierarchy
components) is not closed by this evidence.

## 10. V1 implications (diagnostic mandates only — no design, no V1 implementation)

V1_MUST_REMOVE (each traceable to measured evidence above):

- per-edit O(M) slot/checkpoint/ContextKey re-materialization (P6);
- per-edit O(M) retained prefix/suffix re-walk (P3 + suffix of P5);
- per-edit O(M) definition traversal when `definition_count == 0`
  (P4; maintain the count incrementally, skip in O(1));
- per-edit O(M) old-state retirement (P7; retire incrementally or
  share structure across updates — Adrop proves pure deferral loses);
- O(M) damage scan (P1; index blocks by offset instead of a
  full reverse scan);
- O(M) attribution walks inside the measured region (the
  `nodes_reused` forest walk; move attribution out of the hot path or
  maintain incrementally).

V1_STILL_UNPROVEN (explicitly not decided here):

- that any specific representation (persistent tree, piece/rope
  chains, hierarchical checkpoints) makes all six taxes O(log M) or
  O(edit) **while preserving** losslessness and the
  ContextKey/restart-predicate semantics at splice boundaries;
- that the cache-capacity nonlinearity disappears rather than moving
  (a pointer-chasing structure can be super-linear in a different
  constant — must be measured under the #22 protocol);
- TLB/allocator behaviour at workspace sizes beyond 16 MiB;
- anything about production integration (AGENTS.md: this is benchmark
  research; the architecture document stays frozen).

## 11. Provenance and integrity

- Base `334eea6`; implementation commits `f52fe5a` + `6b2c2a4`
  (diagnostic crate only; frozen crates byte-identical to base —
  `git diff 334eea62 HEAD -- {common,instrumentation,oracle,runner,
  shared-grammar,mechanisms}` is empty). Results/report commit
  `45ad068`; a later documentation-only corrective commit fixes the
  derived root PMU summary and narrows the microarchitectural
  attribution (§4, §7, §9) — it changed no raw evidence and re-collected
  nothing. The current PR head of Draft #51 moves with such
  documentation commits and is deliberately NOT the provenance of any
  measurement: provenance for every number here is the frozen
  executable SHA (below) plus the raw receipt rows, not the branch tip.
- Producing executables: four feature-isolated binaries, frozen in
  `bin/`, SHA256s in `receipts/FROZEN-EXECUTABLES.txt` and recorded
  per lane in `logs/*.log` **and** inside every raw JSONL receipt row.
  Attempts 1–4 were archived, not silently re-reported
  (`attempt-3-mixed-provenance/`, `attempt-4-superseded/`).
- Toolchain rustc/cargo 1.98.1, release profile (opt-level 3, thin
  LTO, codegen-units 1), no RUSTFLAGS, System allocator except the
  allocator lane; host: Fedora 44, 7.2.5 kernel, 20 CPUs, benchmark
  pinned to CPU 1 (full record in `H4-LARGE-N-CAUSE-1-MANIFEST.md`,
  `environment.txt`, `machine-pinning.txt`).
- `RAW_HASH_CLOSURE`: `receipts/PRODUCER-RECEIPTS.csv` maps every
  deliverable to its producing executable SHA and evidence log;
  `analyze.py` and `derive-tables.py` are deterministic over the
  committed raw JSONL, as are `perf/derive-stat-summary.py` over the
  committed raw `perf/stat/*.json` (regression-tested by
  `perf/test-derive-stat-summary.py`).
