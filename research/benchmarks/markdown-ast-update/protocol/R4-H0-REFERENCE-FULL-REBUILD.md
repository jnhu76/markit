# R4 — H0 Reference Full Rebuild

Status: **H0_REFERENCE_PASS** (gate closed 2026-09-17: human final review
passed after CORRECTIVE-1; PR #29 merged into master as
`21d7d832fec84fceedb7600cccb4296745395fc1`. The sections below record the
stage as reviewed; this closure note records the gate result only.)
Campaign: #22 MARKIT-MARKDOWN-BENCHMARK-1
Branch: `research/22-r4-h0-reference-full-rebuild-1`
Base: `master` @ `17f6040b21f59f452fa57d67b512d0f4858f431f` (R3 freeze,
`GRAMMAR_CORPUS_MUTATION_FREEZE_PASS`, PR #28)
Stage heads: `8528aff` (R3 gate closure) → `67d337b` (H0 parser + CORPUS-v1
generator) → `34db86c` (depth-decrease fix + differential gate) → `21107f9`
(R4 gate scripts) → `fa596bb` (frozen matrix reproduction test)
Provenance for every correctness claim in this stage:
`rustc 1.97.1 (8bab26f4f 2026-07-14)`, Linux 6.18.33.2-microsoft-standard-WSL2
x64, workspace pinned by `rust-toolchain.toml`, branch HEAD `fa596bb`.

This file records the R4 stage: what was built, the authority decisions and
observations made while instantiating the frozen workload, the adversarial
review pass, and the verification results. R4 implements the H0 = FULL_REBUILD
reference and its correctness gate. It runs NO benchmark, records NO timing,
and starts NO H1–H4 work.

---

## 1. Authority chain and compliance

```text
Issue #22 -> R0 -> R1 -> R2 -> R3 freeze -> R4 (this stage)
```

R4 consumed the frozen R3 artifacts literally and amended nothing:

- `grammar/BENCH-GRAMMAR-v1.md` — implemented as the sole parser semantics
  (13 deviations D1–D13; NOT CommonMark). 43/43 golden fixtures pass; at
  review head `3f0a6f0` no fixture file had changed. CORRECTIVE-1 later
  repaired the `expected_tree` FIELD LISTS of F027/F028/F038 (they carried
  a `CodeSpan content=` field that NORMALIZED-RESULT-v1 §1 does not
  define): source bytes, source SHA-256, spans, and grammar semantics are
  unchanged — fixture artifact conformance repaired, semantic authority
  unchanged (see `protocol/R4-H0-REFERENCE-CORRECTIVE-1.md`).
- `grammar/NORMALIZED-RESULT-v1.md` — implemented in the new `oracle` crate
  (13 node kinds, UTF-8 byte half-open spans, zero-length spans only for
  `FencedCode.content`, `Document == [0, len)`, deterministic SHA-256
  checksum over the normalized semantic content — no `DefaultHasher`, no
  `HashMap` iteration; audited).
- `corpus/CORPUS-v1.md` — implemented as the pure-tiling reference generator
  `corpusgen` (no PRNG consumed; `GLOBAL_SEED` recorded identity only). The
  24 receipts under `corpus/` are NEW files recording the exact generated
  bytes (`source_sha256`); `corpus/manifest.toml` and `CORPUS-v1.md` are
  unchanged. No corpus payload (16 MiB or otherwise) is committed; the gate
  regenerates all 24 receipts byte-identically.
- `mutations/MUTATION-v1.md` + `cases/case-manifest-v1.toml` — implemented
  as the instantiation library `corpusgen::mutations` (generic edits,
  13 structural recipes, anchors with +7 dephasing / down-snap / TINY-SMALL-
  MEDIUM rules, §2.1 selection). The frozen Block-D expansion reproduces
  exactly (§5 below). The R3 gate-closure note added to
  `protocol/R3-GRAMMAR-CORPUS-MUTATION-FREEZE.md` is the task-ordered
  commit 1; no frozen section changed.

No `R4_AUTHORITY_CONFLICT` was raised: no frozen R0–R3 semantics contradicted
the implementation. The one place the frozen matrix and a naive scan
disagreed (§4.2) was resolved IN FAVOR OF THE FREEZE, not by editing it.

## 2. What was built

```text
oracle/                      NORMALIZED-RESULT-v1 vocabulary + SHA-256
                             checksum + node_path_at (§4 ordered path query)
corpusgen/                   reference CORPUS-v1 generator (pure tiling,
                             byte-exact vs verify_r3.py) + mutation
                             instantiation + receipt rendering + gen-receipts
mechanisms/full-rebuild/     H0 = FULL_REBUILD: eager, total, deterministic,
                             byte-coordinate-correct clean parser + R1
                             Mechanism impl; no reuse machinery of any kind
tests/fixtures.rs            43/43 golden fixtures + non-tautology probe
tests/corpus_differential.rs the R4 differential/invariant/QUERY/eager/
                             attribution/matrix suites (18 tests)
scripts/verify-r4.sh         the R4 correctness gate (prints
                             "R4 H0 REFERENCE GATE: PASS")
scripts/mutation-check-r4.sh negative gate: 5/5 temporary corruptions
                             DETECTED (restored, never committed)
```

H0 boundaries held: the frozen R1 `Mechanism` contract as-is; work facts only
through the `MechanismContext` sink; no clock anywhere in mechanism/oracle
(audited); horse-private parse modules (no other crate links full-rebuild).

## 3. The correctness gate (frozen judge, applied)

For every instantiated case where the comparison is defined:

```text
H0 update result == clean authoritative parse of the same post bytes
```

— structurally (full tree equality) AND by the deterministic checksum. As the
suite's own header states: structural equality with the clean parse of the
SAME bytes is a consistency oracle; grammar conformance is established by the
43 frozen fixtures. Together they are the H0 gate. Coverage instantiated by
the suite: full-parse equality + determinism on all 8 shapes (incl. 16 MiB
for 3 shapes), the Block-B generic grid (plain / fence_heavy / mixed at 64k,
full 5 ops × 3 sizes × 3 anchors), insert-tiny on every shape × {64k, 1m} ×
3 anchors, the Block-C scaling slice (many_blocks / huge_block at 64k and
16 MiB incl. the MEDIUM window cap), multibyte-sensitive ops on all CJK-
carrying shapes, all applicable structural recipes at 64k, the forward-state
/ semantic-dependency probes at 1 MiB, old-state independence (true /
unrelated / corrupted old states must yield identical results), chained
edits vs clean parses at every step, NODE_PATH_AT ordered-batch contract +
EOF/gap/mid-scalar/half-open-boundary extremes, the eager-completion proof,
honest attribution, post-edit tree invariants, and byte-coordinate
regression probes.

## 4. Instantiation decisions and recorded observations

These are recorded per the task contract; none changed a frozen count.

### 4.1 Anchor landing arithmetic vs MUTATION-v1 §2 prose (formula wins)

Raw anchors are `N/4, N/2, 3N/4` — for the frozen sizes these are multiples
of 65 536, so they land on unit/line boundaries determined by the tiling
periods, and the +7 dephase puts them at intra-unit offset 7 for plain,
many_blocks, huge_block, fence_heavy, and mixed; deep_container lands at
mountain offset 7 → down-snapped to 16389 (inside 文). Two §2 prose claims
do not match this arithmetic:

- `reference_fanout`: the prose implies the EARLY/MIDDLE anchors hit CJK
  link lines; the formula lands them on ASCII link lines (ordinal
  `j = 1024k − 8 ≡ 8 mod 16`). The instantiated case content therefore
  differs from the prose expectation; every frozen COUNT and applicability
  decision is unaffected (formula is the authority; prose recorded here as
  drift).
- `inline_dense`: the prose says the MIDDLE anchor lands on a delimiter
  byte; the formula lands it on line byte 6 (`b` of `bb`), one byte off the
  first delimiter. Same disposition.

### 4.2 M-BB-PARA-SPLIT on reference_fanout — gate, not scan fact

The scan finds 8-letter CJK-tail filler runs on fanout link lines that
textually satisfy the recipe's letter-run predicate, but the frozen matrix
does not declare that combo applicable. Because the frozen case counts DEPEND
on the combo not existing, the generator implements the frozen applicability
matrix as an explicit gate (`structural_edit_for` refuses declared-N/A
combos). The frozen counts are authority; the scan observation is recorded
here. (Analogously, M-BB-PARA-SPLIT non-applicability is a GATE decision,
never a "scan found nothing" fact — scan silence must not silently change
case counts.)

### 4.3 Inline segment convention (H0 reading, documented)

Inline scanning runs per content SEGMENT — a maximal byte range of one
block's content between container prefixes (the prefix bytes and the LF
before a prefix line are trivia BETWEEN text runs; NORMALIZED-RESULT-v1 §2).
Emphasis/links therefore never pair across a container prefix or a line
boundary. This is the H0 reference reading of "maximal runs within the
parent block's content region"; BENCH-GRAMMAR-v1 has no construct that
crosses a prefix/line boundary (code spans and link destinations are
line-local by D10/§9.2), and fixtures F021–F035 pin each construct.

