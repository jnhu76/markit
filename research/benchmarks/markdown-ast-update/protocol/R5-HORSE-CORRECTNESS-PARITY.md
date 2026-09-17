# R5 — H1/H2/H3/H4 Mechanism Correctness + Parity Decisions

Status: **R5 MECHANISM DECISION FREEZE (pre-implementation)** (2026-09-17)
Campaign: #22 MARKIT-MARKDOWN-BENCHMARK-1
Branch: `research/22-r5-horses-correctness-parity-1`
Base: `master` @ `21d7d832fec84fceedb7600cccb4296745395fc1`
(PR #29 merged; R4 `H0_REFERENCE_PASS`)
Authority chain: Issue #22 -> R0 -> R1 -> R2 -> R3 freeze -> R4 H0 -> R5
(this record). R5 decides implementation details earlier stages deliberately
left undefined; it does NOT redefine BENCH-GRAMMAR-v1, NORMALIZED-RESULT-v1,
CaseKeyV1, corpus/mutation/case authority, timer semantics, work-counter
semantics, horse identity, or prior-art factual claims.

This record freezes, BEFORE any substantial horse code, the per-horse
mechanism decisions required by the R5 task contract §4. It exists to
prevent implementation drift. It predicts no performance. Any later change
to a frozen field below must be recorded as an explicit amendment in the R5
stage record, never silently.

---

## 1. Shared parity substrate (task contract §5)

New workspace member `shared-grammar/` (crate `markit-mdbench-shared-grammar`)
holds ONLY non-research BENCH-GRAMMAR-v1 semantics, extracted from the R4 H0
parser line-for-line:

```text
lexical helpers          LF scan, space runs, heading_at, parse_marker,
                         refdef_at, fence opener/closer logic
label normalization      norm_label (ASCII fold, space collapse, trim)
reference table          flat first-wins Vec<(label, destination)>
block skeletons          Skel (Para/Heading/Quote/List/Item/Fence/Def)
                         + entry ContextKey per block
block state machine      the H0 line loop (container stack, fence, para)
                         as a resumable scanner: whole-document parse,
                         byte-range region parse, and a spliced-take hook
inline scanner           code spans, links, reference resolution, emphasis
materialization          Skel -> NORMALIZED-RESULT-v1 nodes with the
                         completed definition table
```

The shared crate does NOT own: damage detection, fragment lookup, old-tree
lookup, checkpoint selection, convergence, reuse policy, incremental
indexes, retained mechanism state, or any normalized-equality logic. The
splice hook is a horse-supplied closure — the shared scanner never decides
reuse.

H0 migrates onto the same pure primitives (identical algorithms, identical
inspection events). Regression bar for that migration: `verify-r4.sh` PASS,
43/43 fixtures byte-identical, H0 normalized checksums identical, H0 counter
semantics unchanged, H0 still FULL_REBUILD.

Dependency rule (Cargo-enforced): H1-H4 may depend on `common`, `oracle`,
`corpusgen`, `shared-grammar`. No horse depends on `mechanisms/full-rebuild`.
Test crates may use H0 as the correctness oracle; horses never call H0.

## 2. Shared grammar state: `ContextKey`

The explicit benchmark context state of BENCH-GRAMMAR-v1 at a block-start
line (after prefix consumption), recorded per block and compared at every
reuse/vouching/checkout decision:

```text
ContextKey =
  container stack, outermost..innermost:
    Quote                       | Item { marker, content_indent (strip) }
  fence state                   none | open { opener run length }
  (paragraph state is NEVER part of a block-entry key: dispatch happens
   only at lines where a paragraph is not mid-construction)
```

This is the benchmark analogue of the context dimensions the R2 records
enumerate (tree-sitter-markdown serialized scanner state: container stack
with per-item content indent + fence length; Lezer composite-block rolling
hash). It is deliberately EXPLICIT and structural (not a hash): equality is
by value, collisions impossible, no minimality claim. Components are exactly
the state BENCH-GRAMMAR-v1 dispatch (§2, §6, §7, §8) consumes at a block
start; tabs are ordinary characters (D1), so no tab column exists.

## 3. Counter semantics (frozen before any measurement; task contract §8)

Cumulative/gauge slots report through `MechanismContext` only; source
inspection is reported as raw byte-range events (the common collector owns
the union); PA is never computed by a horse. `nodes_reused` counts native
syntax nodes/subtrees structurally RETAINED (shared Arc identity or
pass-through ownership) across an update; `nodes_rebuilt` counts native
nodes newly constructed. Parser-work avoidance is never counted as
`nodes_reused`. `blocks_reparsed` counts BENCH-GRAMMAR block-level parse
units (Paragraph, Heading, BlockQuote, List, ListItem, FencedCode,
ReferenceDefinition — the H0 block-kind set) actually constructed by a
reparse entered this update. `metadata_records_touched` counts the horse's
own frozen metadata records actually inspected or mutated (per-horse record
kind below), never a value back-computed from document size.

Counter applicability (frozen; `Known` only where the mechanism owns the
concept):

```text
                       H1          H2          H3          H4
blocks_reparsed        Known       Known       Known       Known
nodes_rebuilt          Known       Known       Known       Known
nodes_reused           Known       Known       Known       Known
metadata_records_touched Known     Known       Known       Known
restart_distance       NotApplicable NotApplicable NotApplicable Known
convergence_distance   NotApplicable NotApplicable NotApplicable Known
fallback_to_full_count Known       NotApplicable NotApplicable NotApplicable
```

H2/H3 degrade naturally (lines/blocks reparse when reuse is refused); that
is NOT a fallback event. H4 restart-at-zero is a degraded H4 path, not a
switch to a separate full-rebuild mechanism, so it is not
`fallback_to_full` either. Only H1 has an explicit, counted full-parse
fallback. This matrix may not be renamed into compliance: H2 fragment
gating is not "convergence".

## 4. Eager completion boundary (task contract §7)

For every horse: `update`/`full_parse` return a `Pending` that ALREADY
holds the fully materialized new native state (block structure, inline
resolution, reference resolution, position mapping); `complete()` only
seals the state and derives the checksum. No parsing, damage calculation,
fragment validation, old-tree validation, convergence, reference
resolution, or required position repair may remain after `complete()`.
No lazy iterators, `OnceCell` parser work, deferred subtree validation, or
lazy repair. Proven per horse by the R5 eager-completion suite (structural:
`complete` receives no source/sink; behavioral: all counters and the full
inspection union are observed strictly before `complete()`).

## 5. QUERY strategy (task contract §19)

Every horse answers the frozen ordered batch
`NODE_PATH_AT(EARLY/MIDDLE/LATE)` from its COMPLETED retained state by
projecting the native state to `NormalizedDocument` (`NormalizeV1`) and
applying the oracle's `node_path_at`. No horse-specific query index exists.
The projection is pure traversal of already-complete state (no parser work).

---

## 6. H1 — BLOCK_LOCAL_REPARSE (`mechanisms/block-local`)

```text
MECHANISM
  Retained old top-level document tiling; map the edit to the damaged
  top-level region; clean-reparse ONLY that region; splice prefix blocks
  through, reconstruct the suffix with shifted spans; conservative,
  semantically defined full-parse fallback whenever the H1 model cannot
  soundly localize.

PRIOR_ART_ANCHOR
  mizchi-markdown-inspired (R2 MECHANISM-SOURCE-MAP H1; the shipped
  EditInfo/overlap/region-reparse/splice/delta-shift/definition-fallback
  design at pinned commit ffe7dc00).

FIDELITY_BOUNDARY
  UTF-8 byte coordinates (not upstream UTF-16); nodes_reused reports
  structural retention, never "not reparsed"; no JS handle layer; no
  vendor tuning. The upstream `reused_after`-style parser-work counter is
  NOT mapped to nodes_reused.

NON_GOALS
  No fragment table, no old-tree consultation, no checkpoints, no
  convergence, no content hashes, no block index beyond the retained
  tiling itself (linear scan is the mechanism).

MECHANISM_INTRINSIC_STATE (native retained state)
  `Vec<TopEntry>` tiling the WHOLE old document in order —
  TopEntry = Block { Skel } | BlankRun { span } — plus the retained
  definitions array (mizchi's `Document.definitions`, which doubles as the
  fallback trigger) and the old source length. Blank runs are first-class
  entries exactly as upstream BlankLines blocks are. Entry metadata for the
  soundness gates (paragraph/list/quote/fence-termination facts) is derived
  from the retained Skel + source, not separately indexed.

COMMON_INPUT
  old source, post source, canonical edit (shared substrate).

COMMON_INSTRUMENTATION
  R0 §10 slots via MechanismContext; inspection events only.

MODEL_DEFINED_STATE
  None beyond the tiling. No block index, no hashes, no dependency map,
  no context fingerprint table (R2 MODEL_DEFINED_CANDIDATE_STATE stays
  out).

DAMAGE / INVALIDATION RULE (mizchi-faithful)
  Linear scan of the retained tiling: an entry is affected iff
  entry.start < edit_end && entry.end > edit_start (strict overlap). If no
  entry overlaps (insert at an exact boundary), the insertion-gap mapping
  applies: region = [end of the entry before the gap, start of the entry
  after the gap). The reparse region is [end of the entry before the first
  affected entry, start of the first unaffected entry after the last
  affected entry) in old coordinates, right edge shifted by delta and
  clamped — the mizchi region construction, which necessarily includes the
  blank runs adjacent to the damage.

REUSE UNIT
  The top-level entry (block or blank run). Prefix entries pass through
  (structural retention -> nodes_reused). Suffix entries are RECONSTRUCTED
  as new Skel values with delta-shifted spans (mizchi shift_block_span;
  parser-work avoided, representation rebuilt -> nodes_rebuilt). Region
  blocks are freshly parsed -> nodes_rebuilt.

POSITION STRATEGY
  Eager delta-shift of retained document-global coordinates (R2-H09
  class M2/M3 "delta-shift"), reconstruction-proportional to the suffix.

SEMANTIC-DEPENDENCY HANDLING
  The mizchi TOTAL fallback, reproduced verbatim as a semantic rule:
  if the retained old definitions array is non-empty OR the region parse
  creates a reference definition, H1 performs a TOTAL full parse and
  records fallback_to_full_count += 1. No dependency index is added (that
  would be a different H1; R2-H01 stays testable).

FALLBACK SEMANTICS (all classes semantic, deterministic, source-derived;
  never CaseId/corpus/label/timing-derived; each recorded via
  fallback_to_full_count)
  F1 definition presence (above; the R2-H01 class).
  F2 left-edge paragraph merge: the entry before the region is a
     Paragraph AND the region's first post-edit line is neither blank nor
     a B1-B6 interrupt (the old paragraph would continue into the region).
  F3 left-edge container continuation: the entry before the region is a
     Quote whose quote would carry the region's first line ('>' prefix),
     or a List that the region's first line would continue (sibling
     marker at the list indent, or indent >= the item content indent).
  F4 right-edge paragraph merge: the region parse ends with an open
     paragraph AND the first surviving suffix entry is a Paragraph
     (R2-H02 class, made sound instead of silently wrong).
  F5 right-edge list/quote merge: the region parse ends with an open
     List/Quote frame AND the first surviving suffix entry is a List /
     Quote respectively.
  F6 forward fence state: the region parse ends inside an open fence
     whose extent cannot be bounded within the region (R2-H03 class,
     made sound instead of silently wrong). (A region extending to EOF
     with an open fence IS the bounded case: unclosed fences run to EOF.)
  A fallback re-parses the complete post source with the ordinary block +
  inline pipeline and rebuilds the tiling. R2-H02/H03's predicted
  WRONG_RESULT outcomes therefore become `correct but fallback-to-full` —
  a valid experimental outcome recorded here in advance.

COUNTER APPLICABILITY
  Per §3. metadata_records_touched for H1 = tiling entries inspected by
  the damage scan + entries whose stored spans were shifted (suffix) +
  entries inserted for the region result. On F1-F6 fallback: the full
  scan + full reconstruction counts are reported (they happened).

EAGER COMPLETION BOUNDARY
  The pending state holds the complete new tiling and definition array;
  complete() seals + checksums only.

QUERY STRATEGY
  Projection to NormalizedDocument + oracle node_path_at (§5).

IDENTITY WITNESS (task contract §9; tests, not performance)
  W1 a safe local edit inside one block of a definition-free document:
     fallback_to_full_count == Known(0), unique_source_bytes_inspected <
     post_source.len(), the affected block reparsed, result == H0.
  W2 a definition-bearing document with an edit anywhere:
     fallback_to_full_count == Known(1), result == H0.
```

## 7. H2 — FRAGMENT_REUSE (`mechanisms/fragment-reuse`)

```text
MECHANISM
  Retained old tree (parent-relative, Arc-shared) + a fragment table; the
  edit is mapped through the fragment table (split/drop/trim/shift, open
  edges); the new document is parsed line-driven, and at every block-start
  line the parser consults the fragment cursor: whole old BLOCK subtrees
  whose start aligns with the live position, whose entry ContextKey
  vouches the live container/fence state, and which fit the fragment's
  safe window are taken whole (structural sharing); everything else is
  parsed normally. Reuse authority comes ONLY from the fragment table.

PRIOR_ART_ANCHOR
  lezer-inspired (@lezer/common TreeFragment.applyChanges lifecycle,
  openStart/openEnd edges, offset deltas, minGap; @lezer/markdown
  FragmentCursor moveTo/matches(contextHash)/takeNodes, block-boundary run
  termination, fenced-code reused whole — R2 MECHANISM-SOURCE-MAP H2).

FIDELITY_BOUNDARY
  Not a port: no LR tables (the grammar is line-driven), no JS object
  model, no viewport scheduling, no exact rolling hash — vouching is an
  EXPLICIT structural ContextKey (§2), which is strictly stronger than a
  32-bit hash (no collisions). Not a strict subset either: taking runs of
  sibling blocks at nested levels reproduces Lezer markdown's block-run
  reuse (the HORSE_BOUNDARY_AMBIGUITIES item 1 shape), not merely
  fragment-interior lookup.

NON_GOALS
  No arbitrary old-tree structural search independent of the fragment
  windows; no checkpoint/restart machinery; no content-hash verification.

MECHANISM_INTRINSIC_STATE (native retained state)
  The old tree: Arc<FNode> block nodes — kind data, byte SIZE, children
  as (rel_start, Arc<FNode>), materialized inline content (relative
  spans), entry ContextKey, has_ref/has_def subtree facts, recorded
  definition facts — under a Document wrapper (the root is never reused).
  The fragment table: fragment ranges in UPDATED-document coordinates,
  offset deltas (document -> tree), openStart/openEnd flags. The retained
  definition facts (for rebuilding the document table from surviving
  fragments). FNode spans are PARENT-RELATIVE; absolute coordinates are
  fragment-offset + accumulation at read time.

COMMON_INPUT / COMMON_INSTRUMENTATION
  As §1/§3.

MODEL_DEFINED_STATE
  None. No query index, no per-block hash. The whole-document fragment
  produced after each complete parse is the Lezer addTree lifecycle, not
  an added structure.

DAMAGE / INVALIDATION RULE (applyChanges analogue; deterministic)
  prepare_update maps the edit through the fragment table: the fragment
  covering the edit is split; the edited span is dropped; later
  coordinates shift by delta; edges adjacent to the change are marked
  open. minGap = 128 bytes: a surviving piece shorter than 128 bytes
  adjacent to a change is dropped entirely.
  PRIOR_ART_ANCHORED_PRE_MEASUREMENT_CONSTANT: minGap = 128 (the frozen
  @lezer/common default). Adopted BEFORE any measurement; never tuned
  from R5 runtime.
  Open-edge safe windows (benchmark analogue of cutAt/Lookahead.Margin):
  a candidate must lie fully inside its fragment's old-coordinate window;
  at an OPEN edge the one adjacent whole block is excluded from reuse
  (its termination/entry decision consumed bytes the edit changed; the
  stronger ContextKey check guards the other edge). Trailing-partial-line
  and block-boundary rules are subsumed: candidates are whole blocks in
  unchanged byte regions.

REUSE RULE (all clauses required; no arbitrary search, no content hashes)
  live parse position == candidate old-tree start (mapped by the
  fragment offset); candidate lies inside the fragment safe window and
  does not touch the edited span; no fence is open in the live parser;
  candidate entry ContextKey == live ContextKey; reference-environment
  clause (below); taken runs end at block boundaries. Reuse failure
  reparses the line normally (natural degradation, not a fallback event).

SEMANTIC-DEPENDENCY HANDLING
  Broad conservative invalidation: if the damaged old blocks contain a
  ReferenceDefinition, or the reparsed region creates one, the update is
  flagged definition-changing; while flagged, any candidate whose subtree
  has_ref is refused. Reparsed inline content is resolved against the
  rebuilt document-global first-wins table (surviving fragments'
  recorded facts + new facts). When no definition changed, reference-
  bearing subtrees may be reused: their resolved destinations are still
  valid because every definition in the document lives in unchanged
  bytes. No horse-only dependency index.

COUNTER APPLICABILITY
  Per §3 (fallback_to_full_count NotApplicable — H2 has no fallback
  concept). metadata_records_touched = fragment records inspected/mapped
  during applyChanges + cursor alignment consultations (fragment records
  touched per take attempt).

EAGER COMPLETION BOUNDARY
  update() completes the block pass, rebuilds the definition table,
  materializes fresh regions, assembles the new tree, and re-registers
  the new whole-document fragment — all before complete(); complete()
  seals + checksums only.

QUERY STRATEGY
  §5.

IDENTITY WITNESS (task contract §10)
  W1 a safe local edit: >= 1 mapped fragment survives, >= 1 old block
     subtree is taken (nodes_reused > 0), source inspection < full
     source, result == H0.
  W2 a context-changing edit (container/fence change before unchanged
     blocks): reuse is REFUSED where the vouching state differs
     (nodes_reused for the affected region == 0), result == H0.
```

## 8. H3 — OLD_TREE_SUBTREE_REUSE (`mechanisms/old-tree-subtree-reuse`)

```text
MECHANISM
  Retained OLD TREE with edit/change flags; the edit is mapped onto the
  old tree by patching ONLY the affected ancestry; the post-edit source
  is parsed in one forward pass that consults the old tree through a
  forward-only cursor: unmarked subtrees whose start aligns with the
  parse position and whose entry ContextKey equals the live parser state
  are spliced whole (shared by Arc); rejected candidates descend
  (children) or are advanced past and reparsed. No fragment table.

PRIOR_ART_ANCHOR
  tree-sitter-inspired (ts_tree_edit change flags on the edited path;
  ReusableNode forward cursor; position-alignment + state-agreement
  splice; suffix reuse gated on context agreement — R2
  MECHANISM-SOURCE-MAP H3 with its documented hybrid caveat).

FIDELITY_BOUNDARY
  Mechanism model, not a port: no LR/GLR tables, no lex-mode/action
  compatibility (BENCH-GRAMMAR-v1 is line-driven; the ContextKey IS the
  entry-state analogue), no GLR version-count suppression (the grammar is
  total and deterministic), no C runtime/pools. Unlike upstream, reuse is
  NOT content-blind trust: the #22 oracle judges every result. The
  tree-sitter position strategy is reproduced honestly: patch the edited
  path only, derive absolute coordinates at read time — NOT an
  artificial O(N) shift of every node (task contract §11).

NON_GOALS
  No fragment table; no checkpoint selection/convergence as primary
  mechanism; no content hashes; no parse-new-tree-then-compare.

MECHANISM_INTRINSIC_STATE (native retained state)
  Top-level entry list: { gap_before, size, Arc<TNode> } with NO stored
  absolute offsets at top level (absolute = prefix sum at read; an edit
  never shifts sibling entries). TNode: kind data, byte SIZE (patched
  only along the edited ancestry), children (rel_start, Arc<TNode>),
  changed flag, entry ContextKey, has_ref/has_def facts, recorded
  definition facts. Edit flags ARE the damage map.

COMMON_INPUT / COMMON_INSTRUMENTATION
  As §1/§3.

MODEL_DEFINED_STATE
  None (no parent map, no node-id lookup, no span index; containment is
  walked, alignment is the cursor).

DAMAGE / INVALIDATION RULE
  prepare_update locates the edited top-level entry by a linear scan
  (reads only) and patches the ancestry chain whose extent the edit
  touches: sizes along the path absorb delta, relative offsets of
  children after the edit inside patched nodes shift, nodes overlapping
  the edit are marked changed. Everything else is untouched — the change
  flags are the damage map (tree-sitter shape). metadata_records_touched
  = scanned top entries + patched nodes + cursor consultation records.

REUSE RULE (per splice)
  parse position == candidate start (new coordinates; the patched
  ancestry makes cursor accumulation correct past the edit); candidate
  !changed and disjoint from the edited span; no fence open; candidate
  entry ContextKey == live ContextKey; reference clause as H2 (broad
  invalidation while definition-changing); root never reused. Rejected
  composite candidates descend; rejected leaves advance the cursor;
  affected regions reparse.

SEMANTIC-DEPENDENCY HANDLING
  Same conservative rule as H2 (definition change -> refuse has_ref
  candidates; reparsed inline resolved against the rebuilt table). No
  hidden global dependency index.

COUNTER APPLICABILITY
  Per §3 (restart/convergence NotApplicable — the forward cursor is not
  renamed into convergence; fallback NotApplicable — degradation is
  natural).

EAGER COMPLETION BOUNDARY
  update() finishes the forward pass, rebuilds the table, materializes
  fresh regions into TNodes, and assembles the new entry list before
  complete(); complete() seals + checksums only.

QUERY STRATEGY
  §5.

IDENTITY WITNESS (task contract §11)
  W1 a local same-context edit: old-tree edit metadata patched (edited
     path), changed path marked, nodes_reused > 0 AND nodes_rebuilt > 0,
     full source not reparsed, result == H0.
  W2 a context-mismatched or changed candidate is refused (descend/
     reparse), result == H0.
```

## 9. H4 — RESTART_CONVERGENCE (`mechanisms/restart-convergence`)

```text
MECHANISM
  Retained checkpoint records + retained old top-level blocks; select a
  restart checkpoint at/before the damage; parse the post source forward
  from the restart; at each live block start beyond the damage compare
  the live parser state against the mapped old checkpoint; when the
  frozen convergence predicate holds, reuse the old stable suffix
  (structural sharing via per-block base offsets). No fragment table, no
  arbitrary old-tree search.

PRIOR_ART_ANCHOR
  wagner-graham / swift-inspired benchmark model (restart-before-damage,
  forward validation, stable-suffix reuse; R2 MECHANISM-SOURCE-MAP H4).
  FIDELITY NOTE (required by the R2 record): the convergence authority
  here is parser-STATE agreement at block boundaries — W&G's convergence
  is batch-parser equivalence at theorem matchpoints and Swift's is a
  byte-local checkpoint predicate; neither is literally reproduced. This
  is a benchmark-model-defined convergence in the family abstraction R0
  §3 licenses, documented here rather than attributed to either anchor.

NON_GOALS
  No fragment table; no structural old-tree search; no theorem-level
  safety claim; no claim that the predicate is byte-local (it compares
  full parser state, precisely because Markdown block context is not
  byte-local — R2-H10).

MECHANISM_INTRINSIC_STATE (native retained state)
  Top-level block list: { base_shift: isize, Arc<Skel> } — Skel spans are
  absolute-as-parsed; the CURRENT absolute span = stored + base_shift
  (per-block base offsets; derive-at-read; a reused suffix block is the
  SAME Arc with a shifted base — no reconstruction). Checkpoint records
  at every top-level block start: { position, parser state (ContextKey
  §2), reference-environment generation }. Retained definition facts
  (prefix + suffix) for table rebuild. No benchmark counters are stored
  as mechanism state.

COMMON_INPUT / COMMON_INSTRUMENTATION
  As §1/§3. restart_distance / convergence_distance are COMMON
  INSTRUMENTATION (R2 classification), reported from frozen definitions
  below.

MODEL_DEFINED_STATE
  The checkpoint DENSITY (every top-level block start) is a frozen
  pre-measurement policy choice justified by BENCH-GRAMMAR-v1 (state is
  well-defined exactly at block starts; §12 of the task contract names
  block/line boundaries as the preferred semantic boundaries). It is
  declared here as model-defined-and-frozen, never runtime-tuned.

CHECKPOINT PAYLOAD
  { position, container stack + fence state (ContextKey), reference-
  environment generation }. Explicit, deterministic, correct for
  BENCH-GRAMMAR-v1; minimality not claimed. The payload prevents false
  convergence under container-depth change, fence-state change, block-
  continuation change, and semantic reference change.

DAMAGE / INVALIDATION RULE
  The damage position is the canonical edit range. Restart = the LATEST
  checkpoint at or before the damage start whose state can seed a parse
  (every top-level block start qualifies structurally; when the update
  is definition-changing the reference clause forces restart at the
  document start instead — see below). Blocks before the restart are
  unaffected by construction and are retained (structural sharing).

RESTART RULE (frozen definition)
  restart_distance = edit_start_new - restart_position (post-edit bytes
  reparsed before the damage is reached). Restart-at-zero reports
  Known(0) — a measured zero, never NotApplicable.

CONVERGENCE RULE (frozen definition)
  Convergence at live block-start position p (p > damage end, mapped old
  position q = p - delta) requires ALL of:
    (a) q is exactly an old checkpoint position;
    (b) live ContextKey == checkpoint ContextKey (container + fence
        state — full state equality, never byte equality; R2-H10's false
        convergence is excluded by construction);
    (c) the reference-environment generation matches (no definition was
        changed/created/deleted by this update);
    (d) q lies beyond every damaged old entry (the suffix is untouched
        by the edit by construction of (a)+(c)+the forward parse).
  On convergence the old suffix from q is reused with base_shift += delta.
  convergence_distance = convergence_position - restart_position (post
  bytes). If no checkpoint satisfies the predicate before EOF, the parse
  runs to EOF and convergence is recorded AT EOF with an empty suffix.

SEMANTIC-DEPENDENCY HANDLING
  If the damaged old entries contain a ReferenceDefinition, or the fresh
  parse creates one, the reference-environment generation changes: the
  convergence clause (c) then fails everywhere, so H4 restarts at the
  document start and parses forward to EOF (the degraded restart-at-zero
  path; explicitly NOT fallback_to_full — no separate full-rebuild
  mechanism exists in H4). Results remain correct; the counters report
  the degradation honestly.

COUNTER APPLICABILITY
  Per §3: restart_distance and convergence_distance Known on every
  update (Known(0) is a measured zero); fallback NotApplicable;
  metadata_records_touched = checkpoint records consulted during restart
  selection + checkpoint records re-registered + suffix base-shift
  records.

EAGER COMPLETION BOUNDARY
  update() completes the forward parse, convergence, suffix splice,
  table rebuild, and checkpoint re-registration before complete();
  complete() seals + checksums only.

QUERY STRATEGY
  §5.

IDENTITY WITNESS (task contract §12)
  W1 a safe local edit deep in a document: restart_position >
     document start (restart_distance Known > 0), convergence BEFORE EOF
     (convergence_distance Known), stable suffix reused
     (nodes_reused > 0), result == H0.
  W2 a forward/container-state case (e.g. inserting an unclosed fence
     opener or changing container state): convergence is DELAYED (no
     checkpoint satisfies the predicate at the adjacent boundary); result
     == H0.
  W3 a definition-changing edit: restart-at-zero path, parse to EOF,
     result == H0.
```

## 10. Implementation parity (task contract §25 preview)

Common across H0-H4 (must match): BENCH-GRAMMAR-v1 semantics (one shared
crate), source representation (`Source`), canonical edit, normalized result
(vocabulary + validation + checksum), allocator (workspace default),
compiler/profile (workspace frozen), inspection-event discipline, oracle
usage (test-side only). Horse-specific (must map to mechanism identity):
native retained state, damage/invalidation, reuse authority, position
strategy (H1 delta-shift rebuild / H2 fragment offsets + parent-relative /
H3 patch-path + derive-at-read / H4 per-block base offsets — exactly the
four R2-H09 position classes), fallback, counters. No unexplained
one-horse privilege; no undeclared optimization; no timing anywhere.
