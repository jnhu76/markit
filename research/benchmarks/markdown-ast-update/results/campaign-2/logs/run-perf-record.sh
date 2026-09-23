#!/usr/bin/env bash
# Campaign-2 `perf record` lane (task §37).
#
# Records ONLY the slots whose primary gap the region-scoped counters did
# not explain. Release-optimized binary (release-primary-v1 profile; no
# debug-info rebuild, so symbol resolution comes from the binary's
# symbol table). `perf record` OUTPUT IS NEVER MIXED INTO PRIMARY TIMING.
set -uo pipefail

ROOT="${1:?usage: run-perf-record.sh <benchmark_root> <slots.tsv> <out_subdir>}"
SLOTS="${2:?slot file}"
SUBDIR="${3:?output subdir}"
cd "$ROOT" || exit 2

REPLAY=./target/release/mdbench-replay
OUT="results/campaign-2/profiling/$SUBDIR"
mkdir -p "$OUT"

SHA=$(sha256sum "$REPLAY" | cut -d' ' -f1)
{
  echo "=== PERF RECORD LANE $SUBDIR ==="
  echo "utc_start=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host=$(hostname)"
  echo "profiling_binary=$REPLAY"
  echo "profiling_binary_sha256=$SHA"
  echo "flags=release-primary-v1 (opt-level 3, lto thin, codegen-units 1); no debug-info rebuild"
  echo "event=cycles:u  frequency=999Hz  call-graph=dwarf (falls back to flat symbols)"
  echo "affinity=taskset -c 1"
  echo "evidence_class=PROFILE_PERF"
  echo "label=POST_HOC NON_PRIMARY"
} | tee "$OUT/perf-record-header.txt"

while IFS='|' read -r slot surface axis cell case_index trace step reps; do
  case "$slot" in ''|'#'*) continue ;; esac
  echo "=== RECORD SLOT $slot ==="

  ARGS=(.)
  if [ "$surface" = "controlled" ]; then
    ARGS+=(--surface controlled --axis "$axis" --cell "$cell")
  elif [ "$surface" = "construction" ]; then
    ARGS+=(--surface construction --case-index "$case_index")
  elif [ "$surface" = "resident-update" ]; then
    ARGS+=(--surface resident-update --case-index "$case_index")
  elif [ "$surface" = "lifecycle" ]; then
    ARGS+=(--surface lifecycle --trace "$trace" --step "$step")
  fi

  for horse in H0 H1 H2 H3 H4; do
    DATA="$OUT/slot-${slot}-${horse}.data"
    rm -f "$DATA"
    taskset -c 1 perf record -o "$DATA" -e cycles:u -F 999 --call-graph dwarf \
      -- "$REPLAY" "${ARGS[@]}" --horse "$horse" --reps "$reps" \
      > "$OUT/slot-${slot}-${horse}-record.stdout" 2>&1
    perf report -i "$DATA" --stdio --no-children -g none 2>/dev/null \
      | head -60 > "$OUT/slot-${slot}-${horse}-report.txt"
    perf report -i "$DATA" --stdio --children -g none 2>/dev/null \
      | head -40 > "$OUT/slot-${slot}-${horse}-report-children.txt"
    echo "  recorded $horse -> $(basename "$DATA") ($(stat -c%s "$DATA" 2>/dev/null || echo 0) bytes)"
  done
  echo "SLOT_RECORDED $slot"
done < "$SLOTS"

{
  echo "utc_end=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "PERF_RECORD_LANE_DONE $SUBDIR"
} | tee -a "$OUT/perf-record-header.txt"
