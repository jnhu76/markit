# prior-art/ — R2 Prior-art Mechanism Extraction

Status: **R2 RECORDS COMPLETE — CORRECTIVE PASS APPLIED;
READY_FOR_FINAL_R2_REVIEW**
Authority: GitHub Issue #22 + `protocol/R0-METHODOLOGY.md` §2–§3 (prior-art
role and frozen horses) + `ROADMAP.md` (R2 stage gate).

This directory holds the R2 prior-art mechanism extraction for
MARKIT-MARKDOWN-BENCHMARK-1. Direction of inference is extraction-first:

```text
paper/code says X
-> reconstruct X (state / reuse / invalidation / fallback)
-> separate OBSERVED / INFERRED / UNKNOWN
-> only then ask whether X is relevant to H0-H4
-> record fidelity boundary
```

Prior-art sources are provenance and mechanism evidence only. No upstream
project is a benchmark subject here; upstream absolute timing is
`REFERENCE_ONLY` (R0 §2). Every repository source is pinned to a tag or
commit SHA; every claim in the records is classified OBSERVED (cited to the
pinned source), INFERRED (labeled reasoning), or UNKNOWN (insufficient
evidence). Negative evidence is first-class.

## Records

| File | Subject | Pinned version | Role highlights |
|---|---|---|---|
| [md4c.md](./md4c.md) | MD4C | `release-0.5.3` | H0 clean full-parse anchor; negative evidence: no incremental API |
| [pulldown-cmark.md](./pulldown-cmark.md) | pulldown-cmark | `v0.13.4` | H0 anchor; eager/lazy two-pass split; zero retained state |
| [comrak.md](./comrak.md) | Comrak | `v0.55.0` | H0 anchor; refmap-before-inlines ordering |
| [tree-sitter.md](./tree-sitter.md) | Tree-sitter core engine | `v0.27.0` | H3 old-tree subtree-reuse concepts; hybrid reuse+convergence gating |
| [tree-sitter-markdown.md](./tree-sitter-markdown.md) | tree-sitter-markdown grammars | `v0.5.3` | Markdown hidden scanner state (container stack, fence state); issue-backed limits |
| [lezer.md](./lezer.md) | @lezer/common + @lezer/lr + @lezer/markdown | `1.5.2` / `1.4.8` / `1.6.3` | H2 fragment-reuse anchor; state-anchored node/block absorption; markdown convergence guards |
| [mizchi-markdown.md](./mizchi-markdown.md) | mizchi/markdown (`markdown.mbt`) | master `ffe7dc00` | H1 block-local reparse anchor (MoonBit, not Rust); definition-fallback |
| [wagner-graham.md](./wagner-graham.md) | Wagner & Graham TOPLAS 1998 (+ dissertation CSD-97-946) | DOI 10.1145/293677.293678 | H4 classical restart/convergence; subtree reuse overlap |
| [swift-incremental-syntax.md](./swift-incremental-syntax.md) | swift-syntax incremental parsing | `604.0.0` | H4 per-checkpoint reuse; lookahead ranges; forum-thread-vs-code divergence |

## Synthesis

- [SOURCE-MAP.md](./SOURCE-MAP.md) — source inventory with classes,
  versions, retrieval dates, and additional-source decisions.
- [MECHANISM-SOURCE-MAP.md](./MECHANISM-SOURCE-MAP.md) — required per-horse
  map: MECHANISM_SOURCE_MAP / PRIOR_ART_ANCHOR / FIDELITY_BOUNDARY /
  NON_GOALS / MECHANISM_INTRINSIC_STATE for H0–H4.
- [MECHANISM-MATRIX.md](./MECHANISM-MATRIX.md) — cross-mechanism comparison
  (rows are mechanisms, not products) with confidence classes.
- [R2-HYPOTHESES.md](./R2-HYPOTHESES.md) — hypotheses only; questions R3+
  must test. No performance numbers, no winners.

## Scope of this directory

Research documentation only. No mechanism code, no benchmark results, no
timing numbers, no R3 grammar/corpus work, no Markit production algorithm.
The per-record manifest entries are consolidated in
`../manifest/prior-art.toml`.

## R2 adversarial self-review (2026-09-16)

A fresh-context review tested failure modes A–L (confirmation bias,
product/mechanism confusion, inference laundering, provenance drift,
false equivalence, hidden state, reconstruction/position-cost blindness,
optimization leakage, unsupported weakness, horse-preservation bias,
source-quality failure) plus scope/manifest/consistency checks, including
12 load-bearing citation spot-checks against the pinned upstream clones
(12/12 VERIFIED).

Verdict: `PRIOR_ART_REVIEW_IMPORTANT_ONLY` — zero MAJOR findings.
Resolved before PASS:

1. IMPORTANT — R2-H09 asserted H3 offset maintenance is O(N)/edit,
   contradicting its cited record (tree-sitter patches only the edited
   path); rewritten to separate naive patching, edit-path patching, and
   read-time derivation.
2. MINOR — R2-H04 dropped the ContextTracker qualifier and attributed the
   markdown rolling hash to the LR consumer; corrected.
3. MINOR — dangling "§8 STOP rule" cross-reference in
   MECHANISM-SOURCE-MAP.md; replaced with the actual authority (R0 §3).
