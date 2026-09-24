# H4-LARGE-N-CAUSE-REPORT — H4 large-N update cost attribution (Issue #50)

```text
result class:      POST_HOC_EXPLANATORY / H4_ONLY / RESIDENT_SINGLE_RESET
authority base:    master @ 334eea6201fc0258e35a7c5b21feb722641ddcbd
                   (merged Campaign-2 evidence authority, PR #47)

workspace / source state at collection:
                   HEAD f52fe5a PLUS uncommitted h4diag working-tree
                   changes. The formal collection (attempt 5) ran
                   2026-09-24T02:07:09Z .. 02:22:34Z, i.e. while HEAD was
                   f52fe5a; the working-tree changes it ran from were
                   committed only afterwards, as 6b2c2a4, at
                   2026-09-24T02:38:48Z. No commit SHA is the provenance
                   of a measurement (see below).
diagnostic implementation commit:
                   f52fe5a — *h4diag: diagnostic instrumentation for H4
                   large-N cost attribution (#50)*
collection-correction commit (committed AFTER collection):
                   6b2c2a4 — *h4diag: fix window ordering, counter
                   attribution and allocator tagging for the formal
                   collection*. Its content is the source state attempt 5
                   ran from, but it is NOT the HEAD that produced the
                   binary, and no byte-level binary->commit mapping can be
                   re-derived from the repository (there is no reproducible
                   build). It is recorded as source state only.
producing executable SHA256 (the actual executable authority):
                   4d23df55… (U_PLAIN, ablations, PMU, perf record, eBPF),
                   8185f825… (U_PHASE), aa731757… (W_COUNTERS),
                   fd81e27d… (A_ALLOCATOR); full hashes in
                   receipts/FROZEN-EXECUTABLES.txt, in every raw receipt
                   row and in every lane log. The binaries are frozen
                   under bin/ and were first committed in 45ad068.
results commit:    45ad068 (raw evidence, first report, receipts, binaries)
corrective commits: c6c52bb + 84bc333 + THIS commit — documentation and
                   derived-summary layer only; raw evidence untouched,
                   nothing re-collected
final PR head:     Draft PR #51, branch research/50-h4-large-n-cause-1
                   (moves with documentation commits; it is NOT the
                   provenance of any measurement)
collection:        attempt 5 (attempts 1-4 archived under attempt-*-superseded/,
                   each with the defect that forced the re-run)
H4_LARGE_N_CAUSE_RESULT = PASS
```

## 0. Question and method (one paragraph)

Campaign 2 **discovered** the large-N degradation regime: in its
controlled-N lane the H4 resident update grew from ~87.5 µs at N = 128 KiB
to ~29.3 ms at N = 16 MiB (×334 for ×128 blocks; ×43 over 1→16 MiB) while
a single block is edited. Issue #50 **reproduced and diagnosed** it in a
fresh scoped experiment: this campaign's own uninstrumented baseline
measures ~86.3 µs → ~25.0 ms (×289.6 for ×128 blocks; ×36.0 over
1→16 MiB) on this host. The two are different collections with different
absolute latencies and different repetition structure; the numbers must not
be interchanged, but the qualitative regime is the same. This campaign
determines WHICH operations cause that growth with parser work held fixed.
The method: a diagnostic copy of the frozen A0 H4 mechanism (`h4diag/`),
proven result-checksum and work-counter equivalent to the original on all
8 cells before timing (`equivalence.jsonl`) — the *timing authority* is the
original frozen mechanism itself, run by `run-plain` (§3.1, §4); five
separate measurement lanes (uninstrumented timing, mutually exclusive
phases, work counters, counting allocator, single-factor ablations); PMU
counters and sampling scoped to the resident update; kernel tracepoint and
eBPF probes for scheduler/fault/allocator suspects. All correctness work
stays strictly outside every timer.

## 1. Answer in one paragraph

**The parser is not the cause; the retained representation is.** A0
inspects a constant 283 reported source bytes — that is the scanner's
*total* inspection count, 138 of which are unique post-source bytes
(re-inspection counted; the two metrics are not interchangeable, §2) —
and rebuilds 2 nodes in every update at every N (parser work fixed by
construction and verified). But the update also walks and re-materializes
O(M) retained state: every one of the M top-level blocks has its
pair→slot entry rebuilt, a checkpoint record created, its `ContextKey`
cloned, its `Arc` handle cloned and (next update) released; the retained
prefix and suffix forests are re-walked to reassemble the block list; the
(empty) definition table is re-collected by walking all M skeletons and
re-compared against the retained table; the whole old state is retired.
Those O(M) streams account for essentially all of the measured update: the
only fundamental measured phase (P2, forward parse + convergence) is
0.02% of U_PHASE at 16 MiB (§7, §8). The executed
instruction count stays linear (constant ~260–275 instructions per block
at every N), so the *amount* of representation work stays linear in M.

