# R2 MECHANISM-SOURCE-MAP — prior-art anchors for H0–H4

Status: **READY_FOR_ADVERSARIAL_R2_REVIEW** (post corrective pass
MARKIT-R2-PRIOR-ART-CORRECTIVE-1, 2026-09-17)
Authority: GitHub Issue #22 + `protocol/R0-METHODOLOGY.md` §2–§3.

This file is written **after** the individual extraction records, not
before. It maps extracted prior-art mechanisms onto the frozen horses. Per
the R0 fidelity naming rule, every horse is a **mechanism model** documented
as `<project>-inspired`; no horse is a port or reproduction of any upstream
project.

The mapping below is classification work (hypothesis-level), not new
extraction: all OBSERVED evidence lives in the per-record files.

---

## H0 — FULL_REBUILD

```text
MECHANISM_SOURCE_MAP:
  md4c (release-0.5.3)          — clean whole-buffer push-model parse; zero
                                  retained state (grep-verified negative
                                  evidence: no incremental API)
  pulldown-cmark (v0.13.4)      — clean two-pass rebuild (eager block pass in
                                  constructor, lazy inline/event stage); zero
                                  state across parses
  comrak (v0.55.0)              — clean two-phase parse (blocks, then inlines)
                                  into caller-owned arena; zero state across
                                  parses
```

PRIOR_ART_ANCHOR:
  The clean full-parse routes of MD4C / pulldown-cmark / Comrak establish
  that zero-reuse whole-document rebuild is the mainstream production design
  for Markdown parsing (the H0 cost floor is not a straw man). All three
  independently fix the same ordering: block/line analysis completes before
  inline resolution, with the document-global reference-definition table
  built in the block phase and consumed by the inline phase.

FIDELITY_BOUNDARY:
  An H0 model is `md4c`/`pulldown-cmark`/`comrak`-inspired, not a port.
  NOT reproduced: arena/CowStr/InlineStr allocation strategies, SIMD or
  jetscii scanning, callback ABI shapes (pointer-into-input text events,
  missing block ranges), 32-bit offsets, ref-def output budgets, upstream
  event ordering, spec-suite test corpora.

NON_GOALS:
  H0 implements no damage detection, no reuse, no retained state across
  calls, no fallback. It exists as the correctness authority and the
  zero-reuse cost floor.

MECHANISM_INTRINSIC_STATE:
  None across edits. Zero retained state IS the mechanism. (Transient
  per-parse state — block tables, delimiter stacks, ref-def maps — is
  rebuilt every call and is not mechanism-intrinsic state in the parity
  sense.)

---

## H1 — BLOCK_LOCAL_REPARSE

```text
MECHANISM_SOURCE_MAP:
  mizchi/markdown.mbt (master ffe7dc00) — the only extracted project with a
      shipped block-local incremental Markdown mechanism: top-level block
      span map → overlap damage mapping → clean reparse of region
      [prev_block_end, next_block_start+delta) → splice (prefix blocks pass
      through; suffix values reconstructed with shifted spans) → TOTAL
      full-parse fallback whenever link reference definitions exist in old
      or new document. Its `reused_*` counters mean "not reparsed"
      (parser-work reuse), NOT object/representation reuse.
  tree-sitter-markdown (v0.5.3) — maintainer-authored enumeration (the
      serialized external-scanner state) of KNOWN-SUFFICIENT entry-context
      dimensions for one implementation's reuse gate: container stack with
      per-item content indent, phase flags, partial-line indentation,
      tab-stop column, open-fence length. Sufficiency is shown for that
      grammar+engine only; MINIMALITY UNKNOWN and necessity of this exact
      representation NOT ESTABLISHED — other mechanisms may recompute
      context from source, restart farther backward, encode equivalent
      context in parser states, use hashes/fingerprints, retain a
      different representation, invalidate conservatively, or avoid
      retaining cross-edit scanner state altogether.
  md4c / pulldown-cmark — Q9 catalogs of cross-line and retroactively-bound
      state (list loosening, fence/HTML continuation, Setext/table
      rewrites, ref-def consumption) that bound what "block-local" can mean.
```

PRIOR_ART_ANCHOR:
  Block-oriented incremental Markdown designs such as mizchi/markdown
  (R0 §3 wording) — substantively fair per the extraction: the reuse unit
  is exactly the top-level block (blank-line runs included as blocks), and
  B (block count) / L (affected block length) are the dominant variables.

