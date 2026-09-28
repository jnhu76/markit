# H1 fidelity contract — BLOCK_LOCAL_REPARSE

Status: **FROZEN GATE-A FIDELITY CONTRACT (#76)**

```text
HORSE = H1 — BLOCK_LOCAL_REPARSE (mizchi-markdown-inspired)

LOCAL_MECHANISM_AUTHORITY =
  research/benchmarks/markdown-ast-update/mechanisms/block-local/
  (crate markit-mdbench-block-local)
  protocol/R5-HORSE-CORRECTNESS-PARITY.md §6 (frozen horse identity)

DONOR_SOURCES =
  primary donor:
    mizchi/markdown (`markdown.mbt`, MoonBit)
    github.com/mizchi/markdown.mbt
    ffe7dc00e60e6778e6407fb0c9bbfe9f202fea94 (master pin)
    src/incremental.mbt (entire mechanism), src/types.mbt,
    src/block_parser.mbt, src/api/exports.mbt, src/incremental_test.mbt
  supporting (non-authoritative):
    tree-sitter-markdown v0.5.3 f969cd3… (candidate entry-context dims)
    md4c / pulldown-cmark records (cross-line-state catalogs)
```

## D1–D12 summary (full detail in `../reviews/gate-a-r1-h0-h1-2026-09-28.md`, H1 half)

| Dim | Classification | Essence |
|---|---|---|
| D1 retained representation | DECLARED_SIMPLIFICATION | donor: whole old `Document` (top-level blocks + global UTF-16 spans, BlankLines first-class, definitions array); local: equivalent tiling (`Block{skel,sem,facts}` \| `Blank`) + retained `BlockFacts` (donor-absent, declared, not a search index) |
| D2 edit/damage propagation | DECLARED_SIMPLIFICATION | strict-overlap damage rule + region construction verbatim-faithful; boundary-insertion path deviates: local parses inserted-bytes-only where donor includes the left-adjacent entry (declared; justification sentence in freeze note misdescribes donor — P2-1) |
| D3 candidate discovery | MATCH | linear enumeration over retained tiling, no index/hash (donor identical); local omits donor's early break (P3, O(B) either way, honestly charged) |
| D4 eligibility/context | DECLARED_SIMPLIFICATION | donor validates NOTHING; local adds soundness guards F2–F6 (+F4(a), F6 fence checks) converting donor silent-wrong classes into counted TOTAL fallbacks (declared pre-implementation in R5 §6; conservative direction — worse for H1, required by the #22 oracle) |
| D5 reuse/reparse granularity | MATCH | top-level entry incl. blank runs; prefix moves unchanged (`nodes_reused`); suffix reconstructed with shifted spans (`nodes_rebuilt`, never reused) — the R2-corrected parser-work-reuse semantics exactly |
| D6 candidate rejection | DECLARED_SIMPLIFICATION | donor rejects nothing; local converts unsound localization to counted total fallback — never silent |
| D7 fallback/progress | MATCH (F1) + declared additions | definition-presence TOTAL fallback reproduced VERBATIM (both trigger directions; region-parse-before-check wasted work identical); guards F2–F6 declared additions |
| D8 coordinate/edit mapping | DECLARED adaptation | UTF-8 bytes replace UTF-16 (R0 §6); local shifts EVERY stored span (incl. inline subtrees + FencedCode.content) where donor shifts block-level only and leaves inline children stale — deeper shift is oracle-required, unfavorable-to-H1, declared in code docs (freeze surface thinner — P3-2) |
| D9 semantic/global dependency | MATCH | definition fallback is the donor's only global-semantic mechanism; reproduced; no reference-resolution pass substituted; no dependency index added |
| D10 retained-state maintenance | MATCH | eager O(suffix) suffix reconstruction (probe: 200/200 suffix nodes rebuilt on early edit) — donor's `shift_block_span` signature |
| D11 indexing vs enumeration | MATCH | enumeration only; MODEL_DEFINED_CANDIDATE_STATE (block index / def flag / context cache) explicitly kept out |
| D12 lifetime/retirement | MATCH | old state consumed by value at update; ownership transfer = disposal boundary; JS handle layer declared not reproduced |

```text
EXPERIMENT_SPECIFIC_ADAPTATION =
  BENCH-GRAMMAR-v1; common Source/Edit contract; normalized result contract;
  Rust substrate; UTF-8 byte coordinates; honest fallback counters
  (discarded region work charged, unlike donor's zeroed counters).

INTENTIONALLY_OMITTED =
  JS handle/source-retention layer; vendor SIMD tuning of the full parser;
  the donor's silent divergence classes (paragraph-merge on blank deletion,
  unclosed-fence forward swallow, attribute/alert attachment) — converted
  to counted fallbacks instead.

PERFORMANCE_RELEVANT_DEVIATIONS =
  1. guards F2–F6 + BlockFacts (declared): conservative; inflates H1's
     fallback-inclusive cost vs shipped donor; makes H1 oracle-sound.
  2. boundary-insertion region mapping (declared; justification
     misdescribes donor — P2-1): real divergence on the boundary-insertion
     edit class, both directions.
  3. deeper suffix shift incl. inline spans (declared in code docs): more
     per-suffix-node rebuild work than donor; oracle-required.
  4. no early break in damage scan (undeclared micro, P3): full-tiling
     enumeration always; O(B) either way; charged.
  5. fallback counter semantics (declared): charges discarded region work.

CHALLENGE_CASES =
  H1-F1 valid-seam local block edit → local reparse only — PASS-MECHANISM
  H1-F2 boundary/seam invalidation → expansion/rejection — PASS-MECHANISM
    (declared donor-deviation: silent-wrong → counted fallback)
  H1-F3 suffix shift; parser-work vs object reuse — PASS-MECHANISM
  H1-F4 definition/reference edit → TOTAL fallback both directions —
    PASS-MECHANISM (verbatim F1)
  H1-F5 many retained blocks → enumeration discovery, no index —
    PASS-MECHANISM

UPSTREAM_QUALITATIVE_CROSS_CHECK =
  PARTIAL — MoonBit toolchain unavailable (execution NOT_FEASIBLE);
  full source-level cross-check at the pinned SHA instead: every
  load-bearing file:line citation of the R2 record re-verified against
  src/incremental.mbt read in its entirety (458 lines).

CLAIM_BOUNDARY =
  "mizchi-markdown-inspired (pinned ffe7dc00) block-local reparse: the
  donor's strict-overlap damage rule, region construction, clean region
  reparse, prefix pass-through, suffix delta-shift value reconstruction,
  and definition-presence TOTAL fallback are reproduced faithfully;
  UTF-8 byte coordinates replace UTF-16; PLUS conservative soundness
  guards (F2–F6, BlockFacts) converting the donor's silent-divergence
  classes and boundary-join insertions into counted total fallbacks.
  H1 is NOT a reproduction of mizchi's shipped update behavior (which
  would emit WRONG_RESULTs under the benchmark oracle), and H1 results
  are not mizchi product-performance measurements." — matches what the
  crate header, README, and frozen R5 §6 actually state.

FIDELITY_VERDICT =
  FAITHFUL_WITH_DECLARED_SIMPLIFICATION
```

Open findings (non-blocking for Gate A, tracked for follow-up):

- P2-1: the frozen R5 §6 insertion-gap justification misdescribes the
  donor's `(i,i+1)` mechanics (it includes the LEFT-adjacent entry, not
  "the following block") and leaves the true behavioral difference
  unstated. Documentation repair; implementation sound and declared.
- P3: no early break in damage scan; deeper-suffix-shift declaration gap
  in R5 §6 surface; stale `mechanisms/full-rebuild/README.md`.
