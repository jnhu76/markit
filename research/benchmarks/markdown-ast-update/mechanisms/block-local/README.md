# mechanisms/block-local — H1 BLOCK_LOCAL_REPARSE

Stage-R5 horse (issue #22). Mechanism identity, damage rule, reuse unit,
fallback classes F1–F6, counter applicability, and identity witnesses are
frozen in `protocol/R5-HORSE-CORRECTNESS-PARITY.md` §6 (mizchi-markdown-
inspired; R2 MECHANISM-SOURCE-MAP H1).

Retained state: the old top-level tiling (blocks + blank runs) + the
definitions array. Update: strict-overlap damage scan (insertion-gap
mapping at exact boundaries), clean reparse of ONLY the damaged region over
`markit-mdbench-shared-grammar`, prefix pass-through, suffix delta-shift
reconstruction, and conservative soundness guards firing a counted total
fallback. Correctness gate: `H1 update result == H0 clean authoritative
parse`, proven by this crate's test suite (`tests/h1_gate.rs`).
