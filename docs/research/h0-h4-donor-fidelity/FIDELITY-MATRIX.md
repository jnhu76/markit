# H0–H4 donor-fidelity matrix (#76 Gate A)

Status: **FROZEN GATE-A SYNTHESIS — VERDICT: FAIL / BLOCKED (H2, H3)**

Frozen: 2026-09-28, from four independent donor-first reviewer records
(R1–R4, `../reviews/gate-a-*-2026-09-28.md`) + pinned donor evidence
(`DONOR-SOURCES.md`) + challenge-case evidence (`CHALLENGE-CASES.md`).

| Horse | Donor lineage | D1–D12 summary | Fidelity verdict | Material deviations | Allowed performance claim |
|---|---|---|---|---|---|
| H0 | MD4C / pulldown-cmark / Comrak (lineage only; benchmark's own reference) | 12/12 MATCH or N/A-MATCH; no retained state; blocks-before-inlines; ref env rebuilt per call | **FAITHFUL_MECHANISM_MODEL** | none | "Our BENCH-GRAMMAR-v1 zero-reuse full-rebuild reference and correctness authority; donor-INSPIRED lineage only. No donor performance claims." |
| H1 | mizchi/markdown.mbt @ ffe7dc00 (primary) | faithful core: overlap damage, region reparse, prefix passthrough, suffix delta-shift reconstruction, definition TOTAL fallback, enumeration-only discovery; declared conservative guards F2–F6 + BlockFacts; UTF-8 coords; deeper suffix shift | **FAITHFUL_WITH_DECLARED_SIMPLIFICATION** | none unresolved (1×P2 doc-hygiene, 3×P3) | "mizchi-markdown-inspired block-local reparse with conservative soundness guards converting the donor's silent-divergence classes into counted fallbacks; NOT mizchi shipped-behavior reproduction; no mizchi product-performance claims." |
| H2 | @lezer/common 1.5.2 + @lezer/lr 1.4.8 + @lezer/markdown 1.6.3 | faithful fragment-table lifecycle, ContextKey vouching, per-line degradation cadence; **D3/D5**: no mid-container candidate discovery/granularity (donor takes item-level runs — 25/30 upstream vs 0 local) while frozen §7 claims it; **D11**: per-consult O(top-level-blocks) discovery scan vs donor amortized cursor (O(B²) worst case), undeclared | **MATERIAL_DEVIATION** | 2×P1 unresolved (candidate discovery + granularity; discovery-cost class) + 4×P2 + 3×P3 | Until corrected: only "a lezer-inspired, soundness-conservative fragment-table-gated whole-TOP-LEVEL-block reuse model" — container-axis results carry an anti-H2 bias and must be labeled; the frozen §7 "not a strict subset" claim is NOT deliverable today |
| H3 | tree-sitter v0.27.0 (primary) + ts-markdown v0.5.3 + W&G + swift-syntax (support) | faithful damage map, edit-path patching, entry-state gating, no-fallback posture, Arc/COW lifetime; **D5/D3**: damaged-ancestor interior descent is dead code (verified 4 ways) while docs claim it works — effective granularity = whole top-level entries only; **D11**: per-consult re-enumeration vs donor incremental cursor, invisible to counters | **MATERIAL_DEVIATION** | 2×P1 unresolved (interior descent + misleading claim boundary; enumeration-shaped discovery) + 3×P2 + 2×P3 | Until corrected: only "edit-flag damage map on a patched old tree + entry-state-gated reuse of maximal runs of unchanged TOP-LEVEL entries, with whole-damaged-entry reparse" — tree-sitter-INSPIRED, no Tree-sitter product-performance claims |
| H4 | W&G TOPLAS 1998 + swift-syntax 604.0.0 (composite anchors) + tree-sitter v0.27.0 + @lezer/markdown 1.6.3 (gating/guards) | defensible composite: W&G restart-before-damage + one-shot suffix-to-EOF, Swift checkpoint consult + pre-edit mapping (no byte-local predicate, no interleaving — declared), tree-sitter state-agreement content, Lezer boundary guards, model-defined definition-generation restart; deviations counted | **FAITHFUL_MECHANISM_MODEL** | none unresolved (3×P2 evidence/wording caveats, 2×P3) | "Our controlled restart/convergence model for the explicitly named composite; convergence authority is parser-state agreement at block boundaries — neither W&G nor Swift literal; no donor product-performance claims." |

## Gate-A decision

```text
GATE_A_VERDICT = FAIL / BLOCKED
BLOCKERS = H2 (2×P1), H3 (2×P1)
PASSING_HORSES = H0, H1, H4
GATE_B_AUTHORIZED = NO (Gate A did not pass)
PERFORMANCE_COLLECTION_AUTHORIZED = NO
PERFORMANCE_COLLECTION_RUN = NO
```

Pass rule (#76 §13 / task §39) requires: all donors pinned (yes), all
D1–D12 classified (yes), all challenge obligations evidenced or
NOT_FEASIBLE-justified (yes), claim boundaries explicit (yes — after the
corrections this matrix records), **no unresolved P0** (met: P0 = 0
across all lanes), **no unresolved P1** (NOT met: 4×P1 across H2/H3),
no unresolved MATERIAL_DEVIATION that would make the planned comparison
misleading (NOT met: H2 and H3 each carry donor-mechanism mismatches in
exactly the dimensions #76 names as blockers — candidate discovery,
reuse granularity, claim boundary).

Corrective decisions required (per horse, separately reviewed — none may
be silently repaired inside this evidence PR):

```text
H2: either restore live-level candidate discovery (mid-container runs)
    or re-freeze the claim boundary as "conservative strict-subset with
    stronger vouching" + declare the per-consult discovery scan.
H3: either restore functional interior descent + incremental cursor
    (or counter attribution) or freeze top-level-entry granularity as a
    declared simplification with its under-reuse quantified.
Any mechanism repair requires a separate PR and a fresh fidelity
re-review; the Gate-A verdict is then re-evaluated.
```

Honest-direction note (recorded by R2/R3, not a mitigation): the
unresolved H2 deviations bias results AGAINST H2 on container axes; the
H3 deviations bias against H3 on nested damage and discovery cost, and
in H3's favor on fragility-degraded composites. None of the horses was
inflated toward beating the donor — the failures are under-reuse and
undeclared cost-shape, not performance flattery. They still block
donor-level interpretation.

## Evidence inventory

- Reviewer records: `../reviews/gate-a-r{1..4}-*-2026-09-28.md` (verbatim)
- Donor manifest: `DONOR-SOURCES.md`
- Challenge cases + execution record: `CHALLENGE-CASES.md`
- Frozen donor-side models + probe/harness scripts + observed outputs:
  `challenge-evidence/{r1-h0-h1,r2-h2-lezer,r3-h3-treesitter,r4-h4}/`
- Clean-room donor clones (provenance): `jnhu@192.168.31.75:
  ~/research/markit-gate76/donors/` at the pinned commits above;
  Markit clean clone at `~/research/markit-gate76/markit` @
  `ad115bd12301a926aa1c3c19603f2a72fc1d183b`

---

## Corrective re-evaluation — 2026-09-28 (#79, #80)

The frozen Gate-A FAIL above is preserved as history. Both blockers were
repaired by their own corrective issues/PRs and re-reviewed by independent
fresh-context reviewers (neither an author of the repaired code):

| Horse | Corrective issue | Corrective PR (head) | Merge commit | Fresh review | New verdict | P0/P1 |
|---|---|---|---|---|---|---|
| H2 | **#79** | PR #82 (367060a) | **a3a551f** | RH2 — `../reviews/gate-a-rh2-h2-recheck-2026-09-28.md` | **FAITHFUL_MECHANISM_MODEL** | 0 / 0 |
| H3 | **#80** | PR #83 (532fd52) | **a8327fb** | RH3 — `../reviews/gate-a-rh3-h3-recheck-2026-09-28.md` | **FAITHFUL_WITH_DECLARED_SIMPLIFICATION** | 0 / 0 |

Both repairs took the mechanism-repair branch (no claim-boundary
downgrade): H2 gained live-level (mid-container) candidate discovery
(post-closure consult point, `starts_block` margin replacement,
persistent forward cursor) and H3 gained operative damaged-ancestor
interior descent plus attributed persistent-cursor discovery. Decisive
witnesses: H2 27/30 mid-container ListItem identity sharing (donor 25/30,
byte-identical re-run at the pinned npm versions); H3 interior-paragraph
identity sharing inside a flagged BlockQuote (donor F4/F4b reproduced by
rebuilding the frozen C harness at the pinned SHAs). Full per-horse
detail: the dated corrective sections in `H2-FIDELITY.md` /
`H3-FIDELITY.md`; challenge evidence: the corrective-cycle section of
`CHALLENGE-CASES.md`.

Gate-A status after the correctives (still governed by this frozen
record until re-evaluated):

```text
GATE_A_VERDICT            = FAIL / BLOCKED   (frozen above — unchanged)
CORRECTIVE_CYCLE          = COMPLETE (both P1 pairs repaired, merged,
                            fresh-reviewed; #79/#80 closed as completed)
GATE_A_RE_EVALUATION      = PENDING — requires a fresh synthesis reviewer
                            (R6: not R5/RH2/RH3, not any author) evaluating
                            from the merged master; only a documented R6
                            PASS may update GATE_A_VERDICT
GATE_B_AUTHORIZED         = NO
PERFORMANCE_COLLECTION_AUTHORIZED = NO
PERFORMANCE_COLLECTION_RUN       = NO
```

Residual (non-blocking) findings carried into the record: H2 P2×4 /
P3×5 and H3 P2-1 (fragility over-reuse, H3's favor) / P2-2 (first-
interior-block under-reuse, against H3; declaration completed in the R5
§8 addendum) / P3×3 — see the dated sections of the per-horse contracts.
H3-vs-donor comparisons must always state P2-1.

R5-review hygiene debt accepted in this cycle (all documentation-level,
non-blocking; recorded here so the debt is explicit):

- H1 accepted debt: the R5 §6 insertion-gap justification misdescribes
  the donor's `(i,i+1)` mechanics (P2-1), and the P3 set (no early break
  in the damage scan; deeper-suffix-shift declaration gap; stale
  `mechanisms/full-rebuild/README.md`) — implementation sound; accepted
  as documented debt in `H1-FIDELITY.md` "Open findings", not repaired
  in this cycle.
- H4 caveats: P2-1 (convergence clause (b) decisive role unwitnessed),
  P2-2 (clause (c) vacuous + R5 §9 semantic-dependency narrative
  misdescribes the implemented restart path), P2-3 (damage-detection
  linear scan divergence undeclared) and P3 wording items — accepted as
  documented caveats in `H4-FIDELITY.md` "Open findings"; any later H4
  comparison must restate them.
- Upstream probe outputs in `challenge-evidence/` are annotated (see the
  corrective-cycle section of `CHALLENGE-CASES.md`) so commands,
  observations, and interpretation are distinguishable without opening
  the scripts.
