# Prior-art mechanism record — Tree-sitter core incremental parsing engine

R2 stage (prior-art mechanism extraction) of MARKIT-MARKDOWN-BENCHMARK-1.
Authority: `research/benchmarks/markdown-ast-update/protocol/R0-METHODOLOGY.md` §2.

Evidence rules applied: OBSERVED claims cite `file:line` at the pinned commit;
INFERRED claims are labeled; UNKNOWN is stated as unknown. No timing numbers,
no performance claims, no benchmark execution. The mechanism is extracted as
it exists; relevance to H0–H4 is assessed only in §14. This record covers the
**C incremental parsing engine** (`lib/src/`), not any specific grammar.

## 1. Identity

```text
PROJECT:    Tree-sitter (parser generator + incremental parsing engine;
            GLR/LR core + generated per-language tables)
VERSION:    v0.27.0 (latest upstream release tag at retrieval)
COMMIT_SHA: 6070dbfefd326bd735e5683eb128cc1b57dad0c0
            (annotated tag object 3e719425fc48f5b4cdb25c580e44023882f5e2a7
            resolves to this commit; verified with git tag --points-at)
REPO_URL:   https://github.com/tree-sitter/tree-sitter
RETRIEVED:  2026-09-16 (shallow clone into /tmp/markit-r2-prior-art/tree-sitter)
LANGUAGE:   C11 core engine; grammar tables are generated C per language
LICENSE:    MIT (LICENSE; "Copyright (c) 2018 Max Brunsfeld")
```

PRIMARY SOURCES: repository source at the pinned SHA (sole mechanism
authority). SECONDARY SOURCES: Context7 retrieval of official docs
(`/tree-sitter/tree-sitter`, sourced from
`docs/src/using-parsers/3-advanced-parsing.md` — TSInputEdit/ts_tree_edit
usage, edit-then-reparse workflow, ts_node_edit; and `docs/src/cli/fuzz.md`
describing the fuzz oracle) — orientation only, never sole authority for a
mechanism claim.

RELEVANT FILES / FUNCTIONS (all paths relative to repo root, v0.27.0):

- `lib/include/tree_sitter/api.h` — `TSInputEdit` (`:118-131`),
  `ts_parser_parse` contract (`:285-330`), `ts_tree_edit` (`:455-464`),
  `ts_tree_get_changed_ranges` (`:466-492`), `ts_node_edit` (`:732-740`).
- `lib/src/parser.c` — `MAX_VERSION_COUNT=6`/`MAX_COST_DIFFERENCE`
  (`:77-80`), `ts_parser__breakdown_top_of_stack` (`:176-222`),
  `ts_parser__breakdown_lookahead` (`:224-244`),
  `ts_parser__external_scanner_deserialize` (`:414-438`),
  `ts_parser__can_reuse_first_leaf` (`:470-503`),
  `ts_parser__get/set_cached_token` (`:703-738`),
  `ts_parser__has_included_range_difference` (`:740-751`),
  `ts_parser__reuse_node` (`:753-836`), `ts_parser__shift` (`:914-930`),
  `ts_parser__reduce` (`:932-1053`; fragile `:1023-1025`),
  `ts_parser__accept` (`:1054-1105`), `ts_parser__recover` (`:1288+`),
  `ts_parser__handle_error` (`:1477-1572`), `ts_parser__advance`
  (`:1595-1809`), `ts_parser__balance_subtree` (`:1912-1960`),
  `ts_parser_reset` (`:2094-2119`), `ts_parser_parse` (`:2121-2249`;
  reuse enable `:2179`).
- `lib/src/reusable_node.h` — `ReusableNode` pre-order cursor (`:3-12`),
  `advance` (`:39-60`), `descend` (`:62-74`), `advance_past_leaf`
  (`:76-79`), `reset` (`:81-95`; root never reused, comment `:89-94`).
