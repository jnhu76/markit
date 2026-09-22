# R7 — PRIMARY PERFORMANCE CAMPAIGN FREEZE (v1)

Status: **FROZEN CANDIDATE (MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-1, #31)**.
Base authority: PR #41 MEASUREMENT-SUBSTRATE-CORRECTIVE merge
`911788a2ad5fcb5b939d8bc3576d3aea77f7df62`.
Workload authority: #35 frozen real workload (unchanged; `WORKLOAD_IDENTITY_CHANGED = NO`).
Measurement substrate: #41 (timer boundary, attribution schema v2,
FULL_READ 110/110 + EDIT_WRITE 1810/1810 parity).

This document freezes the **campaign execution contract**: what runs,
how sessions and iterations run, how case and horse order are
determined, what machine/profile is allowed, what metrics are
qualified, how raw rows are identified, how failures are handled, how
quantiles are computed, and how cases/traces/files/projects are
aggregated. It collects **no primary performance numbers**. Primary
wall-clock timing belongs to the separate task
`MARKIT-31-PRIMARY-PERFORMANCE-RUN-1`, only after human review promotes
this freeze to `PRIMARY_PERFORMANCE_CAMPAIGN_READY`.

Implementation: the `campaign` crate (`markit-mdbench-campaign`, binary
`mdbench-campaign`). The campaign layer is orchestration only — it
reuses the existing runner dispatch, the five mechanism crates, the
workload adapters, `CaseId`, and result row v2 unchanged (task §42;
the crate-level separation note documents why `runner` itself stays
horse-agnostic).

---

## 0. Authority chain

```text
#22 mechanisms / runner / controlled protocol
#35 frozen primary real workload   (22 G0-strict FULL_READ files,
                                    362 G0_PRIMARY EDIT_WRITE cases)
#41 measurement substrate          (110/110 + 1810/1810 parity, schema v2)
THIS FREEZE: campaign execution contract (#31)
human review -> PRIMARY_PERFORMANCE_CAMPAIGN_READY
separate task -> MARKIT-31-PRIMARY-PERFORMANCE-RUN-1 (primary timing)
#33 synthesis / Weakness Map (later; consumes the frozen DQ matrix)
```

## 1. Frozen artifacts

| artifact | path |
|---|---|
| campaign manifest | `results/manifests/primary-performance-campaign-v1.toml` |
| machine manifest | `results/manifests/primary-machine-v1.toml` |
| schedule manifest | `results/manifests/primary-schedule-v1.jsonl` |
| campaign receipt | `results/manifests/primary-campaign-receipt-v1.json` |
| observation envelope schema | `protocol/campaign-observation-schema-v1.json` |

The protocol document (this file) explains; the manifests bind
machine/spec/order; the receipt hashes them. The campaign receipt
SHA256-binds, at minimum (task §41): campaign manifest, machine
manifest, schedule manifest, observation envelope schema, #35 workload
freeze receipt, FULL_READ manifest, EDIT_WRITE manifest, transition
registry, strict-surface profile, `result-schema-v2.json`, `Cargo.lock`,
and `manifest/environment.toml`. The receipt is produced BEFORE timing;
changing one bound artifact after the freeze invalidates the
`CampaignSpecId`.

## 2. Primary surfaces (frozen; #35 authority)

```text
Surface A — CLEAN_STATE: clean parse + native-state construction
            (NOT "raw parser speed")
            22 G0-strict FULL_READ files × H0-H4 = 110 logical dispatches

Surface B — EDIT_WRITE: 362 frozen G0_PRIMARY cases × H0-H4
            = 1810 logical dispatches
```

No additional real case may enter the primary campaign. `#31` consumes
the frozen workload; it never reselects files, projects, traces, or
edits. `project` in this campaign means: aggregation/provenance group
for already-frozen real workload rows.

## 3. Trace semantics: SINGLE_RESET

Every measured EDIT_WRITE iteration starts from the exact frozen
pre-edit source and a **freshly constructed valid pre-edit horse
state**; no state is retained from a previous measured iteration and
none is cloned from a cached build. The campaign therefore does NOT
claim long-session behavior, state aging, repeated-session
fragmentation, whole-editor interaction, or render-ready latency —
those remain separate future surfaces.

