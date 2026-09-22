# R7 machine-binding corrective — 1

Task: **MARKIT-31-MACHINE-BINDING-CORRECTIVE-1**.
Base authority: PR #42 merge `2a58f95ee34bc65da5b5488e32c081c81e4a13ac`
(#22 MARKIT-MARKDOWN-BENCHMARK-1 campaign freeze).
Amends: `protocol/R7-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-v1.md` §12/§13
wording and the `campaign` crate's machine-matching implementation only.

## 1. Observed launch failure

Phase A preflight on `primary-e5-xeon2666v3-fedora44`:

```text
CAMPAIGN_PREFLIGHT_BLOCKED

machine field total_ram_bytes:
frozen  = 67252445184
current = 67252449280
delta   = +4096 bytes
```

No formal campaign data existed or exists (timing rows 0, attribution
rows 0, no RunId, `results/raw` empty). Every other launch-gate check
passed.

## 2. Classification

```text
CAMPAIGN_IMPLEMENTATION_BUG
```

Not `HOST_CONFIGURATION`. `total_ram_bytes` is captured from
`/proc/meminfo:MemTotal`, which Linux defines as usable RAM —
installed capacity minus firmware/kernel reservations — not immutable
installed physical capacity. MemTotal legitimately moves between
boots (a 4 KiB reservation shift is normal kernel behavior). A
byte-exact gate over it was an implementation error in the matching
semantics, not a property of the host.

## 3. Change made (matching semantics only)

- `campaign/src/machine.rs`: machine matching is refactored into
  `observe_host_for_binding` (capture, fail-closed on unreadable
  facts) + `compare_host_binding` (pure comparison) +
  `memory_binding_diagnostic`. The `total_ram_bytes` blocker is
  REMOVED; there is no tolerance window of any size — the field is
  simply not compared, because MemTotal is not machine identity.
- `campaign/src/preflight.rs`: the Enforce path now reports a
  `machine_memory` diagnostic (frozen/current MemTotal, delta,
  `identity_role: diagnostic_only`) — visible, auditable,
  non-blocking, emitted even when other blockers exist.
- No workload, horse, timer, counter, schedule, sampling,
  aggregation, or metric-qualification semantics were touched.

## 4. What is preserved (fail-closed, byte-exact)

Every other machine gate still blocks on mismatch: architecture,
kernel, OS distribution, CPU vendor/model/microcode, physical cores,
logical CPUs, SMT, NUMA node count + CPU map, selected CPU + core +
SMT siblings + NUMA node, governor, turbo/boost policy, rustc, cargo,
LLVM, target triple, allocator policy, Cargo.lock digest, RUSTFLAGS,
release profile, affinity applicability. Capture failures
(unreadable `/sys`//proc facts) remain blockers. No fuzzy matching was
introduced anywhere.

## 5. Frozen values untouched

`results/manifests/primary-machine-v1.toml` is NOT modified:
`total_ram_bytes = 67252445184` remains the value observed when the
manifest was captured. It was never measured from SMBIOS/DMTF and is
not re-described as installed physical capacity — it is the recorded
MemTotal observation.

## 6. Campaign identity

```text
CampaignSpecId (unchanged) =
ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2
```

The scientific campaign specification is unchanged: same machine
target, workload, mechanisms, schedule, sampling, measurement
semantics, and statistics; only the preflight classification of Linux
MemTotal changed. Concretely, the spec binding recomputes from the
manifests, workload freeze receipt, envelope schema, and frozen
constants — none of which this corrective modifies — and the campaign
receipt's `BOUND_ARTIFACTS` list contains no Rust source file, so no
receipt drift occurs and no receipt was regenerated.

The corrective commit does change the runner commit and executable
SHA, so a future formal `RunId` will differ from a hypothetical
pre-corrective one. That is expected: `RunId` binds the executing
binary, and no formal run has started.

## 7. Tests

- `machine::tests::mem_total_drift_alone_never_blocks` — Test A: a
  host mirroring every hard field of the frozen manifest with
  MemTotal +4096 (and, separately, −1 GiB — no hidden tolerance)
  produces zero blockers.
- `machine::tests::hard_identity_mismatches_still_block` — Test B:
  cpu_model / kernel / selected_cpu / governor / microcode / rustc /
  target_triple / Cargo.lock digest / RUSTFLAGS / affinity failures
  still block; a failed topology recomputation stays fail-closed.
- `machine::tests::mem_total_remains_captured_and_reported` — Test C:
  the frozen manifest still carries `total_ram_bytes = 67252445184`
  unchanged, live capture still reads MemTotal, and the diagnostic
  reports frozen/current/delta/`identity_role: diagnostic_only`.

## 8. Verification evidence

Recorded from the corrective branch on the primary host
(`primary-e5-xeon2666v3-fedora44`, kernel `7.2.5-200.fc44.x86_64`):

```text
cargo fmt --all -- --check                    PASS
cargo clippy --workspace --all-targets
    -- -D warnings                            PASS
cargo test --workspace                        PASS
mdbench-campaign manifest-verify              CAMPAIGN_MANIFEST_OK
mdbench-campaign receipt-verify               CAMPAIGN_RECEIPT_OK
    spec_id=ad45c7cbd2565d7f78c54a75fdaa27defe22788febb21d583e09805c6d6c66f2
mdbench-campaign schedule-determinism         SCHEDULE_DETERMINISM_PASS
mdbench-campaign preflight (release build)    CAMPAIGN_PREFLIGHT_PASS
    host_binding=enforced blockers=[]
    machine_memory.delta_bytes=4096 (diagnostic_only)
```

(Exact outputs are in the corrective PR; this record states the
verified facts.)

## 9. Scope discipline

No primary timing, no attribution lane, and no receipt regeneration
were performed by this corrective. Dynamic-safety evidence is
unaffected: the change touches only machine matching / diagnostics,
not unsafe/FFI/data-path code beyond moving the existing affinity
probe call.
