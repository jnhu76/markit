#!/usr/bin/env bash
# Issue #50 §14/§15 — attempt an actual eBPF load.
#
# This script only ATTEMPTS; it never weakens a kernel setting
# (unprivileged_bpf_disabled and perf_event_paranoid stay untouched).
# The load attempt runs under authorized sudo because
# unprivileged_bpf_disabled=2 blocks unprivileged BPF; changing that
# sysctl for the experiment's convenience is forbidden by the task text.
set -uo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$DIR/raw/attempt-ebpf-load.txt"
mkdir -p "$DIR/raw"
SUDO_PW="${SUDO_PW:-}"
{
  echo "=== utc ==="; date -u +%Y-%m-%dT%H:%M:%SZ
  echo "unprivileged_bpf_disabled=$(cat /proc/sys/kernel/unprivileged_bpf_disabled)"
  echo "perf_event_paranoid=$(cat /proc/sys/kernel/perf_event_paranoid)"
  echo "btf=$([ -r /sys/kernel/btf/vmlinux ] && echo AVAILABLE || echo UNAVAILABLE)"
  for tool in bpftrace bpftool; do
    if command -v "$tool" >/dev/null 2>&1; then
      echo "$tool=PRESENT $(command -v "$tool") ($("$tool" --version 2>&1 | head -1))"
    else
      echo "$tool=ABSENT"
    fi
  done

  attempt=0
  if command -v bpftrace >/dev/null 2>&1; then
    echo "--- attempt 1: bpftrace trivial BEGIN program (sudo) ---"
    if [ -n "$SUDO_PW" ]; then
      printf '%s\n' "$SUDO_PW" | sudo -S -p '' timeout 60 bpftrace -e 'BEGIN { printf("BPF_LOAD_OK\n"); exit(); }' 2>&1
      rc=${PIPESTATUS[0]}
    else
      timeout 60 bpftrace -e 'BEGIN { printf("BPF_LOAD_OK\n"); exit(); }' 2>&1
      rc=$?
    fi
    echo "bpftrace_trivial_rc=$rc"
    if [ "${rc:-1}" -eq 0 ] && ! [ "${rc:-1}" -eq 124 ]; then
      attempt=1
      echo "--- attempt 2: tracepoint probe sched:sched_switch (sudo) ---"
      if [ -n "$SUDO_PW" ]; then
        printf '%s\n' "$SUDO_PW" | sudo -S -p '' timeout 90 \
          bpftrace -e 'tracepoint:sched:sched_switch { @switches = count(); } interval:s:1 { exit(); }' 2>&1 | tail -5
        rc2=${PIPESTATUS[0]}
      else
        timeout 90 bpftrace -e 'tracepoint:sched:sched_switch { @switches = count(); } interval:s:1 { exit(); }' 2>&1 | tail -5
        rc2=$?
      fi
      echo "bpftrace_tracepoint_rc=$rc2"
      if [ "${rc2:-1}" -eq 0 ]; then attempt=2; fi
    fi
  fi

  if [ "$attempt" -ge 2 ]; then
    echo "EBPF_ATTEMPT = AVAILABLE"
    echo "  both the BPF loader stack and at least one tracepoint program"
    echo "  loaded successfully under authorized sudo; no security policy"
    echo "  was changed."
  elif [ "$attempt" -eq 1 ]; then
    echo "EBPF_ATTEMPT = PARTIAL (trivial program loaded, tracepoint attach failed)"
  elif command -v bpftrace >/dev/null 2>&1 || command -v bpftool >/dev/null 2>&1; then
    echo "EBPF_ATTEMPT = FAILED_WITH_REASON (loader present but load failed; see rc above)"
  else
    echo "EBPF_ATTEMPT = UNAVAILABLE_WITH_REASON"
    echo "reason=no eBPF loader is installed (bpftrace and bpftool both absent)."
  fi
} > "$OUT" 2>&1
cat "$OUT"
