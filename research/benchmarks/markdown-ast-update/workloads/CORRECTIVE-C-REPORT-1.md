# CORRECTIVE-C-REPORT-1 — real Markdown workload payload freeze (#35, PR #39)

Status: **WORKLOAD CONSTRUCTION COMPLETE — core performance freeze blocked on horse correctness.**
This report is the final Stage-A record of MARKIT-WORKLOAD-CORRECTIVE-C.
The machine artifacts it cites live under `workloads/payloads/` and are
digest-bound by `workloads/payloads/freeze-receipt-v1.json`.

```text
BASE_SHA   3a7d24e1ecffbe3f21c8091ed6dcdb6a5fa22473  (settled master, PR #38)
REPORT_GENERATION_SHA   b4ad4b3fa3320e53999b2643d6b66ced6673f311
BRANCH     research/35-corrective-c-payload-freeze-1
PR         jnhu76/markit#39
WORKTREE   clean at report generation time
```

## 0. What this corrective claims — and what it does not

It completes the **workload construction** for the primary G0 campaign:
sources, anchors, edits, transitions, payload identities, and the
correctness evidence for all of them. It does **not** claim any
performance fact: no H0–H4 timing, allocation, reuse counter, or scaling
workload exists anywhere in this pipeline (checked: G8 below). It does
not start #31, does not change the 3970-file acquisition universe, and
does not replace any representative/extremal/full-document member.

The workload itself is frozen by this corrective. The correctness-only A8
dry-run exposed H2/H3/H4 wrong-result cases on the frozen G0 workload.
Those are horse implementation correctness failures under #22 authority,
not a reason to mutate the workload. Therefore primary performance remains
blocked until the same frozen cases pass the #22 correctness contract.

## 1. Authority chain (verified at start)

```text
master 3a7d24e (PR #38 settlement)          verified
source-lock.json 7884c61b…                  verified (acquire.py verify --full: PASS)
candidate manifest f8a9917b…                unchanged
selection identity CORRECTIVE-B-…-v1        unchanged (see repair §4)
membership counts at start                  REP 18 / EXT 12 / SYN 2 / FULL 7,
                                            39 memberships over 36 files
```

Acquisition re-verification at report time:
`VERIFY PASS: 20 sources, 3970 materialized files byte-verified
(66391919 bytes)`.

## 2. What was built (A6 + A7 + A8)

New crate `workload-freeze` (bin `mdbench-corrective-c`), deterministic:
two runs produce byte-identical artifacts (§6). It consumes ONLY the
frozen acquisition/selection state + sources and the frozen semantic
substrate (GRAMMAR-LANES-v1, REAL-MARKDOWN-PROFILER-v1,
TRANSITION-ORACLE-v1, PAYLOAD-LIFECYCLE-v1) and the EXISTING runner
correctness path. No new benchmark engine; no measurement.

### A6 — applicability matrix

`workloads/payloads/applicability-matrix-v1.jsonl` — **555 rows** = every
physical file × every BREAK-side registry transition (15 of the 23
entries; the 8 `exact_restore_leg` entries are reached through their
BREAK entries, never as independent cells) × requested positions
(EARLY/MIDDLE/LATE), each row an explicit closed-vocabulary status:
APPLICABLE / NO_TARGET_SYNTAX / GRAMMAR_INELIGIBLE /
POST_GRAMMAR_INELIGIBLE / NO_PROVABLE_TRANSITION / NO_VALID_REAL_ANCHOR
(no silent missing entries). Per-anchor rejections are recorded on every
non-applicable row (**415 cells** carry failure reasons;
`coverage-final-v1.json §failures`).

Breakdown of non-applicable cells: NO_TARGET_SYNTAX 214,
GRAMMAR_INELIGIBLE 165, NO_VALID_REAL_ANCHOR 26,
NO_PROVABLE_TRANSITION 8, POST_GRAMMAR_INELIGIBLE 2.

Set-level gate: every BREAK-side registry transition has ≥1 applicable
cell (restore legs are reached through their BREAK entries).

### A7 — manifests

- `full-read-manifest-v1.jsonl` — **37** FULL_READ records, 111 lane
  records (22 G0_STRICT_FULL_READ, 30 G1_SEMANTIC_FULL_READ,
  22 realism-only, 37 G2 deferred).
