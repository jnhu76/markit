# parser-survey — final report (issue #19 campaign close; CORRECTIVE-1 applied)

MARKIT-19-PARSER-MECHANISM-COMPARISON
Branch: `exp/19-parser-survey-1`. Base: master (`d7837fc`), run-1:
`1c21b9a`, then run-1.1 / ORACLE-B / RUN-2a–c / RUN-3 / RUN-4 evidence
commits, then the MARKIT-19-CORRECTIVE-1 commits (ts fix `616014f`,
history gate `29dcbdf`, reference index `267c6d2`). Machine: WSL2 x64,
rustc 1.97.1 release (per-run `meta.txt`).

Evidence chain (all committed, per-run summaries in `results/summary/`):

| run | report | product code touched |
|---|---|---|
| RUN-1 (prior) | parser-survey-1-run1.md | none |
| RUN-1.1 corrective | parser-survey-1-run1-corrective.md | none |
| ORACLE-B CommonMark | parser-survey-oracle-b-commonmark.md | none |
| RUN-2a MD4C | parser-survey-2-baselines-md4c.md | none |
| RUN-2b tree-sitter | parser-survey-2-baselines-treesitter.md (**erratum applied**) | none |
| RUN-2c Lezer | parser-survey-2-baselines-lezer.md | none |
| RUN-3 green tree | parser-survey-3-representation.md (**extended with history gate**) | none |
| RUN-4 reference index | parser-survey-4-semantic-deps.md (**reworked**) | none |
| CORRECTIVE-1 A–D | this report + the three above | none |

**Zero markit-core changes across the whole campaign including
CORRECTIVE-1.** All prototypes and adapters live in
`crates/parser-survey` (research-only).

---

## CORRECTIVE-1 (adversarial review outcomes, all resolved)

1. **tree-sitter `changed_ranges` measured the wrong object.** The
   adapter compared the UNEDITED old tree against the new tree; the
   tree-sitter contract requires the `edit()`-adjusted clone. The first
   run's coverage column (BOF 99.3 % / mid 49.5 % / EOF 0 %) was pure
   coordinate-offset artifact. Corrected rerun: ordinary-edit coverage
   0.00–0.15 % at every position; fence cascade 99.63 % (real global
   damage) and the 50-deep quote chain 50.6 % unchanged; timing
   reproduced within noise; the 200-deep abort unchanged. **Retracted:**
   "ts changed coverage = suffix fraction", "suffix-scale reuse
   granularity", and the generalization "suffix-cost is a property of
   position-carrying representations".
2. **G2 was not a proven weight-balanced sequence.** Updates are pure
   path-copy with no rebalance, and run-3 measured single edits from a
   balanced root. The G2-HISTORY-STABILITY gate now supplies the
   missing evidence: naive path-copy degrades under structural churn
   (height 11 → 128 after 100 k hotspot-heavy edits, 10.7× balanced;
   reads and memory degrade with it); a height-triggered rebuild
   policy bounds it (≤ budget, amortized 35 ns/edit, worst pause
   0.22 ms); repeated truncation alone does NOT degrade height.
   Concrete balancing strategy remains open/unearned.
3. **ReferenceIndex conflated stable identity with document order**
   (candidates stored `(id, id, url)`, resolution = min id). Reworked:
   candidates store `(BlockId, url)`; resolution consults a `DocOrder`
   provider from the syntax layer. Six winner-mutation gates pass,
   including the review's counterexample (a new def with a LARGER id
   inserted before the winner now correctly wins) and the decisive
   move test (winner flips with an unchanged id, zero maintenance).
4. **Wording corrections** applied throughout: H5 narrowed to the
   measured pairing (below); Q6 freezes a requirement, not a chunk
   size; Q7 demoted to a provisional inventory; memory numbers labeled
   green-representation-only.

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
metadata work). Resolved by prototype: yes — RUN-3 (below).

**Q4 — does position-free green remove local-parse+global-metadata?**
Yes, with explicit scope. Position-free persistent block sequence,
single length-changing edit: cost flat in N — **0.18 µs at 1 MB,
13–14 ancestors path-copied, coordinate rewrites = 0 by construction**
(vs markit 132.9 µs @BOF). G1 (naive Vec) proves the plan-§15 warning:
without a sequence structure you still pay O(B) (38–40 µs). Red-view
position query 130 ns @1 MB ≈ the product's own 194 ns. **Long-lived
balance (CORRECTIVE-1 gate):** structural churn degrades a naive
path-only sequence (height 10.7× balanced after 100 k hotspot edits);
balance maintenance is a REQUIRED mechanism and the trivial
rebuild-on-height policy bounds it at amortized tens of ns/edit — the
concrete strategy (WB tree, B-tree, …) is NOT earned. Green
representation bytes ≈ 1× document bytes at per-block granularity —
that is the green tree ONLY, not total editor state (source + green +
inline + semantic indexes + allocator overhead; Arc headers not fully
counted). Not prototyped: multi-edit transactions, inline-IR placement
inside green nodes.

