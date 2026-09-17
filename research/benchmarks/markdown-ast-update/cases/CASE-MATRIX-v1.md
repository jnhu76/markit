# CASE-MATRIX-v1 — frozen first-round case matrix (R3)

Status: **R3 FREEZE — CORRECTIVE-1 APPLIED — READY_FOR_FINAL_R3_REVIEW**
Authority: R0 §8–§9 (operations/shapes/sizes) + this file. Single owner of
which cases exist, why each exists, and the R2-hypothesis coverage map.
`cases/case-manifest-v1.toml` is the machine-readable projection; this
file wins on narrative disagreement, and `expected_unique_cases` in that
manifest is the single machine-readable total.

The full Cartesian product (8 shapes × 3 sizes × ops × edit sizes ×
positions × recipes) is deliberately NOT instantiated — most of it would
be meaningless duplication. The matrix is three targeted blocks plus one
core baseline. Every case has a stated reason. Frozen total:
**expected_unique_cases = 370** (recipe-slot expansion after the 7
declared slot overlaps; §6). Cases multiply by mechanisms and lanes at
measurement time only.

Case identity is R1's `CaseKeyV1` exactly — mapping frozen in
`mutations/MUTATION-v1.md` §7. No second ID scheme.

---

## 1. Dimensions (frozen)

```text
shape        plain | many_blocks | huge_block | deep_container |
             inline_dense | fence_heavy | reference_fanout | mixed
size         64k | 1m | 16m                    (exact bytes)
operation    full_parse | insert | delete | replace_eq | replace_grow |
             replace_shrink | structural_edit | query
edit_size    tiny | small | medium                  (generic ops only)
position     early | middle | late                  (generic ops only)
family       local_text | block_boundary | container_state |
             forward_state | inline_delimiter_state | semantic_dependency
mutation_id  M-* (structural_edit only)
seed         GLOBAL_SEED (constant for corpus-gen-v1; recorded in keys)
applicability APPLICABLE | NOT_APPLICABLE (explicit, never implicit)
```

## 2. Block A — mandatory core (reason: coverage floor)

Every corpus (all 24) gets exactly three cases:

```text
A1  FULL_PARSE        zero-edit baseline per corpus (also the R6
                      state-construction cost surface, R2-H08)
A2  QUERY             ONE ordered batch of three NODE_PATH_AT subqueries
                      (generic EARLY/MIDDLE/LATE anchors) in ONE case
                      per corpus (identity: query has no edit fields;
                      payload/result contract: NORMALIZED-RESULT-v1 §4)
A3  INSERT-TINY-MIDDLE the minimal mutation, present on every corpus so
                      every shape/size pair has at least one update case
```

Count: 24 × 3 = **72 cases**.

## 3. Block B — generic edit grid (reason: op × size × position semantics)

Full cross on three structurally distinct payloads at 1 MiB:

```text
shapes: mixed (representative mixture), plain (structure-free control),
        fence_heavy (raw-region control — edits land inside fence bodies
        and on fence lines)
ops × edit_size × position = 5 × 3 × 3
```

Count: 3 × 45 = **135 cases**. Reasons: operation byte-semantics across
sizes and positions (R7 surface); same-size edit inside a fence body vs
inside a paragraph is the R2-H05 locality probe; PLAIN anchors the
structure-free baseline; each grid case is also a correctness-oracle
differential point.

## 4. Block C — scaling slice (reason: locality vs document size)

```text
shapes: many_blocks, huge_block      sizes: 64k, 16m
cases per pair: insert-tiny-early, insert-tiny-middle,
                replace_eq-medium-middle
```

Count: 2 × 2 × 3 = **12 cases**. Reasons: tiny-edit-early in a large
document is the R2-H11 reconstruction-vs-parser-work probe; EARLY vs
MIDDLE isolates restart/reuse distance from edit content; HUGE_BLOCK
makes the affected-region length L dominate, MANY_BLOCKS makes boundary
count B dominate.

## 5. Block D — structural targeted matrix (reason: six families)

Every §5 recipe in `mutations/MUTATION-v1.md`, applied to its
`applicable_shapes` × all three sizes × its frozen anchors. Explicit
NOT_APPLICABLE decisions are part of the matrix (examples):

```text
M-BB-PARA-MERGE on fence_heavy            -> NOT_APPLICABLE (no blank-
                                             separated paragraphs)
M-BB-PARA-MERGE on huge_block-64k         -> NOT_APPLICABLE (single block)
M-CS-ITEM-INDENT on plain                 -> NOT_APPLICABLE (no lists)
M-FS-FENCE-OPEN on fence_heavy            -> NOT_APPLICABLE (nothing
                                             outside fences to swallow)
M-SD-DEF-REPLACE on plain                 -> NOT_APPLICABLE (no definitions)
M-LOC-TEXT on many_blocks                 -> NOT_APPLICABLE (runs < 32 B)
```

Structural recipe counts (shape × size × anchor, per the manifest):

