# Gate-A corrective re-review — RH2 (H2 fragment-reuse, #79)

Status: **FRESH-CONTEXT CORRECTIVE RE-REVIEW RECORD (verbatim)**

```text
REVIEWER      = RH2 (independent fresh-context reviewer; NOT an author of
                #79, PR #82, or any H2 mechanism code)
SCOPE         = issue #79 only
REVIEWED_HEAD = fix/79-h2-donor-fidelity @ 367060aff624c35fbbad96bc5be72f35702ec487
DIFF          = git diff 834239d..367060a
MERGE         = a3a551f (PR #82, LIVE_HEAD == REVIEWED_HEAD verified at merge)
METHOD        = read-only review + suite reruns + independent /tmp probe
                crates (no repo modification); pinned donor source read at
                @lezer/markdown 1.6.3 @9942d7ce (verify + advance()/
                reuseFragment/FragmentCursor); donor runtime re-reproduced
                at the pinned npm versions (local node v24.15.0; server has
                no node); NO timing collected
```

Verbatim verdict (2026-09-28):

---

All verification complete. The repository is untouched (clean tree,
branch restored to `fix/80-h3-donor-fidelity`); all scratch work stayed
in `/tmp`.

H2 FRESH DONOR-FIDELITY RE-REVIEW

P1_1_MID_CONTAINER_DISCOVERY = PASS
P1_2_DISCOVERY_COST_SHAPE = PASS

D1 = MATCH (family) — FTree/FSlot/FNode + `Fragment{new_start,new_end,to_old,open_start,open_end}` (lib.rs:141-201), the donor Tree+offset+open-flags shape; unchanged by the corrective.
D2 = DECLARED_SIMPLIFICATION — single-edit split/drop/delta/open-edges identical to applyChanges (lib.rs:344-393); residual P3: right piece < 128 dropped where the donor never drops the after-last-change piece.
D3 = FAITHFUL (mechanism model) — was MATERIAL_DEVIATION; now a persistent forward-only cursor: monotone `frag_idx` (lib.rs:795-798), per-level monotone `LevelCursor` path with advance/pop-advance/descend-on-containment (`seek`, lib.rs:890-946), post-closure live-level consult (parser.rs run loop phases 1-2); runtime: 27/30 nested ListItems taken with List identity NOT shared.
D4 = FAITHFUL core + declared margins — structural ContextKey (declared, strictly stronger than the 32-bit hash); `starts_block` refusal is the sound, donor-faithful replacement of the blank-line margin (takes at interruptor lines now happen, as the donor does); left-edge continuation margin and open-edge one-block exclusion retained; NotLast/trailing-partial-line/openEnd−1 guards still absent (declared, bounded over-reuse).
D5 = FAITHFUL (mechanism model) — was MATERIAL_DEVIATION; item-level sibling runs inside damaged containers are taken (27/30 list; 7/12 quote-nested), leaf blocks atomic, damaged block forfeited wholly; the frozen "not a strict subset" §7 claim is now delivered.
D6 = MATCH — refusal = per-line natural degradation, fragments retained, retried every dispatched line (202-consult refusal regime demonstrates the cadence); refusal-on-mismatch without descent now mirrors donor `matches()` (markdown.ts:1866-1869) — the old ctx-mismatch descent's removal is an improvement.
D7 = MATCH — empty table → plain full parse; `fallback_to_full_count` NotApplicable; unchanged.
D8 = MATCH (family) — `to_old` delta mapping; old tree immutable (Arc-shared); unchanged.
D9 = NON_MATERIAL_ADAPTATION — reference clause + rematerialization (benchmark-necessitated, declared §7/§12); my probe accidentally exercised the take-then-rematerialize path: ==H0 PASS.
D10 = MATCH (family) — table rebuilt per edit; whole-document fragment re-registered (addTree analogue); unchanged.
D11 = FAITHFUL (mechanism model) — was MATERIAL_DEVIATION; no per-consult table rebuild (`table_rebuilds`=0 in every regime; LevelCursor borrows, no Arc clones), monotone discovery, fully attributed: `metadata_records_touched = consultations + total_visits + slot_count` (lib.rs:492-497), numerically confirmed (6+31+3=40; 3+201+3=207; 202+202+0=404).
D12 = MATCH — fresh wrapper spine with Arc-shared members; unchanged.

MID_CONTAINER_WITNESS = Independently reproduced via a /tmp probe crate (no repo modification): 30-item tight list, insertion inside item 3 → 27/30 ListItem Arc identities shared (positions 4-30), damaged item 3 reparsed, result == H0 (normalize_v1 + checksum + validate_root). Decisively NOT faked by top-level takes: the new List identity is NOT shared (0/1) — since a wholesale List take would require sharing the damaged item 3 through the window check (impossible), the 27 shared items can only come from nested-level member takes inside the damaged container. Donor side reproduced at the pinned npm versions (node v24.15.0 locally — the server has no node/npm, so donor runtime claims rest on this local rerun plus pinned-source reading): f_deep.ts → "ListItem objects: 25/30 shared", byte-identical to frozen UPSTREAM-OBSERVED-OUTPUT.txt. Local 27 vs donor 25 = the declared absent NotLast trailing pop.
DISCOVERY_SHAPE = Code-verified (i): no per-consult Vec/Arc rebuild — `find_run`'s entries rebuild is deleted, LevelCursor holds `&'a [FSlot]`/`&'a [(usize, Arc<FNode>)]` (lib.rs:660-714); (ii) monotone — `advance()` only increments idx, pops advance the parent past the consumed child, descents push only into the entry containing p_old (and p_old is globally monotone: fragments tile the old doc excluding the edit span, consulted only inside fragments); (iii) attributed — total_visits feeds metadata_records_touched and `discovery_summary()`. Runtime: true refusal-heavy regime (definition-changing replacement inside the Def block over 100 collapsed-ref `[def][]` paragraphs): 202 consultations → 202 total visits on a 501-entry tree, ==H0 PASS — the pre-#79 shape was ~202×101 ≈ 20,402 entry visits plus per-consult allocations. The witness bound 2E+8C is sound (skips/pops/descents each globally ≤ entry count because each node's child list is one level instance advanced past at most once; re-checks are O(1) per consult) and excludes O(B²) decisively (2,618 vs 20,402 on that shape). Note: an earlier probe of mine initially showed C=4 — that was a take+rematerialization doc (1-byte insertion ⇒ empty damaged range ⇒ `definition_changing=false`); the corrected regime above is the genuine refusal case.
CORRECTNESS = All green, 0 failures: fragment-reuse 34 passed (h2_gate 17, mid_container_reuse 4, audit_identity 7, reference_environment 5, adversarial_r5 1; matrix_r5 549 ignored — frozen campaign, not run); shared-grammar 45 (13 lib + 32 parser_observer); neighbors: old-tree-subtree-reuse 29, restart-convergence 36, block-local 32, horse-a 241 (163 lib + 78 integration). Plus 7 independent /tmp probe scenarios, every one == H0 clean parse.

