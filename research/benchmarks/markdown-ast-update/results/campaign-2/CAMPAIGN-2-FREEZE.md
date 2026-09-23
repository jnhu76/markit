# CAMPAIGN-2 FREEZE — MARKIT-31 FULL EVIDENCE CAMPAIGN-2

Status: **FROZEN BEFORE THE FIRST FORMAL ROW.**

```text
task         MARKIT-31 — FULL EVIDENCE CAMPAIGN-2
authority    3762b7a42e1c284a4c2c2e0ebac8496e70c63431   (master, PR #46 merge)
branch       research/31-full-evidence-campaign-2
worktree     ~/Source/markit-campaign-2
```

This document is the campaign's execution contract. It was written and
committed **before** the first formal measurement row, and no field in it
is derived from a measured value. The pilot (`results/campaign-2/pilot/`)
produced no headline evidence and is explicitly `NON_RESEARCH`.

---

## 1. Campaign identity

```text
StudyId              cf151a01b1a347210320d6d2cf920c3de2310612cd8f4aa2f5aadb01d4b19856
CampaignSpecId       bd89908c51b0724073bdb090c7b54a380e08bfc2a4240df4b296b26e47800990
CampaignId           MARKIT-31-FULL-EVIDENCE-CAMPAIGN-2
CampaignSeed         <derived: sha256-first64-bigendian-v1>
Authority SHA        3762b7a42e1c284a4c2c2e0ebac8496e70c63431
Envelope schema      campaign2-observation-v1
```

`StudyId` is structural: `hex(SHA256("MARKIT-31-FULL-EVIDENCE-STUDY" ||
authority_sha || the nine sorted sub-campaign tags))`. It cannot move
without an authority change or a sub-campaign-set change.

`CampaignSpecId` is `hex(SHA256(canonical JSON of the frozen campaign
spec))`. The spec binds, and binds only:

```text
authority SHA                    sampling (sessions / warmup / measured /
horse set + mechanism ids               lifecycle reps / controlled reps)
real workload identity           CPU binding
  (#35 manifests + digests +     statistics policy
   resolved counts)              correctness oracle
controlled generator identity    failure policy
timer boundaries                 profiling policy
interpretation rules R1-R4       envelope schema + digest
metric qualification             order algorithms + seed domains
```

`SubCampaignSpecId` is derived per sub-campaign as
`hex(SHA256(CampaignSpecId || tag || canonical JSON of the sub-campaign
binding))`; the binding records the surface, evidence classes, lanes, case
cardinality, sampling and raw path.

`RunId` is the execution-time binding:

```text
hex(SHA256(StudyId || CampaignSpecId || SubCampaignSpecId
           || runner commit || machine manifest digest
           || rustc || target || build profile id || Cargo.lock digest
           || executable SHA256))
```

No `RunId` may ever mix two executables. A rebuild changes the `RunId`.

**Campaign-1 identities are historical and untouched:**

```text
CampaignSpecId  ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2
RunId           905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d
```

Neither is reused, appended to, rewritten or renamed.

---

## 2. Production horses — frozen, unchanged

```text
H0  FULL_REBUILD                 mechanisms/full-rebuild
H1  BLOCK_LOCAL_REPARSE          mechanisms/block-local
H2  FRAGMENT_REUSE               mechanisms/fragment-reuse
H3  OLD_TREE_SUBTREE_REUSE       mechanisms/old-tree-subtree-reuse
H4  RESTART_CONVERGENCE          mechanisms/restart-convergence
```

`git diff` against the authority SHA over `mechanisms/`, `runner/src/`,
`common/src/`, `oracle/src/` is EMPTY. Campaign-2 adds one workspace
member (`campaign2/`) and nothing else; Cargo.toml/Cargo.lock changes are
the workspace-member addition only.

---

## 3. Machine identity

Captured at freeze time into
`results/campaign-2/manifests/campaign-2-machine-v1.toml`.

```text
machine_id               primary-e5-xeon2666v3-fedora44
architecture             x86_64
distribution             Fedora Linux 44 (KDE Plasma Desktop Edition)
kernel                   7.2.5-200.fc44.x86_64
cpu                      Intel(R) Xeon(R) CPU E5-2666 v3 @ 2.90GHz
microcode                0x49
physical cores / logical CPUs   10 / 20   (SMT enabled)
numa                     single node
selected CPU             1  (physical core 1, SMT siblings 1,11, NUMA node 0)
governor                 schedutil        (inspected, never modified)
turbo                    intel_pstate no_turbo=0 (ENABLED; never modified)
rustc                    rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo                    cargo 1.97.1 (c980f4866 2026-06-30)
LLVM                     22.1.6
target                   x86_64-unknown-linux-gnu
RUSTFLAGS                (empty)
release profile          release-primary-v1  (opt-level 3, lto thin,
                         codegen-units 1, incremental false, panic unwind)
allocator                rust-system-default
Cargo.lock SHA256        recorded in the machine manifest
MemTotal                 RECORDED DIAGNOSTIC ONLY (per the #43 corrective);
                         never a gate
```

