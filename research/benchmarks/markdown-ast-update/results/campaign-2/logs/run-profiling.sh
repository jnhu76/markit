#!/usr/bin/env bash
# Campaign-2 matched profiler driver (task §34-§38).
#
# PROFILING EVIDENCE ONLY — never primary timing.
#
#   usage: run-profiling.sh <benchmark_root> <slots.tsv> <out_subdir>
#
# Slot file: TAB-separated, one slot per line, comment lines start with #:
#
#   slot_id <TAB> surface <TAB> axis <TAB> cell <TAB> case_index <TAB> trace <TAB> step <TAB> reps
#
# For every slot the driver runs, for EVERY horse in the rotated order:
#
#   1. a `--setup-only` companion (validates any whole-process number), and
#   2. the measured region replay, with the region-scoped PMU counters the
#      replay binary opens itself;
#
# then, once per slot, a `perf stat` run over the SAME replay command with
# the frozen core events, repeated PERF_STAT_REPETITIONS times with the
# horse order rotated each repetition, so every horse in a slot is
# measured the same number of times on the same CPU with the same binary.
set -uo pipefail

ROOT="${1:?usage: run-profiling.sh <benchmark_root> <slots.tsv> <out_subdir>}"
SLOTS="${2:?slot file}"
SUBDIR="${3:?output subdir}"
cd "$ROOT" || exit 2

REPLAY=./target/release/mdbench-replay
OUT="results/campaign-2/profiling/$SUBDIR"
mkdir -p "$OUT"

HARDWARE_EVENTS="cycles:u,instructions:u,branches:u,branch-misses:u,cache-references:u,cache-misses:u"
SOFTWARE_EVENTS="task-clock,page-faults,context-switches,cpu-migrations"
ALL_EVENTS="$HARDWARE_EVENTS,$SOFTWARE_EVENTS"
REPETITIONS="${PERF_STAT_REPETITIONS:-10}"

SHA=$(sha256sum "$REPLAY" | cut -d' ' -f1)
{
  echo "=== PROFILING LANE $SUBDIR ==="
  echo "utc_start=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host=$(hostname)"
  echo "profiling_binary=$REPLAY"
  echo "profiling_binary_sha256=$SHA"
  echo "campaign_executable_sha256=$(sha256sum ./target/release/mdbench-campaign2 | cut -d' ' -f1)"
  echo "events=$ALL_EVENTS"
  echo "perf_stat_repetitions=$REPETITIONS"
  echo "rotation=horse order rotated once per repetition"
  echo "affinity=taskset -c 1"
  echo "evidence_class=PROFILE_PERF"
  echo "label=POST_HOC NON_PRIMARY"
} | tee "$OUT/profiling-header.txt"

replay_args() {
  # $1 surface $2 axis $3 cell $4 case_index $5 trace $6 step $7 horse $8 extra...
  local surface="$1" axis="$2" cell="$3" case_index="$4" trace="$5" step="$6" horse="$7"
  shift 7
  printf '%s\n' . --surface "$surface" --horse "$horse" "$@"
}

while IFS='|' read -r slot surface axis cell case_index trace step reps; do
  case "$slot" in ''|'#'*) continue ;; esac
  reps="${reps:-30}"
  echo "=== SLOT $slot surface=$surface axis=$axis cell=$cell case=$case_index trace=$trace step=$step ==="

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

  # 1) region-scoped PMU counters + in-process timing, per horse.
  for horse in H0 H1 H2 H3 H4; do
    taskset -c 1 "$REPLAY" "${ARGS[@]}" --horse "$horse" --reps "$reps" \
      > "$OUT/slot-${slot}-${horse}-region.jsonl" 2> "$OUT/slot-${slot}-${horse}-region.err"
    taskset -c 1 "$REPLAY" "${ARGS[@]}" --horse "$horse" --reps "$reps" --setup-only \
      > "$OUT/slot-${slot}-${horse}-setuponly.jsonl" 2> "$OUT/slot-${slot}-${horse}-setuponly.err"
  done
  cat "$OUT"/slot-"${slot}"-*-region.jsonl > "$OUT/slot-${slot}-region.jsonl"
  cat "$OUT"/slot-"${slot}"-*-setuponly.jsonl > "$OUT/slot-${slot}-setuponly.jsonl"
  rm -f "$OUT"/slot-"${slot}"-H*-region.jsonl "$OUT"/slot-"${slot}"-H*-setuponly.jsonl

  # 2) perf stat, matched repetitions with a rotated horse order.
  : > "$OUT/slot-${slot}-perf-stat.txt"
  for rep in $(seq 0 $((REPETITIONS - 1))); do
    for offset in 0 1 2 3 4; do
      idx=$(( (rep + offset) % 5 ))
      horse="H$idx"
      {
        echo "### slot=$slot rep=$rep horse=$horse"
        perf stat -e "$ALL_EVENTS" -- taskset -c 1 "$REPLAY" "${ARGS[@]}" \
          --horse "$horse" --reps "$reps" 2>&1 | grep -vE '^\{"' | sed 's/^/    /'
      } >> "$OUT/slot-${slot}-perf-stat.txt"
    done
  done
  echo "SLOT_DONE $slot"
done < "$SLOTS"

{
  echo "utc_end=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "PROFILING_LANE_DONE $SUBDIR"
} | tee -a "$OUT/profiling-header.txt"