The super-linear **latency** growth (×36.0 for ×16 over 1→16 MiB) is
therefore not extra executed work. What the evidence supports is an
association with the memory hierarchy, at exactly this strength:

```text
ALGORITHMIC WORK
    linear in M; no second super-linear algorithmic work term is visible
    in the measured regime.
COST PER RECORD
    rises in the large-N regime: cycles/block 294 -> 678, LLC
    misses/block 0.007 -> 9.38, CPI 1.12 -> 2.57.
MEMORY HIERARCHY
    STRONGLY SUPPORTED as the amplification mechanism: the rise of CPI
    and of LLC misses per block coincides with the same 2-4 MiB regime
    transition. It is an association with the regime transition, not an
    arithmetic closure.
EXACT STALL PARTITION
    UNRESOLVED: the collected counters do not partition the excess
    cycles into LLC-latency, memory-level-parallelism, prefetch,
    downstream-memory, TLB, allocator or other backend-stall components.
```

Two quantitative boundaries are recorded so the claim is not read as
sharper than it is. First, the resident representation is already far
larger than the recorded 25 MiB L3 at *every* measured N (the allocator
lane measures 69.1 MB of requested live payload at update start for
128 KiB rising to 348.9 MB at 16 MiB), so this is not a simple
fits/does-not-fit threshold and no such threshold is claimed. Second, the
kernel-side lane found no evidence of a regime transition large enough to
explain the scaling, but did not *quantify-exclude* the kernel suspects
(§5, narrowed wording).

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
| parser source footprint | 138 unique post-source bytes, 140 unique total source bytes, **283 total inspection bytes** (re-inspection counted), all constant at every N |
| correctness | `normalize(H4 result) == normalize(H0 clean full parse(post))` — PASS, 8/8 |
| equivalence to original A0 | checksums AND work counters identical, 8/8 cells |

The 138 / 283 distinction matters and is used consistently below: 138 is
the unique post-source footprint the parser must read; 283 is the
diagnostic scanner's reported total inspection count (it re-reads bytes at
region boundaries). "Parser work is fixed" holds for both — each is
constant at every N — but the two numbers are not substitutes for each
other, and 283 is not a statement about the parser's unique input size.

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

Median-of-sessions at 16 MiB (full table in `u-phase-summary.csv`, derived
`derived-level-16mib.csv`, `derived-growth-1-to-16.csv`). The `class`
column is the **frozen Issue #50 §14 rule**, not a post-hoc threshold:
DOMINANT requires a reliable phase share or single-factor net effect of at
least **50%**:

| phase | p50 @16 MiB | share of U_PHASE | share of 1→16 increment | frozen-rule class |
|---|---|---|---|---|
| P6 pairs→slots + checkpoints | 7.02 ms | 26.6% | 28.2% | MATERIAL |
| P5 fresh materialization + suffix assembly | 5.67 ms | 21.5% | 22.8% | MATERIAL |
| P3 prefix pair assembly | 5.31 ms | 20.2% | 21.2% | MATERIAL |
| P7 seal + retirement | 3.26 ms | 12.4% | 12.9% | MATERIAL |
| P1 prepare: damage scan + restart selection | 3.07 ms | 11.6% | 12.4% | MATERIAL |
| P4 definition collect + table compare | 2.27 ms | 8.6% | 9.1% | SECONDARY |
| P2 forward parse + convergence | 4.0 µs | 0.02% | 0.01% | NEGLIGIBLE |
| closure residual | 2.6 µs | 0.01% | — | NEGLIGIBLE |

```text
LARGEST MATERIAL CONTRIBUTORS:
    P6, P5, P3

DOMINANT under the frozen rule:
    none
```

No single operation dominates alone; three O(M) reconstruction streams
(P6 + P5 + P3 = 68.3% of U_PHASE at 16 MiB) jointly account for most
representation-update cost. P7 and P1 also clear the 10% MATERIAL bar;
P4's 8.6% share and its −8.7% Adefs net effect both fall below it, so P4
is SECONDARY under the frozen rule, not MATERIAL. Measured shares are
reported unchanged — only the labels were corrected (`derived-classification.csv`
carries the machine-checkable form, including `basis`).

