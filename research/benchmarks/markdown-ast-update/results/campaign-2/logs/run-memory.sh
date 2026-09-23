#!/usr/bin/env bash
# Campaign-2 DESCRIPTIVE_PROCESS_MEMORY lane (task §30).
#
# Process-isolated: each mode is a separate process invocation, so the
# samples describe that process and are never blended across modes.
# RSS is a process observation, NOT an object size.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2
BIN=./target/release/mdbench-campaign2
OUT=results/campaign-2/memory
mkdir -p "$OUT"

SHA=$(sha256sum "$BIN" | cut -d' ' -f1)
{
  echo "=== MEMORY LANE ==="
  echo "utc_start=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host=$(hostname)"
  echo "executable_sha256=$SHA"
  echo "methodology=in-process /proc/self/status (VmRSS, VmHWM, VmSize) + /proc/self/statm;"
  echo "methodology_note=probes run strictly between operations; no allocator interposition,"
  echo "methodology_note=no instrumentation inside any timed region; label DESCRIPTIVE_PROCESS_MEMORY"
  echo "affinity=taskset -c 1"
  for mode in construction resident_update; do
    echo "--- mode=$mode ---"
    taskset -c 1 "$BIN" run-memory . --mode "$mode" --out "$OUT/memory-$mode.jsonl"
    echo "mode_exit_code=$?"
  done
  echo "utc_end=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} 2>&1 | tee results/campaign-2/logs/70-memory.log

# Machine-readable copy of the same lane, for the inventory.
taskset -c 1 "$BIN" run-memory . --mode construction \
  --out "$OUT/memory-construction.jsonl" > /dev/null 2>&1
echo "MEMORY_LANE_DONE"
