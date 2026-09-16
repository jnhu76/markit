# parser-survey run 1.1 — measurement corrective (influence vectors, oracle naming, losslessness)

Issue: #19 — RUN-1.1-MEASUREMENT-CORRECTIVE (hard gate before external baselines)
Branch: `exp/19-parser-survey-1`
Base: run-1 evidence commit `1c21b9a`; harness evolves in this commit.
Measured impl: markit-core `MarkdownState` (P0-02 incremental block index), unchanged — **zero product-code changes in this run**.
Raw data: `results/raw/parser-survey/run-1.1-corrective/` (local, ignored;
`cases.csv` is the authoritative row set, 397 scenarios + 13 skips).
Toolchain: rustc 1.97.1 release, WSL2 x64, epoch 1789497912.

## Why a corrective run

Run 1 scored every edit with one axis (I0–I6). That conflates four costs
that fail independently:

- *syntax* — how far the reparse propagates;
- *semantic* — which dependents must re-resolve meaning;
- *representation* — which stored records must move/rewrite;
- *downstream* — which consumers are invalidated, and why.

Run 1's headline ("insert 69.6 µs vs substitute 4.3 µs ⇒ 94% is metadata
work") also attributed wall-clock to survivor shifts without a fit. The
corrective run adds an observation schema, splits the downstream axis,
naming-corrects the oracles, and adds a losslessness oracle.

## Schema (research observation, not a product API)

`crates/parser-survey/src/influence.rs` defines:

```
InfluenceVector {
  syntax:         S0_NO_REPARSE | S1_TOKEN_OR_INLINE_LOCAL | S2_BLOCK_LOCAL
                | S3_NEIGHBOR_LOCAL | S4_CONTAINER_SCOPED | S5_STATE_PROPAGATING
                | S6_DOCUMENT_REPARSE
  semantic:       D0_NONE | D1_LOCAL | D2_NEIGHBOR | D3_CONTAINER
                | D4_INDEXED_DEPENDENTS | D5_GLOBAL | D_UNKNOWN
  representation: M0_NONE | M1_LOCAL_NODE_REWRITE | M2_ANCESTOR_PATH
                | M3_LOCAL_SEQUENCE_SHIFT | M4_SUFFIX_METADATA_REWRITE
                | M5_GLOBAL_REBUILD
  downstream:     P_semantic (blocks/bytes) | P_coordinate (block records,
                  inline nodes) | P_provider
}
```

Rules: classes are derived **only from that row's observed counters**
(`MarkdownWork` + state observations), never from the mutation id.
Thresholds: S5/S6 split by restart position on ≥half-document scans;
S5 escape = convergence past max(covering block end, edit end + inserted
lines) > 16 lines; M4 needs shifts ≥ ½ the suffix-proportional
expectation; M5 at ≥ 0.9 × block count. All thresholds sit next to raw
counters in every table and are auditable per row.

Honest unobservables in flat L1: **S1** is never emitted (any touched
block re-parses whole; no inline-only repair path exists), **M2** is
never emitted (no ancestor paths exist), and **semantic is D_UNKNOWN for
all 397 rows** (no semantic dependency layer exists yet — run 4 adds
one). Provider axis: `MERMAID_CANDIDATE` flags edits touching a mermaid
fence (53 rows); no provider consumer is measured.

## Oracle renames and additions

- **ORACLE-A (SELF_EQUIVALENCE)** — what run 1 called "the oracle":
  incremental == *same implementation's* clean rebuild. 397/397 pass.
  This proves self-consistency, not CommonMark correctness.
- **ORACLE-C (LOSSLESSNESS)** — new: block ranges must tile the source
  in ascending, char-boundary-safe, non-overlapping ranges with only
  whitespace-only gaps (nothing can hide outside a block). 397/397 pass,
  including the new `adv-emoji` corpus (astral emoji, ZWJ family,
  skin-tone, flag sequences, combining marks — U3 on the Unicode ladder).
- **ORACLE-B (DIALECT SEMANTICS, CommonMark)** — not yet wired; next run.

## Observed class distribution (397 rows)

| syntax | n | | representation | n |
|---|---:|---|---|---:|
| S0_NO_REPARSE | 6 | | M1_LOCAL_NODE_REWRITE | 69 |
| S1 (unobservable in L1) | 0 | | M2 (unobservable in L1) | 0 |
| S2_BLOCK_LOCAL | 130 | | M3_LOCAL_SEQUENCE_SHIFT | 6 |
| S3_NEIGHBOR_LOCAL | 179 | | M4_SUFFIX_METADATA_REWRITE | 125 |
| S4_CONTAINER_SCOPED | 30 | | M5_GLOBAL_REBUILD | 197 |
| S5_STATE_PROPAGATING | 46 | | | |
| S6_DOCUMENT_REPARSE | 6 | | | |

- **S0 is a reproduced class, not a fluke**: `blank_delete` at BOF (all
  6 plain/variant corpora) reparse **zero** blocks (restart = convergence
  = line 1) — yet shifts 13 657 block records + 6 846 inline nodes and
  pays 215 µs at 1 MB. ZERO_REPARSE_CONVERGENCE: zero parse work is **not**
  zero metadata work.
- **S6×6** are `blank_insert@BOF`-family rows: restart at BOF with a
  ≥half-document scan is classified document-level even though few
  blocks rebuild.