The phases close **per observation** (never across p50s): Σ(disjoint)
accounts for 98.8–98.9% of U_PHASE at 128 KiB and 99.99% at 16 MiB, with
an absolute residual of 904–3 014 ns at every cell — so the residual is
largest *relatively* only at the smallest cells, where a whole update is
~90 µs. `PHASE_PERTURBATION = NONE`, with one recorded exceedance: the
per-(session, cell) `U_PHASE/U_PLAIN` ratio spans 0.884…1.155, which is
inside each cell's `max(5%, conservative A/A noise)` threshold except at
16 MiB session 2, where the ratio is 0.884 (11.6% deviation against a
9.9% threshold). That excursion belongs to the *comparator* lane, not to
the phase probes: the plain lane's own 16 MiB session-2 p50 is 29.8 ms
against 24.8 / 25.0 ms in sessions 0/1 (case-estimate session max/min =
1.195), while the phase lane's three 16 MiB sessions agree to within
1.022 (26.16–26.73 ms). No phase share is transferred to U_PLAIN; every
share in the table above is a share of U_PHASE. Per-cell ratios are in
`u-phase-vs-plain.csv`.

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

One counter-lane observation that is **also original-mechanism work**
(verified in the frozen source, not only in the diagnostic copy):
computing `nodes_reused` requires walking the retained inline forest.
The original `RestartConvergenceMechanism::update` calls
`retained_block_nodes(&s.block)` at two sites
(`mechanisms/restart-convergence/src/lib.rs:511` and `:606`), and that
helper is `skel_count(&b.skel) + inline_forest_count(&b.sem)` — the O(M)
skeleton + retained-inline-forest walk. It runs inside `update()`, i.e.
inside the frozen measured region (`T_native = update + complete +
black_box`), even though the timing sink discards the value. The frozen
U_PLAIN binary is compiled with that code: `markit_mdbench_restart_convergence::inline_node_count`
and its `map_fold` adapter are present as symbols in
`bin/mdbench-h4diag`.

The magnitude of that walk is measured on the **equivalent diagnostic
copy**, not on U_PLAIN: `perf record` attributes 21.1% self of sampled
cycles at 16 MiB to the `map_fold` frame feeding `inline_node_count`.
Per the mechanism-scope note at the top of §4, the copy is somewhat
slower than the original at large N, so
this percentage is qualitative evidence of the walk's weight, not an
exactly transferable share of U_PLAIN.

### 3.4 A_ALLOCATOR (diagnostic-only counting allocator; never mixed with U_PLAIN)

Median p50s across 3 sessions (`allocator.csv`,
`allocator-summary.csv`). Every byte figure is **requested payload bytes
of tracked allocations** as reported by the counting allocator — not RSS,
not resident bytes, and not DRAM traffic. The window is the update region
only, opened after the fresh-state baseline B0 is established (Issue #50
§7); realloc-internal temporary double-buffering is not observable and is
not included.

| quantity | 1 MiB | 16 MiB | per block @16 MiB |
|---|---|---|---|
| alloc / realloc / dealloc calls per update | 13 / 11 / 13 | 13 / 15 / 13 | — |
| fresh requested bytes (`alloc_requested`) | 0.66 MB | 10.49 MB | ≈ 80 B × M |
| realloc: **new** requested bytes | 1.31 MB | 20.97 MB | ≈ 160 B × M |
| realloc: old requested bytes | 0.66 MB | 10.49 MB | ≈ 80 B × M |
| **freed** requested bytes (realloc-old + final payloads) | 1.31 MB | 20.97 MB | ≈ 160 B × M |
| total requested (alloc + realloc-new) | 1.97 MB | 31.46 MB | ≈ 240 B × M |
| live requested at window start | 84.5 MB | 348.9 MB | — |
| live requested at window end | 84.5 MB | 348.9 MB | net change 0 |
| peak live requested (incl. retained B0) | 85.8 MB | 369.9 MB | — |
| peak growth (peak − start) | 1.31 MB | 20.97 MB | ≈ 160 B × M |

The lane's own byte tags locate the fresh streams exactly:

```text
cat_fresh_materialization   80.0 B × M   (10 488 096 B at 16 MiB, 7 calls)
cat_prefix_assembly         80.0 B × M   (10 485 440 B, 15 calls)
cat_slots_checkpoints       80.0 B × M   (10 485 760 B, 2 calls)
cat_forward_parse               504 B    (constant at every N)
cat_defs_table / cat_retirement / cat_seal_other / cat_untagged = 0 B
```

So the three fresh streams are **≈ 80 B × M each, ≈ 240 B × M in
total**, not 128 B × M each. 128 B is the *source block unit*; it is not
the allocation granularity this lane measures, and "3 × 128 B × M" would
overstate each stream by ~1.6×. The tag names are the lane's; the
per-block ratios are what the counters report.

