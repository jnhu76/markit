# H4-LARGE-N-CAUSE-1 — eBPF lane summary (Issue #50 §14–§16)

```text
EBPF = COMPLETE
EBPF_ATTEMPT = AVAILABLE
EBPF_PERTURBED = NO   (with_BPF/without_BPF median native ratio = 1.0040)
Security policy changes made for this lane: NONE
  (unprivileged_bpf_disabled stays 2, perf_event_paranoid stays 2;
   every privileged operation ran under authorized sudo.)
Producing executable: results/h4-large-n-cause-1/bin/mdbench-h4diag
  sha256 4d23df55856a1e71115abdc45c0f5d7993182520f0b53df2a0cab6cdb8c0b4e6
```

## Capability audit (`capability.txt`, `scripts/01`+`02`)

- kernel `7.2.5-200.fc44.x86_64`, BTF **available**,
  tracefs/debugfs mounted.
- `bpftrace v0.24.2`, `bpftool v7.6.0` (installed from Fedora repos
  AFTER the timed lanes and the PMU lane had finished; the install is
  a userspace package operation and shared no window with any
  measurement).
- `unprivileged_bpf_disabled=2`: unprivileged BPF is blocked, so every
  load ran under **authorized sudo**. The task forbids changing this
  sysctl for the experiment; it was not changed.
- Attempt record: trivial `BEGIN` program loaded and a
  `sched:sched_switch` tracepoint attached successfully →
  `EBPF_ATTEMPT = AVAILABLE`.

## Narrow probes (causal questions, `scripts/03`, `scripts/04`)

| Question | Probe | Result (whole batch) |
|---|---|---|
| S1 scheduler interference | `sched_switch` / `sched_migrate_task` | 73 switches / **0 migrations** of the benchmark thread → scheduler is not a cause |
| S2 page faults | `page_fault_user`, plus `perf stat -e page-faults` inside the update window | all **minor**; 0 major faults in every window; windowed faults are a small linear quantity (x16 for x16 M) — allocation-fault cost is NOT super-linear |
| S3 address-space churn | `mmap/munmap/brk/mremap` tracepoints | 31 mmap / 19 munmap / 4109 brk / **0 mremap** over 20 updates → no mmap churn; the heap grows by brk a few times per update; the allocator is not creating address-space pressure |

Interpretation: the three "environment" suspects (scheduler
interference, fault storms, allocator address-space churn) are all
ruled out as causes of the 1→16 MiB super-linearity. This agrees with
the PMU lane: the missing time is LLC/TLB misses walking the O(M)
representation, not kernel-side effects.

## Perturbation control (`scripts/04`)

Matched batch, same binary, same cells (1 MiB + 16 MiB x 10 rounds):

```text
with_BPF    median native = 11,083,954 ns
without_BPF median native = 11,039,810 ns
ratio = 1.0040  →  EBPF_PERTURBED = NO
```

The windowed software-event run (`scripts/03`) also reported
traced/untraced = 0.988 — both within the lane's A/A noise.

## Raw artifacts

- `raw/attempt-ebpf-load.txt` — capability + load attempt record
- `raw/s1s2-software-events.txt`, `raw/s1s2-observations.jsonl`,
  `raw/untraced-control.jsonl` — windowed software events + control
- `raw/s1-perf-sched.data|report|script` — sched tracepoint capture
- `raw/s3-syscall-trace.txt` — mmap/munmap/brk/mremap trace
- `raw/bpftrace-counts.txt` — BPF-attached probe counts
- `raw/with-bpf.jsonl`, `raw/without-bpf.jsonl` — perturbation pair
