#!/usr/bin/env bash
# Issue #50 §14 — eBPF capability audit. Read-only: it changes no kernel
# setting and installs nothing.
set -uo pipefail
OUT="$(cd "$(dirname "$0")/.." && pwd)/capability.txt"
{
  echo "=== utc ==="; date -u +%Y-%m-%dT%H:%M:%SZ
  echo; echo "=== kernel ==="; uname -a
  echo; echo "=== /proc/sys/kernel/unprivileged_bpf_disabled ==="
  cat /proc/sys/kernel/unprivileged_bpf_disabled 2>&1
  echo; echo "=== /proc/sys/kernel/perf_event_paranoid ==="
  cat /proc/sys/kernel/perf_event_paranoid 2>&1
  echo; echo "=== bpftrace ==="; command -v bpftrace || echo "NOT_FOUND"
  bpftrace --version 2>&1 || true
  echo; echo "=== bpftool ==="; command -v bpftool || echo "NOT_FOUND"
  bpftool version 2>&1 || true
  echo; echo "=== BTF ==="
  test -r /sys/kernel/btf/vmlinux && echo BTF_AVAILABLE || echo BTF_UNAVAILABLE
  echo; echo "=== tracefs / debugfs ==="
  mount | grep -E 'tracefs|debugfs' || echo "NOT_MOUNTED"
  echo; echo "=== tracepoint inventory (sched / page-fault / syscall-relevant) ==="
  ls /sys/kernel/tracing/events/sched/ 2>/dev/null | head -30 || echo "(unavailable)"
  ls /sys/kernel/tracing/events/exceptions/ 2>/dev/null | head -10 || echo "(no exceptions group)"
  echo; echo "=== non-interactive sudo ==="
  sudo -n true 2>&1 && echo "SUDO_NONINTERACTIVE_OK" || echo "SUDO_NONINTERACTIVE_UNAVAILABLE"
  echo; echo "=== identity / capabilities ==="
  id
  capsh --print 2>/dev/null | head -6 || grep Cap /proc/self/status
  echo; echo "=== eBPF program loaders present ==="
  command -v clang || echo "clang NOT_FOUND"
  command -v llc || echo "llc NOT_FOUND"
  ls /usr/include/bpf/bpf.h 2>/dev/null || echo "libbpf headers NOT_FOUND"
  command -v python3 || true
  python3 -c "import bcc; print('python bcc available')" 2>/dev/null || echo "python bcc NOT_FOUND"
} > "$OUT" 2>&1
echo "wrote $OUT"