- **M5 dominance is partly anchor geometry**: mutations anchored by
  construct search (codespan/emphasis/heading/quote/ref/mermaid) resolve
  to the kitchen-sink *header* — edits land near BOF, so shifting every
  subsequent record is geometrically correct, not an anomaly. Known
  corpus limitation; §30 matrix expansion should seed constructs at
  depth.

## Attribution upgrade for E2 (the hidden-O(N) delta)

Least squares over all 48 length-changing-edit rows (plain corpora, all
sizes × positions):

```
inc_us ≈ 0.37 µs + 9.0 ns × survivor_blocks_shifted    (R² = 0.971)
```

Three anchors at 1 MB mid/BOF:

| edit | survivor records shifted (blocks+inline) | inc_us |
|---|---:|---:|
| equal-length substitute (mid) | 0 | 2.4 |
| insert 1 B (mid) | 6 809 + 3 404 | 59.5 |
| insert 1 B (BOF) | 13 658 + 6 846 | 136.9 |

Wall-clock tracks survivor-record movement linearly with a small parse
base. Run 1's "94%" is thereby **supported as a slope, not asserted**:
~9 ns per shifted record explains the insert/substitute delta.

Remaining attribution gap (honest): `blank_delete@BOF@1MB` examines all
13 658 blocks (`blocks_examined`), a second O(B) component the shift
slope does not cover; and no phase timers were added this run (§4 makes
structural counters primary; timers would intrude on the product hot
path and are deferred until an intervention experiment needs them).

## Prediction → observed deltas worth recording

| case | run-1 prediction | observed syntax | note |
|---|---|---|---|
| blank_delete | I2 | S0@BOF / S3@mid | I2 splits exactly as run 1 suspected |
| fence_info_edit | I1 | S3_NEIGHBOR_LOCAL | info change rewrites opener + neighbor |
| mermaid_body_char | I6 | S2 + MERMAID_CANDIDATE | markdown layer == fence body; provider axis separate |
| ref_def_edit / ref_user_edit | I5 | S3 + D_UNKNOWN | "semantic-global" prediction leaves the syntax axis entirely |
| fence_* family | I4 | S5_STATE_PROPAGATING | confirmed, renamed to mechanism language |

The I-axis survives only as *predictions* (`pred` column); observed
truth lives in the S/D/M/P axes.

## Full tables

Machine-generated tables follow (identical content to
`results/raw/parser-survey/run-1.1-corrective/summary.md`).

# parser-survey run summary

scenarios measured: 397 (skipped: 13)

ORACLE-A (SELF_EQUIVALENCE) incremental == same-parser clean rebuild: 397/397 pass

ORACLE-C (LOSSLESSNESS) full byte coverage, whitespace-only gaps: 397/397 pass

## T0 — influence vectors at synth-1m (mid position; observed classes)

| case | pred(I) | syntax | semantic | representation | Psem_blk | Pcoord_rec+inline | provider |
|---|---|---|---|---|---:|---:|---|
| blank_delete | I2 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 1 | 6809+3404 | UNKNOWN |
| blank_insert | I2 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 1 | 6810+3405 | UNKNOWN |
| codespan_delim_insert | I1 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13656+6837 | UNKNOWN |
| emphasis_close_completion | I1 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13654+6829 | UNKNOWN |
| emphasis_delim_insert | I1 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13656+6837 | UNKNOWN |
| fence_body_char | I0 | S2_BLOCK_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13643+6821 | MERMAID_CANDIDATE |
| fence_closer_delete | I4 | S5_STATE_PROPAGATING | D_UNKNOWN | M1_LOCAL_NODE_REWRITE | 1 | 0+0 | UNKNOWN |
| fence_info_edit | I1 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13643+6821 | MERMAID_CANDIDATE |
| fence_len_grow | I4 | S5_STATE_PROPAGATING | D_UNKNOWN | M1_LOCAL_NODE_REWRITE | 2 | 0+0 | MERMAID_CANDIDATE |
| fence_opener_break | I4 | S5_STATE_PROPAGATING | D_UNKNOWN | M1_LOCAL_NODE_REWRITE | 2 | 0+0 | MERMAID_CANDIDATE |
| fence_opener_delete | I4 | S5_STATE_PROPAGATING | D_UNKNOWN | M1_LOCAL_NODE_REWRITE | 2 | 0+0 | MERMAID_CANDIDATE |
| heading_char | I0 | S2_BLOCK_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13658+6846 | UNKNOWN |
| heading_from_para | I2 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 2 | 6809+3404 | UNKNOWN |
| large_delete_mid | - | S2_BLOCK_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 1 | 5447+2723 | UNKNOWN |
| large_paste_1k | - | S2_BLOCK_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 1 | 6809+3404 | UNKNOWN |
| list_indent_add | I3 | S4_CONTAINER_SCOPED | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13650+6824 | UNKNOWN |
| list_marker_add | I3 | S4_CONTAINER_SCOPED | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13650+6824 | UNKNOWN |
| list_marker_remove | I3 | S4_CONTAINER_SCOPED | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13650+6824 | UNKNOWN |
| mermaid_body_char | I6 | S2_BLOCK_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13643+6821 | MERMAID_CANDIDATE |
| para_delete_char | I0 | S2_BLOCK_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 1 | 6809+3404 | UNKNOWN |
| para_insert_char | I0 | S2_BLOCK_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 1 | 6809+3404 | UNKNOWN |
| para_split | I2 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 3 | 6809+3404 | UNKNOWN |
| para_sub_char | I0 | S2_BLOCK_LOCAL | D_UNKNOWN | M1_LOCAL_NODE_REWRITE | 1 | 0+0 | UNKNOWN |
| quote_add | I3 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13647+6821 | UNKNOWN |
| quote_char | I3 | S4_CONTAINER_SCOPED | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13648+6822 | UNKNOWN |
| quote_remove | I3 | S4_CONTAINER_SCOPED | D_UNKNOWN | M5_GLOBAL_REBUILD | 2 | 13647+6821 | UNKNOWN |
| ref_def_add | I5 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M1_LOCAL_NODE_REWRITE | 3 | 0+0 | UNKNOWN |
| ref_def_edit | I5 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13652+6828 | UNKNOWN |
| ref_user_edit | I5 | S3_NEIGHBOR_LOCAL | D_UNKNOWN | M5_GLOBAL_REBUILD | 1 | 13656+6837 | UNKNOWN |
| setext_add | I2 | S2_BLOCK_LOCAL | D_UNKNOWN | M4_SUFFIX_METADATA_REWRITE | 1 | 6809+3404 | UNKNOWN |

