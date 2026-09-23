#!/usr/bin/env bash
# Campaign-2 formal collection driver.
#
# Runs every formal lane in the frozen order. STOP-ON-FIRST-FAILURE: a
# non-zero lane exit leaves its partial raw in place and stops the driver
# (task §48 — no delete, no impute, no invisible retry).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2
RUN=./results/campaign-2/logs/run-lane.sh
export CAMPAIGN2_EXPECTED_SHA="$(sha256sum ./target/release/mdbench-campaign2 | cut -d' ' -f1)"

echo "CAMPAIGN2_COLLECTION_START executable_sha256=$CAMPAIGN2_EXPECTED_SHA"
run() {
  local lane="$1"; shift
  echo "--- LANE $lane ---"
  if ! "$RUN" . "$lane" -- "$@"; then
    echo "LANE_FAILED $lane"
    echo "CAMPAIGN2_COLLECTION_STOPPED_AT $lane"
    exit 1
  fi
}

# ---- Surface A: CONSTRUCTION -------------------------------------------
for s in 0 1 2; do
  run "20-construction-session-$s" run-construction . --session "$s"
done
# ---- Surface B: RESIDENT_UPDATE (SINGLE_RESET) -------------------------
for s in 0 1 2; do
  run "30-resident-update-session-$s" run-resident-update . --session "$s"
done
# ---- Surface C: LIFECYCLE ----------------------------------------------
for s in 0 1 2; do
  run "40-lifecycle-session-$s" run-lifecycle . --session "$s"
done
# ---- Surface D: CONTROLLED (five axes x three sessions) ----------------
for axis in N B D F K; do
  for s in 0 1 2; do
    run "50-controlled-${axis}-session-$s" run-controlled . --axis "$axis" --session "$s"
  done
done
echo "CAMPAIGN2_COLLECTION_COMPLETE"
