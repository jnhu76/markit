# Prior-art mechanism record — MD4C

R2 stage (prior-art mechanism extraction) of MARKIT-MARKDOWN-BENCHMARK-1.
Authority: `research/benchmarks/markdown-ast-update/protocol/R0-METHODOLOGY.md` §2.

Evidence rules: OBSERVED claims cite `file:line` at the pinned version;
INFERRED claims are labeled; UNKNOWN is stated, never guessed. No timing
numbers, no performance claims, no benchmark execution. H0–H4 relevance is
assessed only in §14. This record is deliberately negative-evidence-heavy:
MD4C's primary roles are H0 prior-art anchor and proof that a widely
embedded production parser ships no update mechanism at all.

## 1. Identity

```text
PROJECT:    MD4C ("Markdown for C"; CommonMark 0.31 + opt-in extensions)
VERSION:    release-0.5.3 (latest upstream release tag at retrieval)
COMMIT_SHA: 472c417005c2c71b8617de4f7b8d6b30411d78f4
            (shallow-clone HEAD == tag release-0.5.3; verified via
            git rev-parse HEAD and git rev-parse release-0.5.3^{commit};
            HEAD commit message "Bump version to 0.5.3.")
REPO_URL:   https://github.com/mity/md4c
RETRIEVED:  2026-09-16 (shallow clone into /tmp/markit-r2-prior-art/md4c)
LANGUAGE:   C (single translation unit src/md4c.c, 6462 lines + src/md4c.h,
            407 lines; C standard library only)
LICENSE:    MIT (LICENSE.md; "Copyright © 2016-2024 Martin Mitáš")
```

PRIMARY SOURCES: upstream source at the pinned tag (sole mechanism
authority). SECONDARY SOURCES: `README.md`, `CHANGELOG.md` — identity and
positioning only.

RELEVANT FILES / FUNCTIONS (paths relative to repo root, release-0.5.3):

- `src/md4c.h` — complete public API: `MD_SIZE`/`MD_OFFSET` (`:47-48`),
  `MD_BLOCKTYPE`/`MD_SPANTYPE`/`MD_TEXTTYPE` (`:54-192`), `MD_ATTRIBUTE`
  (`:231-236`), detail structs (`:239-299`), flags (`:306-323`), `MD_PARSER`
  (`:339-383`), `MD_RENDERER` compat typedef (`:388`), `md_parse`
  (`:391-400`).
- `src/md4c.c` — the entire parser: `MD_CTX` (`:162-264`),
  `MD_LINETYPE`/`MD_LINE_ANALYSIS`/`MD_LINE`/`MD_VERBATIMLINE`
  (`:266-302`), `MD_MARK` + flags (`:2527-2564`), `MD_BLOCK`/`MD_CONTAINER`
  (`:4652-4679`), callback macros (`:439-484`), `md_analyze_line` (`:5814`),
  `md_process_line` (`:6280`), `md_process_doc` (`:6356`),
  `md_process_all_blocks` (`:4884`), `md_push_block_bytes` (`:4971`),
  `md_start_new_block` (`:5003`), `md_consume_link_reference_definitions`
  (`:5059`), `md_end_current_block` (`:5104`),
  `md_add_line_into_current_block` (`:5148`), `md_collect_marks` (`:2995`),
  `md_analyze_inlines` (`:4097`), `md_process_inlines` (`:4200`),
  `md_is_link_reference_definition` (`:2166`), `md_lookup_ref_def`
  (`:1878`), `md_build_ref_def_hashtable` (`:1746`), `md_parse`
  (`:6417-6462`).
- `src/md4c-html.c` — `md_html` wrapper (`:532`), ending in
  `return md_parse(...)` (`:571`).
- `README.md` — "There is actually just one function, `md_parse()`" (`:37`);
  push model (`:39`); CommonMark 0.31 claim; users list. `LICENSE.md` — MIT.

## 2. Problem actually solved

OBSERVED: one-shot conversion of a complete, contiguous, in-memory Markdown
buffer into an ordered stream of renderer callbacks (block enter/leave, span
enter/leave, typed text; `src/md4c.h:339-383`, `:391-400`). Push-model batch
parsing: no tree object reaches the caller; the input is one whole buffer —
no chunked/streaming feed either.

