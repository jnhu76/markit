# parser-survey — final report (issue #19 campaign close)

MARKIT-19-PARSER-MECHANISM-COMPARISON
Branch: `exp/19-parser-survey-1`. Base: master (`d7837fc`), run-1:
`1c21b9a`, then run-1.1 / ORACLE-B / RUN-2a–c / RUN-3 / RUN-4 evidence
commits. Machine: WSL2 x64, rustc 1.97.1 release (per-run `meta.txt`).

Evidence chain (all committed, per-run summaries in `results/summary/`):

| run | report | product code touched |
|---|---|---|
| RUN-1 (prior) | parser-survey-1-run1.md | none |
| RUN-1.1 corrective | parser-survey-1-run1-corrective.md | none |
| ORACLE-B CommonMark | parser-survey-oracle-b-commonmark.md | none |
| RUN-2a MD4C | parser-survey-2-baselines-md4c.md | none |
| RUN-2b tree-sitter | parser-survey-2-baselines-treesitter.md | none |
| RUN-2c Lezer | parser-survey-2-baselines-lezer.md | none |
| RUN-3 green tree | parser-survey-3-representation.md | none |
| RUN-4 reference index | parser-survey-4-semantic-deps.md | none |

**Zero markit-core changes across the whole campaign.** All prototypes
and adapters live in `crates/parser-survey` (research-only).

---

## Required questions (plan §38)

**Q1 — syntax radius distribution of ordinary edits.** 397 scenarios
(run-1.1): S0_NO_REPARSE ×6, S2_BLOCK_LOCAL ×130, S3_NEIGHBOR_LOCAL
×179, S4_CONTAINER_SCOPED ×30, S5_STATE_PROPAGATING ×46,
S6_DOCUMENT_REPARSE ×6, S1 unobservable in flat L1 ×0. Ordinary content
edits: one block, size-independent R≈24–154 at 1 MB; boundary edits
spill to ≤4 blocks; fence opener/closer corruption propagates to EOF.

**Q2 — what is syntax vs semantic vs representation.** The axes are
separable and were separated: syntax = reparse radius (S-axis);
semantic = dependents whose resolution changed (D-axis, measured in
RUN-4: 10 k dependents invalidated while 2–3 blocks reparsed);
representation = coordinate/record maintenance (M-axis: run-1.1's
M4/M5 + the 9.0 ns/record fit). The run-1 "hidden O(N)" is
*representation maintenance*, not syntax propagation. The only true
syntax propagation in the block grammar is forward fence state (S5).

**Q3 — root cause of the hidden O(N), with scalings.** Three terms,
all measured: (1) absolute-coordinate rewrites of survivor records —
dominant, inc_us ≈ 0.37 µs + 9.0 ns × survivors (R² = 0.971, 48 rows;
three anchors: equal-length 2.4 µs / mid 10.2 k records 59.5 µs / BOF
20.5 k records 136.9 µs at 1 MB); (2) record-sequence movement (Vec
tail relocations on block-count change, `block_records_moved`); (3)
O(B) block examination in boundary ops (`blank_delete@BOF@1MB`
examines 13 658 blocks with ZERO blocks reparsed — 215 µs of pure
metadata work). Resolved by prototype: yes — RUN-3 G2 (below).

**Q4 — does position-free green remove local-parse+global-metadata?**
Yes, with scope. G2 (weight-balanced persistent block sequence):
edit cost flat in N — **0.18 µs at 1 MB, 13–14 ancestors path-copied,
coordinate rewrites = 0 by construction** (vs markit 132.9 µs @BOF).
G1 (naive Vec) proves the plan-§15 warning: without balance you still
pay O(B) (38–40 µs). Costs: memory ≈ 1.05× document bytes at
per-block granularity; red-view position query 130 ns @1 MB ≈ the
product's own 194 ns. Not prototyped: multi-edit transactions,
inline-IR placement inside green nodes.

