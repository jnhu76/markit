# H4-CONVERGENCE-PREDICATE

Audit authority SHA: `0bf678cc1505098e6afe26cb8ec54cda24115831`.
Subject: `mechanisms/restart-convergence/src/lib.rs` `Cursor::consult`
(L739–L782) against the frozen predicate (R5 §9).

The predicate decides when the forward parse may STOP and hand the old
stable suffix to EOF. A wrong predicate produces either a wrong tree
(premature convergence with an open paragraph / changed fence state) or
silent non-reuse. Clause by clause:

## (a) exact checkpoint at the mapped position — L748–L752

```rust
let q = map_back(pos, self.delta);
let Ok(idx) = self.checkpoints.binary_search_by(|cp| cp.position.cmp(&q)) else { return None };
```

`map_back` (L787–L793) maps post coordinates to old coordinates by
delta alone — sound because bytes before the edit are identical and
bytes after it shifted by exactly delta. Exact match (binary search on
a sorted-by-construction table — checkpoints are registered in block
order, L843–L852, and assembly preserves document order).

Evidence: restart-point test — the CJK insertion consults at post 37,
q = 34 exact. Near misses (q=19 in the canonical test, q=12/13 in the
restore case) refuse. `TEST_SUPPORT`.

## (b) ContextKey agreement — L754–L757

The live key arrives from the scanner BEFORE prefix consumption
(parser.rs L332), so frame state (quote/list) and fence state are
included; paragraph state deliberately is not (R5 §2).

Evidence: fence-closer removal — every consult inside the would-be
fence body carries the fence key and mismatches the top-level
retained entry (H3 uses the same key rule; for H4 the equivalent case
is the restore test where q=12/13 fails (a), and the BREAK case where
no consult is offered at all inside fence body).
`CODE_INSPECTION_SUPPORT` + `TEST_SUPPORT`.

## (c) generation agreement — L758–L761

`cp.gen != self.gen` refuses records registered under an older
definition table. Reaches here only after a restart-at-zero bumped the
generation (L282–L283); pinned by the definition-damage test
(gen 0 -> 1). `TEST_SUPPORT`.

## (d) beyond every damaged old entry — L762–L765

`q < self.damaged_end` refuses. `damaged_end` = max end (OLD
coordinates) over damaged entries (prepare L402–L418). This is what
keeps a convergence candidate from covering bytes the patch still
believes are damaged. Evidence: canonical edit consult(20): q=20 >=
damaged_end=18. The refused candidate at q=10 (consult(10) is ≤ ee_new
anyway) shows the ordering. `TEST_SUPPORT`.

## (e) paragraph margin (blank line before the splice) — L766–L779

```rust
let prev_ls = line_start_of(self.post, pos - 1);
self.blank_checks.push((prev_ls.saturating_sub(1), (pos - 1)));
if !sg::parser::all_spaces(self.post, prev_ls, pos - 1) { return None; }
```

The load-bearing clause: the ContextKey cannot see an open paragraph,
so the splice is only safe where NO paragraph is open — exactly at a
blank-line boundary, in BOTH parses (a blank line terminates every
continuation). The read is buffered and reported even when the check
FAILS (R5-CORRECTIVE-2 closure).

Evidence (the adversarial case): old = `para one / ## head / (blank) /
para two`. The live parse offers a block start at the heading (q = 9
exact, key matches top-level, gen matches, beyond damage) — but the
line before it is p1's text, so (e) refuses. Convergence then happens
at 18, proven by `convergence_distance = Known(18)` and by the exact
counters (head' fresh: blocks=2, rebuilt=4, reused=2). A predicate
without (e) would splice mid-paragraph and deliver a WRONG tree.
`TEST_SUPPORT` (`h4_predicate_e_refuses_interruptor_adjacent_splice`).

## One-take rule and post-take silence — L740–L743, L780–L781

At most one convergence per update (`if self.take.is_some() { return
None }`); the take end is always `post.len()` (the suffix runs to
EOF). After the take, the scanner splices directly to EOF
(`splice_to`), so subsequent line starts inside the taken range never
consult — visible in the counters: the canonical edit has consults at
line starts 10, 19, 20 only (not 30). `TEST_SUPPORT` (metadata pin).

## Restart-boundary continuation margin (the pre-forward analogue)

`prepare_update` L344–L400: the retained prefix's last block could
continue into the reparsed region when no blank line separates it from
the restart. The margin reads the OLD separation (reported, L367–L371)
and, if the edit reaches INTO the boundary line (`es < k_line_end`,
via `memchr_lf_reported_in(Old)` L378–L383, also reported), backs the
restart up one checkpoint so the merge happens inside the reparsed
region. Pinned: the interruptor doc backs up from slot 1 to slot 0,
with OLD coverage exactly [8,17) = 9 bytes and
`restart_distance = Known(12)`. `TEST_SUPPORT`
(`h4_restart_boundary_backs_up_...`).

## Restart-at-zero: the frozen degraded-adjacent response

`restart_at_zero` (L268–L301) is reached by two sound paths:
(1) pre-parse fast probes — damaged subtree has a def, or `]: ` in the
edited span's post bytes (L452–L459, probe short-circuited when the
first path already fired — observable: no probe event in the
definition-damage test); (2) the assembled-table comparison after the
forward pass (L564) — the fence-closer case where no byte is
definition-like. Path (2) counts the discarded forward pass
(skeleton-only) before restarting (L579–L588,
MEASUREMENT-CORRECTIVE-1 §14). Gauges: `restart_distance = Known(es)`,
`convergence_distance = Known(post.len())`. `fallback_to_full_count`
stays NotApplicable — a restart is NOT a fallback (R5 §9).
`TEST_SUPPORT` (both paths pinned separately).

## Residual risks (observations, not defects)

1. (e) reads `post[prev_ls-1 .. pos-1]` — one line back. On a line
   start immediately after a LONG line, the margin read covers that
   whole line even when the take would be refused by (a) later — no:
   (e) runs AFTER (a)-(d), so the read only happens for candidates
   that passed the cheap clauses. Cost profile is bounded by candidate
   count. `CODE_INSPECTION_SUPPORT` (order L748→L776).
2. `map_back` for negative delta uses `pos + |delta|` — correct only
   while the byte identities after the edit are unchanged, which the
   patch/restart machinery guarantees (the suffix beyond damaged_end).
   No stale-map path observed in the adversarial cases.
   `CODE_INSPECTION_SUPPORT`.
3. The take end is unconditionally `post.len()` — the frozen model's
   "old stable suffix to EOF". An implementation free to take a
   shorter suffix would produce different trees on trailing-damage
   edits; not possible here (constant). 
   `MECHANICALLY_PROVEN_BY_INVARIANT`.