- `edit-write-manifest-v1.jsonl` — **449 payloads** (`rp1:` identities):
  329 BREAK (step 0) + 120 RESTORE (step 1); 209 single-step traces +
  120 BREAK/RESTORE pairs, each pair chain-validated with exact
  restoration (`S2 == S0` byte-equal; 120/120 reports in the workload).
- `trace-manifest-v1.jsonl` — chained-trace form with per-step edits and
  declared-step digests.
- Positions: EARLY 140 / MIDDLE 140 / LATE 140 valid resolutions; 108
  requested positions had no applicable anchor (recorded); 152
  position resolutions collapsed onto a shared anchor and are
  deduplicated, never double-counted.

### Family coverage (payload attributions; a payload can serve two kinds)

| family | payloads | files | projects |
|---|---|---|---|
| E1_LOCAL_TEXT | 51 | 18 | 9 |
| E2_PARAGRAPH_SPLIT_MERGE | 83 | 19 | 10 |
| E3_CONTAINER_DEPTH | 50 | 10 | 6 |
| E4_FENCE_OPEN_CLOSE | 44 | 10 | 6 |
| E5_INLINE_DELIMITER | 102 | 17 | 9 |
| E6_REFERENCE_DEFINITION | 20 | 8 | 4 |
| ATX_HEADING_TOGGLE | 12 | 4 | 2 |
| TABLE_SEMANTIC (G1) | 87 | 7 | 6 (SEMANTIC_ONLY, never horse-dispatched) |

### G0 syntax coverage — all 12 core kinds ACTIVE_EDIT_COVERED

paragraph, text, heading_atx (required: ATX active coverage present — 12
payloads), list, list_item, block_quote, code_block_fenced, emphasis,
code_span, link_inline, link_reference, reference_definition, plus the
blank-line block boundary. Two kinds are covered THROUGH a paired
construct's count-flip and the report says so explicitly:
`list` via G0-LIST-ITEM-INDENT (List node-count flip),
`link_reference` via G0-REFDEF-REMOVE (resolved→unresolved use flip).

Every observed non-core syntax carries an explicit status
(`coverage-final-v1.json §extensions/§other_observed`): G1 table
ACTIVE (semantic-only), G2 math LANE_DEFERRED with zero executable
payloads, seven G1-declared kinds PARSE_COVERAGE_ONLY, four
GRAMMAR_EXTENSION_REQUIRED (each with recorded candidate evidence), four
OUT_OF_SCOPE_WITH_REASON. **No observed syntax silently disappears.**

## 3. TRANSITION-REGISTRY-v1

