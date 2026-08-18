# PocketJS MVP → Rust core: knowledge transfer

This document preserves the **transferable design knowledge** that lived
in `mvp/pocketjs/` (deleted 2026-08-18 with the architecture pivot). It
does not archive runnable code — the source history preserves that. The
A4-era implementation (framework-free TS modules) is the seed of
`markit-core`; this file records what must carry over and where the
evidence lives.

See also: ADR-003 (line index), ADR-004 (Markdown invalidation),
ADR-005 (viewport-bounded rendering), ADR-007 (IME composition).

## 1. Document + LineIndex (ADR-003)

- `Document` is a string; `LineIndex` is built with one full scan at
  load and maintained incrementally per edit.
- Local update rule: drop/add newline entries **inside the changed
  range**, shift the suffix by the length delta.
- Every edit returns an explicit changed range (`EditResult.change`) —
  the seam every layer consumes. Never re-derive it by re-scanning.
- Remaining cost: O(lines-after) suffix shift, position-dependent
  (measured ~2.3 ms at 1M begin-position edits). A buffer redesign
  (rope / piece table) is a real-workload decision, not a pre-emptive
  one.
- Evidence: ADR-003; A3 224.8 → 9.9 ms; 38 tests incl. randomized
  differential vs the full-scan oracle.

## 2. Incremental Markdown L1 (ADR-004)

- `BlockIndex`: line→block map; `applyEdit(startLine, endLine, ...)`
  rescans from the first affected block forward and **stops at the first
  stable boundary** (kind + alignment match beyond the edited lines),
  carrying fence state across the rescan.
- Styled runs are computed per affected block (inline parse, cached by
  block start line, invalidated for exactly the replaced blocks) and
  sliced per visible line.
- Local edits (paragraph, inline, heading, list, off-viewport) reparse
  **exactly one block** at any document size (10K and 1M).
- The full-document scan is the load-time and test-oracle path only.
- Structural fence-boundary edits invalidate forward through the fence
  cascade (1M: 30 197 lines, 68.9 ms, honest) — must be bounded by a
  product strategy (only treat ``` as an opener when a matching close
  exists ahead), never benchmarked away.
- Conformance scope: the L1 subset (heading, paragraph, bold, emphasis,
  inline code, link, blockquote, ul/ol list, fenced code) — **not**
  CommonMark. The 5000-edit randomized differential proves incremental
  invalidation correctness (incremental == full scan of the same
  parser), not Markdown conformance.

## 3. Stateless visible projection (ADR-005 companion)

- Visible-list items are keyed by absolute document line number and
  carry **no state**; every doc-dependent read is hoisted into
  item-scoped memos.
- Rationale: absolute line numbers are not stable document identity
  (inserting `\n` before the viewport shifts every later line), so
  correctness comes exclusively from stateless re-derivation.
- The A4-R1 finding: fresh item objects per render re-mounted all 26
  visible line components per edit (~90 native node creations/GC churn
  measured) — the "reactive identity amplification" that cost ~7 ms in
  the Solid layer. The direct-GPUI core must express the equivalent
  invariant for whatever per-line presentation it materializes.
- If a future line widget needs state (IME, folding, inline widgets),
  identity must move to a stable block/content ID, not the absolute line
  number.

## 4. Correctness tests worth re-encoding

- Line index: randomized differential vs full-scan oracle (38 tests in
  the TS seed).
- Markdown: 5000-edit randomized differential vs the full-scan oracle
  (10 tests).
- `caretFromX` regression: the pre-existing bug `lineIndex * doc.length`
  clamped every click on a line ≥ 1 to the document end; a permanent
  regression test belongs in the core suite (backlog item).

## 5. What does NOT transfer

- Solid reconciliation discipline (no Solid in the GPUI core).
- The PocketJS host/svc protocol and its wire messages.
- The DrawList contract and its word-count measurement.
- QuickJS / pak / companion / capability runtime concepts.

## 6. Where the evidence lives

```text
docs/phase-a2-causal-decomposition.md        lineStarts scan root cause
docs/phase-a3-intervention-validation.md     LineIndex + viewport fixes
docs/phase-a4-final-research-closeout.md     residual decomposition + L1 pipeline
docs/adr/ADR-003..007                        decisions
results/summary/a2|a3|a4/                    per-run benchmark summaries
workloads/corpus-md/                          L1 corpora + positions manifests
```

Source history: `git log -- docs/phase-a1-* mvp/pocketjs` (the
`mvp/pocketjs` tree remains in git history after its removal).
