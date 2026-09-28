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
