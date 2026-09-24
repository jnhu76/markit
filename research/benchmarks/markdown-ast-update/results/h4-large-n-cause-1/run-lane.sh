#!/usr/bin/env bash
# H4-LARGE-N-CAUSE-1 formal lane runner (Issue #50).
#
# Every formal run:
#   - is executed through one of the four diagnostic executables,
#   - is pinned to cpu1 (physical core 1, SMT siblings 1,11), the same
#     frozen lane CPU Campaign-2 used,
#   - records the ACTUAL producing executable SHA256 in its log, so no
#     artifact can be attributed to a binary that did not produce it,
#   - does not delete, retry or repair anything: a non-zero exit stops the
#     lane and leaves the partial raw file in place.
set -uo pipefail

ROOT="${1:?usage: run-lane.sh <benchmark_root> <lane-name> <binary> -- <command...>}"
LANE="${2:?lane name}"
BIN_NAME="${3:?binary name}"
shift 3
[ "${1:-}" = "--" ] && shift

cd "$ROOT" || exit 2

BIN="results/h4-large-n-cause-1/bin/${BIN_NAME}"
LOG="results/h4-large-n-cause-1/logs/${LANE}.log"
mkdir -p results/h4-large-n-cause-1/logs

{
  echo "=== LANE $LANE ==="
  echo "utc_start=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host=$(hostname)"
  echo "binary=$BIN"
  echo "command=$*"
  echo "affinity=taskset -c 1"
  echo "loadavg_start=$(cat /proc/loadavg)"

  if [ ! -x "$BIN" ]; then
    echo "BINARY_MISSING $BIN"
    echo "EXIT_CODE=2"
    exit 2
  fi
  ACTUAL_SHA=$(sha256sum "$BIN" | cut -d' ' -f1)
  echo "executable_sha256=$ACTUAL_SHA"
  # The binary is a FROZEN COPY in results/.../bin/, never the mutable
  # cargo output: building this package without --bin would otherwise
  # overwrite every feature-specific binary with the default build.
  FEATURES=$("$BIN" receipt --lane feature-check 2>/dev/null | python3 -c "import sys,json;print(json.loads(sys.stdin.readline())['features'])" 2>/dev/null)
  echo "executable_features=$FEATURES"

  taskset -c 1 "$BIN" "$@"
  CODE=$?
  echo "EXIT_CODE=$CODE"
  echo "loadavg_end=$(cat /proc/loadavg)"
  echo "utc_end=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  exit $CODE
} 2>&1 | tee "$LOG"
