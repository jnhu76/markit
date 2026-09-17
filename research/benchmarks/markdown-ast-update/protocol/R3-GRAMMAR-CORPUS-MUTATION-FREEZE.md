# R3 — BENCH-GRAMMAR-v1 + Corpus / Mutation Freeze

Status: **GRAMMAR_CORPUS_MUTATION_FREEZE_PASS** (2026-09-17; corrective-1
applied, then human final review passed; PR #28 merged into master as
`17f6040b21f59f452fa57d67b512d0f4858f431f`. This closure note records the
gate result only; sections 1–8 are the frozen record and are unchanged.)
Campaign: #22 MARKIT-MARKDOWN-BENCHMARK-1
Branch: `research/22-r3-grammar-corpus-mutation-freeze-1`
Base: `master` @ `c9bad01b6668c6921272e7cc989e8af6186f73ab` (PR #27 merged;
R2 lineage `R2_FINAL_REVIEW_PASS` / `PRIOR_ART_EXTRACTION_PASS` verified via
merge commit)
First-freeze head reviewed by the adversarial pass: `e5edd7d`
Corrective head: see §8 (MARKIT-R3-GRAMMAR-CORPUS-MUTATION-CORRECTIVE-1)

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

## 8. Corrective pass — MARKIT-R3-GRAMMAR-CORPUS-MUTATION-CORRECTIVE-1