## 4. Timing boundary (unchanged from #41)

- CLEAN_STATE, inside timer: `full_parse` + native complete/seal +
  `black_box(native state)`. Outside: NormalizeV1, validate, checksum,
  oracle comparison, serialization.
- EDIT_WRITE, outside before timing: source lookup, pre/post
  construction, canonical-edit verification, **fresh pre-edit horse
  state construction**, pre-state verification.
- EDIT_WRITE, inside: `T_prepare` = `prepare_update` only;
  `T_native` = `update` + native complete/seal + `black_box(state)`.
- `T_total = T_prepare + T_native` — never a third enclosing timer.
- Attribution runs in separate processes; no WorkCounters inside
  timing sessions; no timing value from attribution processes.

## 5. Sampling policy (R0, retained)

```text
sessions = 3 (fresh worker processes; CLEAN_STATE and EDIT_WRITE
              never interleave in one process: 3 + 3 process runs)
warmup iterations per case × horse × session = 10
measured iterations per case × horse × session = 30
p50, p95 (no p99, no outlier deletion)
```

Warmup and measured iterations have IDENTICAL setup semantics; they
differ only in `sample_kind`. Warmups execute the full correctness
path — a wrong warmup is a campaign failure, not something to ignore.
Warmup rows are retained (`sample_kind = warmup`), auditable, and
excluded from primary summaries. No measured sample may be substituted
for a failed sample.

## 6. Expected cardinality (exact guards)

```text
logical cells per session   = (22 + 362) × 5            = 1,920
timing rows per session     = 1,920 × (10 + 30)         = 76,800
total timing rows           = 76,800 × 3                = 230,400
  measured                  = 1,920 × 30 × 3            = 172,800
  warmup                    = 1,920 × 10 × 3            = 57,600
attribution rows            = (22 + 362) × 5            = 1,920
schedule rows               = 3 × (22 + 362)            = 1,152
```

Per-lane identities (the granularity the preflight enumerates):

```text
timing session, CLEAN_STATE = 22 × 5 × (10 + 30)        = 4,400
timing session, EDIT_WRITE  = 362 × 5 × (10 + 30)       = 72,400
attribution, CLEAN_STATE    = 22 × 5                    = 110
attribution, EDIT_WRITE     = 362 × 5                   = 1,810
campaign-wide enumeration   = 230,400 + 1,920           = 232,320
```

A completed primary campaign must not silently contain fewer or more
rows; schedule verification and the preflight enforce these numbers
against the FROZEN surface cardinalities (never against the materialized
case list, which is the thing under test).

## 7. Campaign seed and derivation

```text
campaign_seed = first 64 bits (big-endian) of
    SHA256("MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-v1\n"
           + "911788a2ad5fcb5b939d8bc3576d3aea77f7df62")
              = 6000671815411757117 (0x5346ab392350f03d)
algorithm id  = sha256-first64-bigendian-v1
```

Chosen before any timing exists; never regenerated after results are
seen. Per-(surface, session) case-order seeds and the base horse
permutation seed are further SHA256 derivations with fixed domains
(`MARKIT-31-CAMPAIGN-SESSION-SEED-v1`,
`MARKIT-31-CAMPAIGN-HORSE-PERMUTATION-SEED-v1`).

## 8. Case order

Per `surface × session`: stable sort of case identities by CaseId
bytes, then the frozen shuffle
`splitmix64-v1 + fisher-yates-lemire-rejection-v2` — the SAME runner
primitive (`SplitMix64V1::next_below`), never a second implementation.
The order is materialized in
`results/manifests/primary-schedule-v1.jsonl` BEFORE timing and never
regenerated after results are seen. Schedule verification RECOMPUTES
the order from the seed, so any corruption is caught positionally.

## 9. Horse order

Identity order H0-H4 is never the execution order (that would confound
horse identity with time/thermal drift). Frozen policy
(`seeded-base-permutation-rotate-v1`):

```text
1. one seeded base permutation of H0-H4 (sorted labels + frozen
   seeded shuffle, derived from the campaign root seed);
2. for case order ordinal j (0-based) in session s (0-based):
   execution order = base permutation rotated by (j + s) mod 5.
```