### 4.4 Query anchors

QUERY is ONE ordered batch of three NODE_PATH_AT subqueries at the generic
EARLY/MIDDLE/LATE anchors over the INITIAL state, answered from the
normalized tree alone (`node_path_at` takes no source and no parser — it
cannot reparse). EOF answers `[Document]` (end-inclusive special case);
whitespace gaps answer `[Document]`; a mid-scalar offset answers the
containing Text path (the tree contract is pure byte containment — frozen
anchors are boundary-snapped upstream).

## 5. Frozen matrix reproduction

`structural_recipe_slots_reproduce_the_frozen_matrix_counts` enumerates every
declared Block-D slot via `structural_edit_for` over all 24 corpora and
asserts the per-recipe counts equal the freeze exactly:

```text
M-LOC-TEXT 12   M-LOC-UTF8-SWAP 21   M-BB-PARA-SPLIT 15
M-BB-PARA-MERGE 17   M-CS-ITEM-INDENT 6   M-CS-BQ-NEST-LINE 6
M-FS-FENCE-OPEN 42   M-FS-FENCE-CLOSE 6   M-IDS-EMPH-INSERT 6
M-IDS-CODE-DELIM 6   M-IDS-LINK-DELIM 9   M-SD-DEF-REPLACE 6
M-SD-DEF-DELETE 6                     total = 158   == frozen
```

