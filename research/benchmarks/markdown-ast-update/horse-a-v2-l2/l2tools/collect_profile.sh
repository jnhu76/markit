#!/usr/bin/env bash
# Issue #100 L2 — single paired-profile collector (frozen sampling contract).
#
# Usage: collect_profile.sh <CELL> <ARM> <REPS> <WARMUP> <RUNTAG> <OUTDIR>
#
# FROZEN sampling contract (identical for every profile, both arms):
#   event        cycles:u (user-space CPU cycles, the only L2 event)
#   period       fixed -c 100000 (one sample per ~100k cycles; no adaptive
#                frequency, identical across P1/P0 and all cells)
#   call chain   --call-graph dwarf (DWARF/eh_frame unwinding; see the
#                pilot gate before this was frozen)
#   affinity     taskset -c 2 (same pinned core for every profile)
# Run from the markdown-ast-update workspace root. Raw perf.data and the
# perf script dump stay on the host (git-ignored); folded stacks, the
# receipt and checksums are the committed evidence.
set -euo pipefail

CELL=$1; ARM=$2; REPS=$3; WARMUP=$4; RUNTAG=$5; OUT=$6
BIN=target/release-l2-profile-v1/mdbench-horse-a-v2-l2
PERF_FLAGS="-e cycles:u -c 100000 --call-graph fp"

mkdir -p "$OUT/raw" "$OUT/mid" "$OUT/folded" "$OUT/receipts"
NAME="${CELL}_${ARM}_${RUNTAG}"

taskset -c 2 perf record $PERF_FLAGS -o "$OUT/raw/perf_${NAME}.data" -- \
  "$BIN" --cell "$CELL" --arm "$ARM" --reps "$REPS" --warmup "$WARMUP" \
  > "$OUT/receipts/run_${NAME}.json"

perf script --inline -i "$OUT/raw/perf_${NAME}.data" > "$OUT/mid/perf_${NAME}.txt"

python3 horse-a-v2-l2/l2tools/perf_fold.py \
  --in "$OUT/mid/perf_${NAME}.txt" \
  --out "$OUT/folded/perf_${NAME}.folded" \
  --quality "$OUT/receipts/quality_${NAME}.json"

sha256sum "$OUT/raw/perf_${NAME}.data" "$OUT/mid/perf_${NAME}.txt" \
  "$OUT/folded/perf_${NAME}.folded" "$BIN" \
  > "$OUT/receipts/sha256_${NAME}.txt"

echo "collected $NAME"
