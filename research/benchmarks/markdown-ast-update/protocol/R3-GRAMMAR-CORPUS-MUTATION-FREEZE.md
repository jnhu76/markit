# R3 — BENCH-GRAMMAR-v1 + Corpus / Mutation Freeze

Status: **READY_FOR_ADVERSARIAL_R3_REVIEW** (2026-09-17)
Campaign: #22 MARKIT-MARKDOWN-BENCHMARK-1
Branch: `research/22-r3-grammar-corpus-mutation-freeze-1`
Base: `master` @ `c9bad01b6668c6921272e7cc989e8af6186f73ab` (PR #27 merged;
R2 lineage `R2_FINAL_REVIEW_PASS` / `PRIOR_ART_EXTRACTION_PASS` verified via
merge commit)

This file records the R3 stage: what was frozen, the authority decisions,
the adversarial review pass, and the verification results. R3 is a
preregistration/freeze stage — it implements no parser, no mechanism, no
benchmark.

---

## 1. Authority chain and compliance

```text
Issue #22  ->  R0-METHODOLOGY.md (frozen)  ->  R1-HARNESS-CONTRACT.md
           ->  R2 prior-art records        ->  R3 freeze (this stage)
```

R3 specialized ONLY undefined workload details. Frozen R0/R1 items were
not amended:

```text
H0-H4 definitions, implementation parity, timer boundaries,
measurement semantics, operation set, shape set, size set,
structural minimum (six families), R1 interfaces, CaseKeyV1
```

Authority check against R2: hypotheses were converted into workload
COVERAGE only (see `cases/CASE-MATRIX-v1.md` §7); no hypothesis became an
expected outcome; no R2 `MECHANISM_INTRINSIC_STATE` classification was
silently frozen into workload artifacts.

QUERY precedence: Issue #22 and R0 name `QUERY` but define no containment
semantics; R9 names the "offset -> enclosing region" surface. The frozen
`NODE_PATH_AT` contract (NORMALIZED-RESULT-v1 §4) is consistent with all
three and is the first specific definition — no contradiction, so no
`R3_AUTHORITY_CONFLICT`.

One R1-identity constraint shaped the freeze honestly: `CaseKeyV1`
rejects edit fields for `query`, so post-edit querying is DEFERRED (not
squeezed into STRUCTURAL_EDIT), and QUERY is one case per corpus
answering three anchors.

## 2. Deliverables (all under `research/benchmarks/markdown-ast-update/`)

```text
protocol/R3-GRAMMAR-CORPUS-MUTATION-FREEZE.md      (this file)

grammar/BENCH-GRAMMAR-v1.md        exact semantics of the shared subset
grammar/NORMALIZED-RESULT-v1.md    node vocabulary, spans, NODE_PATH_AT
grammar/fixtures/*.toml            43 golden semantic fixtures (hand-
                                   authored expected trees + sha256)

corpus/CORPUS-v1.md                8 shapes x 3 sizes, exact recipes,
                                   determinism contract, receipts schema
corpus/manifest.toml               24 corpora, machine-readable

mutations/MUTATION-v1.md           operations, edit sizes, positions, 13
                                   structural recipes, case-identity map
mutations/manifest.toml            13 recipes, machine-readable

cases/CASE-MATRIX-v1.md            4 blocks, 370 unique cases, R2
                                   hypothesis coverage map
cases/case-manifest-v1.toml        block + structural expansion
scripts/verify_r3.py + verify-r3.sh   static freeze gate (no benchmark)
```

Ownership is single: each definition lives in exactly one file; the
manifests are projections. Key frozen decisions:

