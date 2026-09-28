# Gate-A challenge cases — execution record (#76)

Status: **FROZEN GATE-A CHALLENGE EVIDENCE RECORD**

All executions below are **mechanism-fidelity observations only**. No
timing, no CPU counters, no allocation economics, no throughput, no
horse ranking. Where an upstream tool emitted timing by default, it was
excluded from evidence. Local horse observations come from the frozen
crates at master `ad115bd…` (test suites + scratch probe crates with
path dependencies — no repo modification).

## H0 (R1)

| Case | Donor expectation (source-level) | Local observed | Verdict |
|---|---|---|---|---|
| H0-F1 old-state independence | donors have no old-state input at all | `update()` drops old state first; history-independence test proves byte-identical results + identical counters across histories | PASS-MECHANISM |
| H0-F2 global ref env rebuilt | ref-def table complete before inline phase | `parse_full` two-phase order; golden fixtures pass | PASS-MECHANISM |
| H0-F3 no old-node reuse | fresh tree per call | `nodes_reused = Known(0)` on update and full_parse; unique coverage == full source | PASS-MECHANISM |

Cross-check: COMPLETE (source-level negative evidence — no incremental
API in any donor).

## H1 (R1)

| Case | Donor expectation (mizchi @ ffe7dc00, source-verified) | Local observed | Verdict |
|---|---|---|---|
| H1-F1 valid-seam local edit | region = affected block only; prefix passthrough; suffix shift | fallback=0, 1 block reparsed, prefix reused / suffix rebuilt counters, ==H0 | PASS-MECHANISM |
| H1-F2 seam/boundary invalidation | donor detects NOTHING (silent divergence) | guard fires: exactly 1 counted total fallback, ==H0 (correct merge/fence behavior) | PASS-MECHANISM (declared donor-deviation: silent-wrong → counted fallback) |
| H1-F3 suffix shift | suffix reconstructed as new values via `shift_block_span` (parser-work reuse, not object reuse); inline children left stale | suffix `nodes_rebuilt` incl. inline subtrees (oracle-required deeper shift), ==H0 | PASS-MECHANISM |
| H1-F4 definition edit | TOTAL fallback whenever definitions exist in old OR new doc | F1 reproduced verbatim, both trigger directions; collapse-to-H0 confirmed | PASS-MECHANISM |
| H1-F5 many blocks | overlap arithmetic over old spans; NO index | pure enumeration over tiling; no index/hash in state; full-scan counters | PASS-MECHANISM (no early break — P3) |

Cross-check: PARTIAL — MoonBit execution NOT_FEASIBLE (no toolchain);
full source-level cross-check instead (entire `src/incremental.mbt`
re-read at the pinned SHA; every R2-record citation re-verified).

## H2 (R2)

| Case | Donor expectation (frozen model) | Donor observed (runtime, pinned npm) | Local observed | Verdict |
|---|---|---|---|---|
| H2-F1 aligned fragment + compatible context | after-fragment reuse; minGap before-drop | List+Fence shared; heading dropped (minGap); tail dropped (partial line) | List+Fence+tail shared; heading+para reparsed; ==clean | MATCH core |
| H2-F2 fragment overlaps edit | split + open edges + delta; edited span dropped | exactly that; heading+para1+fence reused | same table semantics BUT left piece dropped entirely by continuation margin (donor reuses it) | table MATCH; left-edge MATERIAL conservative divergence |
| H2-F3 unchanged bytes, incompatible context | hash mismatch → per-line rejection; reuse resumes after container closes | marker change → list split; only fence shared | fence+tail shared; ==clean | MATCH |
| H2-F4 edit shifts fragment | delta remap; old tree untouched; reuse succeeds | off=+20; 2/5 shared | to_old mapping; List+Fence+tail shared (9/11 deep) | MATCH |
| H2-F5 multiple fragments | ≥3 fragments for two edits; middle kept iff stretch ≥ minGap; forward-linear consumption | minGap drops 9B before-piece; keeps 157B middle; minGap=2 keeps all | single-edit contract → ≤2 pieces; forward consumption with take jumps (38/40 shared) | DECLARED_SIMPLIFICATION (substrate); D3/D11 govern discovery |

Cross-check: PARTIAL — markdown-consumer lane COMPLETE at runtime
(node v24.15.0; @lezer/common@1.5.2 + @lezer/lr@1.4.8 + @lezer/markdown@1.6.3);
LR lane COMPLETE at pinned-source reading level. Extra decisive probe:
**mid-container** — donor shares 25/30 ListItem objects after an edit
inside item 3 of a 30-item list; local shares 0 (2/63 nodes) → the D3/D5
MATERIAL_DEVIATION witness.

## H3 (R3)

