# RUN-3 — M1 green-tree representation prototype (plan §13–22;
# CORRECTIVE-1 extended)

Issue: #19, branch `exp/19-parser-survey-1`. Research-only prototypes:
`crates/parser-survey/src/green.rs` (data structures) +
`greenbench.rs` (`--green` mode, `--green-history` gate). Zero product
code touched.
Raw: `results/raw/parser-survey/run-3-representation/green/`
(superseded),
`results/raw/parser-survey/corrective-1/green-history/` (history gate).

> **CORRECTIVE-1 (2026-09-16) — naming and claim scope per review
> MAJOR-2.** The original report called G2 a "weight-balanced persistent
> sequence", but the update path is pure path-copy with NO rebalance,
> and every T-G1..T-G4 row measured ONE edit starting from the initially
> balanced root. What run-3 actually proved: *a position-free persistent
> sequence measured from a balanced initial tree keeps single-edit
> locality*. It proved nothing about long-lived balance. The
> G2-HISTORY-STABILITY gate below supplies that missing evidence. H4's
> core (coord_rewrites = 0, flat-in-N single edits) stands.

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
document bytes**. Scope (CORRECTIVE-1 wording): this is **green-tree
representation bytes only** — the green nodes store kind + length, not
source text. Total editor state is at least source storage + green
representation + inline data + semantic indexes + allocator overhead,
and the prototype estimate above does not fully count Arc allocation
headers or enum layout padding. Memory budgets derived from this number
must re-derive against the whole-state inventory.

## H4 verdict from this run

**SUPPORTED_WITH_SCOPE (prototype level)** — position-free green
representation eliminates the measured suffix metadata rewrite
(flat-in-N single edits, coord_rewrites ≡ 0) with a read path comparable
to the product's existing query. Scope: (a) parse work is not included
and is common to all representations; (b) green representation bytes ≈
1× document bytes at per-block granularity (whole-state inventory not
taken); (c) long-lived balance was NOT proven by the original battery —
see the history gate below; (d) multi-edit transactions and
identity/inline-IR placement are not prototyped yet.

---

# CORRECTIVE-1 B — G2-HISTORY-STABILITY gate (review MAJOR-2)

Question: does the naive path-copy-only sequence hold O(log B) over a
long mixed structural edit history (the realistic editor condition:
`root0 → root1 → … → root100000`), or does height drift as the review
predicted?

Protocol: mixed history — 30 % char insert / 20 % char delete / 25 %
block split / 20 % block merge / 5 % append (net block drift
+0.05/edit); positions 40 % a 16-leaf hotspot band at 40 % of the
document, 40 % uniform, 20 % near-BOF. Three arms: `replace-only`
(1→1 edits, control), `naive-path-copy` (no rebalance), and
`rebuild-on-height` (rebuild the whole sequence balanced whenever
height exceeds 2·log2(B)+4 — deliberately the cheapest possible
balance policy, measured to prove a bound exists, NOT a frozen
strategy). Every operation passes a length/count invariant check; a
sentinel asserts no op changes the block count by more than its
declared ±1; checkpoints audit stored-vs-computed totals recursively.
`g2_replace_range` (the range update used by split/merge) is verified
against a Vec reference model over all (n ≤ 33, start, count,
repl ≤ 3) combinations — the first implementation had a real
straddle bug the model test caught (fixed before any gate data was
recorded).

## T-G6 — results

**synth-1m, 10 000 edits** (B 13 659 → 13 408):

| arm | height 0 → end | avg depth | repr KiB | q_ns | copies/edit | rebuilds |
|---|---|---:|---:|---:|---:|---:|
| replace-only | 14 → 14 | 13.8 | 1 280 | 114 → 130 | 13.8 | 0 |
| naive-path-copy | 14 → **53** (3.8× balanced) | 14.0 | 1 298 | 114 → 140 | 19.3 | 0 |
| rebuild-on-height | 14 → 19 (budget 32) | 13.8 | 1 260 | 114 → 121 | 16.5 | 2 (3.4 ms total, worst 2.36 ms, amortized 339 ns/edit) |

**synth-100k, 100 000 edits** (B 1 363 → 2 317):

| arm | height 0 → end | avg depth | repr KiB | q_ns | copies/edit | rebuilds |
|---|---|---:|---:|---:|---:|---:|
| replace-only | 11 → 11 | 10.5 | 128 | 61 → 55 | 10.5 | 0 |
| naive-path-copy | 11 → **128** (10.7× balanced) | 10.5 → **26.2** | 128 → **357** | 61 → **157** | 10.5 → **41.3** | 0 |
| rebuild-on-height | 11 → 27 (budget 28) | 11.6 | 228 | 61 → 70 | 13.6 | 21 (3.5 ms total, worst 0.22 ms, amortized **35 ns/edit**) |

## T-G7 — repeated front-collapse (the review's `Node{left, Empty}` concern)

32 sequential `g2_truncate` rounds (each dropping 1/8 of the tail) on a
4 096-leaf root: **height stays 12 throughout** (blocks 4 096 → 61).
The right-spine shape the review flagged does not, by itself, degrade
height under repeated truncation — truncation only ever removes depth.
Recorded as a partial correction to MAJOR-2's mechanism story: the
degradation driver is hotspot split/merge churn (T-G6), not the
truncate shape.

## Gate findings

1. **The review's prediction is confirmed**: naive path-copy-only
   persistence degrades under structural churn — height 11 → 128 after
   100 k hotspot-heavy edits (10.7× the balanced height), and the
   damage is not confined to writes: average leaf depth 10.5 → 26.2
   inflates green memory 128 → 357 KiB (~1.8× the document) and red
   view queries 61 → 157 ns.
2. **A trivial policy bounds it**: height-triggered balanced rebuild
   kept height ≤ 27 (budget 28) at an amortized 35 ns/edit and worst
   single pause 0.22 ms on the 100 k corpus (2 rebuilds, 339 ns/edit
   amortized at 1 MB). Balance maintenance is therefore CHEAP to
   provide at L1 scales — but this policy is a placeholder, not a
   design win; weight-balanced/B-tree/finger-tree joins remain open
   and unmeasured.
3. **Replace-only control confirms attribution**: with no structural
   ops, height never moves (14 → 14, 11 → 11). The degradation is
   caused by split/merge churn specifically, not by path-copy
   persistence or edit volume.

## Corrected H4 wording (feeds the final report)

> Position-free persistent sequence with single-edit locality:
> SUPPORTED (coord_rewrites = 0, flat-in-N edits from a balanced root).
> Long-lived balance: a REQUIRED mechanism for structural-churn
> histories — naive path-copy degrades height ~linearly with churn;
> a trivial height-triggered rebuild bounds it at amortized tens of
> ns/edit. The concrete balancing strategy is NOT earned/frozen.

## Full tables

See `results/raw/parser-survey/run-3-representation/green/summary.md`
(machine-generated, same content).