Retirement allocates nothing (refcount decrements);
`final_payload_destructions = 1`.

The "~52 MB" figure, defined exactly: it is `total_requested_bytes +
freed_requested_bytes` = 31.46 MB + 20.97 MB = 52.4 MB. Those two
quantities are **not disjoint** — a reallocated buffer is counted once as
requested (inside `total_requested_bytes`, as realloc-new) and again as
freed (inside `freed_requested_bytes`) — so 52.4 MB is the sum of the
lane's two requested-byte totals, not 52 MB of distinct bytes, and it is
still not DRAM traffic. The measurable cost of the capacity churn it
represents is bounded by the `Acapacity` ablation (§3.5):
−0.7%…−2.7% of U per cell.

### 3.5 Ablations (single-factor, interleaved with A0 + A0-dup in one balanced schedule)

| ablation | what it removes | effect (median across sessions) | verdict |
|---|---|---|---|
| **Adefs** | definition traversal + table compare when maintained state proves `definition_count == 0` | **−8.7%** at 16 MiB (session range −8.7%…−8.8%); −5.4% at 128 KiB; **not monotonic** across cells (median per cell spans −4.7%…−8.7%) | direction-consistent in 3/3 sessions at every cell; at 16 MiB just under the 9.9% conservative threshold (`CONSISTENT_BELOW_THRESHOLD`) — consistent with P4's 8.6% phase share |
| **Adrop** | O(M) retirement via ownership-correct deferral (move, no pre-clone) | −12.4% at 16 MiB (`REACHES_THRESHOLD`) — **but** `T_drain` = 19.07 ms at 16 MiB (0.65 ms at 1 MiB), so DERIVED(U_deferred + T_drain) = 23.08 + 19.07 = **42.1 ms** ≫ 24.99 ms | retirement is real, non-boundary-artifact O(M) work; deferral only moves it out of the timer and makes it more expensive (cache-cold bulk drop). The sum is a derived value, not an enclosing wall interval (Issue #50 §10) |
| **Acapacity** | Vec growth/reallocation via exact-capacity preallocation | median −0.7%…−2.7% per cell (per-session range −0.4%…−4.4%) | direction-consistent at 6 of 8 cells and **INCONSISTENT at 2 MiB and 4 MiB** (session 0 / session 2 reverse sign); always below threshold. The ~21 MB realloc churn costs a few percent of U at most |

The A0-duplicate control stays inside ±2.1% with INCONSISTENT
direction across sessions — i.e. the schedule's noise floor behaves
like noise. Per-session ratios and verdicts are in
`ablations-effects.csv`; drain times in `ablations-drain.csv`.

Adefs is the one ablation whose *intervention* is a removal of its
phase's own work, and its net effect (−8.7%) sits just below the 10%
MATERIAL bar, which is why P4 is classified SECONDARY rather than
MATERIAL. Adrop clears the bar for P7. Acapacity is not a removal of P6's
work — it changes the capacity policy of the `pairs` vector P6 consumes —
so it is recorded as its own capacity-churn suspect, not as P6's effect.

## 4. PMU evidence (`perf/`, issue §13)

**Mechanism scope of this lane (read this before using any number below).**
`perf stat` and `perf record` are driven by `pmu-run`, which runs the
**equivalent diagnostic copy** (`H4Diag`); the lane's own raw rows record
`"mechanism":"diag-copy"`. The primary latency authority remains
**U_PLAIN on the frozen original H4** (§3.1), which is a different code
path in the same binary (`run-plain` → `RestartConvergenceMechanism`). The
copy is equivalent in checksum and work counters on all 8 cells, but not
identical in timing: at 16 MiB the PMU lane's median total is 25.81 ms
against U_PLAIN's 24.99 ms (+3.3%), while at 1 MiB they agree (694.2 µs
vs 693.9 µs, +0.04%). PMU and perf-record data are therefore used for
**qualitative / regime-level microarchitectural support**; their
cycle-share percentages are not transferred exactly to U_PLAIN.

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

Instructions per update grow ×16.1 for ×16 M (linear; per-block constant).
Cycles per update grow ×36.9. LLC-load-misses per update grow from
**54 per update at 1 MiB to 1.23 M at 16 MiB (×22 789)**; taken over the
full range the growth is ×1.08 M (1.13 per update at 128 KiB → 1.23 M at
16 MiB) — the ×22 789 figure has a 1 MiB baseline, not a 128 KiB one, and
is labelled that way here. Page faults grow ×17 (7 → 119 per update, all
minor in the windowed lane); context switches: 0 in user-mode counters,
15–37 per 15-update window in root mode (≈1–2.5 per update); migrations 0.

