# RUN-4 — ReferenceIndex semantic dependency experiment (plan §23–27;
# CORRECTIVE-1 rework)

Issue: #19, branch `exp/19-parser-survey-1`. Research-only index:
`crates/parser-survey/src/refindex.rs`; driver: `refbench.rs`
(`--refs`). Zero product code touched.
Raw: `results/raw/parser-survey/run-4-semantic-deps/refs/` (superseded),
`results/raw/parser-survey/corrective-1-refs/refs/` (corrective rerun).

> **CORRECTIVE-1 (2026-09-16) — the first RUN-4 prototype conflated
> stable identity with document order (review MAJOR-3).** Candidate
> definitions were stored as `(doc_order, block_id, url)` but both slots
> received the block id, and resolution took `min_by_key(order)` — so
> "first wins" actually meant "smallest id wins". Under the planned
> stable-identity model (new blocks get NEW larger ids), a definition
> inserted BEFORE the current winner would wrongly lose. The index was
> reworked: candidates store `(BlockId, url)` only; resolution consults
> a `DocOrder` provider supplied by the syntax layer (the block
> sequence), so the index owns no order authority. The H3 fanout
> conclusion below was NOT affected (those scenarios never insert
> blocks), and survives the rework with equivalent numbers.

## Question (H3/Q8)

Can a GLOBAL meaning change (a definition edit) keep a LOCAL syntax
parse, with semantic work proportional to actual dependents?

## EXPERIMENTAL_SUBSET (stated per plan §26)

Whole-paragraph def lines only — ORACLE-B proved L1 has no ref-def
block construct, so the research layer extracts definitions from
paragraph text. CommonMark label normalization (lowercase + whitespace
collapse), first definition in DOCUMENT ORDER wins (via the `DocOrder`
provider), losing duplicates are candidates. Definitions inside
containers: UNSUPPORTED here. Correctness fixtures (§26) all pass:
def-before-use, def-after-use, duplicate→first-wins, case normalization
`[Foo]`/`[f o o]`→`f o o`, undefined reference, inline-link-vs-shorthand
extraction — plus the CORRECTIVE-1 identity≠order fixtures (below).

## CORRECTIVE-1 winner-mutation gates (identity ≠ order)

Structural mutations with stable ids deliberately ≠ positions; each
asserts the CommonMark first-wins-by-document-order outcome AND the
index self-equivalence oracle (per-label resolution + user sets):

| gate | mutation | winner after | changed users | oracle |
|---|---|---|---:|---|
| insert_winner_before | NEW def id 999 inserted before winner id 10 | **/new** (flips) | 2 | ok |
| insert_loser_after | same def appended after the winner | /old (unchanged) | 0 | ok |
| delete_winner | current winner removed, dup present | /second | 2 | ok |
| move_candidate_before | id 224 MOVED before winner — same id, new rank only | **/moved** (flips) | 2 | ok |
| split_def_block | `[a]`+`[b]` paragraph split into two blocks | both preserved | 0 | ok |
| merge_def_blocks | two def paragraphs merged into one | both preserved | 0 | ok |

`insert_winner_before` is the review's counterexample — the old
id-as-order index resolved it to `/old`; the reworked index resolves it
to `/new`. `move_candidate_before` is the decisive identity≠order
proof: the winner flips with an UNCHANGED stable id, because rank — not
identity — decides. A pure order change costs ZERO index maintenance
(order is external; the winner flips at the next resolution query).

Architecture consequence (feeds the final report): the semantic index
MUST consume an order/rank service from the syntax layer; `BlockId` is
durability identity only. The six gates are regression fixtures for
that boundary.

## H3 fanout battery (corrective rerun)

