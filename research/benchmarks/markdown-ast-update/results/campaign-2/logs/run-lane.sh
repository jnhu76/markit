#!/usr/bin/env bash
# Campaign-2 formal lane runner.
#
# Every formal run is:
#   - executed through the frozen Campaign-2 binary,
#   - pinned to the frozen single CPU (cpu1 = physical core 1,
#     SMT siblings 1,11, NUMA node 0) with `taskset`,
#   - guarded by an executable SHA256 check taken before the run,
#   - logged in full under results/campaign-2/logs/.
#
# The script does not delete, retry or repair anything: a non-zero exit
# stops the lane and leaves the partial raw file in place.
set -uo pipefail

ROOT="${1:?usage: run-lane.sh <benchmark_root> <lane-name> -- <command...>}"
LANE="${2:?lane name}"
shift 2
[ "${1:-}" = "--" ] && shift

cd "$ROOT" || exit 2

BIN=./target/release/mdbench-campaign2
EXPECTED_SHA="${CAMPAIGN2_EXPECTED_SHA:-}"
LOG="results/campaign-2/logs/${LANE}.log"
mkdir -p results/campaign-2/logs

{
  echo "=== LANE $LANE ==="
  echo "utc_start=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host=$(hostname)"
  echo "command=$*"
  echo "affinity=taskset -c 1"

  if [ ! -x "$BIN" ]; then
    echo "BINARY_MISSING $BIN"
    echo "EXIT_CODE=2"
    exit 2
  fi
  ACTUAL_SHA=$(sha256sum "$BIN" | cut -d' ' -f1)
  echo "executable_sha256=$ACTUAL_SHA"
  if [ -n "$EXPECTED_SHA" ] && [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
    echo "EXECUTABLE_SHA_MISMATCH expected=$EXPECTED_SHA actual=$ACTUAL_SHA"
    echo "EXIT_CODE=3"
    exit 3
  fi
  if [ -n "$EXPECTED_SHA" ]; then
    echo "EXECUTABLE_SHA_OK"
  fi

  taskset -c 1 "$BIN" "$@"
  CODE=$?
  echo "EXIT_CODE=$CODE"
  echo "utc_end=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  exit $CODE
} 2>&1 | tee "$LOG"