**Q5 — Markdown-specific boundary convergence vs generic subtree
reuse.** Narrowed per review (MAJOR-4): the campaign compared markit's
convergence, Lezer's per-block-hash fragments, and ONE pinned generic
pairing (tree-sitter runtime 0.19.5 + grammar 0.7.1); it did not
isolate "generic engine" from "grammar implementation quality", so no
generic-vs-Markdown-specific causal claim is made. Measured on the
tested workloads: Lezer fragments — incremental cost nearly
position-independent at 1 MB (bof 3.3 / mid 4.5 / eof 3.7 ms vs its
77.6 ms full), survives all adversarial docs; tree-sitter pairing —
structurally local reuse (corrected coverage ≈ 0 % for ordinary edits)
but N-scaled incremental cost (≈18 ms at 1 MB for a 1-byte edit with 0
structural changes) and a fatal abort on the 200-deep quote; markit's
own convergence stays µs-scale with the representation caveat RUN-3
removes. H5 verdict below reflects this scope.

**Q6 — keep flat blocks? which syntax needs finer trees?** Keep the
flat block stream as the *semantic layer* for ordinary paragraphs
(best convergence in the survey). Where run-1 measured
block-granularity floors, the architecture requirement to freeze is:
**huge blocks MUST admit sub-block locality, and containers MUST admit
nested structure** — both are requirements, not implementations. The
concrete chunking strategy for huge paragraphs (64-byte chunks, token
chunks, pieces, lazy inline tree) and the concrete container tree
shape remain open implementation choices: run-3 showed the chunked
representation itself is cheap (0.56 µs) but its value is a
*prospective* parse-scope win that has not been measured parse-side;
the nested-container chain costs O(depth) per edit (17.7 µs @200
levels). Fence forward state stays a block-level mechanism (S5).

**Q7 — is the propagation mechanism set small and enumerable?** The
campaign's honest answer is a **provisional inventory, not a proven
exhaustive set** (demoted per review): (1) local block replace; (2)
boundary neighbor repair (split/merge/convert); (3) forward fence
state; (4) container nesting (L1 flattens it; full grammar needs it);
(5) document reinterpretation (rare, S6); (6) semantic dependency
classes (reference labels — definitions/users); (7) provider
delegation (mermaid — flagged, unmeasured). CommonMark completeness is
NOT in hand: ORACLE-B has the current parser at 60.0 % PASS / 3.4 %
FLAT / 16.7 % FAIL / 19.9 % UNSUPPORTED, with failures concentrated in
reference definitions, setext, list grouping, and HTML block
boundaries. The architecture must treat propagation mechanisms as an
open, extensible set (`PropagationMechanism::…` grows), not a frozen
seven.

**Q8 — can a semantic index keep global meaning changes locally
parsed?** Yes (RUN-4). Definition edits: syntax 2–3 blocks / 1–6 µs
while 100–10 000 dependents invalidate; losing-duplicate edits proven
semantic no-ops (0 changed); index self-equivalence oracle passes on
every scenario; delta ∝ dependents and ~5–6× under full rebuild at
10 k. **Identity/order boundary (CORRECTIVE-1):** the index MUST take
document order from the syntax layer (`DocOrder` provider) and treat
`BlockId` as durability identity only — verified by six
winner-mutation gates including insert-before-winner (new larger id
wins) and move-before-winner (winner flips with unchanged id, zero
maintenance).

**Q9 — stable full/incremental crossover predictor?** Crossover is
real and reproduced cross-implementation: markit fence cascade
(incremental 2.0 ms > full 1.2 ms @1 MB); Lezer fence cascade
(1.57×); huge-block shapes near 1.0× (Lezer 1.04×, TS 0.66×).
Candidate predictor: *expected-reparse-share* (forward-state flips and
damage ≈ document scale) + block-count delta magnitude. **Insufficient
evidence for a validated threshold** — two implementations, two shapes
only; no adaptive threshold is proposed (plan §29). What the
architecture may freeze is the ESCAPE HATCH (below), not a predictor.

**Q10 — ADOPT / ADAPT / HYBRID / BUILD?** See verdict below: HYBRID
(mechanism level, with the CORRECTIVE-1 freeze list).

---

## Hypothesis scoreboard (plan §39)

