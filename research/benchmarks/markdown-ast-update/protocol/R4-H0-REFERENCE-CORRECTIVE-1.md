# R4-CORRECTIVE-1 — H0 Reference: field authority, zero-length, attribution

Status: **H0_REFERENCE_PASS** (2026-09-17; human final review passed after
this corrective; PR #29 merged into master as
`21d7d832fec84fceedb7600cccb4296745395fc1`. The FINAL VERDICT below is the
historical self-assessment at the corrective head.)
Campaign: #22 MARKIT-MARKDOWN-BENCHMARK-1
Branch: `research/22-r4-h0-reference-full-rebuild-1` (PR #29, not merged)
Base reviewed head: `3f0a6f02ea8a4174e237a8a07dd56a3efa79ef6b`
Task: MARKIT-R4-H0-REFERENCE-CORRECTIVE-1 — ONE targeted corrective after
the human adversarial review of R4. Not a rewrite of H0; no H1–H4 work; no
R5; no benchmark; no grammar broadening.

---

## 1. R3 artifact corrective (recorded verbatim in substance)

```text
R3 normalized semantics were NOT expanded.

The single-owner closed vocabulary (grammar/NORMALIZED-RESULT-v1.md §1)
was already unambiguous: CodeSpan carries no fields. Three golden
expected-tree artifacts (F027, F028, F038) violated that vocabulary by
giving CodeSpan a content= field, and were corrected before any mechanism
race or performance measurement began.

Source bytes, source hashes, grammar semantics, spans, corpus, mutations,
case counts, and workload identity are unchanged.
```

Distinguish, and never conflate again:

```text
semantic authority              unchanged (zero-diff vs R3 merge 17f6040)
fixture artifact conformance    repaired (F027/F028/F038 expected_tree
                                field lists only)
```

Consequently the earlier claim "the entire `grammar/` directory has zero
diff versus the R3 merge" is no longer made and has been corrected in the
stage record (`R4-H0-REFERENCE-FULL-REBUILD.md` §1, Q11).

## 2. MAJOR — CodeSpan field authority contradiction (FIXED)

The review found F027/F028/F038 declaring `(CodeSpan start end
content=a:b)` while NORMALIZED-RESULT-v1 §1 (single owner, closed) gives
CodeSpan no fields. Resolved IN FAVOR OF the contract; `CodeSpan.content`
was NOT added to NORMALIZED-RESULT-v1. The grammar statement that a code
span's content is the exact source bytes between its delimiters is
unchanged — it is derivable from source + span, not a normalized field.

- Fixtures: F027, F028, F038 `expected_tree` — `content=` removed from the
  CodeSpan nodes. All 43 fixtures audited (grep): no other CodeSpan carries
  a field; no other kind carries a field outside the frozen table (now also
  machine-enforced, §3). Source bytes, source SHA-256, and CodeSpan spans
  untouched.
- H0: `mechanisms/full-rebuild/src/inline.rs` no longer populates
  `Node.content` for `NodeKind::CodeSpan` (the `n.content = Some(...)`
  assignment removed). `FencedCode.content` remains required and unchanged.

## 3. Exact field-kind legality as a shared gate (NEW)

The defect survived because both the R3 verifier and the R4 fixture loader
accepted any recognized field on any node kind. Fixed permanently with one
frozen legality table (NORMALIZED-RESULT-v1 §1):

```text
Document             {}
Paragraph            {}
Heading              {level}
BlockQuote           {}
List                 {}
ListItem             {marker}
FencedCode           {info, content}
Text                 {}
Emphasis             {}
CodeSpan             {}
Link                 {destination}
ReferenceLink        {label, destination}
ReferenceDefinition  {label, destination}
```

For every normalized node: required fields exist, forbidden fields are
absent, no duplicate field key, no unknown field — never merely "the
required subset exists".

Enforcement surfaces:

- `oracle/src/validate.rs` (NEW) — the one shared Rust validator
  (`validate_normalized` / `validate_root`): exact legality, zero-length
  rule (§4), `FencedCode.content` interval, parent containment, sibling
  order, UTF-8 char boundaries, `Document == [0, len)`. Reusable by later
  horses; no horse-specific assertions scattered.
- `oracle/src/fixture.rs` — every fixture expected tree passes the gate at
  load time (R4 fixture loading); the tree reader rejects duplicate/unknown
  field keys.
- `mechanisms/full-rebuild/src/lib.rs` — every H0 result (parse_document
  and both update/full_parse paths) passes the gate before it is returned.
- `scripts/verify_r3.py` — the Python static gate independently enforces
  the same frozen table (`FIELD_TABLE`), rejects duplicate field keys in
  expected trees, and carries self-test regressions so the gate provably
  catches drift (a temporarily reintroduced `CodeSpan content=` FAILS the
  R3 gate — verified in §6).

Regression tests prove these FAIL: `CodeSpan content=`, `Text
destination=`, `Paragraph level=`, `FencedCode` missing content (and
missing info), `ReferenceLink` missing destination, `Heading` missing
level, zero-length nodes (§4). And these PASS: `CodeSpan` with no fields,
`FencedCode` with info + content, `ReferenceLink` with label + destination.
(Rust: `oracle/src/validate.rs` tests + `oracle/src/fixture.rs`
`load_gate_rejects_the_corrected_codespan_field`. Python:
`check_gate_self_test()` — 7 must-fail, 4 must-pass, duplicate-key
rejection.)

## 4. Zero-length node semantics (FIXED)

NORMALIZED-RESULT-v1 §2: normalized node spans are NEVER zero-length; the
only zero-length interval in the vocabulary is the `FencedCode.content`
FIELD (empty body). The R4 validator previously encoded the incorrect rule
`start == end => kind must be FencedCode`; it now enforces, for EVERY
normalized node (Document included):

```text
start < end
```

and separately validates the `FencedCode.content` field:

```text
node.start <= content_start <= content_end <= node.end
content_start/content_end on UTF-8 char boundaries (source given)
content_start == content_end is LEGAL
```

Regressions: zero-length FencedCode NODE -> FAIL; non-empty FencedCode node
with empty content interval -> PASS; zero-length Text -> FAIL (both Rust
and Python). The gate runs on every fixture expected tree and every H0
result.

## 5. Attribution — H0 `nodes_reused` is Known(0) (FIXED)

R1 freezes `Known(0)` != `Unknown` != `NotApplicable` and
`add(0)` = measured zero. H0 FULL_REBUILD has the precise fact
`nodes_reused == 0` because it intentionally reuses no old parse node.
`report_attribution` now reports it through the ordinary cumulative counter
path (`add_nodes_reused(0)` -> `Known(0)`) instead of declaring the slot
`NotApplicable`. Genuinely inapplicable H0 slots remain `NotApplicable`:
`metadata_records_touched`, `restart_distance`, `convergence_distance`,
`fallback_to_full_count`.

`NotApplicableSlot::NodesReused` (an R4-added variant that existed only to
support the erroneous classification) was REMOVED from
`common/src/work.rs` rather than left as a misleading authority surface;
the enum documents the reclassification. Regressions:
`common` — `nodes_reused_zero_is_measured_and_distinct_from_not_applicable`
(`Known(0)` != `NotApplicable`); `corpus_differential.rs` — H0
`nodes_reused == Known(0)`, `nodes_rebuilt > 0` on non-empty parses, and
the distinctness assertion pinned explicitly.

## 6. Verification (all re-run on the corrective HEAD)

```text
git diff --check                         PASS
bash scripts/verify-r1.sh                PASS
bash scripts/verify-r3.sh                PASS
  (now incl. FIELD_TABLE enforcement + 7 must-fail / 4 must-pass
   self-tests; negative probe: reintroducing `content=` on a fixture
   CodeSpan FAILS the gate — applied temporarily, restored)
bash scripts/verify-r4.sh                PASS (final line:
  "R4 H0 REFERENCE GATE: PASS")
  cargo fmt --all -- --check             PASS
  cargo clippy --workspace -D warnings   PASS
  gen-receipts --check (24 receipts)     PASS (byte-identical)
  cargo test --workspace                 PASS
    fixtures                             2/2 test fns (43/43 fixtures)
    oracle validate/fixture/common       all green (new regressions incl.)
    corpus_differential                  17 passed + 1 release-only ignored
  verify-r1.sh / verify-r3.sh            PASS
  frozen 158-slot matrix (--release)     PASS (158/158, counts unchanged)
  mutation-check-r4.sh                   5/5 DETECTED
```

No timing. No performance numbers. Corpus/mutation/case identity:
unchanged (unique case total still 370; Block-D total still 158).

## 7. Focused adversarial pass (the review's 7 checks)

```text
1. Does any CodeSpan still carry content?          NO (grep + gates)
2. Can any node carry a field not authorized?      NO (shared gate on
                                                   fixtures, H0, R3 static)
3. Can any normalized node have start == end?      NO (start < end for
                                                   every node, both gates)
4. Can FencedCode.content legally be empty?        YES — legal, validated
                                                   as a FIELD, pinned by
                                                   PASS regressions
5. Is H0 nodes_reused exactly Known(0)?            YES (regression-pinned)
6. Did any source bytes / corpus / mutation /
   case count change?                              NO (receipts byte-stable,
                                                   158/158, 370 unique)
7. Did H1-H4 remain untouched?                     YES (only common/
                                                   oracle/full-rebuild/
                                                   fixtures/scripts/
                                                   protocol changed)
```

## 8. Scope and stop

```text
H1–H4 untouched:        YES
benchmark run:          NONE (no timing recorded anywhere)
R5 started:             NO

FINAL VERDICT: READY_FOR_FINAL_R4_REVIEW
STOP: PR #29 left unmerged by the agent; R5 not started.
```