The knee is at the 2–4 MiB cells. What is recorded about the memory
hierarchy is: L1d 32 KiB per core, L2 256 KiB per core (2.5 MiB per
socket), **L3 25 MiB** (`environment.txt`); and the allocator lane
measures the retained representation at 84.5 MB of requested live payload
at 1 MiB rising to 348.9 MB at 16 MiB. The representation is therefore
already far above L3 at every measured N, so the knee is *not* a simple
"stops fitting" threshold and no such threshold is claimed; the supported
statement is the association between the CPI / LLC-miss rise and the same
2–4 MiB regime transition (§9).

**perf record** (1 MiB ×300 / 16 MiB ×20 windows, `-F 4000 --call-graph
dwarf`, `perf/stat` scope rule applies; percentages QUALITATIVE and, per
the scope note above, measured on the diagnostic copy). The figures below
are perf's **self** column, which is disjoint across symbols; they must
not be added to a parent's *children* value (e.g. `update` shows self
37.8% and children 40.3%, and `map_fold` runs inside it — adding 40.3%
and 21.1% would double-count). At 16 MiB: `update` self 37.8%, the
retained-forest node count feeding `nodes_reused` (`map_fold`) 21.1%,
`prepare_update` (damage scan/restart) 11.6%, old `Vec<DiagBlockSlot>`
drop glue 10.5%, `collect_defs_skel` 8.2%, `skel_count` 5.9%,
`realloc`+`memmove` 3.8%.

The hotspot set is the O(M) representation loops of §3.2. **No parser
symbol appears — with a stated qualification**: the field is a sample of
~2 000 samples over a run whose forward parse is O(1) (one block, 2 nodes,
283 inspection bytes), so absence here is weak evidence on its own; it
means "the parser is far below the sampling resolution", which is
consistent with, and not independent of, P2's 0.02% phase share. Frames
attributed to bare addresses (`[unknown]`, `0x2`, `0xffff`) are DWARF
call-graph artifacts of this build and carry no symbol-level conclusion.

## 5. eBPF / kernel-trace evidence (`ebpf/`)

An additional kernel-side lane beyond the issue's mandated set, recorded
with its reason per issue §17.

`EBPF = COMPLETE`, `EBPF_ATTEMPT = AVAILABLE`, `EBPF_PERTURBED = NO`
(with_BPF/without_BPF median native ratio = 1.0040; traced/untraced
software-event run = 0.988). bpftrace 0.24.2 + bpftool 7.6.0 installed
from Fedora repos **after** all timed lanes finished; every load ran
under authorized sudo; `unprivileged_bpf_disabled=2` and
`perf_event_paranoid=2` were **not changed**.

**The conclusion this lane supports is narrow:**

> The eBPF/kernel lane found no evidence of CPU migration, of major-fault
> storms, or of a kernel-side regime transition large enough to explain
> the observed nonlinear scaling.

Stronger statements are **not** supported, because of the scopes the
probes actually had. Recorded explicitly:

- The bpftrace counts are **whole-process aggregates over one
  ~20-update batch**, not per-window values, and they are not
  resume-filtered: `@sw` 73 `sched_switch` events (`next_comm ==`
  benchmark), `@mig` 0 `sched_migrate_task`, `@pf` 276 385
  `page_fault_user`, `@mmap` 31, `@munmap` 19, `@brk` 4 109,
  `@mremap` 0.
- The `sched` trace was taken with `perf record -a` and is
  **system-wide**: its `perf script` output shows the *profiler's own*
  process being switched and migrated (`perf:43290`, migrated by
  `migration/0`). Those events are not the benchmark's. So scheduler
  activity is *not evident as a cause* — which is weaker than
  "scheduler excluded".
- `@brk` 4 109 over 20 updates is **≈205 `brk` syscalls per update**, and
  the raw trace shows the break moving both up and down. The earlier
  claim that "the heap grows by brk a few times per update" is not
  supported by this count and is withdrawn.
- Address-space churn is **not** absent: the raw trace shows large
  anonymous `mmap`/`munmap` pairs (`len` ≈ 1.04–2.10 MB) at the scale
  glibc uses for the large reallocated vectors. The lane shows this
  activity was not quantified as a cost; it does not show that it is
  absent, and "allocator irrelevant" is not claimed.
- Two fault lanes exist and they do **not** share a scope. The eBPF
  `page_fault_user` count above is whole-process over the batch, whereas
  the windowed `perf stat` S1/S2 run — scope identical to the PMU lane
  (`--delay=-1 --control=fifo`, update region only, 30 counted windows)
  — reports 13 056 page-faults = 13 056 minor-faults and **0
  major-faults**. The two figures must not be compared numerically; they
  differ because they count different things. "0 major faults" is
  supported **for the windowed scope**, not as a per-window statement
  about the whole process.
