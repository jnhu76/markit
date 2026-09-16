# Prior-art mechanism record — tree-sitter-markdown (grammar + external scanner)

R2 stage (prior-art mechanism extraction) of MARKIT-MARKDOWN-BENCHMARK-1.
Authority: `research/benchmarks/markdown-ast-update/protocol/R0-METHODOLOGY.md` §2.

Evidence rules applied: OBSERVED claims cite `file:line` at the pinned commit
(or an official issue number); INFERRED claims are labeled; UNKNOWN is stated
as unknown. No timing numbers, no performance claims, no benchmark execution.
The mechanism is extracted as it exists; relevance to H0–H4 is assessed only
in §14. This record covers the **Markdown grammars** (block + inline) and
their external scanners — the engine itself is recorded separately in
`tree-sitter.md` (v0.27.0), which this record cross-references for all
engine-level behavior.

## 1. Identity

```text
PROJECT:    tree-sitter-markdown (maintained fork of ikatyang's original;
            two grammar packages: block-level `tree-sitter-markdown` and
            `tree-sitter-markdown-inline`)
VERSION:    v0.5.3 (latest release tag at retrieval; tag dated 2026-02-26)
COMMIT_SHA: f969cd3ae3f9fbd4e43205431d0ae286014c05b5 (tag v0.5.3 == HEAD of
            default branch `split_parser`; a `main` branch exists that is
            ahead — be3e08acfd85bd87d85f41fde74fdcec25f76dbe — not pinned)
REPO_URL:   https://github.com/tree-sitter-grammars/tree-sitter-markdown
RETRIEVED:  2026-09-16 (clone into /tmp/markit-r2-prior-art/tree-sitter-markdown)
LANGUAGE:   JS grammar definitions -> generated C LR tables; hand-written C
            external scanners; CommonMark 0.30 target
LICENSE:    MIT (LICENSE; "Copyright (c) 2021 Matthias Deiml")
```

PRIMARY SOURCES: repository source at the pinned SHA (sole mechanism
authority). SECONDARY SOURCES: (a) Context7 retrieval of official tree-sitter
docs (`/tree-sitter/tree-sitter`, `docs/src/creating-parsers/4-external-scanners.md`
— serialize is "called after a token is successfully recognized", buffer cap
`TREE_SITTER_SERIALIZATION_BUFFER_SIZE`, deserialize must clear state first);
(b) official GitHub issues on the subject repo: #92, #134, #184, #242, #243,
#209, #202, #186, #114, #257.

RELEVANT FILES / FUNCTIONS (paths relative to repo root, v0.5.3):

- `tree-sitter-markdown/grammar.js` — document/section structure
  (`:17-24, :50-89`), fenced code block (`:165-184`), paragraph
  branch-kill tactic (`:263-281`), block_quote (`:295-301`),
  list/list_item (`:310-379`), `_newline` (`:383-390`), externals list +
  block-structure comment (`:495-583`, esp. `:500-511`), `$._error` role
  (`:571-574`), `extras: []` (`:594`).
- `tree-sitter-markdown/src/scanner.c` — `TokenType` enum, 47 tokens
  (`:9-57`); `Block` enum, 18 kinds (`:67-88`); list-indentation encoding
  (`:99-101`); `paragraph_interrupt_symbols` (`:124-172`); state bitflags
  (`:174-181`); `Scanner` struct (`:197-219`); `serialize`/`deserialize`
  (`:239-289`); `advance` (tab-stop column, `:317-327`); `match()`
  (`:334-400`); `parse_fenced_code_block` (`:402-461`);
  star/plus/ordered/minus marker parsers (`:463-544, :628-728, :730-796,
  :798-935`); pipe-table lookahead (`:1178-1312`); `scan()`: EOF closing
  (`:1331-1343`), non-matching (`:1345-1431`) and matching (`:1432-1467`)
  phases, line-ending simulation (`:1469-1541`); entry points
  (`:1565-1602`).
- `tree-sitter-markdown/src/parser.c` (generated) — `STATE_COUNT 925`,
  `LARGE_STATE_COUNT 351` (`:18-19`), `EXTERNAL_TOKEN_COUNT 47` (`:23`),
  `ts_external_scanner_states[48][47]` (`:59057`).