- `lib/src/subtree.c` — `ExternalScannerState` init/copy/eq (`:24-63`),
  `ts_subtree_compress` (`:292`), `ts_subtree_summarize_children`
  (`:348-479`; error-child fragile/parse_state clearing `:446-449`,
  `first_leaf` caching `:467-470`), `ts_subtree_new_node` (`:491-528`),
  `ts_subtree_set_has_changes` (`:637-642`), `ts_subtree_edit`
  (`:645-815`), `ts_subtree_last_external_token` (`:800-812`),
  `ts_subtree_external_scanner_state_eq` (`:1087-1091`), inline threshold
  `TS_MAX_INLINE_TREE_LENGTH = UINT8_MAX` (`:22`), pool cap (`:23`).
- `lib/src/subtree.h` — `Subtree` data layout: inline flags `visible`,
  `extra`, `has_changes`, `is_missing`, `parse_state`, `lookahead_bytes`
  (`:53-71`); heap flags `fragile_left/right`, `has_changes`,
  `depends_on_column`, `has_external_scanner_state_change`, `error_cost`,
  `first_leaf` (`:115-152`); accessors `has_changes` (`:239`),
  `leaf_parse_state` (`:272-276`), `has_external_scanner_state_change`
  (`:364-366`), `is_fragile` (`:372-374`).
- `lib/src/tree.c` — `ts_tree_edit` (`:97-104`),
  `ts_tree_get_changed_ranges` (`:106-127`).
- `lib/src/get_changed_ranges.c` — `ts_range_array_intersects` (`:30-44`),
  `ts_range_array_get_changed_ranges` (`:46-104`), `ts_range_edit`
  (`:106-138`), diff iterator (`:140-340`), `iterator_compare`
  (`:348-395`), `ts_subtree_get_changed_ranges` (`:413-557`).
- `lib/src/lexer.c` — lex modes, included ranges, column tracking (lex-mode
  equality consumed via `TSLexerMode` memcmp in `parser.c:478-495`).
- `crates/cli/src/fuzz.rs` — upstream oracle: random edits + reparse + undo
  + reparse, compare against corpus CST and changed-range consistency
  (`:200-270`).

## 2. Problem actually solved

