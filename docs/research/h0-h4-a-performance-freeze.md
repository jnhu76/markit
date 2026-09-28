# H0–H4 + Horse-A performance freeze (Gate B compact preflight)

Status: **PERFORMANCE_FREEZE_READY (candidate — frozen before any formal
collection)**

```text
FORMAL_PERFORMANCE_COLLECTION_STARTED = NO
GATE_A — DONOR FIDELITY               = PASS (closed 2026-09-28, PR #85;
                                        not reopened by this preflight)
GATE_B — IMPLEMENTATION-PARITY        = COMPACT PREFLIGHT (this document)
PERFORMANCE_COLLECTION_RUN            = NO
```

This manifest replaces the previously planned over-sized Gate B with the
minimum defensible implementation-parity preflight. It does NOT claim the
six horses are equally optimized. The fairness criterion is only: all six
implementations are correct; all six implement their already-accepted
mechanisms; all six use the same eligible input/edit/build/lifecycle/
measurement rules; no known concrete accidental defect invalidates the
comparison; implementation limitations stay explicit; and the formal
sampling and reporting rules below were frozen BEFORE any result was seen.

This document references existing authorities instead of copying their
history. It binds no measured value.

## 1. Repository / subject identities

```text
LIVE_MASTER_AT_PREFLIGHT_START = efa392752bfb2604e3d2fdfabaf0ba9825f70b74
LIVE_TREE_AT_PREFLIGHT_START   = ccbae20a29b0a060532ff014187a98b798d30595
PERFORMANCE_FREEZE_COMMIT      = <the merge commit of the Gate-B parity
                                 preflight PR (recorded after review)>
PERFORMANCE_FREEZE_TREE        = <recorded with the freeze commit>
```

Six implementation identities (all Gate-A accepted; verdicts in
`docs/research/h0-h4-donor-fidelity/FIDELITY-MATRIX.md`):

```text
H0      markit-mdbench-full-rebuild          h0-full-rebuild            FAITHFUL_MECHANISM_MODEL
H1      markit-mdbench-block-local           block-local-reparse-h1     FAITHFUL_WITH_DECLARED_SIMPLIFICATION
H2      markit-mdbench-fragment-reuse        fragment-reuse-h2          FAITHFUL_MECHANISM_MODEL  (#79 repair)
H3      markit-mdbench-old-tree-subtree-reuse old-tree-subtree-reuse-h3 FAITHFUL_WITH_DECLARED_SIMPLIFICATION (#80 repair)
H4      markit-mdbench-restart-convergence   restart-convergence-h4     FAITHFUL_MECHANISM_MODEL
HorseA  markit-mdbench-horse-a               horse-a-v1                 #62/#60 frozen (Gate A complete)
```

Gate A is closed. This preflight did not reopen it; no fix below changes a
mechanism's identity, retained representation, reuse decisions, fallback
semantics, or completion semantics.

## 2. Toolchain / build / allocator policy

Reference authority: `manifest/environment.toml` +
`protocol/implementation-parity.md` (unchanged, inherited):

```text
toolchain      rustc 1.97.1 pinned by rust-toolchain.toml
profile        release-primary-v1 (opt-level 3, lto=thin, codegen-units=1,
               incremental=false, panic=unwind, overflow-checks off)
RUSTFLAGS      empty; target-cpu default
allocator      Rust/system default for TIMING and ATTRIBUTION lanes and
               all informal runs. The counting allocator
               (markit-mdbench-instrumentation::CountingAllocator) exists
               ONLY inside the dedicated memory-lane binary
               (`mdbench-memory-smoke` + the formal memory-lane runs);
               it is never the allocator of a timing process.
features       none beyond the workspace defaults; no per-horse features
```

## 3. Host identity and binding

```text
FORMAL HOST    jnhu@192.168.31.75  (host "E5", Fedora 44,
               kernel 7.2.5-200.fc44.x86_64, x86_64)
machine record results/manifests/six-horse-machine-v1.toml — captured ON
               the formal host before the campaign; hard host-binding
               fields fail closed (R7 §12/§13, incl. the
               MACHINE-BINDING-CORRECTIVE-1 wording)
CPU affinity   single-worker horse execution pinned to ONE frozen logical
               CPU (lowest logical CPU != 0, benchmark-free, recorded
               core/SMT siblings); the worker fails closed if affinity
               cannot be established
governor/turbo inspected + recorded, never modified (R7 §12)
```

