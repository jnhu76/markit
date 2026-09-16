# R1 Harness Contract — MARKIT-MARKDOWN-BENCHMARK-1

Status: **R1-CORRECTIVE-1 MATERIALIZED** (R1 base HARNESS_SUBSTRATE_PASS,
corrective applied per adversarial review)
Authority: GitHub Issue #22 + `protocol/R0-METHODOLOGY.md` (semantic truth —
not duplicated here) + `ROADMAP.md` (stage gates).
Scope: infrastructure materialization only. This file records what R1 built
and the interface boundaries later horses must live inside. It contains no
performance claims.

## 1. What R1 is

R1 proves exactly this chain, with no parser and no research claims:

```text
one deterministic case
-> common runner
-> one intentionally trivial null mechanism
-> enforced T_prepare / T_native boundary
-> explicit completion boundary
-> correctness hook
-> schema-valid result row
-> deterministic CaseId
-> deterministic case order from a fixed seed
-> supervised failure isolation (worker death never erases a case)
```

R1 explicitly does NOT contain: H0-H4 implementations, Markdown
scanner/parser/grammar code, AST/CST/tree/fragment/restart representations,
horse tuning, headline tables, PMU analysis, Markit algorithm or
architecture work.

## 2. Workspace topology

Independent Cargo workspace rooted exactly at
`research/benchmarks/markdown-ast-update/`. The repository root has no Cargo
workspace; this workspace does not leak upward (`rust-toolchain.toml` is
local to the benchmark tree).

```text
Cargo.toml            workspace manifest + frozen release profile
Cargo.lock            frozen dependency graph
rust-toolchain.toml   pinned toolchain (see manifest/environment.toml)

common/               markit-mdbench-common
instrumentation/      markit-mdbench-instrumentation
oracle/               markit-mdbench-oracle
runner/               markit-mdbench-runner
                      (+ r1-smoke, mdbench-gen-schema, mdbench-worker bins)
mechanisms/null-r1/   markit-mdbench-null-r1        (the ONLY mechanism in R1)
mechanisms/{full-rebuild,block-local,fragment-reuse,old-tree-subtree-reuse,restart-convergence}/
                      RESERVED — README only, zero code in R1
manifest/             environment.toml (environment/build policy record)
protocol/             R0 authority + R1 materialization docs
results/{raw,normalized,summary}/   README only — no benchmark output is committed
corpus/ prior-art/ report/ scripts/  stage-reserved directories
```

Dependency direction (enforced by Cargo):

```text
common
  ↑         ↑           ↑
oracle  instrumentation  null-r1
     \        |         /
           runner
```

Critical rules:

- a mechanism never depends on the runner;
- a mechanism never depends on the timer implementation (it cannot even
  name a clock type);
- the runner owns orchestration, clock access, and timer placement;
- runner -> null-r1 exists only so the smoke/worker binaries can wire the
  null mechanism through the real runner.

## 3. Substrate types (common)

`Source` (immutable shared `Arc<str>` UTF-8 bytes + `SourceId`),
`CanonicalEdit { [start,end) byte range + inserted UTF-8 }` with
char-boundary validation, `OperationKind` (R0 §8 set), `PayloadId/Shape/Size`
(R0 §9 names), `Seed`, `CaseKeyV1`/`CaseId`, `MechanismId`,
`FailureStatus`/`ExecutionStatus`/`CorrectnessStatus`, `Observed<T>`
(`Known`/`Unknown`/`NotApplicable`), `WorkCounters` (R0 §10 slots),
`WorkSink` (`CounterSink` records, `NoopWorkSink` discards), `Mechanism`,
`MechanismContext`, `Completed<State>`, and the source-inspection event
collector that derives the unique-inspection counters.

R1 deliberately does NOT define: Markdown nodes, AST/CST, fragments, trees,
restarts, or any grammar implementation. R0 froze semantic requirements, not
the R1 parser representation.

## 4. Mechanism phase boundary (frozen for horses)

```rust
trait Mechanism {
    type State; type Prepared; type Pending;
    fn id(&self) -> MechanismId;
    fn full_parse<W: WorkSink>(&self, source: &Source, cx: &mut MechanismContext<'_, W>)
        -> Result<Self::Pending, FailureStatus>;
    fn prepare_update<W: WorkSink>(&self, old_source, post_source, edit,
        old_state: &Self::State, cx: &mut MechanismContext<'_, W>)
        -> Result<Self::Prepared, FailureStatus>;
    fn update<W: WorkSink>(&self, old_source, post_source, edit, old_state, prepared, cx)
        -> Result<Self::Pending, FailureStatus>;
    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus>;
}
```

