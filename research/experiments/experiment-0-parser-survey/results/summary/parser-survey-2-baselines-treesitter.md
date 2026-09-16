# RUN-2b — external baseline: tree-sitter markdown (plan §10)

Issue: #19, branch `exp/19-parser-survey-1`. Pinned **tree-sitter runtime
0.19.5 + tree-sitter-markdown grammar 0.7.1** (the grammar pins that
runtime; both share one crate so the Language type unifies). Research-only
adapter: `crates/parser-survey/src/tsitter.rs` (`--ts` mode).
Raw: `results/raw/parser-survey/run-2-baselines/ts/` (superseded),
`results/raw/parser-survey/run-2b-corrective/ts/` (corrective rerun).

> **CORRECTIVE-1 (2026-09-16) — coverage column of the first run was a
> measurement artifact and is RETRACTED.** The adapter compared
> `changed_ranges` against the **unedited** old tree instead of the
> `edit()`-adjusted clone. Tree-sitter's contract requires the edited
> tree: positions must already be in new-document coordinates before the
> comparison. The unedited comparison measured pure coordinate offset —
> BOF insert 99.3 % "changed" (≈ whole doc), mid insert 49.5 % (≈ suffix
> fraction), EOF/equal-length 0 %. The corrected run below shows
> ordinary-edit coverage collapsing to 0.00–0.15 % at every position.
> Campaign consequences: the claims *"tree-sitter changed coverage =
> suffix fraction"*, *"suffix-scale reuse granularity"*, and the final
> report's generalization *"suffix-cost is a property of
> position-carrying representations"* are **withdrawn**. Timing columns
> are unaffected (the incremental parse always used the edited tree) and
> are reproduced by the corrective run within noise. The final-report
> wording is corrected in MARKIT-19-CORRECTIVE-1.

## Protocol (true incremental API only, plan §10)

