# Attempt 3 — mixed-provenance, superseded

Lanes A (U_PLAIN), B (U_PHASE), C (W_COUNTERS), D (A_ALLOCATOR) and E
(ablations) all completed, but the default-features executable was rebuilt
afterwards (to add the PMU lane's `--mechanism original` switch), so the
binary that produced these rows is no longer the binary in `../bin/` and
its bytes cannot be reproduced.

Issue #50 §22 explicitly forbids repeating the PR #47 provenance mistake,
so this attempt is preserved here and NOT used as evidence. The final
collection (attempt 4) runs every lane against the frozen binaries in
`../bin/`, whose SHA256 are in `../executable-shas.txt`.

Note: the *allocator* files in `raw-superseded-allocator-scope/` are the
attempt-2 allocator run, superseded for a second, independent reason (its
accounting window wrongly included the post-timer oracle/checksum export).