- `tree-sitter-markdown/src/tree_sitter/parser.h` —
  `TREE_SITTER_SERIALIZATION_BUFFER_SIZE 1024` (`:14`).
- `tree-sitter-markdown-inline/src/scanner.c` — 15-token enum (`:10-26`);
  `STATE_EMPHASIS_DELIMITER_IS_OPEN` (`:39`); `Scanner` (`:58-67`);
  `serialize`/`deserialize`, fixed 4 bytes (`:70-93`); leaf-delimiter
  parse-ahead (`:95-137`); `parse_star` delimiter-run state (`:153-213`).
- `tree-sitter-markdown-inline/src/parser.c` (generated) —
  `STATE_COUNT 1161`, `EXTERNAL_TOKEN_COUNT 15` (`:18-23`),
  `ts_external_scanner_states[27][15]` (`:75461`).
- `tree-sitter-markdown-inline/grammar.js` — externals incl. never-emitted
  `$._last_token_whitespace`/`$._last_token_punctuation` probes (`:28-62`,
  esp. `:46-49`); context-variant generation `add_inline_rules`
  (`:392-474`).
- `tree-sitter-markdown/queries/injections.scm` — injects
  `markdown_inline` into `(inline)` nodes; other languages into fenced
  code / html / metadata.
- `README.md` — two-pass standalone usage (`:54-63`); WASM caveats
  (`:67-71`). `common/common.js` — compile-time extension toggles
  (`:1-10`).

## 2. Problem actually solved

OBSERVED: interactive parsing of Markdown block structure for editors under
tree-sitter's LR/GLR engine. The grammar comment states the division of
labor: "After every newline (`$._line_ending`) we try to match as many open
blocks as possible... For this process the external scanner keeps a stack
of currently open blocks" (grammar.js `:500-503`). The external scanner
exists because Markdown block recognition depends on cross-line context
(open-container stack with indentation arithmetic, open-fence delimiter
length) that LR lookahead cannot express; `extras: $ => []` (`:594`) means
whitespace is never implicitly skipped — every line's indentation is
scanner-visible. Inline structure (emphasis, code spans, links) is
delegated to a second grammar, `markdown_inline`, parsed separately over
ranges the block tree marks as `inline` (README `:54-63`). Retained across
edits: the engine tree plus the serialized scanner state attached to
external tokens (§3). No latency target is stated anywhere in the repo
(UNKNOWN by design; none claimed here).

## 3. Retained representation

Engine-level (cross-ref `tree-sitter.md` §3): old tree with per-subtree
`parse_state`, `has_changes`, and an `ExternalScannerState` byte buffer
attached to subtrees containing external tokens; equality is length+memcmp.

Grammar-level, OBSERVED at pinned SHA:

- **Block scanner `Scanner` struct** (scanner.c `:197-219`):
  - `open_blocks`: a dynamic stack of `Block` values — the currently open
    container/leaf blocks. `Block` is an 18-value enum (`:67-88`):
    `BLOCK_QUOTE`, `INDENTED_CODE_BLOCK`, `LIST_ITEM` .. `LIST_ITEM_MAX_INDENTATION`
    (13 variants encoding content-indentation level 2..15 via
    `LIST_ITEM + n`, `:99-101`), `FENCED_CODE_BLOCK`, `ANONYMOUS` (html
    blocks etc. whose close is grammar-driven). Marker character identity
    (`-` vs `+` vs `*`) is NOT in the stack — only indentation class.
  - `state`: bitflags `STATE_MATCHING` (0x1, at line start matching open
    blocks), `STATE_WAS_SOFT_LINE_BREAK` (0x2), `STATE_CLOSE_BLOCK` (0x10)
    (`:174-181`).
  - `matched`: how many open blocks matched so far this line; reset after
    each line ending (`:208-210`).
  - `indentation`: "consumed but unused" indentation; tabs are split across
    tokens (`:211-213`).
  - `column`: current column mod a 4-space tab stop (`advance`, `:317-327`).
  - `fenced_code_block_delimiter_length`: opening fence length of the
    currently open fenced code block (`:215-216`).
  - `simulate`: transient (lookahead simulation inside a line-ending
    decision, `:1494`); NOT serialized.
