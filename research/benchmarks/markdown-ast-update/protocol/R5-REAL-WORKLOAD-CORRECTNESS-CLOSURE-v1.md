# R5 Real-Workload Correctness Closure v1 — H2/H3/H4 on the frozen G0 workload

Status: **REAL_WORKLOAD_CORRECTNESS_CLOSURE_PASS** (self-assessment at the
verified head, 2026-09-21). Human review owns
`CORE_REAL_WORKLOAD_FREEZE_PASS`.
Campaign: #22 MARKIT-MARKDOWN-BENCHMARK-1
Task: MARKIT-22-REAL-WORKLOAD-CORRECTNESS-CLOSURE-1
Branch: `research/22-real-workload-correctness-1`
BASE_SHA: `83d535ecce5b5e66552aea047de7461f834e7366` (master; PR #39 merged)
HEAD_SHA: `890f63f146cb6a9e1f7e8b68dcee136dc1aab653` (verified head — every
number in this report was produced at this commit)
PR: #40 (Draft; this report was added in the commit immediately after the
verified head and changes no mechanism, oracle, workload or test code.
An earlier verified head `c94fdaa` carried the same mechanisms; the only
difference is the H4 accounting correction recorded in §11 CONCERN 3.)

Authority chain: issue #22 -> R0 -> R1 -> R3 freeze -> R4 H0 -> R5 (mechanism
identity) -> R6 real-workload authority -> CORRECTIVE-A/B/C freeze (PRs #37,
#38, #39) -> this closure. This record repairs **mechanism correctness**; it
does not redefine BENCH-GRAMMAR-v1, NORMALIZED-RESULT-v1, the frozen workload,
the transition registry, the mechanism identities or any counter semantics.
The invariant correction itself is recorded in
`protocol/R5-HORSE-CORRECTNESS-PARITY.md` §12 (R5-CORRECTIVE-3).

```text
frozen workload artifacts changed?  NO
formal timing run?                  NO
```

---

## 1. Starting matrix reproduced before any change (no drift)

At `master` @ `83d535e` (PR #39 merged), with the frozen G0 primary
EDIT_WRITE workload:

| Horse | Result | Wrong (`wrong_result`) | Execution failures |
|---|---|---|---|
| H0 FULL_REBUILD | 362/362 | 0 | 0 |
| H1 BLOCK_LOCAL_REPARSE | 362/362 | 0 | 0 |
| H2 FRAGMENT_REUSE | **351/362** | **11** | 0 |
| H3 OLD_TREE_SUBTREE_REUSE | **346/362** | **16** | 0 |
| H4 RESTART_CONVERGENCE | **354/362** | **8** | 0 |

FULL_READ: 22/22. Total dispatches 1810, of which 35 wrong.

The matrix reproduced exactly as stated in the task contract, and a re-run of
the dry-run on the unmodified tree reproduced the derived evidence files
byte-for-byte. **No drift.** Nothing was changed on the strength of a
mismatch.

The 35 wrong dispatches are **not** distributed as three tidy root causes;
they are 35 rows over 16 distinct payload ids and 4 source ids
(`oci-image` 16, `rust-rfcs` 11, `myst-parser` 6, `rust-book` 2), so each was
classified on its own evidence rather than by family label.

## 2. Failure classification (task contract §7 isolation)

A/B/C/D isolation was run per row with the same frozen `Mechanism` phases the
runner uses (new `diagnostics` crate, `mdbench-diverge`):

```text
A  H0 clean full_parse(post)                  oracle authority
B  Hx clean full_parse(post)                  horse's own parser
C  Hx clean full_parse(pre)                   horse's own parser, old source
D  Hx update(pre_state, edit)                 the dispatch under test
```

```text
UPDATE_WRONG:D_update        35   (every single row)
FULL_PARSE_WRONG              0
OLD_STATE_WRONG               0
HARNESS_ADAPTER_SUSPECT       0
ORACLE_AUTHORITY_CONFLICT     0
```

Every horse parses its own post source correctly (B == A) and its old source
correctly (C). The defect is therefore entirely in **reuse validity** — which
retained material may be carried across the edit — not in a horse's parser,
not in the harness, and not in the oracle. No authority conflict arose, so
this task is not `AUTHORITY_CONFLICT_BLOCKED`.

## 3. BEFORE — the 35 wrong dispatches

| Horse | Transition family | Count | Direction of the wrong result |
|---|---|---|---|
| H2 | `G0-FENCE-CLOSER-REMOVE` | 4 | stale `ReferenceLink` kept where the post source no longer resolves |
| H2 | `G0-FENCE-CLOSER-RESTORE` | 4 | stale `Text` kept where the post source now resolves |
| H2 | `G0-REFDEF-RESTORE` | 3 | stale `Text` kept where the post source now resolves |
| H3 | `G0-FENCE-CLOSER-REMOVE` | 4 | as H2 |
| H3 | `G0-FENCE-CLOSER-RESTORE` | 4 | as H2 |
| H3 | `G0-REFDEF-RESTORE` | 8 | as H2 |
| H4 | `G0-FENCE-CLOSER-REMOVE` | 4 | as H2 |
| H4 | `G0-FENCE-CLOSER-RESTORE` | 4 | as H2 |

Per family the first normalized divergence is the same shape for all three
horses, e.g.

```text
G0-FENCE-CLOSER-REMOVE   path=1/0 field=kind
  expected=Text(16..199)                 (H0 clean parse of post)
  actual=ReferenceLink(16..40) destination="https://github.com/.../badge.svg?branch=master"
G0-FENCE-CLOSER-RESTORE  path=1/0 field=kind
  expected=ReferenceLink(16..40 destination=...)   actual=Text(16..199)
G0-REFDEF-RESTORE        path=1/0/1/0 field=kind
  expected=ReferenceLink(.. destination="https://www.docker.com/" label="docker")
  actual=Text(..)
```

## 4. Root cause

One semantic class, two independent holes in the same clause.

The R5 SEMANTIC-DEPENDENCY HANDLING clause (§7, inherited by §8 and §9)
asserted that a reference-bearing subtree may be reused when no *definition*
changed, because "every definition in the document lives in unchanged bytes".
That is **false in both directions**:

1. **Unchanged definition bytes are not sufficient.** Whether a line is a
   definition at all is decided by the forward parse state at that line
   (container stack + fence state). Removing a fence closer makes the fence
   run to EOF, so definition-looking lines *become fence content*: every
   definition byte is unchanged, yet definitions leave the table. The edit's
   own three bytes (`` ``` ``) carry no definition marker, so no source-local
   probe over the edit span can see it.
2. **Definitions entering the environment are invisible to `has_ref`.** A
   retained payload whose reference did not resolve holds no `ReferenceLink`
   at all, so nothing inside it can witness that it must now resolve
   (`G0-FENCE-CLOSER-RESTORE`, `G0-REFDEF-RESTORE`).

H4 additionally declared a definition-environment *generation* in its model
but never actually checked it: the generation counter was never advanced, so
the declared clause was inert.

## 5. Root-cause table

| Horse | Failure class | Violated invariant | Evidence | Fix | Mechanism-fidelity justification | Regression test |
|---|---|---|---|---|---|---|
| H2 | `UPDATE_WRONG:D_update` (11 rows) | R5 §7 SEMANTIC-DEPENDENCY HANDLING: reuse validity of a reference-bearing payload was conditioned on unchanged definition bytes / `has_ref` | 11 frozen dispatches; first divergence `expected=Text actual=ReferenceLink` (and the mirror); `has_ref` false on the restore direction | Re-materialize **only** reference-sensitive retained payloads against the mechanism's own rebuilt table (`mechanisms/fragment-reuse/src/lib.rs:443` compare, `:1034` `mentions_reference`, `:1055` `rematerialize`, `:1287` conditional arm) | The clause is a reuse-validity condition, not a new mechanism. Block structure, safe windows, fragment table, MIN_GAP and Arc identity of reference-insensitive descendants are untouched; reuse authority is still the fragment table only; there is still no fallback slot (`fallback_to_full_count` stays `NotApplicable`). Cost accounting moves the member from `nodes_reused` to `nodes_rebuilt` without double counting (`:468`) | `mechanisms/fragment-reuse/tests/reference_environment.rs` (5 tests; 3 fail on the pre-fix code) |
| H3 | `UPDATE_WRONG:D_update` (16 rows) | same clause, §8 wording | 16 frozen dispatches; identical divergence shapes | Same repair on the patched old tree: `TPayload` re-scan against the mechanism's own table (`mechanisms/old-tree-subtree-reuse/src/lib.rs:413` compare, `:1099` `mentions_reference`, `:1121` `rematerialize`, `:1345` conditional arm) | `changed` flags, patch path, cursor and tree shape unchanged; only the retained payload's reference resolution is refreshed; accounting `:438` | `mechanisms/old-tree-subtree-reuse/tests/reference_environment.rs` (5 tests; 3 fail on the pre-fix code) |
| H4 | `UPDATE_WRONG:D_update` (8 rows) | R5 §9: the declared definition-environment generation was never checked; detection rested on a source-local `]: ` probe | 8 frozen dispatches; all fence-closer family (the probe is blind to forward-state effects) | Keep the probe as a fast path and add the sound condition — compare the **assembled** table (retained prefix facts + fresh region facts + retained suffix facts, complete before any materialization) against the retained one; on a difference take the restart-at-zero §9 already declares for definition-changing damage (`mechanisms/restart-convergence/src/lib.rs:548`, fast path `:453`, `restart_at_zero` `:270`) | Uses H4's own existing generation/restart machinery; not a fallback (H4 has no degraded mode); fires only when the environment actually changed; the delivered counters are the restart's (`nodes_reused = 0` at the restart gauges), and the discarded forward pass's own work is still reported — its source inspections through the sink and its metadata work (`consultations + slot_count`) in the branch itself | `mechanisms/restart-convergence/tests/reference_environment.rs` (5 tests; 2 fail on the pre-fix code) |

No horse was collapsed into H0: `update()` still reuses retained material on
every safe case, H2/H3 still have no full-rebuild path at all
(`fallback_to_full_count` is `NotApplicable` for both), and H4's restart is
its own frozen response, not a delegated H0 call. No mechanism consults the
oracle, H0, a payload id, a transition id, a source path or workload
membership; no mechanism imports `workload-freeze`, the transition registry or
the payload manifest (`diagnostics/tests/anti_cheat.rs`, 4 structural tests).

## 6. Regression hierarchy (task contract §16)

```text
Level 1  mechanism fixtures — mechanisms/*/tests/reference_environment.rs
         5 tests each: fence-closer REMOVE, fence-closer RESTORE, refdef
         RESTORE, refdef REMOVE, and a fidelity witness that safe local
         edits still reuse (H2/H3) / converge (H4). Documents are built by
         byte surgery so each pair differs by exactly the edited bytes.
         VERIFIED FAILING ON THE PRE-FIX CODE, in an isolated copy with
         the three mechanism sources checked out from BASE_SHA:
             H2  3 failed / 2 passed  (both fence directions + refdef RESTORE)
             H3  3 failed / 2 passed  (both fence directions + refdef RESTORE)
             H4  2 failed / 3 passed  (both fence directions)
         The pre-fix failures mirror the frozen failure distribution per
         horse exactly: H4 has no refdef-RESTORE fixture failure, and H4
         had no G0-REFDEF-RESTORE failure in the frozen matrix either
         (its `]: ` probe does catch an edit that carries the marker).
         All 15 pass after the fix.
Level 2  diagnostics/tests/frozen_correctness_closure.rs — the 35 pinned
         (horse, payload_id) pairs must now isolate as Pass; the set of
         transitions that flipped must be exactly
         {G0-FENCE-CLOSER-REMOVE, G0-FENCE-CLOSER-RESTORE,
         G0-REFDEF-RESTORE}; three pinned safe frozen cases must still show
         reuse (H2/H3 reused>0) and convergence (H4 reused>0 and
         convergence before EOF).
Level 3  the same file — the complete frozen matrix: 22 FULL_READ records
         and 362 cases x H0-H4 = 1810 dispatches, each horse
         (362 pass, 0 wrong, 0 execution failed); 1810 + 22 rows.
```

Synthetic counterexamples added here are **correctness regression fixtures
only**; none of them became a #35 primary payload, and no workload
membership, anchor, canonical edit or payload id was added or changed.

## 7. AFTER

| Horse | Pass | Wrong | Execution failures |
|---|---|---|---|
| H0 FULL_REBUILD | 362/362 | 0 | 0 |
| H1 BLOCK_LOCAL_REPARSE | 362/362 | 0 | 0 |
| H2 FRAGMENT_REUSE | **362/362** | **0** | 0 |
| H3 OLD_TREE_SUBTREE_REUSE | **362/362** | **0** | 0 |
| H4 RESTART_CONVERGENCE | **362/362** | **0** | 0 |

FULL_READ 22/22. `DRY_RUN_OK correctness-only; no timing was performed`.

Independent verification at the verified head:

```text
cargo test --workspace --release                 70 binaries, 322 tests, 0 failed
mdbench-corrective-c verify .                    VERIFY_OK payloads=449 full_read_records=37 receipt_artifacts=6
mdbench-corrective-c determinism .               DETERMINISM_OK artifacts=7 byte-identical
scripts/corrective-c-negative-tests.sh           7/7 fail-closed paths held
harness dry-run                                  FULL_READ 22/22; EDIT_WRITE 362 x H0-H4, 0 wrong
```

## 8. Frozen artifact integrity gate (task contract §18)

82 tracked workload files were hashed before the first change and again at the
verified head: **80 unchanged, 2 changed, 0 missing**.

```text
CHANGED  workloads/payloads/dry-run-cases-v1.jsonl
CHANGED  workloads/payloads/dry-run-report-v1.json
```

Both are the **derived correctness evidence** regenerated by the harness
dry-run (the same files PR #39 introduced as correctness evidence, never as
measurement input). Neither appears in the freeze receipt's
`artifact_sha256` list, and every one of the six artifacts that does appear is
byte-identical to its receipt digest **and** to its pre-fix hash:

```text
OK transition-registry-v1.json     OK full-read-manifest-v1.jsonl
OK applicability-matrix-v1.jsonl   OK edit-write-manifest-v1.jsonl
OK trace-manifest-v1.jsonl         OK coverage-final-v1.json
ALL_RECEIPT_ARTIFACTS_BYTE_IDENTICAL_TO_FROZEN = true
```

The 37 real source files, the selection/membership artifacts, the edit
anchors, canonical edits, payload ids, `expected_pre` / `expected_post`, the
transition registry, EARLY/MIDDLE/LATE resolution and the frozen mechanism
identities are untouched.

The 37 effective real files (36 `selected-files-v1` members plus the one
deterministic SYNTAX_COVERAGE repair member) are not part of that 82-file hash
baseline, because `workloads/sources/` is a local materialization. Their
authority check is `mdbench-corrective-c verify`, which re-hashes every
materialized file against the frozen selection and repair hashes and fails
closed on any drift; it returned `VERIFY_OK` at the verified head. A single
changed source byte cannot pass unnoticed.

**The freeze receipt was not mutated**; the closure
is recorded as this separate versioned report plus the R5 §12 amendment, which
is the correct split — the receipt records workload *identity*, while a
correctness repair of the horses is *derived evidence* about that identity.
By the same rule the CORRECTIVE-C reports and the workload README still
describe the state at freeze time, including the pre-fix horse matrix; they
are frozen artifacts and were deliberately left byte-identical. This report is
the successor record for horse correctness, and it supersedes their horse
numbers without editing them.

## 9. Weakness-Map boundary (task contract §21)

The 35 rows were `CORRECTNESS_FAILURE` and are now closed as:

```text
CORRECTNESS_DEFECT: reuse validity conditioned on unchanged definition BYTES
                    rather than on the document-global definition ENVIRONMENT
                    (forward-parse-state dependence of reference resolution)
```

They are **not** recorded as `ATTRIBUTED_WEAKNESS`,
`PERFORMANCE_WEAKNESS`, `CROSS_MECHANISM_PATTERN` or `DESIGN_OPPORTUNITY`.
No performance property was measured, inferred or claimed from them, and no
weakness-map promotion is made.

## 10. No performance work

No criterion run, no wall-clock, no `T_prepare` / `T_native` / `T_total`, no
speedup, no throughput, no allocations-per-edit, no PA comparison, no
restart/convergence-distance performance plot. The only counters touched are
the correctness-fidelity facts the frozen counter semantics already define
(`nodes_reused`, `nodes_rebuilt`, `convergence_distance`), read inside tests
to prove the mechanisms still do their job and to keep accounting honest. The
new `diagnostics` crate runs no clock and no work-counter metric; the new
`oracle::divergence` comparator is a deterministic first-divergence locator
for diagnosis only, with no changed-node count, damage size or propagation
distance. #31's performance protocol is untouched and no timing was started.

## 11. Adversarial review (task contract §25)

A fresh-context adversarial reviewer with no prior exposure to this work
inspected the diff, the artifacts and the tests against twenty questions
covering workload movement, oracle/grammar integrity, deleted or skipped
payloads, label-based branching, mechanism imports of benchmark identity,
oracle consultation inside a mechanism, disguised collapse into H0, fallback
width, condition semantics, whether safe cases still exercise the mechanisms,
accounting honesty, whether the new tests fail pre-fix, comparator purity,
timing, weakness-map promotion, receipt mutation, artifact verification,
the §23 gates, BREAK and RESTORE, and scope. It re-ran the gates itself
(workspace suite, `verify`, `determinism`, the negative tests in a temp copy,
the harness dry-run into a temp root), rebuilt the pre-fix code in a worktree
to confirm the new tests fail there, and ran its own per-case counter census
over all 362 G0 cases at both base and head.

```text
ADVERSARIAL_REVIEW: PASS   (0 MAJOR; 3 CONCERN, all dispositioned below)
```

Q1–Q20 all returned OK. The findings worth recording:

```text
CONCERN 1  the closure report itself was still untracked when the review
           ran (the committed R5 §12.4 cited it). RESOLVED by this commit,
           which adds the report. No verified result was affected.
CONCERN 2  an early draft of this report overstated the pre-fix coverage of
           the H4 fixture file as "5 tests failing pre-fix". Corrected here
           to the measured 2 of 5 (H4's REFDEF-RESTORE fixture passes
           pre-fix because its pre-existing `]: ` probe does catch an edit
           that carries the marker). Wording only.
CONCERN 3  a real accounting asymmetry introduced by this corrective: on the
           new environment-change restart path, H4 reported the discarded
           forward pass's source inspections but not its metadata work
           (checkpoint consults and slot registrations), because the branch
           returned before the normal reporting site. FIXED in this commit
           (mechanisms/restart-convergence/src/lib.rs:548) at the verified
           head — the branch now reports `consultations + slot_count`
           before restarting. Hiding
           that work would have under-reported H4 on exactly the rows the
           clause fires on, which is the wrong direction for a campaign that
           will later measure these mechanisms.
```

The reviewer's independent census also corroborates the fidelity claims in
§5: at the fixed head H2 shows `nodes_reused > 0` on 356/362 cases, H3 on
362/362, and H4 keeps 290 converging cases with retained blocks; counter
deltas versus base appear in exactly the four definition-changing transitions
and in no ordinary family; and per-case
`Δ(nodes_reused + nodes_rebuilt) == Δ(tree node_count)` held on all 27 changed
rows, so the reassignment introduced here neither double-counts nor loses
work. It separately reproduced the frozen BEFORE from the BASE mechanism
sources against the head harness (H2 `(351, 11, 0)`), which is direct evidence
that the comparison authority did not move.

Because CONCERN 3 changed mechanism code after the review, the reviewer's
verdict applies to the pre-fix head; the one-line accounting change and its
effect are covered by the re-run gates in §7 and by the counter assertions in
the level-1/level-3 tests, and are recorded here rather than left implicit.

## 12. Verdict

```text
REAL_WORKLOAD_CORRECTNESS_CLOSURE_PASS
CORE_REAL_WORKLOAD_FREEZE_PASS = CANDIDATE_FOR_HUMAN_REVIEW
READY_FOR_HUMAN_CORRECTNESS_REVIEW
```

The frozen workload is evidence and was not moved: the horses were repaired
inside their own frozen models, against their own state, with the oracle and
the workload untouched. This closure does not merge automatically, does not
update #31 performance results, does not start timing, and does not design the
final Markit parser.
