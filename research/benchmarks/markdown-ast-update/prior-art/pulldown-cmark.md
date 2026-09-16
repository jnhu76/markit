# Prior-art mechanism record — pulldown-cmark

R2 stage (prior-art mechanism extraction) of MARKIT-MARKDOWN-BENCHMARK-1.
Authority: `research/benchmarks/markdown-ast-update/protocol/R0-METHODOLOGY.md` §2.

Evidence rules applied: OBSERVED claims cite `file:line` at the pinned version;
INFERRED claims are labeled; UNKNOWN is stated as unknown. No timing numbers,
no performance claims, no benchmark execution. The mechanism is extracted as
it exists; relevance to H0–H4 is assessed only in §14.

## 1. Identity

```text
PROJECT:    pulldown-cmark (pull/event-stream parser for CommonMark + extensions)
VERSION:    v0.13.4 (latest upstream release tag at retrieval)
COMMIT_SHA: 38e4d08f14ec4bd9783270e9623db7681ebed968
            (git rev-parse v0.13.4^{commit} resolves to this SHA)
REPO_URL:   https://github.com/raphlinus/pulldown-cmark
RETRIEVED:  2026-09-16 (blob-filtered clone into /tmp/markit-r2-prior-art/pulldown-cmark)
LANGUAGE:   Rust (workspace crate pulldown-cmark/, version 0.13.4 in its Cargo.toml:3)
LICENSE:    MIT (crate Cargo.toml:8; LICENSE and pulldown-cmark/LICENSE,
            "Copyright 2015 Google Inc."; pulldown-cmark-escape/LICENSE also MIT)
```

PRIMARY SOURCES: upstream source at the pinned tag (sole mechanism authority).
SECONDARY SOURCES: `README.md`, `guide/src/dev/*.md` — context only. The
upstream issue tracker was NOT retrieved (web tooling quota exhausted at
retrieval time); absence of an incremental API is established from source
only, which is sufficient under the source hierarchy.

Note: the task briefing mentioned an internal `Tree<Point>`; OBSERVED at this
pin there is no `Point` type in `src/` (repo-wide grep). The tree payload is
`Item { start, end, body }` (`src/parse.rs:48`). Mechanism extracted from
what v0.13.4 actually contains.

RELEVANT FILES / FUNCTIONS (relative to repo root; `pulldown-cmark/src/…`):

- `src/lib.rs` — public API re-exports `Parser`, `OffsetIter`, `RefDefs`
  (`:99-101`); `Event` (`:524`); `Tag`/`TagEnd` (`:154`, `:384`); `Options`
  (`:650`); `Event::into_static` (`:615`).
- `src/parse.rs` — `Item` (`:48`), `ItemBody` + `is_maybe_inline` (`:55-129`,
  `:132`), `Parser` struct (`:187-219`), constructors (`:250`, `:255`,
  `:266-292`), `fetch_link_type_url_title` (`:319`), `handle_inline` (`:377`),
  `handle_inline_pass1` (`:387`), `handle_emphasis_and_hard_break` (`:953`),
  `into_offset_iter` (`:1429-1435`), `scan_containers` (`:1440`),
  `InlineEl`/`InlineStack`/`pop_all` (`:1531`, `:1545`, `:1564`),
  `LinkStack` (`:1765`), `CodeDelims`/`MathDelims` (`:1835`, `:1879`),
  `Allocations` (`:1952`), `RefDefs` (`:1971`), `OffsetIter::next`
  (`:2174-2220`), `item_to_event` (`:2254`), `Parser::next` (`:2345-2392`).
- `src/firstpass.rs` — `run_first_pass` (`:24`), `FirstPass` (`:55`),
  `FirstPass::run` block loop (`:69-76`), `parse_block` (`:81`),
  `parse_paragraph` (`:582`), `parse_line` (`:772`), `parse_refdef_total`
  (`:1766`, called at `:372`), `scan_refdef` (`:1901`).