P0 = none
P1 = none
P2 = 4 (all pre-existing, declared, non-material, none worsened by the new cursor): (1) left-edge continuation margin can drop the entire left fragment on first-line boundary edits (lib.rs `left_window_end`; §7 amendment (b); against-H2); (2) open-edge one-whole-block exclusion vs donor ~one-line rounding (lib.rs `right_window_start`; §7; large-block axis); (3) missing NotLast/trailing-partial-line/openEnd−1 guards → bounded over-reuse vs donor (empirically 27/30 vs 25/30; declared in H2-FIDELITY D4 and the witness comment); (4) exact line-alignment + line-granular consult refuses gap-positioned candidates (declared in the crate header; conservative) together with the `fragment.from ≤ pos−1` step-back not modeled (local accepts from == pos; ≤1 line).
P3 = 5: (1) symmetric minGap right-piece drop still not noted as a donor asymmetry in §7 DAMAGE text (documented only in the frozen H2-FIDELITY record; ≤127 B); (2) single-edit contract (substrate-imposed, declared); (3) `max_consult_visits` omits visits made inside a `seek` that returns None (`let level = self.seek(p_old)?;` lib.rs:810 returns before `bump_max`, lib.rs:874) — informational counter only, total_visits/metadata attribution exact; (4) h2_gate "nested quote edit" probe still asserts min_reused=1, satisfiable by top-level tail takes (its comment's claim is now pinned by mid_container_reuse.rs instead); (5) the shared-scanner protocol change re-baselines H3/H4 attribution expectations — honestly handled in this PR (h3 audit_identity counters updated with a documented rationale; H3/H4 re-freezes belong to their own correctives #80+).

H2_FIDELITY_VERDICT = FAITHFUL_MECHANISM_MODEL

H2_GATE_A_READY = YES
PR_MERGE_READY = YES

Justification: Both Gate-A P1 blockers are repaired and independently verified. P1-1: the consult point moved to the donor-faithful post-closure/pre-push position (parser.rs `run` phase 2, mirroring markdown.ts `advance()`: readLine/finishContext precede reuseFragment, verified by reading the pinned donor at 9942d7ce), and the persistent descent path (`seek`, lib.rs:890-946 — the moveTo childAfter/parent analogue with line alignment) delivers live-level candidate discovery: the frozen 30-item witness now shares 27/30 ListItem identities inside the damaged List with the List identity itself unshared (my probe), against the donor's runtime-reproduced 25/30 (f_deep.ts, byte-identical to the frozen record; the 2-item delta is exactly the declared absent NotLast pop). P1-2: the per-consult entries rebuild and linear scan are gone (LevelCursor borrows slices; `table_rebuilds`=0), discovery is amortized-forward with exact structural accounting (`metadata_records_touched` includes every visit, numerically confirmed in four regimes), and the true refusal regime measures 202 visits for 202 consults on a 501-entry tree versus the pre-#79 ~20,402 — the 2E+8C witness bound excludes the O(B²) shape by an order of magnitude on that regime. The blank-line margin's replacement is sound and donor-faithful: the `starts_block` gate refuses only live-paragraph absorption (my adversarial interruptor→plain-text case: refused, ==H0), takes at interruptor lines are sound because `splice_to` flushes the open paragraph (heading-shared control: ==H0), and the lazy B1-B6 classification mirrors `classify`/`refdef_at`/`fence_opener_at`/`parse_marker` exactly in both directions. Take-end rounding and the placeholder-span/jump split are pinned by `a_mid_line_take_end_jumps_to_the_next_line_start` and my nested-quote probe (==H0). Correctness holds everywhere (34 + 45 + 29 + 36 + 32 + 241 tests green; no benchmark/timing run; matrix_r5 correctly left ignored). Residual deviations are the declared, bounded, non-asymptotic P2/P3 set above — none worsened by the new architecture, and the two remaining conservative margins (left-edge, open-edge) plus the missing donor guards bound behavior on specific edge shapes without changing dominant-work class on the container axes, so the frozen §7 claim ("taking runs of sibling blocks at nested levels … not a strict subset") is now delivered. Fidelity docs (H2-FIDELITY.md, FIDELITY-MATRIX.md) still carry the R2 MATERIAL_DEVIATION verdict as history; recording this re-review's verdict should accompany Gate-A closure but does not block the code merge.
