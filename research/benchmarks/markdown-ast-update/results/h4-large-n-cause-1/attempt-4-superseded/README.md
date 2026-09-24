# Attempt 4 — superseded

All five lanes completed against the frozen binaries, and every lane log
recorded its producing executable SHA. The attempt is superseded because the
PMU lane's window placement was then corrected: in the rewritten `pmu-run`,
`control.enable()` had been placed before the fresh-state construction, so
the counted window contained the construction as well as the update (visible
as ~20x inflated instruction counts). Fixing it changed `cli.rs`, hence the
default-features executable, hence the provenance of every lane that uses it.

These rows are preserved and NOT used as evidence. The final collection
(attempt 5) runs every lane against the binaries in `../bin/`.

## Superseded perf artifacts (archived 2026-09-24)

`perf/stat/` (80 jsonl: 5 groups x 8 cells x {user,root}) and
`perf/record/` (1MiB + 16MiB) were produced by the attempt-4 default
binary `095c03d814c4a3e3574aafd354a90a185a37a5d6fa585d9346c54f2aca492a0a`,
which predates the `pmu-run` window-ordering fix (state construction
moved before `control.enable()`). The window-contamination defect does
not change U_PLAIN/U_PHASE/counters/ablations/allocator semantics, but
perf provenance must match the final frozen binary set, so both trees
were re-collected against `4d23df55...` after attempt 5.