```text
BENCH-GRAMMAR-v1  line-based dispatch (total, ordered), LF-only, tabs are
                  ordinary characters, no escapes, tight-only unordered
                  lists (-,*), single-line reference definitions, no
                  shortcut links, exact-byte code spans, single-'*' LIFO
                  emphasis with close-preferred tie-break AND the
                  empty-emphasis adjacency ban (what makes "**a**" one
                  nested pair), unclosed fences run to EOF, 13 explicit
                  CommonMark deviations (D1-D13)
NORMALIZED-RESULT 13 node kinds, UTF-8 byte half-open spans relative to
                  the whole document, leading indentation never in spans,
                  trivia = span gaps only, resolved reference
                  destinations as REQUIRED facts, no identity/mechanism
                  state in the vocabulary
CORPUS-v1         exact byte sizing (zero tolerance), pure tiling
                  generator contract (corpus-gen-v1, GLOBAL_SEED recorded,
                  PRNG reserved), units divide all three sizes so padding
                  never fires, CJK in 7 of 8 shapes, generation receipts
                  (sha256/actual_bytes) schema frozen, recorded at first
                  generation in R4+
MUTATION-v1       byte-authoritative operation classes, TINY/SMALL/
                  MEDIUM exact recipes, EARLY/MIDDLE/LATE percentile
                  anchors with down-snapping, 13 recipes across the six
                  R0 families with explicit applicability and
                  NOT_APPLICABLE rules, structural_edit labels declared
                  by the generator (consistent with common::edit)
CASE-MATRIX       core + generic grid + scaling slice + structural
                  targeted = 370 UNIQUE cases (content-deduped), every
                  case carrying a reason
```

## 3. Fixture validation methodology (disclosure)

The 43 fixtures are hand-authored authority. Two machine aids were used;
neither is part of the freeze:

```text
1. scripts/verify_r3.py (committed): internal-consistency invariants
   only — vocabulary, bounds, char boundaries, root span, containment,
   sibling order, sha256. It does NOT re-parse Markdown.
2. an UNCOMMITTED throwaway checker (local /tmp): an independent
   implementation of the frozen semantics used to cross-check the
   hand-authored trees. It found 3 real fixture errors (blank-line
   list offset, collapsed-form length, missing inter-link LF Text run)
   and one real grammar gap (the empty-emphasis adjacency ban), all
   fixed. 41/43 fixtures cross-check mechanically; F011
   (list-nested) and F043 (fence-inside-list-item) were hand-verified
   byte-by-byte (the throwaway checker does not implement nested-span
   propagation or fences inside list items).
```

No parser code from that exercise was committed; R4's H0 is the real
implementation.

## 4. Adversarial review pass (one pass, per stage policy)

Findings and dispositions (MAJOR -> correct and rerun; IMPORTANT/MINOR
-> fixed in the same pass):

```text
IMPORTANT-1  Grammar self-contradiction on empty list items: the item
             opening rule required 1..4 spaces after the marker while §7
             also declared a bare "-" line an empty item. Fixed: opening
             rule now admits marker-at-EOL (empty item, content indent =
             marker column + 1).
IMPORTANT-2  Recipe applicability was false for inline_dense:
             M-LOC-TEXT (no >= 32 B letter runs) and M-BB-PARA-SPLIT
             (no >= 4 B plain Text runs) would have violated their own
             preconditions at instantiation. Fixed: inline_dense removed
             from both; manifests and counts updated.
IMPORTANT-3  M-LOC-UTF8-SWAP claimed MIXED, but the MIXED tile is
             all-ASCII by recipe. Fixed: mixed removed from the recipe
             (UTF-8 stress is carried by the other seven shapes);
             applicability note documents this.
IMPORTANT-4  M-FS-FENCE-CLOSE precondition ("first body line at/after
             the anchor with >= 2 before, >= 1 after") is unsatisfiable
             on the frozen 2-body-line fences. Fixed: selection is now
             the fence's LAST body line (>= 1 raw line remains above);
             the postcondition documents the deterministic re-dispatch
             of the orphaned closer line as a new fence opener.
IMPORTANT-5  Case totals ignored content-addressed identity: blocks
             overlap in 7 cases (core TINY-INSERT-MIDDLE also inside the
             grid and scaling slice). Fixed: totals are UNIQUE-case
             counts (370); the overlaps are listed, and the validator
             now expands and dedupes by content identity.
MINOR-1      fixture count (43) exceeds the nominal 20-40 band; kept
             with justification (one fixture per pinned rule, zero
             redundancy) and the widened machine band (20-50).
MINOR-2      NORMALIZED-RESULT-v1 §5 stated a stale fixture count
             ("38"). Fixed to 43 with the band justification.
MINOR-3      verify-r3.sh grep gate flagged the discipline section's own
             quotation of banned phrasings. Fixed by paraphrasing §8 of
             MUTATION-v1.md.

MAJOR: none found.
```

