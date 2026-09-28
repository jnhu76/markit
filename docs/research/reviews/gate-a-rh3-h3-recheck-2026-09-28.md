# Gate-A corrective re-review — RH3 (H3 old-tree-subtree-reuse, #80)

Status: **FRESH-CONTEXT CORRECTIVE RE-REVIEW RECORD (verbatim)**

```text
REVIEWER      = RH3 (independent fresh-context reviewer; NOT an author of
                #80, PR #83, or any H3 mechanism code)
SCOPE         = issue #80 only
REVIEWED_HEAD = fix/80-h3-donor-fidelity @ 532fd5278995199de761facd1f7c8af4526ad323
DIFF          = git diff fix/79-h2-donor-fidelity..532fd52 (merge-base with
                master after the #82 merge is exactly 367060a, so the PR #83
                three-dot diff equals this reviewed diff)
MERGE         = a8327fb (PR #83, LIVE_HEAD == REVIEWED_HEAD verified at merge)
METHOD        = read-only review + suite reruns on a blob-verified /tmp
                extraction of 532fd52 (a concurrent process moved the shared
                working tree mid-review; repo untouched, clean at close);
                pinned donor source read at tree-sitter v0.27.0 @6070dbf
                (reusable_node.h, parser.c:753-836, subtree.c:645-798) and
                tree-sitter-markdown @f969cd3 on the clean-room server;
                frozen C harness REBUILT there (qualitative only, no timing);
                independent scratch probes for interior descent, window
                exclusion, and a discriminating discovery bound
```

Verbatim verdict (2026-09-28):

---

H3 FRESH DONOR-FIDELITY RE-REVIEW

P1_1_INTERIOR_DESCENT = PASS
P1_2_DISCOVERY_COST_SHAPE = PASS