A scan that found no target where the freeze declares APPLICABLE — or an
edit where it declares NOT_APPLICABLE — fails the test. The full expansion
costs ~25 min in the debug profile (16 MiB scans × 13 recipes × 24 corpora),
so it is `#[ignore]`d there and `verify-r4.sh` runs it under `--release`
(~2 min). This is gate-runtime engineering, not H0 tuning: no timing value
is recorded anywhere, and H0 itself is unchanged by it.

Case identity: the 7 declared slot overlaps (core INSERT-TINY-MIDDLE ==
grid/scaling slots) build byte-identical edits BY CONSTRUCTION — the suite's
`generic_grid_case` is the same generator call for both roles — so no
unexpected collapse exists by construction. Content-level `CaseKeyV1`
collision enumeration over all 370 slots is instantiation-time duty that
belongs to the runner (R5+/R6); R4 neither needs nor performs it, and this
limitation is stated rather than papered over.

## 6. Adversarial review pass (one pass; findings with severities)

The review asked fifteen questions and produced one MAJOR, three IMPORTANT,
and four MINOR findings. All MAJOR/IMPORTANT items are already fixed in the
stage commits; the MINOR items are recorded limitations/observations.

```text
Q1  Is the differential judge tautological?
    A: It is a same-parser consistency oracle BY DESIGN and the suite says
       so; grammar conformance comes from the 43 frozen fixtures. No
       conformance claim is based on the differential alone. PASS (no
       overstated claim).
Q2  Could a wrong parser pass the fixtures?
    A: No silently: fixtures pin exact spans/kinds (probe
       fixture_comparison_is_not_tautological mutates a tree and requires
       the comparator to fail), and the negative gate corrupts the parser
       itself 5 ways (M1–M4 detected by fixtures, M5 by the differential).
       PASS.
Q3  Are the frozen case counts reproduced?
    A: Block D: yes, exactly 158/158 (§5). Blocks A–C are instantiated by
       the differential suite on representative sizes (16 MiB full grid
       excluded for suite runtime; scaling slice covers 16 MiB). Content-
       level CaseKey collision enumeration deferred to the runner (recorded,
       §5). PASS with recorded scope.
Q4  Are anchors built per MUTATION-v1 §2?
    A: Yes (+7, fit, down-snap, TINY/SMALL/MEDIUM, §2.1 selection,
       anchor_offset recorded). Two §2 PROSE drifts found and recorded,
       formula authoritative (MINOR-1). PASS.
Q5  Byte coordinates correct?
    A: validate_tree (boundaries, containment, sibling order, half-open,
       zero-length rule) runs on every tree the suite produces; CJK/emoji
       probes pin mid-scalar and boundary behavior; the corpus generator
       rejects CR/tab/non-LF terminators. PASS.
Q6  Determinism?
    A: Pure tiling (no PRNG consumed), SHA-256 checksum (audited: no
       DefaultHasher/HashMap order), double-parse equality incl. 16 MiB,
       receipts regenerate byte-identically (gate). PASS.
Q7  H0 contract: no reuse / eager / total?
    A: Old-state independence proven against corrupted old states; eager
       proven structurally (complete() has no source/sink) and
       behaviorally (all counters observed before complete();
       H0Pending::document()); total over any valid UTF-8 input (no error
       paths). MAJOR-1 found here (below), fixed. Recursion depth on
       adversarial nesting is out of the frozen workload (MINOR-3). PASS.
Q8  Attribution honest?
    A: Known(nodes)/Known(blocks) from real counts, inspection union ==
       complete post source, NotApplicable ≠ fabricated zero for the
       meaningless slots, distinctness asserted, no slot left Unknown,
       measurement overhead separated (inspection events are attribution
       data, never a timer). One misclassification was found by the human
       review and fixed by CORRECTIVE-1: `nodes_reused` is a MEASURED
       `Known(0)` for H0 (a full rebuild has the precise fact zero), not
       `NotApplicable`; the `NotApplicableSlot::NodesReused` variant was
       removed. PASS (as corrected).
Q9  Receipts/payloads?
    A: 24 receipts committed, zero payloads, --check byte-identity in the
       gate. PASS.
Q10 Gate is correctness-only?
    A: verify-r4.sh runs fmt/clippy/receipts/tests/R1/R3/mutation-check;
       no benchmark, no timing output; final line "R4 H0 REFERENCE GATE:
       PASS". PASS.
Q11 Frozen R0–R3 untouched?
    A: Semantics untouched: BENCH-GRAMMAR-v1, NORMALIZED-RESULT-v1,
       CORPUS-v1, MUTATION-v1, case counts — all zero-diff at review head
       `3f0a6f0`; the R3 record carries only the task-ordered
       gate-closure note; corpus/ gains receipts only. One ARTIFACT
       conformance defect was found by the human review and repaired by
       CORRECTIVE-1: three fixture expected trees (F027/F028/F038) carried
       a field the frozen vocabulary does not define — repaired in favor
       of the single-owner contract, semantics NOT expanded. PASS (as
       corrected).
Q12 Scope discipline?
    A: No H1–H4 work (other mechanism crates untouched since R1 — audited
       4bc36d4); no performance measurement or claim anywhere in R4; the
       release-profile matrix run is gate runtime, not an H0 claim.
       PASS.
Q13 Was MAJOR-1's fix a semantic change?
    A: No — the pre-fix behavior violated frozen §6/§7 on depth decrease;
       the fix aligns with the freeze; fixtures unchanged, still 43/43.
       No R4_AUTHORITY_CONFLICT. PASS.
Q14 QUERY semantics frozen-conformant?
    A: Ordered batch, tree-only answering, deterministic, extremes
       defined. PASS.
Q15 Docs/authority updated?
    A: This record; README/ROADMAP gate lines advanced; AGENTS.md
       campaign unchanged (still #22). PASS.

MAJOR-1  H0 container depth-decrease cascade (FIXED, 34db86c).
         strip_prefixes re-examined already-carried frames against the
         advanced column after an inner close, cascading quote/list closes
         when depth DECREASED mid-mountain (e.g. "> > b" then "> c"): the
         inner paragraph was hoisted to document level INSIDE the outer
         quote's span while the quote's span still covered it — sibling
         overlap, wrong parent, wrong spans. Surfaced by the differential
         suite on deep_container (validate_tree), never by the fixtures
         (no fixture exercises depth-decrease-while-outer-carries — itself
         a recorded fixture-coverage note, not a fixture change). Fixed by
         scoping the post-close re-decision to exactly the one frame that
         owes it: a closed item's parent List (sibling-vs-close, §7);
         every other frame below an inner close already carried its prefix
         on that line. Pinned by hand-built regression probes.

IMPORTANT-1  Validator stricter than the contract (FIXED). The suite
         initially asserted strictly-nested spans along NODE_PATH_AT
         paths; NORMALIZED-RESULT-v1 allows a parent (paragraph) to share
         its single Text child's span. Relaxed to the actual contract
         (containment + sibling order); the contract itself was never
         misimplemented.
IMPORTANT-2  Inline segment convention is an H0 READING of the frozen
         text (documented, §4.3) — a later horse or reviewer may read
         "maximal runs within the parent block's content region"
         differently; fixtures pin the constructs but not this sentence.
         Recorded so R5 parity review can re-confirm the reading.
IMPORTANT-3  Full 158-slot expansion excluded from the plain debug suite
         run for runtime (§5); the R4 GATE still executes it (release).
         Recorded so nobody mistakes `cargo test` alone for the full gate.

MINOR-1  MUTATION-v1 §2 prose drifts (§4.1) — recorded, formula
         authoritative, zero frozen counts affected.
MINOR-2  M-BB-PARA-SPLIT/reference_fanout gate (§4.2) — frozen counts
         win over the scan; recorded.
MINOR-3  Recursion depth: inline re-scan recursion (emphasis/link
         nesting) and node Drop recursion are linear-depth; adversarial
         inputs (megabytes of '*') could overflow the stack. The frozen
         workload is bounded (≤ 16 container depth, real inline density),
         so this is out of scope for H0; recorded for future hardening.
MINOR-4  `gen-receipts` needed explicit [[bin]] registration (cargo only
         auto-discovers src/bin); gate covers receipt regeneration.
```

