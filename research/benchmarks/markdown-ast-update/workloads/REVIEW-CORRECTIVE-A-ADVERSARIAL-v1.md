# CORRECTIVE-A adversarial review record (v1)

Status: **REVIEW RECORD — NOT AN AUTHORITY, NOT A FREEZE GRANT**
Scope: the CORRECTIVE-A semantic substrate on branch
`research/35-corrective-a-semantic-substrate-1`
(`semantics/`, `grammar/GRAMMAR-LANES-v1.md`, `grammar/TRANSITION-ORACLE-v1.md`,
`workloads/profiles/`, `workloads/payloads/`, `workloads/pilots/`).

This file records a **fresh-context adversarial review** performed against the
working tree before the Draft PR was opened, and what was done about each
finding. It is kept because a review whose findings are only visible in a PR
comment thread cannot be re-checked later.

---

## 0. Method

The reviewer was given no prior involvement in the work and was asked to
falsify its load-bearing claims rather than confirm them, working from the
tree alone. It read the code and contracts, ran the crate's own test suite,
and ran its own experiments from a scratch crate path-depending on
`semantics/` so that nothing in the repository was modified.

Nine questions were put to it:

```text
Q1  can non-recognition still be read back as absence?
Q2  is the host-context rule implemented as stated, and is the stated rule
    correct — can it give a wrong answer?
Q3  is a "clean / qualified / eligible" result gameable?
Q4  are semantic and horse qualification actually independent in the code?
Q5  is the transition oracle falsifiable — can a payload pass that does not
    perform the change it claims?
Q6  is payload identity deterministic and is the inserted-byte digest a
    substitute for the bytes?
Q7  does the BREAK/RESTORE SHA chain prove what it claims?
Q8  do chained-trace coordinates belong to the right source?
Q9  does the work stay inside its stated boundary?
```

Verdicts were: **Q4, Q8, Q9 SOUND**; **Q1, Q2, Q3, Q5, Q6, Q7 WEAK** —
i.e. the mechanism existed and had teeth, but each had at least one hole.
Every WEAK verdict was reproduced with a concrete input before being acted on,
and every fix below is pinned by a test that fails without it.

---

## 1. Findings and resolutions

### 1.1 MAJOR — a candidate anchored on a literal construct's own delimiter escaped the host-context rule

The candidate rule resolved candidates against the literal construct's
*content* interval, but a lexical probe can anchor on the delimiters. On
`"```\n=====\nTitle\n```\n"` the setext probe's candidate starts at the fence
opener, so it was not contained by the content interval and was reported as a
host-context scope blocker — making a document that is nothing but a fenced
code block ineligible for the lane that owns fences. The same happened to a
table-shaped fence and to table-shaped bytes in an indented code block.

**Fixed.** `LaneParse` now carries `literal_spans` (each construct's *full*
span, widened to the start of its first line, since an oracle reports an
indented block from its content and its indentation is part of the construct).
Candidates resolve against that list; nodes keep the content-interval rule.
Pinned by `semantics/tests/profiler.rs::
a_candidate_anchored_on_a_fence_opener_is_still_literal_content`, which also
asserts the owning fence node stays host-context.

### 1.2 MAJOR — the committed pilot artifact embedded an absolute machine path

`semantic-pilot-results-v1.json` recorded
`/home/hoo/Projects/markit/…/config.md (materialized)`, so the artifact
differed in any other checkout and the drift guard
(`committed_pilot_artifacts_match_the_driver`) could only pass on the author's
machine. This contradicted the pilot README's own "no machine facts, no paths"
claim.

**Fixed.** The detail string is built from the repository-relative `spec.path`.
Pinned by `semantics/tests/contracts.rs::published_artifacts_carry_no_machine_paths`.

### 1.3 MAJOR — G0 panicked on the empty source

`parse_g0("")` panicked in `validate_normalized(...).expect(...)` because
NORMALIZED-RESULT-v1 rejects zero-length node spans and the empty document is
exactly one such span. Reachable from `profile_g0("")` and from any G0
transition whose post source is empty, and it contradicted the lane's
"total over UTF-8" claim.

**Fixed.** The G0 lane skips reference-parse validation for the empty source
only; the frozen validator is not modified. Pinned by
`semantics/tests/profiler.rs::the_empty_source_profiles_under_both_lanes`.

### 1.4 MAJOR — `kind_absent` reported absence for a construct the same profile reports as recognized

Kind predicates counted `LaneParse` nodes only. G1 exposes reference
definitions as *out-of-band* recognized facts, so for
`"[a]: http://example.com\n\n[a]\n"` the profile listed a recognized
`reference_definition` while `kind_absent(reference_definition)` returned
satisfied. This is precisely the "non-recognition read back as absence"
inference the profiler contract forbids, in the one place it must not happen.

**Fixed.** `recognized_count` now counts nodes **plus** out-of-band recognized
facts, without double-counting a fact a node already covers. Pinned by
`semantics/tests/transitions.rs::kind_predicates_see_out_of_band_recognized_facts`.

### 1.5 MAJOR — payload identity and validity did not bind the asserted transition

Three related holes:

```text
a no-op edit with trivially-true assertions was TransitionValid
two records differing only in expected_post shared one payload_id
expected_transition was unvalidated prose
```

**Fixed** in two parts. (a) The oracle now rejects an edit that reproduces the
pre source (`NoSourceChange`) and an assertion set that holds identically on
both sides with no satisfied cross-source predicate
(`TransitionNotExercised`) — see `grammar/TRANSITION-ORACLE-v1.md` §1.1.
(b) `expected_transition`, `expected_pre` and `expected_post` are identity
fields, so two payloads asserting different things can no longer collapse onto
one id. Pinned by `semantics/tests/transitions.rs::a_no_op_edit_is_rejected`
and `::a_vacuous_assertion_set_is_rejected`.

