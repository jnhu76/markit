# Prior-art mechanism record — Lezer / CodeMirror incremental parsing (@lezer/common, @lezer/lr, @lezer/markdown)

R2 stage (prior-art mechanism extraction) of MARKIT-MARKDOWN-BENCHMARK-1.
Authority: `research/benchmarks/markdown-ast-update/protocol/R0-METHODOLOGY.md`
§2 (prior-art roles) and §3 (frozen horses H0–H4).

Evidence rules applied: OBSERVED claims cite `file:line` at the pinned
versions; INFERRED claims are labeled; UNKNOWN is stated, never guessed.
No timing numbers, no performance claims, no benchmark execution. Mechanisms
are extracted as they exist; relevance to H0–H4 is assessed only in §14.

## 1. Identity

```text
PROJECT(s): Lezer parser system — three packages, three official repos:
  a) lezer-parser/common   (@lezer/common)   — Tree/TreeBuffer, NodeProp,
                                              TreeFragment, Parser/Input API
  b) lezer-parser/lezer    — ORIGINALLY the LR engine + generator repo; now a
             forwarding stub (HEAD commit message "Add forwarding link", no
             git tags exist). The LR runtime moved to lezer-parser/lr
             (@lezer/lr). Extracted engine source is pinned from lr.
  c) lezer-parser/markdown (@lezer/markdown) — incremental Markdown parser

VERSION(s)/SHA(s): (shallow clones under /tmp/markit-r2-prior-art/)
  lezer-parser/common    tag 1.5.2  commit de5f96276a2954c249de1475e8b03f79c20d9ce4
  lezer-parser/lr        tag 1.4.8  commit f81d6a25c3482aa7fc12434e9adea9d75c56ad08
  lezer-parser/markdown  tag 1.6.3  commit 9942d7ce41d734d743cbdb48177dddc1975fdc5c
  lezer-parser/lezer     no tags;   main a4284ee6ef0d529a07161f3534c8ef1aaa8efc99 (stub)
  (1.5.2 / 1.4.8 / 1.6.3 verified as latest release tags via git ls-remote
   on 2026-09-16; lezer repo has no refs/tags at all.)

REPO_URLS:
  https://github.com/lezer-parser/common
  https://github.com/lezer-parser/lr        (resolved home of the "lezer" engine)
  https://github.com/lezer-parser/markdown
  https://github.com/lezer-parser/lezer     (stub; recorded for provenance)

RETRIEVED: 2026-09-16
LANGUAGE:  TypeScript / JavaScript (npm packages)
LICENSE:   MIT (lezer-common/package.json:14 + LICENSE; lezer-markdown/
           package.json:14 + LICENSE; lezer-lr/package.json:14 + LICENSE;
           lezer-markdown/LICENSE: "MIT License, (C) 2020 Marijn Haverbeke")
```

PRIMARY SOURCES: pinned source trees above (sole mechanism authority).
SECONDARY SOURCES (orientation only, never sole authority for a mechanism
claim): Context7 snapshots `/lezer-parser/common` and `/lezer-parser/markdown`
(retrieved 2026-09-16: TreeFragment addTree/applyChanges semantics, Parser
base class, BlockContext gating `fragments.length ? new FragmentCursor : null`,
full-vs-incremental `parse(doc)` vs `parse(doc, fragments)` — all consistent
with pinned source); `lezer-markdown/README.md` and `CHANGELOG.md` (design
statement and incremental-bug history).

RELEVANT FILES / FUNCTIONS / SECTIONS (paths relative to repo root, pinned tags):

- `common/src/parse.ts` — `ChangedRange` (:5-14), `TreeFragment`
  (:25-101: ctor :33-51, `openStart` :57, `openEnd` :61, `addTree` :69-73,
  `applyChanges(fragments, changes, minGap = 128)` :78-100), `PartialParse`
  (:105-126), `Parser.startParse`/`parse` (:147-168), `Input` (:174-189).
- `common/src/tree.ts` — `DefaultBufferLength = 1024` (:4), `NodeProp`
  (:15), `NodeProp.contextHash` per-node (:108), `NodeProp.lookAhead`
  per-node (:114), `Tree` (:351), `TreeBuffer` (:606), `SpecialRecord`
  Reuse=-1/ContextChange=-3/LookAhead=-4 (:1388-1392), `buildTree`/`takeNode`
  (:1394-1477; reuse push :1409-1415).
