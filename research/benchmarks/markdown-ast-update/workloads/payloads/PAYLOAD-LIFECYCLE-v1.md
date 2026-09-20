# PAYLOAD-LIFECYCLE-v1 — payload identity, edits, BREAK/RESTORE, traces, positions

Status: **CORRECTIVE-A SEMANTIC SUBSTRATE — PRE-MEASUREMENT, NO FREEZE GRANTED**
Authority: issue #35 Corrective-1 §8 + #35 Workload Construction Algorithm v1
(payload sections) + `protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` §8.
Executable form: `semantics/src/payload.rs` (const
`PAYLOAD_LIFECYCLE_VERSION = "PAYLOAD-LIFECYCLE-v1"`, schema tag
`real-payload-v1`). Generated schema:
`workloads/payloads/schema/real-payload-v1.schema.json`.

---

## 0. What a payload must preserve

Every payload/trace step must retain enough information to reconstruct its
own history — not merely to reproduce a hash:

```text
grammar_id                lane the transition is claimed under
base_source_sha256        the trace's origin document
pre_source_sha256         this step's starting bytes
post_source_sha256        this step's resulting bytes
trace_id / trace_form     which history this step belongs to
step                      its index in that history
syntax_target             the construct the payload is about
edit_family / operation_variant / expected_transition
expected_pre              PREDICATE-v1 assertions on the pre state
expected_post             PREDICATE-v1 assertions on the post state
requested_position        early | middle | late (when anchored)
actual_anchor_byte        the real anchor the request resolved to
actual_relative_position  that anchor's position / source length
edit                      canonical byte range + inserted bytes VERBATIM
inserted_sha256           digest of those same bytes
memberships               logical workload sets this payload belongs to
```

`edit` carries the inserted bytes themselves. The digest is an integrity
check over them, never a substitute: a digest alone cannot reconstruct
content, and a payload that only had a digest would be unusable for
re-running the edit.

`expected_pre` and `expected_post` must both be non-empty. A payload whose
pre/post hashes reproduce but which asserts nothing about syntax state
proves only that bytes changed — it cannot show that the *claimed* change
happened — so an empty assertion list is `MissingTransitionAssertion`
(§7), not a vacuous pass.

## 1. Deterministic identity

```text
payload_id = "rp1:" + first 32 hex chars of
             SHA256(canonical_json(identity fields))
```

Identity fields are exactly: namespace, `source_id`, `source_path`,
`grammar_id`, `pre_source_sha256`, `syntax_target`, `edit_family`,
`operation_variant`, `edit_start`, `edit_end`, `inserted_sha256`,
`expected_transition`, `expected_pre`, `expected_post`, `trace_id`,
`trace_form`, `step`.

The three assertion fields are identity fields on purpose. Two records that
differ only in what they claim to prove are not the same payload:
deduplicating them by edit would collapse a disagreement between two
assertion sets onto one id and hide it. `source_path` is the
repository-relative source path, never a machine path, so an id is a
property of the bytes and the contract rather than of the checkout that
produced it.

Properties this buys:

```text
- the same semantic payload always has the same id, on any machine;
- observed performance, wall-clock, or allocation facts never enter identity;
- a payload cannot be "the same" while testing a different transition: the
  transition labels are part of the identity;
- the namespace is deliberately distinct from the R1 synthetic CaseKeyV1 /
  CaseId space. Real payloads are not squeezed into the synthetic identity
  scheme, and the R1 synthetic vectors stay pinned and untouched
  (semantics/tests/contracts.rs).
```

## 2. Validation of a payload

`validate_payload(record, pre_source, base_source)` checks, in order:

```text
payload_id == identity()                     else PayloadIdMismatch
inserted_sha256 == sha256(inserted bytes)    else InsertedDigestMismatch
pre_source_sha256 matches the bytes          else PreSourceHashMismatch
base_source_sha256 matches the bytes         else BaseSourceHashMismatch
trace_id non-empty                           else EmptyTraceId
anchor byte <= pre source length             else PositionOutOfRange
transition valid under the lane oracle       else TransitionInvalid
edit applies and reproduces the post hash    else EditInvalid
```

The last two go through `TRANSITION-ORACLE-v1`, so a payload whose declared
state transition does not hold is invalid even when every hash matches. The
report carries the full `TransitionReport` plus
`reconstructed_post_sha256`, so an invalid payload can be diagnosed without
re-running anything.

## 3. Canonical edit

```text
edit_start / edit_end   half-open UTF-8 byte range in the PRE source
inserted_text           exact bytes to insert (may be empty)
```

The edit is applied through the frozen R0 canonical-edit path, so
boundaries must be UTF-8 char boundaries and within range. Post sources are
materialized by the host **outside** any mechanism timer (the frozen timer
contract); the payload lifecycle never implies otherwise.

## 4. Real anchors and position dedup

Position requests are percentiles of the pre source, resolved against real
syntax anchors found by the profiler — never against arbitrary byte offsets:

```text
EARLY   target fraction 0.10
MIDDLE  target fraction 0.50
LATE    target fraction 0.90
```

Resolution rule (frozen):

```text
candidate anchor set   the profiler's facts for the lane, in source order
relative_position      anchor.start / pre_source_len   (0.0 if len == 0)
choose                 the anchor minimizing
                       (|relative_position - target|, start byte, occurrence)
```

The third key is the `occurrence` index frozen in the profiler contract; it
is what makes resolution deterministic when two facts share a start byte.

Dedup rule: requests that resolve to the **same anchor identity**
(`kind#occurrence@start-end`) are ONE payload, not three coverage cases:

```text
requested_positions      all requests that landed on this anchor
distance_from_target     per request: target fraction and actual distance
dedup_identity           the shared anchor identity
deduplicated             true when more than one request collapsed here
```

