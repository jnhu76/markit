# MUTATION-v1 — frozen operations, edit recipes, structural families (R3)

Status: **R3 FREEZE CANDIDATE — READY_FOR_ADVERSARIAL_R3_REVIEW**
Authority: `protocol/R0-METHODOLOGY.md` §6 (canonical edit, FROZEN), §8
(operation set + six propagation families, FROZEN) + this file (the exact
recipes R3 freezes). Single owner of operation semantics, edit-size
classes, position classes, and structural mutation recipes.
`mutations/manifest.toml` is the machine-readable projection; this file
wins on disagreement.

The canonical edit stays exactly R0's:

```text
[start, end) UTF-8 byte range + inserted UTF-8 bytes
```

Every mutation must preserve valid UTF-8: no edit may split a multibyte
code point, and every range boundary is a char boundary of the OLD
source. Byte length is authority for classification — never character
count (matches `OperationKind::classify` in `common/src/edit.rs`).

---

## 1. Operation set (frozen; unchanged)

```text
FULL_PARSE   parse the old source; no edit.
INSERT       start == end; inserted bytes non-empty.
DELETE       start < end; inserted bytes empty.
REPLACE_EQ   deleted byte length == inserted byte length.
REPLACE_GROW inserted byte length > deleted byte length.
REPLACE_SHRINK inserted byte length < deleted byte length.
STRUCTURAL_EDIT an edit produced by a §5 structural recipe; the operation
             label is DECLARED by the generator (per common::edit, the
             byte pattern alone cannot distinguish structural intent).
             Its byte lengths still satisfy one of the five classes above.
QUERY        NODE_PATH_AT over the initial state — contract frozen in
             `grammar/NORMALIZED-RESULT-v1.md` §4 (single owner).
```

Degenerate edits (removed == 0 && inserted == 0) are forbidden: every
edit changes at least one byte.

## 2. Position classes

Deterministic percentile anchors on the old source (N = its byte length):

```text
EARLY  = floor(N/4)
MIDDLE = floor(N/2)
LATE   = floor(3N/4)
```

Both the requested class and the ACTUAL byte offset are recorded with
every case. Snapping (in order):

```text
1. range edits: anchor = min(anchor, N - L)      (fit the range)
2. snap DOWN to the nearest UTF-8 char boundary
3. structural recipes may re-anchor deterministically (§5); the
   selection rule names the exact scan
```

Structural placement additionally uses these POSITION SLOTS (found by
deterministic scan, offsets recorded):

```text
block start / block interior / block end
container opener / container interior
fence opener / fence body / fence closer
inline delimiter / reference use / reference definition
```

## 3. Edit-size classes

Small, closed set; exact recipes (no fuzzy descriptions):

```text
TINY   L = 3
SMALL  L = 32
MEDIUM one line (the line containing the snapped anchor), capped:
       if that line is longer than 4096 bytes, a 512-byte window
       [anchor, anchor+512) (snapped, clamped) is used instead
```

Generic recipes per (class, operation); insertions are ASCII-only:

```text
INSERT(3|32)          insert "x"×L at the anchor
DELETE(3|32)          delete [anchor, anchor+L)
REPLACE_EQ(3|32)      replace L bytes at anchor with "z"×L
REPLACE_GROW(3|32)    replace L bytes with "z"×(2L)      (3→6, 32→64)
REPLACE_SHRINK(3|32)  replace L bytes with "z"          (→1 byte)
INSERT-MEDIUM         insert "m"×23 + "\n" (24 B) at the START of the
                      anchor's line (at the anchor for capped windows)
DELETE-MEDIUM         delete the anchor's whole line incl. its LF
                      (the capped window for >4096-B lines)
REPLACE_EQ-MEDIUM     replace the line with "z"×(L−1) + "\n" (length L)
REPLACE_GROW-MEDIUM   replace the line with "z"×(L−1) + "\n" + "q"×31 + "\n"
                      (length L+32)
REPLACE_SHRINK-MEDIUM replace the line with "z"×(s−1) + "\n" where
                      s = min(L−1, 16)  (guarantees inserted < deleted)
```

