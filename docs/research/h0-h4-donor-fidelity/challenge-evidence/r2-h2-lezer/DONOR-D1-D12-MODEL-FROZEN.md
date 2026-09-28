# Gate-A R2 donor-side model — H2 (Lezer fragment/context reuse) — FROZEN before horse inspection

Donor pins (verified via ssh, 2026-09-28):
- @lezer/common 1.5.2 = lezer-parser/common @ de5f96276a2954c249de1475e8b03f79c20d9ce4
- @lezer/lr 1.4.8 = lezer-parser/lr @ f81d6a25c3482aa7fc12434e9adea9d75c56ad08
- @lezer/markdown 1.6.3 = lezer-parser/markdown @ 9942d7ce41d734d743cbdb48177dddc1975fdc5c

All citations below verified by direct read of the pinned source (files mirrored under
/tmp/gate76-r2-lezer/donor-src/). Line numbers refer to the pinned SHAs.
Notation: common = @lezer/common/src/parse.ts; lr = @lezer/lr/src/parse.ts (lr-stack = lr/src/stack.ts);
md = @lezer/markdown/src/markdown.ts.

Since the H2 subject is a Markdown update benchmark, the DONOR SIDE for fidelity is
primarily the @lezer/markdown consumer (md FragmentCursor / CompositeBlock hash /
takeNodes guards), built on the @lezer/common TreeFragment substrate (applyChanges /
open edges / offset deltas). The LR consumer (lr FragmentCursor.nodeAt + advanceStack)
is the second consumer of the same substrate and defines "state-anchored absorption
inside gaps" (HORSE_BOUNDARY_AMBIGUITIES #1).

## D1 — retained representation
- Old Tree(s) (immutable; children/positions arrays; TreeBuffer leaf runs), plus a
  fragment table: each TreeFragment = {from,to in UPDATED-doc coords, tree, offset
  (doc->tree delta), open flags} (common:25-51). openStart/openEnd getters :57,:61.
- Per-node props retained on old trees: NodeProp.contextHash, NodeProp.lookAhead
  (md:22 hashProp; lr:87-88).
- Markdown consumer adds nothing persistent beyond fragments; live state is the
  CompositeBlock stack w/ rolling hash (md:661-662, 675-678).

## D2 — edit/damage propagation
- Host supplies ChangedRange {fromA,toA,fromB,toB} (common:5-14). Lezer does NOT diff.
- TreeFragment.applyChanges(fragments, changes, minGap=128) (common:78-100):
  fragments spanning a change are SPLIT (before-part emitted with openEnd in
  iteration cI; after-part emitted with openStart in iteration cI+1; edited span
  dropped); offset delta off = toA - toB applied to all later fragments (:97);
  fragments in unchanged stretches shorter than minGap are dropped entirely
  (:85 gate `nextPos - pos >= minGap`).
- addTree(tree, fragments, partial) (common:69-73): new whole-tree fragment
  [0,tree.length), keeps only old fragments with f.to > tree.length (partial tails),
  openEnd = partial.

## D3 — candidate discovery
- Markdown: per-LINE forward-only cursor over the ordered fragment list
  (md:1823-1841 nextFragment; moveTo walks fragments while fragment.to <= pos :1843,
  then a TreeCursor descend childAfter/parent to first block at/after pos :1858-1864).
  One candidate at a time at the live line; no search over all fragments, no index
  beyond array order.
- LR: FragmentCursor with nextStart monotonic gate (lr:57-61 `pos must be >= any
  previously given pos`), nodeAt walks children of the active fragment's tree
  (lr:63-101).

## D4 — eligibility / context compatibility
- Markdown chain (per line, md:705 -> 742-759 -> 1823-1908):
  (a) moveTo: a fragment must COVER the line start (fragment.from <= lineStart,
      md:1861 `this.fragment.from <= lineStart` via the c.from >= rPos branch :1860);
  (b) matches: candidate subtree's NodeProp.contextHash == live rolling hash of the
      composite-block stack (md:1866-1869; hash update md:3-6,26-30,866-869);
  (c) takeNodes guards: cur.to - off <= fragEnd (md:1876), fragEnd = fragmentEnd -
      (openEnd?1:0) (md:1872), fragmentEnd = last FULL line of fragment
      (md:1849-1854 scan back to \n); run must END at a Block node; never count a
      NotLast block (CodeBlock/ListItem/OrderedList/BulletList) as run end
      (md:1818-1821,1893-1901); anonymous wrapper descend (md:1877-1878).
- LR chain (lr:395-408): node.start == live pos, within [safeFrom,safeTo]
  (cutAt snaps >= Lookahead.Margin=25 chars inside open edges, lr:13-25,42-55;
  stack.ts:5-9), lookAhead prop must not reach fragment end (lr:87-88), nodeSet
  type identity (lr:398), goto accepts node type at live LR state (lr:398-399),
  length>0 (:399), strict contextHash equality when ContextTracker strict (:396,399);
  wrapper descent through offset-0 children (lr:404-407); TreeBuffers never
  candidates (lr:97-100).
- NO stored parse-state stacks anywhere: context is REBUILT by parsing forward
  (lr Stack.start stack.ts:69-73; md readLine re-runs skipContextMarkup md:832-852).

## D5 — reuse/reparse granularity
- Markdown: whole BLOCK SUBTREES per line-run (taken blocks include inline content;
  cx.addNode(cur.tree!, pos) verbatim md:1882); damage inside a block forfeits the
  whole block. Reuse retried EVERY line (md advance loop :685-727).