A deduplicated record is evidence of one physical anchor, and must never be
reported as "three-position coverage". A request with no anchors at all is
reported in `uncovered_requests` — absence of an anchor is recorded, not
silently filled with a nearby unrelated construct.

`p01` (single anchor: EARLY/MIDDLE/LATE collapse) and `p06` (partial dedup:
EARLY collapses to one anchor, MIDDLE/LATE to another) pin both shapes.

## 5. BREAK / RESTORE

A BREAK/RESTORE pair is a two-step history with a frozen middle state:

```text
S0  base (clean) source
 |  break_edit
S1  broken state
 |  restore_edit
S2  restored source
```

Enforced rules:

```text
- the BREAK edit is applied to S0 and validated against
  break_expected_pre / break_expected_post under the lane oracle;
- the RESTORE edit is applied to S1 (the frozen broken state), and its
  transition is validated against restore_expected_pre /
  restore_expected_post;
- if applying the RESTORE edit to the CLEAN source reproduces S1, the pair is
  not a real BREAK/RESTORE: RestorePreIsNotBrokenState;
- restore_exact = (S2 == S0 byte-for-byte). When the fixture declares
  expected_restore_exact and S2 differs, RestoreNotExact is a failure.
  When a case intentionally restores to a *different* valid state,
  expected_restore_exact = false and the difference is reported, not hidden;
- the report carries the full SHA chain:
  base_source_sha256, s1_source_sha256, s2_source_sha256,
  break_pre/post_sha256, restore_pre/post_sha256, restore_exact,
  expected_restore_exact, valid, failure_codes, and both TransitionReports.
```

`p02-g1-table-delimiter-break.toml` pins the exact-restore case: the G1
table is broken by a delimiter edit, then restored to the original bytes, and
the chain proves both the transition and the byte-level restoration.

## 6. Chained traces

```text
trace_id      identity of the history
trace_form    single_reset | local_burst | document_session
grammar_id    the lane every step's transition is claimed under
steps[]       ordered TraceStep { step, edit, expected_transition,
                                  post_source_sha256 }
```

Replay rules:

```text
- steps are replayed from the base source in order;
- each step's edit is applied to THAT step's pre-source, and its coordinates
  therefore belong to that step's pre-source — not to the base source, and
  not to the previous step's coordinates;
- the step's post hash must match the declared post_source_sha256, and the
  next step continues from those bytes (the hash chain);
- step indices must be exactly 0..n-1 (StepIndexMismatch otherwise);
- every step's report records its own pre_source_sha256, post_source_sha256,
  expected_transition, applied flag, and failure codes;
- a trace under an unregistered grammar_id is not replayable evidence
  (UnknownGrammarLane), exactly as an unknown lane invalidates a single
  transition request.
```

`trace_identity_key = "<form>:<trace_id>"` is the history identity. It is a
*label*, so the report also carries `declared_steps_sha256`, the digest of
the declared step sequence: two traces can share a label while declaring
different steps, and the report makes that difference visible rather than
hiding it behind the name.

Two histories that reach byte-identical sources remain **distinct** trace
identities when their retained mechanism state may differ: a `single_reset`
history and a `local_burst` history with the same final bytes are not the
same experiment, and the identity key keeps them apart.

`p03-g1-content-only-control.toml` carries a three-step `local_burst` trace
whose later steps use coordinates in their own pre-sources, and
`semantics/tests/lifecycle.rs` pins the coordinate/hash-chain failures.

## 7. Failure codes (closed)

```text
payload   PayloadIdMismatch, InsertedDigestMismatch, PreSourceHashMismatch,
          BaseSourceHashMismatch, EmptyTraceId, MissingTransitionAssertion,
          PositionOutOfRange, EditInvalid, TransitionInvalid
break/    BreakEditInvalid, RestoreEditInvalid, RestorePreIsNotBrokenState,
restore   BreakNoEffect, RestoreNotExact, BreakTransitionInvalid,
          RestoreTransitionInvalid
trace     EmptyTraceId, UnknownGrammarLane, StepIndexMismatch,
          EditStartAfterEnd, EditOutOfRange, EditNotCharBoundary,
          StepPostHashMismatch
```

The vocabulary is closed, and every code is reachable from *some* input.
Which ones the committed tests currently pin is a separate, smaller set:
`PayloadIdMismatch`, `InsertedDigestMismatch`, `PreSourceHashMismatch`,
`TransitionInvalid`, `MissingTransitionAssertion`, `RestorePreIsNotBrokenState`,
`RestoreNotExact`, `BreakNoEffect`, `EditStartAfterEnd`, `EditOutOfRange`,
`EditNotCharBoundary`, `StepIndexMismatch`, `StepPostHashMismatch` and
`UnknownGrammarLane` each have a reaching case in `semantics/tests/`. The
remaining codes are reachable in principle but not yet pinned by a test, and
that is recorded here rather than implied away.

A code that no input can produce is drift and is removed rather than kept as
decoration — which is why the earlier `StepPreHashMismatch` is gone: the
step loop already rejects a pre-hash mismatch by construction, so no input
could reach it.

## 8. Scope boundary

```text
- no timing, no mechanism facts, no reuse/allocation counters anywhere in a
  payload record or report;
- no representative/extremal/full-document membership decision is made here;
  `memberships` records what selection decided elsewhere (#35 A4);
- trace forms `local_burst` / `document_session` are structurally supported
  and replay-validated, but long-session performance claims remain DEFERRED
  (`protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` §1): CORRECTIVE-A
  proves the lifecycle, not a longitudinal result;
- no render-ready, whole-project, or resource claim is implied by any payload.
```