- `lr/src/parse.ts` — `cutAt` (:13-25), LR-side `FragmentCursor` (:27-103:
  `nextFragment`/safeFrom/safeTo :42-55, `nodeAt` :58-102), `Rec` constants
  (:216-233), `Parse` ctor fragment gate (:262-265), `advanceStack` reuse
  block (:395-408), `stackToTree` (:501-511), `runRecovery` (:456-498),
  `ContextTracker` (:551-594).
- `lr/src/stack.ts` — `Lookahead.Margin = 25` (:5-9), `Stack.start` (:69-73),
  `Stack.useNode` (:222-235), `emitLookAhead` (:425-429), `setLookAhead`
  (:441-447).
- `markdown/src/markdown.ts` — `CompositeBlock` (:1-48; rolling hash
  `CompositeBlock.create` :3-6, `hashProp` :22, `addChild` rehash :26-30),
  `skipForList` (:205-213), `DefaultSkipMarkup` (:215-233), fenced-code block
  parser (:404-440), `BlockContext` (:638; ctor fragment gate :677;
  `advance` :685-727 with reuse hook :705; `reuseFragment` :742-759;
  `readLine`/skip-context markup :832-852; `startContext` :866-869;
  `addNode` :879-882; `finish`/`addGaps` :904-910; `finishLeaf` :916-921),
  `injectGaps` (:939-968), `MarkdownParser` (:1121; `createParse` :1151-1154),
  `Element` (:1336-1356), `NotLast` (:1819-1821), markdown-side
  `FragmentCursor` (:1823-1864: `moveTo` :1843, `matches` :1866-1869,
  `takeNodes` :1871-1908), `toRelative` (:1911-1919).
- `markdown/src/extension.ts` — GFM extension bundle (Strikethrough, Table,
  Autolink, TaskList) via `MarkdownConfig`.
- `markdown/src/nest.ts` — `parseCode`/`parseMixed` nested-language wiring.
- `markdown/test/test-incremental.ts` — correctness/reuse test oracle.

## 2. Problem actually solved

OBSERVED: editor incremental syntax-tree maintenance for CodeMirror 6. After
text edits (delivered as `ChangedRange {fromA,toA,fromB,toB}` lists,
common/parse.ts:5-14), a new full-document tree is produced such that regions
unaffected by the edit are re-parsed only within bounded "gaps" and otherwise
reused from the previous parse. Two consumers share the fragment vocabulary:

1. `LRParser` (generated grammars): a continuous LR parse of the new document
   that opportunistically absorbs old-tree nodes at the live parse position
   (lr/parse.ts:395-408).
2. `@lezer/markdown`: a line-driven block parser that jumps over runs of old
   block subtrees when a context-hash test passes (markdown.ts:705, 742-759).

OBSERVED: partial/viewport parses are first-class: `Parser.startParse` accepts
`ranges` (common/parse.ts:147-155), `PartialParse.stopAt` bounds work
(:122-125), and a fragment from an incomplete parse carries `openEnd`
(`addTree(tree, fragments, partial)`, :69-73). Preserved across edits: old
`Tree` objects for unchanged regions (shared by object identity), per-node
`contextHash`/`lookAhead` records, and the fragment table itself.

OBSERVED (markdown): `README.md:3-11` — "@lezer/markdown … does not in fact
use the Lezer runtime (that runs LR parsers, and Markdown can't really be
parsed that way), but it produces Lezer-style compact syntax trees and
consumes fragments of such trees for its incremental parsing"; it is single-
pass and "doesn't validate link references" (README:15-18).

## 3. Retained representation

OBSERVED (LR path):
- The old `Tree` (immutable; `Tree` holds `children`/`positions` arrays;
  leaf runs are flat `TreeBuffer` quads `(id, start, end, size)`,
  common/tree.ts:351, :606) — retained in full.
- A `TreeFragment` list: each fragment = `from`/`to` in *updated-document*
  coordinates, a reference to the old `tree`, and `offset` (delta to add when
  going document→tree; common/parse.ts:33-51), plus `openStart`/`openEnd`
  flags marking edges that abut a change or a partial-parse boundary
  (:16, :50, :57-61).
- Per-node (not per-type) props on old nodes: `NodeProp.contextHash`
  (common/tree.ts:108) and `NodeProp.lookAhead` (:114; stored only when the
  tokenizer looked > 25 chars past the node end, lr/stack.ts:5-9 comment,
  common/tree.ts:110-113 doc).