- CPU migration is 0 in both lanes (windowed `cpu-migrations` = 0;
  `@mig` = 0). That is the one suspect this lane excludes within its
  scope.

Full record: `ebpf/summary.md`, whose interpretation section carries the
same narrowing as this report.

## 6. Microkernels

`MICROKERNELS = NOT_NEEDED` — the integrated evidence (phase
decomposition + counters + targeted ablations + PMU + sampling) leaves
no specific causal ambiguity that a microkernel would resolve
(`selection-notes/07-microkernels.md` records the per-candidate
decision).

## 7. Causal classification (frozen issue §14 thresholds)

The labels below are the **frozen Issue #50 §14 rule**: DOMINANT requires a
reliable mutually-exclusive phase share **or** a single-factor net effect of
at least **50%**; MATERIAL requires ≥10% (and, for an intervention basis,
an effect above `delta_N`); SECONDARY is a distinguishable contribution
below MATERIAL; NEGLIGIBLE is only "small at this N/regime and at this
resolution". Machine-checkable form, with the qualifying `basis` per
suspect: `derived-classification.csv`.

```text
DOMINANT under the frozen rule:
    none
```

No single operation dominates alone. The largest contributors are three
O(M) reconstruction streams, which together account for 68.3% of U_PHASE
at 16 MiB:

- **P6 pairs→slots/checkpoint re-materialization** — MATERIAL; 26.6% of
  U_PHASE, 28.2% of the 1→16 increment. M slots + M checkpoints + M
  `ContextKey` clones counted, and one of the three ≈80 B × M allocation
  streams. Basis: phase share.
- **P5 fresh materialization + suffix assembly** — MATERIAL; 21.5%,
  22.8%. The O(M) suffix re-walk/re-take dominates; the fresh-region cost
  is O(1). Basis: phase share.
- **P3 prefix pair assembly** — MATERIAL; 20.2%, 21.2%. M/2 prefix slots
  re-walked and re-paired per edit. Basis: phase share.

MATERIAL, further:

- **P7 seal + retirement** — MATERIAL; 12.4%, 12.9%. Adrop
  `REACHES_THRESHOLD` (−12.4%), so this has an intervention basis as well
  as a phase-share basis. The deferral is not a win: DERIVED(U_deferred +
  T_drain) = 42.15 ms = 1.69× U_PLAIN's 24.99 ms (also 1.60× the ablation
  lane's own A0 p50 of 26.34 ms).
- **P1 prepare: damage scan + restart selection** — MATERIAL; 11.6%,
  12.4%. The M-record reverse scan is counted
  (`damage_records_visited = M`). Basis: phase share.

SECONDARY:

- **P4 definition traversal + table compare** — 8.6% of U_PHASE (9.1% of
  the increment) and its intervention Adefs nets −8.7%: both below the
  10% MATERIAL bar, so SECONDARY, not MATERIAL.
- **Vec capacity churn** — Acapacity median −0.7%…−2.7% per cell
  (direction-INCONSISTENT at 2 and 4 MiB); a distinguishable contribution
  well below MATERIAL.

NEGLIGIBLE (supported by a reliable phase plus the relevant ablation
evidence, not by a zero difference alone):

- **P2 forward parse + convergence** — 0.02%. Parser work fixed for both
  byte metrics: 138 unique post-source bytes / 283 total inspection bytes,
  1 block, 2 nodes at every N.
- **Unphased closure residual** — 0.01% at 16 MiB (see the closure
  paragraph in §3.2 for the small-N values).
- **Kernel-side suspects** — migration excluded within the windowed scope;
  the remaining scheduler/fault/address-space statements are narrowed in
  §5 and are "not evident as a cause", not "excluded".

```text
REPRESENTATION_CAUSAL_UNRESOLVED: none
```
Every phase with a share ≥ 2% has counter evidence and phase evidence, and
P4/P7 additionally have targeted ablations; P6's adjacent capacity probe is
recorded separately rather than counted as P6's own effect. The phase
closure residual is 0.01% at 16 MiB (0.56–1.23% at 256 KiB and below).

```text
MICROARCHITECTURAL_UNRESOLVED: the precise decomposition of the
excess CPI / memory-stall cycles.
```
The collected PMU groups establish *that* the CPI rise and the working set
leaving cache coincide (§4, §9), but they do not independently partition the
excess cycles into LLC-latency, memory-level-parallelism, prefetch,
downstream-memory, TLB, allocator or other backend-stall components. That
partition is UNRESOLVED; it was not needed for the representation-level
classification above, which rests on the phase, counter and ablation
evidence only.

