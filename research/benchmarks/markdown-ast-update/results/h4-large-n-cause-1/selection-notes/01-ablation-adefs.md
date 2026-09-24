# Selection note — `Adefs`

**Trigger observation (from the superseded attempt-3 run).** In the
W_COUNTERS lane the frozen mechanism's definition traversal visits **every**
retained skeleton on every local edit: `definition_nodes_visited == M` at
every N (1024 at 128 KiB, 131072 at 16 MiB), while `definition_blocks_found
== 0` and `definition_table_entries == 0`. The U_PHASE lane put that region
(`P4`) at 6.0 % of U_PHASE at 128 KiB rising to 8.5 % at 16 MiB.

**Two explanations to separate.**

1. The traversal is *semantically required*: a local text edit could change
   which bytes are definitions (a fence closing, a `]: ` marker appearing),
   so the assembled definition environment genuinely must be recomputed over
   the whole document.
2. The traversal is a *representation accident*: the retained state already
   carries a complete definition fact list, and when that list is empty and
   the fresh region provably contributes no definition, the assembled
   environment is empty by construction and the traversal cannot change the
   answer.

**Variant chosen.** `Adefs` (Issue #50 §9): skip the global traversal only
when (i) the retained definition table is empty, (ii) no damaged old entry
carries a definition, (iii) the edited span's post bytes contain no `]: `,
and (iv) no freshly parsed block carries a definition — with (iv) executed
and timed, never read from the case id or generator label.

**What would support each.** If the effect is near zero, explanation 1 wins
for this regime (the traversal is cheap or the guard is not reached). If the
effect tracks `P4`'s share and is consistent across sessions, explanation 2
wins and the traversal is a removable representation cost. The issue's own
§9 restricts the conclusion to the no-definition local regime either way.
