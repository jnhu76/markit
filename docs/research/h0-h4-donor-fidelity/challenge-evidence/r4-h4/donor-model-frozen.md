# GATE-A R4 — FROZEN donor-side D1–D12 model for H4 (written BEFORE reading the horse crate)

Repo authority for the composite (MECHANISM-SOURCE-MAP.md H4 + MECHANISM-MATRIX M8/M7/M5/M4):
H4 = W&G restart/validate/converge SHAPE + Swift checkpoint/forward-parse mechanics +
tree-sitter state-agreement gating + lezer-markdown block-boundary guards. The four
convergence authorities are DIFFERENT and must stay distinct:

- W&G (paper): batch-parser equivalence at theorem matchpoints; suffix-as-INPUT via old-tree
  traversal; validation = subsequent shift of a non-epsilon subtree within k terminals;
  rollback = delayed right_breakdown (bounded backtracking O(kV)). Restart = root path as
  initial stack; no checkpoint table; restart work ~ O(lg N) boundary accesses per reused
  subtree (balanced-sequence precondition).
- Swift (repo 050f1a3): restart = ALWAYS file start; checkpoints = EVERY
  CodeBlockItem/MemberBlockItem start; predicate = byte-local (translated pre-edit position
  equality + expected kind + no edit intersectsOrTouches [pos, pos+lookahead)); reuse/damage
  INTERLEAVED per item; NO one-time converge-and-splice; no parser-state equality; offsets
  DERIVED, never patched; re-registration at every item (self-contained for next edit);
  degradation continuous (predicate failure = fresh parse, no mode switch).
- tree-sitter (repo 6070dbf): position 0 restart with forward-only pre-order old-tree cursor;
  suffix/subtree reuse requires STATE AGREEMENT: byte-position alignment + external-scanner
  entry-state equality + !changed/!error/!missing/!fragile + first-leaf lex-mode/action
  compatibility; composite candidates narrowed to parse_state == current state
  (breakdown_lookahead); splice state = grammar transition, NEVER copied; backdown =
  breakdown_top_of_stack; reuse suppressed while version_count > 1 (interval-shaped).
- lezer-markdown (repo 9942d7c): convergence vouching = composite-block rolling-hash
  equality of the ENTIRE enclosing stack; taken run must END AT BLOCK BOUNDARY; never end
  inside NotLast blocks (CodeBlock, ListItem, OrderedList, BulletList — continuable across
  blank lines); trailing partial line excluded (fragmentEnd = last full line, -1 more if
  openEnd); per-line retry; context rebuilt live by parsing (never restored).

D1 retained representation (composite expectation):
  W&G: versioned production-labeled parse tree + change marks; NO parser state in nodes.
  Swift: previous tree + LookaheadRanges (bytes looked ahead per item id) + ConcurrentEdits.
  TS: edited old tree (offsets patched, has_changes flags) + per-subtree parse_state/first_leaf/
      external-scanner state + ReusableNode cursor.
  Lezer-md: old trees + fragment table (updated-doc coords, offset deltas, open edges) +
      per-node contextHash/lookAhead props + markdown FragmentCursor.
D2 edit/damage propagation:
  W&G: version-diff change marks split tree at modification sites (split points sized by k).
  Swift: NONE upfront — lazy per-item predicate against translated positions/affect ranges.
  TS: ts_tree_edit patches edited path + ancestors; flags ARE the damage map.
  Lezer: host ChangedRanges -> applyChanges splits/drops/trims fragments (minGap=128), open edges.
D3 candidate discovery:
  W&G: sentential-form shift test at reductions + old-suffix subtree stream (grammar-driven).
  Swift: monotone in-order SyntaxCursor walk; consult at EVERY fixed item-kind checkpoint.
  TS: forward-only pre-order ReusableNode cursor at position alignment.
  Lezer-md: per-line FragmentCursor.moveTo at line starts.
D4 eligibility/continuation/state compatibility:
  W&G: exact nonterminal-shift test (necessary+sufficient) in current parse state.
  Swift: pre-edit-position equality + expected-kind equality + untouched affect range.
  TS: entry-state equality + flag exclusions + first-leaf lex/action compatibility +
      composite parse_state == current state.
  Lezer-md: contextHash equality of composite stack + block-boundary/NotLast/partial-line guards.
D5 reuse/reparse granularity:
  W&G: whole grammar subtrees (balanced sequences; lg N boundary cost).
  Swift: whole CodeBlockItem/MemberBlockItem items ONLY (incl. trivia).
  TS: any non-root subtree.
  Lezer-md: runs of whole top-level block subtrees per line-step.
D6 candidate rejection:
  W&G: non-shiftable -> decompose one level (left/right_breakdown); speculative rollback.
  Swift: predicate failure -> fresh parse of that item; cursor stays; later items may reuse.
  TS: descend into children or advance; breakdown_top_of_stack on invalid lookahead.
  Lezer-md: fall through to normal block parse of the line; retry next line.