- NO stored LR parse-state stacks at fragment edges: state lives only in the
  live `Stack` during a parse (`Stack.start` initializes from
  `ranges[0].from`, lr/stack.ts:69-73). There is no checkpoint file/table of
  states. INFERRED: reconstruction of parser context at fragment boundaries is
  achieved by *parsing forward* into the fragment and testing acceptance, not
  by restoring a saved state.

OBSERVED (markdown path):
- Same old-tree + `TreeFragment` list, plus a markdown-side `FragmentCursor`
  holding a `TreeCursor` into the active fragment and a computed
  `fragmentEnd` (last full line of the fragment; markdown.ts:1823-1864).
- The live composite-block stack (`CompositeBlock[]`) with a rolling 32-bit
  hash: `hash = (parentHash + (parentHash << 8) + type + (value << 4)) | 0`
  (markdown.ts:3-6); every block tree built under a context carries
  `NodeProp.contextHash = hash` (`hashProp`, :22).
- `reusePlaceholders: Map<Tree, Tree>` for taken nodes that span input-range
  gaps (:651).

## 4. Reuse unit

OBSERVED — the system has TWO reuse units, one per consumer:

A) LR engine — unit: a single old-tree NODE at the exact live parse position.
The exact predicate, `advanceStack` (lr/parse.ts:395-408) plus
`FragmentCursor.nodeAt` (:58-102):

1. `nodeAt(pos)` yields a candidate only if the node's old-tree start equals
   the current stack position (`start == pos`, :83), the node lies inside the
   fragment's safe window (`start >= safeFrom`, :84; `end <= safeTo`, :86),
   and the per-node lookahead record does not reach the invalidated fragment
   end (`!lookAhead || end + lookAhead < fragment.to`, :87-88).
   `safeFrom`/`safeTo` come from `nextFragment` (:45-46): for `openStart`/
   `openEnd` fragments they are computed by `cutAt` (:13-25), which snaps to
   an old-tree node boundary at least `Lookahead.Margin = 25` characters
   inside the fragment (and never inside an error node, :18).
2. The candidate's node type must be resolvable in the current `NodeSet`
   (`nodeSet.types[cached.type.id] == cached.type`, :398) — guards grammar
   reconfiguration.
3. The live LR state must have a goto transition for the node's type:
   `match = parser.getGoto(stack.state, cached.type.id) > -1` (:398-399).
4. `cached.length > 0` (:399).
5. If a strict `ContextTracker` is configured, the node's `contextHash` must
   equal the live context hash (`(cached.prop(NodeProp.contextHash) || 0) ==
   cxHash`, :396, :399; prop documented common/tree.ts:105-108).
6. If the top candidate is a wrapper (e.g. repeat) node, the engine descends
   through children positioned at offset 0 to find an inner reusable node
   (:404-407). `TreeBuffer` children are never reuse candidates (:97-99 skip
   buffers).
7. Execution: `stack.useNode(cached, match)` (lr/stack.ts:222-235) advances
   `pos`/`reducePos` by `value.length`, pushes the goto state, and writes a
   buffer record with size `-1` referencing the reused tree.

B) @lezer/markdown — unit: a run of whole BLOCK subtrees (each including its
inline content) between line starts. Predicate chain, per line, in
`advance` (markdown.ts:705) → `reuseFragment` (:742-759) → `FragmentCursor`:

1. `moveTo(absoluteLineStart + basePos, absoluteLineStart)` (:1843-1864): a
   fragment must cover the line start (`fragment.from <= lineStart`), and the
   old-tree cursor is walked (`childAfter`/`parent`) to the first block at or
   after the position.
2. `matches(this.block.hash)` (:1866-1869): the candidate's
   `NodeProp.contextHash` must equal the rolling hash of the *live* composite
   block stack.
3. `takeNodes(cx)` (:1871-1908): candidate blocks are taken while
   `cur.to - off <= fragEnd` (:1876), where `fragEnd` is the fragment's last
   full line minus 1 when `openEnd` (:1872; `fragmentEnd` computed by
   scanning back to a line start, :1849-1854). Taken nodes are added verbatim
   via `cx.addNode(cur.tree!, pos)` (:1882).