`old tree (cold, timed)` → `old_tree.edit(InputEdit)` →
`parse(new_source, Some(old_tree)) (timed)` →
`edited_old_tree.changed_ranges(new_tree)` (reuse proxy). No full-parse
masquerading; the parser object persists across measurements. The
corrective run keeps the edited clone (old tree adjusted to the new
document's coordinates) for the comparison, per the tree-sitter contract.

## Results (corrective run)

| corpus | case | pos | doc_B | chg_B | ts_full | ts_inc | chg_ranges | cov_B | cov% | inc/full |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| synth-100k | para_insert_char | bof | 102 493 | 1 | 31.7 ms | 1.53 ms | 1 | 10 | 0.01 | 0.05× |
| synth-100k | para_insert_char | mid | 102 493 | 1 | 31.2 ms | 1.48 ms | 0 | 0 | 0.00 | 0.05× |
| synth-100k | para_insert_char | eof | 102 493 | 1 | 30.4 ms | 1.55 ms | 0 | 0 | 0.00 | 0.05× |
| synth-100k | para_sub_char | mid | 102 493 | 2 | 31.0 ms | 1.74 ms | 0 | 0 | 0.00 | 0.06× |
| synth-100k | blank_delete | mid | 102 493 | 1 | 31.2 ms | 1.55 ms | 1 | 151 | 0.15 | 0.05× |
| synth-100k | para_split | mid | 102 493 | 3 | 30.8 ms | 1.57 ms | 1 | 113 | 0.11 | 0.05× |
| synth-100k | fence_len_grow | mid | 102 493 | 1 | 30.8 ms | 9.01 ms | 1 | 102 115 | 99.63 | 0.29× |
| synth-1m | para_insert_char | bof | 1 048 633 | 1 | 325.0 ms | 17.4 ms | 1 | 10 | 0.00 | 0.05× |
| synth-1m | para_insert_char | mid | 1 048 633 | 1 | 314.1 ms | 17.98 ms | 0 | 0 | 0.00 | 0.06× |
| synth-1m | para_insert_char | eof | 1 048 633 | 1 | 319.6 ms | 18.3 ms | 0 | 0 | 0.00 | 0.06× |
| adv-huge-paragraph | para_insert_char | mid | 100 036 | 1 | 22.5 ms | 15.5 ms | 1 | 4 | 0.00 | 0.69× |
| adv-deep-quote-50 | quote_char | mid | 3 007 | 1 | 0.67 ms | 0.46 ms | 1 | 1 521 | 50.57 | 0.68× |
| **adv-deep-quote (200)** | quote_char | mid | — | — | — | — | — | — | — | **FATAL ABORT** |

Timing reproduced the first run within noise (e.g. mid insert @1 m:
17.1 → 18.0 ms), confirming only the coverage column was wrong.

## Findings (mechanism, not ranking — plan §35; corrected)

1. **This grammar pairing is extremely slow at cold parse**: ≈ 300 ns/B
   at 100 KB–1 MB (vs MD4C 1.1–1.2, markit build 4.1–5.8). Grammar
   implementation quality dominates generic-engine properties; "generic
   incremental engine" is not inherently slow or fast — the pairing is
   the measured object.
2. **Structural reuse is local and position-independent** (corrected):
   ordinary 1-byte edits at any position produce 0–1 changed ranges
   spanning ≈ 0 % of the document. The first run's "suffix-scale
   coverage" was the coordinate-offset artifact retracted above. On this
   corrected evidence, tree-sitter's edited-tree comparison does NOT
   behave like markit's M4 suffix metadata rewrite at the structural
   level.
3. **Incremental cost still scales with N even when nothing structurally
   changes**: mid insert, 0 changed ranges, yet 1.48 ms @100 KB → 18.0 ms
   @1 MB; equal-length edit 1.7 ms @100 KB. Whatever the engine does
   (cache/coordinate machinery inside the C runtime), it is
   N-proportional and NOT visible as structural coverage; the runtime
   exposes no attribution for it. Recorded as an unexplained N-term, not
   as reuse evidence either way.
4. **changed_ranges is a structural-extent proxy, not a content diff**:
   same-length substitution and mid-insert both report 0 ranges; a
   blank-line delete reports 151 B. Content-only changes inside an
   unchanged-extent token are invisible to this proxy (both runs). The
   plan-§34 reuse statement is therefore bounded: local structural reuse
   yes; per-byte reuse unmeasurable with this API.
5. **Huge paragraph**: changed coverage ≈ 0 but inc ≈ 0.69× cold — the
   re-lex of the changed 100 KB node dominates, mirroring markit's
   R=100 003 (E3): granularity, not engine, bounds locality.
6. **Fence cascade is genuine global damage**: cov 99.63 %, inc 9.0 ms
   @100 KB — full reinterpretation like markit's S5 (markit: ≈2.0 ms at
   1 MB). This row was already correct in the first run and survives the
   correction.
7. **Deep-nesting cost is real in the quote chain**: 50-deep quote edit
   marks the enclosing chain (1 521 B ≈ the quote prefix) as changed —
   consistent with markit's own R ≈ 42 093 flat-L1 quote cost (run-1
   E3). Not an artifact.
8. **Robustness**: tree-sitter-markdown 0.7.1 **fatally aborts** (C
   layer, abort — unrecoverable in-process) on the 200-deep quote
   document that markit L1 parses honestly (R ≈ 42 093, 131 µs update).
   Robustness under deep nesting is a measurable mechanism difference,
   recorded as a finding (run-1 E3/E5 context).

## Limitations

- Runtime 0.19.5 + grammar 0.7.1 is what the published grammar pins;
  newer tree-sitter runtimes (0.25+) have parser improvements — the
  pinned pairing is recorded and all scaling statements are relative to
  its own cold parse, not absolute claims about the engine.
- Block-only grammar (the crate's `markdown` language; `markdown-inline`
  is a separate language not injected here) — comparable scope to
  markit's L1 block layer by design.
- Nodes-rebuilt/reused are not exposed by this runtime; per plan §10 we
  use changed-ranges/source coverage as the reuse proxy and do not
  fabricate counters. The proxy measures structural extent changes only
  (finding 4) and required the corrected edited-tree basis (erratum).
- `has_error` true only on the huge-paragraph corpus (grammar error
  node for very long inline runs).
