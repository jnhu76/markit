# Prior-art mechanism record — Comrak

R2 stage (prior-art mechanism extraction) of MARKIT-MARKDOWN-BENCHMARK-1.
Authority: `research/benchmarks/markdown-ast-update/protocol/R0-METHODOLOGY.md` §2.

Evidence rules applied: OBSERVED claims cite `file:line` at the pinned version;
INFERRED claims are labeled; UNKNOWN is stated as unknown. No timing numbers,
no performance claims, no benchmark execution. The mechanism is extracted as
it exists; relevance to H0–H4 is assessed only in §14.

## 1. Identity

```text
PROJECT:    Comrak (Rust port of cmark-gfm; CommonMark + GFM parser)
VERSION:    v0.55.0 (latest upstream release tag at retrieval)
COMMIT_SHA: 6fbe87fafde3953a9f3bc582804318593d703805
            (tag v0.55.0 points at this SHA; verified with git tag --points-at)
REPO_URL:   https://github.com/kivikakk/comrak
RETRIEVED:  2026-09-16 (shallow clone into /tmp/markit-r2-prior-art/comrak)
LANGUAGE:   Rust (rust-version 1.85)
LICENSE:    BSD-2-Clause (Cargo.toml:12)
```

PRIMARY SOURCES: upstream repository source at the pinned tag (sole mechanism
authority). SECONDARY SOURCES: `CHANGELOG.md`, crate docs — used only for API
history, never as sole authority for a mechanism claim.

RELEVANT FILES / FUNCTIONS (all paths relative to repo root, v0.55.0):

- `src/lib.rs` — public API: `parse_document` re-export (`:96`), `Arena` alias
  `typed_arena::Arena<AstNode<'a>>` (`:101`), convenience wrappers
  `markdown_to_html*` / `markdown_to_commonmark*` / `markdown_to_commonmark_xml*`
  (`:106-159`; each builds a fresh `Arena` per call).
- `src/parser/mod.rs` — `parse_document` (`:45`), `Parser` struct, all
  block-phase scanner state (`:115-141`), line loop (`:186-227`),
  `process_line` (`:260-301`), `check_open_blocks_inner` (`:317-400`),
  `find_first_nonspace` (`:402-434`), container prefix matchers (`:436-628`),
  `open_new_blocks` (`:741`), `add_child` (`:1809-1825`),
  `add_text_to_container` (`:1827-1948`), `finalize_document` (`:2016-2037`),
  `propagate_list_sourcepos` (`:2041-2077`), `finalize`/`finalize_borrowed`
  (`:2079-2132`), `resolve_reference_link_definitions` (`:2083-2106`; called
  from paragraph finalize `:2142`), `process_inlines`/`parse_inlines`
  (`:2263-2294`), `parse_reference_inline` (`:2688-2755`).
- `src/parser/inlines.rs` — `Subject` inline-phase state (`:34-56`),
  `parse_inline` char dispatch (`:211`), `handle_delim` (`:775`),
  `process_emphasis` (`:1364`), `handle_close_bracket` (`:1721`; refmap
  lookup `:1837-1843`, `broken_link_callback` fallback `:1847-1856`),
  `RefMap` (`:2408-2438`), `Delimiter` (`:2468-2477`), `Bracket` (`:2505-2510`).
- `src/nodes.rs` — `NodeValue` (`:34`), `Ast` (`:800-816`), size assertions
  128 B `Ast` / 176 B per arena node (`:825-837`), `Sourcepos` (`:841-846`),
  `LineColumn` (`:895-905`), `AstNode`/`Node` aliases (`:996-997`).
- `src/arena_tree.rs` — arena-tree `Node`, five `Cell<Option<&Node>>` links
  (`:18-29`).
- `src/strings.rs` — `normalize_label` (`:340`), `normalize_code` (`:71`).

## 2. Problem actually solved

OBSERVED: one-shot conversion of a full Markdown string into a full AST
(plus rendered output via the formatter wrappers), conforming to CommonMark
and GFM (`src/lib.rs:1-2`). API shape is parse-once, inspect/mutate, format.
No editing problem is solved: there is no edit-descriptor input and no API
accepting prior state. Negative evidence (2026-09-16, v0.55.0): grep across
`src/` for `incremental|edit|update|reparse` entry points returns none; the
only parse entry is `parse_document`.

## 3. Retained representation

