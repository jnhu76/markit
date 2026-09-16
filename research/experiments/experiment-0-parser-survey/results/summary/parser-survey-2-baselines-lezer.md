# RUN-2c — external baseline: Lezer Markdown (plan §11)

Issue: #19, branch `exp/19-parser-survey-1`. Pinned **@lezer/markdown
1.7.2 + @lezer/common 1.5.2**, Node v24.15.0. Research-only persistent
worker: `crates/parser-survey/scripts/lezer-worker.js`; Rust driver:
`src/lezer.rs` (`--lezer`). Raw: `results/raw/parser-survey/run-2-baselines/lezer/`
(local).

## Timing discipline (plan §11 rule, implemented literally)

The Node process stays warm across all scenarios; medians are taken
**inside the JS runtime** (`process.hrtime`), and the Rust-side
wall-clock is reported only as transport (`ipc_wall`) and never merged
into parser numbers. Fragment adjustment (`TreeFragment.applyChanges`)
is timed separately (`adjust_native`) — it is Lezer's
representation-maintenance step.

## Results

| corpus | case | pos | doc_B | chg_B | full_native | inc_native | adjust_native | inc/full | ipc_wall |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| synth-100k | para_insert_char | bof | 102 494 | 1 | 8 714 | 1 117 | 15.3 | 0.13× | 135 667 |
| synth-100k | para_insert_char | mid | 102 494 | 1 | 9 949 | 756 | 6.9 | 0.08× | 86 920 |
| synth-100k | para_insert_char | eof | 102 494 | 1 | 8 042 | 687 | 2.2 | 0.09× | 79 722 |
| synth-100k | para_sub_char | mid | 102 493 | 2 | 8 927 | 637 | 2.6 | 0.07× | 81 399 |
| synth-100k | blank_delete | mid | 102 492 | 1 | 8 468 | 768 | 2.2 | 0.09× | 80 762 |
| synth-100k | fence_len_grow | mid | 102 494 | 1 | **497** | 781 | 2.2 | **1.57×** | 35 245 |
| synth-1m | para_insert_char | bof | 1 048 634 | 1 | 77 806 | 3 272 | 4.8 | 0.04× | 605 333 |
| synth-1m | para_insert_char | mid | 1 048 634 | 1 | 77 591 | **4 483** | 12.6 | 0.06× | 599 580 |
| synth-1m | para_insert_char | eof | 1 048 634 | 1 | 73 171 | 3 701 | 8.5 | 0.05× | 579 640 |
| adv-huge-paragraph | para_insert_char | mid | 100 037 | 1 | 6 706 | 7 000 | 3.4 | **1.04×** | 80 196 |
| adv-deep-quote (200) | quote_char | mid | 42 109 | 1 | 9 785 | 5 810 | 4.7 | 0.59× | 134 757 |

## Findings (mechanism language, plan §35)

1. **Markdown-specific fragment reuse pays where generic subtree reuse
   did not**: a mid 1-byte insert at 1 MB costs 4.48 ms incremental vs
   77.6 ms full (17×), and the incremental cost is nearly
   position-independent (bof 3.27 / mid 4.48 / eof 3.70 ms). Compare
   tree-sitter on the same corpus/edit: 17.1 ms at mid — Lezer's
   per-block-hash fragments keep the effective reparse closer to the
   edit than tree-sitter's suffix-scale granularity. **This is the
   first direct evidence for H5 (Markdown-specific boundary advantage).
   Confidence: moderate — cross-runtime (JS) absolutes are not
   comparable, but the position-independence shape is a mechanism
   signature, not a speed claim.**
2. **H6 crossover exists here too**: the fence cascade is CHEAPER to
   full-parse than to parse incrementally (497 µs vs 781 µs at 100 KB;
   the document collapses to ~a dozen fence blocks). Same direction as
   markit's I4 crossover — the crossover is a property of the document
   transformation, not of the implementation.
3. **E3 again — block granularity bounds reuse**: the 100 KB single
   paragraph shows inc ≈ full (1.04×): fragments are block-level, so a
   huge block gets no reuse. Identical lesson as markit (R=100 003) and
   tree-sitter (re-lex dominated).
4. **Robustness**: the 200-deep quote document — which fatally aborts
   tree-sitter-markdown 0.7.1 and parses honestly in markit L1 — parses
   fine in Lezer (5.8 ms inc). Robustness under deep nesting differs
   across all three measured implementations and is now measured, not
   assumed.
5. Representation work: fragment adjustment is 2–15 µs — negligible
   next to parse cost at these scales (Lezer defers positions to
   fragment-cached offsets rather than rewriting stored ranges at edit
   time).

## Limitations

- JS runtime: absolute numbers are not cross-comparable with the Rust
  implementations (plan §11 forbids it); the used signals are
  within-runtime ratios and position-scaling shapes.
- No changed-range or reuse counters exposed by Lezer; reuse evidence
  is the inc/full ratio + the fragment flow. Nothing fabricated.
- CommonMark dialect: Lezer's CommonMark+GFM-ish dialect was NOT run
  through ORACLE-B in this pass (its tree nests containers, so the
  same normalized-sequence comparison applies but needs container-aware
  flattening of ITS side — deferred; the mechanism evidence above does
  not depend on it).