Attribution support (NOT primary evidence): inc_us ≈ 0.37µs + 9.0ns × survivor_blocks_shifted over the 48 length-changing-edit rows below (R² = 0.971).

## T1 — taxonomy at synth-1m (mid position)

| case | pred | family | chg_B | inc_us | full_us | R | blocks_re | conv_Δ | surv_shift | rec_moved | proj_blk | D_content | oracleA |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| blank_delete | I2 | BlockBoundary | 1 | 69.5 | 5312.2 | 306 | 1 | 4 | 6809 | 6809 | 1 | 306 | ok |
| blank_insert | I2 | BlockBoundary | 1 | 100.9 | 6442.7 | 155 | 2 | 4 | 6810 | 0 | 1 | 2.0 | ok |
| codespan_delim_insert | I1 | DelimiterEdit | 1 | 251.8 | 6391.8 | 73.0 | 2 | 2 | 13656 | 0 | 1 | 72.0 | ok |
| emphasis_close_completion | I1 | DelimiterEdit | 2 | 173.6 | 6077.0 | 30.5 | 2 | 2 | 13654 | 0 | 1 | 30.0 | ok |
| emphasis_delim_insert | I1 | DelimiterEdit | 1 | 156.1 | 6147.0 | 73.0 | 2 | 2 | 13656 | 0 | 1 | 72.0 | ok |
| fence_body_char | I0 | ContentEdit | 1 | 126.8 | 5827.1 | 35.0 | 1 | 3 | 13643 | 0 | 1 | 35.0 | ok |
| fence_closer_delete | I4 | StatePropagating | 4 | 1472.2 | 1166.0 | 262063 | 1 | 20467 | 0 | 0 | 1 | 262063 | ok |
| fence_info_edit | I1 | StatePropagating | 13 | 137.2 | 5254.4 | 2.6 | 2 | 4 | 13643 | 0 | 1 | 2.5 | ok |
| fence_len_grow | I4 | StatePropagating | 1 | 1938.1 | 1190.7 | 1048257 | 3 | 20469 | 0 | 0 | 2 | 1048256 | ok |
| fence_opener_break | I4 | StatePropagating | 2 | 1521.6 | 1168.9 | 524128 | 3 | 20469 | 0 | 0 | 2 | 524128 | ok |
| fence_opener_delete | I4 | StatePropagating | 11 | 1542.5 | 1171.1 | 95295 | 3 | 20468 | 0 | 0 | 2 | 95295 | ok |
| heading_char | I0 | ContentEdit | 1 | 128.9 | 5802.9 | 24.0 | 1 | 1 | 13658 | 0 | 1 | 24.0 | ok |
| heading_from_para | I2 | BlockBoundary | 3 | 70.8 | 5781.3 | 52.0 | 2 | 2 | 6809 | 6809 | 2 | 52.0 | ok |
| large_delete_mid | - | Stress | 209748 | 198.8 | 4552.5 | 0.0 | 1 | 2 | 5447 | 5447 | 1 | 0.0 | ok |
| large_paste_1k | - | Stress | 52890 | 192.6 | 5982.1 | 1.0 | 1 | 1002 | 6809 | 0 | 1 | 1.0 | ok |
| list_indent_add | I3 | Container | 2 | 128.2 | 5853.8 | 47.0 | 1 | 5 | 13650 | 0 | 1 | 47.0 | ok |
| list_marker_add | I3 | Container | 2 | 132.2 | 5947.9 | 47.0 | 1 | 5 | 13650 | 0 | 1 | 47.0 | ok |
| list_marker_remove | I3 | Container | 2 | 113.8 | 5163.3 | 45.0 | 1 | 5 | 13650 | 0 | 1 | 45.0 | ok |
| mermaid_body_char | I6 | ContentEdit | 1 | 137.8 | 6016.1 | 35.0 | 1 | 3 | 13643 | 0 | 1 | 35.0 | ok |
| para_delete_char | I0 | ContentEdit | 1 | 48.8 | 6309.7 | 152 | 1 | 2 | 6809 | 0 | 1 | 152 | ok |
| para_insert_char | I0 | ContentEdit | 1 | 59.5 | 6109.4 | 154 | 1 | 2 | 6809 | 0 | 1 | 154 | ok |
| para_split | I2 | BlockBoundary | 3 | 73.3 | 5891.5 | 51.3 | 3 | 4 | 6809 | 6809 | 3 | 51.3 | ok |
| para_sub_char | I0 | ContentEdit | 2 | 2.4 | 6377.6 | 76.5 | 1 | 2 | 0 | 0 | 1 | 76.5 | ok |
| quote_add | I3 | Container | 2 | 149.9 | 5786.0 | 31.5 | 1 | 3 | 13647 | 13647 | 1 | 31.5 | ok |
| quote_char | I3 | Container | 1 | 119.5 | 5842.3 | 38.0 | 1 | 2 | 13648 | 0 | 1 | 38.0 | ok |
| quote_remove | I3 | Container | 2 | 113.4 | 5280.2 | 29.5 | 2 | 3 | 13647 | 0 | 2 | 29.5 | ok |
| ref_def_add | I5 | SemanticGlobal | 11 | 2.9 | 5679.5 | 1.1 | 3 | 4 | 0 | 0 | 3 | 1.1 | ok |
| ref_def_edit | I5 | SemanticGlobal | 1 | 155.3 | 5908.0 | 30.0 | 2 | 2 | 13652 | 0 | 1 | 29.0 | ok |
| ref_user_edit | I5 | SemanticGlobal | 9 | 129.3 | 5789.3 | 8.1 | 2 | 2 | 13656 | 0 | 1 | 8.0 | ok |
| setext_add | I2 | BlockBoundary | 4 | 51.0 | 5860.2 | 39.2 | 1 | 3 | 6809 | 0 | 1 | 39.2 | ok |