FIDELITY_BOUNDARY:
  An H1 model is `mizchi-markdown-inspired`, not a port. NOT reproduced:
  UTF-16 code-unit coordinates (R0 §6 mandates UTF-8 bytes); treating
  upstream `reused_*` counters as representation reuse — they count
  parser-work reuse ("not reparsed") only, and a faithful H1 model must
  keep nodes_reused / nodes_rebuilt / metadata-touched /
  parser-bytes-inspected as distinct facts (R0 §5 forbids identity in
  correctness); the JS handle / source-retention layer; vendor SIMD tuning
  of the full parser.
  Modeling decisions R3+ must make explicit (recorded, not resolved here):
  whether to reproduce the definition-presence total fallback as part of a
  faithful H1 (predicting collapse-to-H0 on REFERENCE_FANOUT), or to model
  a reference-resolution pass instead — which would be a DIFFERENT
  mechanism and must not silently wear the H1 name.

NON_GOALS:
  No fragment tables, no old-tree consultation, no restart checkpoints, no
  convergence checks beyond what block-boundary context reconstruction
  requires, no hashes/validation machinery beyond the mechanism's own
  damage mapping (mizchi's shipped design validates nothing — see
  R2-HYPOTHESES R2-H02/R2-H03 for the predicted consequences).

MECHANISM_INTRINSIC_STATE:
  OBSERVED PRIOR-ART STATE: the old Document itself — the top-level block
  sequence with document-global spans (BlankLines included), nested
  structure inside block values — plus the API inputs (EditInfo, old/new
  source); the definitions array doubles as the fallback trigger (mizchi
  inspects the array, it does not keep a flag).
  MODEL-DEFINED CANDIDATE STATE (not observed upstream; whether the Rust
  benchmark horse carries any of these is an R3+ implementation-parity
  decision, and none may gain MECHANISM_INTRINSIC status merely because
  it would make H1 faster): a dedicated block index/table (spans/kinds/
  boundaries); a definition-presence flag instead of array inspection; an
  entry-context cache. Candidate H1 entry-context dimensions are the
  tree-sitter-markdown enumeration — KNOWN-SUFFICIENT for that
  implementation, MINIMALITY UNKNOWN; R2 does not define the eventual H1
  context/checkpoint representation.

---

## H2 — FRAGMENT_REUSE

```text
MECHANISM_SOURCE_MAP:
  @lezer/common 1.5.2 — TreeFragment machinery: applyChanges splits/drops/
      trims fragments (minGap=128), applies offset deltas, marks cut edges
      open; per-node contextHash / lookAhead props.
  @lezer/lr 1.4.8 — fragment-gated parse: reuse only inside safe windows;
      state-anchored node absorption (goto must accept the node type at the
      live stack state); strict contextHash equality when context tracking
      is on; no stored parse-state stacks — context rebuilt by parsing.
  @lezer/markdown 1.6.3 — markdown consumer: per-line fragment cursor
      (moveTo) + composite-block rolling-hash vouching (matches()) +
      takeNodes of whole block subtrees; NotLast/block-boundary/
      trailing-partial-line convergence guards; fenced code reused whole.
```

PRIOR_ART_ANCHOR:
  Lezer-style reusable fragments/tree fragments (R0 §3 wording) — supported:
  all reuse is gated through the fragment table. BUT the extraction shows
  Lezer is not "fragment lookup only, gap interiors fully reparsed": both
  consumers absorb old-tree nodes/blocks inside gaps when the live parse
  state reaches them. See HORSE_BOUNDARY_AMBIGUITIES below.

FIDELITY_BOUNDARY:
  An H2 model is `lezer-inspired`, not a port. NOT reproduced: generated LR
  tables / lezer-generator, JS runtime object model, parseMixed nested-
  language plumbing, CodeMirror viewport scheduling (stopAt/parsedPos),
  the exact rolling-hash function. Transferable: fragment table lifecycle,
  offset-delta bookkeeping, open-edge flags, boundary-context vouching,
  state-anchored absorption at fragment boundaries.

NON_GOALS:
  No structural search over an old tree independent of parse position
  (that is the H3 differentiator); no checkpoint-selection restart
  mechanism; no content-hash verification of the underlying text (Lezer
  vouches on parse-state/context hashes, not content hashes).

MECHANISM_INTRINSIC_STATE:
  Fragment table (ranges in updated-document coordinates, offset deltas,
  openStart/openEnd edge flags) + edit→fragment mapping + boundary
  vouching state (context-hash analogue) for nodes/blocks absorbed at
  fragment boundaries.

---

## H3 — OLD_TREE_SUBTREE_REUSE