`expected_transition` remains a label rather than a machine-checked claim: it
is now *part of* the identity and sits beside a checked assertion set, but
nothing validates the string against the predicates. Closing that would need a
frozen mapping from transition names to predicate sets, which is CORRECTIVE-C
work (see §2).

### 1.6 MINOR — a BREAK/RESTORE pair that neither broke nor restored validated

A no-op break edit plus a restore that only edited a cell validated with
`restore_exact = false`. The report was honest but nothing required the break
to have had an effect.

**Fixed.** `BreakNoEffect` rejects a break whose result equals the base source.
Pinned by `semantics/tests/lifecycle.rs::a_break_that_changes_nothing_is_rejected`.

### 1.7 MINOR — abstention was not a named state

With the PR #34 bytes absent, `real_source.pass` was `true` and the checks were
`(ABSTAIN, false, gating: false)`. The encoding was machine-readable but
unnamed, so a consumer reading `pass` alone could read a pass for a source
nothing was evaluated about.

**Fixed.** `real_source.status` is `verified` or `abstained_absent_bytes`, and
the pilot README says plainly that `pass` is a gating verdict.

### 1.8 MINOR — the real-source case is recorded, not asserted

`PROFILER-CONTRACT-v1.md` claimed the real file's "profile is compared against
the declared lane rules"; the driver checks identity (availability, sha256,
UTF-8) only, and `real-source-v1.json` declares no expectations. Changing a
lane rule would move the recorded numbers without failing the pilot.

**Not a code fix — a scope statement.** Asserting a real file's construct
counts is a selection-stage question and this corrective deliberately does not
answer it. The contract and the pilot README now say so, and the artifact
reports `verified` rather than implying the numbers were checked.

### 1.9 MINOR — reachability claims were overstated

Two sentences claimed every failure code / fact reason is reachable from a
declared fixture or test. Measured against the committed tests, several codes
are reachable in principle but unpinned, and one reason
(`interprets_as_text_under_lane`) is unexercised by construction.

**Fixed by telling the truth.** `PAYLOAD-LIFECYCLE-v1.md` §7 now names the
subset the tests actually pin, and the profiler contract records
`interprets_as_text_under_lane` as currently unexercised and why. No vocabulary
was deleted to make the sentence true.

### 1.10 MINOR — `change_class` can be `null` and was undocumented

An edit that changes only a link destination yields `change_class = null`.
`TRANSITION-ORACLE-v1.md` §5 documented only the four named classes.

**Fixed.** §5 defines `null` as "outside this vocabulary", never "no change".

### 1.11 MINOR — G0's tilde-fence claim was wrong in the registry doc

`GRAMMAR-LANES-v1.md` listed tilde fences among the constructs that block G0
scope cleanliness. `code_block_fenced` is in G0's frozen set, so the lane
oracle's non-recognition of `~~~` is a *rejected candidate*, not a blocker.

**Fixed in the document, not the code.** G0's reading of `~~~` bytes is
ordinary paragraph text by its own grammar; the probe still records the
divergence as a `candidate_rejected_by_lane_rule` fact, so a tilde-fence-heavy
file is visibly a poor G0 subject without `strict_scope_clean` claiming it.
Changing the eligibility rule instead would have been a lane-semantic change
with no fixture behind it.

### 1.12 MINOR — boundary loose ends

```text
real_source_spec swallowed I/O and parse errors (`.ok()?`), so a hand-broken
  real-source-v1.json silently dropped the case from the pilot
lane_has_implementation hardcoded lane ids instead of reading the registry
STRUCTURE.md and the pilot README named PR #34 files (licenses/,
  acquisition-config.json, tools/acquire.py) as if they existed on this branch
workloads/_cache and workloads/sources are untracked and unignored, so the
  "never tracked in Git" claim is unenforced on this branch
```

**Partly fixed.** The PR #34 file references are now explicitly marked as
living on PR #34's branch. The other three are recorded here as known and
deliberately deferred: the first two are robustness/purity improvements with no
effect on any published artifact, and the last is PR #34's own `.gitignore`
change — duplicating it on a stacked branch would collide with PR #34 rather
than help it. This branch therefore stages with explicit pathspecs and never
`git add -A`.

---

## 2. What this review did NOT settle

```text
- no H0-H4 timing, ranking, or selection was reviewed: none was performed
- the real 3,970-file universe was not re-verified byte-for-byte; there is no
  tracked inventory on this branch to compare against (see §1.12)
- `expected_transition` remains a label, not a machine-checked claim (§1.5)
- PREDICATE-v1 cannot express every real edit effect; `change_class = null`
  is the recorded answer, not a gap to paper over (§1.10)
- the trace step `expected_transition` strings are unvalidated, for the same
  reason as §1.5
```

## 3. Tests added by this review

```text
semantics/tests/profiler.rs    a_candidate_anchored_on_a_fence_opener_is_still_literal_content
                               the_empty_source_profiles_under_both_lanes
semantics/tests/transitions.rs kind_predicates_see_out_of_band_recognized_facts
                               a_no_op_edit_is_rejected
                               a_vacuous_assertion_set_is_rejected
semantics/tests/lifecycle.rs   a_break_that_changes_nothing_is_rejected
semantics/tests/contracts.rs   published_artifacts_carry_no_machine_paths
```

Each fails against the pre-review code and passes against the current tree.
