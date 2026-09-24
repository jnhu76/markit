# Selection note — PMU N points

**Trigger observation.** The U_PLAIN curve is smooth over the full range but
its *character* changes: the p50 grows 2.0x per doubling up to 1 MiB, then
2.1x, 3.1x, 2.6x and 2.1x per doubling above it, and the session-to-session
p50 spread grows from <= 0.6 % at 128 KiB-1 MiB to 12-22 % at 4-16 MiB.

**Two explanations to separate.**

1. The growth is a *scale* effect that is already visible at 1 MiB and simply
   continues.
2. There is a *transition* somewhere in the 1-16 MiB range where a new
   mechanism (working set leaving the last-level cache) starts to dominate.

**Points chosen.** The issue's default 1 / 4 / 8 / 16 MiB, plus **all eight N
points** for the per-cell PMU collection, because a per-cell series costs
little and shows the transition directly instead of forcing a choice between
two candidate pairs. No preregistered transition pair was needed to justify
a smaller set, so the full set was used.

**What would support each.** A cache-miss / LLC-miss rate that is flat in N
supports explanation 1. A miss rate that rises steeply over a narrow N band
supports explanation 2 and identifies the band.