```text
MECHANISM_SOURCE_MAP:
  tree-sitter v0.27.0 — the core anchor: retained old tree edited in place
      (ts_tree_edit patches offsets/points, sets has_changes flags), a
      forward-only old-tree cursor offering subtrees, and the exact reuse
      predicate: position alignment + external-scanner entry-state equality
      + rejection of changed/error/missing/fragile subtrees + first-leaf
      lex-mode/action-table compatibility; splice state = current grammar
      transition (never copied from old tree); backdown on later
      invalid lookahead; reuse suppressed while GLR version_count > 1
      (allow_node_reuse recomputed per outer parse-loop iteration after
      condense, so suppression is interval-shaped, not sticky);
      ts_tree_get_changed_ranges diff for consumers.
  wagner-graham (dissertation Ch. 6) — concept ancestor: unchanged-subtree
      reuse via the exact nonterminal shift test; bottom-up reuse at
      reduction; top-down isomorphic-replacement pass over modified regions.
  swift-syntax 604.0.0 — old-tree-as-lookup variant: reuse decided at fixed
      checkpoints against the old tree with byte-local compatibility
      (position + kind + untouched lookahead range).
```

PRIOR_ART_ANCHOR:
  Tree-sitter-style old-tree reuse concepts (R0 §3 wording). The extraction
  supports the anchor **within a documented hybrid caveat**: tree-sitter
  couples old-tree reuse with convergence gating (state agreement) and a
  degradation ladder — it is not a pure "consult old tree, splice, done"
  design (see HORSE_BOUNDARY_AMBIGUITIES).

FIDELITY_BOUNDARY:
  An H3 model is `tree-sitter-inspired`, not a port. NOT reproduced: LR
  parse tables, GLR error-recovery policy, C runtime / inline-subtree
  encoding / subtree pools / allocators, query engine, included-range
  injection, repeat-chain balancing, progress/cancellation machinery.
  Also NOT reproduced blindly: tree-sitter's reuse is content-blind —
  correctness rests entirely on trusting the caller's ts_tree_edit mapping;
  the #22 oracle replaces that trust with
  normalize(update) == normalize(H0 clean parse).

NON_GOALS:
  No fragment table; no checkpoint-selection restart mechanism as the
  primary path; no structural pattern-matching between independently parsed
  trees (tree-sitter never compares source content for reuse — reuse is
  position-anchored consultation of the edited old tree).

MECHANISM_INTRINSIC_STATE:
  The retained old tree (memory ∝ document structure), edit-flag
  propagation state on the old tree, the old-tree navigation cursor, and —
  if the mechanism adopts tree-sitter's gating — the state-equality keys
  (parse-state analogue + external-scanner-state analogue) compared at
  splice points. Whether a Markdown H3 needs a scanner-state analogue is
  an R3+ design question; the tree-sitter-markdown record enumerates what
  that state would have to contain.

---

## H4 — RESTART_CONVERGENCE

```text
MECHANISM_SOURCE_MAP:
  wagner-graham (TOPLAS 1998 via dissertation CSD-97-946 Ch. 6) — the
      classical shape: resume forward from before the damage, validate
      speculatively retained structure by bounded forward progress (shift a
      non-ε symbol within k terminals, else undo via delayed
      right_breakdown), splice the stable suffix as parser INPUT (not as a
      reparse-and-compare target). Convergence = equivalence with a batch
      parser on the same tables at theorem matchpoints (Thm 6.4.1.1).
  swift-syntax 604.0.0 — modern editor-shaped variant: forward-only parse
      from file start; reuse decisions at EVERY fixed checkpoint
      (CodeBlockItem / MemberBlockItem starts) via byte-local predicate
      (pre-edit position + expected kind + untouched lookahead range);
      reuse/damage interleaving tolerated; no one-time converge-and-splice.
  tree-sitter v0.27.0 — H4-flavored convergence gating inside an H3
      mechanism: suffix reuse requires state agreement (entry-state
      equality; composite candidates need parse_state == current state).
  @lezer/markdown 1.6.3 — Markdown-specific convergence conditions:
      composite-block context hash equality; taken run must end at a block
      boundary; never end inside a NotLast block (CodeBlock, list items —
      continuable across blank lines); trailing partial line excluded.
```

PRIOR_ART_ANCHOR:
  Classical incremental parsing (Wagner & Graham), Swift incremental syntax
  concepts, and Markdown-specific restart/convergence observations (R0 §3
  wording) — supported, with a sharp fidelity note: the three anchor
  families use DIFFERENT convergence authorities (W&G: batch-parser
  equivalence at matchpoints; Swift: byte-local checkpoint predicate,
  interleaved; tree-sitter: state-agreement gating of skip-based reuse).
  R0's H4 wording ("detect semantic/parser-state convergence", "reuse
  stable suffix after convergence") is consistent with this family
  abstraction; no anchor literally implements "reparse then compare new
  tree vs old tree at a boundary", so an H4 model doing that must be
  documented as `benchmark-model-defined`, not W&G- or Swift-faithful.