| Case | Donor expectation (frozen model) | Donor observed (C harness, pinned commits) | Local observed | Verdict |
|---|---|---|---|---|
| H3-F1 unchanged subtree + compatible state → reuse | damaged path refused; rest reused | 228 reuse events; changed coverage 0/720 | 18/20 nodes reused; 1 entry changed; ==H0 | MATCH |
| H3-F2 unchanged bytes + incompatible state → no reuse | entry-state mismatch (fence open) → no reuse | 0 reuse events; coverage 113/113 | nodes_reused=0; full read; one fence node | MATCH |
| H3-F3 edit overlaps candidate → invalidation | flagged candidate refused; siblings reused | 13 reuse events; refusals on damaged path; 0 changed ranges | entry flagged; 6/8 reused | MATCH |
| H3-F4 damaged ancestor, reusable descendant → descend + interior reuse | interior descendants reused at aligned positions | 60 reuse events inside damaged quote; reused list_items | entire damaged entry rebuilt; no interior reuse (descent dead) | **MATERIAL_DEVIATION** |
| H3-F5 many candidates → cursor discovery | forward consumption; heavy reuse | 3798 reuse events; coverage 0/9400 | 398/400 reused; ~10 consults; ==H0 | MATCH outcome; discovery shape differs |

Cross-check: COMPLETE (executable: tree-sitter v0.27.0 core +
tree-sitter-markdown v0.5.3 compiled at -O0; reuse-event log +
changed-ranges + self-equivalence; NO timing).

## H4 (R4)

| Case | Donor-composite expectation | Local observed | Verdict |
|---|---|---|---|
| H4-F1 valid restart + convergence | restart before damage; converge at block boundary under state agreement + margin; suffix reused | restart at damaged checkpoint; convergence at next blank-separated checkpoint (both delta signs); prefix+suffix Arc-shared; ==H0 | PASS |
| H4-F2 damaged restart support | back up to earlier/stronger restart; no false reuse | restart backed up to fence checkpoint; control stays; list-adjacent case no back-up; committed test | PASS |
| H4-F3 candidate before damage crossed → reject | candidates at/before damage refused | refusal stack verified (strict-beyond + checkpoint + damage) | PASS |
| H4-F4 no interior convergence → EOF/progress | continuous degradation to EOF; model-defined semantic path | convergence_distance = post.len(); restart-at-zero with gen bump, discarded pass counted; assembled-table fence-swallow reproduced | PASS |
| H4-F5 state/context incompatibility → no false convergence | same mapped position must not converge under differing live state | 6 constructions all ==H0, convergence deferred to next agreeing checkpoint; (b)-clause decisive role UNWITNESSED | PASS on behavior (P2 caveat) |

Cross-check: COMPLETE (ts/swift/lezer pinned-source, donor model frozen
before horse inspection); PARTIAL for W&G (paper-level; no fabricated
executable comparison).

## Preservation

- Upstream donor-side frozen models (written BEFORE local horse code was
  read): `challenge-evidence/r2-h2-lezer/DONOR-D1-D12-MODEL-FROZEN.md`,
  `challenge-evidence/r3-h3-treesitter/donor-model-H3.md`,
  `challenge-evidence/r4-h4/donor-model-frozen.md`.
- Upstream runtime scripts + observed outputs: `r2-h2-lezer/*.ts` +
  `UPSTREAM-OBSERVED-OUTPUT.txt` (+ exact npm pins in `package.json` /
  `package-lock.json`); `r3-h3-treesitter/gate_h3.c` (build: gcc -O0
  against tree-sitter v0.27.0 core + ts-markdown v0.5.3 pre-generated
  parser/scanner mirrored from the pinned clean-room clones).
- Local horse probe crates (path-dep, run outside the repo):
  `*/horse-probe-main.rs` (+ `arc_identity.rs`, `ctx_dump.rs` for H3;
  observed outputs `PROBE-OUTPUT.txt`, `TEST-RESULT.txt` for H2).
- These artifacts are as-run records for evidence reproduction; they are
  not maintained tooling and are exempt from workspace lint/fmt policy.

## Corrective-cycle challenge evidence — 2026-09-28 (#79, #80)

New decisive evidence produced by the H2/H3 corrective cycle (mechanism
observations only — no timing). Initial H2-F2/H2-F5 and H3-F4 records
above are preserved unchanged; the new records supersede the
*MATERIAL_DEVIATION observations*, not the frozen history.

### H2 (RH2 re-review; #79, PR #82, merge a3a551f)

