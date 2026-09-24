#!/usr/bin/env bash
# H4-LARGE-N-CAUSE-1 — perf record on the smallest discriminating pair
# (Issue #50 §13).
#
# Scope: like `perf stat`, the recording window is opened and closed by the
# harness through `perf record --control=fifo:...`, so only the resident
# update region is sampled. Construction, oracle, checksum and the final
# result destruction are excluded.
#
# Reports carry symbol / self % / children % / DSO, and call graphs where
# practical. Sampling percentages are treated as QUALITATIVE: they locate
# implementation hotspots inside U, they are not U phase percentages.
set -uo pipefail

ROOT="${1:?usage: run-perf-record.sh <benchmark_root>}"
cd "$ROOT" || exit 2

BIN=results/h4-large-n-cause-1/bin/mdbench-h4diag
OUT=results/h4-large-n-cause-1/perf/record
mkdir -p "$OUT"
sha256sum "$BIN" | tee "$OUT/executable-sha256.txt"

# (cell, rounds) -- 1 MiB needs many more windows than 16 MiB to collect a
# comparable number of samples, because its update is ~37x shorter.
PAIRS=("1MiB 300" "16MiB 20")

for pair in "${PAIRS[@]}"; do
  cell="${pair%% *}"
  rounds="${pair#* }"
  ctl="/tmp/h4diag-rc-$$"; ack="/tmp/h4diag-ra-$$"
  rm -f "$ctl" "$ack"; mkfifo "$ctl" "$ack"
  echo "--- perf record ${cell} rounds=${rounds} ---"
  ( echo "${SUDO_PW:?SUDO_PW not set}" | sudo -S taskset -c 1 perf record \
      --delay=-1 --control="fifo:${ctl},${ack}" \
      -F 4000 --call-graph dwarf,8192 \
      -o "$OUT/${cell}.data" -- \
      "$BIN" pmu-run --cells "$cell" --rounds "$rounds" \
        --ctl-fifo "$ctl" --ack-fifo "$ack" --out "$OUT/${cell}.jsonl" ) \
      > "$OUT/${cell}.record.txt" 2>&1
  echo "exit=$?"
  rm -f "$ctl" "$ack"
  echo "${SUDO_PW}" | sudo -S chown "$(id -u):$(id -g)" "$OUT/${cell}.data" 2>/dev/null || true
  perf report --stdio -i "$OUT/${cell}.data" --sort symbol,dso \
      --percent-limit 0.3 > "$OUT/${cell}.report-symbol.txt" 2>&1
  perf report --stdio -i "$OUT/${cell}.data" -g --percent-limit 0.3 \
      > "$OUT/${cell}.report-callgraph.txt" 2>&1
  perf report --stdio -i "$OUT/${cell}.data" --sort dso \
      > "$OUT/${cell}.report-dso.txt" 2>&1
  perf report --stdio -i "$OUT/${cell}.data" --sort symbol \
      --percent-limit 0.0 --stdio > "$OUT/${cell}.report-all.txt" 2>&1
  echo "samples:"; grep -m1 "Samples:" "$OUT/${cell}.report-symbol.txt" || true
done

echo "PERF_RECORD_COMPLETE utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
