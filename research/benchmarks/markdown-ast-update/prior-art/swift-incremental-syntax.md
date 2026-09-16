# Prior-art mechanism record — Swift incremental syntax parsing (R2)

Status: PRIOR_ART_RECORD (evidence extraction only; no benchmark execution, no timing claims)
Recorded: 2026-09-16

## 1. Identity

PROJECT: `swiftlang/swift-syntax` (formerly `apple/swift-syntax`) — the SwiftSyntax/SwiftParser
Swift-package libraries. Incremental-parse support lives in the `SwiftParser` module.

THREAD: Alex Hoppen (ahoppen), "Incremental syntax parsing", Swift Forums, 2018-05-03,
https://forums.swift.org/t/incremental-syntax-parsing/12368 — official-maintainer design
proposal for the C++ Swift compiler (SyntaxParsingContext, lexer fast-forward). It does NOT
describe the pinned SwiftParser implementation verbatim; divergence is recorded in §5/§7/§12.

VERSION / SHA: repo https://github.com/swiftlang/swift-syntax, pinned tag `604.0.0` (also
tagged `swift-6.4.0-RELEASE`; latest stable release at retrieval), resolved commit
`050f1a346fbbac0ca2cfb15a95274f7bd1cf0ccf` (shallow clone), retrieved 2026-09-16,
license Apache-2.0 (with Runtime Library Exception per source headers, LICENSE.txt).

LANGUAGE: Swift (parser implementation and parsed language both Swift).

PRIMARY SOURCES: pinned repo source at `604.0.0`; forum thread main post (author Alex
Hoppen). No follow-up post added mechanism detail beyond the main post; one fetch, no retry.

SECONDARY SOURCES: in-repo `Release Notes/600.md`, `Release Notes/601.md` (API rework);
in-repo test oracle `Tests/SwiftParserTest/IncrementalParsingTests.swift` +
`Sources/_SwiftSyntaxTestSupport/IncrementalParseTestUtils.swift`.

RELEVANT FILES / FUNCTIONS / SECTIONS (file:line at `604.0.0`):
- `Sources/SwiftParser/IncrementalParseTransition.swift` — `Parser.loadCurrentSyntaxNodeFromCache(for:)`
  :20-32 (on hit advances lexer by `node.totalLength.utf8Length`, :26-28);
  `Parser.registerNodeForIncrementalParse(node:startToken:)` :34-39 (records affect range);
  `ReusedNodeCallback` :50; `IncrementalParseTransition` :54-90 (retained state);
  `IncrementalParseLookup.lookUp` :125-134 / `cursorLookup` :136-148 /
  `nodeAtCursorCanBeReused` :150-189 (THE reuse predicate); `translateToPreEditPosition`
  :191-206 (post-edit -> pre-edit offset mapping); `SyntaxCursor` :211-275 (monotone
  in-order walk of old tree); `ConcurrentEdits` :288-418 (pre-edit coordinates, sorted,
  non-overlapping).
- `Sources/SwiftParser/Parser.swift` — `LookaheadTracker` :908-918 (one monotonic
  `furthestOffset` per parse); `LookaheadRanges` :936-948 (`[RawSyntax.ID: Int]`, bytes
  looked ahead from node start); `Parser.parseLookup` :116 / init :249-253.
- `Sources/SwiftParser/Lexer/LexemeSequence.swift` — `Lexer.tokenize` :152-171 (returns a
  LAZY cursor-based `LexemeSequence`); `advance(by:currentToken:)` :94-104 (reuse skip;
  "Important: This should only be used for incremental parsing" :93).
- `Sources/SwiftParser/ParseSourceFile.swift` — `Parser.parseIncrementally(source:parseTransition:)`
  :119-125; `IncrementalParseResult` :145-156 (tree + lookahead-ranges bundle).
- `Sources/SwiftParser/TopLevel.swift` — `parseCodeBlockItem` :151-245; cache consult
  :156-158; fresh-node registration :243.
- `Sources/SwiftParser/Declarations.swift` — `parseMemberBlockItem` :958-1030; cache consult
  :960-962; fresh-node registration :1027.
