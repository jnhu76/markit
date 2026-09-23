# CAMPAIGN-2 CLOSURE — MARKIT-31 FULL EVIDENCE CAMPAIGN-2

Non-interpretive closure report. Collection integrity and identities only:
no horse ranking, no speedup, no winner, no crossover, no design claim.

---

## 1. Authority and identities

```text
authority merge SHA   3762b7a42e1c284a4c2c2e0ebac8496e70c63431
                      (master, PR #46 merge — the H0-H4 algorithm/counter
                       identity audit)
branch                research/31-full-evidence-campaign-2
worktree              ~/Source/markit-campaign-2

StudyId               cf151a01b1a347210320d6d2cf920c3de2310612cd8f4aa2f5aadb01d4b19856
CampaignSpecId        bd89908c51b0724073bdb090c7b54a380e08bfc2a4240df4b296b26e47800990
```

Sub-campaign spec ids:

```text
construction               26bdc4f63b0e320bb6e39f3a2aa20bfdd91220c4baaaf19f8e06e637940e3c8b
resident_update            754038ae9638b4459d7372a1dd2ab5b811846da6831981fedd190f696bb49b37
lifecycle                  bf56b263a09c44b5c78351c24858fe9d00e9bb16a5edb03d4716f49739ed5925
controlled_n               98e76d34c5608275f3750e6d3ed9bc30a94c8407c968ad43db0a28d5cb5120f6
controlled_b               d4c76429aad76b4db6e928707a86a935c7c61acf43eebb7f7a11f684821f4314
controlled_d_fence         9d7d09a5fc7b3cb2f95e582c915454f4a051b572a15d7ec35dfbc5aef21a0f65
controlled_f_reference     ecfa796fd6af43d525a519392a1ec22512f84675eb69070fdb45435b1b5c43a1
controlled_k_container     7d7c7019d506dff15818cc7b2dc2eb53187c37c6a838139c6dfc23bb39c02a32
profiling                  72bb7d034afebde5def009bc988125c74038a71272c5fb6f5d0e276774049377
```

RunIds (one per sub-campaign; each maps to exactly ONE executable):

```text
RunId                                                             rows       sub-campaign        executable
4c2a016ab180df21e974ee335b6ee64b13b5c6b66cd2a85c3fc13ff75e552265    806,400  lifecycle           c1c7756a
b1f1b2509eb106200a114e1d1952a9746eebe9e64017ce96cdb00535a6dbadaa    219,010  resident_update     c1c7756a
142720eda03d39da4400193df2810a92bc805e01e22fc86f4e5882fc19f58b7a     26,880  lifecycle (attr.)   1bd1c09e
88377126393279123bb450af51f2e7f56a9294319a148f892dca4a8f2839d2ba     13,310  construction        c1c7756a
e53edd02f80d21bd1852b1b289d849dc49fba64375d5ca736640521b2f536ffa      6,655  controlled_n        c1c7756a
953f30932878d4e8858912b461afefa82c8233b93d9b0020d65fc3fc6cdc5e84      5,445  controlled_b        c1c7756a
6d3572cc37eba8bdc1c35e9f245583982f81f4e4efbcb6cc31b0a3dffad23e21      4,840  controlled_f_ref    c1c7756a
9411446f0f5cd8ce5fd116591e7557fd5b674bcd9d09d1de65054ae2ad1d7a46      4,840  controlled_k_ctr    c1c7756a
d0d68391045c9f61af3693e3a4bdd748e8365e75f89fa56ca096f4d33c68b28c      4,235  controlled_d_fence  c1c7756a
```

`EACH_RUNID_MAPS_TO_EXACTLY_ONE_EXECUTABLE = PASS` — recomputed from the
frozen identity derivation, not read from a record.

### Executable note (recorded, not hidden)

```text
c1c7756a1d36316dff2fb278e9db7926ee20a39bceaa0e8248e2619af52f5822
    produced EVERY formal timing row, every work-counter row except the
    lifecycle attribution rows, and the whole pilot.
    build commit 4fd86e489b897c987074550b800c08e613e5185d (pre-freeze build)

1bd1c09e0bc271bb34394f8ad3b8174d5e2a6da81bc0cb5e013e6d2378fbf282
    produced ONLY the lifecycle attribution lane.
    build commit f6262716daddf8b5151de0b2b0e2879d3e838b2b (CAMPAIGN-2-FREEZE)
```

The change between the two is **purely additive campaign tooling**:
`git diff` over `campaign2/src/bin/mdbench-campaign2.rs` is
`160 insertions(+), 0 deletions(-)` — one new match arm and one new
function. No mechanism, runner, oracle, common or timed-executor file
differs between them (see §5). The two lanes are therefore never pooled
under one RunId; they are joined only by the stable identity fields
(CaseId, horse, session, trace + step).

---

## 2. Production horse freeze

```text
git diff --stat <authority> .. HEAD over
    mechanisms/  runner/src/  common/src/  oracle/src/
= EMPTY
```