4. The taken run ends only at a "safe" block: "Taken content must always end
   in a block, because incremental parsing happens on block boundaries. Never
   stop directly after an indented code block, since those can continue after
   any number of blank lines" (:1893-1900 comment); implemented via
   `NotLast = [Type.CodeBlock, Type.ListItem, Type.OrderedList,
   Type.BulletList]` (:1819-1821): `end` does not advance past NotLast blocks
   (:1893-1897), so the next line is re-examined by the normal block parsers.
   Blocks that are not `type.is("Block")` are still taken but never terminate
   a run (:1890, :1901).

## 5. Damage detection / invalidation

OBSERVED: the host (CodeMirror) computes `ChangedRange`s; Lezer itself only
maps them onto the fragment table via `TreeFragment.applyChanges`
(common/parse.ts:78-100):

- Fragments are walked against the change list; each fragment is cut at change
  edges (`fFrom = max(cut.from, pos) - off`, `fTo = min(cut.to, nextPos) -
  off`, :88) and re-emitted with `openStart = (cI > 0)`, `openEnd = !!nextC`
  (:89) — a fragment edge adjacent to a change is "open" and later shrunk via
  `cutAt` (§4.A.1).
- The edited span itself is dropped (fragments spanning it are split into
  before/after parts).
- `minGap = 128` (default :78): fragments whose start lies within 128
  characters of a change are discarded entirely (`nextPos - pos >= minGap`
  gates the emit loop, :85) — INFERRED: an anti-microfragment policy trading
  a bounded strip of reuse for table compactness.
- Position maintenance is delta application, not rebuilding:
  `off = nextC.toA - nextC.toB` (:97) shifts all later fragment coordinates.
- `addTree(tree, fragments, partial)` (:69-73) replaces the table with one
  fragment covering the whole new tree, keeping only old fragments that
  extend beyond `tree.length` (partial-parse tails).

OBSERVED (markdown): no separate invalidation pass — damage is implicit:
a line fails the §4.B tests whenever (a) no fragment covers it, (b) the block
hash differs, or (c) the candidate would end past `fragEnd`; the line then
goes through normal block parsing, and fragment reuse is re-tried on the next
line (`advance` loop, :685-727).

## 6. Restart rule

OBSERVED (LR): there is exactly one parse, started once at
`ranges[0].from` (`Stack.start`, lr/stack.ts:69-73; `Parse` ctor
lr/parse.ts:262-263). The parse advances continuously; at every step,
`advanceStack` first probes `this.fragments.nodeAt(stack.pos)` (:397).
"Gaps" between fragments are parsed normally by the tokenizer/LR machine;
reuse resumes when a fragment node lands exactly on the live position and the
live state's goto accepts it (§4.A). There is NO jump, NO state restore, and
NO restart from a stored checkpoint. Fragment gating is itself conditional:
fragments are consulted only when
`fragments.length && stream.end - from > parser.bufferLength * 4`
(:264-265; `DefaultBufferLength = 1024`, common/tree.ts:4).

OBSERVED (markdown): restart points are line starts. At each new line, before
dispatching block parsers, `reuseFragment(line.basePos)` (markdown.ts:705)
attempts the §4.B chain; on success the parse jumps:
`absoluteLineStart += taken`, `lineStart = toRelative(...)`, next line read
(:746-757). The composite-block context is reconstructed live (never
restored): `readLine` re-runs the `skipContextMarkup` handlers of every
enclosing composite block to consume container markup (:832-852), and
`startContext` extends the rolling hash (:866-869).

## 7. Convergence rule

OBSERVED (LR): "convergence" is implicit — the parse catches up with a
fragment and absorbs its nodes until the cursor passes `safeTo`; there is no
explicit suffix-equality check. False-reuse hazards are guarded by:
(a) `cutAt` + `Lookahead.Margin = 25` inward snapping at open edges
(lr/parse.ts:13-25, :45-46); (b) the per-node `lookAhead` record refusing
reuse when the node's tokenizer lookahead region touches the invalidated end
(:87-88); (c) strict `contextHash` equality (:396, :399); (d) live goto
acceptance (:398) — a node whose production no longer fits the current LR
state is never reused, so grammar-shape drift stops absorption.

OBSERVED (markdown-specific convergence rules — prime evidence):
- Context-hash equality of the entire enclosing composite-block stack
  (`CompositeBlock.create` hash, markdown.ts:3-6; `matches` :1866-1869).
  Editing anything that changes the stack (list marker char/indent `value`,
  quote depth) changes the hash of every node under it.
