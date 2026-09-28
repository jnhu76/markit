# Gate-A synthesis re-evaluation — R6 (post-corrective, #76)

Status: **FRESH-CONTEXT SYNTHESIS RE-REVIEW RECORD (verbatim)**

```text
REVIEWER      = R6 (independent fresh-context reviewer; NOT R1–R5/RH2/RH3,
                NOT an author of any reviewed code or record)
SCOPE         = full Gate-A state after the #79/#80 corrective cycle,
                evaluated from the MERGED master
REVIEWED_TREE = master @ 252bd6c (Merge pull request #84; contains
                #82/a3a551f, #83/a8327fb, and the corrective evidence
                record) — isolated clone at /tmp/r6-review, scratch
                probes untracked, never committed
METHOD        = read all frozen contracts/reviews/protocol sections; read
                both mechanism crates and the shared scanner directly;
                ran the full test sweep; wrote and ran 7 independent
                adversarial probes; verified donor pins read-only via SSH
                (jnhu@192.168.31.75, git rev-parse only). NO timing.
```

Verbatim verdict (2026-09-28):

---

R6 Gate-A synthesis re-review complete. Method: isolated clone at 252bd6c
(`/tmp/r6-review`, scratch probes untracked, never committed); read all
frozen contracts/reviews/protocol sections; read both mechanism crates
and the shared scanner directly; ran the full test sweep; wrote and ran
7 independent adversarial probes; verified donor pins read-only via SSH.

ADVERSARIAL_H2_CHECK = HELD. I could not defeat the repair; every attack confirmed the record. (a) Mid-container witness: the frozen test asserts the damaged item 3's identity is NOT shared (`mechanisms/fragment-reuse/tests/mid_container_reuse.rs:161-166`), and a wholesale List take would necessarily share item 3 through the immutable shared Arc (impossible to rebuild one child inside a shared parent); the window check independently makes it impossible (`src/lib.rs:805-809,826`: the List spans the edit, so `cstart+size > win_end` / `cstart < win_start`). My scratch probe reproduced the exact recorded numbers: 27/30 ListItems shared, List identity 0/1, damaged item unshared. (b) `starts_block` gate: `starts_block = para.is_none() || line_starts_new_block_lazy(...)` (`shared-grammar/src/parser.rs:511-512`) is false ONLY for plain text under an open paragraph (B1-B6 all true, :898-950), and `splice_to` flushes the open paragraph before splicing (:592), so interruptor takes are sound. My discriminating probe (blank line → text, making the next old paragraph's line a continuation; candidate ctx vouches perfectly since ContextKey excludes paragraph state) shows the consult refuses exactly there: result is ONE merged paragraph, old-y identity NOT shared, ==H0 — without the gate the take would produce a WRONG split; and my interruptor control shows a heading taken under an open paragraph with ==H0. (c) Discovery shape: `frag_idx`/`LevelCursor.idx` only advance (`lib.rs:795-798,704-713`), `seek`'s skip loop resumes monotonically and refusals leave the cursor in place (:890-950), `LevelCursor` borrows slices with no per-consult rebuild (:660-714), and `metadata_records_touched = consultations + total_visits + slot_count` is enforced (`lib.rs:495-497`); my refusal-heavy probe reproduced 202 consults → 202 visits on the 501-entry tree with `table_rebuilds=0` (old shape ≈20,402). (d) Plain parse unchanged: `full_parse` uses hook-free `parse_region` (:279), a None hook only adds read-only classification before normal dispatch. The known residuals (right-piece minGap drop `lib.rs:385`, failed-seek `max_consult_visits` omission at :810) match the record exactly.

ADVERSARIAL_H3_CHECK = HELD. (a) Interior descent is genuinely operative, not dead code: `seek` descends on the aligned changed composite (`mechanisms/old-tree-subtree-reuse/src/lib.rs:1188-1200`), and interior lines descend via the line-start containment test (:1206-1216); my probe reproduced the exact P2-2 signature on the 5-paragraph quote (edit in para 2 → shared flags [F,F,T,T,T], quote wrapper fresh, ==H0) — unsatisfiable by top-level takes since the single parent container's children are asserted shared and the damaged child unshared (`tests/interior_reuse.rs:170-237`). (b) Open-edge windows: `Cursor::new` computes windows from the PATCHED tree in post coordinates (:974-975, wired at :394); deepest-node boundaries (:687-771) exclude the stale-terminated adjacent member while gap fallbacks delegate no-blank adjacency to the patch-time continuation margins (:568-607, matching R5 §8 amendment #80 (4)). (c) Attribution: `metadata_records_touched += consultations + discovery.total_visits + slot_count` (:457-459) — my probe confirms metadata ≥ discovery component (133 vs 70); the once-per-update window-setup walk remains uncounted and is accurately recorded as accepted P3-3 debt (H3-FIDELITY.md:201-203). (d) My discriminating probe (40 inserted paragraphs among 100 top-level blocks) gives 86 consults → 184 visits vs bound 1088, while the pre-#80 shape would charge 8600 — the old shape violates the bound 8×, confirming RH3's claim the repair is real where it discriminates; suffix reuse (nodes_reused=198) and ==H0 hold. The exact P2-1/P2-2 declarations are present (R5 §8 re-review addendum, protocol R5:693-708; H3-FIDELITY.md:185-194).