OBSERVED (editor vs batch): embedded in interactive applications (README
users list includes QOwnNotes, Textosaurus, Qt), but the API they get is the
same whole-document parse; no editing concept exists in the API.

OBSERVED (what it does NOT do): no incremental update, no edit descriptor,
no prior-state input, no retained AST, no resumable parse. Negative evidence
(2026-09-16, release-0.5.3): `grep -rni "incremental"` over `src/` and
`test/` → zero hits; grep for symbols matching
`md_*(update|refresh|reparse|edit|resume|continu)*` over `src/*.h`+`src/*.c`
→ zero hits; "incremental" over `README.md`/`CHANGELOG.md` → zero hits. The
only exported parse entry points are `md_parse` (`src/md4c.h:400`) and the
wrapper `md_html` (`src/md4c-html.h:59`, delegating at
`src/md4c-html.c:571`).

## 3. Retained representation

OBSERVED (across updates): **nothing**. `md_parse` declares `MD_CTX ctx;` as
a stack local (`src/md4c.c:6420`), zeroes it (`:6431`), and frees every heap
buffer afterwards — ref defs, ref-def hashtable, aux buffer, marks, block
arena, containers (`:6454-6459`). File-scope statics are only `const` tables
(`md_dummy_blank_line` `:5809`; HTML block-name tables `:5413-5418`); no
module-level mutable state exists. A second call on the same bytes
reproduces the same callback stream from scratch.

OBSERVED (transient state DURING one parse — `MD_CTX`,
`src/md4c.c:162-264`): (1) immutable inputs and `doc_ends_with_newline`
(`:165-172`); (2) `ref_defs` array + hash table + `max_ref_def_output`
guard (`:178-184`); (3) `marks` buffer — commented "only used for parsing a
single block contents", reused across blocks (`:186-192`),
`mark_char_map` (`:194-198`), 16 delimiter-opener stacks (`:200-217`),
`ptr_stack` (`:219-222`); (4) table cell boundary queue, unresolved-link
queue, raw-HTML horizons (`:224-237`); (5) `block_bytes` — one flat growable
byte arena of `MD_BLOCK` records each followed by that block's
`MD_LINE`/`MD_VERBATIMLINE` records (`:239-249`) plus `current_block`;
(6) container stack and `code_indent_offset` (`:251-257`); (7) cross-line
flags `code_fence_length`, `html_block_type`,
`last_line_has_list_loosening_effect`,
`last_list_item_starts_with_two_blank_lines` (`:259-263`).

Record shapes: `MD_BLOCK` (bitfield type/flags/data + `n_lines`,
`:4652-4668`), `MD_LINE {beg,end}` (`:291-295`), `MD_VERBATIMLINE`
(`:297-302`), `MD_LINE_ANALYSIS` (`:281-289`), `MD_MARK
{beg,end|pointer, prev, next, ch, flags}` (`:2527-2546`). UNKNOWN: whether
downstream embedders keep MD4C-level state across edits — outside this
repository; MD4C itself retains and accepts nothing.

## 4. Reuse unit

OBSERVED: none. No unit of prior work survives a parse, and no API exists to
supply or retrieve old state; the single entry point takes only raw text and
a callback vtable (`src/md4c.h:400`), so reuse identity, granularity, and
validation cannot exist at this version. Within one parse, marks-buffer and
containers-vector reuse (`src/md4c.c:186-192`, `:4889-4893` — the containers
vector is repurposed in phase 2 as a tight/loose-list tracker) is allocation
reuse, not syntax reuse: no prior result is consulted or compared.

## 5. Damage detection / invalidation

NOT_APPLICABLE. OBSERVED: no input describes an edit; every `md_parse` call
treats its buffer as entirely new source (§2 negative evidence). No code
path compares a new parse against prior structure.

## 6. Restart rule

NOT_APPLICABLE. OBSERVED: parsing is one forward line loop from offset 0 to
end (`src/md4c.c:6367-6373`); no checkpoints, saved scanner states, or
re-entry points exist, and all state is destroyed at return (§3). The
callback-abort path shows this negatively: a non-zero callback return
unwinds the call and discards the context (`src/md4c.h:360-361` documents
the abort; macros `src/md4c.c:439-484`; `md_parse` returns the callback's
value at `:6461`). An aborted parse cannot be resumed — there is no handle
to resume with.