Host binding is fail-closed on every HARD field; `MemTotal` and per-session
frequency/temperature/load are observations only (task §10, §32).

---

## 4. Real workload identity (task §8)

Reused from the frozen #35 workload. **No case was re-selected from a
Campaign-1 result.**

```text
FULL_READ qualified files        22      (G0_STRICT_FULL_READ)
FULL_READ source bytes           118 756
EDIT_WRITE qualified cases       362     (G0_PRIMARY)
EDIT_WRITE pre-source bytes      2 163 946
trace records                    329     (all break/restore pairs,
                                         trace_form = single_reset)
projects / edit families         recorded in the spec; not re-selected
source pins                      unchanged; `tools/acquire.py verify --full`
                                 reports 20 sources, 3970 files,
                                 66 391 919 bytes byte-verified, 0 missing
```

Manifest digests (full-read / edit-write / trace / freeze-receipt) are
bound into `CampaignSpecId`.

---

## 5. Surfaces and timer boundaries (task §13-§17)

```text
A CONSTRUCTION
    source bytes resident
      -> start timer
      -> clean parse + native state construction + seal (complete)
      -> state usable
      -> stop timer
    EXCLUDED: filesystem IO, oracle validation, normalize, checksum,
              report serialization.
    Primary metric: T_construct.  This is NOT "parser-only speed".

B RESIDENT_UPDATE  (SINGLE_RESET)
    fresh valid pre-edit state built OUTSIDE every timer
      -> start timer
      -> prepare_update; update; complete/seal; black_box
      -> stop timer
      -> oracle verification OUTSIDE the timer -> state discarded
    T_total = T_prepare + T_native (arithmetic, never a third interval).
    Construction cost NEVER enters T_total.

C LIFECYCLE
    state built ONCE per (trace, horse, rep), untimed
      -> edit1 -> state1 -> edit2 -> state2 -> ... -> edit128
    NO reconstruction between edits. Oracle after EVERY step.

D CONTROLLED
    identical to Surface B, on generated cells.
```

All four dispatch through the frozen `markit_mdbench_runner` phase
functions. The only new timed function is the chained lifecycle step
(`campaign2/src/exec.rs::run_update_chain_step`), which uses the identical
boundary and is proven equivalent to `run_update_timed` on execution
status, correctness status and result checksum by
`campaign2/tests/tooling_integrity.rs::chain_step_matches_the_frozen_runner_facts`.

---

## 6. Lifecycle traces (task §15-§16)

Frozen into `results/campaign-2/manifests/lifecycle-traces-v1.jsonl`
(digest recorded in §12) **before any timing**. 14 traces × 128 steps.

Real family — one frozen #35 BREAK/RESTORE pair per G0 break transition
that has an exact `base -> broken -> base` pair, replayed as
`repeat(break, restore)`; every step is a real frozen payload, and every
replayed post-source is cross-checked against the #35 manifest digest:

```text
G0-ATX-TO-PARAGRAPH      G0-BQ-NEST-LINE         G0-CODESPAN-DELIM-BREAK
G0-EMPH-DELIM-BREAK      G0-FENCE-CLOSER-REMOVE  G0-LIST-ITEM-INDENT
G0-REFDEF-REMOVE
```

Selection rule (preregistered, never re-selected from results): at most 1
pair per `(edit_family, break_transition)`, the lexicographically first
`trace_id`, base source ≤ 131 072 B. Break/restore phase is derived from
the frozen trace identity, never from payload-hash order.

Controlled family — `L1 repeated local text`, `L2 moving local edit`,
`L3 paragraph split/restore`, `L4 container mutation`,
`L5 fence open/restore`, `L6 reference definition change/restore`,
`L7 mixed deterministic editor-style sequence`, each on a 65 536-byte
generated base document.

K checkpoints: 1, 2, 4, 8, 16, 32, 64, 128. Every edit is recorded, not
only checkpoints. Expected normalized checksum is frozen per step.

---

## 7. Controlled variables (task §20-§25)

Exactly five CORE variables; no additional dimension was added.

