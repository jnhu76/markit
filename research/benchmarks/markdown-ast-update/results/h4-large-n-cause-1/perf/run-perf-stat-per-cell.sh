#!/usr/bin/env bash
# H4-LARGE-N-CAUSE-1 — PMU collection, PER CELL (Issue #50 §12).
#
# One `perf stat` invocation per (group, cell), so every count is
# attributable to exactly one N. PMU scope is the RESIDENT UPDATE ONLY:
# the window is opened/closed by the harness through
# `perf stat --delay=-1 --control=fifo:<ctl>,<ack>` with a synchronous ack,
# so construction, the oracle, the checksum and the final result
# destruction are all outside the counted window.
#
# Groups are small enough that none multiplexes:
#   groupA  cycles, instructions, branches, branch-misses
#   groupB  cache-references, cache-misses
#   groupC1 LLC-loads, LLC-load-misses
#   groupC2 dTLB-loads, dTLB-load-misses
#   groupS  context-switches, cpu-migrations, page-faults, minor-faults,
#           major-faults
#
# `groupC` (all four LLC+TLB events together) was ATTEMPTED FIRST and is
# kept in the record: it reported pcnt-running 49-75 %, i.e.
# PMU_GROUP_UNRELIABLE, and was split exactly as Issue #50 §12 requires.
#
# Privilege modes:
#   user  unprivileged; perf_event_paranoid=2 restricts events to :u
#   root  authorized sudo (host credential supplied by the operator for
#         this diagnostic work); kernel-inclusive, no policy change
set -uo pipefail

ROOT="${1:?usage: run-perf-stat-per-cell.sh <benchmark_root> [user|root|both]}"
MODE="${2:-both}"
cd "$ROOT" || exit 2

BIN=results/h4-large-n-cause-1/bin/mdbench-h4diag
OUT=results/h4-large-n-cause-1/perf/stat
ROUNDS="${ROUNDS:-15}"
CELL_LIST="${CELL_LIST:-128KiB 256KiB 512KiB 1MiB 2MiB 4MiB 8MiB 16MiB}"
mkdir -p "$OUT"

PERF_GROUPS=(
  "groupA|cycles,instructions,branches,branch-misses"
  "groupB|cache-references,cache-misses"
  "groupC1|LLC-loads,LLC-load-misses"
  "groupC2|dTLB-loads,dTLB-load-misses"
  "groupS|context-switches,cpu-migrations,page-faults,minor-faults,major-faults"
)

run_one() {
  local privilege="$1" gname="$2" gev="$3" cell="$4"
  local ctl="/tmp/h4diag-pc-$$" ack="/tmp/h4diag-pa-$$"
  rm -f "$ctl" "$ack"; mkfifo "$ctl" "$ack"
  local tag="${privilege}-${gname}-${cell}"
  local json="$OUT/${tag}.json" log="$OUT/${tag}.txt"

  if [ "$privilege" = root ]; then
    ( echo "${SUDO_PW:?SUDO_PW not set}" | sudo -S taskset -c 1 perf stat \
        --delay=-1 --control="fifo:${ctl},${ack}" -e "$gev" --json -o "$json" -- \
        "$BIN" pmu-run --cells "$cell" --rounds "$ROUNDS" \
          --ctl-fifo "$ctl" --ack-fifo "$ack" --out "$OUT/${tag}.jsonl" ) >"$log" 2>&1
  else
    taskset -c 1 perf stat --delay=-1 --control="fifo:${ctl},${ack}" \
        -e "$gev" --json -o "$json" -- \
        "$BIN" pmu-run --cells "$cell" --rounds "$ROUNDS" \
          --ctl-fifo "$ctl" --ack-fifo "$ack" --out "$OUT/${tag}.jsonl" >"$log" 2>&1
  fi
  local code=$?
  rm -f "$ctl" "$ack"
  if [ $code -ne 0 ]; then echo "PERF_GROUP_FAILED $tag"; fi
  return $code
}

echo "PERF_STAT_PER_CELL_START utc=$(date -u +%Y-%m-%dT%H:%M:%SZ) loadavg=$(cat /proc/loadavg)"
echo "perf_event_paranoid=$(cat /proc/sys/kernel/perf_event_paranoid)"
echo "rounds=$ROUNDS cells=$CELL_LIST"
sha256sum "$BIN" | tee "$OUT/executable-sha256.txt"

for spec in "${PERF_GROUPS[@]}"; do
  gname="${spec%%|*}"
  gev="${spec#*|}"
  for cell in $CELL_LIST; do
    if [ "$MODE" = user ] || [ "$MODE" = both ]; then
      run_one user "$gname" "$gev" "$cell"
    fi
    if [ "$MODE" = root ] || [ "$MODE" = both ]; then
      run_one root "$gname" "$gev" "$cell"
    fi
  done
done

echo "PERF_STAT_PER_CELL_COMPLETE utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
