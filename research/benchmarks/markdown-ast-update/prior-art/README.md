# prior-art/ — R2 Prior-art Mechanism Extraction

Status: **R2 EXTRACTION RECORDS COMPLETE (pre-review)**
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