23 entries (18 G0 incl. 7 restore legs; 5 G1 table), frozen in
`workloads/payloads/transition-registry-v1.json` and documented in
`workloads/TRANSITION-REGISTRY-v1.md`. Every payload agrees with its
entry (fields + floor containment) at generation AND verify time; the
E1–E6 headline families are all covered. Three constructor/registry
defects were found by the matrix and fixed BEFORE the freeze (they are
part of this PR's history, not hidden):

1. G1 header-pipe removal selected the trailing boundary pipe (removal
   left a valid table) → now selects a cell-separator pipe only;
2. list indent lacked the sibling-adjacency precondition G0 §7 requires
   for nesting → anchors now require an adjacent same-indent sibling;
3. a destination byte replacement is invisible to PREDICATE-v1 → the
   registry's destination edit is the G0 §9.2 break form
   (G0-LINK-DEST-BREAK), renamed accordingly.

Plus one infrastructure fix found by testing: the exact RESTORE inverse
now covers replace-edits (previously it deleted the inserted byte without
re-inserting the replaced one — byte-exactness restored).

## 4. SYNTAX_COVERAGE_SET deterministic repair (the only selection change)

The A6 matrix found exactly one required transition with no applicable
cell anywhere: **G0-BQ-NEST-LINE** (E3_CONTAINER_DEPTH). Root cause: no
G0-strict-eligible selected file contains a real blockquote anchor.

Under the Stage-A §8.2 stop rule (issue #35 comment) and the
CORRECTIVE-C authorization (only SYNTAX_COVERAGE_SET may change, via the
deterministic repair), the repair:

- evaluated all **28** G0-strict-clean universe candidates with real
  blockquote anchors, from the committed CORRECTIVE-B profile artifact
  (`candidate-rows-v1.jsonl`) — no re-profiling, no universe change;
- applied the frozen tie-breaks (cell coverage → rarity → project/domain
  diversity → lexical key) →
  `opentelemetry-spec/files/specification/library-layout.md`
  (2944 bytes, 5 blockquote anchors, sha256 verified against the
  universe evidence);
- is recorded fail-closed in
  `workloads/selections/syntax-coverage-repair-v1.json` (identity-bound
  to the frozen selection; enforced fields: every selection-identity
  digest, the selected key's universe sha256, already-selected
  rejection, and `membership_added == "syntax_coverage"` so the repair
  cannot silently re-label the added file into another logical set;
  any drift is a hard error; tamper-tested 6 ways in §7).

Result: **37 physical files, 40 logical memberships**
(REP 18 / EXT 12 / SYN 3 / FULL 7), 15 projects. No other set changed;
no selected file was replaced. BQ-NEST-LINE is now covered (3 G0 pairs +
restore legs; block_quote ACTIVE_EDIT_COVERED).

## 5. A8 — correctness-only harness dry-run

`dry-run` reads the FROZEN manifests from disk (self-sufficiency proof),
reconstructs every pre state, re-validates every payload, derives
horse-independent CaseIds via CaseKeyV1, and dispatches through the
EXISTING runner correctness-only lane (no clock, no counters):

```text
FULL_READ   22/22 G0-strict files pass the NORMALIZED-RESULT-v1 gate
EDIT_WRITE  362 G0 cases x 5 horses = 1810 dispatches (87 G1 payloads skipped:
            semantic-only, NOT_HORSE_QUALIFIED — boundary G6/G8)
H0          362 pass / 0 wrong
H1          362 pass / 0 wrong
H2          351 pass / 11 wrong_result
H3          346 pass / 16 wrong_result
H4          354 pass /  8 wrong_result
```

**Finding:** all 35 wrong results are H2/H3/H4 reuse mechanisms on
G0-FENCE-CLOSER-REMOVE/RESTORE and G0-REFDEF-RESTORE cases.
The frozen correctness authority remains:

```text
normalize(Hx update result)
==
normalize(H0 clean full parse(post-edit source))
```

Therefore these rows are **correctness failures**, not timing/performance
weaknesses. They must not enter strict H0-H4 performance comparison while
wrong. Root-cause and repair ownership moves back to #22, which owns horse
correctness/parity. The exact frozen workload cases are retained unchanged
for that closure. `workloads/payloads/dry-run-cases-v1.jsonl` carries every
per-case row.

Interpretation boundary:

```text
workload/payload construction       PASS
horse implementation correctness   H0/H1 PASS; H2/H3/H4 FAIL
primary H0-H4 performance start    BLOCKED
```

A8 succeeded as a workload-validation stage precisely because it exposed
these failures. The workload must not be edited, deleted, or reweighted to
make the horses pass.

## 6. Verification evidence (§59)

| check | result |
|---|---|
| `cargo test --workspace` | all green (incl. 4 new constructor/restore regression tests) |
| `cargo test -p markit-mdbench-semantics` | all green |
| `tools/acquire.py verify --full` | PASS (3970 files byte-verified) |
| `generate` | OK — 37 files, 555 rows, 37 FULL_READ, 449 payloads, 120 pairs |
| `verify` (re-validate payloads + registry agreement + FULL_READ identity + receipt digests) | OK |
| `determinism` (two full runs, byte-compare) | OK — 7 artifacts byte-identical |
| negative tests (§53) `scripts/corrective-c-negative-tests.sh` | 6/6 fail closed (source byte, payload coordinate, receipt digest, repair identity, repair candidate hash, repair membership relabel) |
| no benchmark timing anywhere in the pipeline | asserted by construction (correctness-only runner lane; no clock exists on the path) |

## 7. G1–G8 workload audit + horse qualification boundary

The #35 Stage-A workload gates themselves are closed:

- **G1 SOURCE_AUTHORITY** — every artifact binds to sha256-verified source
  bytes; tamper → fail closed. PASS.
- **G2 GRAMMAR_ELIGIBILITY** — every payload carries grammar_id + lane
  qualification; G1 payloads are semantic-only and G2 has zero executable
  payloads. PASS.
- **G3 PROFILER_CONTRACT** — frozen profiler/span/host-context authority used
  for anchors and coverage. PASS.
- **G4 SELECTION_VALIDITY** — CORRECTIVE-B selection preserved; the single
  authorized syntax-coverage repair is deterministic and receipt-bound. PASS.
- **G5 SYNTAX_TRANSITION_COVERAGE** — every observed syntax has an explicit
  status and every active transition is oracle-proven. PASS.
- **G6 PAYLOAD_LIFECYCLE** — RESTORE starts from S1, exact restoration is
  byte-enforced (120/120), chained traces carry declared-step digests. PASS.
- **G7 HARNESS_ADAPTER** — Source/CanonicalEdit/CaseId/runner/oracle are the
  existing frozen contracts; the new path is correctness-only. PASS.
- **G8 CLAIM_BOUNDARY** — no timing/allocation/reuse/scale fact is produced;
  G1/G2 and REAL/SCALE boundaries remain explicit. PASS.

Independent of those workload gates, R6 §3 requires horse implementation
qualification for every horse entering a strict comparison lane. Current
G0 qualification on the frozen real workload is:

```text
H0  PASS  362/362
H1  PASS  362/362
H2  FAIL  351/362  (11 wrong_result)
H3  FAIL  346/362  (16 wrong_result)
H4  FAIL  354/362  ( 8 wrong_result)
```

This implementation-qualification failure blocks
`CORE_REAL_WORKLOAD_FREEZE_PASS` as an authorization to start the strict
five-horse performance campaign, even though workload construction itself
is complete.

## 8. REAL_WORKLOAD_FREEZE_PASS (broader lanes): NOT_GRANTED

Deferred-lane reasons, explicitly:

- **G1 beyond the table pilot** — the G1 lane qualifies ONLY GFM tables
  (pulldown-cmark 0.13.4 ENABLE_TABLES); no frozen G1 semantics exist
  for setext/indented code/strong/autolinks/html, so no executable G1
  payloads may be frozen for them.
- **G2 math** — MARKIT-EXT-MATH-v1 is deferred by authority: identity
  reserved, no semantics, no oracle, zero payloads (by design, not by
  omission).
- Broader lanes become eligible only through a reviewed grammar-extension
  decision; this corrective intentionally does not make one.

## 9. Residual risks / next ownership

1. The §58 adversarial review is recorded as
   `workloads/REVIEW-CORRECTIVE-C-ADVERSARIAL-v1.md`; all construction-side
   blocking findings were fixed in this PR.
2. Workload membership, anchors, edits, payload ids and transition registry
   are now frozen for the primary G0 campaign. Do not mutate them to address
   horse failures.
3. H2/H3/H4 wrong results are not root-caused here. They transfer to #22 as
   correctness-closure work. Their exact frozen case rows are the regression
   authority.
4. #31 remains blocked until the same frozen 362 G0 EDIT_WRITE cases pass for
   all H0-H4 participants.
5. G1 non-table semantics and G2 math remain explicit deferred lanes and do
   not keep #35 primary workload construction open.

## 10. Verdicts

```text
CORRECTIVE_C_PASS                         = YES
#35 PRIMARY WORKLOAD CONSTRUCTION         = COMPLETE
WORKLOAD MEMBERSHIP / PAYLOAD IDENTITY    = FROZEN — DO NOT CHANGE

CORE_REAL_WORKLOAD_FREEZE_PASS             = BLOCKED_ON_HORSE_CORRECTNESS
  H0                                       = PASS 362/362
  H1                                       = PASS 362/362
  H2                                       = FAIL 351/362 (11 wrong)
  H3                                       = FAIL 346/362 (16 wrong)
  H4                                       = FAIL 354/362 (8 wrong)

PERFORMANCE_START                          = BLOCKED
REAL_WORKLOAD_FREEZE_PASS                  = NOT_GRANTED
  reason                                   = broader G1/G2 lanes deferred

NEXT                                       = #22 REAL-WORKLOAD CORRECTNESS CLOSURE
```

**READY_TO_MERGE_AS_WORKLOAD_CONSTRUCTION_COMPLETION.**

This merge does not authorize #31 timing. Once #22 makes the frozen G0
correctness matrix all-green, `CORE_REAL_WORKLOAD_FREEZE_PASS` can be
recorded without changing the workload.