OBSERVED: a caller-owned arena (`src/lib.rs:101`) of
`AstNode = arena_tree::Node<'a, RefCell<Ast>>` (`src/nodes.rs:996`). Each
`Ast` holds `NodeValue`, `Sourcepos` (1-based line/column), a `content:
String` used during the block phase, and parser bookkeeping (`open`,
`last_line_blank`, `table_visited`, `line_offsets`; `src/nodes.rs:811-815`).
Tree links are five `Cell` pointers (`src/arena_tree.rs:20-24`). Within one
parse the parser also retains a `RefMap` (`src/parser/mod.rs:118`; struct
`src/parser/inlines.rs:2408-2412`) and footnote defs.

OBSERVED lifetime: `parse_document` allocates the Document node in the
caller's arena and returns `&AstNode` (`src/parser/mod.rs:45-57`); the AST
lives exactly as long as that arena. Nothing persists across calls:
`Parser::new` builds fresh `RefMap` and state every call
(`src/parser/mod.rs:158-184`); no static/global source-keyed cache exists.

INFERRED: arena allocation is an implementation detail (bulk free, stable
pointers), not a reuse mechanism — the arena only grows until dropped.

## 4. Reuse unit

OBSERVED: none across parses. Within a single parse there are two sharing
structures, both semantic maps, not syntax reuse:

1. `refmap: FxHashMap<String, ResolvedReference>` — reference definitions
   collected once in the block phase, first definition wins via `or_insert`
   (`src/parser/mod.rs:2098`); looked up by inline reference links
   (`src/parser/inlines.rs:1840`) under a cumulative size budget
   `max_ref_size = total_size.min(100000)` (`src/parser/mod.rs:2023`; check
   at `src/parser/inlines.rs:2426-2433`).
2. `FootnoteDefs` — auto-generated inline footnote definitions
   (`src/parser/inlines.rs:2440-2466`).

Neither survives `parse_document`; neither is keyed by source position.

## 5. Damage detection / invalidation

OBSERVED: absent by construction. No API input describes an edit; every
`parse_document` call treats its `&str` as entirely new source. No code
compares parses. (Negative evidence: §2 grep.)

## 6. Restart rule

NOT_APPLICABLE. OBSERVED: parsing is one forward line loop from offset 0 to
end (`src/parser/mod.rs:203-222`); there are no checkpoints, saved scanner
states, or re-entry points, and `Parser` state is created inside the call and
dropped after it. A restart needs retained state to restart from; none exists.

## 7. Convergence rule

NOT_APPLICABLE. OBSERVED: convergence is meaningless without a reuse suffix —
the parse never branches on old state, so no "state equals remembered state"
test point exists. `finalize_document` runs unconditionally at end-of-input
(`src/parser/mod.rs:2016-2037`).

## 8. Reconstruction

OBSERVED: the full tree is constructed fresh every parse, in two phases.

- Phase 1 (blocks): per line, `process_line` resets scanner state
  (`src/parser/mod.rs:272-279`), walks the open-container chain matching
  per-type prefixes (`:317-400`, matchers `:436-628`), opens new blocks
  (`:741`), and appends line content to the open leaf's `Ast.content`
  (`add_text_to_container`, `:1827-1948`). `add_child` finalizes mismatched
  ancestors on the way down (`:1815-1817`); unmatched open blocks are closed
  in the same pass (`:1872-1874`). At paragraph finalize, leading reference
  definitions are extracted into the refmap and stripped from content
  (`:2142`, `:2083-2106`).
- Phase 2 (inlines): `finalize_document` closes all blocks, then
  `process_inlines` walks every node whose value `contains_inlines()` and
  calls `parse_inlines` on it (`:2263-2267`, `:2271-2294`). Each such node
  gets a fresh `Subject` over its taken `content` string (`mem::take`,
  `:2274`) plus a fresh per-node delimiter arena (`:2279`); the char loop
  (`src/parser/inlines.rs:211`) builds a delimiter linked list
  (`:2468-2477`) and bracket stack (`:48`, `:2505-2510`), closed by
  `process_emphasis(0)` / `clear_brackets()` (`src/parser/mod.rs:2292-2293`).

OBSERVED ordering consequence: the refmap is fully built before any inline
runs, so a link may resolve to a definition appearing later in the document —
a document-level forward semantic dependency resolved by phase ordering.

## 9. Position / range maintenance