No production H0-H4 algorithm was modified. The only non-campaign file
change is the workspace member list (`Cargo.toml`) plus the resulting
`Cargo.lock` addition — the campaign-2 crate itself.

Campaign-1 evidence:

```text
CampaignSpecId  ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2
RunId           905427daa02ec3a32a4c743d636ec46c3a7e26bda33806c13dcd0424ece2429d
overwritten / appended / rewritten / renamed / reused = NO
```

PR #45 interpretation was not used as scientific authority anywhere.

---

## 3. Launch-gate preflight (before the first formal row)

```text
machine-verify          CAMPAIGN2_MACHINE_VERIFY_PASS
                        machine_id=primary-e5-xeon2666v3-fedora44,
                        cpu=1, core=1, siblings=1,11
workload-verify         CAMPAIGN2_WORKLOAD_VERIFY_PASS
                        full_read=22 edit_write=362, 0 missing
acquire.py verify       VERIFY PASS: 20 sources, 3970 files,
                        66,391,919 bytes byte-verified, 0 missing
generators-verify       CAMPAIGN2_GENERATORS_VERIFY_PASS cells=43
                        (every pre AND post source passes the H0 clean
                         parse under the NORMALIZED-RESULT-v1 gate)
spec-verify             CAMPAIGN2_SPEC_VERIFY_PASS (9 sub-campaigns)
schedule-generate       CAMPAIGN2_SCHEDULE_PASS rows=1323
lifecycle-freeze        LIFECYCLE_FREEZE_PASS traces=14 steps=1792
runner smoke            reserved-rows smoke + finalize: identity 1/1/1,
                        duplicates 0  (logs/00-runner-smoke-validation.log)
```

No formal row was produced before `FULL_EVIDENCE_CAMPAIGN_2_FREEZE_PASS`.

---

## 4. Formal collection

Every lane was pinned to CPU 1, guarded by an executable SHA256 check
immediately before execution, and logged in full under
`results/campaign-2/logs/`.

```text
construction    session 0/1/2   4,400 rows each   (22 cases x 5 x 40)
resident-update session 0/1/2  72,400 rows each   (362 cases x 5 x 40)
lifecycle       session 0/1/2 268,800 rows each   (14 traces x 5 x 128 x 30)
controlled N    session 0/1/2   2,200 rows each   (11 cells x 5 x 40)
controlled B    session 0/1/2   1,800 rows each   ( 9 cells x 5 x 40)
controlled D    session 0/1/2   1,400 rows each   ( 7 cells x 5 x 40)
controlled F    session 0/1/2   1,600 rows each   ( 8 cells x 5 x 40)
controlled K    session 0/1/2   1,600 rows each   ( 8 cells x 5 x 40)
```

Work-counter lanes (untimed, separate lane):

```text
construction attribution       110 rows  (22 x 5)
resident-update attribution  1,810 rows  (362 x 5)
controlled attribution         215 rows  (43 x 5)
lifecycle attribution       26,880 rows  (14 x 5 x 128 x 3 sessions)
memory (descriptive)         66 + 192 rows, process-isolated
```

Row counts by lane, from the raw inventory (raw JSONL only):

| lane | files | rows | bytes |
|---|---:|---:|---:|
| construction | 4 | 13,310 | 24,728,911 |
| resident-update | 4 | 219,010 | 408,545,716 |
| lifecycle | 6 | 833,280 | 1,945,470,699 |
| controlled/N | 4 | 6,655 | 13,548,572 |
| controlled/B | 4 | 5,445 | 11,095,318 |
| controlled/D-fence | 4 | 4,235 | 8,376,612 |
| controlled/F-reference | 4 | 4,840 | 9,511,217 |
| controlled/K-container | 4 | 4,840 | 9,803,921 |
| **total** | **34** | **1,091,615** | **2,431,080,966** |

Including the non-JSONL artifacts (perf text reports, `.data`, memory
samples, headers), the raw tree is **274 files / 2,494,862,669 bytes**.

---

## 5. Correctness

```text
oracle   normalize(Hx result) == normalize(H0 clean full parse(post source))
         reference validated against NORMALIZED-RESULT-v1, computed strictly
         outside every timer.
lifecycle verified after EVERY edit step, against the frozen per-step
         expected checksum AND the structural reference.
```

```text
construction       4400 x 3   correctness_status = pass on every row
resident-update   72400 x 3   correctness_status = pass on every row
lifecycle        268800 x 3   correctness_status = pass on every row
controlled        25800       correctness_status = pass on every row
attribution      29,015 rows  no failed dispatch
CORRECTNESS FAILURES = 0
INVALID MARKERS      = 0
FAILED FORMAL LANES  = 0
```

`run-lifecycle` would have stopped a trace with a short chain and returned
an error; every lifecycle lane returned its exact expected row count, so
every one of the 14 x 5 x 3 x 30 = 189,000 chains ran all 128 steps.

---

## 6. Raw hash closure and copies

```text
RAW_HASH_CLOSURE   = PASS   (274 files re-hashed from disk)
IDENTITY_CLOSURE   = PASS   (one identity per raw file, 0 duplicate
                             observation ids anywhere)
```