D1 = MATCH (adapted) — same retained shape (TTree top-level `{gap, Arc<TNode>}`, no absolute offsets, entry ctx, def facts, changed flags); the new `last_discovery: Cell<DiscoverySummary>` on the mechanism (lib.rs:246-252) is documented diagnostic export, not mechanism state.
D2 = MATCH / NON_MATERIAL_ADAPTATION — patch_tree/patch_node COW damage map unchanged by #80 (lib.rs:500-735); continuation-margin neighbor marking retained and declared; donor ts_subtree_edit re-read at 6070dbf confirms the descend-only-into-overlapping-children + has_changes shape.
D3 = MATCH (mechanism model) — WAS MATERIAL_DEVIATION, now repaired: persistent forward-only pre-order path `Vec<LevelCursor>` (lib.rs:938-961, 1144-1227) with monotone `idx`, push-nested/pop-and-advance only; top level borrowed once at `Cursor::new` (no per-consult table rebuild — old `find_run` confirmed at 367060a blob b018db5:855-863); root/Document never a candidate; donor reusable_node.h read at the pin confirms advance/descend/reset shape.
D4 = DECLARED_SIMPLIFICATION + NON_MATERIAL_ADAPTATION — line alignment, ContextKey equality now evaluated at true interior positions (settled candidate is line-aligned; consult carries the #79 post-closure live key), changed-flag refusal, declared horse guards (disjoint-from-edit, windows, patch margins, def-clause); no lex-mode/parse_state narrowing (declared no-LR boundary); per-member ctx still not re-gated on extension (outcome-equivalent; window now re-bounds termination per member).
D5 = FAITHFUL_WITH_DECLARED_SIMPLIFICATION — WAS MATERIAL_DEVIATION, now repaired: damaged-ancestor descent is genuinely operative at any nesting depth (seek's aligned-changed-composite push, lib.rs:1189-1199); residual: block granularity only, and the FIRST interior block of any damaged container is never reusable (opening-line consult key is pre-container-push vs the child's recorded frame-carrying ctx — donor accepts those tokens: my harness rerun shows `block_quote_marker` + earlier-paragraph token reuse). Under-reuse, conservative; declared only in a test comment, not in the freeze amendment (P2-2).
D6 = DECLARED_SIMPLIFICATION — descend-or-advance semantics now operative (changed leaf settles then the consult gate refuses; cursor skips lazily — outcome-equivalent to donor advance-past); no fragile/error/missing channels (no-GLR boundary, frozen); harness rerun confirms donor is_fragile degradations on this grammar that H3 cannot model (P2-1).
D7 = MATCH — no fallback/restart, natural degradation, NotApplicable slots unchanged.
D8 = DECLARED_SIMPLIFICATION — bytes-only derive-at-read, clamped patch arithmetic, stale-position refusal guards; window boundaries computed in patched post-coordinates.
D9 = NON_MATERIAL_ADAPTATION (declared) — ContextKey as the entry-state gate; reference-environment rematerialization retained/counted (R5 §12); the per-consult margin source reads are REMOVED (closer to the donor's metadata-only gate than baseline).
D10 = MATCH — flags persist in the patched tree; the new tree is unflagged (taken members are all !changed; rebuilt wrappers fresh).
D11 = MATCH (mechanism model) — WAS MATERIAL_DEVIATION, now repaired: no index, amortized-forward discovery; `metadata_records_touched += consultations + discovery.total_visits` (lib.rs:451-458) closes the attribution blind spot; `DiscoverySummary` exported via `discovery_summary()`; audit_identity re-derived exact fixture (11→17) pins it. Residual: the once-per-update window setup walk (deepest_node_containing ×2) is uncounted (P3-3).
D12 = MATCH — Arc sharing ↔ refcount retain; COW ↔ ts_subtree_make_mut; fresh wrappers/root; repeat-chain balancing remains declared-out representation surface.

DAMAGED_ANCESTOR_WITNESS = Verified five ways. (1) Code: patch_node marks the intersecting top-level BlockQuote changed (lib.rs:693-724); seek descends positionally into damaged composites and interior lines (lib.rs:1184-1219). (2) Crate witness tests/interior_reuse.rs:170-237 (ONE quote, five `>`-separated paragraphs, edit in para 2) asserts >=2 interior paragraphs identity-shared and the damaged one NOT shared — unsatisfiable by top-level-only takes since the paragraphs are children of the single BlockQuote; list variant (items inside a changed List) at :125-160. (3) My independent probe (scratch /tmp copy, not the repo): BlockQuote.changed == true after prepare; exactly paras 3,4,5 Arc-shared inside the rebuilt quote; paras 1-2 rebuilt; == H0. (4) Old dead code confirmed at 367060a (`find_run`/`search_level` ~855-921: `.position()` alignment only at the damaged entry's first line; children's frame-carrying ctx vs pre-prefix live key). (5) Donor harness rerun at the pinned SHAs: F4 = 60 reuse_node events inside the damaged quote with cant_reuse has_changes on block_quote ancestors; F4b = list_item reuse inside a damaged list.
FORWARD_DISCOVERY_WITNESS = Verified structurally + dynamically. The path never moves backward (LevelCursor.advance is idx+=1 only; pop+parent advance; take advances past the last member, lib.rs:1106-1110); no per-consult allocation or rebuild; visits attributed into metadata_records_touched and exported. The crate's witness (interior_reuse.rs:246-298, total_visits <= 2·nodes + 8·consults, table_rebuilds == 0) passes, but its scenario (edit at byte 3, ~3 consults) would NOT numerically exclude the old shape (~800 est. visits < ~850 bound) and table_rebuilds is vacuously 0 — my discriminating probe (40 inserted paragraphs among 100 top-level blocks: consults×top_entries > bound) passes with the new cursor, confirming the repair where it discriminates (P3-2).
CORRECTNESS = All suites green on the pinned tree 532fd52 (scratch copy, blob-verified): H3 crate — interior_reuse 3, audit_identity 8, h3_gate 15, adversarial_r5 1 (504 internal sequences), reference_environment 5, matrix_r5 549 ignored (debug, per freeze); regressions — fragment-reuse (H2) 34 passed incl. h2_gate 17 + mid_container_reuse 4; shared-grammar 45 (lib 13 + parser_observer 32); block-local (H1) 32 passed incl. h1_gate 17; restart-convergence (H4) 36 passed incl. h4_gate 17; horse-a 232 passed. 0 failures. Plus my 3 independent probes (interior descent, window exclusion, discriminating discovery bound) all pass.

P0 = none
P1 = none
P2 = 2
- P2-1 (carried, unchanged): no fragile/error/missing rejection analogues (declared no-GLR boundary). Donor demonstrably degrades composite reuse to token level under is_fragile on this grammar (reconfirmed in my harness rerun, F1/F5) where H3 takes whole blocks — over-reuse in H3's favor; must accompany any H3-vs-donor comparison.
- P2-2 (new residual): interior reuse is directionally limited — the first interior block of a damaged container is never reused (opening-line consult is pre-container-push; child ctx carries the frame), and pre-damage interior reuse is additionally blocked by the left window inside containers; the donor reuses those tokens. Conservative (penalizes H3), but declared only in a h3_gate.rs comment (:510-512), not in the R5 §8 #80 amendment.
P3 = 3
- P3-1 (residual of #80's doc finding): h3_gate.rs:505-512 comment is off-by-one (says edit in the THIRD paragraph / takes the SECOND; the code edits the SECOND and takes the THIRD), and min_reused=1 is still satisfiable by top-level tail takes alone — the probe still does not witness interior descent itself (the decisive witnesses are in interior_reuse.rs, which the freeze cites).
- P3-2: the in-repo forward-discovery witness bound is non-discriminating on its own scenario and table_rebuilds==0 is asserted on a field no code path increments (structural repair itself is real and verified).
- P3-3: the once-per-update open-edge window computation (deepest_node_containing ×2 + fallback walks + Vec builds in Cursor::new) is horse-added discovery-adjacent work not attributed in any counter.
(Scope observation, not a fidelity deviation: the H3 diff includes a 14-line change to H2's fragment-reuse lib.rs — line-start-based containment in H2's cursor, mirroring H3's seek; H2 suites green, but it is cross-horse scope inside #80's PR and should be acknowledged in its description.)

H3_FIDELITY_VERDICT = FAITHFUL_WITH_DECLARED_SIMPLIFICATION

H3_GATE_A_READY = YES
PR_MERGE_READY = YES

Justification: Both Gate-A P1 blockers are genuinely repaired, not papered over. P1-1: damaged-ancestor interior descent is now a real operative path — the patch demonstrably flags the top-level BlockQuote/List ancestor, the persistent cursor descends positionally into damaged composites and interior lines, and unmarked interior descendants with agreeing entry keys are spliced with shared Arc identity inside the rebuilt container wrapper (fresh parent, shared children — the donor's D5/D12 shape); this is pinned by unspoofable identity witnesses, reproduced by my independent probe, and re-anchored against the pinned donor by rebuilding the frozen C harness on the clean-room server (revs re-verified; F4/F4b reproduced; qualitative only, no timing). P1-2: the stateless per-consult enumeration is gone (borrowed persistent path, monotone indices, zero per-consult allocation), and discovery cost is now visible — total entry visits flow into metadata_records_touched and a DiscoverySummary export, with the audit fixture re-derived exactly (11→17). Remaining deltas are declared or conservative: the no-fragility channel (H3's favor, frozen boundary) and the first-interior-block/pre-damage directional limit (against H3, inherent to the line-granularity donor-faithful consult point but under-declared) keep this below full FAITHFUL_MECHANISM_MODEL; the open-edge windows are sound (my M-CS-ITEM-INDENT probe shows the stale-termination member refused while the popped-back suffix is reused; gap fallbacks are covered by the patch-time margins, es=0/EOF edges handled), and correctness held everywhere (all suites plus probes green; H3 == H0 in every check). Procedural note: a concurrent process moved the shared working tree between branches mid-review, so I reviewed the exact fix/80 commit 532fd52 via a blob-verified /tmp extraction (repo untouched, clean at close); the branch-state hazard itself is worth flagging to the maintainer.

Evidence anchors: mechanisms/old-tree-subtree-reuse/src/lib.rs:246-267, 684-847 (windows), 938-1227 (Cursor/seek/consult), 451-458 (attribution); tests/interior_reuse.rs (3 witnesses); tests/audit_identity.rs (17/13 re-derivation); tests/h3_gate.rs:505-528; protocol/R5-HORSE-CORRECTNESS-PARITY.md §8 amendment #80 (:648-687); old shape at 367060a blob b018db5; donor pins verified on server jnhu@192.168.31.75 (tree-sitter 6070dbf, tree-sitter-markdown f969cd3; reusable_node.h, parser.c:753-836, subtree.c:645-798 read; harness rebuilt in server /tmp, F1-F5 numbers match the frozen record).
