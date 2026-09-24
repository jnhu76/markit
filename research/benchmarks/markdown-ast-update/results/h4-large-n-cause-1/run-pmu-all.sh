#!/usr/bin/env bash
# PMU lane: perf stat (per cell x group x privilege) then perf record pair.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export SUDO_PW="${SUDO_PW:?SUDO_PW required}"
./results/h4-large-n-cause-1/perf/run-perf-stat-per-cell.sh "$ROOT" both
stat_rc=$?
./results/h4-large-n-cause-1/perf/run-perf-record.sh "$ROOT"
record_rc=$?
echo "PMU_ALL_STAT_RC=$stat_rc RECORD_RC=$record_rc"
echo "PMU_ALL_DONE"
