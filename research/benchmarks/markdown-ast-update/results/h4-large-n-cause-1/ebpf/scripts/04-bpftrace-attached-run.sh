#!/usr/bin/env bash
# A true BPF-attached benchmark run: narrow bpftrace probes attached to
# the benchmark process while it executes pmu-run, with a matched
# untraced run in the same batch (with_BPF / without_BPF ratio).
set -uo pipefail
ROOT="${1:?usage: 04-bpftrace-attached-run.sh <benchmark_root>}"
cd "$ROOT"
DIR=results/h4-large-n-cause-1/ebpf
RAW="$DIR/raw"; mkdir -p "$RAW"
BIN=results/h4-large-n-cause-1/bin/mdbench-h4diag
PW="${SUDO_PW:?SUDO_PW required}"

run_bench() {  # $1 = out jsonl
  taskset -c 1 "$BIN" pmu-run --cells 1MiB,16MiB --rounds 10 \
    --ctl-fifo /dev/null --ack-fifo /dev/null --out "$1" > /dev/null 2>&1
}

# with_BPF: bpftrace attached for the whole benchmark process
printf '%s\n' "$PW" | sudo -S -p '' bash -c '
  taskset -c 1 bpftrace -e "
  tracepoint:sched:sched_switch /args->next_comm == \"mdbench-h4diag\"/ { @sw = count(); }
  tracepoint:sched:sched_migrate_task /comm == \"mdbench-h4diag\"/ { @mig = count(); }
  tracepoint:exceptions:page_fault_user /comm == \"mdbench-h4diag\"/ { @pf = count(); }
  tracepoint:syscalls:sys_enter_mmap   /comm == \"mdbench-h4diag\"/ { @mmap = count(); }
  tracepoint:syscalls:sys_enter_munmap /comm == \"mdbench-h4diag\"/ { @munmap = count(); }
  tracepoint:syscalls:sys_enter_brk    /comm == \"mdbench-h4diag\"/ { @brk = count(); }
  tracepoint:syscalls:sys_enter_mremap /comm == \"mdbench-h4diag\"/ { @mremap = count(); }
  interval:s:60 { exit(); }
  " -o '"$RAW"'/bpftrace-counts.txt 2>&1 &
  BPID=$!
  sleep 2
  taskset -c 1 '"$BIN"' pmu-run --cells 1MiB,16MiB --rounds 10 \
    --ctl-fifo /dev/null --ack-fifo /dev/null --out '"$RAW"'/with-bpf.jsonl > /dev/null 2>&1
  wait $BPID
' > "$RAW/with-bpf-run.txt" 2>&1
echo "with_bpf_exit=$?"

# without_BPF: matched untraced run, same batch
run_bench "$RAW/without-bpf.jsonl"
echo "without_bpf_exit=$?"

python3 - <<'PY'
import json, statistics
def med(path):
    rs=[json.loads(l) for l in open(path)]
    v=[r['native_ns'] for r in rs if 'native_ns' in r]
    return statistics.median(v) if v else None
w=med('results/h4-large-n-cause-1/ebpf/raw/with-bpf.jsonl')
o=med('results/h4-large-n-cause-1/ebpf/raw/without-bpf.jsonl')
if w and o:
    print(f"with_BPF_median_native={w:,.0f}ns without_BPF={o:,.0f}ns ratio={w/o:.4f}")
PY
echo "BPF_ATTACHED_RUN_COMPLETE"