## 4. Workload / edit / state-history policy

Reference authority: the #35 frozen real workload, never reselected:

```text
CLEAN_STATE    22 G0-strict FULL_READ files (initial-build surface)
EDIT_WRITE     362 G0_PRIMARY frozen cases (canonical edit identities;
               BREAK/RESTORE chains stay frozen)
state history  SINGLE_RESET (R7 §3): every measured EDIT_WRITE iteration
               starts from the exact frozen pre-edit source and a freshly
               constructed valid pre-edit horse state, built OUTSIDE every
               timer, never cloned from a cached build, never retained
               across iterations. Chained-history / session-aging behavior
               is NOT part of this campaign (separate future surface).
initial build  CLEAN_STATE timer covers full_parse + complete/seal +
               black_box(state). The per-iteration pre-state build is
               untimed by the frozen EDIT_WRITE boundary; its cost is
               measured on the CLEAN_STATE surface. No preprocessing is
               shifted outside the initial-build metric.
```

## 5. Timing boundary (frozen; MEASUREMENT-CORRECTIVE-1 + R7 §4)

```text
T_prepare      prepare_update only
T_native       update + complete (native seal) + black_box(state)
T_total        = T_prepare + T_native (arithmetic sum; never an
               enclosing wall-clock timer)
outside all    source lookup, pre/post construction, canonical-edit
timers         verification, fresh pre-state construction, the H0
               reference parse (once per case), NormalizeV1 projection,
               validation, result checksum, oracle comparison, row
               assembly, serialization
```

Per-horse phase mapping (all six implement the same common
`Mechanism` boundary):

```text
H0–H4          existing phase implementations (unchanged by this freeze
               except the attribution gate, §7)
HorseA         full_parse = full_build; prepare_update = update::stage()
               (all fallible pre-frontier staging); update = staging
               prepare (ownership move) + PreparedCommit::commit
               (frontier crossing: splice/retire/install); complete =
               native seal. Retirement of replaced owners happens inside
               commit, inside T_native. No mechanism work is deferred
               past the timer stop.
```

Horse-A ownership note: the adapter
(`markit_mdbench_horse_a::HorseAMechanism`) adds no mechanism semantics;
`stage()` is fallible in T_prepare, the frontier crossing is infallible in
T_native, and errors map per-variant to the shared FailureStatus schema
(association/trivia refusals -> Unsupported; resource refusal -> Oom;
invariant evidence -> Crash).

## 6. Oracle / normalization / checksum placement

Unchanged (MEASUREMENT-CORRECTIVE-1): every horse's normalized projection
and its checksum are post-timer exports via the frozen `ResultChecksum`
trait; the H0 reference gate runs once per case outside every timer; a
wrong warmup is a campaign failure, not an ignorable sample.

## 7. Parity preflight: bounded static inspection result

ONE focused inspection covered the shared runner (`runner/src/
orchestrate.rs`) and the actual timed entry paths of all six horses, for
concrete accidental defects only (input identity, reset/history policy,
completion obligations, deferred work, oracle/logging placement, debug
work in the timed lane, accidental serialization, redundant full-source
copies, duplicate full traversals, adapter-induced quadratic loops,
clearly redundant materialization). A clone/allocation/traversal is not
automatically a defect; the admission rule was: the work must be shown
redundant from code semantics alone and removable without changing
retained representation, reuse decisions, fallback behavior, completion
semantics, or lifetime obligations.

```text
H0      CLEAN apart from the defect family below
H1      CLEAN apart from the defect family below
H2      CLEAN apart from the defect family below (Gate-A #79 mechanism intact)
H3      CLEAN apart from the defect family below (Gate-A #80 mechanism intact)
H4      CLEAN apart from the defect family below
HorseA  Campaign wiring was MISSING (blocker M-1); its own pipeline uses
        the standard inline-sink pattern and the opt-in structural
        recording sink; no defect from the family below
```

### Defect family P-1 — attribution-only counting walks inside the timed lane