- `NotLast` rule (:1819-1821, :1893-1900): never end a taken run directly
  after a block that can continue across blank lines (indented code block,
  list items) — prevents falsely converging before a continuable block.
- Taken runs must end at a block boundary (:1893-1900).
- `fragmentEnd` excludes the fragment's trailing partial line; `openEnd`
  shrinks it by one more character (:1849-1854, :1872).

NOT FOUND (task-hint check): the hinted convergence prop
"`markdownSkip`" / "`skipAfter`" / a NodeProp making the incremental parser
skip ahead after fenced-code edits does NOT exist in @lezer/markdown 1.6.3.
OBSERVED: every `skip` hit in the pinned source is `skipSpace`/`skipSpaceBack`
(:147, :235, :240), `skipForList` (:205) or `skipContextMarkup`/`DefaultSkipMarkup`
(:215-233, field :1139) — container-markup skipping while reading lines, not a
convergence prop; no such NodeProp is defined or set anywhere, and CHANGELOG.md
mentions none. Nearest real analogs: (1) FencedCode content is unparsed raw
`CodeText`, so a whole FencedCode subtree can be reused verbatim — tested by
object identity in test-incremental.ts:238-253; (2) the `NotLast` indented-
`CodeBlock` guard above. Whether any older release carried such a prop:
UNKNOWN (not determinable from the pinned shallow clones).

## 8. Reconstruction

OBSERVED (LR): the new tree is built fresh from the live stack buffers via
`Tree.build` (`stackToTree`, lr/parse.ts:501-511). During `buildTree`,
size `-1` records are resolved through the parse's `reused` array and the OLD
`Tree` object is pushed directly as a child (`SpecialRecord.Reuse`,
common/tree.ts:1388-1392, :1409-1415). Result: fresh wrapper spine + shared
old subtrees (structural sharing by object identity; also the encoding for
`TreeElement` in markdown, markdown.ts:1358-1373: buffer record size -1).
`-3`/`-4` records re-attach `contextHash`/`lookAhead` per node
(common/tree.ts:1416-1421).

OBSERVED (markdown): taken block trees are inserted as-is
(`cx.addNode(cur.tree!)`, :1882). When a taken node would straddle an input
range gap (its span exceeds the current range), a zero-length dummy
`Paragraph` is inserted instead and registered in `reusePlaceholders`
(:1883-1886); after the parse, `finish` → `addGaps` → `injectGaps`
(:904-968) re-inflates the whole document tree to full input coordinates,
substituting the real taken trees for dummies and wrapping gap-crossing
children recursively (:939-968). Result: fresh Document/container spine with
shared block subtrees; positions re-anchored to the new document.

## 9. Position / range maintenance

OBSERVED: trees are parent-relative (`positions` arrays; TreeBuffer quads).
Fragments carry the only absolute mapping: `from`/`to` in updated-document
coordinates and `offset` = document→tree delta ("Add this when going from
document to tree positions, subtract it to go from tree to document",
common/parse.ts:42-46). `applyChanges` maintains this by delta application
(`off = toA - toB`), not by touching the old tree (common/parse.ts:82-98).
The markdown parser additionally distinguishes input-stream-absolute from
document-relative positions when ranges (gaps) are present:
`toRelative` subtracts the sizes of all gaps before a position
(markdown.ts:1911-1919) and `injectGaps` adds them back (:939-968). CodeMirror
glue (not extracted here) normally re-parses over full ranges so gaps are an
edge case; the multi-range machinery is still mechanism-relevant evidence.

## 10. Fallback

OBSERVED (LR): if no fragments are supplied, or the parse region is at most
`bufferLength * 4` (4096 chars by default), `this.fragments = null` and the
parse is a plain full LR parse of the range (lr/parse.ts:262-265). Within an
incremental parse, any position where `nodeAt` yields null (no fragment, past
`safeTo`, no exact-start node) is parsed normally by the tokenizer — the
"gap parse" IS the fallback and is retried per position, not per region.
Error recovery (`runRecovery`: force-reduce / insert / delete, dead-end
restart; lr/parse.ts:456-498) is orthogonal to fragment reuse.

OBSERVED (markdown): no fragments (`fragments.length ? new FragmentCursor(...)
: null`, markdown.ts:677) → pure line-by-line parse. Per-line failure of the
reuse chain → normal block parsers for that line, with reuse retried at every
subsequent line (§6). A full parse is simply the `fragments = []` case
(`parser.parse(doc)`, test-incremental.ts:43).