**Q5 — Markdown-specific boundary convergence vs generic subtree
reuse.** Lezer's per-block-hash fragments: incremental cost nearly
position-independent at 1 MB (bof 3.3 / mid 4.5 / eof 3.7 ms vs its
77.6 ms full) and survives all adversarial docs. Tree-sitter
(grammar 0.7.1, generic reuse): changed coverage = suffix fraction
(49 % for a mid 1-byte insert), incremental cost scales with N
(1.47 ms @100k → 17.1 ms @1m), and **fatally aborts on the 200-deep
quote**. markit's own convergence stays the tightest (µs-scale), with
the representation caveat that RUN-3 removes. H5 has direct, measured
support; confidence moderated by cross-runtime rules and pinned
grammar versions.

**Q6 — keep flat blocks? which syntax needs finer trees?** Keep the
flat block stream as the *semantic layer* for ordinary paragraphs
(best convergence in the survey). Finer/nested structure is justified
where run-1 measured block-granularity floors: huge paragraphs
(R ≈ 100 003) → chunked leaves (§21: representation is cheap either
way — the win is parse-scope, bought with ~1 600 nodes/100 KB);
containers (deep quote R ≈ 42 093) → nested structure (O(depth)
representation cost, 17.7 µs @200 levels, bounds future parse scope);
fence forward state stays a block-level mechanism (S5).

**Q7 — is the propagation mechanism set small and enumerable?** Yes —
refined. L1 suggested "fences only"; the full-campaign set is:
(1) local block replace; (2) boundary neighbor repair (split/merge/
convert); (3) forward fence state; (4) container nesting (L1 flattens
it; full grammar needs it — RUN-3); (5) document reinterpretation
(rare, S6); (6) semantic dependency classes (reference labels —
definitions/users; RUN-4); (7) provider delegation (mermaid — flagged
MERMAID_CANDIDATE, unmeasured). Seven enumerable mechanisms, none
inferable from L1 alone — the plan's warning against extrapolating
from L1 was correct.

**Q8 — can a semantic index keep global meaning changes locally
parsed?** Yes (RUN-4). Definition edits: syntax 2–3 blocks / 1–5 µs
while 100–10 000 dependents invalidate; losing-duplicate edits proven
semantic no-ops (0 changed); index self-equivalence oracle passes;
delta ∝ dependents and ~10× under full rebuild at 10 k.

**Q9 — stable full/incremental crossover predictor?** Crossover is
real and reproduced cross-implementation: markit fence cascade
(incremental 2.0 ms > full 1.2 ms @1 MB); Lezer fence cascade
(1.57×); huge-block shapes near 1.0× (Lezer 1.04×, TS 0.66×).
Candidate predictor: *expected-reparse-share* (forward-state flips and
damage ≈ document scale) + block-count delta magnitude. **Insufficient
evidence for a validated threshold** — two implementations, two shapes
only; no adaptive threshold is proposed (plan §29).

**Q10 — ADOPT / ADAPT / HYBRID / BUILD?** See verdict below: HYBRID.

---

## Hypothesis scoreboard (plan §39)

| H | verdict | evidence |
|---|---|---|
| H1 local convergence | **SUPPORTED** | 397 scenarios: radius independent of N for S0–S3; ZERO_REPARSE_CONVERGENCE reproduced |
| H2 limited propagation mechanisms | **SUPPORTED_WITH_SCOPE** | L1: fences only; full grammar adds container nesting + semantic deps — still enumerable (7) |
| H3 syntax/semantic separation | **SUPPORTED** | RUN-4: syntax blind to 10 k-dependents; delta ∝ dependents; no-op proofs |
| H4 position-free representation | **SUPPORTED_WITH_SCOPE** | RUN-3 G2: flat-in-N edits, coord_rw=0; memory ≈1.05× doc; multi-edit + inline-IR open |
| H5 Markdown-specific boundary advantage | **SUPPORTED_WITH_SCOPE** | RUN-2b vs 2c: position-independent inc + robustness (Lezer) vs suffix-scale cov + fatal abort (TS) |
| H6 full/incremental crossover | **SUPPORTED** (crossover exists); predictor **INSUFFICIENT_EVIDENCE** | markit I4 + Lezer 1.57× at state-flip shapes |

