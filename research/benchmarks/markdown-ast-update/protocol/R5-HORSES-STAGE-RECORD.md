# R5 — H1/H2/H3/H4 Mechanism Implementation (Stage Record)

Status: **READY_FOR_ADVERSARIAL_R5_REVIEW** (2026-09-17)
Campaign: #22 MARKIT-MARKDOWN-BENCHMARK-1
Branch: `research/22-r5-horses-correctness-parity-1`
Base: `master` @ `21d7d832fec84fceedb7600cccb4296745395fc1` (PR #29 merge)
Mechanism decisions: `protocol/R5-HORSE-CORRECTNESS-PARITY.md` (frozen
pre-coding; implementation notes recorded per section as work progressed)
Verification: `scripts/verify-r5.sh` — final line
`R5 HORSE CORRECTNESS + PARITY GATE: PASS`.

This stage implemented the four incremental mechanism horses IN ORDER
(H1 -> H2 -> H3 -> H4), each behind its frozen per-horse gate list. It
produces correctness/identity/parity/counters/fallback/eager-completion
evidence ONLY: no timing, no benchmark JSONL, no rankings, no
performance-tuned constants, no SIMD/unsafe/parallelism, no production
parser, no hybrid H5, no R6 work.

## 1. Authority chain and compliance

```text
Issue #22 -> R0 methodology -> R1 harness -> R2 prior-art map
          -> R3 grammar/corpus/mutation freeze -> R4 H0 reference
          -> R5 mechanism decisions (frozen) -> THIS STAGE
```

Compliance:

- Horses H1-H4 depend only on `common`, `oracle`, `corpusgen`
  (test-side), and `shared-grammar`. NO horse depends on
  `mechanisms/full-rebuild`; H0 is used as the correctness oracle in test
  crates only (R5 freeze §1 parity rule).
- Work order followed the frozen pipeline: H1 (commit `ad45c67`), H2
  (`5b39905`), H3 (`c03bc77`), H4 (this stage), each with its gate suite
  green before the next horse started.
- The mechanism decisions in
  `protocol/R5-HORSE-CORRECTNESS-PARITY.md` were frozen BEFORE coding;
  every deviation discovered during implementation is recorded as an
  implementation-note amendment in that document (§6-§9 notes), never
  silently.
- Fallback predicates are derived only from source/edit state — never
  from case identity, corpus identity, labels, or timing. `minGap = 128`
  (H2) and checkpoint density (every top-level block start, H4) are the
  pre-measurement frozen constants declared in the freeze record.
- No timing API is called anywhere in the five mechanism crates; the
  counters reported are the frozen work-counter schema (R0 §10).

## 2. What was built

```text
shared-grammar/                     ONE grammar substrate for H0-H4
  parser.rs   BlockScanner (§3-§8 dispatch), ContextKey/FrameKey,
              splice hook (horse-supplied takes), parse_region[_with_hook]
  inline.rs   RefTable (first-wins D7/D9), inline scan, materialize,
              finish_document
mechanisms/block-local/             H1 BLOCK_LOCAL_REPARSE   (mizchi)
mechanisms/fragment-reuse/          H2 FRAGMENT_REUSE        (lezer-style)
mechanisms/old-tree-subtree-reuse/  H3 OLD_TREE_SUBTREE_REUSE(tree-sitter)
mechanisms/restart-convergence/     H4 RESTART_CONVERGENCE   (W&G/swift)
scripts/verify-r5.sh                correctness-only stage gate
scripts/mutation-check-r5.sh        negative gate (4 mechanism mutations)
```

Mechanism identity, retained state, damage/invalidation, reuse authority,
counter applicability, and identity witnesses per horse are frozen in the
R5 record §6-§9. The four position strategies are exactly the four
R2-H09 classes: H1 delta-shift rebuild, H2 fragment offsets +
parent-relative, H3 patch-path + derive-at-read, H4 per-block base
offsets.

## 3. The correctness gate (frozen judge, applied)

For every update on every horse, on every case surface:

```text
Hx update result == H0 clean authoritative parse   (structural + checksum)
```

plus NORMALIZED-RESULT-v1 validation of both sides, retained-state
integrity invariants, the frozen counter applicability matrix (§3 of the
freeze), identity witnesses W1/W2/W3 per horse, the eager-completion
boundary (pending holds the complete state+result; complete() seals only),
and the frozen QUERY contract (node_path_at over the completed state ==
H0's answer; no horse-only query index).

Surfaces per horse:

- 43/43 golden fixtures (clean parse == H0, state integrity);
- generic edit grids (shape x anchor x edit size x op) at 64k/1m;
- scaling slice to 16 MiB;
- multibyte/CJK-sensitive ops;
- structural recipes (all 13) at 64k and 1m;
- chained edit sequences (5 steps, state carried step to step);
- hand-built probes for each horse's damage/reuse/fallback authorities;
- the frozen 370-slot CASE-MATRIX-v1 differential (release profile);
- the adversarial small-model differential: 504 deterministic sequences
  (six families x 84 seeds, ASCII + CJK/full-width, 3-6 chained edits
  each, splitmix64-seeded — no `random_device`, no external RNG), every
  intermediate step == H0.

## 4. Adversarial-differential findings (fixed in this stage)

The adversarial generator and the 370-grid earned their keep: five sound-
ness holes were found and fixed, all conservative-restriction or
substrate-totality corrections, all recorded as amendments in the R5
freeze record (§7-§9 notes):

1. SHARED GRAMMAR — an unclosed fence nested inside open container
   frames EOF-closed at the REGION end, so the child fence escaped its
   parent quote and H0 itself failed NORMALIZED-RESULT-v1 validation
   (the grammar is total, so the construct is legal input).
   `flush_fence_eof` now clamps the span and the raw-content interval to
   the innermost open frame's last consumed line; a fence truncated by a
   closing container likewise clamps its content interval into its span.
   Top-level unclosed fences still end at the region end (the §8 one
   permitted final LF). Regression test in `shared-grammar`.
2. H2 — LEFT-EDGE CONTINUATION MARGIN: the left fragment must not end at
   a boundary the edit then invalidated (deleting the backticks that made
   a fence opener re-joins the retained paragraph with the live region).
   When the retained block ending at the window edge is a paragraph, no
   blank line separates it from the boundary, and the edit reaches into
   the boundary block's first line, the left fragment is dropped.
3. H2/H3 — LIVE-SIDE PARAGRAPH MARGIN at take start: the line before a
   splice must be blank, because the entry ContextKey deliberately
   excludes paragraph state. The triggering case: an edit that destroys a
   `>` marker merges the quote line with the FOLLOWING paragraph, whose
   old fragment/subtree is still vouched and would splice with a live
   paragraph still open.
4. H3 — PATCH ARITHMETIC CLAMPS + STALE-POSITION GUARD: multi-entry
   deletions could wrap a shifted child offset past zero; patched
   (clamped) candidates could yield non-monotonic run positions. Shifts
   saturate at zero; the line-aligned match uses checked arithmetic; a
   take whose members do not tile monotonically inside the document is
   refused (natural degradation).
5. H4 — RESTART-BOUNDARY CONTINUATION MARGIN, GENERALIZED: the retained
   prefix's last block can continue into the reparsed region not only as
   a paragraph (the grid-found case) but as a QUOTE regaining its `> `
   prefix or a list item regaining indentation (the adversarial
   ContainerState case). An unchanged boundary line cannot continue any
   block kind (the old parse proves it); when the edit reaches into the
   boundary line, H4 backs the restart up one checkpoint.

Cross-horse pattern (a Weakness Map candidate, recorded for R6+): EVERY
incremental horse needed a continuation-soundness guard at its splice
boundaries, because the shared ContextKey deliberately excludes paragraph
state (and frames continue through prefixes the ContextKey does not
model). Each horse closes the hole with its own authority: H1 via
fallback guards (F1-F6), H2 via fragment windows + two margins, H3 via
the changed-flag margin + consult refusal, H4 via the restart backup +
convergence predicate (e). See the parity table, dimension 12.

## 5. Cross-horse parity audit table (13 dimensions x H0-H4)

Instantiates the §10 implementation-parity contract of the R5 freeze.

| # | dimension | H0 | H1 | H2 | H3 | H4 |
|---|-----------|----|----|----|----|----|
| 1 | Grammar substrate | shared-grammar | shared-grammar | shared-grammar | shared-grammar | shared-grammar |
| 2 | Inputs (`&Source` + `CanonicalEdit`) | common | common | common | common | common |
| 3 | Result contract | NORMALIZED-RESULT-v1 + checksum | same | same | same | same |
| 4 | Retained native state | full normalized doc + source len | tiling Vec<TopEntry{Skel,facts}> + defs | parent-relative Arc<FNode> tree + fragment table + defs | TEntry{gap,node} tree (no absolutes) + defs + change flags | Arc<Skel> slots (base_shift) + checkpoint records (position,key,gen) + defs |
| 5 | Damage/invalidation | none (full rebuild) | strict-overlap region + F1-F6 guards | split/drop windows around the edit; `]: ` + damaged has_def flag the table | changed flags along the edited ancestry + continuation margin | damaged-span scan + `]: ` scan; definition generation bump |
| 6 | Reuse authority | none (reuses nothing; measured 0) | whole blocks OUTSIDE the reparse region, after guard veto | fragment takes via splice hook, window + ctx vouched | unmarked line-aligned ctx-agreeing runs via forward cursor | one stable-suffix take at a converged checkpoint |
| 7 | Position strategy (R2-H09) | n/a (rebuild) | delta-shift rebuild | fragment offsets + parent-relative | patch-path + derive-at-read | per-block base offsets |
| 8 | Fallback / degraded mode | none (NotApplicable) | F1 total fallback (counted; the ONLY horse with a fallback slot) | none (natural degradation; NotApplicable) | none (natural degradation; NotApplicable) | restart-at-zero on definition change — a restart, NOT a fallback (NotApplicable) |
| 9 | Counter applicability (§3) | full-parse profile; nodes_reused Known(0) | restart/convergence NotApplicable; fallback Known | same as H1 | same as H1 | restart/convergence Known EVERY update (Known(0) measured); fallback NotApplicable |
| 10 | Eager completion | pending holds all | pending holds all | pending holds all | pending holds all | pending holds all |
| 11 | QUERY | project completed state | same | same | same | same |
| 12 | Continuation-soundness guard | n/a (always reparses) | F1-F6 fallback guards | window exclusion + left-edge margin + live-side blank margin | changed-flag margin + live-side blank margin + stale-position refusal | restart-boundary backup + convergence (e) blank margin |
| 13 | H0 dependency | IS H0 | test-side only | test-side only | test-side only | test-side only |

No unexplained one-horse privilege: the only horse with a fallback slot is
H1 (its frozen mechanism identity includes the mizchi F1 fallback); the
only horse with restart/convergence gauges is H4 (its frozen identity
defines them). Both asymmetries are declared mechanism identity, not
undeclared optimization.

## 6. Verification results

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace` (debug): green — per-horse gate suites
  (H1 15, H2 16, H3 14, H4 15 tests), adversarial differentials (1 x 504
  sequences per horse), shared-grammar suite (incl. the fence-containment
  regression), R1/R3/R4 suites.
- Frozen 370-slot case matrix differential, release profile: PASS for all
  four horses (24 FULL_PARSE + 24 QUERY + 322 update cases each; Block-D
  per-recipe counts match the frozen table; the 7 declared slot overlaps
  are the only collapses).
- `scripts/verify-r5.sh`: final line
  `R5 HORSE CORRECTNESS + PARITY GATE: PASS`
  (fmt, clippy, workspace tests, four release matrices, R1/R3/R4
  regressions, negative gate).
- `scripts/mutation-check-r5.sh`: 4/4 DETECTED (one mechanism-specific
  mutation per horse; each compiles, each is caught by its targeted
  detector, each restored; nothing mutated is committed).

## 7. Self-assessment verdict

The stage stops at `READY_FOR_ADVERSARIAL_R5_REVIEW`: all frozen H1-H4
gates pass, the parity table is recorded, the freeze-record amendments
are explicit, and no measurement has been taken. R6 (state-construction
surface) is NOT started. The PR is opened for adversarial review and is
NOT merged by the implementing agent.