## 7. Convergence rule

NOT_APPLICABLE. OBSERVED: convergence presupposes retained old state to
compare against; MD4C retains none (§3) and never branches on remembered
state. The nearest structural analogue inside one parse is phase ordering
that defers link resolution until the complete reference-definition set
exists (`md_build_ref_def_hashtable` at `src/md4c.c:6377`, after all lines
are analyzed, before any block content is processed) — a document-level
forward semantic dependency resolved by global ordering, not a convergence
test.

## 8. Reconstruction

OBSERVED: the full output is reconstructed from scratch every call, in two
phases (`md_process_doc`, `src/md4c.c:6356-6409`):

- Phase 1 (block/line analysis): one loop calls `md_analyze_line` (`:5814`;
  line classification, container prefix matching against the container stack
  `:5834-5860`, fence continuation via the pivot line `:5873-5893`) and
  `md_process_line` (`:6280`; blank line ends the leaf block `:6287-6291`;
  HR/ATX are single-line blocks `:6297-6305`; Setext underline
  retroactively converts the paragraph into a heading `:6309-6325`; table
  underline retroactively converts it into a table `:6328-6337`; line-type
  change ends the block `:6340-6341`). Blocks/lines append into the flat
  arena (`:4971-5000`, `:5148-5175`). Paragraphs starting with `[` are
  tested at block end and consumed as reference definitions
  (`md_end_current_block` `:5104-5138` → `:5059`).
- Phase 2 (render): after `md_build_ref_def_hashtable` (`:6377`),
  `md_process_all_blocks` (`:4884-4964`) walks the arena linearly, firing
  block callbacks via `MD_BLOCK_CONTAINER_OPENER/CLOSER` flags
  (`:4926-4947`) and processing each leaf block's lines: `md_analyze_inlines`
  (`:4097`: reset marks `:4101-4102`, collect marks over exactly this
  block's lines `:2995`, resolve links `:4108-4112`, emphasis/autolinks
  `:4123`, opener stacks reset `:4145-4146`), then `md_process_inlines`
  (`:4200`) emits text/span callbacks.

The tree exists only as this intermediate flat arena, consumed once and then
logically emptied (`ctx->n_block_bytes = 0`, `:4960`). No incremental
composition exists: the callback stream is always produced whole.

## 9. Position / range maintenance

OBSERVED: internal positions are byte offsets into the caller's buffer
(`MD_OFFSET`/`MD_SIZE` are `unsigned`, `src/md4c.h:47-48`); internal records
carry `beg`/`end` (§3). The callback ABI carries almost no explicit ranges:
block/span enter/leave receive `(type, detail, userdata)` only — no offset
(`src/md4c.h:363-367`). The `text` callback receives a raw pointer into the
caller's original buffer (`MD_TEXT` macro `src/md4c.c:475-484`, invoked as
`MD_TEXT(text_type, STR(off), tmp - off)` at `:4227`; `STR(off)` is
`ctx->text + off`, `:311`) — offsets recoverable only by pointer arithmetic
against input the caller still holds. Explicit offsets appear only in
`MD_BLOCK_LI_DETAIL::task_mark_offset` (`src/md4c.h:256`) and
`MD_ATTRIBUTE::substr_offsets` (relative to the attribute string,
`substr_offsets[0] == 0`, `:224-236`).

Nothing is maintained across parses; there are no persistent positions
because there is no persistent anything (§3). INFERRED: for an incremental
consumer the ABI is a real obstacle — block/span identity must be
reconstructed by the consumer from text pointers, since MD4C never emits
block ranges.

## 10. Fallback

OBSERVED: none exists and none is needed — full parse is the only mode. The
callback abort (§6) is early exit, not degraded parsing. One capacity guard
exists, semantic rather than a parse-mode fallback:
`max_ref_def_output = 16 * MIN(size, 1 MiB/16)` (`src/md4c.c:6439`) budgets
stored reference-definition expansion; once exhausted, further definitions'
expansions are not retained (`:2301`, `:2334-2339`), affecting only link
resolution on pathological inputs. UNKNOWN: whether any embedder wraps
md_parse with chunking or caching — outside this repository.