Answers to the 20 standing review questions (summary): grammar
ambiguities closed by total ordered dispatch + pinned tie-breaks (Q1);
one shared language/workload for all horses (Q2); subset, 13 declared
deviations (Q3, Q11-adjacent); shapes vary one dominant property each,
MIXED is a fixed tile (Q4, Q20); sizes are exact source bytes (Q5);
generators are pure tiling functions (Q6); all edit boundaries
down-snapped, ASCII insertions, CJK swap boundary-snapped (Q7); labels
byte-true, structural labels declared per common::edit (Q8); §6
invariants + NOT_APPLICABLE discipline (Q9); fanout constructions
guarantee every definition is used (Q10, Q11); propagation always
defined by the total grammar (Q12); NODE_PATH_AT is
representation-independent (Q13); hypotheses are coverage, not outcomes
(Q14, Q19); no horse-specific cases (Q15); H01-H12 all have routes — 10
covered, 2 deferred with named stages (Q16); CaseKeyV1 mapping frozen,
content-dedup rule defined, collision policy STOP (Q17); no parser /
mechanism code committed (Q18).

## 5. Verification results

```text
git diff --check:            clean
TOML validation:             corpus/manifest.toml, mutations/manifest.toml,
                             cases/case-manifest-v1.toml, 43 fixtures —
                             all parse (python3 tomllib)
Enum/uniqueness validation:  shapes, sizes, families, anchors, operation
                             labels, fixture covers tags, unique
                             corpus/mutation/fixture ids — PASS
Deterministic artifacts:     fixture sha256 recomputed and matched — PASS
Case arithmetic:             content-deduped expansion = 370 == frozen
                             total — PASS
bash scripts/verify-r3.sh:   R3 FREEZE GATE: PASS
bash scripts/verify-r1.sh:   R1 ACCEPTANCE GATE: PASS (regression guard;
                             no benchmark campaign run)
Horse-outcome grep gate:     clean
Scope file gate:             no .rs/.c/.mbt under grammar/ corpus/
                             mutations/ cases/
```

## 6. Scope proof

```text
NO H0 IMPLEMENTATION            no parser code committed; fixtures are
                                declarative data; scripts validate
                                static artifacts only
NO H1-H4 IMPLEMENTATION         mechanism directories untouched
NO PARSER IMPLEMENTATION        scope file gate green
NO BENCHMARK TIMING             no timing anywhere in R3
NO PERFORMANCE RESULT           none produced, none claimed
NO R4 WORK                      corpus bytes NOT generated (receipts
                                schema frozen instead)
NO R0/R1 REDEFINITION           authority check §1; amendments NONE
NO NEW PRIOR-ART CAMPAIGN       R2 pins untouched
```

## 7. Verdict

```text
R3 SELF-ASSESSMENT VERDICT: READY_FOR_ADVERSARIAL_R3_REVIEW

After human review the gate may become:
    GRAMMAR_CORPUS_MUTATION_FREEZE_PASS
    READY_FOR_R4_H0_REFERENCE_FULL_REBUILD

STOP: R4 is not started. PR opened against master; not merged by the agent.
```
