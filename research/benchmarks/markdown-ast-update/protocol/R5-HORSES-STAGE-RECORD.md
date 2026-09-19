# R5 — H1/H2/H3/H4 Mechanism Implementation (Stage Record)

Status: **READY_FOR_FINAL_R5_REVIEW** (2026-09-19; PR #30, corrective-1
gate-passed at `e9539806`; corrective-2 (§12, source-inspection
closure) applied and locally green — focused server verification
pending reviewer acceptance, not merged)
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
| 4 | Retained native state | full normalized doc + source len | tiling Vec<TopEntry{Skel, sem, facts}> + defs | parent-relative Arc<FNode> tree (materialized inline payloads) + fragment table + defs | TEntry{gap,node} tree (no absolutes; materialized inline payloads) + defs + change flags | Arc<RetainedBlock{skel, sem}> slots (base_shift) + checkpoint records (position,key,gen) + defs |
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

## 6. Adversarial self-review (25 questions, one pass; MAJOR fixed once)

Answered against the code as committed on this branch; every MAJOR found
was fixed and re-gated within this stage (findings 2.1, 3.1, 3.2, 4.1,
5.1 below are those fixes, already merged into the final tree).

1. Does any horse call H0/`mechanisms/full-rebuild` outside test code?
   No. `full-rebuild` appears only under `[dev-dependencies]` of the four
   horse crates; the lib targets cannot reach it (cargo tree checked).
2. Does any horse produce or consume timing? No `Instant`/`SystemTime`/
   clock API in the five mechanism crates; the runner remains the only
   clock caller (R1 contract).
3. Is the frozen §3 counter applicability matrix honored exactly? Yes —
   per-horse gate tests pin every slot: measured zeros (`Known(0)`) vs
   `NotApplicable` are asserted distinctly (H0 `nodes_reused`,
   H1 fallback `Known(0)`, H4 restart/convergence `Known(0)` at doc-start
   edits).
4. Is completion genuinely eager in all four horses? Yes — each Pending
   struct holds the fully materialized state + result; `complete()`
   computes the checksum only. The eager gate tests inspect
   `pending.result()` BEFORE `complete()` (behavioral) and note that
   `complete()` takes no source and no sink (structural).
5. Is the QUERY path free of parser work and horse-only indexes? Yes —
   `node_path_at` over the projected completed tree, asserted equal to
   H0's answer at frozen anchors on every shape.
6. Is H1's F1 fallback predicate derived only from source/edit state?
   Yes — old definitions non-empty OR the region creates one; no
   case/corpus identity anywhere. The guard probes pin EXACT fallback
   counts per probe (including zero-fallback cases), which also proves
   the predicate is not label-derived.
7. Is H1's fallback counter honest (event, not a vibe)? Yes — measured
   through `add_fallback_to_full`; the fallback-equality test proves a
   fallback update equals the full reconstruction including counters.
8. Is H2's `MIN_GAP = 128` frozen pre-measurement (not tuned)? Yes —
   declared in the freeze record §7 as a pre-measurement policy constant
   (`PRIOR_ART_ANCHORED_PRE_MEASUREMENT_CONSTANT`); no measurement
   existed when it was frozen.
9. Is H2's rebuilt definition table complete without consulting the old
   table? Yes — every surviving definition appears in the fresh skeleton
   or a taken run's recorded facts (first-wins walk). The earlier draft
   that fell back to the old table was REMOVED as unsound before the H2
   gate was declared green (recorded in the §7 notes).
10. Can H3's clamped patches produce WRONG trees (not just less reuse)?
    No — clamping only degrades alignment; every alignment miss reparses.
    The stale-position guard refuses (never splices) runs derived from
    stale positions; the differential holds on all surfaces including
    the adversarial chains.
11. Is H3's continuation coverage complete (paragraph AND container
    continuation)? The edit-adjacent margin alone was NOT (adversarial
    seed 1011: a take behind a destroyed `>` interruptor) — fixed with
    the live-side blank-line margin; the negative gate proves the margin
    is load-bearing (mutation 3 survives neither the adversarial suite
    nor the probe suite).