Copies:

```text
COPY A  ~/Source/markit-campaign-2/research/benchmarks/markdown-ast-update/results/campaign-2
        primary; the tree every receipt and the inventory describe
COPY B  /dev/shm/campaign-2-copy-b
        INDEPENDENT FILESYSTEM (tmpfs), verified byte-identical
COPY C  /home/jnhu/markit-campaign-2-raw
        durable copy outside any git worktree, verified byte-identical
```

```text
verification: 274/274 files byte-identical on all three copies
              (822 file comparisons, 0 mismatches)
SECOND_COPY_VERIFIED = YES
```

Honest limitation: COPY B is on tmpfs and is therefore volatile; it
demonstrates byte-identity on a filesystem independent of the data volume
but is not a durable archive. COPY C is durable but shares the btrfs data
volume with COPY A (btrfs copy-on-write may share extents), so a media
failure of `/dev/sda3` would affect both. The two unmounted physical disks
(`/dev/sdb2`, `/dev/nvme0n1p3`) could not be mounted non-interactively
(polkit requires an authentication agent), so a true off-volume durable
copy was not created. This is recorded in `EVIDENCE-GAPS.md`.

---

## 7. Profiling lane

```text
profiling_binary_sha256  2b4ac6b877c88d6dd2f1408071be52ca008181630613831886423511157c34c6
                          (mdbench-replay; ONE binary for the whole lane —
                           every artifact predates the later rebuild)
slots                     12 (PROFILING-SELECTION-v1.md, POST_HOC/NON_PRIMARY)
perf stat                 10 repetitions x 5 horses per slot, horse order
                          rotated one position per repetition
region counters           perf_event_open, exclude_kernel, baseline sampled
                          while DISABLED; setup-only companion run per horse
perf record               4 slots (P01, P02, P07, P09), cycles:u @ 999 Hz,
                          call-graph dwarf, release binary
```

Artifacts: `profiling/perf-stat/`, `profiling/perf-record/`,
`profiling/PERF-CAPABILITY-AUDIT-v1.txt`, `profiling/PROFILING-SELECTION-v1.md`.
Large `.data` files carry hashes in the inventory but stay out of Git.

`perf` output never enters a primary table.

## 8. eBPF

```text
EBPF_COLLECTION = UNAVAILABLE_WITH_REASON
reason = kernel.unprivileged_bpf_disabled = 2 and no bpftrace/bpftool binary
No kernel setting was modified.
```

Not a blocker (task §39).

## 9. Memory lane

```text
MEMORY_COLLECTION = COMPLETE_DESCRIPTIVE
method  /proc/self/status (VmRSS, VmHWM, VmSize) + /proc/self/statm, in the
        owning process, strictly between operations, process-isolated per mode
label   DESCRIPTIVE_PROCESS_MEMORY
rows    66 (construction) + 192 (resident_update)
```

## 10. Derived mechanical summaries

Deterministic tables only, all reproducible from raw by
`results/campaign-2/tools/summarize.py`:

```text
construction-case-horse.csv              110 rows
resident-update-case-horse.csv         1,810 rows
controlled-{N,B,D,F,K}-cell-horse.csv     43 cells x 5 horses
lifecycle-step-p50.csv                26,880 rows
lifecycle-cumulative-derived-p50.csv   26,880 rows (labelled DERIVED)
lifecycle-checkpoint-derived-p50.csv      560 rows (labelled DERIVED)
attribution-*.csv                      per-lane counter joins
```

Sessions are never pooled into n = 90. The stability diagnostic
(`max(session p50)/min(session p50) > 1.5`) travels with every cell and
was used only as a flag.

---

## 11. What this campaign does NOT conclude

```text
no ranking, winner, speedup conclusion, crossover claim or design
recommendation
no Weakness Map
no Markit algorithm selection
```

Raw observations carry facts only. Final interpretation is deferred to the
fresh-context review: `GPT6-FULL-EVIDENCE-HANDOFF.md`.

---

## 12. Verdict

```text
FULL_EVIDENCE_CAMPAIGN_2_PASS

CONSTRUCTION_COLLECTION    = COMPLETE
RESIDENT_UPDATE_COLLECTION = COMPLETE
LIFECYCLE_COLLECTION       = COMPLETE

CONTROLLED_N               = COMPLETE
CONTROLLED_B               = COMPLETE
CONTROLLED_D_FENCE         = COMPLETE
CONTROLLED_F_REFERENCE     = COMPLETE
CONTROLLED_K_CONTAINER     = COMPLETE

MATCHED_PERF_COLLECTION    = COMPLETE

EBPF_COLLECTION            = UNAVAILABLE_WITH_REASON
MEMORY_COLLECTION          = COMPLETE_DESCRIPTIVE

RAW_HASH_CLOSURE           = PASS
SECOND_COPY_VERIFIED       = YES

FINAL_RESEARCH_ANALYSIS    = NOT_STARTED
READY_FOR_GPT6             = YES
```