## 11. Correctness authority

OBSERVED mechanism rules (invariant-bearing):
- Fragment boundary invariants: open edges are shrunk to old-tree node
  boundaries ≥ 25 chars inside the fragment; reuse candidates must lie within
  `[safeFrom, safeTo]` and not have their recorded lookahead region reach the
  invalidated end (lr/parse.ts:45-46, :83-88).
- Acceptance invariants: LR goto of the live state must accept the node type;
  strict context hash must match (lr/parse.ts:396-399).
- Markdown block invariants: composite-stack hash equality; run must end at a
  block; never end after a `NotLast` block; trailing partial line excluded
  (markdown.ts:3-6, :1866-1869, :1871-1908, :1819-1821).

OBSERVED heuristic (tunable, not invariant): `minGap = 128` fragment trim
distance (common/parse.ts:78); `Lookahead.Margin = 25` (lr/stack.ts:9);
`bufferLength * 4` fragment gate (lr/parse.ts:264).

OBSERVED test oracle (markdown repo): test-incremental.ts builds states with
`TreeFragment.addTree/applyChanges` (using `minGap = 2` in tests, :54) and
asserts (a) correctness — `compareTree(state.tree, parser.parse(state.doc))`,
i.e. incremental result equals a clean full parse of the same document
(:75, :83, :95, :109, :159, :174) — the same self-equivalence shape as the #22
oracle; and (b) reuse diagnostics — an `overlap()` percentage of block-span
sharing with the old tree with minimum floors (>80% local edit :89, >90% large
document :145, >80% list-item reuse :197) and one object-identity assertion
that a FencedCode subtree is the identical old object (:252). Under R0 §5,
(b) would be work/reuse instrumentation only, never correctness — consistent
with how upstream treats it. NOTE: the markdown parser itself is explicitly
not CommonMark-complete (no link-reference validation, README:15-18), which
bounds what its self-equivalence oracle can certify for reference-heavy
inputs.

## 12. Known limitations

- OBSERVED: LR reuse requires exact alignment between an old node's start and
  the live parse position plus goto acceptance; there is no search for
  structurally matching subtrees elsewhere in the old tree. Nodes strictly
  inside gaps are never reused; only whole nodes whose starts the continuous
  parse reaches can be absorbed (lr/parse.ts:395-408).
- OBSERVED: `TreeBuffer` leaf runs are never reuse candidates in the LR path;
  they are shared only as parts of a reused enclosing `Tree`
  (lr/parse.ts:97-99 vs common/tree.ts:1409-1415).
- OBSERVED: markdown reuse is whole-block granularity; a one-character edit
  inside a large paragraph forces re-inline-parsing of that entire paragraph
  (taken runs end at block boundaries; damaged block not reused,
  markdown.ts:1893-1900).
