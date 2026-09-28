# MARKIT #76 GATE-A CORRECTIVE CLOSURE REPORT

Status: **GATE A = PASS (as later re-evaluation 2026-09-28; initial FAIL
preserved) — CLOSED BEFORE GATE B**

This is the closure record of the Gate-A corrective cycle of issue
[#76](https://github.com/jnhu76/markit/issues/76). It closes the cycle;
it does not start Gate B.

## 1. Cycle identity and decision

```text
TASK                    = #76 Gate A — H0–H4 donor-fidelity audit
INITIAL_STATE           = GATE A = FAIL / BLOCKED
                          (H2 = MATERIAL_DEVIATION, 2×P1;
                           H3 = MATERIAL_DEVIATION, 2×P1;
                           frozen in FIDELITY-MATRIX.md + PR #81)
CORRECTIVE_DECISION     = REPAIR_MECHANISM on both tracks
                          (NO claim-boundary downgrade on either)
OUTCOME                 = both repairs materially faithful →
                          GATE A = PASS as later re-evaluation
                          (outcome A; outcome B — a repair still
                          materially unfaithful — did not occur)
GATE_B                  = AUTHORIZED (R6) but NOT STARTED — this cycle
                          STOPS before any Gate B work
```

## 2. Phase log

| Phase | Work | Record |
|---|---|---|
| 0 | Live authority verified (master ad115bd, PR #81 OPEN @ d1021c4, #76/#79/#80 OPEN) | this cycle's opening state |
| 1 | PR #81 merged @ 834239d as the DURABLE Gate-A FAIL evidence record (explicitly not a PASS); #76 updated without erasing the FAIL | #76 comment, PR #81 |
| 2 | H2 (#79) and H3 (#80) corrective tracks, separate branches/PRs, every change mapped to P1-1/P1-2; qualitative fidelity tests only | #82/#83 diffs |
| 3 | RH2 (fresh, #79) → P0=0/P1=0/GATE_A_READY=YES → PR #82 merged (LIVE_HEAD==REVIEWED_HEAD); RH3 (fresh, #80) → same gates → PR #83 merged; #79/#80 closed completed with closure comments | `../reviews/gate-a-rh2-h2-recheck-2026-09-28.md`, `../reviews/gate-a-rh3-h3-recheck-2026-09-28.md` |
| 4 | Corrective evidence recorded on `research/76-donor-fidelity-reevaluation` (dated corrective sections; initial FAILs preserved; R5 hygiene debt accepted) | PR #84 (merge 252bd6c) |
| 5 | R6 fresh synthesis re-evaluation from merged master 252bd6c: adversarial H2/H3 checks HELD (7 independent probes), record integrity OK, governance OK | `../reviews/gate-a-r6-synthesis-2026-09-28.md` |
| 6 | This closure: verdict updates + errata + report; #76 updated (NOT closed) | this file |

## 3. Five-horse verdicts (final, R6)

```text
H0 = FAITHFUL_MECHANISM_MODEL               (P0=0, P1=0)
H1 = FAITHFUL_WITH_DECLARED_SIMPLIFICATION  (P0=0, P1=0)
H2 = FAITHFUL_MECHANISM_MODEL               (P0=0, P1=0)  [was MATERIAL_DEVIATION]
H3 = FAITHFUL_WITH_DECLARED_SIMPLIFICATION  (P0=0, P1=0)  [was MATERIAL_DEVIATION]
H4 = FAITHFUL_MECHANISM_MODEL               (P0=0, P1=0)

DONOR_FIDELITY_GATE_A = PASS
GATE_B_AUTHORIZED     = YES (not started)
```

Decisive reproduced evidence: H2 27/30 mid-container ListItem identity
sharing (List identity 0/1; donor 25/30 byte-identical at pinned npm
versions); H2 refusal-heavy discovery 202 consults → 202 visits on 501
entries (O(B²) excluded); H2 starts_block refusal on the discriminating
absorbed-paragraph shape; H3 interior descent [F,F,T,T,T] inside a
flagged BlockQuote (donor F4/F4b reproduced at pinned SHAs); H3
discriminating discovery probe (86 consults/184 visits vs old shape
8600); full suites 420 passed / 0 failed; matrix_r5 549×4 ignored per
freeze. All probes == H0 clean parse.

## 4. Merge ledger

```text
EVIDENCE_FAIL   PR #81 -> 834239d (Gate-A FAIL record, frozen)
H2_CORRECTIVE   issue #79,  PR #82 (367060a) -> H2_MERGE_COMMIT = a3a551f
H3_CORRECTIVE   issue #80,  PR #83 (532fd52) -> H3_MERGE_COMMIT = a8327fb
EVIDENCE_UPDATE PR #84 (87d762b)           -> 252bd6c
CLOSURE         this PR (docs + 2 comment errata)
ISSUE_STATE     #79 closed (completed), #80 closed (completed),
                #76 OPEN (status updated; Gate B remains future work)
```

## 5. SERVER fields

```text
CLEAN_ROOM_HOST      = jnhu@192.168.31.75 (passwordless key auth added
                       for this cycle; password used once transiently
                       via a discarded askpass helper, NEVER stored in
                       the repo, scripts, docs, shell history, or any
                       committed log)
CLEAN_ROOM_ROOT      = ~/research/markit-gate76/
MARKIT_CLEAN_CLONE   = ~/research/markit-gate76/markit @ ad115bd (unchanged)
DONOR_CLONES         = all 10 verified UNCHANGED at their pins
                       (R6, read-only git rev-parse):
                       @lezer/common de5f962, @lezer/lr f81d6a2,
                       @lezer/markdown 9942d7c, tree-sitter 6070dbf,
                       tree-sitter-markdown f969cd3, mizchi ffe7dc0,
                       md4c / pulldown-cmark / comrak / swift-syntax at
                       their recorded SHAs
SERVER_WORK          = qualitative only: RH3 rebuilt the frozen C harness
                       in the server's /tmp at the pinned SHAs (F4/F4b
                       reproduced); no timing; no server state outside
                       /tmp modified; donor repositories NEVER updated
```

## 6. GOVERNANCE fields

```text
PERFORMANCE_COLLECTION_AUTHORIZED = NO   (held for the entire cycle)
PERFORMANCE_COLLECTION_RUN        = NO   (no timing, Criterion, PMU,
                          allocation economics, throughput, or ranking
                          anywhere; only declared structural counters)
CLAIM_BOUNDARY_DOWNGRADE          = NONE (both tracks took the repair
                          branch; DECISION = REPAIR_MECHANISM)
REVIEWER_INDEPENDENCE             = RH2/RH3 fresh-context, each scoped
                          to one issue, neither an author; R6 fresh
                          synthesis, not R5/RH2/RH3, not any author
MERGE_GATES                       = every mechanism PR merged only on
                          P0=0, P1=0, *_GATE_A_READY=YES,
                          PR_MERGE_READY=YES with LIVE_HEAD ==
                          REVIEWED_HEAD verified at merge
INITIAL_FAIL_PRESERVATION         = yes — PR #81 record, FIDELITY-MATRIX
                          header, H2/H3-FIDELITY headers unchanged;
                          corrective sections appended with dates
FALSIFICATION_DISCIPLINE          = an adversarial probe DID defeat an
                          H2 candidate margin during development and the
                          design was replaced (blank-line margin →
                          starts_block) BEFORE freezing; R6 attacked
                          both repairs with 7 probes and could not
                          defeat either
STOP_POINT                        = this closure; Gate B (six-horse
                          performance comparison) is authorized but
                          must be preregistered and run under its own
                          frozen protocol — NOT STARTED here
```

## 7. Residual declared risks (carried, non-blocking)

1. H3 P2-1 (fragility over-reuse vs donor, H3's favor) and P2-2
   (first-interior-block / pre-damage interior under-reuse, against H3)
   MUST accompany any H3-vs-donor statement; H2's left-edge/open-edge
   margins and missing NotLast guards (27/30 vs 25/30) any H2-vs-donor
   statement. The matrix mandates this.
2. H3 P3-3: the once-per-update open-edge window-setup walk is
   discovery-adjacent work not yet attributed in a counter — include it
   in any future cost attribution.
3. `table_rebuilds` is a constant-0 diagnostic in H3 (no increment
   path) — do not read it as evidence.
4. Errata applied in this closure (non-decision-bearing): horse-a count
   232 → 241 in the derived RH3-lineage records (verbatim review
   transcripts left untouched); H2 discovery-witness comment relabeled
   take-heavy (the genuine refusal-heavy evidence is the 202→202
   regime).
5. The H3 repair carries a 14-line cross-horse touch to H2's cursor —
   acknowledged on PR #83 and in the records; H2 suites re-verified
   green by RH3 and R6.