- `Sources/SwiftSyntax/Syntax.swift` — `position` :89-91; lazy layout-data materialization
  with `absoluteInfo` accumulation :456-473. `Sources/SwiftSyntax/AbsoluteSyntaxInfo.swift`
  :14-53 (offset derived by sibling advance). `Sources/SwiftSyntax/SourceEdit.swift`
  :77-80 (`intersectsOrTouchesRange`).

## 2. Problem actually solved

IDE/IDE-service reparsing: keep the previous syntax tree in memory and avoid reparsing
unchanged regions after small edits. OBSERVED (forum): goal is incremental reparse cost
"(nearly) in O(1) in terms of the size of the original source file" (author's stated design
goal, not a measurement); consumer is SourceKit, which "keeps the latest syntax tree in
memory" for syntax coloring. The pinned library exposes the same capability publicly via
`Parser.parseIncrementally(source:parseTransition:)` (ParseSourceFile.swift:53-70 doc).

## 3. Retained representation

OBSERVED — `IncrementalParseTransition` (IncrementalParseTransition.swift:54-90) holds:
1. `previousIncrementalParseResult` — the previous `SourceFileSyntax` tree PLUS
   `LookaheadRanges` (ParseSourceFile.swift:145-156).
2. `edits: ConcurrentEdits` — edits in PRE-EDIT coordinates, non-overlapping, increasing
   offset order (:277-287, :395-412).
3. optional `reusedNodeCallback` (:50) — diagnostics/testing only.

`LookaheadRanges` is `[RawSyntax.ID: Int]` (Parser.swift:941): per registered node, the UTF-8
bytes "that the parser looked ahead to parse the node, measured from the start of the node's
leading trivia" (Parser.swift:938). Computed during the PREVIOUS parse via
`LookaheadTracker`, a monotonic max of the furthest lexeme end reached by `advance()`/`peek()`
(Parser.swift:908-918, LexemeSequence.swift:71-73), snapshotted per node at registration
(IncrementalParseTransition.swift:34-39: `furthestOffset - offsetToStart(startToken)`).

Trivia state lives INSIDE reused nodes; no separate trivia table. No lexer state is retained
across parses. UNKNOWN: any successor of the 2018 compiler design's lexer-state-settling
logic — no such machinery exists in pinned SwiftParser.

## 4. Reuse unit

OBSERVED. The unit is a whole top-level statement item (`CodeBlockItem`, TopLevel.swift:156)
or member declaration item (`MemberBlockItem`, Declarations.swift:960), including all
descendants and the item's leading/trailing trivia. These are the ONLY cache-consultation
kinds in the parser (only call sites of `loadCurrentSyntaxNodeFromCache`).

Reuse identity/validation (`nodeAtCursorCanBeReused`, IncrementalParseTransition.swift:150-189):
1. exact position: `node.position != prevPosition -> false` (:155-157), `prevPosition` being
   the pre-edit image of the current post-edit offset;
2. kind match: `node.raw.kind != kind -> false` (:158-160), `kind` = what the parser EXPECTS
   there (`.codeBlockItem` / `.memberBlockItem`);
3. fast path: if the last edit ends strictly before the node position, reuse (:162-166) —
   sound because no remaining edit can intersect a range starting at `node.position`;
4. otherwise the node must have a recorded lookahead range (:168-172) and NO edit may
   `intersectsOrTouchesRange` the affect range `node.position ..< node.position +
   lookaheadLength` (:174-186). Comment: "Check if this node or the trivia of the next node
   has been edited. If it has, we cannot reuse it." (:177-178).

Old-tree navigation is NOT a hash lookup: `SyntaxCursor` walks the old tree in source order
with ascending positions (`advanceToNextNode(at:)`, :264-274) — a monotone merge walk.

## 5. Damage detection / invalidation

OBSERVED. No upfront damage computation. Damage is detected lazily, per item, during the
fresh parse: translate the current post-edit offset to its pre-edit image
(`translateToPreEditPosition` :191-206 — walk sorted edits; a position inside an inserted
replacement yields `nil`; else shift by `range.length - replacementLength`), then apply the
§4 predicate against the old-tree cursor.