- `src/tree.rs` — `Tree<T>` (`:54`), `reset` (`:159`), `walk_spine` (`:169`),
  `truncate_siblings` (`:196`).
- `src/scanners.rs` — low-level line/fragment scanners (`:21`).
- `src/html.rs` — `push_html`, independent consumer of the event stream.

## 2. Problem actually solved

OBSERVED: produce a lazy, pull-style stream of `Event`s from one borrowed
`&str`, with minimal allocation and copying, aiming at 100% CommonMark
compliance (`README.md:9-20`; `parse.rs:36`, "Tree-based two pass parser").
The document is an input, not a living object: `Parser` stores
`text: &'input str` (`parse.rs:188`) and has no method to supply an edited or
replacement text.

OBSERVED (negative): the solved problem set does NOT include updating a
previous parse after an edit. The whole public API is: construction from a
whole string, event iteration, `into_offset_iter`, and
`reference_definitions()` (`lib.rs:99-101`; `parse.rs:250`, `:255`, `:266`,
`:296`, `:1434`). A grep of `src/` for `incremental|update|edit|reparse`
finds only intra-parse Item end-index bookkeeping comments
(`firstpass.rs:134`, `:198`, `:214`, `:839`) — no edit entry point exists.

## 3. Retained representation

OBSERVED — across parses: nothing. `Parser` is constructed per call with a
fresh `run_first_pass(text, options)` (`parse.rs:271`); all owned state dies
with the `Parser` value. No parser handle, cache, or global table exists.

OBSERVED — within one parse, two stages retain different state:

- First-pass output (eager, built in the constructor): `Tree<Item>` — a
  `Vec<Node<Item>>` plus a `spine` stack (`tree.rs:54-58`); `Item` is
  `{ start: usize, end: usize, body: ItemBody }` (`parse.rs:48-53`);
  `Allocations` with `refdefs: RefDefs` (HashMap of link reference
  definitions), `footdefs`, and index vecs for links/cows/alignments/headings
  (`parse.rs:1952-1960`, `:1971-1977`).
- Second-pass working state (mutated lazily during iteration):
  `inline_stack: InlineStack` (emphasis delimiter stack with typed
  `lower_bounds`, `parse.rs:214`, `:1545-1553`), `link_stack` +
  `wikilink_stack` (`:215-216`), `code_delims`/`math_delims` (`:217-218`,
  `:1835`, `:1879`), `html_scan_guard` (`:193`), and
  `link_ref_expansion_limit` fuel (`:211`, init `:288`).

## 4. Reuse unit

OBSERVED (negative): no reuse unit across parses — fragment, subtree,
checkpoint, and cache types do not exist in this codebase. Within a parse,
the tree node (`Node<Item>`) is the unit the second pass resolves in place:
`handle_inline_pass1` rewrites `item.body` of existing nodes (e.g.
`MaybeHtml` → `ItemBody::Link`, `parse.rs:416`) and truncates/drops siblings
at block ends via `Tree::truncate_siblings` (`tree.rs:196-247`, called from
`firstpass.rs:753`). These are single-parse resolution mechanics, not
cross-edit reuse.

## 5. Damage detection / invalidation

OBSERVED (negative): NOT_APPLICABLE. There is no edit input, hence no damage
detection and no invalidation of prior results. `Tree::truncate_siblings` and
`InlineStack::pop_all` (`parse.rs:1564-1572`, demoting unmatched delimiters
to `Text`) resemble "invalidation" but operate on the current parse's own
unfinished state — not edit-driven invalidation of retained prior state.

## 6. Restart rule

OBSERVED (negative): NOT_APPLICABLE — verified. No restart or checkpoint
concept exists. The first pass is a single forward loop over the whole text
with one cursor (`firstpass.rs:69-76`); event iteration is a one-way
traversal of the already-built tree. `Tree::reset` (`tree.rs:159-166`) only
rewinds focus to node 1 after the first pass so event emission can begin
(`parse.rs:272`); it re-walks already-built structure within the same parse
and is not a parse restart from saved parser state. No state snapshots are
taken anywhere in `src/`.

