# mechanisms/old-tree-subtree-reuse — H3 OLD_TREE_SUBTREE_REUSE

Stage-R5 horse (issue #22). Mechanism identity, patch rule (copy-on-write
along the edited ancestry; change flags ARE the damage map; sibling
positions derive by prefix sum — never an eager O(N) shift), reuse rule
(LINE-aligned forward cursor + ContextKey agreement + changed-flag
refusal + continuation margin), counter applicability, and identity
witnesses are frozen in `protocol/R5-HORSE-CORRECTNESS-PARITY.md` §8
(tree-sitter-inspired; R2 MECHANISM-SOURCE-MAP H3).

Retained state: top-level `{gap, Arc<TNode>}` entries (no stored absolute
offsets) with per-node sizes, relative children, entry ContextKey,
has_ref/has_def facts, and changed flags. Update: prepare patches only the
edited ancestry; the forward pass takes unmarked context-agreeing runs
whole through the shared scanner's splice hook; rejection descends or
reparses naturally (no fallback concept: slot NotApplicable). Correctness
gate: `H3 update result == H0 clean authoritative parse`, proven by this
crate's test suite (`tests/h3_gate.rs`).