Because the case order is itself a uniform shuffle, this is
deterministic near-exact position balance with no adaptive procedure
and no dependence on observed latency. The exact horse order is
recorded per scheduled case in the schedule manifest.

## 10. Metric qualification (#41 table, frozen)

```text
LATENCY            QUALIFIED
WORK_COUNTERS      QUALIFIED
PARSE_AMPLIFICATION QUALIFIED
CPU_TIME           UNAVAILABLE
ALLOC_COUNT        UNAVAILABLE
ALLOC_BYTES        UNAVAILABLE
PEAK_MEMORY        UNAVAILABLE
RETAINED_MEMORY    UNAVAILABLE
```

No primary memory lane is scheduled. Blanks are never printed as zero;
any result view requesting an unavailable metric must show
`UNAVAILABLE` or omit the view with explanation. No allocator
instrumentor is manufactured by this freeze.

Parse Amplification is a derived, common-layer quantity:

```text
PA = unique_source_bytes / logical_edited_bytes
   (numerator = schema-v2 combined unique coverage:
    old-union bytes + post-union bytes)

inspection_effort_amplification
   = source_bytes_inspected_total / logical_edited_bytes
```

The two are never conflated: unique coverage ≠ cumulative inspection
effort. The campaign's frozen denominator is
`logical_edited_bytes = max(edit_end − edit_start, inserted_bytes)` of
the canonical edit.

## 11. Raw observation identity

Raw output is an append/create-only envelope around ONE existing
schema-v2 result row (`protocol/campaign-observation-schema-v1.json`;
Rust authority `campaign/src/execute.rs::CampaignObservationV1`):

```text
CampaignObservationV1 {
    campaign_spec_id, run_id, session_id,
    surface (clean_state | edit_write),
    sample_kind (warmup | measured | attribution),
    session_ordinal (None for attribution),
    case_order_ordinal, horse_order_ordinal, iteration_ordinal,
    observation_id,
    result_row_v2  // the existing row, schema v2, facts only
}
```

The core row is NOT overloaded with campaign scheduling facts, and no
winner/rank/speedup/score/interpretation field exists at any raw
level. Stable identities:

- `CampaignSpecId` = SHA256 of the canonical campaign spec binding
  (workload freeze receipt digest, grammar/result authority, mechanism
  set, measurement-corrective version, result schema version,
  sampling policy, campaign seed, order algorithms, build profile,
  machine manifest digest, metric qualification, quantile/aggregation/
  failure policy, envelope schema). It binds NO measured value.
- `SessionId` = derived from CampaignSpecId + surface + session
  ordinal (attribution rows use a dedicated attribution derivation).
- `RunId` = execution-time binding of CampaignSpecId + approved runner
  git commit + machine manifest digest + build identity + **the SHA256
  of the exact executable that produces the rows**; all sessions of one
  primary campaign share the same RunId inputs. The executable digest
  is a BINDING, not a footnote: identical commit + toolchain + profile
  + lockfile can rebuild to different bytes, so build identity alone
  cannot prove "one binary". One CampaignSpecId therefore carries
  exactly ONE RunId, and `run-session` / `run-attribution` fail closed
  when the spec's raw directory already holds a different one — a
  rebuilt binary starts a new run or is archived deliberately, it never
  silently continues an existing campaign run.
- `ObservationId` = derived from RunId + SessionId + surface + CaseId
  + HorseId + sample_kind + iteration_ordinal. No two raw observations
  share one; schedule verification and preflight enumerate and guard
  uniqueness and cardinality, in the explicit scope below.
- Preflight enumeration scope (§13) is stated, never inferred. The
  three scopes enumerate structurally different id sets:

FULL_READ cases get a stable CaseId through the EXISTING `CaseKeyV1`
machinery (`operation = FullParse`, payload/source identity from the
frozen manifest, generator `CORRECTIVE-C-WORKLOAD-FREEZE-v1`) — no
second hashing algorithm. EDIT_WRITE CaseIds are derived by the same
function the #35 dry-run used, asserted equal by construction.

## 12. Machine and profile

