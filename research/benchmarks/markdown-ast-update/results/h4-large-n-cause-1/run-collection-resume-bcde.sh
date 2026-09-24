#!/usr/bin/env bash
# H4-LARGE-N-CAUSE-1 formal collection driver (Issue #50 §3).
#
# Runs every formal lane in the frozen order. STOP-ON-FIRST-FAILURE: a
# non-zero lane exit leaves its partial raw file in place and stops the
# driver (no delete, no impute, no invisible retry).
#
# Every lane goes through run-lane.sh, so every lane log records the ACTUAL
# producing executable SHA256 and the affinity used.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT" || exit 2
RUN=./results/h4-large-n-cause-1/run-lane.sh
RAW=./results/h4-large-n-cause-1/raw

echo "H4LARGEN_COLLECTION_RESUME_START utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "loadavg_start=$(cat /proc/loadavg)"

run() {
  local lane="$1"; shift
  local bin="$1"; shift
  echo "--- LANE $lane ($bin) ---"
  if ! "$RUN" . "$lane" "$bin" -- "$@"; then
    echo "LANE_FAILED $lane"
    echo "H4LARGEN_COLLECTION_STOPPED_AT $lane"
    exit 1
  fi
}


# ---- Lane B: U_PHASE ---------------------------------------------------
for s in 0 1 2; do
  run "b-u-phase-session-$s" mdbench-h4diag-phases \
      run-phase --session "$s" \
      --out "$RAW/u-phase-session-$s.jsonl" \
      --schedule-out "$RAW/u-phase-session-$s.schedule.jsonl"
done

# ---- Lane C: W_COUNTERS ------------------------------------------------
run "c-work-counters" mdbench-h4diag-counters \
    run-counters --session 0 --out "$RAW/work-counters.jsonl"

# ---- Lane D: A_ALLOCATOR ----------------------------------------------
for s in 0 1 2; do
  run "d-allocator-session-$s" mdbench-h4diag-alloc \
      run-alloc --session "$s" \
      --out "$RAW/allocator-session-$s.jsonl" \
      --schedule-out "$RAW/allocator-session-$s.schedule.jsonl"
done

# ---- Ablations (A0 / A0-dup / Adefs / Adrop / Acapacity) ---------------
for s in 0 1 2; do
  run "e-ablations-session-$s" mdbench-h4diag \
      run-ablations --session "$s" \
      --out "$RAW/ablations-session-$s.jsonl" \
      --schedule-out "$RAW/ablations-session-$s.schedule.jsonl"
done

echo "loadavg_end=$(cat /proc/loadavg)"
echo "utc_end=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "H4LARGEN_COLLECTION_COMPLETE"