Lookahead/affect regions: the 2018 thread's "lookahead region" survives ONLY as the
`LookaheadRanges` affect range recorded during the previous parse — the span whose bytes the
parser actually consulted (node text plus following trivia/tokens it peeked at). Since
`furthestOffset` is a global monotonic max, a node's affect range can overshoot into the
next item's leading trivia (intended — :177-178) but never lags behind the node itself.

Lexing: OBSERVED divergence from the 2018 thread. Pinned `Lexer.tokenize` returns a lazy
cursor-based `LexemeSequence` (LexemeSequence.swift:152-171); tokens are lexed on demand.
On reuse, `advance(by:)` (LexemeSequence.swift:94-104) jumps the cursor past the item's
bytes; interior bytes are never lexed in the new parse. There is NO edit-triggered re-lex
from the change and NO "lex until lexical state settles" loop in pinned SwiftParser.

## 6. Restart rule

OBSERVED. The parse always restarts at the file start; there is no computed restart offset.
The fixed checkpoint set is: start of every `CodeBlockItem` and every `MemberBlockItem`. At
each checkpoint the parser either (a) reuses the old item and skips its bytes, or (b) parses
freshly. Context at a checkpoint is whatever the fresh parser holds (nesting, `#if` bodies,
`stopCondition` closures); it is not reconstructed from the old tree. Bounded: O(1)
predicate work per checkpoint (sorted-edit early break at :179-181), cursor advance amortized
forward. No backward search for a better restart point.

## 7. Convergence rule (transition-point compatibility)

OBSERVED — most important section. There is NO post-hoc "parser state converged with the old
tree" check and NO one-time splice of the remaining suffix. EVERY item boundary applies the
same pre-reuse compatibility test (§4), and reuse decisions interleave: item A reused, item B
(edited) reparsed fresh, item C reused again. The full compatibility condition, as
implemented:

```
reuse(oldNode)  iff
   oldNode.position == translateToPreEditPosition(currentOffset)      // :155-157, :191-206
&& oldNode.kind      == kind expected by fresh parser at that point    // :158-160
&& ( all edits end before oldNode.position                             // :162-166 fast path
     || ( oldNode has a recorded lookaheadRange                        // :168-172
          && no edit intersectsOrTouches
             [oldNode.position, oldNode.position + lookaheadRange) ) ) // :176-186
```

`intersectsOrTouchesRange` is inclusive at both ends (SourceEdit.swift:77-80): an edit
ABUTTING the affect range also blocks reuse. What "agreement" suffices: same byte position
(post-edit mapped), same node kind, byte-identical affect range. The affect range is the
mechanism's proxy for "everything the parse of this item depended on is unchanged" — it is
byte-local; it compares neither parser states nor the item's semantic environment.

