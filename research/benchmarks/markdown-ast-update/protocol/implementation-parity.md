# Implementation Parity — R1 Materialization

Status: **R1 MATERIALIZED**
Authority: `protocol/R0-METHODOLOGY.md` §4 (IMPLEMENTATION_PARITY_CONTRACT,
FROZEN). This file records how R1 materialized the policy; it does not
restate or amend R0.

## One workspace / one compiler regime

```text
workspace:        research/benchmarks/markdown-ast-update/ (independent; no root workspace)
toolchain:        1.97.1 pinned via rust-toolchain.toml (see manifest/environment.toml)
dependency graph: frozen Cargo.lock (digest recorded in every result row)
```

## Primary build policy (frozen)

```text
profile_id:       release-primary-v1
opt-level:        3
lto:              "thin"
codegen-units:    1
incremental:      false
panic:            "unwind"
target-cpu:       default (no RUSTFLAGS)
RUSTFLAGS:        empty
allocator:        Rust/system default (no custom allocator)
overflow-checks:  release default (off)
```

The policy lives in the workspace `Cargo.toml` `[profile.release]` and in
`manifest/environment.toml`. Debug builds are labeled `debug-non-research`
in result rows and can never be mistaken for research measurements.

## Second frozen profile (R0 §4.6 — FROZEN 2026-09-29)

The R0 optimization-sensitivity check requires a second frozen profile
for key representative cases. It is frozen as:

```text
profile_id:       release-sensitivity-lto-off-v1
inherits:         release (workspace Cargo.toml [profile.release-sensitivity-lto-off-v1])
opt-level:        3          (unchanged)
lto:              false      (THE single intentional difference: "thin" -> false)
codegen-units:    1          (unchanged)
incremental:      false      (unchanged)
panic:            "unwind"   (unchanged)
target-cpu:       default    (unchanged; no RUSTFLAGS)
allocator:        Rust/system default (unchanged)
toolchain:        rustc/cargo 1.97.1 (identical to primary; RUSTUP_HOME=/home/jnhu/rustup-1971)
```

Full binding: `manifest/rq8-sensitivity-profile-v1.toml`. The profile
exists ONLY for the #33 RQ8 sensitivity replication of the frozen K1–K6
shortlist (host-sealed R8 FINAL SYNTHESIS v1 record; sha256
`03cfeae5…` pinned in `results/manifests/sensitivity/rq8-sensitivity-manifest-v1.toml`;
authority: issue #33). It changes nothing about the mechanisms, the
runner's timing semantics, the workload, or the measurement boundary.
The primary `[profile.release]` above is NOT altered; primary evidence
keeps its own frozen identity. `run-session`/`run-attribution` refuse
to execute under the second profile, and the sensitivity session path
refuses to execute under any profile except this one (fail-closed in
both directions).

## Shared substrate (R0 §4.2 materialized)

Source/Edit, work counters, runner/timer, result schema, case identity,
shuffle, lane records, and the correctness hook are implemented once in
`common`/`runner`/`instrumentation`/`oracle` and used identically by every
mechanism. Mechanisms receive the same `&Source`, the same `CanonicalEdit`,
and the same interface; the runner is the only clock caller.

## Horse-owned code / optimization ban

R1 contains no horse code. The five horse directories are reserved with
README-only content, and a guard test fails if any Rust source appears in
them. When horses arrive (R4/R5), the R0 bans (custom allocator, unsafe
fast paths, SIMD/prefetch, parallelism, horse-specific string/hash
representations, one-horse-only inline/cold tuning) apply, with
`MECHANISM_INTRINSIC` marking as the only declared exception path.

## R1 code hygiene

The substrate itself is ordinary safe Rust (no `unsafe`, no custom
allocators, no platform intrinsics). `black_box` protection is a runner
concern (`std::hint::black_box` on inputs and completed states) per R0
§4.5.
