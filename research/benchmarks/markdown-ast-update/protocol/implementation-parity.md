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

## Reserved second profile

The R0 optimization-sensitivity check requires a second frozen profile
(e.g. LTO-off) for key representative cases. It is NOT added now; when it
is needed it must be added to the workspace manifest and this file before
any run that relies on it.

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
