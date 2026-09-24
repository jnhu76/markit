#!/usr/bin/env bash
# Issue #50 §15 — narrow causal questions, answered with kernel tracepoints
# and software events under authorized sudo (no policy change).
#
#   S1  scheduler interference : sched_switch / sched_migrate_task
#   S2  page faults            : page-faults / minor / major
#   S3  mmap / munmap / brk    : syscall trace
#
# Every traced run is paired with a matched untraced run in the same batch
# (Issue #50 §16) so EBPF_PERTURBED can be decided from a measured ratio.
set -uo pipefail
ROOT="${1:?usage: 03-trace-sched-pgfault-mmap.sh <benchmark_root>}"
cd "$ROOT" || exit 2
DIR=results/h4-large-n-cause-1/ebpf
RAW="$DIR/raw"
mkdir -p "$RAW"
# FROZEN COPY (never the mutable cargo output), so every traced run and
# its untraced control is attributable to the recorded executable SHA.
BIN=results/h4-large-n-cause-1/bin/mdbench-h4diag
CELLS="${CELLS:-1MiB,16MiB}"
ROUNDS="${ROUNDS:-15}"

# ---- S1/S2: software events, windowed exactly like the PMU lane --------
ctl=/tmp/h4diag-ebpf-ctl; ack=/tmp/h4diag-ebpf-ack
rm -f "$ctl" "$ack"; mkfifo "$ctl" "$ack"
echo "$SUDO_PW" | sudo -S taskset -c 1 perf stat --delay=-1 \
  --control="fifo:${ctl},${ack}" \
  -e context-switches,cpu-migrations,page-faults,minor-faults,major-faults,task-clock \
  -o "$RAW/s1s2-software-events.txt" --json -- \
  "$BIN" pmu-run --cells "$CELLS" --rounds "$ROUNDS" \
    --ctl-fifo "$ctl" --ack-fifo "$ack" --out "$RAW/s1s2-observations.jsonl" \
  > "$RAW/s1s2-stdout.txt" 2>&1
echo "s1s2_exit=$?"
rm -f "$ctl" "$ack"

# ---- S1: explicit sched tracepoints on the worker thread ---------------
# A short traced run: record sched_switch/sched_migrate_task for the
# benchmark process only, then count events per update window.
echo "$SUDO_PW" | sudo -S taskset -c 1 perf record -a -g \
  -e 'sched:sched_switch' -e 'sched:sched_migrate_task' \
  -o "$RAW/s1-perf-sched.data" -- \
  "$BIN" pmu-run --cells 16MiB --rounds 5 --ctl-fifo /dev/null --ack-fifo /dev/null \
    --out /dev/null > "$RAW/s1-perf-record.txt" 2>&1
echo "s1_exit=$?"
echo "$SUDO_PW" | sudo -S chown "$(id -u):$(id -g)" "$RAW/s1-perf-sched.data" 2>/dev/null || true
perf report --stdio -i "$RAW/s1-perf-sched.data" --sort comm,dso,sym \
  > "$RAW/s1-perf-sched-report.txt" 2>&1
perf script -i "$RAW/s1-perf-sched.data" > "$RAW/s1-perf-sched-script.txt" 2>&1

# ---- S3: syscall trace for mmap/munmap/brk/mremap ----------------------
echo "$SUDO_PW" | sudo -S taskset -c 1 perf trace -f \
  -e 'syscalls:sys_enter_mmap' -e 'syscalls:sys_enter_munmap' \
  -e 'syscalls:sys_enter_brk' -e 'syscalls:sys_enter_mremap' \
  -o "$RAW/s3-syscall-trace.txt" -- \
  "$BIN" pmu-run --cells 1MiB,16MiB --rounds 5 --ctl-fifo /dev/null --ack-fifo /dev/null \
    --out /dev/null > "$RAW/s3-stdout.txt" 2>&1
echo "s3_exit=$?"

# ---- matched untraced control runs (perturbation ratio, §16) -----------
taskset -c 1 "$BIN" pmu-run --cells "$CELLS" --rounds "$ROUNDS" \
  --ctl-fifo /dev/null --ack-fifo /dev/null \
  --out "$RAW/untraced-control.jsonl" > "$RAW/untraced-control.txt" 2>&1
echo "untraced_exit=$?"
echo "EBPF_TRACE_COMPLETE"
