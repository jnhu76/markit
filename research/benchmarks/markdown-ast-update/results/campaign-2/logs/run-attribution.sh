#!/usr/bin/env bash
# Campaign-2 attribution lane (TIMING and ATTRIBUTION stay separate lanes;
# this driver runs the untimed work-counter lane).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2
RUN=./results/campaign-2/logs/run-lane.sh
export CAMPAIGN2_EXPECTED_SHA="$(sha256sum ./target/release/mdbench-campaign2 | cut -d' ' -f1)"
OUT=results/campaign-2

run() {
  local lane="$1"; shift
  echo "--- LANE $lane ---"
  if ! "$RUN" . "$lane" -- "$@"; then
    echo "LANE_FAILED $lane"
    exit 1
  fi
}

run "60-attribution-construction" run-attribution . --surface construction \
  --out "$OUT/construction/attribution-construction.jsonl"
run "61-attribution-resident-update" run-attribution . --surface resident_update \
  --out "$OUT/resident-update/attribution-resident-update.jsonl"
for axis in N B D F K; do
  case "$axis" in
    N) TAG=N ;; B) TAG=B ;; D) TAG=D-fence ;; F) TAG=F-reference ;; K) TAG=K-container ;;
  esac
  run "62-attribution-controlled-$axis" run-attribution . --surface controlled --axis "$axis" \
    --out "$OUT/controlled/$TAG/attribution-$axis.jsonl"
done
echo "CAMPAIGN2_ATTRIBUTION_COMPLETE"