Structural edits are governed by their §5 recipes, not by these sizes.
MEDIUM guarantees "one meaningful local syntactic region": a full line in
regular shapes, a bounded 512-byte window inside HUGE_BLOCK's giant lines.
Generic edits are NOT guaranteed structure-preserving — a line
replacement may rewrite a fence opener; that is intended arbitrary-edit
reality, its damage is whatever BENCH-GRAMMAR-v1 says, and operation
labels remain byte-true.

## 4. R0's six families (frozen mapping)

```text
LOCAL_TEXT              M-LOC-TEXT, M-LOC-UTF8-SWAP
BLOCK_BOUNDARY          M-BB-PARA-SPLIT, M-BB-PARA-MERGE
CONTAINER_STATE         M-CS-ITEM-INDENT, M-CS-BQ-NEST-LINE
FORWARD_STATE           M-FS-FENCE-OPEN, M-FS-FENCE-CLOSE
INLINE_DELIMITER_STATE  M-IDS-EMPH-INSERT, M-IDS-CODE-DELIM, M-IDS-LINK-DELIM
SEMANTIC_DEPENDENCY     M-SD-DEF-REPLACE, M-SD-DEF-DELETE
```

## 5. Structural mutation recipes (frozen)

Common fields every recipe defines: `mutation_id`, family, precondition,
selection rule, canonical edit, postcondition, expected semantic damage
shape, applicable / non-applicable shapes. Recipes record workload
semantics ONLY — which propagation a mutation exercises. They must never
encode performance expectations or predicted winners (R3 §25 check).

### M-LOC-TEXT — LOCAL_TEXT

```text
precondition   a run of >= 32 ASCII letters inside one Text node exists
selection      such a run nearest the MIDDLE anchor; start = run_start +
               floor(run_len/2) - 16, snapped
edit           REPLACE_EQ 32 bytes -> "z"×32          (op: structural_edit)
postcondition  one Text node's bytes change; structure identical
damage shape   none structural — pure text delta (control baseline)
applicable     plain, huge_block, deep_container, mixed
non-applicable many_blocks (14-B runs), inline_dense (delimiter-dense
               lines have no 32-B letter run), fence_heavy (bodies are
               raw fence content), reference_fanout (no 32-B letter
               runs)
```

### M-LOC-UTF8-SWAP — LOCAL_TEXT (byte-authority probe)

```text
precondition   a CJK variant line/region exists
selection      the CJK char (3 B) nearest the MIDDLE anchor, boundaries
               snapped (never splits a code point)
edit           REPLACE_EQ 3 bytes -> "z"×3
postcondition  one CJK char becomes three ASCII bytes inside a Text node
damage shape   none structural; proves byte-length != char-count handling
applicable     plain, many_blocks, huge_block, deep_container,
               inline_dense, fence_heavy (CJK fence bodies),
               reference_fanout
non-applicable mixed (all-ASCII tile by recipe; UTF-8 stress is carried
               by the other seven shapes)
```

### M-BB-PARA-SPLIT — BLOCK_BOUNDARY

```text
precondition   a paragraph with >= 4 B of contiguous plain Text run
               near the anchor
selection      that run's midpoint, snapped
edit           INSERT "\n\n" (2 B)                     (op: structural_edit)
postcondition  one paragraph becomes two at the insertion point
damage shape   one block splits; downstream block offsets shift by 2
applicable     plain, many_blocks, huge_block, deep_container, mixed
non-applicable fence_heavy, reference_fanout, inline_dense (its
               delimiter-dense lines have no >= 4 B plain Text run)
```

### M-BB-PARA-MERGE — BLOCK_BOUNDARY

```text
precondition   two adjacent paragraphs separated by exactly one blank
               line (blank per BENCH-GRAMMAR-v1 §1)
selection      the blank line nearest the MIDDLE anchor
edit           DELETE the 2 bytes "\n\n" (blank line's LF + preceding LF)
postcondition  the two paragraphs merge into one (soft-break join)
damage shape   block-boundary removal; right-neighbor re-interpretation
               (R2-H02's minimal counterexample shape)
applicable     plain, many_blocks, huge_block(1m/16m only), inline_dense,
               reference_fanout, mixed
non-applicable fence_heavy, deep_container (no blank lines),
               huge_block-64k (single block, no interior blank)
```

### M-CS-ITEM-INDENT — CONTAINER_STATE

