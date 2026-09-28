# Gate-A R3 — DONOR-SIDE MODEL (frozen before horse inspection)

All citations: tree-sitter @ 6070dbfefd326bd735e5683eb128cc1b57dad0c0 (v0.27.0),
tree-sitter-markdown @ f969cd3ae3f9fbd4e43205431d0ae286014c05b5 (v0.5.3),
swift-syntax @ 050f1a346fbbac0ca2cfb15a95274f7bd1cf0ccf (604.0.0).
Evidence class: DONOR_PRIMARY_SOURCE (read directly at pinned SHAs this session).

D1 retained representation: whole old CST, client-owned, re-rooted by engine for
  the parse (parser.c:2150-2152) and released at reset (parser.c:2100-2103).
  Per-subtree metadata in-tree: padding/size/lookahead_bytes (bytes + row/col),
  symbol, parse_state (LR state at subtree start), first_leaf{symbol,parse_state}
  cache (subtree.h:115-152; populated in subtree.c:467-470), flags has_changes /
  fragile_left / fragile_right / is_missing / extra / visible / named /
  depends_on_column / has_external_scanner_state_change, error_cost, repeat_depth
  (subtree.h:53-71 inline, :115-152 heap). ExternalScannerState = opaque byte
  buffer attached to external-token subtrees; equality = length + memcmp
  (subtree.c:24-64). Refcounted; inline ≤255 bytes (subtree.c:22).

D2 edit/damage propagation: caller MUST call ts_tree_edit with a TSInputEdit that
  exactly matches the source change (api.h:285-293, :455-464). ts_tree_edit
  (tree.c:97-105) patches included ranges and calls ts_subtree_edit
  (subtree.c:645-798): iterative walk descending ONLY into edit-overlapping
  children (:739-793); three resize/shift cases in bytes AND points (:676-698);
  marks has_changes (:735 via :637-643); inserted text attributed to first
  touching child (:772-781); column-dependent children invalidated along the
  edited row (:666-667, :750-761). Flags ARE the damage map — no range list.

D3 candidate discovery: single forward-only pre-order ReusableNode cursor
  (reusable_node.h:3-12): stack of (subtree, child_index, byte_offset) +
  last_external_token; only advance (:39-60) / descend (:62-74) /
  advance_past_leaf (:76-79); reset pushes root then descends once — root never
  reused (:81-95 with comment :89-94). Consulted from parse position 0 forward
  at each parser advance (parser.c:1609-1613). NO index, NO hash, NO
  arbitrary-position lookup; a candidate can be skipped permanently if the parse
  position jumps past it (parser.c:775-781).

D4 eligibility / parser-state compatibility (ts_parser__reuse_node,
  parser.c:753-836), in order:
  1 position gate: byte_offset > position → break (wait); byte_offset < position
    → descend if candidate spans position else advance (:770-781); EOF candidate
    end treated as UINT32_MAX (:766-768);
  2 external-scanner entry-state equality: cursor.last_external_token's
    serialized state must equal the live parse stack's last external token state
    (:783-787);
  3 rejection flags: has_changes / is_error / is_missing / is_fragile
    (subtree.h:372-374; set for error children subtree.c:446-449,497-498,516-517
    and multi-action reductions parser.c:1023-1025) / included-range difference
    (:798-806);
  4 first-leaf lex compatibility: LR action-table entry for (current state,
    leaf symbol) exists and ts_parser__can_reuse_first_leaf (:818-828; function
    :470-503): leaf lex mode memcmp-equals current state's lex mode, action
    count > 0, keyword case needs leaf parse_state == state (:492-495), empty
    tokens not reusable across differing lookahead sets (:498), else requires
    no external lex state + table is_reusable (:500-502);
  5 accept: retain + return (:830-832).
  Composite candidates narrowed to a descendant whose stored parse_state equals
  the current stack state (ts_parser__breakdown_lookahead, parser.c:224-244,
  condition :232, invoked at shift :1673-1676).

D5 reuse granularity: any non-root old-tree subtree (single token → arbitrarily
  large); root never reused (reusable_node.h:89-94; fresh root at accept,
  parser.c:1077-1082). Rejected composite descends to children (smaller units,
  parser.c:808-811). Accepted subtree spliced whole (one stack push,
  ts_parser__shift parser.c:914-930; pending flag for non-leaves :927).

D6 candidate rejection behavior: past-position → descend-or-advance (:775-781);
  scanner-state mismatch → advance (:783-787); flag rejection → descend if
  composite, else advance past + breakdown_top_of_stack (:808-815); first-leaf
  incompatible → advance_past_leaf + break (:820-828). Error/missing/fragile
  subtrees are permanently excluded while their flags persist (predicate re-runs
  each parse).

D7 fallback / progress semantics: no restart-point search; single forward GLR
  pass from position 0 (parser.c:2150-2159, outer loop :2172-2226). Reuse
  failure → ordinary lexer for that token/region (parser.c:1615-1643); cursor
  stays and re-synchronizes when offsets realign (parser.c:770-781). Backdown:
  invalid lookahead + reused subtree on stack top → breakdown_top_of_stack pops
  it, pushes children with recomputed LR states, retry (parser.c:1789-1798;
  mechanics :176-222; state recompute via ts_language_next_state :198-205).
  Reuse suppressed while GLR version_count > 1 (parser.c:2179
  allow_node_reuse = version_count == 1, recomputed per outer-loop iteration
  after condense :2207 — interval-shaped, not sticky). NO mid-file full-reparse
  fallback; full parse only when old_tree == NULL (parser.c:2165-2168).

