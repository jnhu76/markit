# mechanisms/fragment-reuse — H2 FRAGMENT_REUSE

Stage-R5 horse (issue #22). Mechanism identity, fragment lifecycle
(applyChanges split/drop/shift/open edges, minGap = 128 as the frozen
@lezer/common default), reuse rule (alignment + ContextKey vouching +
safe windows + reference clause), counter applicability, and identity
witnesses are frozen in `protocol/R5-HORSE-CORRECTNESS-PARITY.md` §7
(lezer-inspired; R2 MECHANISM-SOURCE-MAP H2).

Retained state: the old tree as parent-relative `Arc<FNode>` block nodes
(materialized inline content, entry ContextKey, has_ref/has_def facts)
plus the fragment table. Update: line-driven parse over the shared
grammar; at every block-start line a cursor consults the fragment table
and takes whole vouched old-block runs through the scanner's splice hook;
everything else parses normally (natural degradation — H2 has no
fallback). Correctness gate: `H2 update result == H0 clean authoritative
parse`, proven by this crate's test suite (`tests/h2_gate.rs`).
