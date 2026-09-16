# Prior-art mechanism record — mizchi/markdown

Status: R2 prior-art mechanism extraction for MARKIT-MARKDOWN-BENCHMARK-1 (#22).
Retrieval date: 2026-09-16. Evidence rules per
`research/benchmarks/markdown-ast-update/protocol/R0-METHODOLOGY.md` §2.
Labels: OBSERVED (cited file:line at the pinned commit), INFERRED (code-derived,
not executed), HYPOTHESIS (unproven), UNKNOWN (not determined).

## 1. Identity

- PROJECT: mizchi/markdown — "CST-based incremental Markdown parser for
  JavaScript/MoonBit" (README.md:3). MoonBit package name is literally
  `mizchi/markdown` (moon.mod:2); npm package `@mizchi/markdown`
  ("incremental markdown parser"). This is the R0 §2 subject "mizchi/markdown".
- IMPORTANT NAMING CORRECTION: the hypothesized URL `github.com/mizchi/md`
  does NOT exist (`git ls-remote`: "Repository not found", checked 2026-09-16),
  nor does `github.com/mizchi/markdown`. The actual repository is
  **https://github.com/mizchi/markdown.mbt** (found via the npm package's
  repository field; search trail: npm registry query "@mizchi markdown" ->
  `@mizchi/markdown` -> repo URL). It is **MoonBit, not Rust** — the "Rust
  parser with WASM/JS bindings" description in the tasking is wrong for this
  project. Language split: MoonBit core (`src/`, moon.mod:13
  `preferred_target = "js"`, targets js+wasm+wasm-gc+native), JS/TS binding
  layer (`js/api.js`, `js/api.d.ts`, MoonBit FFI in `src/api/exports.mbt`),
  plus a native CLI (`src/cmd/mmmd-native/`).
- VERSION/SHA (pinned): master commit (main HEAD at retrieval)
  `ffe7dc00e60e6778e6407fb0c9bbfe9f202fea94`. Newest git tag is `v0.8.1`
  (`a9679fadfc8f4b4a688554da12425cb0707aee81`); the latest npm release 0.8.3
  (2026-08-30) is untagged and corresponds to `b574e7f` ("Release 0.8.3",
  parent of the pinned commit). moon.mod declares `version = "0.8.3"`.
- LICENSE: MIT (LICENSE:1-3; moon.mod `license = "MIT"`).
- PRIMARY SOURCES (hierarchy: repository source at pinned commit):
  - `src/incremental.mbt` — the entire incremental mechanism (458 lines).
  - `src/block_parser.mbt` — clean full parse (`parse()`), block model.
  - `src/types.mbt` — CST types, spans.
  - `src/incremental_test.mbt` — behavioral specification of the update path.
  - `src/api/exports.mbt`, `js/api.d.ts` — public incremental API surface.
