# mechanisms/restart-convergence — H4 RESTART_CONVERGENCE

Stage-R5 horse (issue #22). Mechanism identity, checkpoint payload,
restart rule, frozen convergence predicate, definition-generation
handling, counter applicability, and identity witnesses are frozen in
`protocol/R5-HORSE-CORRECTNESS-PARITY.md` §9 (wagner-graham/swift-
inspired restart-before-damage + forward validation + stable-suffix
reuse; R2 MECHANISM-SOURCE-MAP H4 — with a benchmark-model-defined
convergence predicate, per the recorded fidelity note).

Retained state: top-level `{base_shift, line_offset, Arc<Skel>}` slots
(per-block base offsets; a reused suffix block is the SAME Arc with a
shifted base — no reconstruction) plus a checkpoint record at every
top-level block start `{position, ContextKey, generation}`. Update:
prepare selects the restart checkpoint at/before the damage (with the
restart-boundary continuation margin) and scans the damage; the forward
parse from the restart consults mapped old checkpoints (`q = p − delta`)
through the shared scanner's splice hook and, when the frozen predicate
(position, state, generation, damage extent, blank-line paragraph
margin) holds, takes the old stable suffix to EOF by retargeting base
offsets. Definition-changing damage restarts at the document start
(generation bump) — a restart, not a fallback: the fallback slot is
NotApplicable. Correctness gate: `H4 update result == H0 clean
authoritative parse`, proven by this crate's test suite
(`tests/h4_gate.rs`).