4. MINOR — embedded TOML table names normalized to `[[prior_art]]` across
   record §15 blocks.
5. MINOR — manifest header reworded: it carries the load-bearing
   mechanism-relevant paths; record §15 lists remain the exhaustive ones.

## R2 corrective pass (2026-09-17, MARKIT-R2-PRIOR-ART-CORRECTIVE-1)

Human adversarial review of PR #27 required fidelity corrections. All pins,
tags, SHAs, and extraction provenance are unchanged; the corrections touch
claim wording only:

1. MAJOR-1 — mizchi reuse semantics corrected: prefix blocks pass through
   unchanged; suffix blocks are RECONSTRUCTED as new values
   (`shift_block_span`) with shifted spans. `reused_*` counters mean
   "not reparsed" (parser-work reuse), NOT object/representation reuse;
   no MoonBit pointer-identity claims are made. (mizchi-markdown.md §4/§8/
   §14/§15, MECHANISM-SOURCE-MAP.md H1, MECHANISM-MATRIX.md M2,
   R2-HYPOTHESES.md R2-H11, manifest.)
2. MAJOR-2 — universal/minimal-state claims removed: the
   tree-sitter-markdown serialized state is KNOWN-SUFFICIENT for that
   implementation's reuse gate (CANDIDATE dimensions for H1/H3/H4;
   MINIMALITY UNKNOWN; field schema enumerable, value size
   depth-dependent per #243; retention vs recomputation
   mechanism-dependent). No document now defines or prescribes the
   eventual H1/H4 checkpoint representation. (tree-sitter-markdown.md
   §14/Q12, MECHANISM-SOURCE-MAP.md H1/H4, R2-H07, manifest.)
3. MAJOR-3 — tree-sitter GLR reuse suppression corrected: reuse is
   suppressed WHILE multiple stack versions exist (`allow_node_reuse`
   recomputed per outer parse-loop iteration after condense); a later
   condense back to one version re-enables it. Not a remainder-of-parse
   shutdown, not a full-parse fallback. (R2-H06, tree-sitter.md §12/Q11/
   §14, MECHANISM-SOURCE-MAP.md H3, MECHANISM-MATRIX.md note, manifest.)
4. IMPORTANT-1 — M6 matrix row repaired: the missing memory-tradeoff cell
   restored (external-scanner state retained with the old tree;
   depth-dependent serialized size); "per-token snapshot" replaced with
   the exact carrier (byte-serialized external-scanner state on
   external-token subtrees); all 15 columns aligned.
5. IMPORTANT-2 — observed mizchi state (old Document / top-level block
   sequence with spans + API inputs) separated from MODEL-DEFINED
   CANDIDATE state (dedicated block index/table, definition-presence
   flag, entry-context cache); no dedicated index carries
   MECHANISM_INTRINSIC status.
6. MINOR — unsupported "common in real Markdown" frequency wording
   removed (real-corpus frequency stays UNKNOWN in the record).

## R2 final cleanup (2026-09-17, MARKIT-R2-FINAL-CLEANUP-1)

Minimal cleanup after the human adversarial review's corrective pass;
no pins, sources, R0/R1, or horse implementations touched:

1. MECHANISM_INTRINSIC_STATE classification repaired
   (MECHANISM-SOURCE-MAP.md): four-way distinction made explicit —
   MECHANISM STATE (only kind carrying MECHANISM_INTRINSIC) vs COMMON
   INPUT (R0 substrate-owned Source pre/post-edit + Edit descriptor;
   mizchi's EditInfo is an instance of the shared input, not retained
   state) vs COMMON INSTRUMENTATION (R0 §10 counters, incl.
   restart/convergence distance bookkeeping — removed from H4 intrinsic
   state) vs MODEL-DEFINED CANDIDATE STATE (non-intrinsic until parity
   review). H1 observed mechanism state = the retained old Document /
   top-level syntax structure with spans.
2. R2-H11 de-overgeneralized: OBSERVED basis rewritten
   mechanism-by-mechanism (mizchi suffix reconstruction; Lezer shared
   subtrees + fresh wrappers; tree-sitter shared subtrees + edit-path
   coordinate repair + fresh parents/root where required; Swift shared
   RawSyntax + fresh parents). The cross-mechanism claim
   (reconstruction/coordinate maintenance offsetting parser-work reuse
   is mechanism- and shape-dependent) is now explicitly HYPOTHESIS; no
   cost proportionality is claimed. The separate-counters requirement
   (parser bytes inspected / nodes reused / nodes rebuilt /
   metadata-ranges touched) is preserved.
3. mizchi-markdown.md §7: "suffix blocks are reused unconditionally"
   replaced by the explicit split — suffix syntax parsing is skipped
   without a semantic boundary-validity check, while suffix block values
   are re-coordinated/reconstructed through `shift_block_span`. No
   object-identity claims reintroduced.

Review history: the earlier extraction self-review
(`PRIOR_ART_REVIEW_IMPORTANT_ONLY — zero MAJOR`) was SUPERSEDED by the
human adversarial review that required R2-CORRECTIVE-1; this cleanup
polishes its outcome.

Gate status: `READY_FOR_FINAL_R2_REVIEW` (final human PASS is not
self-declared).