- **Block serialize/deserialize** (`:239-289`): 5 scalar bytes
  (`state, matched, indentation, column, fenced_code_block_delimiter_length`)
  followed by the whole `open_blocks` stack (`memcpy` of
  `size * sizeof(Block)`; C enum size is implementation-defined, typically
  4 bytes). Deserialize resets everything then reads symmetrically.
  There is no bounds check against the 1024-byte engine serialization
  buffer (`parser.h:14`).
- **No mutable global state**: all file-scope statics in both scanners are
  `const` tables (`paragraph_interrupt_symbols`, HTML tag-name lists);
  scanner instances are malloc'ed per parser (`:1565-1576`). OBSERVED
  absence — serialized state is the only retained scanner state.
- **Inline scanner `Scanner`** (inline scanner.c `:58-67`): `state` flags
  (incl. `STATE_EMPHASIS_DELIMITER_IS_OPEN`, `:39`),
  `code_span_delimiter_length`, `latex_span_delimiter_length`,
  `num_emphasis_delimiters_left` (rest of current delimiter run). Fixed
  4-byte serialization (`:70-93`). This state is what makes `*a*b*`-style
  runs decide open/close once for the whole run (`:187`, `:312`).
- **Per-LR-state valid-token sets** (engine table, generated): block
  `ts_external_scanner_states[48][47]` over 925 states (parser.c `:59057`);
  inline `[27][15]` over 1161 states (inline parser.c `:75461`). These tell
  the scanner which of its 47/15 tokens are legal in the current LR state —
  the scanner is thus a pure function of (serialized state, valid_symbols,
  remaining input) — that purity is the correctness assumption (§11).

## 4. Reuse unit

Engine-level: subtree of the old tree, gated by position alignment, edit
flags, error/missing/fragile exclusion, and **external-scanner entry-state
equality** — the `ExternalScannerState` of the last external token before
the candidate must byte-equal the live scanner's state at the splice point
(`tree-sitter.md` §4, item 2).