## 11. Correctness authority

OBSERVED: upstream's correctness authority is the CommonMark 0.31 spec plus
documented extensions (README; flag docs `src/md4c.h:301-335`); the repo
carries its own test tree (`test/`). For this benchmark MD4C has exactly one
execution path, so "full parse from scratch" is not merely the authority —
it is the totality of behavior (H0 identity, trivially). Benchmark
correctness remains the #22 protocol (self-equivalence against H0), not
MD4C.

## 12. Known limitations

Evidence-backed (OBSERVED unless labeled):

1. Any source change, however small, costs a whole-document parse: no update
   path exists (§2, §5 negative evidence).
2. Output is a push event stream, not a retained tree: a consumer wanting an
   AST builds one itself on top of callbacks (`src/md4c.h:339-383`).
3. Transient parse memory is O(document structure): one `MD_BLOCK` per
   block plus one `MD_LINE`/`MD_VERBATIMLINE` per line (`:5148-5175`; 1.5x
   arena growth `:4979-4981`); all freed only at end of parse
   (`:6454-6459`).
4. Offsets are 32-bit `unsigned` (`src/md4c.h:47-48`): documents beyond
   4 GiB are unrepresentable by construction. (OBSERVED type width; no
   overflow demonstrated — no claim of an observed failure.)
5. Reference-definition storage is subject to the `max_ref_def_output`
   budget (§10) — size-capped, input-dependent semantic state.
6. HYPOTHESIS FOR R3/R8: because block/span callbacks carry no ranges (§9),
   an editor needing per-block re-render regions must derive them by
   re-scanning received text chunks — a property of the rendering-oriented
   ABI, not a defect claim.

## 13. Adversarial hypotheses

All items HYPOTHESIS unless an OBSERVED citation is given. With no
cross-edit mechanism, most questions collapse to "everything is recomputed";
the residue is what the single-parse internal state teaches the benchmark
about Markdown's dependency structure.

- Q1 (old info surviving an edit): none across edits — no state outlives a
  call (OBSERVED §3). Within one parse, ref definitions survive from phase 1
  into phase 2 via the hash table (OBSERVED §7/§8).
- Q2 (who vouches validity): across edits, nobody — nothing is retained.
  Within a parse, ref-def lookup validity rests on label normalization
  shared by insert and lookup (`md_lookup_ref_def`, `src/md4c.c:1878-1915`)
  — OBSERVED mechanism, no cross-edit vouching concept.
- Q3 (maximally invalidating edit): every edit invalidates 100% of state by
  construction (OBSERVED — no retained state exists to invalidate).
- Q4 (tiny edit forcing O(N)): trivially yes for every edit — the whole
  document is re-inspected by the single line loop (`src/md4c.c:6367-6373`,
  OBSERVED): the proven H0 cost floor, not a hypothesis.
- Q5 (early edit propagating far forward): OBSERVED dependency structure:
  link spans anywhere may resolve against reference definitions anywhere,
  resolved only after the full scan (`:6377` precedes all inline
  processing). HYPOTHESIS for H1/H4 modeling: an edit to a reference
  definition semantically touches every link using its label — the
  semantic-dependency mutation family should reproduce this fan-out.
- Q6 (parse work saved vs reconstruction): never traded — reconstruction and
  parse are the same work (OBSERVED).
- Q7 (position maintenance erasing reuse): not applicable — no persistent
  positions (§9). HYPOTHESIS: pointer-based text callbacks are the cheapest
  possible position strategy (zero maintenance); an incremental successor
  must pay real memory to beat it.
- Q8 (memory ∝ N): yes during parse (§12.3, OBSERVED); zero retained after
  (OBSERVED §3).
- Q9 (hidden grammar state): OBSERVED — cross-line state a naive block-local
  design would miss: `last_line_has_list_loosening_effect` /
  `last_list_item_starts_with_two_blank_lines` (list tightness depends on
  blank lines between items; `:259-263`, consumed at `:5822`),
  `code_fence_length` (`:260`), `html_block_type` (`:261`), pivot-line fence
  continuation (`:5873-5893`), the container stack (`:5834-5860`), and
  retroactive block rewrites (Setext `:6309-6325`, table underline
  `:6328-6337`, ref-def consumption `:5104-5138`).