- SECONDARY SOURCES (author's own design documentation, orientation):
  - `README.md` (official design/intent; performance table is a vendor claim).
  - `docs/markdown.md` (architecture doc; partly ASPIRATIONAL — see §14).
  - `CLAUDE.md:53` ("Re-parses only changed blocks, reuses before/after").
  - Third-party inspiration cited by the author: "CRDTs Go Brrr"
    (README.md:9) — orientation only.
- RELEVANT FILES/FUNCTIONS:
  - `src/incremental.mbt:10-14` `EditInfo {offset, old_len, new_len}`
  - `src/incremental.mbt:49-139` `parse_incremental(old_doc, old_source,
    new_source, edit)` — the whole mechanism
  - `src/incremental.mbt:145-186` `find_affected_range` (damage detection)
  - `src/incremental.mbt:190-422` `adjust_spans` / `shift_list_item_spans` /
    `shift_block_span` (recursive suffix span shifting)
  - `src/incremental.mbt:426-446` `Block::get_span`
  - `src/block_parser.mbt:33-40` `parse()`; `:443-495` `parse_document()`;
    `:1175-1227` `insert_blank_line_nodes` / `blank_lines_node`
  - `src/api/exports.mbt:212-268` handle-based JS incremental entry points
  - `src/bench_incremental.mbt` — upstream incremental benchmark exists
    (middle-paragraph edits in 10/50/100-paragraph docs); its numbers are
    vendor claims and are NOT recorded here as evidence.

## 2. Problem actually solved

Real-time Markdown editing (editor/playground product): after a text edit,
produce a new valid CST without a full clean parse, by re-parsing only the
affected top-level block region and reusing the rest (README.md:12;
CLAUDE.md:53; docs/markdown.md §5). Secondary product goals: CommonMark 0.31.2
conformance of the underlying full parser (README.md:10,395-402), a
source-oriented CST with byte spans and preserved markers/trivia for
editor use (README.md:11; types.mbt:83-115, types.mbt:208-343).

The incremental path is exposed API-first: `createDocument` / `doc.update` /
`insertEdit` (js/api.d.ts:185,293,301) over MoonBit FFI handles
(`src/api/exports.mbt:212-268`). OBSERVED: no in-repo caller other than the
API layer and tests invokes `parse_incremental`; the bundled frontend editor
uses a separate literal-render patch path
(`frontend/editor/literal-markdown-editor.ts:243-251`).

## 3. Retained representation

OBSERVED:
- The full CST itself is the retained state: `Document { frontmatter,
  children: Array[Block], definitions: Array[LinkDefinition], span }`
  (types.mbt:155-162). Every `Block` carries a document-global `Span{from,to}`
  of Int offsets (types.mbt:14-17) plus `leading_trivia`/`trailing_trivia`;
  markers are preserved (fence char/length, list marker, closing hashes,
  types.mbt:208-343). Blank-line runs are first-class top-level `BlankLines`
  blocks, inserted post-parse so the children sequence tiles the document
  (block_parser.mbt:1175-1227).
- No per-block hash, no per-block version, no separate block table. Damage
  detection works purely on old-tree spans + the caller-supplied `EditInfo`.
- The JS handle layer additionally retains the old source string per document
  handle (`has_source`, `src/api/exports.mbt:212-220`), because
  `parse_incremental` needs `old_source.length()` as the right-edge fallback
  (incremental.mbt:70-74).
- Nested structure (blockquote children, list items and their children,
  directive/alert children) lives inside one top-level block value
  (types.mbt:284-342); there is no separate retained state for it.

## 4. Reuse unit

OBSERVED: the top-level `Block` (including `BlankLines`). Reuse is
**PARSER-WORK reuse, not representation/object reuse**:
- Prefix blocks pass through unchanged: `final_blocks.push(old_blocks[i])`
  (incremental.mbt:112) — the old block values flow into the new children
  array as-is.
- Suffix blocks do NOT pass through: `final_blocks.push(
  shift_block_span(old_blocks[i], delta))` (incremental.mbt:125).
  `shift_block_span` (incremental.mbt:217-420) constructs a NEW `Block`
  value for every variant with a shifted document-global span, recursively
  reconstructing container children (`adjust_spans`, :190),
  list items (`shift_list_item_spans`, :200), and attributed blocks.
  Suffix syntax parsing is avoided, but every suffix block value is
  structurally re-coordinated/reconstructed.
- Accordingly `reused_before / reused_after` mean "blocks not re-parsed"
  (`reused_after_count = old_blocks.length() - after_idx`,
  incremental.mbt:123,137) — they do NOT mean the result representation
  shares old structure. This record makes no claim about MoonBit pointer
  identity beneath the language's value semantics (not proven here).
- Inside reconstructed suffix blocks, leaf content fields pass through
  unshifted — e.g. `Block::Paragraph(children~, ...)` keeps its inline
  children untouched (incremental.mbt:251-256); only block-level and
  nested-block/list-item spans are shifted. INFERRED consequence: inline
  spans inside reused suffix blocks are stale by `delta` when
  `delta != 0`; the upstream deep-span test asserts block/list-item spans
  only (incremental_test.mbt:205-252), so upstream tests do not cover it.
- Nested blocks and inline nodes are never reused independently of their
  enclosing top-level block; inline parsing always re-runs inside the
  re-parsed region (region is parsed by the full `parse()`,
  incremental.mbt:87). Counters are returned to the caller
  (incremental.mbt:40-45, 114, 120, 123-138).

## 5. Damage detection / invalidation

OBSERVED (incremental.mbt:145-186, `find_affected_range`):
- Input is the caller-supplied `EditInfo {offset, old_len, ...}`; the edit
  byte-range in old coordinates is `[offset, offset+old_len)`.
- A single linear scan over the OLD top-level blocks: a block is affected iff
  `span.to > edit_start && span.from < edit_end` (overlap test,
  incremental.mbt:154). Returns `(first_affected_idx, first_unaffected_idx)`.
- If no block overlaps (edit in a blank gap or at a boundary), a second loop
  maps the position to the enclosing gap: the block whose `span.from >
  edit_start` sets `(i, i)`, the block whose `span.to >= edit_start` sets
  `(i, i+1)` (incremental.mbt:168-184).
- There is no hash/content verification of any kind: the mechanism trusts
  `EditInfo` and the old spans completely; nothing validates that
  `new_source == old_source` with that edit applied (absence observed across
  incremental.mbt).

## 6. Restart rule

NOT_APPLICABLE in the H4 sense (verify result: confirmed). There is no
retained parser-state checkpoint, no choice among multiple restart points, and
no parser-state carried across top-level blocks. The region re-parse
(incremental.mbt:81-87) is a fresh clean `parse()` of the extracted region —
a trivial "restart at a block boundary", but its boundary choice is derived
from old spans only (§5), not from any state-convergence consideration. The
CommonMark line-driven parser itself maintains no cross-document state
(block_parser.mbt:1-11,176-199) except link reference definitions, which the
mechanism handles by fallback (§10), not by restart.

## 7. Convergence rule

NOT_APPLICABLE (verify result: confirmed). No convergence detection exists:
after the region re-parse, suffix syntax parsing is skipped without a
semantic boundary-validity check — there is no check that the region's
last block is semantically independent of the following block (no
fence-still-open check, no setext/lazy-continuation check, no tightness
re-derivation across the boundary) — while suffix block values are
re-coordinated/reconstructed through `shift_block_span` (§4/§8).
Semantics that could
propagate forward are handled either by the region boundary construction
(prev block end -> next block start, incremental.mbt:62-78) or — for link
reference definitions only — by falling back to a full parse (§10). Everything
else is assumed to converge at top-level block boundaries.

## 8. Reconstruction

OBSERVED (incremental.mbt:81-139):
1. Compute the reparse region: `reparse_start` = end span of the block before
   the affected range (or frontmatter end / 0), `reparse_end_old` = start span
   of the first block after the affected range (or `old_source.length()`);
   `reparse_end_new = reparse_end_old + (new_len - old_len)`, clamped
   (incremental.mbt:62-84). The region therefore includes the blank-line runs
   adjacent to the affected block(s) but never any content of a reused block.
2. `parse(reparse_region)` — full block+inline parse of the region
   (incremental.mbt:87), so region-internal structure (containers, inlines)
   is rebuilt from scratch.
3. Fallback check: if `old_doc.definitions` or region definitions are
   non-empty, return a full clean parse of the whole document (§10).
4. Splice: prefix blocks passed through unchanged (incremental.mbt:112) +
   region blocks (spans shifted by `reparse_start`, incremental.mbt:105,
   190-196) + suffix blocks pushed as NEW values reconstructed by
   `shift_block_span` with block/nested-block/list-item spans shifted by
   `delta` (incremental.mbt:121-126, 217-420). The splice therefore
   separates parser-work reuse (suffix syntax parsing skipped) from
   representation reuse (only the prefix is passed through; every suffix
   value is rebuilt).
5. New `Document` reuses old `frontmatter` and old `definitions`,
   `span = (0, new_source.length())` (incremental.mbt:127-132).

## 9. Position / range maintenance

OBSERVED:
- Coordinate system: MoonBit `String` indexes — UTF-16 code units on the JS
  target (entity scanning over `UInt16` units, entity.mbt:60-66; the README's
  SIMD section states offsets are UTF-16 code units with "exact" fallback,
  README.md:375-379). These are NOT UTF-8 byte offsets; a #22 mechanism model
  must re-express the contract in UTF-8 bytes per R0 §6.
- All spans are document-global; there are no block-local coordinate spaces.
  Region-parse results are 0-based and get a constant `reparse_start` added;
  suffix blocks get a constant `delta` added recursively (§8.4).
- Maintenance is eager and proportional to the suffix: every block after the
  edit is structurally rebuilt (`shift_block_span` constructs new block
  values/arrays), incremental.mbt:217-420. No lazy offset propagation, no
  interval tree, no per-block local coordinates.

## 10. Fallback

OBSERVED, explicit and total for one semantic class (incremental.mbt:89-101):
if the old document contains ANY link reference definition, or the re-parsed
region produces ANY, `parse_incremental` discards the incremental work and
returns a full clean parse of the whole document (counters zeroed,
`reparsed = full document block count`). The code comment names the reason:
reference definitions are document-scoped and the definition may live outside
the re-parsed region (incremental.mbt:89-92). Both directions are tested
(deleting a definition unresolves reused references, adding one resolves
them — incremental_test.mbt:175-202).

NOT HANDLED (no fallback, silent): other cross-boundary semantics. The region
right edge is cut at the first reused block's start, so the region parser
cannot see context that would change a reused suffix block. INFERRED
code-derived divergence cases (not executed; each follows from the cited
lines):
- Paragraph merge by newline deletion: deleting the blank line between two
  paragraphs (edit overlaps the `BlankLines` block) yields region text that
  ends before the second paragraph's text, so the merge is invisible; the
  splice keeps two paragraphs where a clean parse produces one
  (incremental.mbt:62-84 boundary construction; `BlankLines` blocks,
  block_parser.mbt:1175-1227; spans include the trailing newline,
  block_parser.mbt:1134,1157 + read_line at :861-876).
- Unclosed-fence forward propagation: an edit that opens a fence inside the
  region parses it as closed-at-region-end; the following reused blocks stay
  in the tree, while a clean parse swallows them into the fence (fenced code
  accepts lines to end of document, block_parser.mbt:392-400; region cut,
  incremental.mbt:70-84).
- Block-attribute attachment: `recognize_attributes` attaches an attribute
  paragraph to the PRECEDING block (block_parser.mbt:498-510) and runs inside
  the region parse (block_parser.mbt:483-487); an attribute paragraph whose
  target block is reused outside the region cannot attach. Same class of
  risk for GitHub-alert recognition across the boundary
  (block_parser.mbt:478-482).
There is no counted or guarded fallback for these classes; they would be
silent divergences (see Q10/Q11).

## 11. Correctness authority

OBSERVED: the underlying full parser is vouched by upstream CommonMark/GFM
conformance suites (652/652 claim, README.md:395-402; TODO.md "Current
status"), but the incremental path itself has no oracle of the form
"incremental result == clean full parse". Tests assert: child-count equality
with a full parse (incremental_test.mbt:24-27,47-51), loose counters > 0,
frontmatter preservation (incremental_test.mbt:125-145), rendered HTML after
the definition fallback (incremental_test.mbt:175-202), and deep span
equality for one shifted nested-list suffix (incremental_test.mbt:205-252).
No test covers paragraph merge/split across a deleted blank line, fence
open/close propagation, or attribute/alert boundary cases. The correctness
authority for unchanged blocks is the implicit CommonMark property that
top-level blocks separated by blank lines are context-free — an assumption
the code makes but does not validate, and which link definitions (handled by
fallback) and attributes/alerts (not handled) show is not universally true.
INFERRED: a faithful #22 reproduction should expect self-equivalence failures
exactly in the §10 "NOT HANDLED" classes; that expectation is a hypothesis to
test under the #22 protocol, not an upstream-admitted defect.

## 12. Known limitations

Evidence-backed:
- Link reference definitions anywhere force a full parse — the incremental
  path degenerates to H0 for documents that use them (incremental.mbt:89-101).
  OBSERVED (code), frequency on real corpora UNKNOWN here.
- Edits inside frontmatter: `frontmatter` is always copied from the old
  document (incremental.mbt:128) and `find_affected_range` only scans blocks;
  INFERRED: frontmatter edits leave a stale frontmatter node and a mis-derived
  region.
- No validation of `EditInfo` against the actual text change (§5).
- Upstream test oracle for incremental updates is weak (child counts, §11).
- TODO.md records that nested list-item span shifting during incremental
  reuse had to be explicitly implemented ("Shift nested ordered/unordered
  list-item and child-block spans during incremental reuse", Completed or
  removed) — i.e., suffix-shift completeness was a discovered, fixed gap, and
  `shift_block_span` (incremental.mbt:217-420) enumerates every block kind by
  hand, so new block kinds must remember to extend it. INFERRED: this is a
  maintenance hazard class, not a current defect.
- README/docs claim editor-grade incremental editing; OBSERVED that the
  bundled frontend editor does not call the incremental API (§2). HYPOTHESIS:
  the incremental path is currently API/demo-grade rather than
  production-editor-grade.
- docs/markdown.md §5/§10/§11 (direct CST patching, ReferenceIndex incremental
  update, nodeId stability) is ASPIRATIONAL design documentation; no
  ReferenceIndex or nodeId machinery exists in the pinned tree (searched;
  definitions are a flat array, types.mbt:158-161). Do not cite the design
  doc as describing the shipped mechanism.
- For R3/R8: HYPOTHESIS — per-block hashes plus a fence/container state
  checksum could replace the definition full-fallback with a targeted
  re-resolution pass; HYPOTHESIS — lazy suffix shifting (per-block base
  offset) would remove the O(suffix) rebuild but changes the position
  contract.

## 13. Adversarial hypotheses

All HYPOTHESIS unless marked otherwise (not executed; code-derived):
- Q1 old info: Damage detection uses only old-tree spans + caller-supplied
  EditInfo (§5); no content hashes. Any stale span (e.g., stale frontmatter,
  §12) or untruthful edit silently corrupts the result.
- Q2 who vouches: For unchanged-block validity, nobody in-repo; only the
  implicit block-independence assumption (§11). Upstream vouches the full
  parser, not the update path.
- Q3 maximally invalidating edit: Adding/deleting any link reference
  definition -> full parse (OBSERVED, incremental.mbt:89-101).
- Q4 tiny edit -> O(N): Yes: `find_affected_range` is O(B)
  (incremental.mbt:150), suffix span-shifting structurally rebuilds every
  suffix block (incremental.mbt:124-126,217-420), and definitions present
  make it a full parse (incremental.mbt:93-94).
- Q5 far-forward propagation: INFERRED broken for unclosed fences crossing
  the region right edge (§10); setext underline insertion is covered only
  when the preceding block is inside the region. Most likely
  false-convergence family of this design.
- Q6 parse saved vs reconstruction paid: INFERRED — for edits near the
  document start, paid reconstruction (suffix rebuild) approaches the cost of
  rebuilding the whole AST (parsing excluded); parse work is traded for
  span-maintenance work with no amortization.
- Q7 position maintenance: Eager global-span shifting, O(suffix nodes) with
  structural copies (§9); no lazy base offsets.
- Q8 memory proportional to N: OBSERVED — retained state is the entire CST
  plus the old source string per handle (§3, exports.mbt:212-220); each
  update allocates a new Document and a new handle (exports.mbt:259-264);
  old handles must be explicitly disposed.
- Q9 hidden state: The `EditInfo` contract itself (§5) — correctness depends
  on an unverified host-side claim. Old source is retained only in the JS
  handle layer, not in the MoonBit Document.
- Q10 false convergence: No convergence checks exist (§7); the §10 INFERRED
  cases (paragraph merge, fence swallow, attribute attachment) would be
  silent wrong trees.
- Q11 fallback frequency: OBSERVED that the only guarded fallback fires for
  any document containing definitions (real-corpus frequency UNKNOWN,
  §12); expected H0-degeneration on definition-bearing payloads. Other
  divergence classes
  have NO fallback (they fail silently instead) — this asymmetry
  (fallback where guarded, divergence where not) is itself a finding.
- Q12 mechanism-intrinsic vs implementation-specific: Intrinsic: block-splice
  reuse unit, region reparse, delta-shift of suffix spans, definition
  fallback. Implementation-specific (do not model as mechanism): MoonBit
  value-semantics structural copies during shifting, UTF-16 coordinates,
  JS handle allocation/dispose policy, `#valtype` EditInfo
  (incremental.mbt:8-14), region `unsafe_substring`.

## 14. Relevance assessment

This project IS the H1 BLOCK_LOCAL_REPARSE anchor named in R0 §3
(protocol line 139) and the anchor claim is substantively fair:
- OBSERVED H1 shape: identify affected top-level block(s) from old spans,
  reparse only the affected region (block + blank-gap context), pass
  unaffected prefix blocks through, repair the document sequence and
  reconstruct suffix values with shifted spans (§4-§9). The reuse unit is
  exactly "block", with B and L as the dominant variables — matching H1's
  stated variables.
- The mechanism is small enough to model faithfully: the entire incremental
  logic is incremental.mbt (458 lines): overlap-based damage mapping, region
  = [prev_block_end, next_block_start) with delta-adjusted right edge,
  region clean-reparse, splice, recursive suffix span shift, definition
  fallback.
- Fidelity boundary — what a #22 H1 model would NOT reproduce:
  UTF-16 code-unit coordinates (R0 §6 mandates UTF-8 bytes); counting
  upstream `reused_*` as representation reuse — they count parser-work
  reuse ("not reparsed") only, and a faithful H1 model must keep
  nodes_reused / nodes_rebuilt / metadata-touched / parser-bytes-inspected
  as DISTINCT facts (R0 §5 forbids identity in the correctness contract);
  the JS handle/source-retention layer (§3); vendor tuning of the full
  parser (SIMD scanning etc. — R0 §2 REFERENCE_ONLY, §4 parity rules).
- Modeling decisions #22 must make explicit (hypothesis-level):
  1. Whether to reproduce the definition full-fallback as part of H1-mizchi
     (faithful) or to allow a reference-resolution pass instead (a
     *different* mechanism). Faithful modeling predicts H1-mizchi collapses
     to H0 on REFERENCE_FANOUT and any reference-bearing corpus slices —
     a strong, testable H1 weakness claim originating from prior art.
  2. Whether silent-divergence classes (§10) are in scope: under the #22
     oracle `normalize(update) == normalize(H0(post-edit))` they are exactly
     the WRONG_RESULT cases that make this prior art interesting for the
     Weakness Map (block-boundary mutation family).
  3. docs/markdown.md's Lezer-style repair/ReferenceIndex/nodeId story must
     NOT be attributed to this codebase (aspirational, §12); the prior-art
     anchor is the simpler shipped splice mechanism.

## 15. Manifest entry

```toml
[[prior_art]]
id = "mizchi-markdown"
name = "mizchi/markdown (@mizchi/markdown, repo markdown.mbt)"
source_type = "repository-source prior-art mechanism record"
repo_url = "https://github.com/mizchi/markdown.mbt"
commit_sha = "ffe7dc00e60e6778e6407fb0c9bbfe9f202fea94"  # main HEAD at retrieval; includes "Release 0.8.3" (b574e7f)
tag = "none for 0.8.3; newest tag v0.8.1 = a9679fadfc8f4b4a688554da12425cb0707aee81"
retrieved_date = "2026-09-16"
language = "MoonBit (js+wasm+wasm-gc+native); JS/TS bindings"
license = "MIT"
relevant_paths = [
  "src/incremental.mbt",
  "src/incremental_test.mbt",
  "src/block_parser.mbt",
  "src/types.mbt",
  "src/api/exports.mbt",
  "js/api.d.ts",
  "src/bench_incremental.mbt",
  "docs/markdown.md",
]
notes = "H1 BLOCK_LOCAL_REPARSE anchor (R0 protocol line 139); MoonBit core with JS/TS bindings (tasking correction: not Rust). Mechanism: EditInfo offset arithmetic over old top-level block spans (incl. BlankLines blocks); region = [prev block end, next block start + delta); clean reparse of region; splice = prefix blocks passed through unchanged + suffix blocks reconstructed as new values with shifted spans (PARSER-WORK reuse; reused_* counters mean not-reparsed, NOT object-identity reuse); TOTAL full-parse fallback whenever link reference definitions exist (old or new). No hashes, no restart/convergence machinery, no EditInfo validation. Inferred silent-divergence classes: paragraph merge via blank-line deletion, unclosed-fence forward propagation, block-attribute attachment across the region boundary; inline spans inside reconstructed suffix blocks are not shifted (test-coverage gap). docs/markdown.md ReferenceIndex/nodeId content is aspirational, not shipped. README benchmark numbers are vendor claims only."
```
