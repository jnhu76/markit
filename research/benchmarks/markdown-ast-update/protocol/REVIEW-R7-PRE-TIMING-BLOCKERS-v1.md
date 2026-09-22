# Maintainer code review — R7 campaign freeze, pre-timing blockers (v1)

Task: MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-1 (#31), PR #42.
Reviewer: maintainer review of the branch
`research/22-primary-performance-campaign-freeze-1` against base
`911788a2ad5fcb5b939d8bc3576d3aea77f7df62`.

This is the second review round. The first (fresh-context adversarial,
Q1-Q30) is recorded in
`protocol/REVIEW-R7-CAMPAIGN-FREEZE-ADVERSARIAL-v1.md`; it accepted the
workload, horse semantics, schedule/statistics design, and closed its
own defect list. This round re-read the same code with the specific
question *"does the gate do what its name says?"* and found two
pre-timing blockers plus one incomplete finalization step.

Verdict before remediation:

```text
WORKLOAD DESIGN              PASS
HORSE SEMANTICS              PASS
SCHEDULE / STATISTICS DESIGN PASS
RAW EXECUTION FAIL-CLOSEDNESS NOT YET PASS
PRIMARY TIMING               NO-GO
```

Nothing in the workload, the horse roster, the sampling policy, the
quantile definitions, or the aggregation policy was in question. Both
blockers were about checks that reported success without testing what
they claimed.

---

## Blocker 1 — the attribution preflight never checked attribution

`preflight::check_observation_uniqueness` took `session_ordinal:
Option<u32>` and used it as BOTH "which scope" and "which session":

```text
session_ordinal = None
  -> expanded to the timing sessions 0..session_count
  -> every enumerated session was Some(..)
  -> the attribution branch (session.is_none()) was unreachable
```

`run-attribution` calls the preflight with exactly that shape
(`Some(surface), None`), so the command that guards the attribution
lane enumerated three TIMING sessions of one surface and checked their
warmup/measured identity: 72,400+ ids that the attribution run never
writes, while the 110/1810 attribution identities it does write were
never enumerated. The gate's claim — "preflight covers the campaign's
raw identity" — did not hold for the lane it was reporting on.

**Remediation.** The scope is now an explicit enum; timing and
attribution can no longer be confused by construction:

```rust
pub enum PreflightScope {
    All,                                      // whole campaign
    Timing { surface: Surface, session: u32 }, // one timing session
    Attribution { surface: Surface },          // one attribution lane
}
```

- `All` enumerates both surfaces × 3 timing sessions AND both
  attribution lanes, unions every id into one campaign-wide set, and
  asserts the frozen totals: 232,320 rows, 0 duplicates, 0 missing,
  0 extra.
- Expectations come from the FROZEN surface cardinalities, never from
  the materialized case list (a missing or extra case must move the
  observed count, not the target).
- The CLI has no inferred scope: `preflight` = All,
  `preflight --timing S --session N` = Timing,
  `preflight --attribution S` = Attribution. The old `--surface` /
  `--session` shape is rejected with an explanation instead of being
  reinterpreted.
- The enumeration result (per-lane rows, warmup/measured/attribution
  split, distinct ids, duplicates) is emitted in the preflight
  diagnostics, so the JSON output shows WHICH identities were checked.
- Negative tests: duplicate attribution case, missing case (21 vs 22),
  extra case (363 vs 362), duplicate timing case, and an out-of-range
  session each produce their own blocker; the frozen-shape tests pin
  110 / 1,810 / 4,400 / 72,400; the CLI scope parser is tested against
  every ambiguous flag shape (legacy `--surface`, bare `--timing`, bare
  `--session`, both scopes at once).

## Blocker 2 — "the same binary" was recorded, not enforced

R7 §12 freezes "build ONCE; all sessions use byte-identical binaries",
but `RunId` bound only `CampaignSpecId` + runner commit + machine
manifest digest + build identity (rustc, target, profile, Cargo.lock
digest). Identical commit, toolchain, profile, and lockfile can rebuild
to different bytes, so a rebuilt executable produced the SAME `RunId`
and would have been mixed into the same campaign run. The executable
SHA256 existed only as a `run-file-receipt` field, written after the
rows were already on disk.

**Remediation.** The executable digest enters the `RunId` binding
(field order: spec id, runner commit, machine manifest digest, rustc,
target, build profile id, Cargo.lock digest, executable SHA256), and
is enforced before execution rather than recorded afterwards:

- `identity::current_executable_sha256()` fails closed when the current
  executable cannot be resolved or hashed — never `"unknown"`;
- `run-session` and `run-attribution` compute it before anything else
  and bind it into the `RunId`;
- the receipt writer re-derives the digest and refuses to write a
  receipt from an executable other than the one the `RunId` names;
- one `CampaignSpecId` carries exactly ONE `RunId`:
  `ensure_single_run_identity` aborts a session when the spec's raw
  directory already holds a different run, with a message telling the
  operator to archive the previous run deliberately. A rebuilt binary
  now starts a new run instead of silently continuing an old one.

The executable digest is deliberately NOT part of `CampaignSpecId`:
the spec binds the scientific campaign, and #42's merge commit would
otherwise change the spec id of an unchanged experiment.

## Incomplete — raw finalization wrote receipts without verifying

`write_run_receipt` recorded sha256 / row_count / first-last id and was
then followed by the `SESSION_COMPLETE` verdict. Nothing had re-read
the file, so a truncated, mixed, or foreign run could be receipted as
complete. Remediation (new `campaign/src/finalize.rs`): the raw file is
re-read and checked against the frozen contract before any receipt or
verdict — envelope schema, result schema v2, CampaignSpecId, RunId,
SessionId, surface, session ordinal; the ObservationId re-derived from
each row's own identity fields (never trusted as stored); the exact row
count and warmup/measured/attribution split; and exact coverage of the
(case × horse × sample_kind × iteration) set against the same schedule
rows the execution consumed, reported as missing/extra.

```text
all checks pass -> raw-file receipt (FINAL_RAW_FILE_PASS) -> SESSION_COMPLETE
any check fails -> PRIMARY_CAMPAIGN_INVALID + .jsonl.invalid.json, no receipt
```

The expectation is built from the schedule BEFORE the session starts,
so a truncated schedule (not 22 / not 362 cases) fails at build time
instead of lowering the bar for a completed run. A failed finalization
never receives a success receipt and never has its evidence deleted.

## Machine profile (accepted, not a blocker)

`schedutil` governor, turbo enabled, SMT enabled, `cpu1` affinity is not
a fixed-frequency environment. Horse order is position-balanced and the
three independent sessions expose drift, so this is not promoted to a
blocker. It is a standing obligation instead: if `SESSION_UNSTABLE`
recurs, a 1-2% difference must not be written up as a mechanism effect.

## Design that was accepted unchanged

```text
22 CLEAN_STATE / 362 EDIT_WRITE, SINGLE_RESET, 3 sessions, 10 + 30,
nearest-rank p50/p95, paired H0 speedup, PROJECT_MACRO,
CASE_WEIGHTED / FAMILY_MACRO secondaries, balanced horse order,
deterministic schedule, no outlier deletion, failure retention,
DQ1-DQ7, MQ qualification
```

## Verification after remediation

Run on the review host (a development machine, NOT the primary
benchmark host) at commit `a44011e` + this remediation:

```text
cargo fmt --all -- --check                            PASS
cargo clippy --workspace --all-targets -- -D warnings PASS (no warnings)
cargo test --workspace                                PASS (77 suites, 383 tests, 0 failures)
campaign manifest verify                              PASS  CAMPAIGN_MANIFEST_OK
campaign receipt verify                               PASS  CAMPAIGN_RECEIPT_OK
                                                            spec_id ad45c7cb...c66f2 (unchanged)
schedule determinism                                  PASS  SCHEDULE_DETERMINISM_PASS
NON_RESEARCH fake-clock smoke                         PASS  NON_RESEARCH_SMOKE_PASS
                                                            timing 20 / attribution 10 /
                                                            failure_propagated
```

Preflight enumeration, release binary, all three scopes:

```text
scope All         232,320 rows  232,320 unique  0 duplicates  (8 lanes)
scope Timing      clean_state:0  4,400 rows (1,100 warmup + 3,300 measured)
scope Attribution edit_write     1,810 rows (1,810 attribution, 0 timing)
```

The preflight verdict on this host is `CAMPAIGN_PREFLIGHT_BLOCKED` with
12 blockers, ALL of them host binding: this machine is not the frozen
primary host (every stable machine field differs — kernel, distro, CPU
vendor/model/microcode, core and NUMA counts, governor, turbo policy,
RAM). That is the documented fail-closed behavior: R7 §13 requires
host-binding equality, and `scripts/verify-r7.sh` states it must run on
the primary machine. No non-host blocker exists, so the receipt,
schedule, schema, workload, and identity-enumeration checks all pass;
the host-binding pass can only be observed on the primary host.

`CampaignSpecId` is unchanged by this remediation (`ad45c7cb...`), as
required: the executable digest was deliberately bound into `RunId`,
not into the spec.

No primary timing was run: `PRIMARY_TIMING = NOT_STARTED` is unchanged.
The H2/H3 matrix was not re-run (no mechanism, workload, or scheduling
change), and no performance number changed in this remediation.

## Outcome

```text
PRIMARY_PERFORMANCE_CAMPAIGN_FREEZE  REQUEST_CHANGES -> remediated
PRIMARY_TIMING                       NO-GO
PRIMARY_PERFORMANCE_CAMPAIGN_READY   CANDIDATE_FOR_HUMAN_REVIEW
```
