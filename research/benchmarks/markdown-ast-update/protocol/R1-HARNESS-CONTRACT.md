# R1 Harness Contract — MARKIT-MARKDOWN-BENCHMARK-1

Status: **R1 MATERIALIZED / HARNESS SUBSTRATE**
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
runner/               markit-mdbench-runner (+ r1-smoke, mdbench-gen-schema bins)
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
- runner -> null-r1 exists only so the smoke binary can wire the null
  mechanism through the real runner.

## 3. Substrate types (common)

`Source` (immutable shared `Arc<str>` UTF-8 bytes + `SourceId`),
`CanonicalEdit { [start,end) byte range + inserted UTF-8 }` with
char-boundary validation, `OperationKind` (R0 §8 set), `PayloadId/Shape/Size`
(R0 §9 names), `Seed`, `CaseKeyV1`/`CaseId`, `MechanismId`,
`FailureStatus`/`ExecutionStatus`/`CorrectnessStatus`, `Observed<T>`
(`Known`/`Unknown`/`NotApplicable`), `WorkCounters` (R0 §10 slots),
`WorkSink` (`CounterSink` records, `NoopWorkSink` discards), `Mechanism`,
`MechanismContext`, `Completed<State>`.

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
    fn prepare_update(&self, old_source, post_source, edit, old_state: &Self::State)
        -> Result<Self::Prepared, FailureStatus>;
    fn update<W: WorkSink>(&self, old_source, post_source, edit, old_state, prepared, cx)
        -> Result<Self::Pending, FailureStatus>;
    fn complete(&self, pending: Self::Pending) -> Result<Completed<Self::State>, FailureStatus>;
}
```

Invariants:

```text
prepare_update != update/native != complete
complete() is an explicit full-work boundary: the runner calls it inside
T_native and black_box()es the completed state, so lazy/unconsumed work
cannot escape timing.
MechanismContext exposes work-counter hooks only — never a timer/clock.
```

Interface notes preserved from the issued R1 design:

- `prepare_update` receives no `MechanismContext`; counter slots are
  reported from the `full_parse`/`update` phase context. If a horse later
  needs prepare-phase attribution, that is a protocol question for its own
  stage — not something to add silently.
- panics inside any phase are caught by the runner and recorded as
  `ExecutionStatus::Crash`; failures are never silently dropped.

## 5. Timer design (R0 §7 materialized)

The runner is the only clock caller; each phase reads the clock exactly
twice (`PhaseGuard::start`/`stop`). UPDATE flow:

```text
T_prepare START -> mechanism.prepare_update -> T_prepare STOP
T_native  START -> mechanism.update
                  -> mechanism.complete     (full-work boundary)
                  -> black_box(completed)   -> T_native STOP
OUTSIDE ALL TIMERS: oracle hook, checksum validation, row assembly,
serialization, logging.
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

## 6. Measurement lanes

```text
T-LANE = timing        real timer, NoopWorkSink, no memory collector
M-LANE = memory        MemoryReporter interface only, no headline TimingRecord
A-LANE = attribution   CounterSink -> WorkCounters, no headline TimingRecord
```

One run produces exactly one `LaneMeasurement` variant; the row's
`measurement` payload is tagged `{ "lane", "metrics" }` and the metric
structs reject foreign keys (`deny_unknown_fields`). R1 ships no production
allocator instrumentor: M-LANE reports are honestly `Unknown`
(`NoMemoryReporter`).

## 7. Work counters

R0 §10 slots materialized with three-valued `Observed<u64>`
(`Known(v)` / `"UNKNOWN"` / `"NOT_APPLICABLE"` in JSON). `Known(0)` is never
conflated with `Unknown` or `NotApplicable`. The null mechanism reports only
its true trivial facts (zero source bytes inspected, zero blocks reparsed,
never falls back, restart/convergence NotApplicable); no H1-H4 counter
behavior is invented anywhere in R1.

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

## 9. Deterministic case order

```text
SHUFFLE_ALGORITHM_ID = splitmix64-v1+fisher-yates-mulshift-v1
```

`order_cases`: sort by CaseId bytes (total order; input-enumeration
independent), then Fisher-Yates driven by the explicitly versioned
SplitMix64-v1 PRNG with unbiased index draws via 64-bit widening
multiplication. Seed is recorded in every row. `HashMap` iteration never
determines order.

## 10. Result schema

`ResultRowV1` (in `runner/src/result.rs`) is the single schema authority;
`protocol/result-schema-v1.json` is GENERATED from it via
`mdbench-gen-schema` (schemars), and a test fails if the checked-in schema
drifts from the Rust model. Fields, taxonomies, and the lane payload are
documented in `protocol/result-schema.md`. Raw rows contain facts only — no
winner/rank/score/speedup field may ever be added (guard-tested).

## 11. Null mechanism and smoke fixture

`mechanisms/null-r1`, id `"__r1_null__"`: O(1) scalar combining only,
understands no Markdown, scans no document body, retains no structure —
it cannot resemble H0-H4 by construction. Its checksum formula is
documented (`null_checksum`). `R1_SMOKE_ONLY` fixture: one tiny UTF-8
source (includes CJK + emoji) + one canonical INSERT; explicitly NOT a
benchmark workload (corpora belong to R3). Smoke output is written to
temporary files, labeled `R1_SMOKE_ONLY`/`NON_RESEARCH_RESULT`, never
committed, and never enters a research table.

## 12. Verification gate

`scripts/verify-r1.sh` runs: `cargo fmt --all -- --check`,
`cargo check --workspace --all-targets`, `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, the null mechanism
end-to-end through the real runner (smoke binary), and a schema-validity
check of the emitted rows. It validates the R1 acceptance gate only; it runs
no benchmark campaign.

## 13. Protocol amendments

```text
NONE
```

No R0 frozen meaning was changed. Ambiguities found and their R1 resolutions
(both conservative, both recorded for R2 review):

1. `prepare_update` has no counter context in the issued interface. R1
   preserved the issued signature; prepare-phase attribution, if ever
   needed, must be raised as a protocol question in R5, not added silently.
2. R0 says T_prepare for FULL_PARSE may be "NotApplicable or an explicitly
   documented zero". R1 records `NotApplicable` (with `T_total == T_native`)
   because a preparation phase does not exist for a clean full parse; the
   alternative zero would fabricate a measured quantity that was never
   measured.