Invariants:

```text
prepare_update != update/native != complete
EVERY working phase — including prepare_update — receives a
MechanismContext: work timed in T_prepare must be attributable in the
A-LANE too (R1-CORRECTIVE-1, MAJOR-1). No phase may report work in one
lane that vanishes in the other.
MechanismContext exposes work-counter hooks only — never a timer/clock.
complete() is the explicit completion AUTHORITY boundary: the runner
requires a fully consumed Completed state inside T_native and
black_boxes it.
```

Completion-boundary precision (R1-CORRECTIVE-1, IMPORTANT-1 — this
corrects an earlier overclaim): `complete()` + `black_box()` establish an
**explicit completion authority boundary**, not a mechanical proof that
lazy work is impossible. `black_box` cannot recursively force deferred
work hidden behind iterators, closures, `OnceCell`s, interior mutability,
or lazy indexes/trees. R1 proves only that the runner REQUIRES a completed
state at `complete()` and does no mechanism work afterwards. Before formal
horse measurement, R4/R5 must additionally prove eager
normalized-result/state materialization for real horses — recorded as the
hard gate:

```text
EAGER_COMPLETION_VALIDATION_PASS
```

## 5. Timer design (R0 §7 materialized)

The runner is the only clock caller; each phase reads the clock exactly
twice (`PhaseGuard::start`/`stop`). UPDATE flow:

```text
T_prepare START -> mechanism.prepare_update -> T_prepare STOP
T_native  START -> mechanism.update
                  -> mechanism.complete     (completion authority boundary)
                  -> black_box(completed)   -> T_native STOP
OUTSIDE ALL TIMERS: oracle hook, checksum validation, row assembly,
serialization, logging, memory-window close, attribution finalization.
T_total = T_prepare + T_native   (checked arithmetic; never a third interval)
```

FULL_PARSE flow: single `T_native` around `full_parse + complete +
black_box`; `T_prepare` is recorded `NotApplicable` and `T_total == T_native`
by documented identity (an inapplicable phase contributes additive zero).

Timing values are integer nanoseconds. `T_prepare`/`T_native` overflow in
`u64` nanoseconds is checked (`TimingError::Overflow`) and poisons the run as
`InstrumentationUnavailable` with `Unknown` timing — never a silent
saturating number. A failed run keeps its lane but records `Unknown` values.

Clocks: `InstantClock` (real harness) and `ManualClock` (tests only; records
every read). Deterministic tests assert the exact clock-read sequence, which
is how "complete() ran inside T_native" and "oracle/serialization ran
outside every timer" are proven without flaky real-time assertions.

When a case runs under the process supervisor (§12), ALL timing is produced
inside the worker by this same code; supervisor overhead (spawn, wait,
classification) is structurally outside every timed interval.

## 6. Measurement lanes

```text
T-LANE = timing        real timer, NoopWorkSink in EVERY phase, no memory window
M-LANE = memory        per-case begin_case/end_case windows, no headline TimingRecord
A-LANE = attribution   CounterSink in prepare AND update, no headline TimingRecord
```

One run produces exactly one `LaneMeasurement` variant; the row's
`measurement` payload is tagged `{ "lane", "metrics" }` and the metric
structs reject foreign keys (`deny_unknown_fields`).

M-LANE lifecycle (R1-CORRECTIVE-1, MAJOR-2): a reporter opens one window
per case (`begin_case`) before the mechanism runs and closes it
(`end_case`) strictly after the mechanism finished — including on failed
runs. Per-case deltas therefore have a frozen authority; values can never
leak across cases. The memory record separates the four logical metrics:

```text
allocated_bytes   total bytes allocated in the window
allocation_count  number of allocation events in the window
peak_bytes        maximum live bytes observed IN the window
retained_bytes    bytes still held when the window closed
```

`peak_retained_bytes` is GONE — peak and retained are distinct R0 metrics
and must never be conflated. R1 ships the lifecycle, the schema, and two
reporters: `NoMemoryReporter` (honestly `Unknown`) and
`ManualMemoryReporter` (deterministic, `ManualClock` analogue for tests —
NOT a production instrumentor). A real allocator-backed reporter is later
infrastructure and must never run inside the timing lane.

## 7. Work counters

R0 §10 slots materialized with three-valued `Observed<u64>`
(`Known(v)` / `"UNKNOWN"` / `"NOT_APPLICABLE"` in JSON). `Known(0)` is never
conflated with `Unknown` or `NotApplicable`.