The machine manifest (`results/manifests/primary-machine-v1.toml`)
records STABLE identity of the actual primary benchmark machine:
architecture, distribution/kernel, CPU vendor/model/microcode, cores,
SMT, NUMA topology, the selected CPU affinity with its core/sibling
relationship, governor + turbo policy, RAM, rustc/cargo/LLVM/target,
allocator policy, release profile id, RUSTFLAGS, Cargo.lock digest.
Transient values (current frequency, load, temperature, uptime) are
per-session preflight diagnostics, never machine identity.

CPU affinity: single-worker horse execution is pinned to ONE frozen
logical CPU (selection rule: lowest logical CPU ≠ 0, benchmark-free,
with recorded core_id and SMT siblings). The worker fails closed if
affinity cannot be established; no session may silently run on
arbitrary CPUs.

Host policy: governor/turbo/SMT are inspected and recorded, never
modified by tooling. If the primary host cannot provide a stable
policy: `MACHINE_PROFILE_NOT_READY` — no timing is invented anyway.
Campaign reports state exactly what is controlled and what is merely
recorded (the machine manifest's `control_notes`).

Build profile: the frozen `release-primary-v1` (opt-level 3,
lto thin, codegen-units 1, incremental false, panic unwind,
target-cpu default, RUSTFLAGS "", allocator rust-system-default).
Build ONCE before session execution; never compile inside timing
sessions; all sessions use byte-identical binaries.

"Byte-identical" is enforced, not merely documented: the executable
SHA256 is part of the `RunId` (§11), each `run-session` /
`run-attribution` invocation hashes its own executable before doing
anything else and fails closed if that digest cannot be computed, and a
spec directory that already holds raw data under a different `RunId`
aborts the session. The real `run-session` / `run-attribution` entry
points also refuse non-release builds.

## 13. Preflight (non-measuring, fail-closed)

Every session runs a non-timed preflight verifying: campaign receipt
valid, machine matches the frozen manifest (exact stable-field
equality), binary/build identity matches, runner commit available,
Cargo.lock matches, CPU affinity applicable, sources materialized,
workload receipt valid, schedule valid, schema v2 + envelope schema
un-drifted, no duplicate ObservationIds, expected cardinalities.
Transient environment (timestamp, load average, available memory,
temperature where readable) is recorded as diagnostics only. If the
host is clearly busy or the policy mismatches, the session aborts
BEFORE collecting data. There is no adaptive inclusion/exclusion based
on whether timing "looks good."

The ObservationId enumeration is **scope-explicit**. A timing session,
an attribution lane, and the whole campaign check different identity
sets, so the scope is a required argument and never inferred from which
flags happened to be present:

```text
preflight                          scope All         232,320 ids
preflight --timing S --session N   scope Timing      4,400 / 72,400 ids
preflight --attribution S          scope Attribution 110 / 1,810 ids
```

Each scope reports what it enumerated (lane, rows, warmup/measured/
attribution split, distinct ids, duplicates) into the preflight
diagnostics, so the JSON output shows which identities were actually
checked. `All` unions every lane into one campaign-wide id set, which
also catches cross-lane collisions (an attribution id colliding with a
timing id). Expectations always come from the frozen surface
cardinalities; a missing case, an extra case, and a repeated case each
fail closed with their own blocker.

## 14. Quantiles (frozen before any timing)

Deterministic **nearest-rank** quantiles per `case × horse × session`
from exactly 30 measured observations, sorted ascending, 1-indexed, no
interpolation:

```text
p50 rank = ceil(0.50 × 30) = 15
p95 rank = ceil(0.95 × 30) = 29
```

With n = 30, p95 is a descriptive tail indicator, not a
high-confidence tail model. The 90 observations of three sessions are
never pooled and presented as 90 independent replications.

## 15. Summaries and speedups

- Session-level first: per `case × horse × session`, p50/p95 of
  `T_prepare`, `T_native`, `T_total`; all three session summaries are
  retained.
- Case-level point estimate: median of the 3 session p50 values (and
  of the 3 session p95 values); constituents stay visible.
- H0-relative speedup preserves pairing FIRST:
  `speedup(C,H,S) = p50_T_total(C,H0,S) / p50_T_total(C,H,S)` per
  session; per-case across sessions = geometric mean of the 3 positive
  ratios. Never `mean(H0)/mean(Hx)` after aggregation. `> 1.0` = H
  faster than H0; `= 1.0` parity; `< 1.0` slower.
- Session instability (`SESSION_UNSTABLE`) is a DIAGNOSTIC flag only
  (frozen threshold: max/min session-p50 ratio > 1.5); it never
  discards or downweights data.

## 16. Aggregation policy

- **CASE_WEIGHTED**: each frozen CaseId contributes one equal unit —
  descriptive of the exact frozen case population only; never called
  external replication.
- **PROJECT_MACRO** (primary for relative speedups): hierarchical
  equal-weight geometric means — session ratios → per CaseId → per
  trace (BREAK/RESTORE of one trace related) → per file → per project
  → equal-weight projects. A repository with 100 generated cases
  cannot dominate one with 10.
- **FAMILY_MACRO** (secondary): E1-E6 families equal weight after
  their project-macro result; never the primary "best mechanism"
  statement.
- Absolute latency: per-case/file/project distributions are retained;
  geometric means of nanoseconds are never presented as physical
  throughput; throughput only as specified total work / specified
  total time.
- Experimental hierarchy (statistical dependence structure):
  `iteration → session → case/trace → file → project`. The 362
  EDIT_WRITE cases are NOT 362 independent real-world samples.

## 17. Failure handling (fail-closed)

Any primary sample with `execution_status != pass`, or
`correctness_status != pass`, or a qualified timing metric `UNKNOWN`
(warmup or measured alike) is retained as raw evidence and NEVER
deleted, replaced, retried, imputed, or converted to zero. The
campaign becomes `PRIMARY_CAMPAIGN_INVALID` for headline comparison;
execution stops cleanly after recording; investigation is separate. If
the substrate must change: new `CampaignSpecId`, new `RunId`, rerun
the affected campaign — missing rows are never patched into an old
campaign. No outlier deletion of any kind; only a WHOLE session may be
invalidated with a recorded reason (the invalid session stays
archived; a replacement gets a new SessionId).

## 18. Raw-data immutability and finalization

```text
results/raw/<CampaignSpecId>/<RunId>/
  timing/session-<N>-clean_state.jsonl
  timing/session-<N>-edit_write.jsonl
  attribution/clean_state.jsonl
  attribution/edit_write.jsonl
```

Append/create-only. A raw file is FINALIZED only after being re-read
from disk and checked against the frozen contract that produced it:
envelope schema, result schema v2, CampaignSpecId, RunId, SessionId,
surface, session ordinal, and the ObservationId re-derived from each
row's own identity fields (never trusted as stored); exact row count
and warmup/measured/attribution split; and exact coverage — the
(case × horse × sample_kind × iteration) set against the frozen
schedule the execution consumed, with no missing and no extra entry.

```text
all checks pass -> write raw-file receipt (verdict FINAL_RAW_FILE_PASS)
                -> SESSION_COMPLETE / ATTRIBUTION_COMPLETE
any check fails -> PRIMARY_CAMPAIGN_INVALID, no success receipt;
                   the file is retained and marked .jsonl.invalid.json
```

The finalization verifier considers the expectations frozen: they are
read from the schedule and the frozen surface cardinalities, so a
truncated schedule fails before the session starts rather than lowering
the bar. After finalization the receipt's SHA256, row counts, and
first/last observation id bind the file; finalized files are never
overwritten and never receive a "success" receipt they did not earn.

## 19. Concurrency and I/O

One case at a time, one horse at a time, one iteration at a time, one
algorithm worker. No harness parallelism, no background prefetch, no
per-horse thread pools — the comparison stays algorithmic, not
scheduler throughput. Sources are materialized and hash-verified
before the measured phase; no disk read / checkout / decompression /
JSON parse / manifest lookup / network ever happens inside mechanism
timing, and the campaign performs no network access at all.

## 20. Attribution lane

One attribution dispatch per logical `case × horse` (110 + 1810 =
1920 rows), in separate processes, deterministic (case, horse) order,
schema v2, counters = cumulative actual work (including
discarded fallback/restart work). Non-timed determinism checks on
representative cases require byte-identical attribution rows across
repeated executions; nondeterminism would be `CAMPAIGN_FREEZE_BLOCKED`
— counters are never averaged silently.

## 21. Verification commands

```text
scripts/verify-r7.sh                 # the full freeze gate
cargo run -q -p markit-mdbench-campaign --bin mdbench-campaign -- \
    manifest-verify .                # CAMPAIGN_MANIFEST_OK
    receipt-verify .                 # CAMPAIGN_RECEIPT_OK
    schedule-determinism .           # SCHEDULE_DETERMINISM_PASS
    preflight .                      # CAMPAIGN_PREFLIGHT_PASS (scope All)
    preflight . --timing clean_state --session 0   # scope Timing
    preflight . --attribution edit_write           # scope Attribution
    smoke . --out <dir> --inject-failure   # NON_RESEARCH_SMOKE_PASS
```

The `smoke` subcommand is the only execution path this freeze runs:
fake deterministic clock, tiny fixed subset, output OUTSIDE
`results/raw`, every row tagged
`CAMPAIGN_SMOKE/NON_RESEARCH_RESULT`, with a poisoned-hook run proving
failure propagation (wrong result retained → campaign invalid → clean
stop). Wall-clock smoke values are never performance evidence.

## 22. DQ answer matrix (EMPTY PLACEHOLDER — bridge into #33)

No DQ has been answered; no performance conclusion exists in this
freeze. The later analysis task fills this matrix under the frozen
campaign spec; every verdict names its DQ, the qualifying MQs, the
evidence class, and the metric families used.

| DQ | Real observation | Cross-project replication | Work evidence | Controlled follow-up | Verdict |
| -- | ---------------- | -------------------------- | ------------- | -------------------- | ------- |
| DQ1 local text edit: best mechanism | | | | | |
| DQ2 affected-block-size growth: first degradation | | | | | |
| DQ3 container edit (list/blockquote): best mechanism | | | | | |
| DQ4 fence propagation: degradation and why | | | | | |
| DQ5 reference dependency change: highest cost | | | | | |
| DQ6 small documents: H0 outright cheaper? | | | | | |
| DQ7 N/B/L/K/F regimes: ranking crossings | | | | | |

DQ-specific strata (frozen): DQ1 → E1 LOCAL_TEXT; DQ2 → affected-block
strata + later controlled B sweep; DQ3 → E3 CONTAINER_DEPTH (list and
blockquote separated where evidence allows); DQ4 → E4 FENCE_OPEN_CLOSE
(BREAK and RESTORE visible separately); DQ5 → E6 REFERENCE_DEFINITION
(dependency/restart/rematerialization facts visible); DQ6 → small-file
strata, H0-relative paired comparison; DQ7 → real observation first,
controlled confirmation later. No DQ may be answered by a global
average when its regime says otherwise.

## 23. Claim discipline

Campaign raw rows and summaries contain facts and mathematical
ratios/aggregates only — no winner/best/worst/rank/score, no
"generally faster", no design choice. Interpretive claims belong after
measurement review. The first campaign does not itself promote
`ATTRIBUTED_WEAKNESS` or `DESIGN_OPPORTUNITY` (that is #33's work on
this matrix).

## 24. Verdict

```text
PRIMARY_PERFORMANCE_CAMPAIGN_FREEZE_PASS
WORKLOAD_IDENTITY_CHANGED = NO
HORSE_SEMANTICS_CHANGED   = NO
CAMPAIGN_SPEC_FROZEN      = YES
MACHINE_PROFILE_FROZEN    = YES
SCHEDULE_FROZEN           = YES
AGGREGATION_POLICY_FROZEN = YES
RAW_RESULT_IDENTITY_FROZEN = YES
PRIMARY_TIMING            = NOT_STARTED
PRIMARY_PERFORMANCE_CAMPAIGN_READY = CANDIDATE_FOR_HUMAN_REVIEW
```

Human review owns the promotion to `..._READY = PASS`. The next task,
only after review and merge, is `MARKIT-31-PRIMARY-PERFORMANCE-RUN-1`.