OBSERVED: nodes store `Sourcepos { start, end }` of 1-based `LineColumn`
(`src/nodes.rs:841-905`); columns count UTF-8 bytes by default, with optional
post-parse conversion to char columns (`src/parser/mod.rs:47-56`). Starts are
set at node creation (`add_child`, `:1821`); ends are computed at finalize
time from scan position (`:2115-2131`), using `last_line_length` carried from
the previous consumed line (`:297`, `:133`). List ends are repaired
afterwards by a post-order descendant scan (`propagate_list_sourcepos`,
`:2041-2077`). All positions are computed during the single pass; none are
maintained against later edits.

## 10. Fallback

OBSERVED: no performance fallback exists or is needed — the full parse is the
only mode. The `broken_link_callback` (`Options.parse.broken_link_callback`,
consumed at `src/parser/inlines.rs:1847-1856`) is a semantic hook supplying
link URLs for unresolvable labels, not a degraded parsing path. API history
(SECONDARY, `CHANGELOG.md:465`): the standalone entry point
`parse_document_with_broken_link_callback` was deprecated in 0.25.0 and
removed (PR #623); the callback moved into `Options`. At v0.55.0 the only
parse entry point is `parse_document`.

## 11. Correctness authority

OBSERVED: upstream's own correctness authority is the CommonMark spec plus
GFM extensions; the crate ships upstream spec tests and its own suite
(`src/tests.rs`, `src/tests/`). For this benchmark, Comrak is (a) an H0
prior-art anchor and (b) a possible semantic reference for BENCH-GRAMMAR-v1
constructs; benchmark correctness authority remains the #22 protocol
(self-equivalence against H0), not Comrak. Using Comrak as a semantic oracle
would be an additional, separable claim.

## 12. Known limitations

Evidence-backed (OBSERVED unless labeled):

1. Any source change costs a full two-phase parse: no update path exists
   (§2, §5 negative evidence).
2. Whole-AST memory scales with document size and is freed only when the
   entire arena drops (`src/lib.rs:101`); upstream documents per-node cost:
   128 B `Ast` + 8 B `RefCell` + 40 B links = 176 B/node
   (`src/nodes.rs:825-837`).
3. Block content is duplicated as `String` per block before the inline pass
   consumes it (`src/nodes.rs:811`; `mem::take` at `src/parser/mod.rs:2274`)
   — transient extra memory during parse.
4. Refmap is first-definition-wins (`or_insert`, `src/parser/mod.rs:2098`)
   and silently suppresses further lookups once cumulative resolved-reference
   size exceeds `max_ref_size` (`:2023`; `src/parser/inlines.rs:2426-2433`)
   — hidden, size-capped semantic state.
5. End positions rely on scan-time heuristics (`last_line_length`,
   `src/parser/mod.rs:2115-2131`) plus a post-hoc list repair pass
   (`:2041-2077`) — position truth is derived, not maintained.
6. HYPOTHESIS FOR R3/R8: the inline phase re-scans each inline container's
   full content even for a one-character change, and the retained tree holds
   only line/column positions — Comrak offers no structure supporting
   fine-grained incremental inlines.

## 13. Adversarial hypotheses

All items are HYPOTHESIS unless an OBSERVED citation is given. Comrak has no
cross-edit mechanism, so most questions collapse into "everything is
recomputed"; the residue is what its single-parse state teaches the shared
grammar core.

- Q1 (old info surviving an edit): none across edits — no state outlives a
  call (OBSERVED §3). Within one parse: refmap + footnote defs survive from
  block phase into inline phase (OBSERVED §4).
- Q2 (who vouches validity): across edits, nobody — nothing is retained.
  Within a parse, refmap validity rests on `normalize_label(.., Case::Fold)`
  applied at both insert (`src/parser/mod.rs:2743`) and lookup
  (`src/parser/inlines.rs:1838`) — OBSERVED.
- Q3 (maximally invalidating edit): every edit, including a 1-byte insert,
  invalidates 100% of state (OBSERVED).
- Q4 (tiny edit forcing O(N)): trivially any edit — the whole document is
  re-inspected by construction (single line loop,
  `src/parser/mod.rs:203-222`, OBSERVED).
- Q5 (far-forward propagation): OBSERVED inversion of the usual worry —
  forward dependencies are resolved globally by phase ordering (§8): a
  definition at EOF resolves links at document start at no propagation cost.
- Q6 (parse vs reconstruction trade): never traded; always full parse and
  full rebuild (OBSERVED).
- Q7 (position maintenance vs reuse): positions are always recomputed from
  the scan; finalize-time end computation via `last_line_length`
  (`src/parser/mod.rs:2115-2131`, OBSERVED) piggybacks position derivation
  on the block pass — HYPOTHESIS: this coupling is what an incremental
  successor must break first.
- Q8 (memory ∝ N): yes for the tree (OBSERVED §12.2); the refmap is capped
  at a document-size-derived budget (OBSERVED §12.4).
- Q9 (hidden grammar state): OBSERVED scanner state any boundary-crossing
  design must model: `last_line_length`, `thematic_break_kill_pos`,
  `partially_consumed_tab`, `blank`, `first_nonspace(_column)`
  (`src/parser/mod.rs:115-141`); per-node `open`, `last_line_blank`,
  `line_offsets` (`src/nodes.rs:812-815`); per-Subject `backticks` table and
  `no_link_openers` (`src/parser/inlines.rs:50-52`).
- Q10 (false convergence): not applicable — no convergence test exists
  (OBSERVED §7).
- Q11 (fallback frequency): not applicable — full-parse mode has frequency 1
  by definition (OBSERVED §10).
- Q12 (mechanism-intrinsic vs implementation-specific): OBSERVED split —
  "full two-phase parse, no retained state" is the mechanism (H0 shape);
  typed-arena representation, `RefCell` interior mutability, `Ast` size
  layout, jetscii line matching are implementation specifics the benchmark
  must not inherit as mechanism facts.

## 14. Relevance assessment for the benchmark

H0 FULL_REBUILD: primary anchor. Comrak is a production-grade clean
full-parse route and contributes concepts to H0 (hypothesis-level: H0 is a
mechanism model, not a Comrak port):

- two-phase parse — blocks accumulate `content`, inlines run as a second
  document-wide pass — with the refmap built strictly in phase 1 (OBSERVED
  §8); the benchmark's semantic-dependency edit family should reproduce this
  ordering;
- line-by-line container open/close with per-type prefix matchers as the
  shape of the shared BENCH-GRAMMAR-v1 block scanner;
- finalize-time position derivation (§9).

H1/H2/H3/H4: Comrak contributes no prior-art mechanism to any of them — none
of the required structures (block damage index, fragment table, old-tree
consultation, checkpoints) exists (negative evidence, §4–§7). Its open-block
walk resembles H4's "container state" only superficially: that state is
per-line within one parse, not across edits.

Fidelity boundary (what we would NOT reproduce): the typed-arena /
`RefCell<Ast>` representation, `Ast` size layout, per-node delimiter arenas,
jetscii line scanning, and line/column sourcepos semantics. The benchmark
contract requires UTF-8 byte spans and forbids pointer/arena identity in
correctness (`R0-METHODOLOGY.md` §5). Per the fidelity naming rule, a
Comrak-anchored H0 must be documented as a **comrak-inspired** full rebuild:
same mechanism (clean two-phase full parse, refmap-before-inlines), not a
port.

## 15. Manifest entry

```toml
[[prior_art]]
id = "comrak"
name = "Comrak"
source_type = "upstream_repository"
repo_url = "https://github.com/kivikakk/comrak"
tag = "v0.55.0"
commit_sha = "6fbe87fafde3953a9f3bc582804318593d703805"
retrieved_date = "2026-09-16"
relevant_paths = [
  "src/lib.rs",
  "src/parser/mod.rs",
  "src/parser/inlines.rs",
  "src/nodes.rs",
  "src/arena_tree.rs",
  "src/strings.rs",
]
license = "BSD-2-Clause"
notes = """
One-shot two-phase (blocks then inlines) full parse into a caller-owned \
typed-arena AST. No incremental/update API at v0.55.0; \
parse_document_with_broken_link_callback removed (deprecated 0.25.0, PR #623); \
broken_link_callback now an Options field (semantic hook, not a fallback). \
Refmap built in block phase (first-wins, size-capped), consumed by inline \
phase: document-level forward dependency resolved by phase ordering. \
Prior-art anchor for H0 FULL_REBUILD only; contributes no mechanism to \
H1-H4. Fidelity: comrak-inspired mechanism model, not a port; arena/RefCell \
representation excluded from the benchmark contract.
"""
```