```text
precondition   a list item line whose previous sibling item shares its
               indent level exists at/after the anchor
selection      the first marker line at/after the MIDDLE anchor satisfying
               the precondition
edit           INSERT "  " (2 spaces) before the marker (after any
               existing indent)                        (op: structural_edit)
postcondition  the selected item (with its subtree) becomes a nested
               list under the previous sibling; following siblings pop
               back to the outer list (BENCH-GRAMMAR-v1 §7 arithmetic)
damage shape   container-state propagation: subtree re-anchoring of the
               item AND reinterpretation of the sibling boundary
applicable     deep_container, mixed
non-applicable plain, many_blocks, huge_block, inline_dense, fence_heavy,
               reference_fanout (no lists)
```

### M-CS-BQ-NEST-LINE — CONTAINER_STATE

```text
precondition   a single-level blockquote line (prefix exactly "> ") not
               inside a fence body exists at/after the anchor
selection      the first such line at/after the MIDDLE anchor
edit           INSERT ">" before the line's existing ">" ("> a" -> ">> a")
postcondition  that line becomes a nested quote one level deeper; its
               neighbors keep their depth
damage shape   container depth change on one line; block-structure
               re-interpretation of the quote's inner content
applicable     deep_container (blockquote mountains), mixed
non-applicable all shapes without blockquotes
```

### M-FS-FENCE-OPEN — FORWARD_STATE

```text
precondition   a non-blank line outside every fence body exists
selection      the first such line at/after the chosen anchor
               (anchors: EARLY and MIDDLE — two case variants, to probe
               propagation distance)
edit           INSERT "```\n" (4 B) at that line's start (before any
               indent)                                 (op: structural_edit)
postcondition  from the inserted opener to the next legal closer (or EOF)
               becomes FencedCode body
damage shape   maximal forward-state propagation: every following block
               is reinterpreted until fence state re-converges
               (R2-H03/R2-H10 counterexample shape)
applicable     plain, many_blocks, huge_block, deep_container,
               inline_dense, reference_fanout, mixed
non-applicable fence_heavy (no structure outside fences to swallow)
```

### M-FS-FENCE-CLOSE — FORWARD_STATE

```text
precondition   a fence with >= 2 body lines before and >= 1 body line
               after the chosen body line
selection      the fence containing the MIDDLE anchor; its first body
               line at/after the anchor
edit           INSERT "\n```\n" (5 B) immediately before that body
               line's terminating LF                   (op: structural_edit)
postcondition  the fence terminates at the inserted closer; the remaining
               body bytes become ordinary document blocks
damage shape   forward-state release: suffix blocks re-enter structure
applicable     fence_heavy, mixed
non-applicable shapes without fences
```

### M-IDS-EMPH-INSERT — INLINE_DELIMITER_STATE

```text
precondition   an emphasis span exists at/after the anchor
selection      the emphasis span nearest after the MIDDLE anchor
edit           INSERT "*" immediately before the span's closing delimiter
postcondition  delimiter re-matching per BENCH-GRAMMAR-v1 §10.2 across
               the affected region (stray literal or re-nesting)
damage shape   inline delimiter state change; bounded by the paragraph
applicable     inline_dense, mixed
non-applicable shapes without emphasis
```

### M-IDS-CODE-DELIM — INLINE_DELIMITER_STATE

```text
precondition   a code span exists at/after the anchor
selection      the code span nearest after the MIDDLE anchor
edit           DELETE 1 byte: the first backtick of its opening run
postcondition  the span cannot close (run-length rule); content plus the
               stray backtick become literal text
damage shape   code-span delimiter state destroyed; inline re-scan
applicable     inline_dense, mixed
non-applicable shapes without code spans
```

### M-IDS-LINK-DELIM — INLINE_DELIMITER_STATE

```text
precondition   an inline or reference link exists at/after the anchor
selection      the link construct nearest after the MIDDLE anchor
edit           DELETE 1 byte: its opening "["
postcondition  the construct decomposes to literal text (link text scan
               fails); for reference links the definition use-count drops
damage shape   inline delimiter failure; touches semantic dependency
               as a secondary effect (primary family recorded here)