## 8. Fundamental vs accidental (issue §16)

| cost | class | evidence |
|---|---|---|
| reparse 1 block + 2 nodes, inspect 138 unique / 283 total bytes | FUNDAMENTAL | P2 = 0.02% of U_PHASE; both byte counters constant at every N |
| convergence predicate over the edit's block neighbourhood | FUNDAMENTAL | O(1) counted checks |
| correctness proof (normalize == H0, checksum) | FUNDAMENTAL but OUTSIDE timing | frozen boundary; strictly post-timer |
| M× slot/checkpoint/ContextKey re-materialization | ACCIDENTAL (representation) | P6 + counters + allocator bytes |
| retained prefix/suffix re-walk to re-pair blocks | ACCIDENTAL (representation) | P3 + suffix half of P5 |
| empty-definition re-traversal per edit | ACCIDENTAL (maintained-state waste) | P4 + Adefs |
| O(M) old-state retirement per edit | ACCIDENTAL (no cross-update structure sharing) | P7 + Adrop/T_drain |
| O(M) damage scan | ACCIDENTAL (no offset index) | P1 + `damage_records_visited = M` |
| O(M) `nodes_reused` walk inside the measured region | ACCIDENTAL (attribution inside the hot path) | frozen source `lib.rs:511,606` + diagnostic-copy sampling 21.1% (§3.3, §4) |
| ≈52.4 MB requested + freed payload bytes per 16 MiB update | ACCIDENTAL (consequence of the above) | allocator lane (§3.4; sum not disjoint, not DRAM traffic) |