---

## Research question verdict (plan §40)

**REFINE** (the four-axis split is confirmed; the refinement is that
the axes are *independent mechanisms with separate data structures*,
and representation minimality must be judged against a coherence
contract):

> For arbitrary lossless Markdown edits, syntax propagation, semantic
> dependency propagation, representation maintenance, and downstream
> invalidation are separable cost axes: each can be served by a
> dedicated mechanism whose work is proportional to its own observed
> quantity (reparse radius; dependents; structure path; visible
> invalidation), provided results are published coherently and a
> full-parse fallback is available when measured damage crosses a
> validated predictor.

Forced by: RUN-1.1 (axes independent, ZERO_REPARSE_CONVERGENCE),
RUN-4 (semantic fanout separable and indexable), RUN-3 (representation
minimum is structure-path cost, not zero), RUN-2c (crossover clause).

---

## Parser mechanism verdict (plan §41)

**HYBRID** — architecture candidate, mechanism-level:

1. **Markdown-specific block/container restart + convergence** (keep
   P0-02's algorithmic core — tightest measured local convergence);
2. **position-free persistent green representation** (G2-class
   balanced sequence; red view for position queries; chunked leaves
   for huge blocks; nested containers where parse scope demands);
3. **separate semantic dependency indexes** (ReferenceIndex pattern:
   defs/users per block, first-wins, self-equivalence oracle);
4. **full-parse fallback when measured damage crosses a validated
   predictor** — predictor NOT validated; MD4C-class full parse
   measured at 1.1–1.2 ns/B as the reference point.

Not a ranking of implementations (plan §35); each baseline answers its
own question: MD4C = full-parse cost floor; tree-sitter = generic
incremental engine behavior (suffix-scale reuse granularity, N-scaled
incremental cost); Lezer = Markdown-specific fragment reuse.

---

## Major limitations

- Single machine (WSL2), single runs per configuration (medians over
  5–9 iters); allocator counting active for all latency rows (support
  evidence only, noted per run).
- Cross-implementation wall-clock not compared across runtimes (Lezer
  in JS); reused Lezer/TS evidence is ratio- and shape-based.
- tree-sitter runtime 0.19.5 + grammar 0.7.1 and @lezer/markdown
  1.7.2 are pinned published versions; newer runtimes may differ.
- ORACLE-B expected side is a targeted HTML scanner (edge cases pinned
  by the corpus; both implementations judged by identical rules).
- Semantic layer is an experimental subset (whole-paragraph defs;
  container defs unsupported).
- P_provider (mermaid/footnote-class invalidation) flagged, never
  measured. Phase timers never added (structural counters stayed
  primary; plan §4).

## Unresolved questions

1. Multi-edit transactions (plan §32: 2/4/16 distant edits — island
   independence, delta-map cost) — not measured this campaign.
2. Multi-edit + green tree interaction (batch path-copy amortization).
3. Inline-IR placement inside green nodes (inline reruns vs inline
   chunks) and its memory/query costs.
4. Crossover predictor validation beyond fence/huge-block shapes.
5. Depth-seeded mutation matrix (plan §30) to remove the
   header-anchor geometry bias observed in run-1.1.
6. GPUI/presentation-side costs (out of scope for this campaign;
   core-only evidence).

---

VERDICT: **READY_FOR_MARKIT_MARKDOWN_ARCHITECTURE_REVIEW**

(No merge performed; no production parser begun; product docs
untouched. Campaign stops here per plan §42.)