```text
M-LOC-TEXT 12   M-LOC-UTF8-SWAP 21   M-BB-PARA-SPLIT 15
M-BB-PARA-MERGE 17 (huge_block-64k NOT_APPLICABLE)
M-CS-ITEM-INDENT 6   M-CS-BQ-NEST-LINE 6
M-FS-FENCE-OPEN 42 (7 shapes × 3 sizes × {early, middle})
M-FS-FENCE-CLOSE 6  M-IDS-EMPH-INSERT 6  M-IDS-CODE-DELIM 6
M-IDS-LINK-DELIM 9  M-SD-DEF-REPLACE 6   M-SD-DEF-DELETE 6
```

Count: **158 cases**.

## 6. Totals (single authority: `expected_unique_cases`)

```text
raw block expansion:   core 72 + grid 135 + scaling 12 + structural 158
                       = 377 recipe slots
declared slot overlaps: 7 — the core INSERT-TINY-MIDDLE case is the same
                       recipe slot as the generic-grid case for
                       mixed/plain/fence_heavy @1m and the scaling-slice
                       case for many_blocks/huge_block @64k/16m (same
                       corpus, same operation, same edit bytes by
                       construction — one case, counted once)
unique case total:     370  ==  expected_unique_cases in
                       cases/case-manifest-v1.toml
```

Precision (frozen wording): the static expansion counts RECIPE SLOTS.
Content-level CaseKey identity — `(edit_start, edit_end,
inserted_sha256)` — does not exist in R3, because corpora are not
generated until instantiation (R4+); `expected_unique_cases` is
therefore a recipe-slot count, not a content-addressed count. The two
notions coincide for the 7 declared overlaps because those slots build
byte-identical edits by construction; any OTHER collapse is unexpected.
Frozen instantiation rule: if two semantically distinct frozen cases
ever produce the same `CaseKeyV1` at instantiation time, that is a
`CASE_IDENTITY_COLLISION` and stops the stage — no ad-hoc identifiers
(MUTATION-v1 §7).

Measurement-time expansion (mechanisms × lanes × iterations) is R0/R7/R8
business and intentionally not frozen here.

## 7. R2 hypothesis coverage (test opportunity, never predicted outcome)

Per R3 rule §17: every R2 hypothesis is classified. "COVERED" means the
frozen workload contains cases that will exercise the hypothesis in a
later stage (R6–R11); it does NOT predict any result.

```text
R2-H01  mizchi definition-presence total fallback
        -> COVERED. M-SD-DEF-REPLACE / M-SD-DEF-DELETE on
        reference_fanout + mixed; fallback-to-full counter (R8).
R2-H02  paragraph-merge right-neighbor divergence
        -> COVERED. M-BB-PARA-MERGE; correctness oracle is the judge.
R2-H03  unclosed-fence forward reinterpretation
        -> COVERED. M-FS-FENCE-OPEN (early+middle); oracle (R8).
R2-H04  container edits void reuse beyond the edited bytes
        -> COVERED. M-CS-ITEM-INDENT / M-CS-BQ-NEST-LINE on
        deep_container; unique-bytes-inspected PA (R7/R8).
R2-H05  block-kind-dependent convergence distance
        -> COVERED. Generic grid on fence_heavy vs plain (same-size edits
        inside fence bodies vs paragraphs), R7.
R2-H06  GLR-version reuse suppression intervals
        -> DEFERRED_TO_LATER_STAGE. Requires parser-error/ambiguity
        constructs that BENCH-GRAMMAR-v1 deliberately lacks (total,
        deterministic grammar). Revisit only via a protocol amendment
        after the first round.
R2-H07  serialized-state propagation distance
        -> COVERED. CONTAINER_STATE recipes on deep_container +
        restart/convergence distance counters (R8).
R2-H08  retained-state fixed cost vs K clean parses crossover
        -> COVERED. Core FULL_PARSE on all corpora across 64k..16m is
        the R6 state-construction surface.
R2-H09  position-strategy classes (emit/delta/patch/derive)
        -> COVERED. Generic grid + scaling slice supply the
        metadata-touched vs reuse workloads (R7/R9); classes are
        implementation choices to be declared at R5 parity review.
R2-H10  byte-local false convergence re-contexts identical bytes
        -> COVERED. M-FS-FENCE-OPEN and M-CS-* re-context unchanged
        bytes; oracle + convergence counters (R8).
R2-H11  parser-work reuse vs reconstruction cost
        -> COVERED. Scaling slice (tiny-early on 16MiB corpora) with
        nodes_rebuilt / nodes_reused / bytes-inspected kept separate
        (R7/R11 attribution).
R2-H12  completion semantics / lazy work hiding
        -> DEFERRED_TO_LATER_STAGE. This is the R4/R5
        EAGER_COMPLETION_VALIDATION_PASS protocol gate; no workload can
        test it, so R3 freezes no case for it (recorded, per the R2
        hypothesis' own reading).

covered 10 · deferred 2 · out_of_scope_first_round 0
```

## 8. Determinism and ordering

Anchors (including the +7 dephasing and snapping, MUTATION-v1 §2),
selection rules (including the §2.1 tie-break default), and edits are
pure functions of `(corpus, recipe, anchor-class)` — the same case
builds byte-identical inputs forever. Case ordering (shuffle, seed) is
R1's frozen machinery and never influences case CONTENT.