12. Is H4's restart-at-zero a fallback in disguise? No — no fallback slot
    is declared (NotApplicable), the path runs the same forward
    machinery over the whole document, and W3 pins the honest gauges
    (restart distance = edit start; convergence = EOF; generation bump).
13. Can H4 falsely converge? The predicate needs position, full
    ContextKey, generation, damage-extent, and a blank-line margin; the
    mapping `q = p − delta` is exact beyond the damage. The
    "paragraph join across thinned gap" probe demonstrates the required
    refusal; no counter-example is known and the differential covers
    370 matrix slots + ~2000 chained adversarial steps per horse.
14. Are the margins' thresholds derived rather than tuned? Yes — the 2-LF
    separation IS the grammar's blank-line semantics (§3/D5/§6), not a
    tunable; no margin constant exists to tune.
15. Is source inspection honestly attributed (no undeclared reads)? All
    reads flow through `record_source_inspection` (scanner lines, def
    scans, margin checks); the W1 witnesses assert sub-full inspection on
    safe local edits, and H0-style tests assert exact full coverage.
16. Do all horses survive multibyte/CJK edits? Yes — multibyte-sensitive
    grids on every shape plus CJK material in half the adversarial
    sequences (the harness snaps spans to char boundaries).
17. Are the negative mutations actually DETECTED (compile-fresh, test
    failing, tree restored)? `mutation-check-r5.sh` verifies all three
    properties per mutation and refuses to run on a dirty tree; expected
    result 4/4 at the time (the negative gate later grew to 7/7 with
    the R5-CORRECTIVE-1 probes A/B/C and to 8/8 with the
    R5-CORRECTIVE-2 probe D — §7.1).
18. Does the matrix instantiation reproduce CASE-MATRIX-v1 exactly? Yes —
    each horse's matrix test asserts 24 FULL_PARSE + 24 QUERY + 322
    updates = 370, the 7 declared overlaps as the only collapses, and all
    13 Block-D per-recipe counts against the frozen table.
19. Is the adversarial generator deterministic and dependency-free?
    Yes — splitmix64 implemented in the test file, seeds derived from
    fixed constants, no `random_device`, no RNG crate (grep-clean).
20. Any test-order dependence or cross-test state? No — every test
    constructs its own states; the determinism tests re-run identical
    pipelines and compare checksums AND counters.
21. Does chained-edit state drift (base offsets, generations, fragment
    tables, change flags)? The chained gate tests and every adversarial
    sequence carry the state across 3-6 updates asserting == H0 at every
    step, on docs that accumulate edits across all six families.
22. Do the freeze-record amendments cover every deviation found during
    implementation? Yes — §6-§9 implementation notes record the
    separator-LF doctrine, per-horse margins, generation handling, gauge
    definitions, and the shared-substrate corrections; the stage record
    §4 indexes them. No silent drift.
23. Do the shared-grammar corrections change any previously-valid golden
    behavior? No — the 43 fixtures and all existing suites were green
    before and after; both fixes only affect constructs that previously
    FAILED NORMALIZED-RESULT-v1 validation (the totality hole), which by
    definition had no golden behavior.
24. Are fmt/clippy/CI-hygiene gates actually part of the stage gate? Yes
    — `verify-r5.sh` runs `cargo fmt --all -- --check` and
    `cargo clippy --workspace --all-targets -- -D warnings` first and
    aborts on any finding.
25. Any scope leak (measurement, benchmark data, R6 work, product
    claims)? None — no timing lane, no result JSONL, no rankings, no
    performance vocabulary in the five crates or the records; the stage
    record predicts no performance; R6 remains untouched.

