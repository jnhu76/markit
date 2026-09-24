# Selection note — `perf record` pair and scope

**Trigger observation.** The per-cell PMU series shows IPC falling from 0.92
(128 KiB) to 0.39 (16 MiB) while instructions per update grow essentially
linearly in M, and LLC-load misses rising from ~0 % to 75 % of LLC loads.

**Two explanations to separate.**

1. The extra cycles are *new work* — more instructions executing.
2. The extra cycles are *stalls* on the same work — the O(M) operations touch
   a working set that no longer fits in cache.

**Pair chosen.** 1 MiB and 16 MiB (the issue's default). 1 MiB is the largest
point whose whole retained state still fits in the 25 MiB last-level cache
and whose LLC miss rate is ~0 %; 16 MiB is the largest point. 1 MiB needs
~15x more windows than 16 MiB to collect a comparable sample count, which is
recorded in the script.

**Scope.** The recording window is opened and closed by the harness through
`perf record --control=fifo:...`, so only the resident update region is
sampled — not the construction, not the oracle, not the checksum, not the
final result destruction. Sampling percentages are therefore treated as
qualitative *hotspot locators inside U*, never as U phase percentages.

**What would support each.** A profile whose hot symbols are the O(M)
representation loops (and not new algorithmic phases) supports explanation 2
and locates which operations the stalls belong to.