OBSERVED: interactive re-parsing of an edited source file. The public
contract (api.h `:285-293`): pass the previous tree as `old_tree` to
`ts_parser_parse` "so that the unchanged parts of it can be reused"; for
this to work "you must have already edited the old syntax tree using the
`ts_tree_edit` function in a way that exactly matches the source code
changes". The input change is a `TSInputEdit` (byte range + row/col points,
api.h `:118-131`). What is preserved: unchanged regions of the previous
concrete syntax tree, spliced by reference into the new tree. What is not
preserved or guaranteed: reuse identity (a fresh tree object is always
returned; the old tree remains the client's possession). Target latency:
none is stated in source; the mechanism aims to make work proportional to
the edit, not the file (INFERRED from design; no numeric target exists in
the engine — UNKNOWN how it behaves for specific grammars). Editor use is
the documented motivation (secondary docs retrieval, advanced-parsing page).

Full-parse vs update path: both go through `ts_parser_parse`; `old_tree ==
NULL` selects a fresh parse (`parser.c:2165-2168`, log "new_parse");
`old_tree != NULL` enables the reuse machinery (`parser.c:2150-2159`, log
"parse_after_edit"). A language mismatch with the old tree returns NULL
(`parser.c:2127-2130`). Mechanism-level fact only; no timings recorded here.

## 3. Retained representation

OBSERVED, all at the pinned SHA:

- The old syntax tree, retained by the client between parses; during a
  parse the engine re-roots it (`parser.c:2150-2152`) and releases it at
  the end (`parser.c:2100-2103`, in `ts_parser_reset`).
- Per-subtree metadata cached in the tree itself (`subtree.h:53-152`):
  `padding`/`size`/`lookahead_bytes` (byte + row/col extents), `symbol`,
  `parse_state` (the LR state at subtree start), cached `first_leaf`
  symbol + parse state for non-terminals (`subtree.c:467-470`), flags
  `has_changes`, `fragile_left/right`, `is_missing`, `is_error` (symbol
  comparison), `extra`, `visible`, `named`, `depends_on_column`,
  `has_external_scanner_state_change`, `error_cost`, `repeat_depth`.
- `ExternalScannerState`: an opaque byte buffer attached to subtrees that
  contain external tokens; inline small-buffer optimization; equality is
  length + memcmp (`subtree.c:24-63`). The last external token before any
  old-tree position is derivable via `ts_subtree_last_external_token`
  (`subtree.c:800-812`).
- A `ReusableNode` cursor (engine scratch state, not part of the tree):
  an explicit stack of `(subtree, child_index, byte_offset)` entries
  walking the old tree in pre-order, plus the last external token seen
  (`reusable_node.h:3-12`, `:39-60`).
- A one-entry `TokenCache` for the most recent lexed token
  (`parser.c:703-738`).
- Subtrees are refcounted; small subtrees are stored inline (≤255 bytes
  total, `subtree.c:22,155-163`); a small pool recycles heap subtrees
  (`subtree.c:23`).

## 4. Reuse unit

OBSERVED: the unit is a **subtree** of the old tree, of any size (a single
token up to any non-root node). The root is never reused, because the
finished root gains EOF/extra children at accept time
(`reusable_node.h:89-94`); at accept the root is rebuilt
(`parser.c:1077-1082`).

Reuse identity/granularity: a candidate is offered by the pre-order cursor
when its old-tree start byte equals the current parse position
(`parser.c:770-781`). There is no content fingerprint, no hash, no text
comparison anywhere in the acceptance path — OBSERVED absence (the only
structural comparison in `subtree.c` is `ts_subtree_compare` `:606-641`,
used for error-cost tie-breaking between stack versions, not for reuse).

Exact reuse predicate, in order, for candidate `result` at old-tree offset
`byte_offset` (`ts_parser__reuse_node`, `parser.c:753-836`):

1. Position gate: `byte_offset > position` → stop (parser has not caught
   up); `byte_offset < position` → descend into the candidate if it spans
   the position, else skip it (`parser.c:770-781`). EOF candidate: its end
   is treated as `UINT32_MAX` so it is never skipped by position
   (`parser.c:766-768`).
2. External-scanner entry-state equality: the last external token before
   the candidate in the old tree must have an `ExternalScannerState`
   equal to the last external token on the current parse stack
   (`parser.c:783-787`, equality at `subtree.c:1087-1091`). For grammars
   with no external tokens both sides are NULL and this passes trivially.
3. Rejection reasons (`parser.c:789-806`), any of which disqualify the
   candidate as a whole: `ts_subtree_has_changes(result)` (edit flag),
   `ts_subtree_is_error(result)`, `ts_subtree_missing(result)`,
   `ts_subtree_is_fragile(result)` (`subtree.h:372-374` — fragile_left or
   fragile_right; set for error symbols and error children
   `subtree.c:446-449,497-498,516-517` and for ambiguous/multi-path reductions
   `parser.c:1023-1025,1684`), or the candidate overlaps a changed
   *included range* (relevant only for injected-language partial parses;
   `parser.c:798-806` + `:740-751`).
   On rejection: descend into children if composite, else advance past and
   break down any reused subtree at the top of the parse stack
   (`parser.c:808-815`).
4. First-leaf lexical compatibility (`ts_parser__can_reuse_first_leaf`,
   `parser.c:470-503`): look up the LR action table entry for
   (current state, candidate's leaf symbol) (`parser.c:818-819`); the
   candidate passes if the leaf's lex mode (a struct including lex_state
   and external_lex_state) memcmp-equals the current state's lex mode and
   the table entry has actions — with a keyword special case requiring the
   leaf's own parse state to equal the current state
   (`parser.c:487-495`); empty tokens are not reusable across differing
   lookahead sets (`parser.c:498`); otherwise reuse requires the current
   state to have no external lexer state and the table entry to be flagged
   `is_reusable` (`parser.c:500-502`).
5. Accept: retain and return the candidate (`parser.c:830-832`).

## 5. Damage detection / invalidation

OBSERVED: two distinct "changed" computations, plus a consumer-facing diff.

(a) Edit-time per-node flags. `ts_tree_edit` (`tree.c:97-104`) rewrites
included ranges (`ts_range_edit`, `get_changed_ranges.c:106-138`) and calls
`ts_subtree_edit` (`subtree.c:645-815`): an iterative walk that, for every
subtree whose span (padding + size + lookahead) the edit touches, resizes
or shifts it (three cases: edit entirely before the content
`subtree.c:676-681`; edit straddling the padding/content boundary
`:682-687`; edit inside the subtree `:689-697`) and marks it
`has_changes` (`:735` via `ts_subtree_set_has_changes` `:637-642`). The walk
descends only into children overlapping the edit (`:738-800`); inserted
text is attributed to the first child touching the edit (`:772-781`);
column-dependent children are invalidated along the edited row when the
edit shifts columns (`depends_on_column`, `:666-667,750-767`). Net effect:
`has_changes` marks exactly the edited nodes and all their ancestors;
subtrees after the edit are untouched. INFERRED: this is why the predicate
needs no range list — the flags ARE the damage map.

(b) Included-range differences. At parse start, old vs new included ranges
are diffed (`parser.c:2153-2157`; `ts_range_array_get_changed_ranges`,
`get_changed_ranges.c:46-104`). This matters only for grammars parsed over
sub-ranges (injections); it participates in the reuse predicate (item 3
above) and in the parse loop's index maintenance (`parser.c:2218-2225`).

(c) `ts_tree_get_changed_ranges` (`tree.c:106-127`,
`get_changed_ranges.c:413-557`): post-parse, a joint in-order walk of the
(old, edited) and new trees comparing visible nodes; "matches" requires
equal start byte, alias/symbol, size, error cost, external-token presence,
no `has_changes` on the old side, non-error/non-NONE parse states, and
equal preceding external-scanner state (`iterator_compare`,
`get_changed_ranges.c:348-395`). Output: a merged range array of regions
whose visible structure differs (api.h `:466-492`).

## 6. Restart rule

OBSERVED: there is **no explicit restart-point search**. Parsing always
runs forward from position 0 over the (GLR) parse stack; the old tree is
consulted strictly in source order by the forward-only `ReusableNode`
cursor (only `advance`/`descend` exist, `reusable_node.h:39-74`). When
reuse fails at a position, the parser simply runs the normal lexer for that
token/region (`parser.c:1615-1643`) and continues; the cursor stays where
it is and can re-synchronize later, when its offset again equals the parse
position and the predicate passes (INFERRED from `parser.c:770-781` +
`reusable_node.h:39-60`; re-synchronization is not special-cased).

Context reconstruction: none is needed at restart because the LR state,
position, and last-external-token live on the parse stack, not in the old
tree. When a reused subtree *is* taken, the post-splice state is the
grammar's own transition from the current state (`:1664-1671`), and
composite candidates are first narrowed to a node whose stored
`parse_state` equals the current stack state (`ts_parser__breakdown_lookahead`,
`parser.c:224-244`, condition at `:232`).

Boundedness: single forward pass; the cursor visits each old-tree node a
bounded number of times (INFERRED from stack push/pop structure). No
backward rescan exists. OBSERVED: the "restart distance" is therefore
always the whole prefix; the mechanism's savings come from skipping, not
from restarting near the damage.

## 7. Convergence rule

OBSERVED: convergence is implicit and per-splice. A reuse acceptance ends
forward parsing for the candidate's whole span in one shift; the parser
continues from the candidate's end byte with a state derived from the
current grammar, not copied from the old tree (`parser.c:1662-1680`,
`ts_parser__shift` `:914-930`). For a composite candidate, the shifted
unit must satisfy `ts_subtree_parse_state(tree) == current state`
(`parser.c:232`). So the agreement set that ends a region's reparse is:

- byte-position alignment (old-tree offset == parse position),
- external-scanner entry-state equality,
- no edit/error/missing/fragile flags,
- leaf lex-mode equality (or `is_reusable` fallback) against the current
  LR state,
- a valid LR action for the candidate's leaf symbol in the current state.

One state agreement is enough per splice — the candidate is trusted for its
full extent after the shift — with a post-hoc safety net: if a later
lookahead has no valid action and the stack top is a reused (pending)
subtree, `breakdown_top_of_stack` pops it and pushes its children with
recomputed LR states, and parsing retries (`parser.c:1789-1798`, mechanics
`:176-222`, state recompute `:198-202`).

False convergence: OBSERVED that nothing downstream re-verifies a spliced
subtree's content against the source. If the client's `ts_tree_edit` does
not "exactly match the source code changes" (api.h `:289-293`), or an
external scanner's serialization omits relevant hidden state, a stale
subtree whose leaf symbol still admits a shift would be accepted. The
upstream guard is a test/fuzz oracle, not a runtime check (see §11).

## 8. Reconstruction

OBSERVED: the new tree is assembled by the ordinary GLR machinery: reused
subtrees are pushed onto the parse stack as single entries
(`ts_parser__shift` `:927`; pending flag for non-leaves) and become
children of newly created parents on reduction (`ts_parser__reduce`
`:932-1053`); at accept, the stack is popped and a fresh root node is built
(`ts_parser__accept` `:1054-1105`, new root `:1077-1082`). Reused subtrees
are shared by reference (refcount) with the old tree (`:831`, `:1074`);
only genuinely reparsed structure is newly allocated. After parsing,
`ts_parser__balance_subtree` (`:1912-1960`) compresses unbalanced
repeat-chains (`ts_subtree_compress`, `subtree.c:292`). Consumers can then
request the old-vs-new structural diff (§5c). The engine never mutates a
reused subtree's children during reconstruction (copy-on-write via
`ts_subtree_make_mut` when refcount > 1).