- Q10 (false convergence): not applicable — no convergence test exists (§7).
  HYPOTHESIS for H4 modeling: the retroactive rewrites in Q9 are exactly the
  events that would defeat a prematurely placed convergence checkpoint — a
  paragraph is not known to be a paragraph until the next line fails to be
  a Setext/table underline or ref definition.
- Q11 (fallback frequency): not applicable — there is no fallback; full
  parse has frequency 1 by definition (§10).
- Q12 (mechanism-intrinsic vs implementation-specific): OBSERVED split —
  "one-shot whole-buffer parse, zero retained state, two-phase
  blocks-then-inlines with global ref-def resolution between phases" is the
  mechanism (H0 shape). The flat byte arena and its growth policy,
  marks/containers buffer reuse, the pointer-into-input callback convention,
  and the 32-bit offset width are implementation specifics a Rust mechanism
  model need not reproduce.

## 14. Relevance assessment for the benchmark

H0 FULL_REBUILD: primary anchor (named as such in `R0-METHODOLOGY.md` §3).
MD4C contributes these concepts to H0 (hypothesis-level: H0 is a mechanism
model, not an MD4C port): (a) the contract itself — one entry point, whole
post-edit buffer in, complete fresh result out, all state destroyed
afterwards (§2, §3, §8); (b) the two-phase shape — block/line analysis
completes before inline resolution, with the document-global
reference-definition table built between phases (§8) — which fixes the
ordering H0's BENCH-GRAMMAR-v1 core must honor for the reference-definition
edit family; (c) proof that zero retained state is a shippable, mainstream
design, so the H0 cost model is not a straw man.

H1/H2/H3/H4: MD4C contributes no prior-art mechanism to any of them — none
of the required structures (block damage index, fragment table, old-tree
consultation, restart checkpoints) exists (negative evidence, §4–§7). Its
contribution to those horses is indirect, via Q9/Q10: the catalog of
cross-line and retroactively-bound state (list loosening, fence/HTML
continuation, Setext/table rewrites, ref-def consumption) that any H1
block-local design must preserve-or-recompute across block boundaries, and
that constrains where H4 convergence checks are sound; plus H4 "hidden
grammar state" observations for R3 mutation design.

Fidelity boundary (what we would NOT reproduce): the flat `block_bytes`
arena and its growth policy; marks/containers buffer reuse; the `MD_PARSER`
callback ABI including pointer-into-input text callbacks and absent block
ranges; 32-bit offset width; the specific `max_ref_def_output` budget. Per
the fidelity naming rule, an MD4C-anchored H0 must be documented as an
**md4c-inspired** full rebuild: same mechanism (clean whole-buffer parse, no
retained state), not a port; correctness stays under the #22
self-equivalence oracle (§11).

## 15. Manifest entry

```toml
[[prior_art]]
id = "md4c"
name = "MD4C (Markdown for C)"
source_type = "upstream_repository"
repo_url = "https://github.com/mity/md4c"
tag = "release-0.5.3"
commit_sha = "472c417005c2c71b8617de4f7b8d6b30411d78f4"
retrieved_date = "2026-09-16"
relevant_paths = [
  "src/md4c.h",
  "src/md4c.c",
  "src/md4c-html.c",
  "src/md4c-html.h",
  "README.md",
  "LICENSE.md",
  "CHANGELOG.md",
]
license = "MIT"
notes = """
One-shot whole-buffer push-model parser (single entry point md_parse; \
md_html is a wrapper around it). Negative evidence at release-0.5.3: no \
incremental/update/edit/reparse/resume API exists anywhere in src/ or test/ \
(grep-verified); no module-level mutable state; MD_CTX is stack-local and \
fully freed per call. Two-phase parse: line/block analysis into a flat \
arena, then callback rendering; document-global ref-def hashtable built \
between phases (forward semantic dependency). Callbacks carry no block/span \
ranges; text callbacks are raw pointers into the caller's buffer. \
Prior-art anchor for H0 FULL_REBUILD only; contributes no mechanism to \
H1-H4 (its cross-line/retroactive state catalog informs H1/H4 modeling). \
Fidelity: md4c-inspired mechanism model, not a port.
"""
```
