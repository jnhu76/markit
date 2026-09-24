# H4-LARGE-N-CAUSE-1-MANIFEST.md

Issue #50 — `[Research] H4-LARGE-N-CAUSE-1 — H4 大 N 更新成本归因与 V1 诊断基线`.

```text
result class:            POST_HOC_EXPLANATORY / H4_ONLY / RESIDENT_SINGLE_RESET
authority base:          master @ 334eea6201fc0258e35a7c5b21feb722641ddcbd
                         (merged Campaign-2 evidence authority, PR #47)
mechanism/counter auth:  3762b7a42e1c284a4c2c2e0ebac8496e70c63431 (includes #46)
PR #45:                  not used as authority
Campaign-2 raw evidence: NOT modified
```

## 1. Commits and collection provenance

Six distinct things are recorded separately. **No commit SHA is the
provenance of any measurement** — the producing executable SHA256 is
(§2), because there is no reproducible build that would tie a binary back
to a commit.

| Role | Value |
|---|---|
| Base SHA | `334eea6201fc0258e35a7c5b21feb722641ddcbd` |
| Diagnostic implementation commit | `f52fe5a` — *h4diag: diagnostic instrumentation for H4 large-N cost attribution (#50)*, committed 2026-09-23T23:50:03Z |
| **Workspace / source state at collection** | `f52fe5a` as **HEAD**, plus **uncommitted `h4diag` working-tree changes**. The formal collection (attempt 5) ran 2026-09-24T02:07:09Z … 02:22:34Z, i.e. while HEAD was `f52fe5a`. |
| Collection-correction commit (committed **after** collection ended) | `6b2c2a4` — *h4diag: fix window ordering, counter attribution and allocator tagging for the formal collection*, committed 2026-09-24T02:38:48Z, **16 minutes after attempt 5 finished**. Its content is the source state attempt 5 ran from; it is *not* the HEAD that produced the binary, and that mapping cannot be re-derived from the repository. |
| Producing executable SHA256 | `4d23df55…` (U_PLAIN, ablations, PMU, perf record, eBPF), `8185f825…` (U_PHASE), `aa731757…` (W_COUNTERS), `fd81e27d…` (A_ALLOCATOR) — full hashes in `receipts/FROZEN-EXECUTABLES.txt`, in every raw receipt row, and in every lane log. **This is the executable authority.** |
| Results commit | `45ad068` — *results: H4-LARGE-N-CAUSE-1 evidence, analysis and report (issue #50)*; committed 2026-09-24T02:43:07Z; first committed `bin/` |
| Corrective commits | `c6c52bb` + `84bc333` + *this commit* — documentation / derived-summary layer only (§12); raw evidence unchanged, no re-collection |
| Final PR head | Draft PR #51, branch `research/50-h4-large-n-cause-1` — moves with documentation commits; **not** the provenance of any measurement |

Per-run receipts are in `raw/*.jsonl` and `logs/*.log`; each of those
rows carries the producing `executable_sha256` and `executable_features`.
The diagnostic implementation is committed **separately** from the
collected results (Issue #50 §17). The complete diagnostic diff is the
`h4diag/` crate plus one added workspace member; the frozen crates
(`common`, `instrumentation`, `oracle`, `runner`, `shared-grammar`, all
five `mechanisms/*`) are **byte-identical to the base SHA** — verified by
`git diff --stat 334eea6 HEAD -- research/benchmarks/markdown-ast-update/{common,instrumentation,oracle,runner,shared-grammar,mechanisms}`.

## 2. Producing executables (Issue #50 §17 — no PR #47 provenance mistake)

Every lane is produced by a named executable whose SHA256 is recorded twice:
inside each lane's `receipt` row and in the lane log written by
`run-lane.sh`. `executable-shas.txt` holds the frozen set.

The `mechanism` column matters: **U_PLAIN is the only lane that runs the
original frozen mechanism.** Every other lane runs the diagnostic copy,
which the PMU lane's own raw rows label `"mechanism":"diag-copy"`.

| Lane | Executable (subcommand) | Features | Mechanism | SHA256 |
|---|---|---|---|---|
| U_PLAIN | `mdbench-h4diag` (`run-plain`) | none | **original** `RestartConvergenceMechanism` | `4d23df55…` |
| Ablations | `mdbench-h4diag` (`run-ablations`) | none | diagnostic copy `H4Diag` (A0 / Adefs / Adrop / Acapacity) | `4d23df55…` |
| PMU stat + perf record | `mdbench-h4diag` (`pmu-run`) | none | diagnostic copy `H4Diag` (default; `--mechanism original` exists but was not used) | `4d23df55…` |
| eBPF bench runs | `mdbench-h4diag` (`pmu-run`) | none | diagnostic copy `H4Diag` | `4d23df55…` |
| U_PHASE | `mdbench-h4diag-phases` | `phases` | diagnostic copy `H4Diag` | `8185f825…` |
| W_COUNTERS | `mdbench-h4diag-counters` | `counters` | diagnostic copy `H4Diag` | `aa731757…` |
| A_ALLOCATOR | `mdbench-h4diag-alloc` | `allocator` | diagnostic copy `H4Diag` | `fd81e27d…` |

```text
PRIMARY LATENCY AUTHORITY:
    U_PLAIN on the frozen original H4 (run-plain), which runs the original
    mechanism through the original frozen runner timer boundary
    (markit_mdbench_runner::run_update_timed) with no phase timer, no
    diagnostic counter and no allocator wrapper.

PMU / PERF SAMPLING:
    equivalent diagnostic copy (pmu-run). Used for qualitative /
    regime-level microarchitectural support. Cycle-share percentages are
    NOT transferred exactly to U_PLAIN.
```

The copy is equivalent in result checksum and work counters on all 8 cells
(`preflight.jsonl`, `equivalence.jsonl`) but **not identical in timing**: at
16 MiB the PMU lane's median total is 25.81 ms against U_PLAIN's 24.99 ms
(**+3.3%**), while at 1 MiB they agree (694.2 µs vs 693.9 µs, +0.04%).
This is why the diagnostic copy is used for shares and hotspots but never
as the latency authority.

## 3. Toolchain and build profile

```text
rustc 1.98.1 (48a229cea 2026-09-01) / LLVM 22.1.8
cargo 1.98.1 (797e8a9bc 2026-08-05)
profile: release, opt-level 3, lto = "thin", codegen-units = 1,
         incremental = false, panic = "unwind"
RUSTFLAGS: none.  target-cpu: default (no -C target-cpu override)
allocator: System (Rust std default) in every lane except A_ALLOCATOR,
           which wraps System with counting and delegates to it
```

Cargo.lock is committed with the implementation commit.

## 4. Host

```text
hostname            E5
kernel              Linux 7.2.5-200.fc44.x86_64 (SMP PREEMPT_DYNAMIC)
CPU                 Intel(R) Xeon(R) CPU E5-2666 v3 @ 2.90GHz
                    (family 6, model 63 / Haswell-EP, stepping 2, microcode 0x49)
topology            1 socket, 10 physical cores, 2 threads/core, 20 logical CPUs
caches              L1d 320 KiB (10 x 32 KiB), L1i 320 KiB (10 x 32 KiB)
                    (L2/L3 from lscpu in environment.txt)
NUMA                single node (no node*/physcpubind for cpu1)
RAM                 65676220 kB total, ~62142600 kB available
governor            schedutil (all 20 CPUs), driver intel_cpufreq
turbo               intel_pstate no_turbo = 0 (turbo enabled, not disabled)
pinning             taskset -c 1 for every lane
                    cpu1 = physical core 1, SMT siblings {1,11}, NUMA node 0
```

Full capture: `environment.txt`, `machine-pinning.txt`.

Temperature at capture: package +52 °C, core 1 +44 °C (limits high 83 °C /
crit 93 °C). Load average at capture 1.04 / 1.29 / 0.62.

## 5. Workload

Frozen by Issue #50 §2 and materialized by `h4diag/src/cells.rs`:

```text
N       = 128 KiB, 256 KiB, 512 KiB, 1 MiB, 2 MiB, 4 MiB, 8 MiB, 16 MiB
unit    = 128 bytes (126 content bytes + LF + LF);  M = N / 128
target  = floor(M / 2);  edit start = target * 128 + 63
edit    = zero-length insertion of "zzzzzzzz"
context = container depth 0, no definitions, no references, no fences
```

Byte identity with the Campaign-2 controlled-N construction was **verified**,
not assumed: `cells.jsonl` records `pre_sha256`, `post_sha256`, `edit_sha256`
and the frozen `case_id_hex` for every point; for the three overlapping
points (`128KiB`, `1MiB`, `16MiB`) `cell_by_label` calls
`campaign2::generators::generate_cell(Axis::N, label)` and refuses to proceed
unless `pre_source`, `post_source` and the `CanonicalEdit` are identical
byte-for-byte. `equivalence.jsonl` records the result.

The five new N points are **new diagnostic cells only**. The Campaign-2 cell
table, its identities and its raw evidence are untouched. Per Issue #50 §2,
the frozen derived `axis_value` and the `KNOWN` attribution are **not** used;
cell facts come from `cell_id` / raw numbers.

## 6. Workload invariants verified before collection (`preflight.jsonl`)

| Cell | restart dist | convergence dist | fresh blocks | fresh nodes | unique post bytes | inspected total | correctness | equivalence to frozen H4 |
|---|---:|---:|---:|---:|---:|---:|---|---|
| all eight | 63 | 136 (before EOF) | 1 | 2 | 138 | 283 | Pass | checksum + work counters identical |

`source_bytes_inspected_total = 283` is **constant at every N** — the
parser work really is fixed, as Issue #50 §1 states. The frozen metadata
counter, by contrast, grows as `metadata_records_touched = 3.5M + 4`
(exactly: 3 588 at 128 KiB, 458 756 at 16 MiB, verified at all 8 cells)
and `nodes_reused = 2M - 2`, both of which are *mechanism-defined
counts*, not hardware accesses — which is exactly the distinction the
issue asks to resolve with new measurements.

## 7. Sampling and schedule

```text
sessions            3 independent sessions per timing lane
warmup              10 per (N, variant, lane)
measured            30 per (N, variant, lane)
schedule            one measured round contains every included
                    (label, cell) slot exactly once; order is a
                    Fisher-Yates shuffle from a frozen SplitMix64 seed
                    (seed = f("H4LARGEN", lane, session)); the ACTUAL
                    schedule is written to raw/*.schedule.jsonl
A/A controls        two labels ("A0", "A0-dup") with an identical
                    implementation, in both the U_PLAIN and the ablation
                    schedules; the empirical resolution is the p95 of
                    their per-round absolute relative difference
statistics          per session: nearest-rank p50 = 15th, p95 = 29th of
                    the 30 measured values; case estimate = median of the
                    three session p50s; 90 samples are NEVER pooled as
                    independent
```

## 8. Timer / measurement boundaries

U_PLAIN, U_PHASE and the ablations all use the frozen runner boundary:
`T_prepare` = `prepare_update`; `T_native` = `update` + `complete` +
`black_box`; `T_total = T_prepare + T_native` (arithmetic). The oracle,
the normalized projection and the result checksum run strictly after every
timer has stopped, and the fresh resident state for each sample is built
before the timer starts.

The U_PHASE lane adds *inner* phase timers (see `PHASE-MAP.md`); their
perturbation is measured as `U_PHASE / U_PLAIN`.

The A_ALLOCATOR lane's own wall time is recorded but is **never** mixed into
U_PLAIN.

The PMU lane's counted window is opened and closed by the harness through
`perf stat --delay=-1 --control=fifo:<ctl>,<ack>` with a synchronous ack per
command, so the window contains exactly `prepare_update` + `update` +
`complete` + `black_box` — no construction, no oracle, no checksum, no final
result destruction.

## 9. Failure policy

No lane deletes, retries or repairs anything. A non-zero lane exit stops the
driver and leaves the partial raw file in place. Failed attempts are kept as
`collection-driver-attempt*.log`. No session was dropped; every session that
ran is reported, including inconvenient ones.

## 10. Collection attempts (full history, nothing silently re-reported)

| attempt | outcome | reason archived |
|---|---|---|
| 1 | FAILED | wrong driver ROOT; log kept (`collection-driver-attempt1-failed.log`) |
| 2 | ABORTED | partial; log kept |
| 3 | superseded | mixed provenance: lanes produced by binaries whose SHAs did not match the frozen set (`attempt-3-mixed-provenance/`) |
| 4 | superseded | PMU window defect: `pmu-run` opened the perf control window BEFORE building the fresh state, so counted windows contained construction (~20x instruction inflation); also all perf artifacts predate the final binary set (`attempt-4-superseded/`) |
| **5** | **FINAL** | all four binaries rebuilt with explicit `--bin` + features, frozen in `bin/`, SHAs re-recorded; all 13 timing lanes exit 0; every lane log records `executable_sha256` + `executable_features`; PMU stat (5 groups x 8 cells x user/root, all pcnt-running = 100.00%), perf record (1 MiB/16 MiB) and the eBPF lane all ran against the same frozen default binary |

## 11. Lane completion verdicts

```text
U_PLAIN      COMPLETE   3 sessions x 40 rounds, 13 lanes exit 0
U_PHASE      COMPLETE   PHASE_PERTURBATION = NONE, with one recorded
                        exceedance: all (session, cell) U_PHASE/U_PLAIN
                        ratios are inside max(5%, A/A noise) except
                        16 MiB session 2 (0.884, i.e. 11.6% vs a 9.9%
                        threshold), and that excursion belongs to the
                        comparator lane's own session-2 plain p50
                        (29.8 ms vs 24.8/25.0 ms), not to the probes;
                        the phase lane's three 16 MiB sessions agree to
                        1.022. See report §3.2.
W_COUNTERS   COMPLETE   8 cells, equivalence to original H4 verified pre-timing
A_ALLOCATOR  COMPLETE   3 sessions; window = update region only (13 allocs/15 reallocs at 16 MiB)
ABLATIONS    COMPLETE   A0/A0-dup/Adefs/Adrop/Acapacity interleaved, 3 sessions, none dropped
PERF_STAT    COMPLETE   5 groups x 8 cells x {user,root}; min pcnt-running = 100.00% everywhere
PERF_RECORD  COMPLETE   1 MiB (300 windows) + 16 MiB (20 windows), DWARF call graphs
EBPF         COMPLETE   EBPF_ATTEMPT = AVAILABLE; EBPF_PERTURBED = NO (ratio 1.0040);
                        conclusions narrowed to the recorded probe scopes (report §5)
MICROKERNELS NOT_NEEDED (selection-notes/07)
```

## 12. Post-collection correctives

Every defect below is in the **documentation / derived-summary layer**.
All are fixed WITHOUT touching any raw evidence and WITHOUT re-collecting
anything; `rerun_required = NO`.

| # | corrective | commit |
|---|---|---|
| 1 | root PMU derived summary was all zeros | `84bc333` (§12.1) |
| 2 | microarchitectural attribution narrowed; ≤1.6% residual claim withdrawn | `84bc333` (§12.2) |
| 3 | results-commit SHA recorded | `c6c52bb` |
| 4 | collection provenance: HEAD-at-collection vs source state vs executable authority | this commit (§12.3) |
| 5 | PMU/perf mechanism scope (original vs diagnostic copy) made explicit | this commit (§12.4) |
| 6 | classification labels switched back to the frozen Issue #50 §14 thresholds | this commit (§12.5) |
| 7 | allocator arithmetic corrected to the raw requested-byte categories | this commit (§12.6) |
| 8 | eBPF conclusions narrowed to the recorded probe scopes | this commit (§12.7) |
| 9 | hash closure regenerated by a committed generator; its 46 untracked evidence logs made trackable | this commit (§12.8) |
| 10 | remaining numeric/prose mismatches (§12.9) | this commit |

### 12.1 Root PMU derived summary was all zeros

The `perf/stat-summary.csv` shipped in `45ad068` was produced by an inline
generator that matched perf event names only under their user-mode names
(`cycles:u`), so the root files' plain names (`cycles`) silently fell
through to a 0.0 default. The committed raw root PMU files were always
valid (e.g. root 16 MiB groupA: cycles 1 365 569 519, instructions
529 446 989, pcnt-running 100.00%); only the derived summary was wrong.
Fix: the derivation is now a committed, deterministic script
(`perf/derive-stat-summary.py`) that normalizes event names by stripping
perf modifiers, refuses missing events, and refuses to derive a zero from
non-zero raw counters; `perf/stat-summary.csv` is regenerated from the
unchanged raw files. The user rows reproduce byte-identically; the root
rows now derive (16 MiB: 91 037 968 cycles/update, CPI 2.579,
LLC-miss/block 9.44 — corroborating the user rows, +2.4% cycles from
kernel-side context switches/faults). Regression test:
`perf/test-derive-stat-summary.py` (7 tests,
`python3 perf/test-derive-stat-summary.py`).
The report's former "context switches ≈ 64/update-window" remark was
likewise unsupported by the derived data and is corrected to the raw
values (user counters 0; root 15-37 per 15-update window).

### 12.2 Microarchitectural attribution narrowed

The report's §9 claimed the 13.9 ms linear-extrapolation gap and the
14.3 ms excess-cycle model were "two independent estimates", closing the
nonlinearity to a ≤1.6% residual. Both quantities measure the same excess
relative to small-N behaviour (wall-clock vs cycles); neither partitions
the excess cycles into LLC-latency / MLP / prefetch / downstream / TLB /
allocator stall components. The report now states: ALGORITHMIC work
linear (unchanged); cache/memory-hierarchy amplification STRONGLY
SUPPORTED (unchanged in strength, now worded as association with the
regime transition rather than arithmetic closure);
PRECISE_STALL_DECOMPOSITION = UNRESOLVED; no quantitative
stall-attribution residual claimed. Representation-level classification
and all V1 mandates are unchanged.

### 12.3 Collection provenance identity

§1 previously stated "HEAD SHA at collection | binaries frozen from the
`6b2c2a4` tree", which reads as though `6b2c2a4` were the HEAD during
collection. The recorded timestamps contradict that: attempt 5 ran
02:07:09Z–02:22:34Z while HEAD was `f52fe5a`, and `6b2c2a4` was committed
at 02:38:48Z, 16 minutes after collection ended. §1 now separates
workspace/source state at collection, diagnostic implementation commit,
collection-correction commit, producing executable SHA256, results
commit, corrective commits and final PR head, and states explicitly that
the working tree held uncommitted `h4diag` changes later committed as
`6b2c2a4`. No stronger binary→commit mapping is asserted: the producing
executable SHA256 is the executable authority.

### 12.4 PMU / perf-record mechanism scope

The PMU, perf-record and eBPF lanes are driven by `pmu-run`, which runs
the **equivalent diagnostic copy** (`H4Diag`); the lane's own raw rows say
`"mechanism":"diag-copy"`. Only U_PLAIN (`run-plain`) runs the original
frozen `RestartConvergenceMechanism`. The copy is checksum/counter
equivalent but is **+3.3% slower at 16 MiB** (25.81 ms vs 24.99 ms median
total) and +0.04% at 1 MiB. §2 now carries a mechanism column and the
"primary latency authority / PMU-perf sampling" wording, and the report
states it in §3.3 and §4. PMU/perf data are used for qualitative,
regime-level microarchitectural support; their cycle-share percentages are
not transferred exactly to U_PLAIN. The PMU data are not invalidated by
this.

### 12.5 Classification labels returned to the frozen rule

`derive-tables.py` classified with post-hoc thresholds (DOMINANT ≥ 15%,
MATERIAL ≥ 8% at 16 MiB or ≥ 8% of the increment) that were never part of
the frozen contract. Issue #50 §14 requires 50% for DOMINANT and 10% for
MATERIAL. The generator now implements the §14 rule literally and emits
the qualifying `basis` per suspect. Result: `DOMINANT = none`;
MATERIAL = P6, P5, P3, P7, P1; SECONDARY = P4, Acapacity; NEGLIGIBLE =
P2, unphased residual. Measured shares are unchanged — only the labels
were corrected, and P4 (8.6%, Adefs −8.7%) is reported as SECONDARY
rather than MATERIAL.

### 12.6 Allocator arithmetic

The report claimed "3 × 128 B × M fresh allocations … ≈ 52 MB of
allocator traffic". The raw requested-byte categories are ≈80 B × M per
stream, and 52.4 MB is the sum of two non-disjoint totals. §3.4 now
distinguishes fresh requested (alloc) from realloc-new requested and from
freed requested, reports live-start / live-end / net change / peak
growth, gives the exact definition of the 52.4 MB figure, and states that
these are counting-allocator requested payload bytes — not RSS and not
DRAM traffic.

### 12.7 eBPF conclusions narrowed

The report previously stated scheduler exclusion, "0 major faults in
every window", "no mmap churn", "allocator irrelevant" and "brk only a
few times per update". The recorded probe scopes support only the narrow
conclusion that the lane found no evidence of CPU migration, major-fault
storms, or a kernel-side regime transition large enough to explain the
nonlinear scaling. §5 now labels the bpftrace counts as whole-process,
whole-batch aggregates, notes they are not resume-filtered, notes that
the `perf record -a` sched trace is system-wide and includes the
profiler's own process, records ≈205 `brk` per update (4 109 over 20
updates — the "few times" claim is withdrawn), records that large
anonymous mmap/munmap pairs *are* present, and records the scope
difference between the two fault lanes (whole-process 276 385 vs
windowed 13 056, with 0 major faults **in the windowed scope**).

### 12.8 Hash closure regenerated and made reproducible

Two defects, both in the closure artifact:

- `receipts/PRODUCER-RECEIPTS.csv` was corrected in `84bc333` and its
  recorded hash was not regenerated, so `sha256sum -c` verified only
  1003/1004. This was the independent review's single P1 blocker. Fixed by
  regeneration, not by hand-patching one digest.
- The closure covered 46 `.log` files that the repository's `*.log`
  ignore rule kept out of git, and its line order was filesystem readdir
  order. It therefore could not have verified from a fresh clone and was
  not byte-reproducible.

Both are fixed: `hash-closure.py` is now the committed deterministic
generator (excludes the closure itself, the generator, and `__pycache__`;
orders by byte order; warns if any covered file is untracked), the
evidence logs are tracked via a scoped `.gitignore` negation, and the
closure is regenerated: **1004/1004 PASS**, independently confirmed by
`sha256sum -c RAW-HASH-CLOSURE.txt`.

### 12.9 Remaining numeric / prose mismatches

Corrected, with measured values unchanged: LLC-miss growth now names its
1 MiB baseline (×22 789) and gives the 128 KiB-based figure (×1.09 M);
page-fault growth ×17 not ×16; the phase-closure claim is 98.8–98.9% at
128 KiB rising to 99.99% at 16 MiB rather than 99.99% at every cell; the
linear-fit r² claim now carries the P2 exception (0.9767) and the correct
floor (≥0.9988); the Adefs effect is −8.7% at 16 MiB and −5.4% at
128 KiB and is not monotonic across cells; DERIVED(U_deferred + T_drain)
is 42.1 ms not ≈41 ms; Acapacity is direction-INCONSISTENT at 2 and
4 MiB rather than "consistent" everywhere; the perf-record list is
labelled as perf's disjoint `self` column with an explicit
no-parent-plus-child warning; "no parser symbol appears" carries its
sampling-resolution qualification; the 138-unique-byte and 283-total-
inspection-byte metrics are distinguished; 3.56 GHz is labelled a derived
cycles-per-wall-second rate rather than a measured clock; and the
Campaign-2 (≈87.5 µs → ≈29.3 ms, ×43) and #50 (≈86.3 µs → ≈25.0 ms,
×36.0) collections are stated separately instead of being merged.