## T2 — hidden-O(N) gate (offset rewrites vs parse radius)

| case | corpus | pos | R | survivor_shifted | records_moved | proj_bytes | inc_us |
|---|---|---|---:|---:|---:|---:|---:|
| bof_insert_char | synth-100k | bof | 24.0 | 1362 | 0 | 24 | 12.2 |
| eof_append_char | synth-100k | eof | 2.0 | 0 | 0 | 1 | 1.3 |
| para_delete_char | synth-100k | bof | 71.0 | 1360 | 0 | 70 | 13.0 |
| para_delete_char | synth-100k | eof | 150 | 1 | 0 | 150 | 1.4 |
| para_delete_char | synth-100k | mid | 151 | 673 | 0 | 150 | 6.2 |
| para_delete_char | synth-100k | q1 | 150 | 1011 | 0 | 150 | 8.4 |
| para_delete_char | synth-100k | q3 | 150 | 337 | 0 | 150 | 3.7 |
| para_insert_char | synth-100k | bof | 73.0 | 1360 | 0 | 72 | 12.6 |
| para_insert_char | synth-100k | eof | 152 | 1 | 0 | 152 | 1.5 |
| para_insert_char | synth-100k | mid | 153 | 673 | 0 | 152 | 6.2 |
| para_insert_char | synth-100k | q1 | 152 | 1011 | 0 | 152 | 8.5 |
| para_insert_char | synth-100k | q3 | 152 | 337 | 0 | 152 | 3.8 |
| bof_insert_char | synth-10k | bof | 24.0 | 148 | 0 | 24 | 1.8 |
| eof_append_char | synth-10k | eof | 2.0 | 0 | 0 | 1 | 0.7 |
| para_delete_char | synth-10k | bof | 71.0 | 146 | 0 | 70 | 3.0 |
| para_delete_char | synth-10k | eof | 148 | 1 | 0 | 148 | 0.9 |
| para_delete_char | synth-10k | mid | 148 | 69 | 0 | 148 | 1.4 |
| para_delete_char | synth-10k | q1 | 148 | 103 | 0 | 148 | 1.6 |
| para_delete_char | synth-10k | q3 | 149 | 33 | 0 | 148 | 1.4 |
| para_insert_char | synth-10k | bof | 73.0 | 146 | 0 | 72 | 2.9 |
| para_insert_char | synth-10k | eof | 150 | 1 | 0 | 150 | 1.0 |
| para_insert_char | synth-10k | mid | 150 | 69 | 0 | 150 | 1.5 |
| para_insert_char | synth-10k | q1 | 150 | 103 | 0 | 150 | 1.7 |
| para_insert_char | synth-10k | q3 | 151 | 33 | 0 | 150 | 1.4 |
| bof_insert_char | synth-1k | bof | 24.0 | 26 | 0 | 24 | 0.8 |
| eof_append_char | synth-1k | eof | 2.0 | 0 | 0 | 1 | 0.5 |
| para_delete_char | synth-1k | bof | 71.0 | 24 | 0 | 70 | 1.6 |
| para_delete_char | synth-1k | eof | 146 | 1 | 0 | 146 | 0.9 |
| para_delete_char | synth-1k | mid | 147 | 7 | 0 | 146 | 1.0 |
| para_delete_char | synth-1k | q1 | 60.0 | 15 | 0 | 23 | 1.1 |
| para_delete_char | synth-1k | q3 | 147 | 3 | 0 | 146 | 0.9 |
| para_insert_char | synth-1k | bof | 73.0 | 24 | 0 | 72 | 1.6 |
| para_insert_char | synth-1k | eof | 148 | 1 | 0 | 148 | 0.8 |
| para_insert_char | synth-1k | mid | 149 | 7 | 0 | 148 | 1.0 |
| para_insert_char | synth-1k | q1 | 62.0 | 15 | 0 | 25 | 1.2 |
| para_insert_char | synth-1k | q3 | 149 | 3 | 0 | 148 | 1.0 |
| bof_insert_char | synth-1m | bof | 24.0 | 13658 | 0 | 24 | 136.9 |
| eof_append_char | synth-1m | eof | 2.0 | 0 | 0 | 1 | 2.7 |
| para_delete_char | synth-1m | bof | 71.0 | 13656 | 0 | 70 | 152.5 |
| para_delete_char | synth-1m | eof | 152 | 1 | 0 | 152 | 2.9 |
| para_delete_char | synth-1m | mid | 152 | 6809 | 0 | 152 | 48.8 |
| para_delete_char | synth-1m | q1 | 153 | 10213 | 0 | 152 | 85.0 |
| para_delete_char | synth-1m | q3 | 152 | 3405 | 0 | 152 | 24.2 |
| para_insert_char | synth-1m | bof | 73.0 | 13656 | 0 | 72 | 109.6 |
| para_insert_char | synth-1m | eof | 154 | 1 | 0 | 154 | 4.0 |
| para_insert_char | synth-1m | mid | 154 | 6809 | 0 | 154 | 59.5 |
| para_insert_char | synth-1m | q1 | 155 | 10213 | 0 | 154 | 78.0 |
| para_insert_char | synth-1m | q3 | 154 | 3405 | 0 | 154 | 25.2 |