| Case | Donor observed (pinned npm re-run) | Local observed (post-#82) | Verdict |
|---|---|---|---|
| H2-MID 30-item list, edit inside item 3 (re-run of the R2 decisive probe, `f_midcontainer.ts`/`f_deep.ts` lineage) | **25/30 ListItem objects shared**, byte-identical to frozen `UPSTREAM-OBSERVED-OUTPUT.txt` | **27/30 ListItem Arc identities shared** (positions 4–30), damaged item 3 reparsed, new List identity NOT shared (0/1) — unsatisfiable by top-level takes; == H0 | repaired (delta = declared absent NotLast trailing pop) |
| H2-QUOTE nested quote interior (`mid_container_reuse.rs`) | — (donor takes nested-level runs per pinned-source reading) | ≥5/12 nodes shared inside the damaged quote; == H0 | repaired |
| H2-DISCOVERY refusal-heavy regime (definition-changing replacement, 100 collapsed-ref paragraphs) | donor `reuseFragment` consults per line at pinned source | 202 consultations → **202 total visits** on a 501-entry tree; `table_rebuilds = 0`; witness bound `total_visits ≤ 2·entries + 8·consults`; == H0 | repaired (pre-#82 shape ≈ 20,402 visits) |
| H2-INTERRUPTOR `starts_block` soundness (reviewer adversarial: `>`-marker destroyed ahead of a vouched run) | donor takes runs up to interruptor lines | take refused at the absorbed line, == H0; interruptor-line takes restored where sound (splice flushes the open paragraph) | repaired margin replacement |

Reviewer probes: 7 independent `/tmp` scenarios, every one == H0 clean
parse. Suites: fragment-reuse 34 + shared-grammar 45 + neighbors
(29+36+32+241), 0 failures.

### H3 (RH3 re-review; #80, PR #83, merge a8327fb)

| Case | Donor observed (frozen C harness REBUILT at pinned SHAs, server) | Local observed (post-#83) | Verdict |
|---|---|---|---|
| H3-F4 re-run: damaged ancestor, reusable descendant | F4 = 60 reuse_node events inside damaged quote; F4b list_item reuse inside damaged list (numbers match the frozen record; qualitative only) | ONE quote, five `>`-separated paragraphs, edit in para 2 → quote flagged changed; **paras 3–5 Arc-shared inside the rebuilt wrapper**, paras 1–2 rebuilt; == H0 (identity witness unsatisfiable by top-level takes; `tests/interior_reuse.rs`) | repaired |
| H3-LIST damaged list interior | F4b lineage | ≥6/12 items shared inside the changed List, damaged item not shared; == H0 | repaired |
| H3-DISCOVERY forward cursor | donor single forward ReusableNode pass (pinned-source + harness) | persistent monotone path, zero per-consult allocation; reviewer's discriminating probe (40 inserted paragraphs among 100 top-level blocks) passes where the pre-#83 shape would not; visits attributed into `metadata_records_touched` | repaired |
| H3-WINDOW open-edge exclusion (64k DeepContainer M-CS-ITEM-INDENT lineage) | — (local-substrate guard) | stale-termination member refused; popped-back suffix reused; gap fallbacks covered by patch-time margins; == H0 | sound (reviewer probe) |

Suites: H3 32 + regression sweep (fragment-reuse 34, shared-grammar 45,
block-local 32, restart-convergence 36, horse-a 232), 0 failures; 3
independent reviewer probes pass. The frozen `gate_h3.c` harness was
rebuilt in the server's `/tmp` at tree-sitter 6070dbf /
ts-markdown f969cd3 — as-run artifacts unchanged; no repo tooling
touched.

### Upstream probe output annotation (R5 hygiene item)

The as-run upstream outputs are annotated as follows — do not edit the
artifacts themselves:

- `r2-h2-lezer/UPSTREAM-OBSERVED-OUTPUT.txt`: lines beginning with `$`
  or `> ` are the **commands** that produced the following block; blocks
  beginning `NODES`/`SHARED`/`ListItem objects:` are **observations**
  (verbatim program output); the frozen donor models
  (`DONOR-D1-D12-MODEL-FROZEN.md`, `donor-model-H3.md`,
  `donor-model-frozen.md`) contain the **interpretation** — observed
  outputs carry no interpretation of their own.
- `r2-h2-lezer/TEST-RESULT.txt` and `*/PROBE-OUTPUT.txt`: program output
  plus the exact execution command header recorded at the top of each
  file; pass/fail lines are observations, prose in the fidelity
  contracts is interpretation.
- The C harness (`r3-h3-treesitter/gate_h3.c`): build line recorded in
  `CHALLENGE-CASES.md` §H3 above (gcc -O0 against the pinned core +
  pre-generated parser/scanner); its stdout event log is observation.
  The 2026-09-28 server re-run reproduced the recorded event counts but
  left no new artifacts (rebuilt in the server's `/tmp`, qualitative
  re-verification only).
