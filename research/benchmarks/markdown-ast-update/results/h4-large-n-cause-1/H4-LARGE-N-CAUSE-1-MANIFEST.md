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

## 1. Commits

| Role | Value |
|---|---|
| Base SHA | `334eea6201fc0258e35a7c5b21feb722641ddcbd` |
| Diagnostic implementation commit | `f52fe5a` — *h4diag: diagnostic instrumentation for H4 large-N cost attribution (#50)* |
| Implementation corrections commit | `6b2c2a4` — *h4diag: fix window ordering, counter attribution and allocator tagging for the formal collection* |
| Results commit | *(filled in when the results are committed)* |
| HEAD SHA at collection | *(recorded in `raw/*.jsonl` receipts and in the lane logs)* |

The diagnostic implementation is committed **separately** from the collected
results (Issue #50 §23). The complete diagnostic diff is the `h4diag/` crate
plus one added workspace member; the frozen crates (`common`,
`instrumentation`, `oracle`, `runner`, `shared-grammar`, all five
`mechanisms/*`) are **byte-identical to the base SHA** — verified by
`git diff --stat 334eea6 HEAD -- research/benchmarks/markdown-ast-update/{common,instrumentation,oracle,runner,shared-grammar,mechanisms}`.

## 2. Producing executables (Issue #50 §22 — no PR #47 provenance mistake)

Every lane is produced by a named executable whose SHA256 is recorded twice:
inside each lane's `receipt` row and in the lane log written by
`run-lane.sh`. `executable-shas.txt` holds the frozen set.

| Lane | Executable | Features compiled in | SHA256 |
|---|---|---|---|
| U_PLAIN | `mdbench-h4diag` (`run-plain`) | none | `4d23df55856a1e71115abdc45c0f5d7993182520f0b53df2a0cab6cdb8c0b4e6` |
| U_PHASE | `mdbench-h4diag-phases` | `phases` | `8185f825473ebc480c9102101546cb367abc3f5b80bc041e1923d05833372ca2` |
| W_COUNTERS | `mdbench-h4diag-counters` | `counters` | `aa7317579d8ddf000d1f84ba1495ff735421ac9a14a0aaf845b2c318918c1410` |
| A_ALLOCATOR | `mdbench-h4diag-alloc` | `allocator` | `fd81e27d4014a229fe2ce9b63b52a38299a186db8abfd78f23a09c51e80d026e` |
| Ablations | `mdbench-h4diag` (`run-ablations`) | none | `4d23df55…` (same as U_PLAIN) |
| PMU | `mdbench-h4diag` (`pmu-run`) | none | `4d23df55…` (same as U_PLAIN) |

The U_PLAIN lane runs the **original frozen mechanism**
`markit_mdbench_restart_convergence::RestartConvergenceMechanism` through the
**original frozen runner timer boundary**
(`markit_mdbench_runner::run_update_timed`). It contains no phase timer, no
diagnostic counter and no allocator wrapper, because those are
compile-time features of a *different* binary.

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
U_PHASE      COMPLETE   PHASE_PERTURBATION = NONE (all cells below max(5%, A/A noise))
W_COUNTERS   COMPLETE   8 cells, equivalence to original H4 verified pre-timing
A_ALLOCATOR  COMPLETE   3 sessions; window = update region only (13 allocs/15 reallocs at 16 MiB)
ABLATIONS    COMPLETE   A0/A0-dup/Adefs/Adrop/Acapacity interleaved, 3 sessions, none dropped
PERF_STAT    COMPLETE   5 groups x 8 cells x {user,root}; min pcnt-running = 100.00% everywhere
PERF_RECORD  COMPLETE   1 MiB (300 windows) + 16 MiB (20 windows), DWARF call graphs
EBPF         COMPLETE   EBPF_ATTEMPT = AVAILABLE; EBPF_PERTURBED = NO (ratio 1.0040)
MICROKERNELS NOT_NEEDED (selection-notes/07)
```