## 9. Position / range maintenance

OBSERVED: the client's `ts_tree_edit` is the position-maintenance step.
`ts_subtree_edit` rewrites each affected subtree's `padding`/`size` in
bytes AND row/column extents (`subtree.c:693-735`), converting the edit
into each child's coordinate space (`:770-773`) — no text rescan happens;
row/col arithmetic uses the `TSInputEdit` points supplied by the caller
(api.h `:455-464` requires the edit "both in terms of byte offsets and in
terms of (row, column) coordinates"). Included ranges are patched the same
way (`ts_range_edit`, `get_changed_ranges.c:106-138`). The new tree's
offsets are produced fresh by the parse (lexer positions + pushed subtree
extents); reused subtrees already carry post-edit offsets because the old
tree was patched first. External `TSNode` handles are updated by the client
via `ts_node_edit` if needed (secondary docs retrieval; `api.h:732-740`).

## 10. Fallback

OBSERVED triggers for abandoning reuse, and what happens:

- Candidate rejected by any predicate clause (§4): descend to children
  (smaller reuse unit) or advance; the affected token/region is then
  produced by the ordinary lexer (`parser.c:1624-1643`).
- No valid LR action after a splice: top-of-stack reused subtree is broken
  down into children and retried; if that also fails, the version enters
  error handling (`parser.c:1789-1807`).
- Error recovery: `handle_error`/`recover` skip tokens into ERROR nodes,
  insert missing tokens (`parser.c:1477-1572`, `:1288+`); error, missing,
  and fragile nodes are permanently excluded from future reuse by the
  predicate (`parser.c:792-797`), so damage near error regions reparses.
- GLR ambiguity/errors create additional stack versions; node reuse is
  attempted only while `version_count == 1` (`parser.c:2179`), i.e. any
  live ambiguity or error-split disables reuse until versions merge or are
  condensed (`:1811+`).
- There is no internal fallback to a full clean parse mid-file; a full
  parse happens only when the client passes `old_tree == NULL`.

## 11. Correctness authority

OBSERVED classification: tree-sitter's safety argument is a **mechanism
rule** — (edit-exclusion flags) + (grammar state agreement at every splice
and every subsequent token) + (hidden-state equality for external
scanners) + (breakdown backdown) — implemented in C with no runtime
content verification. It is not a content-diff implementation heuristic,
and it is not proven formally in-repo. The empirical oracle is upstream's
fuzz harness: random edit series are applied, the tree is reparsed
incrementally, edits are undone and reparsed again, and results are checked
against the corpus CST and for changed-range consistency
(`crates/cli/src/fuzz.rs:200-270`; also described in the official docs
retrieved via Context7, `docs/src/cli/fuzz.md`). The API doc states the
precondition as a caller obligation ("exactly matches the source code
changes", api.h `:289-293`) — i.e., correctness of reuse is conditional on
client-supplied edit truth, not self-checked.

## 12. Known limitations

Evidence-backed from source:

- The root is never reused; the top-level node is always rebuilt
  (`reusable_node.h:89-94`, `parser.c:1077-1082`).
- Reuse is disabled whenever more than one GLR stack version exists
  (`parser.c:2179`; the flag is recomputed at each outer parse-loop
  iteration after condense, so suppression is interval-shaped, not
  sticky) — ambiguous or error-split regions run without reuse while
  their versions remain live.
- Error/missing/fragile subtrees are never reused, and fragility
  propagates to parents of error children and ambiguous reductions
  (`parser.c:792-797`, `subtree.c:446-449`, `parser.c:1023-1025`).
- The `ReusableNode` cursor is forward-only; a candidate can be skipped
  permanently if the parse position jumps past it (`parser.c:775-781`).
- `ts_tree_get_changed_ranges` documentation warns ranges "may be slightly
  larger than the exact changed areas" (api.h `:484-487`).
- Reuse validity is conditional on caller-supplied `TSInputEdit` exactness;
  the engine does not verify it (api.h `:289-293`).
- Column-dependent structure (`depends_on_column`) forces invalidation of
  same-row siblings when columns shift (`subtree.c:665-670,758-768`) —
  cost bounded by row, but triggered by any column-shifting edit.

HYPOTHESIS FOR R3/R8 (not established by this extraction): behavior under
pathological error-heavy editing; interaction of `MAX_VERSION_COUNT = 6`
condensation (`parser.c:77,1811+`) with reuse availability; cost balance of
`ts_tree_get_changed_ranges` for consumers.

## 13. Adversarial hypotheses

All items HYPOTHESIS unless marked OBSERVED. (Q1-Q12 per R2 brief.)

- Q1 (what old info survives): the entire old tree, with patched offsets
  and per-node parse_state/first_leaf/scanner-state/error-cost metadata
  (§3). OBSERVED.
- Q2 (who vouches): the LR action table + edit flags + scanner-state
  equality + breakdown backdown (§4, §7). No content comparison exists —
  OBSERVED absence.
- Q3 (maximally invalidating edit): an edit that changes LR-state
  trajectories early (e.g., a Markdown fence opener or container-prefix
  change) forces lex-mode/action mismatches and descend-past-rejection at
  each old node; because offsets are patched (§9), a pure prefix insertion
  does NOT by itself break later reuse — structural/state divergence, not
  byte shift, is the invalidation channel. HYPOTHESIS about magnitude;
  channels are OBSERVED.
- Q4 (tiny edit → O(N)): cursor skipping is O(old-tree nodes) in the worst
  case regardless of edit size; `has_changes` marking is O(edited path);
  error-triggered multi-version phases can disable reuse entirely
  (`parser.c:2179`). Whether any O(N) term dominates is a benchmark
  question. HYPOTHESIS.
- Q5 (far-forward propagation): only via (a) external-scanner state
  mismatch gate (`parser.c:783`), (b) state divergence at splice points,
  (c) fragile/error exclusion. No explicit far-forward marking exists.
  Channels OBSERVED; reach HYPOTHESIS.
- Q6 (parse saved vs reconstruction paid): splicing is refcount-retention
  (`:831`); reconstruction costs are new parents + fresh root + balancing
  (`:1912-1960`) + optional consumer diff. Balance is not free and is
  proportional to repeat-chain depth imbalance, not to reuse amount.
  OBSERVED mechanics; net win HYPOTHESIS.
- Q7 (position maintenance erasing reuse benefit): `ts_subtree_edit`
  touches only edit-overlapping nodes (§5), so maintenance is
  edit-proportional by construction; row/col updates ride the same walk.
  OBSERVED; benefit-erasure HYPOTHESIS (unlikely from structure alone).
- Q8 (memory ∝ N): the old tree must be retained across edits by the
  client — that is the mechanism's retained state; during a reparse old
  and new trees coexist, sharing reused subtrees via refcount (§8).
  OBSERVED; peak-factor magnitude UNKNOWN without measurement.
- Q9 (hidden grammar/parser state): external-scanner state serialized per
  token and compared at reuse (`parser.c:783`, `subtree.c:24-63`); lex
  modes per LR state compared at first-leaf check (`parser.c:478-495`);
  keyword ambiguity via leaf parse-state equality (`parser.c:492-494`).
  Completeness of a scanner's serialization is the grammar author's
  responsibility — a residual hidden-state risk. OBSERVED mechanism,
  residual risk HYPOTHESIS.
- Q10 (false convergence): possible if edit flags are wrong (client bug),
  a scanner under-serializes state, or a stale subtree's leaf symbol still
  admits a shift — nothing re-checks content downstream (§7). Mechanism
  OBSERVED; realized false convergence HYPOTHESIS.
- Q11 (fallback frequency dominating): in error-heavy or highly ambiguous
  regions, reuse is suppressed WHILE multiple GLR stack versions exist
  (`version_count > 1`; recomputed at each outer parse-loop iteration, so a
  later condense back to one version re-enables it) and error/fragile
  exclusion forces local reparse; how much of such workloads is covered by
  suppression intervals vs post-condense reuse recovery is HYPOTHESIS,
  gated on the OBSERVED version-count rule.
- Q12 (mechanism-intrinsic vs implementation-specific): intrinsic —
  retained old tree with per-subtree parse state + edit flags, in-order
  reuse cursor, state-agreement splice, scanner-state equality gate.
  Implementation-specific — GLR error-cost policy and version caps, inline
  subtree encoding, subtree pool, repeat balancing, included-range
  injection machinery, wasm store, progress callbacks.

## 14. Relevance assessment for the benchmark

Tree-sitter contributes **old-tree/edit-aware subtree reuse concepts**
within the documented fidelity boundary. Concretely transferable to
H3-style modeling:

- the edit-flags-on-old-tree invalidation pattern (`ts_subtree_edit` +
  `has_changes`) as an alternative to recomputed damage ranges;
- the reuse predicate's shape: position alignment + damage exclusion +
  hidden-state equality + grammar-state/action agreement + backdown;
- the observation that a reuse decision can be content-blind if and only
  if the edit mapping is trusted (correctness-authority question for §11
  comparisons with oracles);
- convergence conditions for suffix reuse (state equality + entry-state
  equality + position equality) — H4-relevant;
- reuse suppression while multiple GLR stack versions exist (with
  condense-dependent recovery) — a fallback-policy data point.

We would NOT reproduce (non-goals for any horse): the generated LR parse
tables and GLR error-recovery policy; the C runtime, inline-subtree
encoding, subtree pool, or allocators; the query engine; included-range
injection; wasm store; repeat-chain balancing; progress/cancellation
machinery. Horses are mechanism models under the R0 fidelity naming rule
(`<project>-inspired`); no claim that H3 reproduces tree-sitter is made or
implied.

## 15. Manifest entry

```toml
[[prior_art]]
id = "tree-sitter-core-incremental"
name = "Tree-sitter core incremental parsing engine (GLR/LR + old-tree subtree reuse)"
source_type = "upstream_source_pinned"
repo_url = "https://github.com/tree-sitter/tree-sitter"
tag = "v0.27.0"
commit_sha = "6070dbfefd326bd735e5683eb128cc1b57dad0c0"
retrieved_date = "2026-09-16"
license = "MIT"
relevant_paths = [
  "lib/include/tree_sitter/api.h",
  "lib/src/parser.c",
  "lib/src/reusable_node.h",
  "lib/src/subtree.c",
  "lib/src/subtree.h",
  "lib/src/tree.c",
  "lib/src/get_changed_ranges.c",
  "lib/src/lexer.c",
  "crates/cli/src/fuzz.rs",
]
notes = """
Core engine record, not a Markdown grammar record. Reuse unit = old-tree
subtree (root excluded); predicate = position alignment + !has_changes +
!error/missing/fragile + external-scanner entry-state equality + included-
range disjointness + first-leaf lex-mode/action-table compatibility; splice
state = current grammar transition (never copied from old tree); composite
candidates narrowed by parse_state == current state; backdown =
breakdown_top_of_stack. Reuse off while GLR version_count > 1
(parser.c:2179). No content verification; correctness conditional on exact
caller ts_tree_edit; upstream oracle = fuzz harness. Secondary source:
Context7 /tree-sitter/tree-sitter docs (advanced-parsing page, fuzz page).
"""
```
