# PROFILING-SELECTION-v1 — MARKIT-31 Stage-B post-hoc case selection

Status: **PROFILING_FOLLOWUP / NON_PRIMARY / POST_HOC_CASE_SELECTION**
(task §33-§34). Written BEFORE any perf/eBPF execution, after
PRIMARY-OBSERVATIONS-v1.md was frozen
(SHA256 5cfb5c993d5aa212a01cad19b415e9d5cba10ff49eff684a03aa9af0b07f5d66).

Selection rule (fixed before looking at any profile): for each slot,
the session-stable case whose paired H0-relative speedup is closest to
the MEDIAN of its stratum — never the extreme case. Counters below are
frozen primary attribution values (attribution-joined.csv).

Primary binary identity: a3ef4e63...d5bb (unchanged). Profiling replay
uses the SAME mechanism crates and the SAME runner orchestration
primitives (`build_initial_state`, `run_update_timed`,
`run_full_parse_timed`) at analysis commit 0bf678c; profiling binaries
carry `PROFILING_BINARY != PRIMARY_BINARY` only in the sense of a
separate diagnostic ELF — source identical.

Replay contract (task §36): frozen CaseId/payload, same mechanism
dispatch, fresh pre-state outside every timer, same complete() boundary,
CPU-pinned like the campaign, no primary raw output, no ObservationId,
no horse implementation change.

| Slot | CaseId | Horse(s) | Reason | Primary timing | Work-counter observation | Question |
| ---- | ------ | -------- | ------ | -------------- | ------------------------ | -------- |
| P1 | 9946b5d49a56529eebb9de39e11cf1b90fdce580569534ab54d782c00ba8b593 | H1 (+H0 baseline) | R1: H1 winner stratum (owasp, block-fragmented) | speedup 2.567, p50 21846ns | inspected 1266/9958B, 8/174 blocks, fallback 0 | Where does H1's residual go when it genuinely avoids work? |
| P2 | a3d4da7ba3d5b5dc18b530f3f465498db4aa464e7087e5dce79a0d4828fc1f3f | H1 (+H0) | R1 counterexample: H1 loser (rust-rfcs) | speedup 0.667, p50 71850ns | inspected 14530B > H0 13511B, fallback 1 | Is the loss pure fallback cost or block-scan overhead before fallback? |
| P3 | abf0d9be96b4f1ee1cc656f1d4760bb7b208201baef57c703ddaf7ddf84c6fb2 | H3 | R2: H3 prepare outlier (E5 median) | speedup 2.218, prepare p50 1575ns vs H2 82ns | reused 168/174 | What runs in H3.prepare_update? |
| P4 | fea305d72dbbeee48fdea65ea9893a4889602407d2c96d9baf476141ba785ac3 | H3 (+H2) | R3: E6 semantic rematerialization (§18) | H3 speedup 0.738, p50 60055ns | inspected 11154 ≈ H0 11628 despite reuse 127/215 | Is the residual definition-environment rebuild (traversal) or parse? |
| P5 | 1a0deadd0f109c2a22aacdb0046eabc2a7a4ee1666b254455b1a07e548e44369 | H2 (+H3) | R3: E6 definition invalidation | H2 speedup 0.766, H3 0.432 | H2 rebuilt 64/130 vs usual 6-27 | Same question, H2 anchor |
| P6 | 73039631b4e45389797f5ef0bb26cc57ac1f3da406ae556a70236a194049d66a | H2 | R4: CLEAN_STATE premium (largest file, 44914B) | p50 56844ns vs H0 ~45µs-class | n/a (clean build) | Where do the extra ~25% of native-state construction go? |
| P7 | 73039631b4e45389797f5ef0bb26cc57ac1f3da406ae556a70236a194049d66a | H4 | R4: same file, H4 premium | p50 58494ns | n/a | Same question, H4 anchor |
| P8 | d0a2203aa2451c26f9c1f910511bd35f908ddec5cd61bcfd575d67400cfec5f5 | H4 | R5: depth>=3 latency beyond convergence (k8s FAQ, depth 6) | speedup 0.834, p50 10940ns | restart 1018, conv 1173 of 1632B file | Is deep-container cost convergence scanning or container validity repair? |
| P9 | d5502cc52c2173802f4aa342217f7333ea54ca238086c96a253ff4568a0a2828 | H4 (+H0) | R6 counterexample/sanity: H4 big win (E5) | speedup 4.246, p50 4865ns | inspected 271 vs H0 5849 | Confirm the win is avoided parse work, not a timing artifact |

```text
slots = 9 case x horse comparisons (<= 12 budget, task §33)
counterexamples included: P2 (H1 loser), P8 (H4 loser), P9 (sanity win)
eBPF role: capability discovery only (unprivileged BFP disabled on this
host -> recorded UNAVAILABLE unless run as root; no root assumed)
```
