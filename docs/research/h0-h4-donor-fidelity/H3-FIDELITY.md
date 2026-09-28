# H3 fidelity contract — OLD_TREE_SUBTREE_REUSE

Status: **FROZEN GATE-A FIDELITY CONTRACT (#76) — VERDICT: MATERIAL_DEVIATION**

```text
HORSE = H3 — OLD_TREE_SUBTREE_REUSE (tree-sitter-inspired)

LOCAL_MECHANISM_AUTHORITY =
  research/benchmarks/markdown-ast-update/mechanisms/old-tree-subtree-reuse/
  (crate markit-mdbench-old-tree-subtree-reuse)
  protocol/R5-HORSE-CORRECTNESS-PARITY.md §8 (+§8 impl notes, §12 amendments)

DONOR_SOURCES =
  primary anchor:
    tree-sitter core v0.27.0 6070dbfefd326bd735e5683eb128cc1b57dad0c0
    lib/src/{parser.c,reusable_node.h,subtree.c,subtree.h,tree.c,
    get_changed_ranges.c}, lib/include/tree_sitter/api.h
  supporting:
    tree-sitter-markdown v0.5.3 f969cd3… (scanner entry-state dimensions)
    Wagner & Graham dissertation Ch. 6 (concept ancestor)
    swift-syntax 604.0.0 050f1a34… (overlap variant)
```

## D1–D12 summary (full detail in `../reviews/gate-a-r3-h3-2026-09-28.md`)

| Dim | Classification | Essence |
|---|---|---|
| D1 retained representation | MATCH (adapted) | TTree top-level {gap, Arc<TNode>}, sizes, relative children, entry ContextKey, changed flag — donor shape adapted to the semantic core; no stored absolutes (both derive) |
| D2 edit/damage propagation | MATCH / NON_MATERIAL_ADAPTATION | patch only the intersecting ancestry, size absorbs delta, mark changed — flags-are-damage-map; continuation-margin neighbor marking = Markdown analogue of donor's lookahead-reaching/column invalidation (declared); no row/col (declared, R0 §6) |
| D3 candidate discovery | **MATERIAL_DEVIATION** | donor: single incremental forward pre-order ReusableNode cursor (O(1)-amortized consults); local: stateless per-consult search that rebuilds the full top-level list and linear-searches from index 0 at EVERY line-start consult; interior descent exists in code but cannot take |
| D4 eligibility/state compatibility | DECLARED_SIMPLIFICATION + NON_MATERIAL_ADAPTATION | line-alignment position gate + ContextKey equality (faithful line-start projection of ts-markdown scanner state; slightly stricter on markers); no lex-mode/parse_state narrowing or backdown — declared (no LR tables; total deterministic grammar); run extension batches soundly |
| D5 reuse granularity | **MATERIAL_DEVIATION** | donor reuses any non-root subtree INCLUDING interior descendants of damaged ancestors (verified upstream: list_item reuse inside damaged list, word tokens inside damaged quote); local effective granularity = whole top-level entries only — descent provably dead (ctx frame mismatch at first-line consults; interior lines never align at top level), verified four independent ways, while freeze §8/README/lib docs/h3_gate comments claim it works |
| D6 candidate rejection | DECLARED_SIMPLIFICATION / NON_MATERIAL_ADAPTATION | descend-or-reparse outcome matches; error/missing/fragile exclusions absent (grammar is total/deterministic — declared); consequence: horse takes whole blocks where donor's fragility degrades to token-level reuse — over-reuse in H3's favor |
| D7 fallback/progress | MATCH | natural degradation; no mid-file full reparse in either |
| D8 coordinate/edit mapping | DECLARED_SIMPLIFICATION | byte-only; prefix-sum derivation; clamping + stale-position refusal guards (conservative) |
| D9 semantic/global dependency | NON_MATERIAL_ADAPTATION (declared) | ContextKey = scanner-entry-state gate; horse adds reference-environment clause + rematerialization (donor resolves links in a separate inline grammar) — declared with honest node accounting |
| D10 retained-state maintenance | MATCH | flags persist in patched tree; new tree unflagged |
| D11 indexing vs enumeration | **MATERIAL_DEVIATION** | no invented index (good), but per-consult top-level re-enumeration (O(#entries) + n Arc clones + allocation per consult) where donor cursor avoids it; cost invisible to the horse's counters — attribution blind spot |
| D12 lifetime/retirement | MATCH (mechanism level) | Arc sharing ↔ refcount; COW patch ↔ make_mut; fresh wrappers/root |

```text
EXPERIMENT_SPECIFIC_ADAPTATION =
  BENCH-GRAMMAR-v1 (total deterministic block grammar — no GLR, no error
  nodes, no fragile flags); common Source/Edit contract; normalized
  result contract; reference-environment clause + rematerialization.

INTENTIONALLY_OMITTED =
  LR parse tables, GLR error-recovery policy, C runtime / inline-subtree
  encoding / subtree pools / allocators, query engine, included-range
  injection, repeat-chain balancing, progress/cancellation machinery,
  ts_tree_get_changed_ranges consumer diff (oracle normalize==H0
  replaces it).

PERFORMANCE_RELEVANT_DEVIATIONS =
  P1-class (see corrective issue):
    1. interior-of-damaged-entry reuse impossible (D5/D3) — rebuild work
       grows with the damaged top-level entry's full extent where donor
       reuses the undamaged interior; penalizes H3 on container-heavy
       workloads; docs claim otherwise.
    2. per-consult re-enumeration of the top-level entry list (D3/D11) —
       O(consults × entries) worst shape; invisible to the horse's own
       work counters; undeclared.
  P2-class:
    3. no fragile/error/missing rejection analogues — coarser-but-wider
       reuse than observed donor behavior on this grammar (H3's favor;
       declared via no-GLR boundary)
    4. run extension does not re-gate per extended member (sound,
       outcome-equivalent; undeclared detail)
    5. reference-environment rematerialization + per-consult margin
       source reads are horse-added work (declared, honestly counted)
  P3-class: h3_gate "nested quote edit" probe does not test what its
  comment says; "forward-only cursor" wording suggests state the
  implementation does not have.

CHALLENGE_CASES =
  H3-F1 unchanged eligible subtree + compatible state → reuse — MATCH
  H3-F2 unchanged bytes + incompatible state (fence opened) → no reuse —
    MATCH (donor-executed: 0 reuse events, full changed coverage)
  H3-F3 edit overlaps candidate → invalidation — MATCH
  H3-F4 damaged ancestor + reusable descendant — **MATERIAL_DEVIATION**
    (donor: interior reuse observed, 60 reuse events inside damaged
    quote + reused list_items; local: entire damaged entry rebuilt,
    no interior reuse)
  H3-F5 many top-level candidates — MATCH on outcome; discovery work
    shape differs per D3/D11
  Evidence: upstream qualitative cross-check COMPLETE — tree-sitter
  v0.27.0 core + tree-sitter-markdown v0.5.3 compiled into the C harness
  (challenge-evidence/r3-h3-treesitter/gate_h3.c, gcc -O0): reuse-event
  log, ts_tree_get_changed_ranges, incremental-vs-clean self-equivalence;
  NO timing. Local lane: h3_gate etc. all green + scratch probe
  (counters, Arc identity, ctx dumps).

UPSTREAM_QUALITATIVE_CROSS_CHECK =
  COMPLETE (executable donor lane at pinned commits).

CLAIM_BOUNDARY =
  DEFENSIBLE TODAY: "an edit-flag damage map on a patched old tree +
  entry-state-gated reuse of maximal runs of unchanged TOP-LEVEL entries,
  with whole-damaged-entry reparse" — tree-sitter-INSPIRED, results are
  not Tree-sitter product-performance claims.
  NOT DEFENSIBLE AS WRITTEN: frozen §8 / README / lib.rs header /
  h3_gate.rs:505-507 claims of donor-style "rejected composite candidates
  descend (children)" nested reuse — the mechanism cannot do this
  (verified four ways) and the cited fixture does not exercise it.
  Before any H3/Horse-A comparative claim: either restore functional
  interior descent, or freeze top-level-entry granularity as a declared
  simplification with its under-reuse quantified; and either adopt an
  incremental cursor or attribute the per-consult enumeration in counters.

FIDELITY_VERDICT =
  MATERIAL_DEVIATION
  (correctness unaffected: H3 == H0 held in every probe and suite run;
  the deviation is mechanism-level: the donor's defining damaged-ancestor
  interior navigation has no working local counterpart, and candidate
  discovery is enumeration-shaped)
```

Gate-A consequence: H3 is a Gate-A BLOCKER (candidate discovery + reuse
granularity + misleading claim boundary). Corrective issue: see
FIDELITY-MATRIX.md and the #76 status record. Repair requires a separate
PR + fresh fidelity re-review (not this PR).
