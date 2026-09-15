# RUN-4 — ReferenceIndex semantic dependency experiment (plan §23–27)

Issue: #19, branch `exp/19-parser-survey-1`. Research-only index:
`crates/parser-survey/src/refindex.rs`; driver: `refbench.rs`
(`--refs`). Zero product code touched.
Raw: `results/raw/parser-survey/run-4-semantic-deps/refs/` (local).

## Question (H3/Q8)

Can a GLOBAL meaning change (a definition edit) keep a LOCAL syntax
parse, with semantic work proportional to actual dependents?

## EXPERIMENTAL_SUBSET (stated per plan §26)

Whole-paragraph def lines only — ORACLE-B proved L1 has no ref-def
block construct, so the research layer extracts definitions from
paragraph text. CommonMark label normalization (lowercase + whitespace
collapse), first definition in document order wins, losing duplicates
are candidates. Definitions inside containers: UNSUPPORTED here.
Correctness fixtures (§26) all pass: def-before-use, def-after-use,
duplicate→first-wins, case normalization `[Foo]`/`[f o o]`→`f o o`,
undefined reference, inline-link-vs-shorthand extraction.

## Results

| scenario | users | syntax_us | blocks_reparsed | rebuild_us | delta_us | truly_changed | labels | oracle |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| def_url_edit | 100 | 2.2 | 3 | 68 | 14.0 | 100 | 1 | ok |
| def_delete | 100 | 1.5 | 2 | 40 | 5.2 | 100 | 1 | ok |
| dup_second_edit | 100 | 1.2 | 2 | 39 | 7.7 | **0** | 0 | ok |
| user_edit | 100 | 1.1 | 2 | 38 | 6.3 | **0** | 0 | ok |
| def_url_edit | 1 000 | 1.7 | 3 | 741 | 87.2 | 1 000 | 1 | ok |
| def_delete | 1 000 | 1.7 | 2 | 513 | 56.6 | 1 000 | 1 | ok |
| dup_second_edit | 1 000 | 2.0 | 2 | 491 | 37.3 | 0 | 0 | ok |
| user_edit | 1 000 | 1.8 | 2 | 456 | 38.0 | 0 | 0 | ok |
| def_url_edit | 10 000 | 3.3 | 3 | 8 447 | 845.6 | 10 000 | 1 | ok |
| def_delete | 10 000 | 4.0 | 2 | 8 073 | 577.0 | 10 000 | 1 | ok |
| dup_second_edit | 10 000 | 3.3 | 2 | 5 258 | 371.0 | 0 | 0 | ok |
| user_edit | 10 000 | 4.8 | 2 | 5 576 | 416.4 | 0 | 0 | ok |

Index self-equivalence oracle: every incremental update equals a clean
rebuild (resolutions per label + user-set sizes), on all 12 scenarios.

## Findings

1. **The syntax axis is blind to semantic fanout** — `blocks_reparsed`
   stays at 2–3 and `syntax_us` at 1–5 µs whether the def has 100 or
   10 000 dependents. This is the run-1 E-evidence now completed with a
   semantic layer: H3's separation is real and measurable.
2. **Semantic fanout = actual dependents**: def edits invalidate
   exactly the users whose resolution changed (100/1 000/10 000);
   user edits and losing-duplicate edits invalidate exactly 0.
3. **A losing-duplicate edit is provably a no-op** (`dup_second_edit`,
   truly_changed = 0). A full re-resolve pays O(document) to learn
   nothing; the index knows the winner did not change.
4. **Delta vs rebuild (the no-index world)**: the incremental delta is
   ~10× under a full rebuild at 10 k users (845 µs vs 8.4 ms) and its
   cost scales with dependents, not document size.
5. **Prototype artifact, stated**: `delta_us` on zero-change scenarios
   (37–416 µs) is enumeration of users-of-touched-labels with
   per-user resolution clones *inside* the prototype's update. The
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