## 7. Convergence rule

OBSERVED (negative): NOT_APPLICABLE — verified. There is no notion of parser
state becoming "stable" so a suffix can be reused; nothing in source or guide
mentions suffix reuse or convergence. The only cross-block semantic state —
link/footnote reference definitions — is resolved by exhausting the entire
document in the first pass (`firstpass.rs:69-76`; refdefs scanned at `:372`/
`:1766`, stored in `Allocations.refdefs`, `parse.rs:1952-1953`), i.e. by
completeness, not by convergence detection.

## 8. Reconstruction

OBSERVED: the only route from "edited source" to "events" is constructing a
new `Parser` over the new string — a clean full parse. Within a parse, events
are reconstructed lazily from the tree: `Parser::next` / `OffsetIter::next`
walk the `Tree<Item>`; when the current node `is_maybe_inline()`
(`parse.rs:132-145`), `handle_inline` (`:377-380`, invoked at `:2203-2205`
and `:2373`) resolves that block's inline items, mutating the tree in place
before events are emitted (`item_to_event`, `parse.rs:2254-2343`). HTML
rendering (`html.rs::push_html`) is an independent downstream consumer of the
event stream, decoupled from parsing.

## 9. Position / range maintenance

OBSERVED: positions are absolute byte offsets into the borrowed input `&str`,
stored per node (`Item.start`/`Item.end`, `parse.rs:48-53`). `Event`s carry
no offsets by default; offsets are exposed only via `into_offset_iter`
("produces `(Event, Range)` pairs, where the `Range` value maps to the
corresponding range in the markdown source", `parse.rs:1429-1435`), where
each pair's range is read straight off the node (`item.start..item.end`,
`parse.rs:2190`, `:2216`) and text events slice the input
(`text[item.start..item.end]`, `parse.rs:2256`). There is no persistent
offset index, no per-edit shift, no line/column map, no secondary coordinate
system: the coordinate is the UTF-8 byte offset in the input, valid only
while that input is borrowed. Nothing survives to be maintained across parses.

## 10. Fallback

OBSERVED (negative): no fallback mechanism exists — no degraded or recovery
parse mode; a parse either runs the full two passes or is never started.
Adversarial-input guards are DoS containment, not fallbacks:
`LINK_MAX_NESTED_PARENS = 32` (`parse.rs:40-46`),
`link_ref_expansion_limit` fuel (`parse.rs:211`, `:288`, `:319-368`),
`MATH_BRACE_CONTEXT_MAX_NESTING` (`firstpass.rs:43-55`).

## 11. Correctness authority