| H | verdict | evidence |
|---|---|---|
| H1 local convergence | **SUPPORTED** | 397 scenarios: radius independent of N for S0–S3; ZERO_REPARSE_CONVERGENCE reproduced |
| H2 limited propagation mechanisms | **SUPPORTED_WITH_SCOPE** | observed set is enumerable and small (7 so far) but PROVISIONAL — completeness unproven (ORACLE-B gaps); architecture keeps the set extensible |
| H3 syntax/semantic separation | **SUPPORTED** | RUN-4: syntax blind to 10 k-dependents; delta ∝ dependents; no-op proofs; identity≠order boundary gated |
| H4 position-free representation | **SUPPORTED_WITH_SCOPE** | RUN-3: flat-in-N single edits, coord_rw=0; CORRECTIVE-1 gate: balance maintenance required for churn, trivial policy bounds it, concrete strategy open; memory = green-tree-only ≈1× source |
| H5 measured locality strategies beat the pinned tree-sitter pairing on tested workloads | **SUPPORTED_WITH_SCOPE** (narrowed) | RUN-2c + corrected RUN-2b: Lezer position-independent inc + robustness vs TS N-scaled cost + fatal abort; NO generic-vs-specific causal claim (grammar quality not isolated); the review's retraction of TS suffix-coverage applied |
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
> coherent full-rebuild escape hatch exists (its trigger threshold
> remains unvalidated).

Forced by: RUN-1.1 (axes independent, ZERO_REPARSE_CONVERGENCE),
RUN-4 (semantic fanout separable and indexable), RUN-3 + corrective
gate (representation minimum is structure-path cost plus mandatory
balance maintenance), RUN-2c (crossover clause).

---

## Parser mechanism verdict (plan §41, corrected freeze list)

**HYBRID** — architecture candidate, mechanism level. What the evidence
earns freezing:

**FROZEN (evidence-backed requirements):**

1. **Markdown-aware restart/convergence** — P0-02's algorithmic core
   (safe restart → reparse → earliest proven convergence → suffix
   reuse); tightest measured local convergence, radius independent of
   N for ordinary edits.
2. **Position-free persistent syntax representation** — unchanged
   suffix coordinates must never be rewritten merely because earlier
   source length changed (coord_rewrites ≡ 0 measured); red view for
   position queries; **balance maintenance is part of the
   requirement** (churn degrades naive path-copy — gate data), with a
   full-rebuild bound available; concrete balancing strategy open.
3. **Syntax / semantic dependency separation** — a semantic index
   keyed by stable block identity, taking document order from the
   syntax layer (never `BlockId = order`); fanout ∝ dependents, zero
   fanout for resolution-preserving edits, proven.
4. **Coherent full-rebuild escape hatch** — the parser architecture
   MUST allow abandoning incremental work and rebuilding coherently
   (crossover measured in two implementations); the trigger predictor
   is NOT validated and no adaptive threshold is proposed.

**NOT FROZEN (open, unearned):** exact sequence tree (WB / B-tree /
finger / …); huge-block chunking strategy (64-byte chunks were one
prototype, not a decision); container tree shape; inline-IR placement;
`BlockId`/`OrderKey` concrete encoding; crossover predictor; the
propagation-mechanism set (provisional inventory, extensible);
`P_provider` costs (never measured).

Not a ranking of implementations (plan §35); each baseline answers its
own question: MD4C = full-parse cost floor; tree-sitter = the measured
N-scaled cost and robustness of one pinned pairing (structural reuse
itself is local — corrected data); Lezer = Markdown-specific fragment
reuse.

---

## Major limitations

- Single machine (WSL2), single runs per configuration (medians over
  5–9 iters); allocator counting active for all latency rows (support
  evidence only, noted per run).
- Cross-implementation wall-clock not compared across runtimes (Lezer
  in JS); reused Lezer/TS evidence is ratio- and shape-based.
- tree-sitter runtime 0.19.5 + grammar 0.7.1 and @lezer/markdown
  1.7.2 are pinned published versions; newer runtimes may differ —
  and grammar implementation quality was never isolated from engine
  properties (H5 scope).
- ORACLE-B expected side is a targeted HTML scanner (edge cases pinned
  by the corpus; both implementations judged by identical rules).
- Semantic layer is an experimental subset (whole-paragraph defs;
  container defs unsupported); winner gates model the syntax layer as
  an ordered slot list, not a production OrderKey service.
- Green memory numbers are representation-only (kind+len nodes); no
  whole-state inventory was taken.
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
7. Concrete balance strategy for the persistent sequence (WB / B-tree
   / finger joins vs the measured rebuild policy) — the gate bounds
   the problem, it does not design the answer.

---

## Review disposition

MARKIT-19-CORRECTIVE-1 resolved all four MAJOR findings (A: tree-sitter
basis; B: history gate; C: identity/order split; D: wording) and the
three MINORs (provisional inventory; no chunk/container freeze;
memory wording). Per the review's gate — "这四项过了，#19 就应该正式
CLOSED" — the campaign's evidence work is complete.

VERDICT: **READY_FOR_MARKIT_MARKDOWN_ARCHITECTURE_REVIEW** (corrected
scope; architecture freezing limited to the FROZEN list above)

(No merge performed; no production parser begun; product docs
untouched. Campaign stops here per plan §42.)
