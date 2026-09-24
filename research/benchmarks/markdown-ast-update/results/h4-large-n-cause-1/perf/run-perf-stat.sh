#!/usr/bin/env bash
# H4-LARGE-N-CAUSE-1 — PMU collection (Issue #50 §12).
#
# PMU scope is the RESIDENT UPDATE ONLY. The counted window is opened and
# closed by the harness through `perf stat --delay=-1
# --control=fifo:<ctl>,<ack>` with a synchronous ack per command:
# construction, the oracle, the checksum and the final result destruction
# all happen outside the window.
#
# Groups are kept small so that no group multiplexes. The frozen schedule
# discipline (balanced randomized mixed cells) is used, because a
# homogeneous tight loop measures a different latency for the same update.
#
# Two privilege modes:
#   user  unprivileged; perf_event_paranoid=2 restricts events to :u
#   root  via authorized sudo (host credential supplied by the operator for
#         this diagnostic work). No kernel setting is changed.
set -uo pipefail

ROOT="${1:?usage: run-perf-stat.sh <benchmark_root> [user|root|both]}"
MODE="${2:-both}"
cd "$ROOT" || exit 2

BIN=results/h4-large-n-cause-1/bin/mdbench-h4diag
OUT=results/h4-large-n-cause-1/perf/stat
ROUNDS="${ROUNDS:-15}"
CELLS="${CELLS:-1MiB,4MiB,8MiB,16MiB}"
mkdir -p "$OUT"

PERF_GROUPS=(
  "groupA|cycles,instructions,branches,branch-misses"
  "groupB|cache-references,cache-misses"
  "groupC|LLC-loads,LLC-load-misses,dTLB-loads,dTLB-load-misses"
  "groupS|context-switches,cpu-migrations,page-faults,minor-faults,major-faults"
)

run_one() {
  local privilege="$1" name="$2" events="$3"
  local ctl="/tmp/h4diag-ctl-$$-$name" ack="/tmp/h4diag-ack-$$-$name"
  rm -f "$ctl" "$ack"; mkfifo "$ctl" "$ack"
  local tag="${privilege}-${name}"
  local json="$OUT/${tag}.json" log="$OUT/${tag}.txt"

  echo "--- perf stat ${tag}: events=${events} ---"
  if [ "$privilege" = root ]; then
    ( echo "${SUDO_PW:?SUDO_PW not set}" | sudo -S taskset -c 1 perf stat \
        --delay=-1 --control="fifo:${ctl},${ack}" -e "$events" --json -o "$json" -- \
        "$BIN" pmu-run --cells "$CELLS" --rounds "$ROUNDS" \
          --ctl-fifo "$ctl" --ack-fifo "$ack" --out "$OUT/${tag}.jsonl" ) >"$log" 2>&1
  else
    taskset -c 1 perf stat --delay=-1 --control="fifo:${ctl},${ack}" \
        -e "$events" --json -o "$json" -- \
        "$BIN" pmu-run --cells "$CELLS" --rounds "$ROUNDS" \
          --ctl-fifo "$ctl" --ack-fifo "$ack" --out "$OUT/${tag}.jsonl" >"$log" 2>&1
  fi
  local code=$?
  echo "exit_code=$code"
  rm -f "$ctl" "$ack"
  return $code
}

echo "PERF_STAT_START utc=$(date -u +%Y-%m-%dT%H:%M:%SZ) loadavg=$(cat /proc/loadavg)"
echo "perf_event_paranoid=$(cat /proc/sys/kernel/perf_event_paranoid)"
echo "cells=$CELLS rounds=$ROUNDS"
sha256sum "$BIN" | tee "$OUT/executable-sha256.txt"

for spec in "${PERF_GROUPS[@]}"; do
  gname="${spec%%|*}"
  gev="${spec#*|}"
  echo "group=${gname} events=${gev}"
  if [ "$MODE" = user ] || [ "$MODE" = both ]; then
    run_one user "$gname" "$gev" || echo "PERF_GROUP_FAILED user-$gname"
  fi
  if [ "$MODE" = root ] || [ "$MODE" = both ]; then
    run_one root "$gname" "$gev" || echo "PERF_GROUP_FAILED root-$gname"
  fi
done

echo "PERF_STAT_COMPLETE utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