## 7. Verification results

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace` (debug): green — per-horse gate suites
  (H1 15, H2 16, H3 14, H4 15 tests), adversarial differentials (1 x 504
  sequences per horse), shared-grammar suite (incl. the fence-containment
  regression), R1/R3/R4 suites.
- Frozen 370-slot case matrix differential, release profile: PASS for all
  four horses (24 FULL_PARSE + 24 QUERY + 322 update cases each; Block-D
  per-recipe counts match the frozen table; the 7 declared slot overlaps
  are the only collapses). Per MARKIT-R5-GATE-OBSERVABILITY-2 the matrix
  binaries are case-per-test (548 named slot tests + an inventory test
  carrying the frozen counts; 178 declared NOT_APPLICABLE structural
  slots pass trivially), so failures are isolated, individually
  re-runnable, and nextest-compatible; case definitions, counts, and
  assertions are unchanged.
- `scripts/verify-r5.sh`: final line
  `R5 HORSE CORRECTNESS + PARITY GATE: PASS`
  (fmt, clippy, workspace tests, four release matrices, R1/R3/R4
  regressions, negative gate).
- `scripts/mutation-check-r5.sh`: 4/4 DETECTED at the pre-corrective
  baseline (one mechanism-specific mutation per horse; each compiles,
  each is caught by its targeted detector, each restored; nothing
  mutated is committed). The negative gate later grew to 7/7
  (R5-CORRECTIVE-1 probes A/B/C) and to 8/8 (R5-CORRECTIVE-2 probe D) —
  see the corrective run entries below.

### 7.1 Gate run history (recorded honestly)

Gate run 1 reached every positive step green — workspace tests, all four
release matrices, R1/R3/R4 regressions — and then FAILED (VERIFY_EXIT=1)
at the R4 negative gate, ~6 h in. Two detector defects, zero mechanism
defects (no mutation ever reached the tree; both scripts restore on any
exit):

1. `mutation-check-r4.sh` still targeted
   `mechanisms/full-rebuild/src/{parser,inline}.rs`, which ceased to
   exist when the shared-substrate extraction moved H0's parser/inline
   semantics into `shared-grammar` (full-rebuild is now the H0 wrapper).
   Fix: targets moved to `shared-grammar/src/{parser,inline}.rs`; the
   corrupted expressions, mutation classes, and the detector (H0's own
   suites through the shared crate) are unchanged; M1 now anchors the
   `OpenPara` site explicitly (first-match would have hit the list-item
   frame). Validated: 5/5 DETECTED
   (MARKIT-R5-NEGATIVE-GATE-CORRECTIVE-1).
2. The H4 negative gate originally detected the predicate-(e) removal
   via the probe suite, which stays green under that removal: the
   restart-boundary continuation backup already covers the probes' join
   cases, so (e) is not the deciding guard there. Experiment (temporary
   mutation, then restore) showed the adversarial 504-sequence
   differential DOES fail under (e) removal; the detector was switched
   to `adversarial_r5` (as H2/H3 already were). The probes remain fully
   differential and keep their role pinning gauge semantics and
   soundness on the convergence path. Validated: 4/4 DETECTED
   (pre-corrective).
   Freeze-doc §9 carries the matching amendment — recorded, not silent.

Gate run 2 (local, tree at `e986301`) was killed by a host freeze of the
WSL workstation mid-run and produced no result — recorded as an aborted
run, not evidence either way.

The AUTHORITATIVE run is a single-pass `scripts/verify-r5.sh` executed
on a dedicated server host (Fedora Linux, 20 cores, 62 GB RAM) against
`research/22-r5-horses-correctness-parity-1` at `470934c`, with the
toolchain pinned by the workspace's `rust-toolchain.toml`
(1.97.1, rustc 8bab26f4f — same version and commit hash as the
development host). Every step green in one pass:

- debug workspace step: all suites green (incl. the 549-test
  case-per-test matrices compile-checked, ignored in debug);
- release matrices, `--test-threads=8 --nocapture`: 549/549 passed for
  each of H1 (277 s), H2 (5383 s), H3 (6103 s), H4 (272 s) — per-case
  `ok` lines and the inventory frozen-count test included;
- R1/R3/R4 regressions: PASS (R4 re-runs R1+R3 and its own negative
  gate internally);
- `R4 MUTATION CHECK: 5/5 DETECTED`, `R4 H0 REFERENCE GATE: PASS`,
  `R5 MUTATION CHECK: 4/4 DETECTED` (pre-corrective baseline; the
  corrective runs below supersede this count);
- final line: `R5 HORSE CORRECTNESS + PARITY GATE: PASS`, followed by
  the stage banner "Correctness only: no benchmark campaign was run and
  no timing was recorded." The wall durations above are operational
  logs of the gate run, not benchmark data (the R5 stage records no
  performance measurements).

The AUTHORITATIVE corrective run is a single-pass `scripts/verify-r5.sh`
on the same dedicated server host (Fedora Linux, 20 cores, 62 GB RAM)
against `research/22-r5-horses-correctness-parity-1` at `e9539806` —
the exact HEAD carrying MARKIT-R5-HORSE-CORRECTNESS-PARITY-CORRECTIVE-1
(H1 ownership pass-through, H1/H4 retained semantic subtrees,
instrumented inline source inspection, retained-inline node accounting,
completed-state QUERY law, negative probes A/B/C) — toolchain pinned
1.97.1 (rustc 8bab26f4f). Window 2026-09-19 09:36:54–14:03:15 (+0800),
~4 h 26 min wall. Every step green in one pass:

- debug workspace step: 193 `test result: ok` lines, 0 `FAILED`;
- eager completed-state step: `H1_EAGER_COMPLETION_PASS` …
  `H4_EAGER_COMPLETION_PASS`, `EAGER_COMPLETION_VALIDATION_PASS`;
- release 370-slot matrices, `--test-threads=8 --nocapture`:
  549/549 passed for each of H1 (285 s), H2 (5450 s), H3 (6012 s),
  H4 (274 s) — per-case `ok` lines and the inventory frozen-count
  test included;
- static completed-state QUERY authority check: PASS;
- R1/R3/R4 regressions: PASS, `R4 MUTATION CHECK: 5/5 DETECTED`;
- `R5 MUTATION CHECK: 7/7 DETECTED` (the original 4 mechanism
  mutations + corrective probes A clone-pseudo-reuse, B inline
  emission off, C Pending-bypass);
- final line: `R5 HORSE CORRECTNESS + PARITY GATE: PASS`, followed by
  the stage banner "Correctness only: no benchmark campaign was run
  and no timing was recorded." Wall durations above are operational
  logs of the gate run, not benchmark data.

## 8. Self-assessment verdict

The stage stops at `READY_FOR_FINAL_R5_REVIEW`: all frozen H1-H4 gates
pass, the corrective-1 verdict items (MAJOR: 3 / IMPORTANT: 1) are
implemented and re-gated green at `e9539806` (§7.1 corrective run,
§10/§11), the parity table is recorded, the freeze-record amendments
are explicit, and no measurement has been taken. R6 (state-construction
surface) is NOT started. The PR is NOT merged by the implementing
agent.

## 9. Observations carried to the Weakness Map (structural; no measurements)

- H3 (old-tree-subtree-reuse) consults the live parse at every line
  start, and each consultation rebuilds the full top-level entry list
  from the tree (one Arc clone + linear scan per entry) before the
  line-aligned search. The cost therefore grows with the product of
  consultation count and top-level entry count, and the matrix's
  1 M/16 M-scale Block-B/C cases dominate the gate's wall time. This is
  a mechanism characteristic of the horse as frozen — recorded here as
  a Weakness Map candidate for the measurement stages, NOT tuned away:
  any lookup-structure change would alter the mechanism identity that
  R6/R7 are supposed to measure.

## 10. CORRECTIVE-1 (human adversarial review of PR #30 @ a199819)

Verdict received: `MAJOR: 3 / IMPORTANT: 1`, `R5_CORRECTIVE_1_REQUIRED`.
One targeted corrective was executed
(MARKIT-R5-HORSE-CORRECTIVE-PARITY-CORRECTIVE-1; commit 6f3961c and
follow-ups); H1-H4 mechanism identities are unchanged, the R3 case
authority is unchanged, no performance content entered, R6 was not
started.

- MAJOR-1 (H1 pseudo reuse): H1's update now CONSUMES the old state;
  prefix entries MOVE (ownership pass-through) and only those are
  counted `nodes_reused`; suffix entries are reconstructed by recursive
  delta-shift over their retained syntax and counted `nodes_rebuilt`.
  A heap-address ownership witness (`h1_prefix_reuse_is_ownership_pass_through`)
  pins the path; negative probe A (move -> clone) must fail it.
- MAJOR-2 (inline inspection): the shared inline scanner reports every
  scanned content segment (`scan_region_with_sink` and siblings);
  `parse_full` threads the same sink, so H0 keeps full-document
  coverage through the identical instrumented path. Per the corrective,
  instrumenting the old H1/H4 behavior alone was NOT acceptable: both
  mechanisms now avoid global inline rescans natively (MAJOR-1/§6
  retained syntax), so the identity witnesses show genuinely sub-full
  inspection. Probe B (emission disabled) is caught by the raw-event
  gate `inline_inspection_events_are_reported`.
- §4/§5/§6 (retained syntax): H1 `TopEntry::Block { skel, sem, facts }`
  and H4 `BlockSlot.block: Arc<RetainedBlock { skel, sem }>` retain the
  MATERIALIZED semantic subtree; prefix/converged-suffix reuse shares
  complete syntax with zero parser source reads; H1's suffix shift and
  H4's `base_shift` retargeting are pure representation rebuilds over
  retained syntax (spans + `FencedCode.content`), counted as rebuilt.
  H2/H3 already retained inline payloads; their fresh-node construction
  is now sink-instrumented and reused `Arc` members require zero inline
  source reads.
- MAJOR-3 (completed-state QUERY): every horse implements
  `NormalizeV1`; the law is
  `done.state.normalize_v1() == H0 clean result` with
  `checksum(normalize_v1) == result_checksum`, no `Source`, no sink, no
  parser. Matrix helpers and QUERY batches project from the COMPLETED
  state only; the eager `Pending.result() == H0` check remains as an
  eager-completion fact, and each eager gate additionally feeds the
  completed state into a second update. Probe C (helper reverted to the
  Pending shortcut) is caught by the static completed-state authority
  check in `verify-r5.sh`.
- IMPORTANT (node accounting): counting rule clarified and implemented
  (protocol §11.6): one structural block/container node per block unit
  + every retained inline syntax node, recursively. H2/H3 counters now
  include payload inline nodes; H1/H4 count skeleton structure once and
  the semantic subtree's inline nodes — under this rule a full clean
  parse reports exactly the number of distinct normalized nodes (H0
  parity). H1's W1 arithmetic is updated accordingly (4 = 2 moved
  entries x (1 skeleton + 1 Text)).
- Eager-completion banners: the gate runs the four
  `h*_counters_and_eager_completion` tests explicitly and records
  `H1_EAGER_COMPLETION_PASS` … `H4_EAGER_COMPLETION_PASS`,
  `EAGER_COMPLETION_VALIDATION_PASS`.
- Negative gate: `R5 MUTATION CHECK: 7/7 DETECTED`
  (4 mechanism mutations + corrective probes A/B/C), each validated to
  compile and be detected, each restored, nothing mutated committed.
  The script's restore trap is now installed only AFTER its clean-tree
  refusal check (an earlier draft could have reverted uncommitted work
  on refusal; the check and the trap were reordered).

## 11. Focused adversarial self-review (corrective §15; ONE pass)

Scope: the corrective surfaces only. Every question answered with a
concrete check; findings were fixed BEFORE the final authoritative run
was launched. Local verification at the review HEAD: fmt clean, clippy
`-D warnings` clean, full workspace suite green, mutation gate 7/7
DETECTED.

1. Does H1 report any cloned representation as nodes_reused? NO.
   Prefix entries MOVE out of the consumed old state
   (`moved_prefix.push(e)`; no clone on the safe-local path). Witness:
   `h1_prefix_reuse_is_ownership_pass_through` (heap-address identity
   of a retained inline buffer across the update); Probe A
   (move -> clone) makes that witness FAIL. Fallback reports
   `nodes_reused = Known(0)`; full_parse reports the measured zero.
2. Are all inline source reads represented in inspection events? YES.
   `scan_region_with_sink` reports `[ss, se)` before scanning; every
   helper read (`run_len`, `find_run_exact`, `scan_link_candidate`,
   `find_text_region_end`) is bounded by `se`, so the event covers the
   scanned bytes exactly; recursive sub-scans report overlapping
   subranges the collector unions. Fresh construction in H1-H4 uses
   only the `*_with_sink` forms (grep: the only plain `finish_document`
   call left in the tree is inside shared-grammar's own unit test).
3. Can any reused subtree trigger inline scanning? NO. H2/H3 taken
   members and H4 retained prefix/converged-suffix are `Arc` clones
   with no scan call on the path; H1's prefix is moved and its
   projection clones the retained subtree (pure).
4. Can H1 safe-local W1 still prove actual sub-full coverage? YES —
   `unique_source_bytes_inspected < post.len()` holds WITH honest
   instrumentation, because only the damaged region's lines and inline
   content are read; the suite asserts it (h1_identity_w1).
5. Can H4 convergence prove suffix SYNTAX reuse, not skeleton reuse +
   full inline rescan? YES, now structurally: new witness
   `h4_converged_suffix_shares_retained_syntax_identity` asserts
   `Arc::ptr_eq`-level identity of the converged suffix head between
   old and new states, plus `nodes_reused > 0` and the sub-full
   inspection union. Self-review finding F1 (this witness was missing;
   added).
6. Does completed-state normalization require Source? NO. Every
   `normalize_v1(&self)` takes nothing; H1/H4 projections use the
   stored `src_len`.
7. Does completed-state QUERY use Pending.result? NO. All four matrix
   helpers and QUERY batches project via `done.state.normalize_v1()`;
   the static authority check in `verify-r5.sh` enforces >= 3
   occurrences per matrix file, and Probe C fails it when a helper
   reverts to the eager shortcut.
8. Does completed-state QUERY invoke parser work? NO — pure traversal
   over stored subtrees (`clone` + span shift only); `node_path_at` is
   the oracle's pure query.
9. Do H2/H3 counters include retained inline syntax? YES —
   `count_node = 1 + payload_inline_nodes + children`; reused counts
   and rebuilt counts use the same rule; fresh payload construction
   adds `1 + payload_inline_nodes` per node.
10. Does any horse store an undeclared full NormalizedDocument cache?
    NO. H1State = tiling + defs + src_len; H2State = tree + fragments;
    H3State = tree; H4State = blocks + checkpoints + gen + defs +
    src_len. `Pending` holds the eager result only until `complete()`
    (the frozen phase boundary), never retained. No Source is retained.
11. Did mechanism identity change? NO. Guards F1-F6, fragment
    windows/margins/minGap, patch-path + change flags + margins,
    checkpoints/restart selection/backup/predicates (a)-(e),
    restart-at-zero, gauges and fallback semantics are byte-identical
    policy; only retained representation, instrumentation, counting
    units, and the query surface changed. The four adversarial
    differentials (504 sequences each) and all in-file probes pass
    unchanged.
12. Did R3 workload authority change? NO — zero diff under `cases/`,
    `corpusgen/`, `grammar/`; matrix CASE_TABLEs and the frozen
    inventory test are untouched.
13. Did any performance tuning enter? NO — no constant, threshold, or
    layout change; the only additions are measurement (Built counters),
    retained data, and witnesses.
14. Did R6 start? NO — no runner/instrumentation/cases changes; no
    benchmark, no timing anywhere.

Process incident recorded honestly: during gate preparation, the
negative-gate script's restore-on-EXIT trap fired after its clean-tree
REFUSAL and reverted six uncommitted corrective files; the premature
authoritative run was killed (twice — once on reviewer instruction to
self-review first), the files were re-applied with assert-guarded
edits, the trap was moved below the refusal check, and the gate was
revalidated 7/7 locally. Findings fixed before any run counted as
evidence: (a) `shift_node_owned` initially missed the
`FencedCode.content` interval — caught by the H1 64k grid differential;
(b) the first counting rule double-counted block nodes stored in both
skeleton and semantic subtree — resolved by the §11.6 rule, caught by
the H1 fallback counter exactness test; (c) H4's definition table must
be computable from skeletons BEFORE fresh inline materialization —
restructured; (d) finding F1 above.

CORRECTIVE-1 close-out: the focused self-review above (§11) was
accepted by the human reviewer with `CORRECTIVE_CODE_VERDICT: PASS`;
the single authoritative corrective gate run at `e9539806` ended
`R5 HORSE CORRECTNESS + PARITY GATE: PASS` (§7.1). Nothing was written
to the tree after that gate except this status/evidence record.

## 12. CORRECTIVE-2 (final review verdict MAJOR 1: source-inspection closure)

The final review of the corrective-1 gate state accepted correctness,
identity, eager completion, completed-state QUERY, and node accounting,
but found the MAJOR-2 instrumented-attribution surface still incomplete:
mechanism-private source reads (margins, definition probes, boundary
scans, line-offset derivations) were not reported as
`record_source_inspection` events. Correctness was never in question —
the defect is attributional: R0/R1's planned
`unique_source_bytes_inspected / logical edited bytes` ratio would have
under-reported real source work and could have produced falsely low
PA values for H3/H4.

Audit (non-test code, all four horses) and repairs — every repair is a
sink call plus sink threading; NO parser/reuse/fallback/state/result
expression changed, so the `e9539806` full correctness matrices remain
valid for these mechanisms:

- H4 `prepare_update` (the named finding): the restart-boundary
  margin's separation scan `[prev.abs_end, boundary)` and the boundary
  line's `k_line_end` forward scan now report exact ranges (the
  `MechanismContext` was already in scope).
- H4 `consult` (e): the blank-check read now reports on EVERY consult
  that reaches it (pass or fail; previously only passing checks
  reported, and the range omitted the backward scan's byte).
- H1: `would_continue`'s line reads, F4(a)'s region backward scan +
  terminator probe, `block_facts`/`last_item_strip` list-fact reads.
- H2: the edited-span `]: ` probe, `left_window_end`'s three `old`
  reads, the consult-time paragraph margin (previously unreported),
  both `line_offset` derivations.
- H3: `patch_tree`'s two separation `lfs` counts, the `]: ` probe, the
  consult-time paragraph margin, both `line_offset` derivations.
- Shared substrate: `line_start_of_reported` / `memchr_lf_reported`
  (report the bytes actually scanned);
  `CounterSink::inspections()` exposes raw events for event-level
  assertions (a margin read inside a parser-reported line range is
  invisible to the derived union counters).

Detection: four per-horse attribution tests (event-level containment +
multiplicity assertions, all green) and negative probe D in
`mutation-check-r5.sh` (removing H4's separation-scan report must fail
`h4_prepare_margins_report_source_inspection`). Gate count 7/7 → 8/8.

Reviewer round on this commit (Corrective-2 review): the horse-private
closure was accepted, with ONE remaining shared-substrate gap found —
`BlockScanner::splice_to` reads the taken range's last byte
(`src.get(new_pos - 1)`) for the frame-carry check, and a successful
take skips `[pos, new_pos)` outright, so no per-line report ever covers
that byte (1 byte per successful splice; PA-sensitive on small edits).
Closure boundary: the reviewer explicitly did not extend the audit
further. Fix: the carried check now reports the exact `(new_pos - 1,
new_pos)` event; a shared attribution regression
(`splice_take_reports_its_tail_byte_source_inspection`) asserts the
event `(8, 9)` for a take `[5, 9)` AND that no event covers the reused
interior `[5, 8)`; negative probe E removes the event and must fail the
shared test. Gate count 8/8 → 9/9.

Verification status at this commit: fmt / clippy `-D warnings` /
`cargo test --workspace` (exit 0) and the four attribution tests pass
locally. The focused server run — R1 regression, mutation gate 9/9,
eager + attribution tests (per-horse and shared splice) — runs after
reviewer acceptance of this commit; the 370×4 release correctness
matrices are NOT re-run (no mechanism semantics changed).