Grammar-level (what the equality gate concretely compares for Markdown):
the candidate's snapshot `(state flags, matched, indentation, column,
fence_delimiter_length, open_blocks[0..depth])` must equal the incremental
parse's live state at that byte. INFERRED consequence: a subtree whose text
is byte-identical to post-edit source is still non-reusable whenever any of
those fields differs — e.g. one extra open block quote, or a different
`fenced_code_block_delimiter_length`. The unit of reuse is therefore
"subtree with compatible scanner context", and the state bytes are the
compatibility key. OBSERVED mechanisms; quantified invalidation reach is a
benchmark question (HYPOTHESIS).

## 5. Damage detection / invalidation

Engine-level: `ts_tree_edit` marks `has_changes` on the edited path only
(`tree-sitter.md` §5a); the scanner contributes no grammar-specific damage
logic. Grammar-level channels, OBSERVED:

- Scanner-state mismatch at the reuse gate (§4): any edit that changes the
  serialized state for subsequent lines acts as forward damage detection —
  each subsequent candidate is rejected and reparsed until state converges
  again (or document end).
- Fence state: `parse_fenced_code_block` closes a fence only if the run
  length ≥ the stored opening length (`:415-417`); the opening length is
  serialized, so an edit to an opening fence invalidates matching of its
  closing fence arbitrarily far forward (reach HYPOTHESIS).
- Container stack: `match()` consumes one continuation per open block per
  line (`:334-400`, `:1432-1450`); the stack itself is the damage-carrier
  for depth changes.
- The grammar re-nests whole `section` subtrees by ATX level
  (grammar.js `:50-89`): editing a heading marker changes parent topology
  for everything under it until a same-or-higher heading. INFERRED: leaf
  blocks beneath may still pass the reuse gate; the visible churn is
  topological, but engine parse_state agreement can also be broken — not
  established here (UNKNOWN without measurement).

## 6. Restart rule

Engine-level: forward-only pass from offset 0; no restart-point search
(`tree-sitter.md` §6). Grammar-level, OBSERVED: the block scanner is a
line-synchronous state machine — all container matching and block-start
decisions happen at line boundaries (`grammar.js:500-511`; scanner.c
`:1345-1467`), and each `$._newline` re-establishes `STATE_MATCHING` and
resets `matched` (`:1542-1560`). Line starts are therefore the natural
checkpoint density of this design. Paragraph termination uses a bounded GLR
branch split at every intra-paragraph newline; each branch is killed by an
emitted `$._error` by the next newline ("after the next newline only one
branch will exist", grammar.js `:263-281`, `:571-574`). Restart state needed
to resume mid-document is exactly the serialized `Scanner` (§3) plus LR
stack — nothing else is consulted.

## 7. Convergence rule

Engine-level: convergence per splice = byte alignment + scanner-state
equality + no damage flags + leaf/lex-mode compatibility (`tree-sitter.md`
§7); reused subtrees are trusted to their full extent.

Markdown-specific hazards, from the serialized state contents (§3):

- Container-depth edits: opening one extra block quote at line k changes
  the serialized stack at every subsequent external token; convergence
  (state equality restored) requires the stack to return to its pre-edit
  contents, which for an unclosed container is document end. INFERRED from
  the equality gate + state contents; distance is workload-dependent
  (HYPOTHESIS for magnitude).
- Fence edits: `fenced_code_block_delimiter_length` persists until the
  closing fence (`:422`, reset `:455`); convergence points are the closing
  fence line.
- The `matched` counter is serialized even mid-line (snapshot taken after
  every token, incl. `BLOCK_CONTINUATION`), so state equality also encodes
  mid-line progress; two snapshots at different columns of the same line
  are unequal by construction. OBSERVED field; consequences HYPOTHESIS.
- Marker-type aliasing: the stack records list indentation class, not the
  marker character (`:67-88`); a `-` list and a `+` list at equal
  indentation serialize identically. False convergence via this aliasing is
  blocked only by the engine's separate LR-state agreement at the splice
  (`tree-sitter.md` §4 item 4 / §7). OBSERVED aliasing; realized wrong
  reuse UNKNOWN (no issue found either way — would require the two LR
  states to share a lex mode).

## 8. Reconstruction

Engine-level: splice by reference + rebuild of unreduced parents
(`tree-sitter.md` §8). Grammar/composition-level, OBSERVED: the document
truth for this design is TWO parses. The block grammar aliases line content
to `$.inline` leaves (grammar.js `:135`, `:281`); the inline grammar is then
run over those ranges — either via the block grammar's own
`injections.scm` (`(inline) @injection.content` -> `markdown_inline`) or
programmatically via `ts_parser_set_included_ranges` ("first parse the
document with the block grammar. Then perform a second parse with the
inline grammar", README `:54-63`). INFERRED: an edit confined to inline
content of an unchanged block structure can in principle leave the block
tree fully reused and only reparse the inline range; the composition and
its reuse behavior across the two parses is host-level (no unified CST in
this repo). What the inline parse itself reuses is governed by the same
engine gate with the inline scanner's 4-byte state (§3).

## 9. Position / range maintenance

Engine-level: client `ts_tree_edit` patches byte + row/column extents;
new offsets come from the parse (`tree-sitter.md` §9). Grammar-level,
OBSERVED: the block scanner maintains its own internal `column` — a
mod-4 tab-stop counter used only for indentation decisions (`advance`,
scanner.c `:317-327`), not a source coordinate. It is serialized (§3)
because indentation legality (≤3 spaces for block starts, 4+ for indented
code) is column-dependent. Grammar coordinates that reach the output tree
are engine bytes/points; the scanner adds no persistent coordinate system.
Note for §16 of the protocol: tabs make `column` and `indentation` distinct
(a tab may be "split" between tokens, `:210-213`), so scanner state is not
reconstructible from byte offsets alone.

## 10. Fallback

Engine-level: reuse rejection -> ordinary lexer for that token/region;
no-action lookahead -> `breakdown_top_of_stack` backdown -> GLR error
recovery; error/missing/fragile subtrees excluded from future reuse
(`tree-sitter.md` §10). Grammar-level, OBSERVED: the `$._error` token is a
branch-kill device, not recovery (`:571-574`); there is no grammar-defined
fallback to clean parse. `TRIGGER_ERROR`/`_no_indented_chunk` exist to let
ordinary rules veto scanner tokens (`:566-569`, scanner.c `:1317-1319`).
WASM targets need static linking of the scanner's C helpers (README
`:67-71`, upstream issues #93/#126 referenced there) — a packaging-level
fallback constraint, not a parsing one.

## 11. Correctness authority

OBSERVED: (a) the serialize/deserialize pair is declared "fully symmetric"
(scanner.c `:256`) and is the sole carrier of scanner state across parse
resumptions and across engine reuse gates; (b) the engine accepts a reused
subtree only when that serialized state byte-equals the live state
(`tree-sitter.md` §4); (c) the engine never re-verifies spliced content
against source (`tree-sitter.md` §7/§11). Together the correctness argument
is: the scanner must be a deterministic function of (serialized state,
per-LR-state valid_symbols, remaining input), so equal state + equal source
implies equal future token stream. Completeness of the serialization is the
grammar author's burden with no runtime check — issue #243 (buffer overflow
in `serialize`, open since 2026-04-28) shows even the *size* discipline of
this carrier failed in practice. Upstream empirical oracle: corpus tests
(`tree-sitter-markdown/test/corpus/`, `tree-sitter-markdown-inline/test/corpus/`)
plus engine fuzz (`tree-sitter.md` §11). No formal proof exists in-repo.

## 12. Known limitations

Official-issue evidence (subject repo):

- **#243** "bug: Buffer overflow in serialize in scanner.c" (open,
  2026-04-28, filed by tree-sitter core maintainer casouri; linked PR #259):
  `memcpy` of `open_blocks` (scanner.c `:248`) can exceed the fixed
  1024-byte engine serialization buffer when the container stack is deep;
  reporter observed crashes in Emacs and confirms fixing the overflow
  resolved them. OBSERVED. INFERRED bound: with 5 header bytes and a
  typical 4-byte enum, overflow needs roughly >254 simultaneously open
  blocks (exact sizeof(Block) implementation-defined).
- **#92** "markdown_inline can't reparse old trees (web-tree-sitter)"
  (closed 2024-02-18 via PR #134): passing an old tree to `parse` made
  emphasis nodes vanish on subsequent edits (each edit dropped the next
  emphasis node). OBSERVED report. Fix (PR #134, commit 2821521, branch
  `stable-underscores`, merged 2024-02-18, verified in clone history)
  rewrote the underscore open/close decision to depend only on the
  per-LR-state `valid_symbols` (`LAST_TOKEN_WHITESPACE`/`LAST_TOKEN_PUNCTUATION`
  — grammar-state probes, inline grammar.js `:46-49`) plus lookahead, and
  to clear `STATE_EMPHASIS_DELIMITER_IS_OPEN` deterministically. The exact
  root cause was never stated on the issue (UNKNOWN as diagnosed); the fix
  pattern supports INFERRED mechanism: the pre-fix decision depended on
  live scanner state that the engine's restore-at-reuse path did not
  reproduce, i.e. hidden-state nondeterminism at reuse boundaries.
- **#257** "Single-digit ordered markers other than 1 interrupt paragraphs
  in v0.5.3 (CommonMark deviation)" (open, 2026-08-24) — deviation present
  in the pinned version despite commit cee71b8 "#226 allow ordered lists to
  start from any number" being included. OBSERVED.
- **#184** "Using --- within a code fence breaks fenced_code_block"
  (closed, 2025-02-11): `---` lines inside a fence are swallowed into a
  `minus_metadata` node; fence/info-string interplay. OBSERVED.
- **#242** "Inconsistent (and incorrect) pipe table parsing errors" (open,
  2026-04-21); **#209** "Bold text ending with colon creates massive
  overlapping ranges" (open, 2025-10-31); **#202** "incorrect parsing of
  indented single -" (open, 2025-09-15); **#186** "image ... more than 8
  images on adjacent lines" (open, 2025-02-24); **#114** "Neovim crashes
  when editing multilevel list" (closed, 2023-11-05). OBSERVED titles/
  states/dates; internal mechanisms UNKNOWN (not diagnosed in fetched
  content).
- README `:67-71`: WASM/web-tree-sitter "does not work out of the box"
  (missing C exports). OBSERVED.

HYPOTHESIS FOR R3/R8: deep-container edits above the serialization bound
(pre-#259) as a crash-class fallback; interplay of #243-class state growth
with benchmark DEEP_CONTAINER payloads; whether section re-nesting (§5)
breaks engine parse_state agreement forward.

## 13. Adversarial hypotheses

All HYPOTHESIS unless marked OBSERVED. (Q1-Q12 per R2 brief.)

- Q1 (old info surviving): the entire per-token scanner snapshots (§3)
  survive in the tree; `matched`/`column`/fence length are old facts a new
  parse must reproduce exactly to reuse anything. #92 is a realized case
  of stale-state interaction. OBSERVED carriers; wrongness instances as
  cited.
- Q2 (who vouches): no content hash anywhere; the voucher is the
  determinism assumption "equal serialized state ⇒ equal future tokens"
  plus engine LR-state agreement (§11). OBSERVED absence of content checks.
- Q3 (maximally invalidating edit): a container opener inserted early in
  the document (e.g. `> ` at line 1) changes the serialized stack at every
  subsequent external token, plausibly killing forward reuse wholesale.
  Channel OBSERVED (§4/§5); magnitude HYPOTHESIS.
- Q4 (tiny edit -> O(N)): same channel — a one-byte container/fence edit
  may invalidate state equality for O(N) lines; whether engine reparse then
  costs O(N) is a benchmark question. HYPOTHESIS.
- Q5 (far-forward propagation): yes-by-construction channels exist — fence
  length (until closing fence), container depth (until closure), carried
  in serialized state, not in any edit-range computation. OBSERVED
  carriers; distances workload-dependent (HYPOTHESIS).
- Q6 (parse saved vs reconstruction paid): two-pass design pays a second
  parse pass for inline ranges; block/inline split can save block work for
  inline-only edits (§8). Both OBSERVED structurally; net balance
  HYPOTHESIS.
- Q7 (position maintenance erasing reuse): scanner adds no source
  coordinate maintenance; its internal `column`/`indentation` are recomputed
  during reparse and only compared via snapshots. OBSERVED; benefit-erasure
  not indicated.
- Q8 (memory ∝ N): every external token in the tree carries a serialized
  state of size 5 + depth*sizeof(Block) bytes (block grammar); in
  DEEP_CONTAINER payloads retained memory scales with depth × external-
  token count, i.e. worse than O(N) flatness in depth. Carrier OBSERVED;
  total-memory claim HYPOTHESIS (needs measurement per protocol).
- Q9 (hidden scanner state): this design IS the Q9 exhibit — cross-line
  state invisible in the CST, serialized per token, correctness-critical,
  unbounded in size (#243), and historically a source of incremental-parse
  wrongness (#92). OBSERVED.
- Q10 (false convergence): the marker-type aliasing (§7) shows serialized
  state is not injective over semantic context; engine LR-state agreement
  is the second gate. Whether any real edit produces equal scanner state +
  equal LR state while semantics differ: UNKNOWN (none found).
- Q11 (fallback frequency): paragraph GLR splits fire at every
  intra-paragraph newline (grammar.js `:263-281`); engine reuse is gated on
  single-version phases (`tree-sitter.md` §10) — whether splits suppress
  reuse often in prose workloads is HYPOTHESIS, testable under the #22
  protocol.
- Q12 (mechanism-intrinsic vs implementation-specific): intrinsic — a
  container-context stack with indentation arithmetic, open-fence length,
  line-synchronous matching, and serialized-state-gated reuse; the *need*
  for these is Markdown's, not tree-sitter's. Implementation-specific —
  the 5-byte packing, 18-value Block enum encoding, `matched` counter,
  GLR ERROR branch-kill tactic, `section` re-nesting, two-grammar split
  mechanics, compile-time extension flags.

## 14. Relevance assessment for the benchmark

This grammar is the strongest prior-art demonstration that **Markdown
incremental mechanisms must retain hidden cross-line state outside the
visible tree**, and that the retained set is small, enumerable, and
line-synchronous. Transferable evidence, per horse:

- **H1 (BLOCK_LOCAL_REPARSE)**: to reparse an arbitrary block locally, one
  must first reconstruct the block's entry context. The `Scanner` struct
  (§3) is a maintainer-authored enumeration of what that context is:
  container stack with per-container content indentation (13 list
  variants), phase flags, partial-line indentation, tab-split column, and
  open-fence length. Any H1 "preserve unaffected block states" step must
  reproduce at least this set (fidelity: our model, not the C scanner).
- **H3 (OLD_TREE_SUBTREE_REUSE)**: the engine's scanner-state equality
  gate shows subtree reuse in Markdown is conditional on hidden-state
  compatibility, not byte ranges; container/fence edits are the natural
  maximally-invalidating mutation family for the benchmark's STRUCTURAL_EDIT
  set (Q3/Q4/Q5 above give concrete mutation targets).
- **H4 (RESTART_CONVERGENCE)**: convergence cannot be byte-alignment only;
  it must include equality of the serialized context (§7). The state
  contents define the checkpoint payload per line start, and container
  edits define worst-case convergence distance — directly relevant to the
  protocol's restart-distance / convergence-distance counters.
- **Split grammar**: evidence that "block damage" and "inline damage" are
  separable damage classes with different state carriers (block: container
  stack; inline: delimiter-run state) — relevant to the protocol's
  local-text vs inline-delimiter mutation families.

Fidelity boundary: we would NOT reproduce the tree-sitter grammar tables,
the GLR branch machinery, the C scanners, the section-node design, or the
injection-based two-grammar composition. Any horse borrowing these ideas is
`tree-sitter-markdown-inspired` under the R0 fidelity naming rule; the
retained-state *requirements* (not the encodings) are the transferable
claim.

## 15. Manifest entry

```toml
[[prior_art]]
id = "tree-sitter-markdown-grammar"
name = "tree-sitter-markdown block+inline grammars with state-retaining external scanners"
source_type = "upstream_source_pinned"
repo_url = "https://github.com/tree-sitter-grammars/tree-sitter-markdown"
tag = "v0.5.3"
commit_sha = "f969cd3ae3f9fbd4e43205431d0ae286014c05b5"
retrieved_date = "2026-09-16"
license = "MIT"
relevant_paths = [
  "tree-sitter-markdown/grammar.js",
  "tree-sitter-markdown/src/scanner.c",
  "tree-sitter-markdown/src/parser.c",
  "tree-sitter-markdown/src/tree_sitter/parser.h",
  "tree-sitter-markdown-inline/grammar.js",
  "tree-sitter-markdown-inline/src/scanner.c",
  "tree-sitter-markdown-inline/src/parser.c",
  "tree-sitter-markdown/queries/injections.scm",
  "common/common.js",
  "README.md",
]
notes = """
Two grammars: block (925 LR states, 47 external tokens, 48 distinct
external-scanner valid-token sets) and inline (1161 states, 15 external
tokens, 27 sets). Block scanner serializes 5 scalars (state flags,
matched, indentation, column mod-4 tab stop, fence delimiter length) plus
the open-blocks stack (18 Block kinds; list items encoded as
LIST_ITEM+content-indent 2..15; marker char NOT stored) — snapshot taken
after every external token and used by the engine as the reuse gate.
Inline scanner serializes 4 bytes (flags, code/latex delimiter lengths,
remaining delimiter-run count). No mutable global scanner state.
Line-synchronous matching at line starts = natural checkpoint density.
Issue-backed limitations: #243 serialize buffer overflow (open, 1024-byte
engine buffer, deep containers); #92 inline old-tree reparse lost emphasis
nodes, fixed by PR #134 stable-underscores (decision moved onto
valid_symbols probes); #257 CommonMark deviation in pinned v0.5.3; #184,
#242, #209, #202, #186, #114. Engine-level reuse/restart/convergence
mechanics are recorded in prior-art/tree-sitter.md (v0.27.0) and apply
unchanged here. Secondary: Context7 tree-sitter external-scanner docs.
No timing numbers; no benchmarks run.
"""
```