```text
C-N  document size            N = 512 B .. 16 MiB              (11 points)
     fixed: affected block 128 B, LOCAL_TEXT, 8 edit bytes, target near
            middle, depth 0, no references, no fences.
C-B  affected block size      B = 128 B .. 64 KiB              (9 points)
     fixed: N = 128 KiB, LOCAL_TEXT, 8 edit bytes, target ~0.5,
            depth 0, no references, no fences.
C-D  fence propagation span   D = 64 B .. 64 KiB               (7 points)
     fixed: N = 128 KiB, one fence transition (closer removal, 3 bytes),
            identical edit offset for every D, same non-target structure.
     D = pre-edit byte distance from the edited closer to the
     reconvergence closer.
C-F  reference fanout         F = 0,1,2,4,8,16,32,64           (8 points)
     fixed: N = 128 KiB, definition offset, edit bytes (32-byte
            definition removal), 64 fixed use-site slots, fixed
            use-distance distribution, fixed definition environment.
C-K  container depth          K = 0,1,2,3,4,6,8,12             (8 points)
     fixed: N = 128 KiB, sibling width 62 B, sibling count 256,
            block_quote containers, LOCAL_TEXT 8-byte insertion at line
            index 0.
```

43 controlled cells total. Every generated document is BENCH-GRAMMAR-v1
valid: both the pre- and post-edit source of every cell pass the H0 clean
parse under the NORMALIZED-RESULT-v1 conformance gate.

### Recorded N-invariance entanglements (not hidden, not blockers)

With `N` fixed, one region must absorb each axis value. Recorded so any
reader can see the confound:

```text
C-N  nothing is absorbed — N itself is the variable.
C-B  absorbed by the 128-byte padding block count (N - B).
C-D  absorbed by the SUFFIX length, which sits AFTER the reconvergence
     closer (interior = D - 6 B, suffix = N - prefix - fence - D).
C-F  nothing is absorbed — the 64 use-site slots are a fixed-size
     region; F selects how many hold a reference use.
C-K  absorbed by the trailing padding length (suffix = N - 32256(K+4)).
```

Per-cell geometry is written to
`results/campaign-2/manifests/controlled-cells-v1.txt` at freeze time.

---

## 8. Sampling, statistics, ordering (task §18-§19, §45-§46)

```text
Sessions                       3
Construction / ResidentUpdate  10 warmup + 30 measured per (case, horse)
Controlled                     10 warmup + 30 measured per (cell, horse)
Lifecycle repetitions          30 per (trace, horse); EVERY edit recorded
Case order                     frozen Campaign-1 shuffle
                               (sha256-first64-bigendian session seeds)
Horse order                    frozen Campaign-1 policy
                               seeded-base-permutation-rotate-v1
                               (rotated per case and per session — never
                                "all H0, then all H1, ...")
Quantiles                      nearest-rank: p50 = rank 15 of 30,
                               p95 = rank 29 of 30; sessions are NEVER
                               pooled into n = 90
Case estimate                  median of the 3 session p50s
H0-relative                    per-session p50(H0)/p50(Hx), geometric mean
Lifecycle                      per-step p50 + a cumulative p50-derived
                               trajectory labelled DERIVED
Stability flag                 max(session p50)/min(session p50) > 1.5
                               flagged only, never deleted/downweighted/
                               rerun
```

The lifecycle and controlled repetition counts above were chosen from the
pilot's cost measurements and are frozen here and in the spec before the
first formal row of their sub-campaign.

---

## 9. Correctness oracle (task §12)

```text
normalize(Hx result) == normalize(H0 clean full parse(post-edit source))
```

computed strictly OUTSIDE every timer, with the reference document
validated against NORMALIZED-RESULT-v1 before use. Lifecycle verifies
after EVERY edit step, against the frozen per-step expected checksum as
well as the structural reference. Any failure stops the affected formal
sub-campaign, preserves the failing evidence, and is not retried until
classified.

---

## 10. Profiling policy (task §31-§41)

```text
Primary timing is clean: no perf record, bpftrace, strace, heap profiler,
allocator interposer or debug build touches any primary timing run.

Region isolation (task §38): `mdbench-replay` runs ONLY the measured
region in a loop with the PMU counters DISABLED during setup, so no
whole-process-minus-setup subtraction is used. A `--setup-only` companion
run exists purely to validate any whole-process number.

perf stat        core events, repeated process-level runs (10 repetitions,
                 horse order rotated). PROFILING EVIDENCE, not primary.
perf record      release binary, debug symbols allowed, only for the
                 selected unexplained gaps (PROFILING-SELECTION-v1.md,
                 POST_HOC / NON_PRIMARY).
Matched rule     same case, source, edit, replay structure, profiling
                 binary and CPU across whatever horses are compared.
eBPF             UNAVAILABLE — see §11.
allocator        profile-only, labelled PROFILE_ONLY_ALLOCATOR_EVIDENCE;
                 never promoted to a primary metric.
```

---

## 11. Capability audit (task §33, §39)

`results/campaign-2/profiling/PERF-CAPABILITY-AUDIT-v1.txt`

