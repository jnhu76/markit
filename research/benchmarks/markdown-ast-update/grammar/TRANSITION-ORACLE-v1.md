# TRANSITION-ORACLE-v1 — pre/post syntax-transition validation (CORRECTIVE-A)

Status: **CORRECTIVE-A SEMANTIC SUBSTRATE — PRE-MEASUREMENT, NO FREEZE GRANTED**
Authority: issue #35 Corrective-1 §5 (coverage becomes transition coverage) +
#35 Workload Construction Algorithm v1 + `protocol/
R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` §4. Executable form:
`semantics/src/transition.rs` (consts `TRANSITION_ORACLE_VERSION =
"TRANSITION-ORACLE-v1"`, `PREDICATE_VERSION = "PREDICATE-v1"`). Generated
schema: `workloads/payloads/schema/transition-oracle-v1.schema.json`.

---

## 0. The rule this contract enforces

```text
hash replayability  = payload reproducibility   (necessary)
oracle              = grammar correctness       (necessary)
transition assertion= proves the case tests the claimed state change
                        (necessary)
```

All three are necessary; none implies the others. An edit whose bytes are
perfectly reproducible, whose post hash is pinned, and which parses cleanly
may still fail to produce the state change the payload claims — for example
a "Table → non-Table" case whose edit actually leaves a valid table behind.
That payload is not active syntax coverage for that transition, no matter
how well it replays.

## 1. Frozen validation sequence

```text
1. lane lookup              grammar_id must name a registered lane
                            else: UnknownGrammarLane
2. predicate binding        every kind predicate bound to a grammar_id
                            must name THIS lane
                            else: PredicateGrammarMismatch
3. pre source               lane parse + profile
                            evaluate expected_pre predicates
                            record eligibility facts (lane_valid,
                            strict_scope_clean, scope_blockers)
4. post source              (absent + edit present: MissingPostSource)
5. edit reproduction        apply the declared canonical edit to the pre
                            source; the result must equal the declared post
                            source byte-for-byte
                            else: PostHashMismatch, or one of
                            EditStartAfterEnd / EditOutOfRange /
                            EditNotCharBoundary
6. post source              lane parse + profile
                            evaluate expected_post predicates
                            record post eligibility facts
7. no-op check              post source must differ from the pre source
                            else: NoSourceChange (an edit that reproduces
                            the pre bytes is not a transition)
8. exercised check          the declared assertion set must distinguish the
                            two states (see §1.1)
                            else: TransitionNotExercised
9. verdict                  any failure code -> InvalidPayload
                            no failure codes -> TransitionValid
10. eligibility             strict_comparison_eligible =
                            pre.lane_valid AND post.lane_valid AND
                            pre.strict_scope_clean AND post.strict_scope_clean
```

The report carries both `verdict` and `strict_comparison_eligible` because
they answer different questions: "did the claimed transition hold?" and
"may the two states be compared under this lane at all?". A valid
transition between two lane-out-of-scope states is not strict comparison
evidence.

## 2. PREDICATE-v1 (closed vocabulary)

Deliberately minimal. Adding a predicate is a versioned contract event
(`PREDICATE_VERSION`), never an ad-hoc extension at a call site.

```text
kind_present          { grammar_id, syntax_kind }
                      a recognized node of this kind exists
kind_absent           { grammar_id, syntax_kind }
                      no recognized node of this kind exists
kind_count            { grammar_id, syntax_kind, op, count }
                      recognized-node count satisfies `op count`
                      op ∈ { ==, !=, <, <=, >, >= }
topology_equal        cross-source: pre and post lane topology (kind
                      sequence, nesting, kind details; offsets ignored)
                      are identical. Evaluated on the post side only.
text_content_differs  cross-source: at least one text/code span differs
                      between pre and post. Evaluated on the post side only.
```

Two properties are frozen here:

- **Recognized counts only.** `kind_present`/`kind_absent`/`kind_count`
  count nodes the lane oracle *recognized*. Candidate evidence (a
  table-shaped line the lane declined, bytes inside a fence) never
  satisfies a kind predicate. "We saw bytes that look like a table" is not
  "the lane has a table".
- **Cross-source predicates are post-side.** `topology_equal` and
  `text_content_differs` compare the two sources and are therefore
  evaluated where both exist. On a pre-only request they are reported
  unsatisfied with `not evaluable on a pre source`.

Every predicate result records what was actually observed
(`recognized_count=2 expected == 1`, `topology_equal=false`, …), so a
failure is auditable without re-running the oracle.

## 3. Canonical edit reproduction

The edit is `[edit_start, edit_end)` in **UTF-8 bytes of the pre source**,
half-open, plus the inserted bytes verbatim (`EditSpec`). Validation applies
it through the frozen R0 canonical-edit path
(`markit_mdbench_common::CanonicalEdit`), so:

```text
- boundaries must be UTF-8 char boundaries of the pre source
- start <= end <= len(pre source)
- the derived post source must equal the declared post source byte-for-byte
```

Inserted bytes are carried verbatim and additionally digested by the payload
lifecycle (`workloads/payloads/PAYLOAD-LIFECYCLE-v1.md` §3). A digest alone
is never accepted as the edit's content.

## 4. Pre and post eligibility are independent

`strict_scope_clean` is computed from each source's own host-context facts.
It is never inherited, never assumed, and never relaxed because the edit was
"small". The pilot pins this with `p04-g0-post-ineligibility.toml`: a G0 edit
that introduces a host-context table leaves the post state out of G0 scope,
and the report says so while the transition itself is valid.

```text
pre in scope, post out of scope  -> valid transition, NOT eligible
                                    (recorded, not silently benchmarked)