- OBSERVED: incremental correctness bugs shipped and were fixed across
  releases — CHANGELOG.md:141 (1.0.3 "Fix a crash doing an incremental parse
  on input ranges with gaps"), :133 (1.0.4 "another bug in incremental
  parsing across input gaps"), :127 (1.0.5 "another issue in reuse of nodes
  when the input has gaps") — evidence the gap/position invariants are subtle.
- HYPOTHESIS FOR R3/R8: the 32-bit rolling block hash (`| 0`, markdown.ts:6)
  is a heuristic vouching device; adversarial documents could collide two
  different composite stacks. Not demonstrated; would need a crafted case.
- HYPOTHESIS FOR R3/R8: `minGap` trimming plus `cutAt` margins mean edits near
  fragment edges systematically forfeit up to ~128+25 chars of reuse per
  fragment edge; repeated edits could keep a region permanently unreusable.
  Mechanism exists (OBSERVED); the sustained-loss behavior is hypothetical.

## 13. Adversarial hypotheses

All HYPOTHESIS unless marked OBSERVED (no measurements performed; Q1-Q12 of
the R2 protocol):

- Q1 (what old info survives): old Trees, per-node `contextHash`/`lookAhead`,
  and fragment table survive; from a partial parse, `addTree(..., partial)`
  keeps tail fragments (common/parse.ts:69-73). Reused nodes keep stale
  prop values into the new tree. OBSERVED mechanisms; net effect HYPOTHESIS.
- Q2 (who vouches): three vouching devices — LR live goto acceptance +
  context hash (lr/parse.ts:396-399); markdown rolling stack hash
  (markdown.ts:3-6, :1866); fragment safe windows + lookahead records
  (lr/parse.ts:45-46, :87-88). OBSERVED.
- Q3 (maximally invalidating edit): an edit that changes an early composite
  block's identity (e.g. list marker character or quote marker) changes the
  rolling hash for the whole subtree under it, failing `matches` until the
  damaged container closes and the outer hash matches again. OBSERVED hash
  dependence (markdown.ts:3-6); "reuse resumes at container close" HYPOTHESIS.
- Q4 (tiny edit → O(N)): plausible via Q3 (hash invalidated to end of a huge
  container) or via damage inside a viewport-parse tail. Mechanism exists;
  scaling claim unproven. HYPOTHESIS.
- Q5 (far-forward propagation): markdown has no global pass — reference
  definitions are parsed in place and never validated (README:15-18), so a
  definition edit does not propagate to earlier links (they are parsed as
  links regardless). OBSERVED absence of propagation. LR: per-node lookahead
  records are the only forward coupling. OBSERVED.
- Q6 (parse saved vs reconstruction paid): reuse saves tokenization+reduction
  but pays `Tree.build` over all stack buffers, `injectGaps` recursion, and
  `toRelative` conversions per taken node. Costs exist (OBSERVED code paths);
  net tradeoff HYPOTHESIS.
- Q7 (position maintenance erasing reuse): fragment delta application is
  O(fragments × changes) and never touches trees (common/parse.ts:82-98);
  unlikely to erase reuse. HYPOTHESIS (low prior).
- Q8 (memory ∝ N): the full old tree is retained alongside fragments
  (∝ change count); reuse shares subtrees rather than duplicating them
  (common/tree.ts:1409-1415). Retained-size growth HYPOTHESIS.
- Q9 (hidden parser state): only per-node `contextHash`/`lookAhead` props
  persist between parses; LR state and markdown block stacks are rebuilt
  (lr/stack.ts:69-73; markdown.ts:866-869, :832-852). OBSERVED.
- Q10 (false convergence): guarded by §7 rules; upstream changelog documents
  real false-reuse/gap bugs fixed post-release (CHANGELOG.md:127-141). Past
  unsoundness OBSERVED; present soundness HYPOTHESIS pending fuzzing.
- Q11 (fallback frequency): gate `stream.end - from > bufferLength * 4`
  disables fragments for small parses entirely (lr/parse.ts:264); short
  documents and small viewports fall back to full parse. OBSERVED gate;
  real-world frequency HYPOTHESIS.
- Q12 (mechanism-intrinsic vs implementation-specific): intrinsic — fragment
  model (offset/open flags/minGap), goto-anchored absorb, rolling context
  hash, NotLast/block-boundary/line-rounding rules, cutAt margins,
  lookahead records. Implementation-specific — JS arrays/WeakMap node caches,
  `DefaultBufferLength = 1024` multiples, `minGap = 128`/`Margin = 25`
  constants, CodeMirror's change-range production.

## 14. Relevance assessment

Fidelity boundary first: a Markit mechanism model would NOT reproduce Lezer's
generated LR tables / lezer-generator, the JS runtime object model,
`parseMixed` nested-language plumbing, or CodeMirror's viewport scheduling
(`stopAt`/`parsedPos` driving). Extractable are the *mechanism* concepts:
fragment table maintenance (split/drop/minGap/delta offsets/open flags),
state-anchored node absorption, context-hash vouching, and the markdown
convergence rules (§7).

Relation to the frozen horses (assessment only, per R0 §2 role 6):

- H2 (FRAGMENT_REUSE, prior-art anchor "Lezer-style reusable fragments"):
  SUPPORTED as an anchor — fragments are real, and all reuse is gated through
  the fragment table (`applyChanges` → safe windows → consumers).
- H3 (OLD_TREE_SUBTREE_REUSE): the task question — is Lezer pure
  fragment-lookup reuse, or also node-level reuse within gaps? Evidence:
  BOTH. The LR consumer reuses individual old-tree NODES — but only inside
  fragment windows and only when the live parse reaches their exact start
  with a goto-accepting state (lr/parse.ts:395-408); the markdown consumer
  reuses whole BLOCK SUBTREES located by a fragment cursor and vouched by a
  context hash (markdown.ts:1871-1908). No consumer scans the old tree for
  structurally matching subtrees independent of parse position/state — that
  is the tree-sitter-style differentiator.
- HORSE_BOUNDARY_AMBIGUITY: the H2/H3 split must therefore be defined by the
  LOOKUP/VOUCHING strategy (fragment-window + live-state anchoring vs
  old-tree consultation + structural matching), NOT by reuse granularity
  ("region vs node"), because Lezer exhibits node-level and block-level
  subtree reuse while still being the canonical H2 anchor. If H2 is
  implemented as "fragments only, nodes inside gaps reparsed", it is a
  strict subset of what Lezer does; a faithful "Lezer-inspired" H2 model
  includes state-anchored absorption at fragment boundaries. This blurring
  should be recorded in the horse definitions rather than resolved by
  fiat — OBSERVED evidence, classification HYPOTHESIS-level per the
  fidelity naming rule (R0 §3).
- Markdown-specific takeaways for BENCH-GRAMMAR-v1 mutations: the rolling
  composite-block hash, `NotLast` continuation blocks, block-boundary run
  termination, and trailing-partial-line exclusion are exactly the
  convergence/invalidation behaviors that structural edits (paragraph
  split/merge, list depth change, fence open/close) will exercise (§7).
- The hinted "skip prop" for fenced code must NOT be cited in downstream
  documents — it does not exist in the pinned versions (§7 NOT FOUND); use
  the FencedCode-identity-reuse test (test-incremental.ts:238-253) and the
  `NotLast` CodeBlock guard instead.

## 15. Manifest entry

```toml
[[prior_art]]
id = "lezer-common"
name = "@lezer/common (TreeFragment, Tree/TreeBuffer, Parser API)"
source_type = "official-upstream-repository"
repo_url = "https://github.com/lezer-parser/common"
commit_sha = "de5f96276a2954c249de1475e8b03f79c20d9ce4"
tag = "1.5.2"
retrieved_date = "2026-09-16"
relevant_paths = [
  "src/parse.ts",
  "src/tree.ts",
  "src/index.ts",
  "src/mix.ts",
]
license = "MIT"
notes = "TreeFragment.addTree/applyChanges (minGap=128, openStart/openEnd, delta offsets); per-node NodeProp.contextHash/lookAhead; buildTree SpecialRecord.Reuse structural sharing. Secondary: Context7 /lezer-parser/common."

[[prior_art]]
id = "lezer-lr"
name = "@lezer/lr (LR engine; resolved home of lezer-parser/lezer)"
source_type = "official-upstream-repository"
repo_url = "https://github.com/lezer-parser/lr"
commit_sha = "f81d6a25c3482aa7fc12434e9adea9d75c56ad08"
tag = "1.4.8"
retrieved_date = "2026-09-16"
relevant_paths = [
  "src/parse.ts",
  "src/stack.ts",
  "src/constants.ts",
  "src/token.ts",
]
license = "MIT"
notes = "FragmentCursor.nodeAt + advanceStack reuse predicate (goto-anchored node absorption; cutAt/Lookahead.Margin=25 safe windows; strict contextHash); Parse ctor fragment gate (bufferLength*4); Stack.useNode size=-1 reuse records; no stored parse-state stacks. lezer-parser/lezer is a forwarding stub (main a4284ee6ef0d529a07161f3534c8ef1aaa8efc99, no tags). Secondary: Context7 /lezer-parser/lr."

[[prior_art]]
id = "lezer-markdown"
name = "@lezer/markdown (incremental Markdown parser)"
source_type = "official-upstream-repository"
repo_url = "https://github.com/lezer-parser/markdown"
commit_sha = "9942d7ce41d734d743cbdb48177dddc1975fdc5c"
tag = "1.6.3"
retrieved_date = "2026-09-16"
relevant_paths = [
  "src/markdown.ts",
  "src/extension.ts",
  "src/nest.ts",
  "test/test-incremental.ts",
  "README.md",
  "CHANGELOG.md",
]
license = "MIT"
notes = "Does not use the LR runtime (README). Block-level subtree reuse via markdown-side FragmentCursor (moveTo/matches(contextHash)/takeNodes); CompositeBlock rolling hash; NotLast + block-boundary + line-rounded fragmentEnd convergence guards; reusePlaceholders + injectGaps for range gaps; compareTree-vs-clean-parse oracle. NO 'markdownSkip'/'skipAfter' NodeProp exists in this version. Secondary: Context7 /lezer-parser/markdown."
```