| scenario | users | syntax_us | blocks_reparsed | rebuild_us | delta_us | truly_changed | labels | oracle |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| def_url_edit | 100 | 3.1 | 3 | 76 | 16.5 | 100 | 1 | ok |
| def_delete | 100 | 1.7 | 2 | 43 | 7.2 | 100 | 1 | ok |
| dup_second_edit | 100 | 1.3 | 2 | 43 | 12.1 | **0** | 0 | ok |
| user_edit | 100 | 1.3 | 2 | 42 | 8.4 | **0** | 0 | ok |
| def_url_edit | 1 000 | 1.9 | 3 | 740 | 123.3 | 1 000 | 1 | ok |
| def_delete | 1 000 | 1.8 | 2 | 520 | 92.6 | 1 000 | 1 | ok |
| dup_second_edit | 1 000 | 2.0 | 2 | 523 | 73.9 | 0 | 0 | ok |
| user_edit | 1 000 | 2.3 | 2 | 510 | 65.3 | 0 | 0 | ok |
| def_url_edit | 10 000 | 3.0 | 3 | 8 079 | 1 456.8 | 10 000 | 1 | ok |
| def_delete | 10 000 | 4.3 | 2 | 7 949 | 1 127.2 | 10 000 | 1 | ok |
| dup_second_edit | 10 000 | 3.2 | 2 | 5 802 | 1 035.0 | 0 | 0 | ok |
| user_edit | 10 000 | 5.8 | 2 | 5 663 | 676.1 | 0 | 0 | ok |

Index self-equivalence oracle: every incremental update equals a clean
rebuild (per-label resolution + user sets), on all 12 scenarios and all
6 winner gates.

## Findings

1. **The syntax axis is blind to semantic fanout** — `blocks_reparsed`
   stays at 2–3 and `syntax_us` at 1–6 µs whether the def has 100 or
   10 000 dependents. H3's separation is real and measurable (unchanged
   by the CORRECTIVE-1 rework).
2. **Semantic fanout = actual dependents**: def edits invalidate
   exactly the users whose resolution changed (100/1 000/10 000);
   user edits and losing-duplicate edits invalidate exactly 0.
3. **A losing-duplicate edit is provably a no-op** (`dup_second_edit`,
   truly_changed = 0). A full re-resolve pays O(document) to learn
   nothing; the index knows the winner did not change.
4. **Delta vs rebuild (the no-index world)**: the incremental delta is
   ~5–6× under a full rebuild at 10 k users (1.46 ms vs 8.1 ms) and its
   cost scales with dependents, not document size.
5. **Prototype artifact, stated**: `delta_us` on zero-change scenarios
   (~0.7–1.0 ms at 10 k) is enumeration of users-of-touched-labels with
   per-user resolution clones inside the prototype's update. The
   mechanism allows O(1) no-change detection (compare the winning
   candidate record before fanning out) — a production delta would
   short-circuit; the prototype does not. The mechanism conclusion
   (fanout ∝ dependents, local syntax) does not depend on this.

## D-axis fills (plan §3.3)

| scenario | syntax (observed) | semantic | representation |
|---|---|---|---|
| def_url_edit | S2/S3_BLOCK-NEIGHBOR | **D4_INDEXED_DEPENDENTS (N)** | M1 local |
| def_delete | S3 | D4 (N) | M1 |
| dup_second_edit | S2 | D0_NONE (proven by index) | M1 |
| user_edit | S2 | D1_LOCAL | M1 |

This is the first run where D ≠ D_UNKNOWN is measured rather than
asserted.

## Limitations

- Whole-paragraph def subset; container defs, link titles, reference
  *creation by inline definition* are out of scope.
- `truly_changed` counts user blocks whose resolution changed; a
  downstream renderer's own per-element cost is not modeled (that is
  the projection layer's job, plan §3.5 P_provider).
- Users/labels scale only to 10 k here; no wall-clock claims beyond
  this corpus.
- The winner gates model the syntax layer as an ordered slot list with
  caller-assigned stable ids; a production BlockId/OrderKey source
  (order-maintenance or the block index's own sequence) is a design
  task, not measured here.