FIDELITY_BOUNDARY:
  An H4 model is `wagner-graham`/`swift`-inspired, not a reproduction.
  NOT reproduced: LALR/LR table generation, GLR/parse-dag machinery
  (W&G Ch. 7), the self-versioning storage subsystem, Swift's grammar /
  generated parser / C++ interop / arena machinery, the exact
  nonterminal-shift test (no LR states exist for Markdown — its safety
  theorem does not transfer). What transfers: restart-before-damage shape,
  bounded speculative validation with defined rollback, checkpoint
  payloads, suffix-as-input vs suffix-as-reparse as an explicit modeling
  choice.

NON_GOALS:
  No fragment table; no arbitrary-position old-tree structural matching;
  no theorem-level safety claim for the Markdown model; no claim that
  convergence is byte-local unless the model explicitly chooses and
  defends that (Swift's byte-local predicate is hypothesis-level for
  Markdown, whose block-boundary state — container depth, fence state —
  is NOT byte-local).

MECHANISM_INTRINSIC_STATE:
  Restart/checkpoint state, convergence-validation state, and
  restart/convergence distance bookkeeping (R0 §10 counters).
  CANDIDATE H4 CONTEXT DIMENSIONS (not an R2 representation decision):
  the tree-sitter-markdown serialized-state enumeration (container stack,
  fence state, phase flags, partial-line indentation, tab column) is
  KNOWN-SUFFICIENT for that implementation's reuse gate — evidence that
  these dimensions matter for one design, NOT that they are the minimal
  or necessary payload for every mechanism (minimality unknown; retention
  vs recomputation mechanism-dependent; R2 does not define the eventual
  H4 checkpoint representation). Lookahead-range records (Swift) are
  mechanism-intrinsic if the model adopts checkpoint lookahead.

---

## HORSE_BOUNDARY_AMBIGUITIES (reported, not resolved)

Per the R2 rules, ambiguities are recorded for review; R0 is not modified
during R2.

1. **H2 vs H3 — lookup strategy, not granularity.** Lezer (the canonical
   H2 anchor) absorbs individual old-tree NODES and whole BLOCK SUBTREES
   inside fragment gaps, state-anchored (goto-accepting live state /
   context-hash vouching). The extracted differentiator between H2 and H3
   is therefore the lookup/vouching strategy — fragment-window +
   live-state anchoring (H2) vs free navigation/consultation of an edited
   old tree by position (H3) — NOT "region vs node" granularity. If H2 is
   implemented as "fragments only, gap interiors fully reparsed", it is a
   strict subset of what Lezer does and will under-measure the anchor.
2. **H3 vs H4 — tree-sitter is a hybrid.** Old-tree reuse in tree-sitter
   is gated by convergence-style state agreement, has a degradation ladder
   (descend → lex region → GLR recovery → reuse suppression), and its
   "restart" is always position 0 with an opportunistic forward cursor —
   skip-based, not checkpoint-based. Citing tree-sitter as a pure H3
   anchor silently imports convergence gating; citing it for H4 would
   misrepresent its checkpoint shape (it has none).
3. **W&G supports H3 and H4 simultaneously.** One mechanism contains both
   unchanged-subtree reuse (shift test, bottom-up + top-down reuse passes)
   and restart/convergence (speculative validation, suffix-as-input).
   Using W&G as an anchor for one horse imports concepts of the other.
4. **Swift straddles H3/H4.** Old-tree reuse (H3-like) at fixed checkpoints
   during a forward parse with interleaved reuse (H4-like); its
   compatibility predicate is byte-local, not parser-state equality.
5. **H1's boundary is bounded by other horses' machinery.** The
   tree-sitter-markdown scanner-state enumeration and the md4c/pulldown
   cross-line-state catalogs show that "reparse the affected block" is not
   self-sufficient: some entry context must be re-established (by
   retention, recomputation, or a farther-back restart — mechanism-
   dependent), and where a faithful H1 ends and a converging H4 begins
   (unclosed fence, setext underline, container nesting) is exactly the
   boundary the structural edit campaign (R8) must probe. mizchi's shipped
   design resolves this by NOT checking (silent divergence classes) and by
   falling back to H0 for definitions only.
6. **H1 partially collapses to H0 by design** (mizchi's
   definition-presence total fallback). A faithful H1 model must either
   reproduce this (and be predicted to degenerate on REFERENCE_FANOUT) or
   explicitly model a different mechanism.

Items 1–4 are recorded in the per-horse FIDELITY_BOUNDARY sections above;
they do not contradict R0's horse definitions as written (R0 §3 freezes
horses as mechanism models with prior-art anchors, not literal
reproductions), so R2 records the ambiguities and leaves R0 unchanged —
but they constrain how R5 horses may be described and how R8 results may
be attributed.