RECORD_INTEGRITY = OK (initial FAIL verdicts preserved verbatim as frozen history — FIDELITY-MATRIX.md:3,20-26,101-112; H2/H3-FIDELITY.md:3,114-121 — with dated corrective sections; GATE_A still frozen FAIL pending exactly this R6 evaluation; all new declarations present: minGap asymmetry in R5 §7 re-review addendum :506-514, H3 P2-2 in R5 §8 re-review addendum :693-708, H2 P2×4/P3×5 and H3 P2-1/P2-2/P3×3 in the matrix addendum :114-118; h3_gate.rs:505-512 off-by-one P3-1 honestly left recorded rather than silently fixed). Two non-decision-bearing nits found: the RH3-lineage records say "horse-a 232" but the actual suite at master is 241 (RH2's count is correct; horse-a was untouched by #83), and the H2 discovery-witness comment calls its scenario "refusal-heavy" when it is not — the genuinely decisive refusal-heavy evidence exists and I reproduced it exactly.

GOVERNANCE = OK (no timing/Criterion/benchmark code anywhere in the corrective diffs — only declared structural counters, DiscoverySummary, "structural counters only; no timing" per R5 §7/§8 amendments; Cargo.toml untouched; all 10 donor clones + markit clean clone verified UNCHANGED at their pins on jnhu@192.168.31.75: lezer de5f962/f81d6a2/9942d7c, tree-sitter 6070dbf, ts-markdown f969cd3, mizchi ffe7dc0, md4c/pulldown/comrak/swift at recorded SHAs, markit ad115bd; server access read-only `git rev-parse` only; matrix_r5 left ignored 549×4; working repo untouched — all scratch in /tmp).

H0 = FAITHFUL_MECHANISM_MODEL (P0=0, P1=0)
H1 = FAITHFUL_WITH_DECLARED_SIMPLIFICATION (P0=0, P1=0)
H2 = FAITHFUL_MECHANISM_MODEL (P0=0, P1=0)
H3 = FAITHFUL_WITH_DECLARED_SIMPLIFICATION (P0=0, P1=0)
H4 = FAITHFUL_MECHANISM_MODEL (P0=0, P1=0)

DONOR_FIDELITY_GATE_A = PASS   (P0=0, P1=0 on all five; no MATERIAL_DEVIATION remains; H0/H1 crates unchanged since R1/R4, H4 changed only by the documented 6-line hook-signature adaptation with mechanism semantics intact; carried residuals are declared and non-blocking: H2 P2×4/P3×5, H3 P2-1/P2-2/P3×3, H1 P2-1/P3×3, H4 P2×3/P3×2)

GATE_B_AUTHORIZED = YES

WITNESSES_REPRODUCED = H2 27/30 mid-container ListItem sharing with List identity 0/1 and damaged item unshared; H2 refusal-heavy discovery 202 consults → 202 visits on 501 entries, table_rebuilds=0 (O(B²) excluded); H2 starts_block refusal on a discriminating absorbed-paragraph shape (would be WRONG without the gate) ==H0; H2 interruptor-line take under an open paragraph ==H0; H3 interior descent [F,F,T,T,T] on the damaged-quote F4 shape with fresh wrapper ==H0; H3 discriminating discovery (86 consults/184 visits vs old shape 8600) ==H0; H3 attribution closure (metadata 133 ≥ discovery 70); full suites 420 passed / 0 failed across block-local 32, fragment-reuse 34, horse-a 241, old-tree-subtree-reuse 32, restart-convergence 36, shared-grammar 45; matrix_r5 549×4 ignored per freeze.

RESIDUAL_RISKS = (1) horse-a count reported as 232 in the RH3-lineage records (H3-FIDELITY.md, CHALLENGE-CASES.md) vs actual 241 — a bookkeeping slip worth a one-line errata; (2) H3's P3-3: the once-per-update open-edge window-setup walk is still unattributed work — fine for Gate A, but include it in any future cost attribution; (3) H3 P2-1 (fragility over-reuse, donor's favor absent) and P2-2 (first-interior-block/pre-damage under-reuse) MUST accompany any H3-vs-donor statement, and H2's left-edge/open-edge margins and missing NotLast guards (27/30 vs 25/30) any H2-vs-donor statement — the matrix already mandates this; (4) `table_rebuilds` is a constant-0 diagnostic in H3 (no increment path) — do not read it as evidence of anything in H3; (5) the H3 repair touched H2's cursor (14 lines, line-start containment) — cross-horse scope acknowledged in the records, H2 suites re-verified green here; (6) my two scratch probe files remain untracked in /tmp/r6-review only and were never committed.
