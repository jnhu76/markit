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

| Question | Probe | Result |
|---|---|---|
| S1 scheduler interference | `sched_switch` / `sched_migrate_task` under bpftrace, attached to the benchmark process | **whole-run counts for the benchmark comm, not resume-filtered**: 73 switches / **0 migrations** |
| S2 page faults | `page_fault_user` under bpftrace (whole process), plus `perf stat -e page-faults,minor-faults,major-faults` inside the update window | windowed scope: 13 056 page-faults = 13 056 minor, **0 major** across the 30 counted windows; whole-process scope: 276 385 `page_fault_user` events. Windowed faults are a small linear quantity (×17 for ×16 M: 7 → 119 per update) — allocation-fault cost is NOT super-linear |
| S3 address-space churn | `mmap/munmap/brk/mremap` tracepoints, whole process | 31 mmap / 19 munmap / **4 109 brk** / **0 mremap**, with large anonymous mmap/munmap pairs (len ≈ 1.04–2.10 MB) present |

Interpretation — **narrowed by the corrective below**. The supported
conclusion is:

> The eBPF/kernel lane found no evidence of CPU migration, of major-fault
> storms, or of a kernel-side regime transition large enough to explain
> the observed nonlinear scaling.

The three "environment" suspects are therefore *not evident as causes* of
the 1→16 MiB super-linearity within the scopes these probes actually had
— which is weaker than "ruled out".

**Corrective (documentation layer; raw counts unchanged).** The first
version of this section stated that the scheduler was "excluded", that
there were "0 major faults in every window", that there was "no mmap
churn", that the heap "grows by brk a few times per update", and that the
allocator was irrelevant. The probe scopes recorded in `raw/` do not
support those statements:

- the counts in the table are **whole-process aggregates over one
  ~20-update batch** for the benchmark comm, and are not resume-filtered;
- the `sched` trace was taken with `perf record -a` and is
  **system-wide** — `raw/s1-perf-sched-script.txt` shows the *profiler's
  own* process (`perf:43290`) being switched and migrated by
  `migration/0`, so those events are not the benchmark's;
- `4 109 brk` over 20 updates is ≈205 per update, and the raw trace shows
  the break moving both up and down; "a few times per update" is
  withdrawn;
- large anonymous `mmap`/`munmap` pairs *are* present, i.e. the allocator
  does create and destroy mappings for the large vectors; this lane did
  not quantify that as a cost, and "allocator irrelevant" is not claimed;
- the two fault lanes do not share a scope (whole-process 276 385 vs
  windowed 13 056 over 30 windows) and must not be compared numerically;
  "0 major faults" holds **for the windowed scope**;
- migration is 0 in both lanes — the one suspect this lane does exclude
  within its scope.

The conclusion is unchanged for the #50 result: the kernel lane does not
explain the nonlinearity, but it does not quantitatively exclude the
kernel suspects either. The matching wording in the #50 report §5 is
identical.

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