OBSERVED: the CommonMark spec, stated as the goal ("Correct; the goal is 100%
compliance with the CommonMark spec", `README.md:19`) and enforced by vendored
spec suites (`pulldown-cmark/specs/`, `pulldown-cmark/tests/suite/`).
Extensions (tables, footnotes, strikethrough, math, GFM, definition lists,
wikilinks, …) are opt-in via `Options` bitflags (`lib.rs:650-736`). There is
no incremental oracle because there is no incremental path: every parse is
self-validating against the full source by construction.

## 12. Known limitations

- OBSERVED: no retained parse state and no incremental/update API of any kind
  (§2, §3). Embedding editors must re-parse the whole document per revision;
  the library offers no assistance.
- OBSERVED: the first pass over the entire document runs eagerly inside the
  constructor (`parse.rs:271`), before a single event is emitted — callers
  cannot amortize it against lazy consumption of the first events.
- OBSERVED: the input is borrowed (`text: &'input str`, `parse.rs:188`) with
  `'input` threaded through `Event`/`Tag`/`CowStr`; results cannot outlive
  the source buffer without `into_static()` copies (`lib.rs:615`).
- OBSERVED: iteration is single-shot (`Iterator` + `FusedIterator`,
  `parse.rs:2345`, `:2394`; `into_offset_iter` consumes the `Parser`,
  `parse.rs:1434`); a stream cannot be replayed or rewound.
- OBSERVED: "increment(al)" in the official guide means streaming / intra-parse
  tree construction only, not cross-edit updates
  (`guide/src/dev/block-parsing.md:84`; `guide/src/dev/performance.md:96`).
- HYPOTHESIS FOR R3/R8: reference definitions create a document-wide semantic
  dependency (a `[ref]: url` line at the end changes how a `[ref]` at the top
  resolves — §13 Q5), so any block-local incremental Markdown scheme must
  either keep a document-wide refdef index or accept cross-block
  invalidation; pulldown-cmark sidesteps this by making the first pass total.
- HYPOTHESIS FOR R3/R8: `scan_containers` walks the open-container spine per
  line (`parse.rs:1440-1478`) and `ItemBody::ListItem(indent)` carries indent
  in the node (`parse.rs:112`), so an incremental restart design would need to
  checkpoint spine, indents, blank-line flags (`firstpass.rs:59`), and math
  brace contexts (`firstpass.rs:44-55`).

## 13. Adversarial hypotheses

- Q1 (what old info survives an edit): OBSERVED — none by construction;
  nothing outlives a `Parser`, and no API accepts an edit.
- Q2 (who vouches validity): nothing is retained; every result is vouched by
  a fresh complete parse over the full source (OBSERVED design). UNKNOWN
  whether upstream discussions propose vouching mechanisms (tracker not
  retrieved).
- Q3 (maximally invalidating edit): INFERRED — all edits are equally
  invalidating: any byte change forces a fresh constructor call and full
  re-parse; there is no partial-damage notion to grade edits by.
- Q4 (tiny edit forcing O(N)): OBSERVED by code path for the first pass —
  `run_first_pass` scans the whole `text.len()` unconditionally
  (`firstpass.rs:69-76`) and is invoked eagerly in the constructor
  (`parse.rs:271`), so a one-byte edit costs a full block scan. INFERRED: the
  lazy second pass is also O(document) over full iteration.
- Q5 (far-forward propagation): INFERRED from mechanism + CommonMark
  semantics — yes for reference definitions: refdefs are collected
  document-wide in the first pass (`firstpass.rs:372`, `:1766`) and consulted
  during second-pass link resolution regardless of position
  (`fetch_link_type_url_title`, `parse.rs:319-368`), so a definition appended
  at document end changes events for references at the start. Footnote
  use-counting (`FootnoteDef.use_count`, `parse.rs:1824-1827`) is a similar
  document-wide counter.
- Q6 (parse vs reconstruction trade): OBSERVED — no trade is made; parsing is
  the only significant retained work, events are re-derived lazily.
  HYPOTHESIS: because the second pass mutates tree nodes in place
  (`parse.rs:416`), an incremental variant could not reuse first-pass output
  without freezing inline resolution or adding copy-on-write — a structural
  cost this design never pays.
- Q7 (position maintenance vs reuse): OBSERVED — zero position-maintenance
  cost (offsets are recomputed from scratch each parse as scan byproducts).
  HYPOTHESIS: this is the extreme point of the trade-space the Weakness Map
  should measure against; any incremental design inherits a shifting/anchoring
  cost pulldown never pays.
- Q8 (memory ∝ N): OBSERVED within a parse — the tree is a `Vec` of nodes
  (`tree.rs:55`) pre-sized by `max(128, text.len()/32)` (`firstpass.rs:27-28`),
  plus `Allocations` vecs; all freed on drop. HYPOTHESIS: peak memory is a
  real cost for huge documents, unlike true streaming token emitters.
- Q9 (hidden grammar state): OBSERVED inventory — open-container spine
  consulted per line (`scan_containers`, `parse.rs:1440`; `walk_spine`,
  `tree.rs:169`), `last_line_blank` (`firstpass.rs:59`), list indent inside
  `ItemBody::ListItem(usize)` (`parse.rs:112`), `html_scan_guard`
  (`parse.rs:193`), `InlineStack` with typed lower bounds (`parse.rs:1545`),
  `CodeDelims`/`MathDelims` maps (`parse.rs:1835`, `:1879`), math
  brace-context stack (`firstpass.rs:55-65`). HYPOTHESIS: this set defines
  the minimum checkpoint for any H4-style restart in a pulldown-like
  architecture.
- Q10 (false convergence): NOT_APPLICABLE to this mechanism (no convergence
  concept exists). UNKNOWN beyond that; no evidence either way.
- Q11 (fallback frequency): OBSERVED — zero by construction; no fallback path
  exists (§10).
- Q12 (mechanism-intrinsic vs implementation-specific): OBSERVED — "no
  retained state / no incremental API" is mechanism-intrinsic (API- and
  structure-level, holds for any consumer). INFERRED — the two-pass
  block/inline split is a design choice of this parser family, not a
  CommonMark requirement; the DoS fuel limits (`parse.rs:288`, `:364-366`)
  are implementation-specific hardening.