The only measured FUNDAMENTAL phase is P2 at 0.02% of U_PHASE, so
essentially all of the measured 16 MiB update is non-parse representation
work, spread over six O(M) streams (P1, P3, P4, P5, P6, P7). Per-phase
shares are shares of the same enclosing window computed from
median-of-sessions p50s, so they are **not additive** (Issue #50 §15);
the additive quantity is the per-observation closure of §3.2, whose
residual is 0.01% at 16 MiB.

## 9. The 1→16 MiB nonlinearity explained (issue §14–§15)

M grows ×16; U grows ×36. Decomposition of the increment (medians):

1. **Work is linear.** Instructions/update ×16.1; all counted event
   streams are exact linear forms in M (§3.3). The descriptive linear fit
   T = α + β·M on the 128 KiB–1 MiB cells has r² ≥ 0.9988 for every phase
   **except P2**, whose r² is 0.9767 — P2 is a near-constant 1.5–4.5 µs
   bracket, not a β·M series, so a linear fit is not meaningful for it
   (`derived-nonlinear.csv`). The fit is descriptive only: not a causal
   proof, and not an asymptotic bound (Issue #50 §14).
2. **The constant per block inflates.** cycles/block rises 294 → 678
   while the per-block instruction count stays flat (262 → 264):
   LLC misses/block 0.007 → 9.38, CPI 1.12 → 2.57, with the sharp part of
   the rise at the 2–4 MiB cells. Nothing here asserts a cache-capacity
   threshold: the retained representation already exceeds the recorded
   25 MiB L3 at every measured N (§4).
3. **The excess is delivered as extra cycles — in aggregate only.**
   Linear-fit extrapolation predicts 11.1 ms at 16 MiB; measured 25.0 ms;
   a gap of 13.9 ms. The total-excess-cycle view — actual cycles minus
   instructions × small-N baseline CPI (88.9 M − 34.6 M × 1.10 ≈ 50.8 M
   cycles) — converts to 14.3 ms at the effective cycle rate of
   88.9 M / 24.99 ms = 3.56 GHz. That 3.56 GHz is a **derived
   cycles-per-wall-second rate, not a measured clock** (the host records
   2.90 GHz base, 3.50 GHz max, `schedutil` scaling). The two figures
   agree to ~3% **because they measure the same excess in different
   units** — one in wall-clock, the other in cycles. This is a
   consistency check between two aggregate views of one quantity: it is
   NOT an independent second estimate, NOT a partition of stalls, and NOT
   a residual closure of the cache cause. What it establishes is only
   that the added latency arrives as added cycles per unit of work — a
   memory-hierarchy slowdown, not extra executed work and not a timing
   artifact.

```text
ALGORITHMIC_WORK_RESULT:
The amount of representation work remains linear in M.
There is no evidence of a second super-linear algorithmic work term
within the measured regime.

MICROARCHITECTURAL_RESULT:
CACHE_CAPACITY_AMPLIFICATION = STRONGLY_SUPPORTED. The large-N
nonlinear latency amplification is associated with the memory
hierarchy: CPI and LLC misses per block rise sharply at the same
2-4 MiB regime transition, at a working set that already exceeds the
recorded 25 MiB L3 at every measured N. This is an association with
the regime transition, not an arithmetic closure.

KERNEL_SUSPECTS:
The eBPF/kernel lane found no evidence of CPU migration, major-fault
storms, or a kernel-side regime transition large enough to explain the
observed nonlinear scaling. Scheduler / fault / address-space suspects
are "not evident as a cause" within their recorded scopes, which is
weaker than "excluded" (§5).

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

- Base `334eea6`. Source state at collection: `f52fe5a` as HEAD **plus
  uncommitted `h4diag` working-tree changes**, committed only after
  collection ended as `6b2c2a4` (see the header block for the exact
  timeline). The diagnostic diff is the `h4diag/` crate plus one
  workspace member; the frozen crates are byte-identical to base —
  `git diff 334eea62 HEAD -- {common,instrumentation,oracle,runner,
  shared-grammar,mechanisms}` is empty. Results/report commit `45ad068`
  (which first committed `bin/`); then documentation-only correctives
  `c6c52bb` + `84bc333` + this commit, covering the derived root PMU
  summary (§4), the microarchitectural attribution (§7, §9), the
  classification labels (§7), the allocator arithmetic (§3.4) and the
  closure/reproducibility defects below. None of them touched raw
  evidence or re-collected anything. The PR head of Draft #51 moves with
  such documentation commits and is deliberately NOT the provenance of
  any measurement: provenance for every number here is the frozen
  executable SHA plus the raw receipt rows, not any commit SHA.
- Producing executables: four feature-isolated binaries, frozen in
  `bin/`, SHA256s in `receipts/FROZEN-EXECUTABLES.txt` and recorded
  per lane in `logs/*.log` **and** inside every raw JSONL receipt row.
  `run-plain` runs the **original frozen** `RestartConvergenceMechanism`;
  `pmu-run` (PMU, perf record, eBPF lanes) runs the **equivalent
  diagnostic copy** and records `"mechanism":"diag-copy"` in its rows.
  Attempts 1–4 were archived, not silently re-reported
  (`attempt-3-mixed-provenance/`, `attempt-4-superseded/`).
- Toolchain rustc/cargo 1.98.1, release profile (opt-level 3, thin
  LTO, codegen-units 1), no RUSTFLAGS, System allocator except the
  allocator lane; host: Fedora 44, 7.2.5 kernel, 20 CPUs, benchmark
  pinned to CPU 1 (full record in `H4-LARGE-N-CAUSE-1-MANIFEST.md`,
  `environment.txt`, `machine-pinning.txt`).
- `RAW_HASH_CLOSURE`: **1004/1004 files PASS.** The closure is now
  produced by a committed generator, `hash-closure.py`, so a stale entry
  cannot recur silently:
  `python3 hash-closure.py` verifies, `python3 hash-closure.py --write`
  regenerates, and `sha256sum -c RAW-HASH-CLOSURE.txt` is the
  independent check. Two closure defects were corrected in this commit,
  both documentation/derived-layer only:
  1. `receipts/PRODUCER-RECEIPTS.csv` was corrected in `84bc333` and its
     recorded hash was not regenerated, leaving the closure at 1003/1004
     (the P1 blocker of the independent review). Regenerated, not
     hand-patched.
  2. The closure covered 46 `.log` files that the repository's `*.log`
     ignore rule kept out of git, so it could not have verified from a
     fresh clone, and its line order was filesystem readdir order. The
     evidence logs are now tracked (scoped `!research/.../h4-large-n-cause-1/**/*.log`
     negation in `.gitignore`), the order is byte order, and the
     generator warns if any covered file is untracked.
- Derivation determinism, re-verified in this commit: `analyze.py` →
  `derive-tables.py` reproduce every committed derived CSV
  **byte-identically** from the committed raw JSONL, and
  `perf/derive-stat-summary.py` reproduces `perf/stat-summary.csv`
  byte-identically from the committed raw `perf/stat/*.json`
  (`perf/test-derive-stat-summary.py`: 7/7). `receipts/PRODUCER-RECEIPTS.csv`
  maps every deliverable to its producing executable SHA and evidence log.
- Raw evidence directories (`raw/`, `attempt-*-superseded/raw*/`,
  `perf/stat/`, `perf/record/`, `ebpf/raw/`) are byte-identical to the
  pre-corrective head `84bc333`; only documentation, the derivation
  script, the derived classification table and the closure changed.
