#!/usr/bin/env bash
# verify-six-horse.sh — MARKIT-76-SIX-HORSE-PERFORMANCE-CAMPAIGN-v1
# performance-freeze gate (#76 Gate B; FREEZE VERIFICATION ONLY; NO
# FORMAL COLLECTION).
#
# Validates the frozen six-horse campaign execution contract:
#   - formatting + lints + full workspace tests (incl. the campaign
#     crate's schedule determinism / negative tests, statistical
#     contract tests on synthetic data, attribution determinism, the
#     NON_RESEARCH fake-clock end-to-end smoke, and the memory-lane
#     instrument validation);
#   - #35 workload freeze verification + determinism (mdbench-corrective-c);
#   - six-horse campaign manifest verify / receipt verify / schedule
#     determinism;
#   - machine preflight (non-measuring, fail-closed) in all three scopes:
#     All / Timing / Attribution — 278,784 / 5,280 / 2,172 identity sets,
#     never inferred from flags.
#
# The campaign steps run the RELEASE build (the frozen
# `release-primary-v1` profile): the preflight verifies the build identity
# against that profile, so run this script ON THE FORMAL BENCHMARK
# MACHINE (jnhu@192.168.31.75) — failing closed anywhere else is the
# preflight's job.
#
# This script runs NO formal collection: no real campaign clock over the
# frozen surfaces, no p50/p95 from real mechanism timing, no speedup
# table, no ranking. The only executions are the NON_RESEARCH fake-clock
# smoke and the NON_RESEARCH memory smoke (instrument validation).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

step() { printf '\n=== %s ===\n' "$1"; }

step "cargo fmt --all -- --check"
cargo fmt --all -- --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "cargo test --workspace (six-horse freeze contract + prior gates)"
cargo test --workspace

step "#35 workload freeze verify (WORKLOAD_IDENTITY_CHANGED must be NO)"
cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c -- verify .

step "#35 workload freeze determinism"
cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c -- determinism .

step "build the frozen release profile (release-primary-v1)"
cargo build -q --release -p markit-mdbench-campaign
CAMPAIGN="./target/release/mdbench-campaign"

step "six-horse campaign manifest verify"
"$CAMPAIGN" manifest-verify .

step "six-horse campaign receipt verify"
"$CAMPAIGN" receipt-verify .

step "six-horse campaign schedule determinism (byte-identical regeneration)"
"$CAMPAIGN" schedule-determinism .

step "machine preflight (non-measuring, fail-closed) — scope All"
"$CAMPAIGN" preflight .

step "machine preflight — scope Timing (clean_state session 0)"
"$CAMPAIGN" preflight . --timing clean_state --session 0

step "machine preflight — scope Attribution (edit_write lane)"
"$CAMPAIGN" preflight . --attribution edit_write

step "campaign fake-clock NON_RESEARCH smoke (end-to-end plumbing, six horses)"
SMOKE_DIR="$(mktemp -d)"
trap 'rm -rf "$SMOKE_DIR"' EXIT
"$CAMPAIGN" smoke . --out "$SMOKE_DIR" --cases 1 --warmup 1 --measured 1 --inject-failure

step "memory smoke (NON_RESEARCH instrument validation, counting allocator)"
cargo run -q --release -p markit-mdbench-campaign --bin mdbench-memory-smoke

step "FORMAL_PERFORMANCE_COLLECTION_STARTED = NO (nothing above ran a formal campaign clock)"
printf 'SIX_HORSE_PERFORMANCE_FREEZE_VERIFY_DONE\n'