## 14. Relevance assessment for the benchmark

Contribution by horse (hypothesis-level; extraction-first, no retrofitting):

- H0 FULL_REBUILD — primary anchor value. OBSERVED: the entire project is a
  clean, mature full-rebuild route (construct → eager first pass → lazy event
  emission → optional offset pairing). Transferable concepts: (a) two-stage
  normalized syntax state = block tree (`Tree<Item>` + `Allocations`) distinct
  from resolved inline state; (b) eager block pass vs lazy inline/event stage
  as separable cost centers for instrumentation; (c) absolute byte ranges as
  the canonical position payload (§9).
- H1 BLOCK_LOCAL_REPARSE / H2 FRAGMENT_REUSE / H3 OLD_TREE_SUBTREE_REUSE —
  negative/contrastive contribution only: pulldown-cmark defines the
  zero-reuse baseline these horses must beat, and its refdef/footdef
  document-wide dependencies (Q5) are exactly the cross-block semantics that
  block-local or fragment schemes must handle correctly.
- H4 RESTART_CONVERGENCE — concept extraction only: the Q9 hidden-state
  inventory is a concrete checklist of what a restart/checkpoint state would
  have to capture in a pulldown-like architecture; pulldown-cmark itself
  implements none of it.

Fidelity boundary (what we would NOT reproduce): the benchmark's H0 model
would be pulldown-cmark-inspired, not a reproduction — we would not
reimplement its emphasis/delimiter resolution, scanners, `CowStr`/`InlineStr`
allocation strategy, SIMD paths, DoS fuel limits, exact event ordering, or
spec-version test corpus. What transfers is the route shape (full rebuild =
fresh eager block pass + event derivation) and the position model, not code.

## 15. Manifest entry

```toml
[[prior_art]]
id = "pulldown-cmark"
name = "pulldown-cmark"
source_type = "upstream_repository"
repo_url = "https://github.com/raphlinus/pulldown-cmark"
tag = "v0.13.4"
commit_sha = "38e4d08f14ec4bd9783270e9623db7681ebed968"
retrieved_date = "2026-09-16"
relevant_paths = [
  "pulldown-cmark/src/lib.rs",
  "pulldown-cmark/src/parse.rs",
  "pulldown-cmark/src/firstpass.rs",
  "pulldown-cmark/src/tree.rs",
  "pulldown-cmark/src/scanners.rs",
  "pulldown-cmark/src/html.rs",
  "guide/src/dev/block-parsing.md",
  "guide/src/dev/performance.md",
]
license = "MIT"
notes = """
Two-pass event-stream parser; eager full-document first pass in the \
constructor (parse.rs:271), lazy inline resolution during iteration. \
Negative finding (verified at v0.13.4): no retained state across parses, \
no edit/update/incremental API; positions are absolute UTF-8 byte ranges \
exposed only via into_offset_iter. H0 FULL_REBUILD anchor and zero-reuse \
baseline; 'incremental' in the guide means streaming/intra-parse tree \
building only. Issue-tracker discussion not retrieved (web quota); \
source-level evidence is primary and sufficient.
"""
```