One targeted corrective against reviewed head `e5edd7d` (PR #28), from
human adversarial review. Scope discipline held: no R0/R1 change, no R2
reopening, no grammar broadening (BENCH-GRAMMAR-v1 sections 1–13 are
byte-identical), no parser/mechanism code, no corpus generation, no R4.

```text
MAJOR-1  FENCE_HEAVY CJK body arithmetic was false ("中"×41 + filler(4)
         = 127 B, not 250 B). Fixed: "中"×82 + filler(4) = 250 B, so
         BOTH fence unit variants are exactly 512 B. New CORPUS-v1 §4.9
         freezes the closed unit-variant arithmetic for ALL eight
         shapes ("every declared unit variant has its declared byte
         size"), and §3.1 pins the every-Nth-unit index rule (0-based,
         index mod N == 0; deep_container lines counted per mountain).
         Verified: every variant row asserted by verify_r3.py on
         in-memory recipe units.

MAJOR-2  DEEP_CONTAINER quote mountain did not encode depth d: the old
         indent-positioned marker form violates frozen §6 (only up to
         3 leading spaces before '>'; nesting requires content STARTING
         with '>') and collapsed to depth <= 2. Fixed: quote line(d) =
         "> "×d + content(63−2d) + "\n" (still exactly 64 B for
         d=1..16). Proven: max quote depth 16 by the §6 marker chain
         and max list depth 16 by the §7 indent arithmetic (with the
         sibling-continuation rule), simulated per line by
         verify_r3.py; mountains do not compound (a mountain's first
         line closes the previous containers per §6 D4 / §7 item-end),
         so max normalized depth is 16 for BOTH kinds. The stale
         "containers stay open across the whole corpus" claim is gone.

MAJOR-3  M-FS-FENCE-CLOSE was inconsistent (doc: ">=2 body lines
         before, >=1 after" — unsatisfiable on the 2-body-line fences;
         manifest: a different last-body-line rule) and its INSERT form
         left the old closer behind as an accidental new opener.
         Redesigned as ONE executable canonical edit, single owner
         MUTATION-v1 §5, projected identically into the manifest:
           precondition  fence with >= 2 body lines
           selection     FIRST such fence in document order; its LAST
                         body line is chosen
           edit          REPLACE_EQ [body2_start, old_closer_end) ->
                         old_closer_bytes + original_body2_bytes
                         (255 B removed == 255 B inserted)
           postcondition fence closes after body1; body2 released into
                         ordinary block structure; the old closer
                         position ceases to exist (no accidental
                         opener); re-convergence local to the unit
         Verified on recipe units: precondition satisfiable on
         fence_heavy AND mixed (identical 512-B ASCII fence unit in the
         MIXED tile); post unit = opener|body1|closer|body2|blank; no
         stray backtick run after the released line.

MAJOR-4  Generic anchors phase-locked onto generator periods: all sizes
         and units are powers of two, so floor(N/4), floor(N/2),
         floor(3N/4) are ≡ 0 (mod u) for every unit u — generic
         same-size edits landed on tile/unit boundaries, invalidating
         the paragraph-interior vs fence-body-interior comparison.
         Fixed: frozen dephasing rule generic_anchor = raw_anchor + 7
         (ONE universal shape-independent constant; every shape, size,
         anchor class), then range clamp, then down-snap. Frozen
         landing table (MUTATION-v1 §2): PLAIN inside paragraph
         content; FENCE_HEAVY inside body1; deep_container line 0
         content (down-snapped onto a char boundary); inline_dense and
         reference_fanout interiors documented (delimiter/label bytes —
         recorded consequences, not surprises); MIXED heading text;
         never ≡ 0 (mod 16/64/512/2048/4096/65536). EARLY/MIDDLE/LATE
         deliberately share the intra-unit offset so cross-class deltas
         isolate absolute-position effects. Structural recipes keep
         their own deterministic re-anchoring (§2 step 4).

IMPORTANT-1  M-LOC-UTF8-SWAP on fence_heavy claimed the swapped char
         becomes ASCII "inside a Text node" — false (fence bodies are
         raw FencedCode.content). Fixed per option A (kept applicable,
         family stays LOCAL_TEXT): fence_heavy postcondition is now
         "one 3-byte CJK scalar inside raw content -> three ASCII
         bytes; FencedCode topology and span UNCHANGED". Case totals
         unchanged (applicability kept).

IMPORTANT-2  Case-count authority drift (386 vs 370 prose; 386/167
         manifest comments; 322 stale docstring). Fixed: single
         machine-readable field expected_unique_cases = 370 in
         case-manifest-v1.toml; verify_r3.py compares its expansion to
         that field and cross-checks the total echoed in
         CASE-MATRIX-v1.md; stale numbers are gated out. The expansion
         is now precisely labeled a RECIPE-SLOT count: content identity
         (edit_start, edit_end, inserted_sha256) exists only at
         instantiation (R4+), where an unexpected semantic collapse is
         CASE_IDENTITY_COLLISION and stops the stage (MUTATION-v1 §7).

IMPORTANT-3  QUERY semantics made explicit: one QUERY case per corpus =
         ONE ordered batch of three NODE_PATH_AT subqueries (generic
         EARLY/MIDDLE/LATE), result = the ordered tuple of paths
         (NORMALIZED-RESULT-v1 §4 single owner). One CaseId (query has
         no edit fields — R1 validation), no new CaseKeyV1 fields, no
         per-anchor CaseIds. Batch must be measured consistently across
         horses; R9 may derive per-subquery statistics only by explicit
         definition there.

IMPORTANT-4  Tie-breaking frozen (MUTATION-v1 §2.1): minimum distance
         wins; distance tie -> lower byte offset; remaining tie ->
         source-order first ("first at/after" is source-order by
         nature). Under this default every §5 selection is a total
         deterministic function selecting exactly one edit per
         applicable corpus.
```

Honest supersession note: the FIRST pass's IMPORTANT-4 "fix" (select the
fence's LAST body line and insert a closer before its LF) was itself
defective — it introduced the inconsistency MAJOR-3 describes and would
have re-frozen the orphaned closer as an unclosed new opener. The
corrective supersedes it entirely.

Regressions added to verify_r3.py (in-memory recipe units ONLY — no
corpus generation, no receipts, no parser, no timing): unit-variant
byte sizes for all 16 declared variants (incl. both fence units = 512),
deep list/quote depth-16 proofs, fence-close executability + release
simulation, generic-anchor dephasing + landing checks (3 sizes × 3
classes), doc-number extraction binding CORPUS-v1.md formulas to the
mirror, totals-agreement and stale-total gates. Mutation-tested: an
injected 41-count body and an injected +9 anchor constant each fail the
gate; restored state passes.

Final adversarial self-review on the repaired surfaces only (exact-byte
math, actual deep depth, fence-close semantics, phase locking,
applicability, query identity, totals consistency): no further MAJOR or
IMPORTANT findings. Known-and-documented properties (not defects):
generic anchors always land on variant units (consequence of raw
anchors being multiples of 65536 — CORPUS-v1 §5), and they share one
intra-unit offset across anchor classes (deliberate, §2).

Corrective verification results:

```text
git diff --check:            clean
bash scripts/verify-r3.sh:   R3 FREEZE GATE: PASS (43 fixtures; recipe-
                             slot expansion 370 == expected_unique_cases;
                             corrective regressions green)
bash scripts/verify-r1.sh:   R1 ACCEPTANCE GATE: PASS (no benchmark run)
Scope gates:                 no .rs/.c/.mbt under grammar/ corpus/
                             mutations/ cases/; horse-outcome grep clean
```

Corrective verdict:

```text
R3 SELF-ASSESSMENT VERDICT: READY_FOR_FINAL_R3_REVIEW

After human final review the gate became:
    GRAMMAR_CORPUS_MUTATION_FREEZE_PASS
    READY_FOR_R4_H0_REFERENCE_FULL_REBUILD

Human final review: PASSED (2026-09-17). PR #28 merged into master as
17f6040b21f59f452fa57d67b512d0f4858f431f. No frozen artifact changed in
this closure; only this gate note was added.
```