## T3 — adversarial & encoding-variant corpora

| corpus | case | pred | chg_B | R | blocks_re | conv_Δ | surv_shift | rec_moved | inc_us | D_content | oracleA | losslessC |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| adv-crlf-mixed | blank_delete | I2 | 2 | 22.0 | 1 | 2 | 3 | 3 | 0.7 | 22.0 | ok | ok |
| adv-crlf-mixed | para_insert_char | I0 | 1 | 26.0 | 2 | 2 | 3 | 0 | 0.7 | 24.0 | ok | ok |
| adv-deep-quote | quote_char | I3 | 1 | 42093 | 1 | 200 | 2 | 0 | 137.5 | 42093 | ok | ok |
| adv-deep-quote | quote_remove | I3 | 2 | 21045 | 1 | 200 | 2 | 0 | 134.4 | 21045 | ok | ok |
| adv-duplicate-refs | ref_def_edit | I5 | 1 | 13.0 | 1 | 1 | 7 | 0 | 0.7 | 13.0 | ok | ok |
| adv-emoji | eof_append_char | - | 1 | 25.0 | 1 | 2 | 0 | 0 | 0.5 | 25.0 | ok | ok |
| adv-emoji | para_insert_char | I0 | 1 | 114 | 2 | 2 | 9 | 0 | 0.8 | 113 | ok | ok |
| adv-fence-near-bof | fence_body_char | I0 | 1 | 37.0 | 1 | 4 | 3 | 0 | 0.5 | 37.0 | ok | ok |
| adv-fence-near-bof | fence_closer_delete | I4 | 4 | 12.2 | 1 | 6 | 0 | 0 | 0.5 | 12.2 | ok | ok |
| adv-half-written | emphasis_close_completion | I1 | 2 | 11.5 | 2 | 2 | 7 | 0 | 0.8 | 11.0 | ok | ok |
| adv-huge-paragraph | para_insert_char | I0 | 1 | 100003 | 2 | 2 | 3 | 0 | 232.4 | 100002 | ok | ok |
| adv-lazy-continuation | para_insert_char | I0 | 1 | 44.0 | 2 | 2 | 3 | 0 | 1.5 | 26.0 | ok | ok |
| adv-many-refs | ref_def_edit | I5 | 1 | 16788 | 2 | 1001 | 1 | 0 | 56.3 | 16787 | ok | ok |
| adv-unclosed-fence | fence_body_char | I0 | 1 | 571 | 1 | 54 | 0 | 0 | 1.4 | 571 | ok | ok |
| synth-cjk | blank_delete | I2 | 1 | 0.0 | 0 | 0 | 135 | 135 | 1.4 | 0.0 | ok | ok |
| synth-cjk | blank_delete | I2 | 1 | 328 | 1 | 4 | 93 | 93 | 2.3 | 328 | ok | ok |
| synth-cjk | blank_delete | I2 | 1 | 328 | 1 | 4 | 61 | 61 | 1.9 | 328 | ok | ok |
| synth-cjk | blank_delete | I2 | 1 | 328 | 1 | 4 | 31 | 31 | 1.7 | 328 | ok | ok |
| synth-cjk | blank_insert | I2 | 1 | 2.0 | 1 | 2 | 135 | 0 | 1.4 | 2.0 | ok | ok |
| synth-cjk | blank_insert | I2 | 1 | 166 | 2 | 4 | 94 | 0 | 1.9 | 2.0 | ok | ok |
| synth-cjk | blank_insert | I2 | 1 | 166 | 2 | 4 | 62 | 0 | 1.6 | 2.0 | ok | ok |
| synth-cjk | blank_insert | I2 | 1 | 166 | 2 | 4 | 32 | 0 | 1.4 | 2.0 | ok | ok |
| synth-cjk | bof_delete_char | - | 1 | 20.0 | 1 | 1 | 136 | 0 | 1.6 | 20.0 | ok | ok |
| synth-cjk | bof_insert_char | - | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.6 | 22.0 | ok | ok |
| synth-cjk | bof_insert_newline | - | 1 | 22.0 | 2 | 2 | 136 | 136 | 2.0 | 1.0 | ok | ok |
| synth-cjk | codespan_delim_insert | I1 | 1 | 76.0 | 2 | 2 | 134 | 0 | 2.6 | 75.0 | ok | ok |
| synth-cjk | emphasis_close_completion | I1 | 2 | 31.5 | 2 | 2 | 132 | 0 | 2.5 | 31.0 | ok | ok |
| synth-cjk | emphasis_delim_insert | I1 | 1 | 76.0 | 2 | 2 | 134 | 0 | 2.7 | 75.0 | ok | ok |
| synth-cjk | eof_append_char | - | 1 | 2.0 | 2 | 2 | 0 | 0 | 0.7 | 1.0 | ok | ok |
| synth-cjk | fence_body_char | I0 | 1 | 39.0 | 1 | 3 | 123 | 0 | 1.3 | 39.0 | ok | ok |
| synth-cjk | fence_body_char | I0 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.3 | 35.0 | ok | ok |
| synth-cjk | fence_body_char | I0 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.2 | 35.0 | ok | ok |
| synth-cjk | fence_body_char | I0 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.2 | 35.0 | ok | ok |
| synth-cjk | fence_body_char | I0 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.3 | 35.0 | ok | ok |
| synth-cjk | fence_closer_delete | I4 | 4 | 2476 | 1 | 184 | 0 | 0 | 13.5 | 2476 | ok | ok |
| synth-cjk | fence_info_edit | I1 | 13 | 2.6 | 2 | 4 | 121 | 0 | 1.3 | 2.5 | ok | ok |
| synth-cjk | fence_len_grow | I4 | 1 | 9910 | 3 | 186 | 0 | 0 | 14.6 | 9909 | ok | ok |
| synth-cjk | fence_opener_break | I4 | 2 | 4954 | 3 | 186 | 0 | 0 | 14.7 | 4954 | ok | ok |
| synth-cjk | fence_opener_delete | I4 | 11 | 900 | 3 | 185 | 0 | 0 | 14.4 | 900 | ok | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.7 | 22.0 | ok | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.6 | 22.0 | ok | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.7 | 22.0 | ok | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.6 | 22.0 | ok | ok |
| synth-cjk | heading_char | I0 | 1 | 22.0 | 1 | 1 | 136 | 0 | 1.6 | 22.0 | ok | ok |
| synth-cjk | heading_from_para | I2 | 3 | 56.0 | 3 | 3 | 61 | 61 | 1.9 | 55.7 | ok | ok |
| synth-cjk | large_delete_mid | - | 2059 | 0.1 | 1 | 3 | 49 | 49 | 1.8 | 0.1 | ok | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 145.3 | 1.0 | ok | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 144.5 | 1.0 | ok | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 144.5 | 1.0 | ok | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 143.4 | 1.0 | ok | ok |
| synth-cjk | large_paste_1k | - | 52890 | 1.0 | 2 | 1003 | 61 | 0 | 143.1 | 1.0 | ok | ok |
| synth-cjk | list_indent_add | I3 | 2 | 46.0 | 1 | 5 | 128 | 0 | 2.4 | 46.0 | ok | ok |
| synth-cjk | list_marker_add | I3 | 2 | 46.0 | 1 | 5 | 128 | 0 | 2.4 | 46.0 | ok | ok |
| synth-cjk | list_marker_remove | I3 | 2 | 44.0 | 1 | 5 | 128 | 0 | 2.2 | 44.0 | ok | ok |
| synth-cjk | mermaid_body_char | I6 | 1 | 35.0 | 1 | 3 | 121 | 0 | 1.3 | 35.0 | ok | ok |
| synth-cjk | para_delete_char | I0 | 1 | 74.0 | 2 | 2 | 134 | 0 | 2.8 | 73.0 | ok | ok |
| synth-cjk | para_delete_char | I0 | 1 | 164 | 2 | 3 | 93 | 0 | 1.9 | 163 | ok | ok |
| synth-cjk | para_delete_char | I0 | 1 | 164 | 2 | 3 | 61 | 0 | 1.6 | 163 | ok | ok |
| synth-cjk | para_delete_char | I0 | 1 | 163 | 1 | 2 | 31 | 0 | 1.2 | 163 | ok | ok |
| synth-cjk | para_delete_char | I0 | 1 | 163 | 1 | 2 | 1 | 0 | 1.0 | 163 | ok | ok |
| synth-cjk | para_insert_char | I0 | 1 | 76.0 | 2 | 2 | 134 | 0 | 2.8 | 75.0 | ok | ok |
| synth-cjk | para_insert_char | I0 | 1 | 166 | 2 | 3 | 93 | 0 | 1.9 | 165 | ok | ok |
| synth-cjk | para_insert_char | I0 | 1 | 166 | 2 | 3 | 61 | 0 | 1.7 | 165 | ok | ok |
| synth-cjk | para_insert_char | I0 | 1 | 165 | 1 | 2 | 31 | 0 | 1.2 | 165 | ok | ok |
| synth-cjk | para_insert_char | I0 | 1 | 165 | 1 | 2 | 1 | 0 | 1.0 | 165 | ok | ok |
| synth-cjk | para_split | I2 | 2 | 83.5 | 4 | 5 | 61 | 61 | 2.0 | 83.0 | ok | ok |
| synth-cjk | para_sub_char | I0 | 2 | 37.5 | 2 | 2 | 0 | 0 | 1.8 | 37.0 | ok | ok |
| synth-cjk | para_sub_char | I0 | 2 | 82.5 | 2 | 3 | 0 | 0 | 1.3 | 82.0 | ok | ok |
| synth-cjk | para_sub_char | I0 | 2 | 82.5 | 2 | 3 | 0 | 0 | 1.2 | 82.0 | ok | ok |
| synth-cjk | para_sub_char | I0 | 2 | 82.0 | 1 | 2 | 0 | 0 | 1.0 | 82.0 | ok | ok |
| synth-cjk | para_sub_char | I0 | 2 | 82.0 | 1 | 2 | 0 | 0 | 1.0 | 82.0 | ok | ok |
| synth-cjk | quote_add | I3 | 2 | 30.0 | 1 | 3 | 125 | 125 | 2.2 | 30.0 | ok | ok |
| synth-cjk | quote_char | I3 | 1 | 37.0 | 1 | 2 | 126 | 0 | 1.9 | 37.0 | ok | ok |
| synth-cjk | quote_remove | I3 | 2 | 28.0 | 2 | 3 | 125 | 0 | 2.0 | 28.0 | ok | ok |
| synth-cjk | ref_def_add | I5 | 11 | 1.1 | 3 | 4 | 0 | 0 | 0.7 | 1.1 | ok | ok |
| synth-cjk | ref_def_edit | I5 | 1 | 30.0 | 2 | 2 | 130 | 0 | 1.8 | 29.0 | ok | ok |
| synth-cjk | setext_add | I2 | 4 | 42.2 | 2 | 4 | 61 | 0 | 1.7 | 42.0 | ok | ok |
| synth-crlf | blank_delete | I2 | 2 | 0.0 | 0 | 0 | 147 | 147 | 1.5 | 0.0 | ok | ok |
| synth-crlf | blank_delete | I2 | 2 | 151 | 1 | 4 | 103 | 103 | 2.2 | 151 | ok | ok |
| synth-crlf | blank_delete | I2 | 2 | 151 | 1 | 4 | 67 | 67 | 1.9 | 151 | ok | ok |
| synth-crlf | blank_delete | I2 | 2 | 151 | 1 | 4 | 33 | 33 | 1.6 | 151 | ok | ok |
| synth-crlf | blank_insert | I2 | 1 | 3.0 | 1 | 2 | 147 | 0 | 1.5 | 3.0 | ok | ok |
| synth-crlf | blank_insert | I2 | 1 | 154 | 2 | 4 | 104 | 0 | 1.8 | 3.0 | ok | ok |
| synth-crlf | blank_insert | I2 | 1 | 154 | 2 | 4 | 68 | 0 | 1.5 | 3.0 | ok | ok |
| synth-crlf | blank_insert | I2 | 1 | 154 | 2 | 4 | 34 | 0 | 1.3 | 3.0 | ok | ok |
| synth-crlf | bof_delete_char | - | 1 | 23.0 | 1 | 1 | 148 | 0 | 1.6 | 23.0 | ok | ok |
| synth-crlf | bof_insert_char | - | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.7 | 25.0 | ok | ok |
| synth-crlf | bof_insert_newline | - | 1 | 25.0 | 2 | 2 | 148 | 148 | 1.9 | 1.0 | ok | ok |
| synth-crlf | codespan_delim_insert | I1 | 1 | 75.0 | 2 | 2 | 146 | 0 | 2.5 | 73.0 | ok | ok |
| synth-crlf | emphasis_close_completion | I1 | 2 | 31.5 | 2 | 2 | 144 | 0 | 2.4 | 30.5 | ok | ok |
| synth-crlf | emphasis_delim_insert | I1 | 1 | 75.0 | 2 | 2 | 146 | 0 | 2.6 | 73.0 | ok | ok |
| synth-crlf | eof_append_char | - | 1 | 3.0 | 2 | 2 | 0 | 0 | 0.5 | 1.0 | ok | ok |
| synth-crlf | fence_body_char | I0 | 1 | 42.0 | 1 | 3 | 135 | 0 | 1.4 | 42.0 | ok | ok |
| synth-crlf | fence_body_char | I0 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.4 | 38.0 | ok | ok |
| synth-crlf | fence_body_char | I0 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.4 | 38.0 | ok | ok |
| synth-crlf | fence_body_char | I0 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.4 | 38.0 | ok | ok |
| synth-crlf | fence_body_char | I0 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.3 | 38.0 | ok | ok |
| synth-crlf | fence_closer_delete | I4 | 5 | 2023 | 1 | 202 | 0 | 0 | 13.2 | 2023 | ok | ok |
| synth-crlf | fence_info_edit | I1 | 13 | 2.9 | 2 | 4 | 133 | 0 | 1.4 | 2.8 | ok | ok |
| synth-crlf | fence_len_grow | I4 | 1 | 10122 | 3 | 204 | 0 | 0 | 14.3 | 10120 | ok | ok |
| synth-crlf | fence_opener_break | I4 | 2 | 5060 | 3 | 204 | 0 | 0 | 14.5 | 5060 | ok | ok |
| synth-crlf | fence_opener_delete | I4 | 12 | 842 | 3 | 203 | 0 | 0 | 14.4 | 842 | ok | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.7 | 25.0 | ok | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.7 | 25.0 | ok | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.7 | 25.0 | ok | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.7 | 25.0 | ok | ok |
| synth-crlf | heading_char | I0 | 1 | 25.0 | 1 | 1 | 148 | 0 | 1.7 | 25.0 | ok | ok |
| synth-crlf | heading_from_para | I2 | 3 | 51.3 | 2 | 2 | 69 | 69 | 1.7 | 51.3 | ok | ok |
| synth-crlf | large_delete_mid | - | 2142 | 0.1 | 1 | 2 | 55 | 55 | 1.6 | 0.1 | ok | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 141.2 | 1.0 | ok | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 139.4 | 1.0 | ok | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 141.0 | 1.0 | ok | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 140.1 | 1.0 | ok | ok |
| synth-crlf | large_paste_1k | - | 52890 | 1.0 | 1 | 1002 | 69 | 0 | 138.4 | 1.0 | ok | ok |
| synth-crlf | list_indent_add | I3 | 2 | 49.5 | 1 | 5 | 140 | 0 | 2.2 | 49.5 | ok | ok |
| synth-crlf | list_marker_add | I3 | 2 | 49.5 | 1 | 5 | 140 | 0 | 2.4 | 49.5 | ok | ok |
| synth-crlf | list_marker_remove | I3 | 2 | 47.5 | 1 | 5 | 140 | 0 | 2.1 | 47.5 | ok | ok |
| synth-crlf | mermaid_body_char | I6 | 1 | 38.0 | 1 | 3 | 133 | 0 | 1.3 | 38.0 | ok | ok |
| synth-crlf | para_delete_char | I0 | 1 | 73.0 | 2 | 2 | 146 | 0 | 2.7 | 71.0 | ok | ok |
| synth-crlf | para_delete_char | I0 | 1 | 150 | 1 | 2 | 103 | 0 | 1.6 | 150 | ok | ok |
| synth-crlf | para_delete_char | I0 | 1 | 150 | 1 | 2 | 69 | 0 | 1.4 | 150 | ok | ok |
| synth-crlf | para_delete_char | I0 | 1 | 152 | 2 | 3 | 33 | 0 | 1.2 | 150 | ok | ok |
| synth-crlf | para_delete_char | I0 | 1 | 150 | 1 | 2 | 1 | 0 | 0.9 | 150 | ok | ok |
| synth-crlf | para_insert_char | I0 | 1 | 75.0 | 2 | 2 | 146 | 0 | 2.6 | 73.0 | ok | ok |
| synth-crlf | para_insert_char | I0 | 1 | 152 | 1 | 2 | 103 | 0 | 1.7 | 152 | ok | ok |
| synth-crlf | para_insert_char | I0 | 1 | 152 | 1 | 2 | 69 | 0 | 1.4 | 152 | ok | ok |
| synth-crlf | para_insert_char | I0 | 1 | 154 | 2 | 3 | 33 | 0 | 1.3 | 152 | ok | ok |
| synth-crlf | para_insert_char | I0 | 1 | 152 | 1 | 2 | 1 | 0 | 1.0 | 152 | ok | ok |
| synth-crlf | para_split | I2 | 3 | 50.7 | 3 | 4 | 69 | 69 | 1.8 | 50.7 | ok | ok |
| synth-crlf | para_sub_char | I0 | 2 | 37.0 | 2 | 2 | 0 | 0 | 1.6 | 36.0 | ok | ok |
| synth-crlf | para_sub_char | I0 | 2 | 75.5 | 1 | 2 | 0 | 0 | 0.9 | 75.5 | ok | ok |
| synth-crlf | para_sub_char | I0 | 2 | 75.5 | 1 | 2 | 0 | 0 | 0.9 | 75.5 | ok | ok |
| synth-crlf | para_sub_char | I0 | 2 | 76.5 | 2 | 3 | 0 | 0 | 1.0 | 75.5 | ok | ok |
| synth-crlf | para_sub_char | I0 | 2 | 75.5 | 1 | 2 | 0 | 0 | 0.9 | 75.5 | ok | ok |
| synth-crlf | quote_add | I3 | 2 | 33.0 | 1 | 3 | 137 | 137 | 2.2 | 33.0 | ok | ok |
| synth-crlf | quote_char | I3 | 1 | 40.0 | 1 | 2 | 138 | 0 | 1.7 | 40.0 | ok | ok |
| synth-crlf | quote_remove | I3 | 2 | 31.0 | 2 | 3 | 137 | 0 | 1.9 | 31.0 | ok | ok |
| synth-crlf | ref_def_add | I5 | 11 | 1.2 | 3 | 4 | 0 | 0 | 0.6 | 1.2 | ok | ok |
| synth-crlf | ref_def_edit | I5 | 1 | 32.0 | 2 | 2 | 142 | 0 | 1.7 | 30.0 | ok | ok |
| synth-crlf | ref_user_edit | I5 | 9 | 8.3 | 2 | 2 | 146 | 0 | 2.5 | 8.1 | ok | ok |
| synth-crlf | setext_add | I2 | 4 | 38.8 | 1 | 3 | 69 | 0 | 1.5 | 38.8 | ok | ok |

## Column notes

- `R` = bytes_scanned / changed_bytes (reparse amplification).
- `D_content` = proj_bytes_invalidated / changed_bytes: how much source a
position-keyed downstream consumer must re-project, ignoring pure offset
shifts (those appear as `surv_shift` / `rec_moved`).
- `conv_Δ` = convergence_line − restart_line (propagation distance, lines).
- `surv_shift` = survivor_blocks_shifted, `rec_moved` = block_records_moved
(the hidden-O(N) gate: metadata work that parsing locality alone hides).
- Influence classes (T0) are derived ONLY from observed counters of the same
row: S0 needs blocks_reparsed=0 (a ZERO_REPARSE_CONVERGENCE — zero parse
work, NOT zero metadata work); S5/S6 split by restart position on ≥half-doc
scans; M4 needs survivor shifts ≥ half the suffix-proportional expectation.
Semantic is D_UNKNOWN for every row: no semantic dependency layer exists in
this run. See crates/parser-survey/src/influence.rs for the full rules.