Reporting semantics are explicit in the method names (R1-CORRECTIVE-1,
MAJOR-1):

```text
add_*                        cumulative work; multiple calls ACCUMULATE,
                             never overwrite; add(0) is a measured zero
set_*                        single-valued gauges only (restart/convergence
                             distance); last value wins
record_fallback_to_full      event (one fallback occurred)
record_source_inspection     event: bytes [start,end) inspected in the
                             CURRENT phase; empty ranges ignored
```

Source-inspection facts are reported as raw byte-range EVENTS — never as
final counters. The `unique_source_intervals_inspected` /
`unique_source_bytes_inspected` slots are DERIVED by the common collector
(`CounterSink::finalize_derived`): sort + union of all prepare-phase AND
update-phase events, so overlaps and duplicates never double-count. The
derivation runs only for completed runs; failed runs keep the derived
slots `Unknown`.

PA authority: Parse Amplification (unique source bytes re-inspected /
logical edited bytes), when activated at a later stage, must be computed
by the common layer from these derived counters. Horses never compute PA
themselves — they can only emit raw events into the shared collector.

No H1-H4 counter behavior is invented anywhere in R1.

## 8. Case identity

```text
CaseId = SHA256(canonical_encode(CaseKeyV1))
CASE_ID_ALGORITHM_ID   = sha256-of-casekey-v1
CASE_KEY_VERSION_V1    = 1
```

`CaseKeyV1` (payload logical id, shape, size, old-source SHA256, operation,
edit start/end, inserted-text SHA256, generator id/seed) is encoded by an
explicit, versioned, little-endian, length-prefixed byte format — not a
serde/memory representation, not `DefaultHasher`, no UUID/timestamp/PID/
pointer/iteration index. The key contains NO mechanism id and NO lane, so
the same logical case keeps one CaseId across horses and lanes.

Raw-result `edit_meta(operation, edit)` uses the SAME operation contract
as `CaseKeyV1` (R1-CORRECTIVE-1, MAJOR-4):

```text
FULL_PARSE / QUERY:  no range, inserted_sha256 = null
DELETE:              range present, inserted_sha256 = null
INSERT / REPLACE_EQ / REPLACE_GROW / REPLACE_SHRINK / STRUCTURAL_EDIT:
                     range present, inserted_sha256 present
                     (empty insertions hash the empty string, exactly
                     like the case key)
```

Inconsistent combinations are rejected (`EditMetaError`), never
serialized. Table-driven tests cover every `OperationKind`.

GOLDEN VECTORS (R1-CORRECTIVE-1, IMPORTANT-2): the canonical encoding
bytes, the derived CaseId, the SplitMix64-v1 stream, and one full case
order permutation are pinned byte-for-byte in
`common/tests/golden_vectors.rs` and `runner/src/case_order.rs` tests. If
any of them change, the corresponding version/algorithm identifier MUST
change in the same commit. Never silently regenerate goldens.

## 9. Deterministic case order

```text
SHUFFLE_ALGORITHM_ID = splitmix64-v1+fisher-yates-lemire-rejection-v2
```

`order_cases`: sort by CaseId bytes (total order; input-enumeration
independent), then Fisher-Yates driven by the explicitly versioned
SplitMix64-v1 PRNG with bounded draws via Lemire multiply-shift WITH
rejection of the biased zone — exactly uniform over `0..n`.

(R1-CORRECTIVE-1, IMPORTANT-3: the retired v1 algorithm,
`splitmix64-v1+fisher-yates-mulshift-v1`, used a single widening multiply
without rejection; its "unbiased" description was an overclaim — the
mapping is not strictly uniform for arbitrary bounds. The algorithm
identifier changed with the fix; v1 is never reused.)

Seed is recorded in every row. `HashMap` iteration never determines order.

## 10. Result schema

`ResultRowV1` (in `runner/src/result.rs`) is the single schema authority;
`protocol/result-schema-v1.json` is GENERATED from it via
`mdbench-gen-schema` (schemars), and a test fails if the checked-in schema
drifts from the Rust model. Fields, taxonomies, and the lane payload are
documented in `protocol/result-schema.md`. Raw rows contain facts only — no
winner/rank/score/speedup field may ever be added (guard-tested).

Schema versioning note (R1-CORRECTIVE-1): the memory metric fields and
edit-metadata invariants changed BEFORE any row was published (the PR was
unmerged, `results/` empty), so `schema_version` remains `1`; the change
is recorded here instead of fabricating a v1->v2 history.

## 11. Null mechanism and smoke fixture