```text
perf                     7.2.5-200.fc44.x86_64
perf_event_paranoid      2
CAP_PERFMON (CapEff)     0000000000000000 (no capability)
available (user scope)   cycles, instructions, branches, branch-misses,
                         cache-references, cache-misses, task-clock,
                         page-faults, context-switches, cpu-migrations
perf record              AVAILABLE (user-space samples only)
unprivileged_bpf_disabled  2
bpftrace / bpftool       NOT INSTALLED
ulimit -l                8192 KiB
EBPF                     EBPF_UNAVAILABLE
                         reason = unprivileged_bpf_disabled=2 and no
                                  bpftrace/bpftool binary
NO kernel setting was modified. eBPF availability is NOT a blocker
(task §39).
```

---

## 12. Freeze checklist

```text
authority SHA                       3762b7a42e1c284a4c2c2e0ebac8496e70c63431
production H0-H4 changed            NO
Campaign-1 raw changed              NO
PR #45 artifacts used as authority  NO

StudyId                             cf151a01b1a347210320d6d2cf920c3de2310612cd8f4aa2f5aadb01d4b19856
CampaignSpecId                      bd89908c51b0724073bdb090c7b54a380e08bfc2a4240df4b296b26e47800990
  construction                      26bdc4f63b0e320bb6e39f3a2aa20bfdd91220c4baaaf19f8e06e637940e3c8b
  resident_update                   754038ae9638b4459d7372a1dd2ab5b811846da6831981fedd190f696bb49b37
  lifecycle                         bf56b263a09c44b5c78351c24858fe9d00e9bb16a5edb03d4716f49739ed5925
  controlled_n                      98e76d34c5608275f3750e6d3ed9bc30a94c8407c968ad43db0a28d5cb5120f6
  controlled_b                      d4c76429aad76b4db6e928707a86a935c7c61acf43eebb7f7a11f684821f4314
  controlled_d_fence                9d7d09a5fc7b3cb2f95e582c915454f4a051b572a15d7ec35dfbc5aef21a0f65
  controlled_f_reference            ecfa796fd6af43d525a519392a1ec22512f84675eb69070fdb45435b1b5c43a1
  controlled_k_container            7d7c7019d506dff15818cc7b2dc2eb53187c37c6a838139c6dfc23bb39c02a32
  profiling                         72bb7d034afebde5def009bc988125c74038a71272c5fb6f5d0e276774049377

machine manifest SHA256             0fd3665ed1dbbe9e38ef4d877309ae2d8eb94f8eefdde173044ed4722e88ef93
lifecycle trace manifest SHA256     c7fcb9e3e9c2b7e38b3504323973248ba96ccea3f180d6dd03652696cde9585e
controlled cell table SHA256        fdfc2689e23b6f7556283f6b360df46ea6cde14ecd2725200d1d997f8fa108c6
executable SHA256                   <recorded per RunId at collection time>
```

Every executable formal run receives its own `RunId` bound to the
executable SHA256 that produced it.

---

## 13. Failure policy (task §47-§48)

```text
lane failure      preserve partial raw, mark the lane invalid, stop the
                  affected sub-campaign
code change       new executable SHA, new RunId, restart the affected
                  sub-campaign
forbidden         delete, impute, retry invisibly
adaptation        no performance-driven adaptation once formal rows start
                  (no dropping losing horses, no extra repetitions for
                  surprising cases, no removing noisy cases, no changing
                  scales/files/CPU/compiler)
new ideas         become separately labelled follow-ups
```

---

## 14. Evidence classes (task §54)

```text
PRIMARY_TIMING          construction + resident_update timing lanes
PRIMARY_WORK            construction + resident_update attribution lanes
PRIMARY_LIFECYCLE       lifecycle rows
CONTROLLED_TIMING       controlled timing lanes
CONTROLLED_WORK         controlled attribution lanes
PROFILE_PERF            perf stat / perf record / replay PMU counters
PROFILE_EBPF            (none — UNAVAILABLE)
PROFILE_ALLOCATOR       profile-only allocator evidence
DESCRIPTIVE_MEMORY      process RSS samples
ENVIRONMENT_TELEMETRY   per-session frequency/temperature/load
DERIVED_MECHANICAL      deterministic summary tables
```

Never silently mixed.

---

## 15. Raw layout (task §49)

```text
results/campaign-2/
  manifests/            spec / identity / machine / schedule / traces / cells
  construction/
  resident-update/
  lifecycle/
  controlled/{N,B,D-fence,F-reference,K-container}/
  profiling/{perf-stat,perf-record,ebpf,allocator}/
  memory/
  receipts/
  logs/
  pilot/                NON_RESEARCH pilot (never headline evidence)
```

Every raw file gets a receipt carrying StudyId, CampaignSpecId,
SubCampaignSpecId, RunId, authority SHA, executable SHA, machine id, lane,
row count, byte size, SHA256, complete marker and exit status (task §50).

---

```text
FULL_EVIDENCE_CAMPAIGN_2_FREEZE_PASS
```