- LR: individual old-tree NODES absorbed at the exact live position INSIDE fragment
  windows (not only whole fragments): gaps between fragments are parsed normally but
  nodes at fragment starts whose live goto accepts are absorbed (lr:395-408).
  IMPORTANT (HORSE_BOUNDARY_AMBIGUITIES #1): "fragments only, gap interiors fully
  reparsed" is a STRICT SUBSET of Lezer behavior.

## D6 — candidate rejection behavior
- Markdown: moveTo false or matches false or taken==0 -> fall through to normal
  block parsers FOR THAT LINE (md:705 conditional; reuseFragment returns false);
  fragments are never removed on rejection; retry next line.
- LR: nodeAt null / no goto match -> normal tokenization at that position (per
  position, not per region); wrapper descent exhausted -> break (lr:404-407).

## D7 — fallback / progress semantics
- No fragments supplied (fragments.length==0) -> plain full parse (md:677
  `fragments.length ? new FragmentCursor : null`; lr:264-265 also gates on
  stream.end - from > bufferLength*4 = 4096 default).
- Partial parse: stopAt/parsedPos; partial fragments carry openEnd (common:69-73).
- Markdown fallback is per-line and total: full parse is just fragments=[] case.
- LR error recovery (runRecovery) orthogonal to reuse.

## D8 — coordinate / edit mapping
- Fragment from/to in updated-doc coords; offset = doc->tree delta (add doc->tree,
  subtract tree->doc) (common:42-46); applyChanges maintains via off=toA-toB (:97).
- Old trees are NEVER touched (immutable). Markdown multi-range machinery:
  toRelative subtracts gaps (md:1911-1919), injectGaps re-inflates (md:939-968),
  reusePlaceholders dummies for gap-crossing taken nodes (md:650-651,1883-1886).

## D9 — semantic / global dependency handling
- NONE in the reuse mechanism: md does not validate link references (README:15-18);
  definitions parsed in place, no propagation to earlier links. Per-node lookAhead
  props are the only forward coupling in LR. The contextHash is a *syntactic
  container-stack* hash, not a semantic dependency tracker.

## D10 — retained-state maintenance
- Fragment table rebuilt per edit via applyChanges (O(fragments x changes) walk,
  common:82-98); old trees shared by identity (structural sharing through Reuse
  records, common/tree.ts buildTree; md takes cur.tree! verbatim).
- Rolling context hash maintained live during every parse (md addChild rehash :26-30;
  startContext :866-869); hashProp attached per constructed subtree :22.

## D11 — candidate/search indexing vs enumeration
- Both consumers: FORWARD-ONLY linear advance over the ordered fragment array with a
  single active fragment + tree cursor. No index, no binary search, no free
  navigation of the old tree (that is H3's differentiator). LR additionally keeps
  nextStart to skip re-probing (lr:59,79).

## D12 — lifetime / retirement work
- Old generation trees become garbage when no fragment references them (JS GC);
  no explicit retirement pass. Fragments dropped by minGap or addTree replacement
  simply vanish from the table. Fresh wrapper spine is built every parse
  (stackToTree / block.toTree), so per-parse reconstruction cost is intrinsic.

## FROZEN donor-side expectations for H2-F1..F5 (markdown consumer lane)

H2-F1 (aligned fragment + compatible context -> reuse expected):
Donor: parse doc -> addTree -> single edit mid-paragraph -> applyChanges ->
fragment(s) for unaffected regions (before/after split, both open at cut edges for
adjacent ones); reparsing takes whole blocks of unaffected lines (matches hash ==
document-level hash 0 at top level). Reuse expected: yes for block-aligned regions
not within minGap of the edit; equality with clean parse maintained.

H2-F2 (fragment overlaps edit -> reject/trim/remap):
Donor: fragment spanning the edit is SPLIT into before/after parts with
openStart/openEnd set at cut edges; edited span dropped; after-part remapped by
delta; with default minGap=128, fragments in <128-char unchanged stretches are
dropped entirely. Safe-window shrinkage on open edges is a *consumer-side* effect
(LR cutAt); markdown's analogue is fragEnd -= 1 when openEnd (md:1872) and the
line-rounded fragmentEnd.

H2-F3 (unchanged bytes + incompatible context -> reuse rejected):
Donor: e.g. a line whose enclosing composite-block stack hash changed (list marker
char change, quote depth change) has node.contextHash != live block.hash ->
matches() false -> that line falls through to normal parsing; blocks under the
changed container are NOT reused even though their bytes are unchanged. Reuse can
resume after the container closes (outer hash matches again). Expected: rejection
at the hash gate, per line, with retry.

H2-F4 (edit shifts retained fragment position -> mapping behavior):
Donor: pure insertion/deletion before a fragment -> applyChanges emits the after-
fragment with from/to shifted by (toA-toB) and offset adjusted so doc->tree mapping
stays correct; the referenced old tree object and its contextHash are untouched;
reuse of the shifted fragment should succeed for unchanged content.

H2-F5 (multiple candidate fragments -> discovery/search behavior):
Donor: two edits in one change list -> applyChanges yields >=3 fragments (before,
between, after) with middle fragments kept only if inter-edit stretch >= minGap;
the markdown consumer advances linearly fragment-by-fragment as the parse position
moves forward; at a given line only ONE fragment is active (the first whose
to > lineStart). Expected: linear forward-only enumeration, no scoring/ranking
between candidates; fragments before the parse position are consumed and gone.

(For the LR lane the analogous F1/F5 behavior is nodeAt monotonic probing.)