D8 coordinate/edit mapping: caller-supplied TSInputEdit (bytes + points);
  ts_subtree_edit converts the edit into each visited child's coordinate space
  (:765-773); zero text rescan; new-tree offsets produced fresh by the parse;
  reused subtrees already carry post-edit offsets because the old tree was
  patched first. Client TSNode handles updated via ts_node_edit (api.h:732-740).

D9 semantic/global dependency handling: hidden cross-line state = external
  scanner state, byte-compared at the splice gate (parser.c:783-787; eq
  subtree.c:60-64). tree-sitter-markdown entry-state dimensions: state flags
  STATE_MATCHING/WAS_SOFT_LINE_BREAK/CLOSE_BLOCK, matched, indentation,
  column (mod-4 tab stop), fenced_code_block_delimiter_length, open_blocks
  stack (18 Block kinds; LIST_ITEM+n encodes content indent 2..15; marker char
  NOT stored) — scanner.c:174-181 (flags), :197-219 (Scanner struct),
  serialize/deserialize :239-289 (5 scalar bytes + memcpy of open_blocks).
  No other global semantic dependency exists in the engine.

D10 retained-state maintenance: per parse: reusable_node_reset (parser.c:2158),
  old_tree refcount retain (:2151) / release at reset (:2100-2103); 1-entry
  TokenCache (parser.c:703-738); external scanner deserialize-before-scan from
  stack's last external token (parser.c:544) and serialize-after-found
  (:550-555). Across parses: has_changes flags + patched coordinates persist in
  the client-retained old tree; the engine keeps nothing between parses.

D11 candidate/search indexing vs enumeration: no index of any kind; one
  monotone pre-order cursor synchronized with the live parse position. Worst
  case cursor skipping is O(old-tree nodes) per parse regardless of edit size
  (parser.c:770-781). Included-range difference index maintained during the
  parse (parser.c:2218-2225).

D12 lifetime/retirement work: reused subtrees retained by refcount (:831);
  new parents built on reduction (:932-1053) and fresh root at accept
  (:1077-1082); copy-on-write via ts_subtree_make_mut when refcount > 1;
  post-parse repeat-chain balancing (parser.c:1912-1960; ts_subtree_compress
  subtree.c:292); subtree pool recycles heap subtrees (subtree.c:23);
  ts_tree_get_changed_ranges for consumers (tree.c:114-137;
  get_changed_ranges.c:413-557; iterator_compare :348-395).

Overlaps: Wagner & Graham (concept ancestor): unchanged-subtree reuse via exact
  nonterminal shift test; bottom-up reuse at reductions; top-down isomorphic
  pass over modified regions (dissertation Ch. 6, §6.3/§6.7). Swift (overlap
  variant): reuse at fixed checkpoints (CodeBlockItem/MemberBlockItem starts)
  with byte-local predicate — pre-edit position equality + expected kind +
  no edit intersectsOrTouches the lookahead affect range
  (IncrementalParseTransition.swift:150-189; translateToPreEditPosition
  :191-206) — NOT parser-state equality.

FROZEN EXPECTATIONS H3-F1..F5 (mechanism outcomes only; no timing):

F1 unchanged eligible subtree + compatible parse state → REUSE. Old-tree
  candidate at aligned byte offset, equal external-scanner entry state, no
  flags, leaf lex-mode/action compatible → accepted whole; parser continues at
  candidate end with grammar-derived next state (never the old tree's stored
  state); consumer changed-ranges cover only the truly damaged region.

F2 unchanged bytes + incompatible parser state → NO REUSE. Byte-identical text
  at the aligned position is still rejected when (a) the external-scanner entry
  state differs (e.g. an inserted "> " line 1 changes open_blocks for every
  later external token), or (b) the first-leaf lex mode / action-table entry is
  incompatible with the current LR state. Rejection cascades forward until the
  entry state converges again. Composite candidates may be decomposed to a
  parse_state-compatible descendant (breakdown_lookahead). State outcome:
  region reparsed by ordinary lexing; no error.

F3 edit overlaps candidate → INVALIDATION. ts_tree_edit sets has_changes on the
  edited nodes + ancestors (D2). Predicate rejects flagged candidates; descent
  into unflagged descendants remains possible; candidates entirely after the
  edit keep patched offsets and NO flag → still reusable when states align.

F4 damaged ancestor containing reusable descendant → DESCEND, REUSE PARTS.
  Donor rejects the flagged ancestor then descends (parser.c:808-811); any
  unflagged descendant at an aligned position with compatible entry state /
  leaf state is reused individually; descent bottoms out at leaves (advance
  past leaf + breakdown of reused stack top when even leaves fail
  :810-813). Damage is a path, not a wall; navigation is inside the damaged
  ancestor's subtree, forward-only.

F5 many top-level candidates → CURSOR SYNCHRONIZATION, NO ENUMERATION. The
  pre-order cursor advances one top-level child at a time in lockstep with the
  parse position; each accepted candidate jumps the parse position to the
  candidate's end and advances the cursor past it (parser.c:1678-1679). Work is
  proportional to candidates actually consulted in source order from position
  0, plus per-candidate predicate evaluation (O(1)-ish per node: flag checks +
  one memcmp + one lex-mode memcmp + one table lookup). The root itself is
  never a candidate; its children are. No whole-tree enumeration, no index
  construction.
