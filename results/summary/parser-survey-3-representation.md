# RUN-3 — M1 green-tree representation prototype (plan §13–22)

Issue: #19, branch `exp/19-parser-survey-1`. Research-only prototypes:
`crates/parser-survey/src/green.rs` (data structures) +
`greenbench.rs` (`--green` mode). Zero product code touched.
Raw: `results/raw/parser-survey/run-3-representation/green/` (local).

## Question (H4/Q4)

Does a position-free persistent syntax representation eliminate the
suffix absolute-offset rewrite that runs 1/1.1 measured
(`M4_SUFFIX_METADATA_REWRITE`, fit ≈ 9.0 ns/record)?

## Scope discipline

Green nodes carry only block kind + byte length — no offsets, no
parent pointers, no line numbers (plan §14). Parse work is common to
every representation and measured in runs 1/1.1; these numbers are
representation maintenance only, verified by a length/count invariant
after every operation. Every claim below pairs a structural counter
with wall-clock per column; nothing is merged into one ranking.

## T-G1 — the H4 experiment (length-changing 1-byte edit)

| corpus | pos | B | markit inc_us | markit survivors | G1 us | G1 ptrs | G2 us | G2 ancestors | coord_rw |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| synth-10k | bof | 149 | 1.8 | 148 | 0.351 | 149 | 0.130 | 7 | 0 |
| synth-100k | mid | 1363 | 5.6 | 675 | 3.747 | 1363 | 0.151 | 10 | 0 |
| synth-100k | bof | 1363 | 10.4 | 1362 | 3.787 | 1363 | 0.231 | 10 | 0 |
| synth-1m | bof | 13 659 | 132.9 | 13 658 | 38.282 | 13 659 | **0.180** | **13** | **0** |
| synth-1m | mid | 13 659 | 62.3 | 6 809 | 40.086 | 13 659 | **0.191** | 14 | 0 |
| synth-1m | eof | 13 659 | 3.4 | 0 | 39.966 | 13 659 | 0.191 | 14 | 0 |

- **G2 (balanced persistent sequence) edit cost is flat in N**: ~0.18 µs
  at 1 MB with 13–14 ancestors path-copied ≈ log2 B, and
  `absolute_coordinate_records_rewritten = 0` by construction. Against
  markit's bof row: **~700× less representation work**, and the
  survivor-record term the run-1.1 fit priced at 9.0 ns/record is gone
  entirely.
- **G1 (naive position-free Vec) isolates the second O(N)**: with no
  coordinates to rewrite, the root Vec clone still costs O(B)
  (38–40 µs at 1 MB). This is the plan §15 warning measured: killing
  the offset rewrite alone leaves the sequence-movement cost.
- Both green variants stay ~O(1)/O(log B) at eof, matching markit there
  (its suffix is empty).

## T-G5 — red view / position queries (plan §17: don't make reads unacceptable)

Per-query ns over 2 000 uniform random offsets:

| corpus | G1 (O(B) scan) | G2 red view (O(log B)) | markit `block_at_offset` |
|---|---:|---:|---:|
| synth-10k | 28.5 | 38.2 | 10.4 |
| synth-100k | 293.8 | 71.6 | 21.1 |
| synth-1m | **3 403** | **129.7** | 194.2 |

- G1's read path is the proof that naive position-freeness is not
  viable: 3.4 µs per position query at 1 MB.
- G2's lazy red view is ~2× markit's flat binary search at 10k–100k and
  *comparable* at 1 MB (130 vs 194 ns). Position queries without stored
  positions are viable at product-like scales. (A balanced tree with
  wider fanout, or a periodically memoized offset index, would close
  the small-N gap; not prototyped here.)

## T-G2 — huge paragraph granularity (§21)

| representation | mid +1B edit | nodes | red query ns |
|---|---:|---:|---:|
| one flat leaf | 0.211 µs | 1 | — |
| 64 B chunks in G2 | 0.561 µs | 1 600 | 61.5 (10 ancestors) |

Representation-side, both are microseconds — which sharpens, rather
than settles, §21: for the *representation* the flat leaf is fine; the
100 KB cost markit pays (R ≈ 100 003) is a *parse-scope* cost. The
chunked layout's value is enabling parse-scope locality (re-parse one
chunk), bought with ~1 600 nodes + slower queries for that block.
Not an automatic win (plan §21 says exactly this); the decision needs
parse-side measurements that only exist for the flat case today.

## T-G3 — recursive container (§22, 200-deep quote)

| representation | inner edit | structure cost |
|---|---:|---|
| nested chain, 200 levels | 17.653 µs | 200 ancestors copied |
| one flat leaf | 0.071 µs | 0 ancestors; parse scope stays 42 KB (E3) |

The nested chain bounds *future parse scope* to the innermost block at
O(depth) representation cost (17.7 µs at depth 200 — allocation-bound;
a chunked/fanout container would cut the constant). markit's flat L1
parses all 42 KB today (R ≈ 42 093, 131 µs). Container-awareness is
therefore worth between 0 and ~8× on this shape depending on how much
the parse savings count — a mechanism trade, now quantified from both
sides.

## T-G4 — suffix-absorbing growth (fence-cascade shape)

`g2_truncate` at the fence block: **0.481 µs, 13 ancestors copied**
(subtrees fully kept are shared, not copied). markit's same shape
(fence_len_grow, run-1.1) pays R ≈ 1 048 257 with the O(B) metadata
tail on top of ~2 ms parse.

## Memory accounting (prototype estimate)

Per-block granularity at the 1 MB corpus: leaves ≈ 13 659 × 32 B
(Arc<GreenLeaf>) + inner nodes ≈ 13 658 × 48 B ≈ **1.1 MB ≈ 1.05× the
document bytes**. Chunked leaves or wider fanout change the constant;
this is the honest price tag of the H4 candidate at L1 granularity.

## H4 verdict from this run

**SUPPORTED_WITH_SCOPE (prototype level)** — position-free green
representation + balanced persistent sequence eliminates the measured
suffix metadata rewrite (flat-in-N edits, coord_rewrites ≡ 0) with a
read path comparable to the product's existing query. Scope: (a) parse
work is not included and is common to all representations; (b) memory
≈ 1× document bytes at per-block granularity; (c) multi-edit
transactions and identity/inline-IR placement are not prototyped yet
(RUN-1.1's multi-edit dimension also still open on the markit side).

## Full tables

See `results/raw/parser-survey/run-3-representation/green/summary.md`
(machine-generated, same content).