D7 fallback/progress:
  W&G: batch parse only when no reference version; errors NOT a fallback trigger; fragile
       regions recreated; bounded backtracking within validation.
  Swift: NO explicit fallback; continuous degradation (clean parse + lookup overhead).
  TS: no mid-file full-rebuild fallback; error/missing/fragile permanently excluded;
      reuse suppression interval while GLR versions > 1.
  Lezer-md: fragments=[] => full parse; per-line fallback is the mechanism; small-parse gate
       (bufferLength*4) disables fragments.
D8 coordinate/edit mapping:
  W&G: versioned queries + <token,offset> pairs; approximated yields.
  Swift: translateToPreEditPosition (post->pre; nil inside inserted region); offsets derived.
  TS: caller-supplied TSInputEdit patches old tree (bytes + points); new tree fresh offsets.
  Lezer: fragment offset deltas applied by applyChanges; trees untouched.
D9 semantic/global dependency:
  W&G: N/A (parse-tree only; annotations preserved on reused nodes).
  Swift: none (byte-local; context risk documented Q10).
  TS: external-scanner hidden state equality gate.
  Lezer-md: reference definitions NOT validated (no global pass).
D10 retained-state maintenance:
  W&G: versioning/history services; 2 bits/node if absent.
  Swift: re-register affect range at EVERY item (fresh or reused) -> self-contained.
  TS: old tree refcounted; edit walk maintains extents.
  Lezer: fragment table maintenance only (O(fragments x changes)).
D11 candidate/search indexing vs enumeration:
  W&G: grammar-driven stream (no index).
  Swift: cursor walk, NOT hash lookup (record Q: implementation-specific choice).
  TS: pre-order cursor (no index).
  Lezer: fragment array + TreeCursor (no index).
D12 lifetime/retirement:
  W&G: versioned storage retains old values until parse completes.
  Swift: old tree kept alive by transition; arena sharing; refcount.
  TS: old tree released at parse end (ts_parser_reset); refcounted sharing.
  Lezer: old trees retained by fragments; replaced by addTree.

FROZEN donor-side expectations for challenge cases:

H4-F1 (valid nearby restart + valid nearby convergence):
  Composite expectation: restart chosen at/before damage with valid context (W&G root-path
  analogue; Swift has NO restart search — restart=0 always; lezer per-line; TS position 0);
  forward parse from restart; convergence accepted when the appropriate authority holds
  (state/key agreement at a block/item boundary, damage region behind, boundary guards
  Lezer-style); suffix reused WITHOUT reparsing it (W&G suffix-as-input; TS one-splice state
  agreement; Swift skip-by-lexer-advance). Observable: restart distance < edit position,
  convergence position > damaged end, suffix blocks not reparsed, result == clean parse.

H4-F2 (restart support damaged -> reject / earlier restart):
  Composite expectation: if the checkpoint/context backing the restart is itself damaged,
  the mechanism must fall back to an earlier/stronger restart (W&G: farther-back split /
  more breakdown; Swift: N/A — no restart choice, item reparsed fresh; TS: descend/past
  rejection; lezer: moveTo fails -> parse line normally). NO reuse of a damaged checkpoint.
  Observable: restart moves earlier or to 0; no false reuse.

H4-F3 (candidate before required damaged region crossed -> reject):
  Composite expectation: a convergence/reuse candidate whose position is not beyond the
  damage must be rejected (Swift: affect-range edit intersection; TS: has_changes exclusion +
  position alignment; lezer: fragment trimmed at change edges/open edges; W&G: validation
  fails -> rollback). NO convergence strictly inside/before the damaged region's end.
  Observable: convergence only at position > damaged end (or mapped pre-edit position
  untouched by edits).

H4-F4 (no interior convergence -> correct progress / EOF path):
  Composite expectation: if no candidate passes, the forward parse simply runs to EOF
  (Swift continuous degradation; TS normal lexer path; lezer per-line fallback; W&G batch
  equivalence still holds). Correctness must not depend on convergence occurring.
  Observable: convergence distance = EOF-ish, blocks rebuilt, result still == clean parse;
  no fallback-to-full "mode" required by the composite (Swift/TS semantics), though a
  definition/semantic fallback may exist for Markdown specifics (D9).

H4-F5 (state/context incompatibility -> no false convergence):
  Composite expectation: matching BYTES/POSITION alone must not converge when the parser
  state / composite context differs (TS parse_state/scanner-state equality; lezer contextHash
  equality; Swift expected-kind byte-local predicate is the WEAK authority — repo explicitly
  flags byte-local as hypothesis-level for Markdown). Observable: context key mismatch at
  same position => rejection; no false convergence; e.g. same text at same offset under a
  different container stack must not splice.

Claim boundary to enforce: local H4 must NOT be documented as W&G-faithful or Swift-faithful
if it does one-time converge-and-splice or tree-comparison; repo authority pre-declares H4
as multi-source mechanism abstraction (benchmark-model-defined convergence predicate is
ALLOWED if documented). Correct output != fidelity.