False convergence: the predicate can only return a node with identical bytes and matching
expected kind; residual risk is CONTEXT dependence — same bytes + same kind parsed under a
different surrounding construct would reuse the old interpretation. The 2018 thread hit this
class explicitly ("the outer initialiser parses as an initialiser declaration while the
inner init parses as an unknown node") and resolved it by making the invalid case parse
identically to the valid one (forum, author's caveat/mitigation). Whether every context
sensitivity of the current parser is neutralized this way is INFERRED, not proven (Q10).

## 8. Reconstruction

OBSERVED. The new tree is produced by an ordinary fresh left-to-right parse that builds all
new parent/collection structure. On a cache hit, `loadCurrentSyntaxNodeFromCache` returns the
old `Syntax` node and `parseCodeBlockItem` / `parseMemberBlockItem` wrap its `RawSyntax`
directly as the new item (TopLevel.swift:156-158, Declarations.swift:960-962); the lexer is
advanced by the node's UTF-8 length (IncrementalParseTransition.swift:26-28). The old raw
node is shared (same arena storage) between old and new trees — SwiftSyntax trees are
immutable, so sharing is safe by construction. Reused and fresh items interleave freely; no
bulk tail-splice operation exists. Every item (fresh at :243/:1027, reused at :157/:961)
re-registers a new affect range, so the returned `IncrementalParseResult` is self-contained
for the NEXT edit.

## 9. Position / range maintenance

OBSERVED. `RawSyntax` stores NO absolute offsets. `Syntax.position` is `absoluteInfo.offset`
(Syntax.swift:89-91), computed when a parent's child layout data is lazily materialized:
`absoluteInfo` starts at 0 for the root (AbsoluteSyntaxInfo.swift:46-53) and advances across
siblings by `+ totalLength.utf8Length` (AbsoluteSyntaxInfo.swift:17-28; Syntax.swift:456-473).
Consequences: (a) reused subtrees need ZERO offset patching — offsets re-derive from their
new location; (b) offsets are UTF-8 byte offsets (`AbsolutePosition.utf8Offset`,
AbsolutePosition.swift:14-17); (c) offset cost is paid lazily at traversal, not at parse time.

## 10. Fallback

OBSERVED. No explicit fallback mode, bail-out flag, or "force full reparse" condition. If no
lookup succeeds (no transition supplied, cursor exhausted `cursor.finished` :140, position
inside an inserted region :198-201, or predicate failure), the invocation IS a clean full
parse plus residual lookup/cursor overhead. Degradation is continuous and implicit, not a
mode switch.

## 11. Correctness authority

OBSERVED. The reuse rule is a conservative heuristic contract: identical bytes at the
translated position, identical expected kind, untouched affect range. No formal statement
that "reused result == clean parse" appears in the pinned source; the guard is
"decision inputs unchanged" reasoning.

Empirical authority is the in-repo oracle `assertIncrementalParse`
(IncrementalParseTestUtils.swift:42-132): (1) string round-trip of the incrementally parsed
tree (:66-77); (2) `assertSameStructure` against a from-scratch parse of the post-edit
source, `includeTrivia: true` (:80-85) — self-equivalence vs clean parse, the same oracle
family as #22's H0 comparison; (3) expected reused-node ranges/kinds (:87-131).
`IncrementalParsingTests.swift` (469 lines) exercises it. Test-time oracle, not a runtime
guarantee.

## 12. Known limitations

From the forum thread (author's own caveats, 2018 design):
- environment-dependent parsing had to be normalized (the `init` inside/outside type context
  example); reuse safety required making both parses identical;
- initial coverage limited to "the dominating syntax collections CodeBlockList and
  MemberDeclList" (still true at `604.0.0`: only two consult sites);
- tree serialization (JSON) judged too costly for the edit loop ("unacceptably high cost"),
  motivating in-memory tree retention;
- O(1)-ish cost is an aspiration ("(nearly) in O(1)"), not a guarantee.

From the pinned source (OBSERVED):
- reuse granularity fixed at the two item kinds; no intra-item or expression-level reuse;
- affect ranges derive from one monotonic `furthestOffset`, so ranges can overshoot (never
  undershoot the node) — conservatism, not precision;
- lexing of the affected item(s) plus every checkpoint token is redone; no token-level reuse;
- API in active rework: tuple-returning `parseIncrementally` and the old
  `IncrementalParseTransition` initializer deprecated in favor of `IncrementalParseResult`
  (ParseSourceFile.swift:71/85; IncrementalParseTransition.swift:64; Release Notes/600.md
  issue #2267 PR #2272); `IncrementalEdit` renamed `SourceEdit` (Release Notes/601.md:58-59);
- current production consumer status (e.g. SourceKit-LSP): UNKNOWN from this repo alone;
  only the 2018 design intent ("incremental parsing should be incorporated into the
  SourceKit service") is on record.

## 13. Adversarial hypotheses (all HYPOTHESIS unless marked OBSERVED)

- Q1 old info: mechanism is live at `604.0.0` (public `parseIncrementally` API); the 2018
  thread partially describes an OLDER compiler-side design — thread and code are two
  versions of one mechanism family.
- Q2 who vouches: Apple Inc. / Swift project maintainers (thread author = original design
  implementer); correctness vouched only by test oracle (§11), not proof.
- Q3 maximally invalidating edit: OBSERVED structure — an edit that abuts/overlaps a wide
  affect range blocks reuse broadly; an edit inside item 1 still allows reuse of later items
  whose affect ranges are clean (predicate is per-item; positions still translate).
  HYPOTHESIS: a parse whose first item's `furthestOffset` reached EOF (e.g. unterminated
  construct) gives item 1 an affect range to EOF, so a tiny edit inside it invalidates
  (nearly) all reuse.
- Q4 tiny edit -> O(N)?: HYPOTHESIS from structure (no timing): work is at minimum O(#items)
  (checkpoint consults + new list skeleton + one lexed token per boundary), NOT O(#bytes);
  interiors of reused items are never lexed. Whether skeleton rebuilding dominates at 16 MiB
  scale is a #22 benchmark question.
- Q5 far-forward propagation: OBSERVED cap — propagation distance is bounded by the damaged
  item's affect range; after one fresh item, later items can pass again. No forward state
  crosses item boundaries except the fresh parser's own state (rebuilt, not compared).
- Q6 parse saved vs reconstruction paid: reuse saves lex+parse of item interiors but pays
  new parent/collection nodes for ALL items plus predicate evaluation; net is HYPOTHESIS for
  #22 measurement.
- Q7 position maintenance: OBSERVED — none at update time; offsets derived lazily from tree
  structure (§9). Cost moves to first traversal; it does not disappear.
- Q8 memory ∝ N: OBSERVED — the old tree (or its arena) must stay alive; reused raw nodes are
  shared, the new tree adds O(#items) new structure; retained memory >= one full tree +
  lookahead map.
- Q9 hidden lexer/parser state: OBSERVED risk surface — a skipped region is never re-lexed,
  and the lexer restarts at the byte after the reused node with fresh state; safety relies on
  item boundaries being lexer-state-neutral and on the affect range having captured what
  mattered. INFERRED, not stated in code or comments.
- Q10 false convergence: same-bytes+same-kind at the same mapped position under a DIFFERENT
  parse context (e.g. bytes pre-existing at top level now inside a newly inserted `#if`
  clause) would pass the predicate. Whether current parser behavior makes such re-contexts
  parse-identical is UNVERIFIED — HYPOTHESIS; the 2018 thread shows the team resolves such
  cases by parse normalization, and `testAddElse` (no reused nodes expected,
  IncrementalParsingTests.swift:31-36) shows awareness at `#if` boundaries.
- Q11 fallback frequency: no counter exists in the pinned source (UNKNOWN); structurally,
  fallback frequency = frequency of checkpoints failing the predicate.
- Q12 mechanism-intrinsic vs implementation-specific: intrinsic — item-kind checkpoints,
  pre-edit position translation, kind+position+affect-range predicate, lazy skip, derived
  offsets, `IncrementalParseResult` re-registration. Implementation-specific — `SyntaxCursor`
  walk vs hash map, monotonic `furthestOffset` tracker, arena sharing, lazy `SyntaxData`
  materialization, bump-allocated lexeme states.

## 14. Relevance assessment (vs H0-H4)

- H4 RESTART_CONVERGENCE (R0 §3 anchor: "Swift incremental syntax concepts"): Swift IS the
  canonical modern anchor for this horse. Concepts to carry into a Markdown H4 model:
  fixed-density checkpoints (statement/item boundaries ~ Markdown block starts), forward-only
  parse from file start, per-checkpoint cheap compatibility predicate, reuse of the stable
  suffix through the same forward pass, re-registration of checkpoint metadata for the next
  edit. Key differences a Markdown H4 must decide for itself: Swift never "converges once and
  splices" — it re-evaluates at every checkpoint and tolerates reuse/damage interleaving; and
  its compatibility test is byte-local (position+kind+untouched affect range), not
  parser-state equality. Markdown block-boundary state (container depth, fence state) is NOT
  byte-local, so a Swift-style byte-local predicate is a HYPOTHESIS-level fit at best for
  H4's "semantic/parser-state convergence".
- H3 overlap: Swift also retains the old tree and reuses compatible subtrees from it (like
  H3/Tree-sitter), but only at two fixed kinds via a monotone cursor walk — not
  arbitrary-position subtree matching with validity ranges. Swift sits ON the H3/H4 boundary
  (see below); the #22 horses split it more cleanly than Swift splits itself.
- H0/H1/H2: no mechanism contribution (H0 comparison exists only as the test oracle).
- Fidelity boundary: a Markit H4 model would be `swift-incremental-syntax-inspired`, NOT a
  reproduction. We would NOT reproduce: the Swift grammar / generated parser, C++ compiler
  interop, SwiftSyntax arena/reference machinery, lazy `SyntaxData` layout materialization,
  the `Lookahead`/`LexemeSequence` cursor implementation, or the exact `CodeBlockItem` /
  `MemberBlockItem` kinds. We WOULD model: checkpoint-at-block-start restart choice, pre-edit
  position translation for retained state, a per-checkpoint compatibility predicate (kind +
  untouched influence range), per-item interleaved reuse, derived (unpatched) offsets.

HORSE_BOUNDARY_AMBIGUITY: Swift's design straddles H3 (old-tree subtree reuse) and H4
(restart + convergence at checkpoints). R0 names it an H4 anchor; this record extracts both
halves so R3/R8 can attribute without retrofitting.

## 15. Manifest entry

```toml
[[prior_art]]
id = "swift-incremental-syntax"
name = "swiftlang/swift-syntax (SwiftParser incremental parsing)"
source_type = "official-upstream-repository"
repo_url = "https://github.com/swiftlang/swift-syntax"
commit_sha = "050f1a346fbbac0ca2cfb15a95274f7bd1cf0ccf"
tag = "604.0.0"
retrieved_date = "2026-09-16"
relevant_paths = [
  "Sources/SwiftParser/IncrementalParseTransition.swift",
  "Sources/SwiftParser/Parser.swift",
  "Sources/SwiftParser/Lexer/LexemeSequence.swift",
  "Sources/SwiftParser/ParseSourceFile.swift",
  "Sources/SwiftParser/TopLevel.swift",
  "Sources/SwiftParser/Declarations.swift",
  "Sources/SwiftSyntax/Syntax.swift",
  "Sources/SwiftSyntax/AbsoluteSyntaxInfo.swift",
  "Sources/SwiftSyntax/SourceEdit.swift",
  "Sources/_SwiftSyntaxTestSupport/IncrementalParseTestUtils.swift",
  "Tests/SwiftParserTest/IncrementalParsingTests.swift",
  "Release Notes/600.md",
  "Release Notes/601.md",
]
license = "Apache-2.0"
notes = "Reuse unit: CodeBlockItem/MemberBlockItem only. Predicate nodeAtCursorCanBeReused (IncrementalParseTransition.swift:150-189): pre-edit position equality (translateToPreEditPosition :191-206) + expected-kind equality + no edit intersectsOrTouches affect range [node.position, +lookaheadRange) (LookaheadRanges, Parser.swift:936-948). Lazy lexing, advance(by:) skips unlexed (LexemeSequence.swift:94-104); no re-lex/lexer-state-settling at this tag. Offsets derived lazily, zero patching. No explicit fallback; degrades to clean parse. Oracle: assertIncrementalParse round-trip + assertSameStructure vs clean parse. APIs under rework (IncrementalParseResult 600, SourceEdit 601)."

[[prior_art]]
id = "swift-incremental-syntax-thread"
name = "Alex Hoppen, 'Incremental syntax parsing' (Swift Forums, 2018)"
source_type = "official-maintainer-design-thread"
thread_url = "https://forums.swift.org/t/incremental-syntax-parsing/12368"
retrieved_date = "2026-09-16"
license = "unknown-forum-post"
notes = "Author: Alex Hoppen (ahoppen), 2018-05-03, design for the C++ Swift compiler (SyntaxParsingContext), WIP apple/swift PR #16340. Old tree as cache keyed by source location; reuse iff same mapped position + same SyntaxKind + no edits in node range + no edits in next node's leading trivia; lexer fast-forward; consumer SourceKit; caveats: init-context parsing normalized, JSON serialization too costly, initial scope CodeBlockItemList/MemberDeclList, '(nearly) O(1)' goal. DIVERGES from pinned swift-syntax code; thread = design intent, pinned code = mechanism truth."
```