## 7. Verification results

```text
scripts/verify-r4.sh                     PASS (final run on HEAD, 2026-09-17)
  cargo fmt --all -- --check             PASS
  cargo clippy --workspace -D warnings   PASS
  gen-receipts --check (24 receipts)     PASS (byte-identical)
  cargo test --workspace                 PASS
    fixtures                             2/2 test fns (43/43 fixtures)
    corpus_differential                  18/18
    corpusgen (incl. all-24-corpora)     37/37
    oracle / common / runner / null-r1   all green
  verify-r1.sh (R1 regression)           PASS
  verify-r3.sh (R3 regression)           PASS
  frozen 158-slot matrix (--release)     PASS
  mutation-check-r4.sh                   5/5 DETECTED
```

CORRECTIVE-1 re-ran the full gate on the corrective HEAD (see
`R4-H0-REFERENCE-CORRECTIVE-1.md` §6): all lines PASS again, plus the new
field-legality / zero-length / attribution regressions and the R3 static
gate's self-tests.

## 8. Self-assessment verdict

```text
R4 SELF-ASSESSMENT VERDICT: READY_FOR_ADVERSARIAL_R4_REVIEW
(superseded 2026-09-17 by CORRECTIVE-1 — current verdict:
 READY_FOR_FINAL_R4_REVIEW)

H0_REFERENCE_PASS conditions met:
  - 43/43 golden fixtures (no fixture changed)
  - update == clean authoritative parse, structurally + checksum, over
    the instantiated CORPUS-v1 cases
  - EAGER_COMPLETION_VALIDATION_PASS proof for H0
  - honest attribution (Known/Unknown/NotApplicable distinct)
  - deterministic corpus + receipts + checksum
  - negative gate: 5/5 mutations detected
Gates this stage does NOT claim: any H1-H4 work, any measurement, any
tuning, any Markit algorithm or architecture decision.

STOP: R5 (H1/H2/H3/H4 mechanism implementation) is not started.
PR opened against master; not merged by the agent.
```