```text
DEFECT       H0–H4 computed attribution counters by dedicated tree walks
             (count_blocks_nodes / count_node / count_forest /
             skel_count / entry_native_nodes / retained_block_nodes)
             executed inside mechanism phases in EVERY lane, including
             the timing lane whose NoopWorkSink discards every value.
             H0's walk covered the WHOLE output document on every
             update; H2/H3 re-walked retained subtrees on their reuse
             paths; H1/H4 were region-local. This contradicted the
             frozen lane contract (NoopWorkSink "adding no attribution
             cost"; runner doc "T-LANE ... zero attribution cost") and
             AGENTS.md §7 ("Separate measurement overhead from the
             measured metric"). Prior campaigns carried this cost in
             their measured numbers; whether the release optimizer
             eliminated parts of it was never established.
WHY REDUNDANT           in the timing lane the computed values are never
                        observed; the walks exist only to feed counters.
WHY REMOVAL CHANGES     the walks feed only WorkSink counter values;
NO MECHANISM            retention, reuse, fallback, completion and
                        lifetimes are untouched; in recording lanes the
                        reported values are identical (A-LANE exact
                        fixtures H1/H4 + full suites prove this).
AFFECTED HORSE          H0, H1, H2, H3, H4 (Horse-A already followed the
                        inline-sink pattern).
CORRECTNESS CHECK       full test suites of all six crates PASS after the
                        fix, including the attribution exact fixtures
                        (A-LANE values byte-identical) and the
                        correctness-only lanes.
FIX SHAPE               common::WorkSink::attribution_active() (default
                        false; CounterSink -> true); horses gate ONLY the
                        pure-attribution walks on it.
```

### Blocker M-1 — Horse-A was not dispatchable by the campaign executor

```text
DEFECT       the campaign executor returned "unknown horse" for HorseA;
             manifest/schedule/preflight/finalize hard-pinned five
             horses. The frozen campaign literally could not start.
RESOLUTION   HorseA added to the frozen roster, dispatch, permutation
             (rotate mod 6), and cardinality guards (x6); new campaign
             identity MARKIT-76-SIX-HORSE-PERFORMANCE-CAMPAIGN-v1 with a
             pre-timing seed; CampaignSpecId changes as required.
```

### Gap G-1 — no real allocation/retained-memory backend existed

```text
DEFECT       the M-LANE shipped a schema and a NoMemoryReporter
             placeholder only; the R7 freeze had frozen
             ALLOC_*/RETAINED_MEMORY as UNAVAILABLE. The #76 gate
             requires a REAL backend before the freeze.
RESOLUTION   instrumentation::alloc::CountingAllocator (process
             allocator of the dedicated memory-lane binary only) +
             AllocReporter (per-case windows over the process counters).
             Counting semantics frozen in the module doc: alloc/dealloc
             events; realloc = dealloc + alloc; alloc_zeroed = alloc;
             shared allocations counted once at creation and once at
             final destroy; allocated_bytes/allocation_count are window
             deltas; peak = max live bytes in window; retained =
             live_at_close - live_at_open saturating at zero (growth
             always exact; shrink reports the measured zero, with both
             window edges available for signed re-derivation).
```

`PARITY_CLEANUP` = the one focused PR containing P-1, M-1, G-1 + this
manifest. No per-horse PRs. Ambiguous optimization ideas were NOT
catalogued beyond the deferred list in §9.

## 8. Instrumentation readiness (smoke-validated, non-comparative)

The dedicated `mdbench-memory-smoke` binary (NON_RESEARCH, provenance
`CAMPAIGN_MEMORY_SMOKE/NON_RESEARCH_RESULT`) runs ONE predetermined micro
case × six horses through the REAL campaign lane functions and proved:

```text
incremental latency   run_update_timed: T_prepare/T_native measured with
                      completion included for all six (Horse-A stage->
                      T_prepare, commit->T_native visible)
initial build         run_full_parse_timed + build_initial_state: state
                      construction measured, no preprocessing shifted
                      outside
allocation            counting-allocator windows report non-zero
count/bytes           allocation_count and allocated_bytes for build and
                      update windows on all six
retained memory       windows report Known retained bytes (<= peak) at
                      the frozen lifecycle point: the window closes
                      strictly after the sealed state is black_boxed and
                      still alive
verdict               MEMORY_SMOKE_PASS (instrument validation only; the
                      printed values are not decision-bearing evidence)
```

Formal memory metrics are QUALIFIED ONLY in the separate M-LANE
(`QUALIFIED_M_LANE` in the campaign manifest): memory numbers come from
dedicated instrumented runs of the memory-lane binary, never from a
timing or attribution session. CPU_TIME stays UNAVAILABLE; no PMU lane is
scheduled.

