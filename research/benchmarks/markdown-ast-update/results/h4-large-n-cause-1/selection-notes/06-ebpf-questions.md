# Selection note — eBPF lane questions

**Trigger observation.** The per-cell PMU series leaves three runtime
questions that hardware counters alone cannot settle:

1. the `page-faults` software event counts 0.9-119.5 minor faults per update
   window and **zero** major faults — is the 16 MiB point crossing into a
   different kernel memory-management behaviour (mmap/brk/mremap), or is it
   the same behaviour scaled?
2. the window is bracketed by a synchronous FIFO round trip per update, so
   ~1-2 `context-switches` per window are expected — is any of the large-N
   cost *scheduler* interference (involuntary switches, migrations)?
3. user-only `cycles:u` is 97-98 % of kernel-inclusive `cycles`, but the
   residual kernel share is largest at the *smallest* N — is that a real
   kernel component of U or the measurement protocol's own syscall cost?

**Questions chosen.** Exactly those three, expressed as: scheduler
interference, page faults, and mmap/munmap/brk/mremap activity, with a
matched untraced control run in the same batch for the perturbation ratio
(Issue #50 §16). No unbounded per-allocation or per-function trace.

**What would support each.** Zero migrations and a context-switch count that
matches the protocol's own round trips refutes scheduler interference. A
major-fault count of zero with minor faults scaling with allocated bytes
refutes a qualitative memory-management transition. A kernel cycle share
that shrinks as the update grows refutes a kernel-side cause of the
large-N cost.