```

### 1.1 What makes an assertion set non-vacuous

A non-empty assertion set is necessary but not sufficient. A set that holds
identically on both sides, with no satisfied cross-source predicate,
describes nothing the edit did: the case would pass on hashes and a label
alone, which is the hole the oracle exists to close.

```text
exercised  <=>  a kind predicate whose truth value DIFFERS between the two
                sides
             OR a cross-source predicate (topology_equal,
                text_content_differs) that is SATISFIED on the post side
```

A cross-source predicate is undefined on a pre source, so it cannot
"differ"; it counts by being satisfied. That is what makes a content-only
control a legitimate case rather than a vacuous one, and it is why the
`p03` fixture declares `text_content_differs` alongside `topology_equal`.

`semantics/tests/transitions.rs` pins both negative directions: a no-op edit
is rejected (`NoSourceChange`), and an edit that changes bytes while
asserting only that a paragraph is present on both sides is rejected
(`TransitionNotExercised`).

### 1.2 Kind predicates count out-of-band recognized facts

`kind_present` / `kind_absent` / `kind_count` count parse nodes **plus the
lane's out-of-band recognized facts** (G1 exposes reference definitions
outside its node tree). A count that walked nodes only would answer
`kind_absent` for a kind the same profile reports as `recognized` — reading
non-recognition back as absence, the inference
`workloads/profiles/PROFILER-CONTRACT-v1.md` §2.1 forbids. A fact a node
already covers is not counted twice.

## 5. Change classification (minimal, CORRECTIVE-A scope)

```text
CONTENT_CHANGE              topology unchanged, text/code content changed
STRUCTURE_CHANGE            topology changed, or a declared kind predicate's
                            truth value flipped between the sides
DEPENDENCY_RESOLUTION_CHANGE  RESERVED — not derivable in CORRECTIVE-A
POSITION_SHIFT_ONLY           RESERVED — not derivable in CORRECTIVE-A
```

`change_class` is `null` when neither condition holds: the topology is
unchanged, no declared kind predicate flipped, and the text/code fingerprint
is identical. That is a defined outcome, not a missing value — an edit that
changes only a link destination, for instance, is a real byte change the
PREDICATE-v1 vocabulary cannot classify, and the report says so instead of
guessing. Consumers must read `null` as "outside this vocabulary", never as
"no change".

`change_class` is derived only from the two `SourceOracleCheck` records
(topology digest + text fingerprint + declared kind predicates). It is
explicitly **not** semantic interference: every report carries
`change_class_note` stating that this is an observed effect under the frozen
lane rules and the PREDICATE-v1 vocabulary, not the theoretical minimal work
an incremental parser must perform, and not a horse work counter. The full
semantic-interference contract is deferred to the attribution stage
(`workloads/attribution/SEMANTIC-INTERFERENCE-CONTRACT-v1.md`); CORRECTIVE-A
must not implement it as a naive AST diff.

## 6. Failure codes (closed)

```text
UnknownGrammarLane          grammar_id names no registered lane
PredicateGrammarMismatch    a predicate is bound to a different lane
MissingPostSource           an edit was declared but no post source given
EditStartAfterEnd           edit_start > edit_end
EditOutOfRange              edit range exceeds the pre source
EditNotCharBoundary         a boundary splits a UTF-8 sequence
PostHashMismatch            applying the edit does not reproduce the post
NoSourceChange              the edit reproduces the pre source exactly
TransitionNotExercised      the assertion set holds identically on both
                            sides and has no satisfied cross-source
                            predicate
PrePredicateFailed          an expected_pre predicate is unsatisfied
PostPredicateFailed         an expected_post predicate is unsatisfied
PreLaneInvalid              the pre source is not lane-valid
PostLaneInvalid             the post source is not lane-valid
```

Verdict is `TransitionValid` exactly when the failure list is empty.

## 7. Worked pilot transitions

```text
F1  P01  G0  ATX heading -> paragraph      kind_absent(heading_atx) post
                                            kind_present(paragraph) post
        proves: block reinterpretation under the controlled lane
F2  P02  G1  Table -> non-Table (delimiter) kind_absent(table) post
                                            kind_present(paragraph) post
        plus RESTORE back to the exact original bytes
        proves: a small block-level edit flips grammar interpretation,
                and the flip is proven by the lane oracle, not by bytes
F3  P03  G1  content-only control           topology_equal post
                                            text_content_differs post
        proves: an edit that changes no structure is classified as a
                content change, and the chained trace steps stay in their
                own pre-source coordinates
```

Negative cases are pinned in `semantics/tests/transitions.rs`: a post source
that does not match the edit (`PostHashMismatch`), a non-char-boundary edit,
a predicate bound to another lane, and an unknown lane.

## 8. What this contract does not decide

```text
- no horse behavior is consulted or implied;
- no timing, no work counters, no reuse/allocation facts;
- no claim that a valid transition is "the" transition the workload needs:
  that is selection's decision (#35 A4.3), made against these predicates;
- no CommonMark/GFM completeness claim beyond the lane's qualified scope
  (G1 tables only, in CORRECTIVE-A).
```
