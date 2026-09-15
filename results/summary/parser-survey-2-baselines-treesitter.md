# RUN-2b — external baseline: tree-sitter markdown (plan §10)

Issue: #19, branch `exp/19-parser-survey-1`. Pinned **tree-sitter runtime
0.19.5 + tree-sitter-markdown grammar 0.7.1** (the grammar pins that
runtime; both share one crate so the Language type unifies). Research-only
adapter: `crates/parser-survey/src/tsitter.rs` (`--ts` mode).
Raw: `results/raw/parser-survey/run-2-baselines/ts/` (local).

## Protocol (true incremental API only, plan §10)

`old tree (cold, timed)` → `old_tree.edit(InputEdit)` →
`parse(new_source, Some(old_tree)) (timed)` →
`Tree::changed_ranges(old', new)` (reuse proxy). No full-parse
masquerading; the parser object persists across measurements.

## Results

| corpus | case | pos | doc_B | chg_B | ts_full | ts_inc | chg_ranges | cov_B | cov% | inc/full |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| synth-100k | para_insert_char | bof | 102 493 | 1 | 29.9 ms | 1.62 ms | 683 | 101 737 | 99.3 | 0.05× |
| synth-100k | para_insert_char | mid | 102 493 | 1 | 29.6 ms | 1.47 ms | 339 | 50 740 | 49.5 | 0.05× |
| synth-100k | para_insert_char | eof | 102 493 | 1 | 28.5 ms | 1.47 ms | 2 | 2 | 0.00 | 0.05× |
| synth-100k | para_sub_char | mid | 102 493 | 2 | 28.6 ms | 1.44 ms | **0** | **0** | 0.00 | 0.05× |
| synth-100k | blank_delete | mid | 102 493 | 1 | 29.5 ms | 1.42 ms | 338 | 50 889 | 49.7 | 0.05× |
| synth-100k | para_split | mid | 102 493 | 3 | 28.9 ms | 1.43 ms | 338 | 50 850 | 49.6 | 0.05× |
| synth-100k | fence_len_grow | mid | 102 493 | 1 | 28.7 ms | 7.95 ms | 1 | 102 116 | 99.6 | 0.28× |
| synth-1m | para_insert_char | bof | 1 048 633 | 1 | 308.0 ms | 17.6 ms | 6 831 | 1 041 729 | 99.3 | 0.06× |
| synth-1m | para_insert_char | mid | 1 048 633 | 1 | 304.7 ms | 17.1 ms | 3 406 | 520 814 | 49.7 | 0.06× |
| synth-1m | para_insert_char | eof | 1 048 633 | 1 | 304.2 ms | 17.4 ms | 2 | 2 | 0.00 | 0.06× |
| adv-huge-paragraph | para_insert_char | mid | 100 036 | 1 | 20.5 ms | 13.6 ms | 1 | 3 | 0.00 | 0.66× |
| adv-deep-quote-50 | quote_char | mid | 3 007 | 1 | 0.66 ms | 0.47 ms | 1 | 1 522 | 50.6 | 0.71× |
| **adv-deep-quote (200)** | quote_char | mid | — | — | — | — | — | — | — | **FATAL ABORT** |

## Findings (mechanism, not ranking — plan §35)

1. **This grammar pairing is extremely slow at cold parse**: ≈ 290 ns/B
   at 100 KB–1 MB (vs MD4C 1.1–1.2, markit build 4.1–5.8). Grammar
   implementation quality dominates generic-engine properties; "generic
   incremental engine" is not inherently slow or fast — the pairing is
   the measured object.
2. **Incremental genuinely wins vs its own cold parse** (16–18× at 1 MB
   inserts) **but its cost still scales with N**: mid-insert 1.47 ms at
   100 KB → 17.1 ms at 1 MB. Tree-sitter reuse granularity on this
   fence-less corpus is suffix-scale, not paragraph-local.
3. **changed-ranges mirror markit's M4 phenomenon in tree form**: a pure
   offset shift makes every shifted subtree's spans differ, so a mid
   1-byte insert marks ~49% of the source as "changed" (≈ suffix
   fraction). Tree-sitter does not *rewrite* stored offsets at edit
   time (that work is deferred into position queries), but a
   position-keyed downstream consumer sees the same suffix-scale
   invalidation surface as markit's `survivor_blocks_shifted`. This is
   a core §34 observation: **the suffix-cost is a property of
   position-carrying representations, not of one implementation.**
4. **Equal-length edit → 0 changed ranges** (para_sub): tree-sitter's
   floor cost ≈ 1.4 ms is the machinery cost (re-lex + validation) even
   when nothing changes — vs markit's 2.4 µs for the same edit.
5. **Huge paragraph**: changed coverage ~0 but inc ≈ 0.66× cold — the
   re-lex of the changed 100 KB node dominates, mirroring markit's
   R=100 003 (E3): granularity, not engine, bounds locality.
6. **Fence cascade**: cov 99.6%, inc 7.95 ms @100 KB — full
   reinterpretation like markit's S5 (markit: ≈2.0 ms at 1 MB;
   ≈ raw scan cost).
7. **Robustness**: tree-sitter-markdown 0.7.1 **fatally aborts** (C
   layer, exit via abort — unrecoverable in-process) on the 200-deep
   quote document that markit L1 parses honestly (R ≈ 42 093, 131 µs
   update). A 50-deep variant works. Robustness under deep nesting is a
   measurable mechanism difference, recorded as a finding (run-1 E3/E5
   context).

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
  fabricate counters. changed_ranges includes pure-position changes
  (see finding 3) — stated, not corrected away.
- `has_error` true only on the huge-paragraph corpus (grammar error
  node for very long inline runs).