applicable     inline_dense, reference_fanout, mixed
non-applicable shapes without links
```

### M-SD-DEF-REPLACE — SEMANTIC_DEPENDENCY

```text
precondition   a reference definition whose label is used by >= 1 link
selection      the definition line nearest after the EARLY anchor
edit           REPLACE_EQ its destination bytes -> "z"×len(destination)
postcondition  every dependent ReferenceLink resolves to the new
               destination (fanout: uses/def is controlled by the
               REFERENCE_FANOUT / MIXED recipes)
damage shape   wide semantic damage with zero structural movement
applicable     reference_fanout, mixed
non-applicable shapes without reference definitions
```

### M-SD-DEF-DELETE — SEMANTIC_DEPENDENCY

```text
precondition   as M-SD-DEF-REPLACE
selection      the same definition-line rule (EARLY anchor)
edit           DELETE the whole definition line incl. LF
postcondition  all dependent links become UNRESOLVED: ReferenceLink
               nodes disappear, literals appear (frozen fallback
               semantics, BENCH-GRAMMAR-v1 §9.3)
damage shape   maximal semantic dependency damage (R2-H01 counterexample
               shape; H1-style fallback pressure)
applicable     reference_fanout, mixed
non-applicable shapes without reference definitions
```

## 6. Mutation validity invariants (frozen)

```text
start <= end <= old_source.len()
start/end on UTF-8 char boundaries of old_source
apply(old_source, edit) == post_edit_source (host-side, byte-exact)
post-edit source: valid UTF-8, valid BENCH-GRAMMAR-v1 input
case deterministic: same corpus + recipe + anchor => byte-identical edit
operation contract valid (inserted/deleted byte relationship matches the
  operation label)
mutation-family preconditions hold at instantiation time; a recipe whose
  preconditions fail on a given corpus is NOT_APPLICABLE (recorded, never
  silently skipped)
where a structural mutation intentionally changes interpretation, the
  post-edit document remains inside BENCH-GRAMMAR-v1 semantics (total
  grammar — always true by construction)
```

## 7. Case identity compatibility (R1 CaseKeyV1)

Every frozen case maps into R1 identity without a second ID scheme:

```text
payload_id       = "CORPUS-v1/" + corpus_id        (e.g. "CORPUS-v1/mixed-1m")
payload_shape    = the corpus shape (snake_case, matches PayloadShape)
payload_size_bytes = the corpus target size (exact, §CORPUS-v1 §2)
old_source_sha256 = SHA256 of the generated old source (receipt)
operation        = full_parse | insert | delete | replace_eq | replace_grow
                   | replace_shrink | structural_edit | query
edit_start/end   = the frozen actual byte offsets (None for
                   full_parse/query — R1 validation enforces this)
inserted_sha256  = SHA256 of the exact inserted bytes (empty insertion
                   hashes the empty string, per R1 MAJOR-4)
generator_id     = "CORPUS-v1"
generator_seed   = 0x4D41524B49542D33
```

Identity is CONTENT-addressed: two recipes that produce byte-identical
edits on the same corpus ARE one case. Rule (frozen): the case manifest
must not contain two rows with the same
`(corpus_id, operation, edit_start, edit_end, inserted_sha256)`; recipe
provenance is a manifest column, never part of identity. Recipe-level
uniqueness is checked statically by `scripts/verify-r3.sh`;
offset-level deduplication happens at case instantiation (R4+), where
offsets first exist. If two SEMANTICALLY distinct cases (different
corpus, operation, or edit bytes) ever produce the same CaseKey, that is
a `CASE_IDENTITY_COLLISION` and stops the stage — no ad-hoc identifiers.

QUERY identity note: QUERY carries no edit fields (R1 `CaseKeyV1`
rejects them for `query`), so exactly one QUERY case exists per corpus
and it answers all three anchors in one case (NORMALIZED-RESULT-v1 §4).
Post-edit querying cannot be expressed in R1 identity and is deferred
(later stage), not silently squeezed into STRUCTURAL_EDIT.

## 8. No horse knowledge (R3 §25 discipline)

These documents state workload semantics ("this mutation exercises
forward-state propagation"). They must never state outcomes (declaring a
winning horse, or that a mechanism ought to fail here). R2 hypotheses MAY
motivate a recipe; predicted results never become authority (see
CASE-MATRIX coverage table for the motivation mapping). The grep gate in
`scripts/verify-r3.sh` enforces the banned phrasings across all R3
artifacts — including this file, which is why this section paraphrases
them.