`mechanisms/null-r1`, id `"__r1_null__"`: O(1) scalar combining only,
understands no Markdown, scans no document body, retains no structure —
it cannot resemble H0-H4 by construction. Its checksum formula is
documented (`null_checksum`). `R1_SMOKE_ONLY` fixture: one tiny UTF-8
source (includes CJK + emoji) + one canonical INSERT; explicitly NOT a
benchmark workload (corpora belong to R3). Smoke output is written to
temporary files, labeled `R1_SMOKE_ONLY`/`NON_RESEARCH_RESULT`, never
committed, and never enters a research table.

## 12. Process-level failure isolation (R1-CORRECTIVE-1, MAJOR-3)

`catch_unwind` covers unwind panics only. OOM, stack overflow, `abort()`,
segfaults, and hangs kill the process, so in-process catching alone could
erase a case. The failure-isolation boundary is:

```text
parent supervisor
  -> worker execution boundary (mdbench-worker: the WHOLE case —
     initial-state build, mechanism phases, row emission)
  -> parent classifies worker termination / timeout
  -> the failure row SURVIVES (worker-reported, or parent-synthesized)
```

Classification (`runner/src/supervisor.rs`):

```text
exit 0 + row file      row is authoritative (incl. mechanism-reported failures)
budget overrun         Timeout        (supervisor kills the worker)
fatal signal           Crash          (SIGABRT/abort, stack-overflow abort,
                                       SIGSEGV, OOM-killer SIGKILL)
nonzero exit, no row   Crash          (abnormal exit)
```

Platform boundary, recorded honestly: R1 does NOT distinguish individual
fatal signals — the taxonomy values `oom` and `stack_overflow` remain
worker-self-reportable through `FailureStatus` rows only. OOM and
stack-overflow classification paths are proven with controlled worker
fixtures (`abort`, `exit_nonzero`, `hang`); reliable injection of real
OOM/stack-overflow is deliberately not attempted in the test environment.
A dedicated OOM/stack-overflow observer is later measurement-
infrastructure work.

`build_initial_state` additionally catches unwinds itself (runner-side),
so initial-state construction can never take a case down unclassified.
The worker has a whole-case `catch_unwind` guard as a final belt-and-
braces layer. Supervisor overhead is structurally outside
`T_prepare`/`T_native` (§5).

## 13. Verification gate

`scripts/verify-r1.sh` runs: `cargo fmt --all -- --check`,
`cargo check --workspace --all-targets`, `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, the null mechanism
end-to-end through the real runner (smoke binary), a supervised
`mdbench-worker` end-to-end run, and a schema-validity check of the
emitted rows. It validates the R1 acceptance gate only; it runs
no benchmark campaign.

## 14. Protocol amendments

R1 base recorded `NONE`. **R1-CORRECTIVE-1** (issued by the issue owner's
adversarial review before PR #26 merge) amends the R1 materialization as
follows — none of these touch R0-frozen semantics:

1. MAJOR-1: `prepare_update` now receives a `MechanismContext` (was
   context-free in the issued interface; the earlier ambiguity record #1
   is resolved). A-LANE now covers prepare + update; `WorkSink` moved to
   an explicit `add_*`/`set_*`/event model; source-inspection final
   counters were replaced by `record_source_inspection` events + a common
   union collector (`finalize_derived`).
2. MAJOR-2: `MemoryReporter` gained the `begin_case`/`end_case` per-case
   window lifecycle; the memory record now has `allocated_bytes`,
   `allocation_count`, `peak_bytes`, `retained_bytes`;
   `peak_retained_bytes` was removed (peak != retained).
3. MAJOR-3: process-level supervision added (`mdbench-worker`,
   `runner/src/supervisor.rs`) with honest classification limits.
4. MAJOR-4: `edit_meta(operation, edit)` — validated, operation-aware
   edit metadata; rejects contradictions instead of serializing them.
5. IMPORTANT-1: completion-boundary claim downgraded to an authority
   boundary; `EAGER_COMPLETION_VALIDATION_PASS` gate added before formal
   horse measurement.
6. IMPORTANT-2: golden vectors pinned (case-key bytes, CaseId,
   SplitMix64 stream, full permutation).
7. IMPORTANT-3: shuffle bounded draws now rejection-based; algorithm id
   bumped to `splitmix64-v1+fisher-yates-lemire-rejection-v2` (v1
   retired).

Unchanged R0-era resolution, kept for the record:

- FULL_PARSE rows serialize `T_prepare = NOT_APPLICABLE` and
  `T_total = T_native` (prepare does not exist for a clean full parse;
  the alternative zero would fabricate a measured quantity that was
  never measured).