## 9. Deferred (NOT defects; not done here)

```text
- Horse-A's dyn HorseAStructuralSink call overhead: the structural
  record is its frozen #59/#60 attribution seam, by design; removal
  would delete accepted mechanism surface.
- H2/H3 candidate-discovery shapes (per-consult scans): Gate-A accepted
  mechanism identity post-#79/#80; redesign is forbidden here.
- Any new index, persistent locator, representation change, reuse-policy
  change, fallback change, or caching architecture: out of scope by the
  admission rule.
- Unifying Horse-A's HORSE-A-STRUCTURAL-COUNTERS-v1 into the common
  WorkCounters slots: new design work; Horse-A reports what maps through
  the shared parser seam and the common schema slots stay honest
  (Unknown) where no exact equivalent exists. Its own structural record
  remains its attribution authority.
```

## 10. Formal campaign contract (frozen BEFORE any result)

The campaign execution contract is
`results/manifests/six-horse-performance-campaign-v1.toml`
(`MARKIT-76-SIX-HORSE-PERFORMANCE-CAMPAIGN-v1`), inheriting R7
(`protocol/R7-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-v1.md`) with the
six-horse amendments recorded there. Summary of the frozen rules:

```text
sampling         3 sessions (fresh worker processes) x (10 warmup + 30
                 measured) per case x horse per surface; warmups run the
                 full correctness path; identical setup semantics
order            per surface x session: frozen seeded case shuffle;
                 horse order = seeded base permutation rotated by
                 (j + s) mod 6 (seeded-base-permutation-rotate-v1) —
                 the campaign NEVER always runs H0..HorseA in identity
                 order; order materialized in the schedule manifest
                 before timing, recomputed by verification
campaign seed    first 64 bits BE of
                 SHA256("MARKIT-76-SIX-HORSE-PERFORMANCE-CAMPAIGN-v1\n"
                 + efa392752bfb2604e3d2fdfabaf0ba9825f70b74)
                 = 0xd7b11f1df0cc7cb5, fixed before any timing
cardinality      (22+362) x 6 = 2,304 cells/session; 92,160 timing
                 rows/session; 276,480 total; 2,304 attribution rows;
                 enforced exactly by the preflight/finalize
statistics       nearest-rank p50/p95 (p95 DESCRIPTIVE at n=30); no p99;
                 no outlier deletion; per-session summaries first — three
                 sessions are three process-level realizations, never 90
                 independent samples; session variability is reported
reporting        per workload -> per regime -> per horse, with session
                 variability, BEFORE any global aggregate; H0-relative
                 speedups pair per session before aggregation
outcomes         timeout / incorrect-result / unsupported-case /
                 failed-run classes are preserved and reported per horse;
                 never silently excluded; INCONCLUSIVE is a legal final
                 result when uncertainty prevents a defensible ranking
fail-closed      any failed qualified sample invalidates the campaign
                 (PRIMARY_CAMPAIGN_INVALID); warmup failures abort the
                 session; raw files append/create-only; receipts bind
                 spec/machine/schedule/binary (RunId carries the
                 executable SHA256)
```

## 11. Freeze / execution sequence

```text
1. independently reviewed PR merges  -> PERFORMANCE_FREEZE_COMMIT/TREE
2. on the formal host ONLY: build the release binaries under the frozen
   recipe (cargo build --release, profile release-primary-v1), record
   binary SHA256 identities, machine-capture + machine-verify, generate
   + verify the schedule, generate + verify the campaign receipt
3. preflight (fail closed) — then, and only then:
   FORMAL_PERFORMANCE_COLLECTION = AUTHORIZED
4. run-session x 3 x 2 surfaces, run-attribution x 2 surfaces, and the
   memory-lane runs; finalize each raw file
```

The exact commands for step 2–4 are produced by the freeze step and
recorded in the #76 update; the campaign itself is a SEPARATE task and is
NOT run as part of this preflight.

## 12. Stopping rule

This preflight stops at `PERFORMANCE_FREEZE_READY`. Gate A stays closed
unless a concrete mechanism-affecting defect appears; none did. No formal
performance numbers were collected here, and no mechanism changed.
```

FORMAL_PERFORMANCE_COLLECTION_STARTED = NO
```
