#!/usr/bin/env bash
# verify-r7.sh — MARKIT-31-PRIMARY-PERFORMANCE-CAMPAIGN-FREEZE-1 gate
# (FREEZE ONLY; NO PRIMARY TIMING).
#
# Validates the frozen campaign execution contract:
#   - formatting + lints + full workspace tests (incl. the campaign
#     crate's schedule determinism / negative tests, statistical
#     contract tests on synthetic data, attribution determinism, and
#     the NON_RESEARCH fake-clock end-to-end smoke);
#   - #35 workload freeze verification + determinism (mdbench-corrective-c);
#   - campaign manifest verify / receipt verify / schedule determinism;
#   - machine preflight (non-measuring, fail-closed).
#
# The campaign steps run the RELEASE build (the frozen
# `release-primary-v1` profile): the preflight verifies the build identity
# against that profile, so run this script ON THE PRIMARY BENCHMARK
# MACHINE — failing closed anywhere else is the preflight's job.
#
# This script runs NO primary timing: no real clock over the campaign,
# no p50/p95 from real mechanism timing, no speedup table, no ranking.
# The only execution is the NON_RESEARCH fake-clock smoke.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

step() { printf '\n=== %s ===\n' "$1"; }

step "cargo fmt --all -- --check"
cargo fmt --all -- --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "cargo test --workspace (campaign freeze contract + prior gates)"
cargo test --workspace

step "#35 workload freeze verify (WORKLOAD_IDENTITY_CHANGED must be NO)"
cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c -- verify .

step "#35 workload freeze determinism"
cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c -- determinism .

step "build the frozen release profile (release-primary-v1)"
cargo build -q --release -p markit-mdbench-campaign
CAMPAIGN="./target/release/mdbench-campaign"

step "campaign manifest verify"
"$CAMPAIGN" manifest-verify .

step "campaign receipt verify"
"$CAMPAIGN" receipt-verify .

step "campaign schedule determinism (byte-identical regeneration)"
"$CAMPAIGN" schedule-determinism .

step "machine preflight (non-measuring, fail-closed)"
"$CAMPAIGN" preflight .

step "campaign fake-clock NON_RESEARCH smoke (end-to-end plumbing)"
SMOKE_DIR="$(mktemp -d)"
trap 'rm -rf "$SMOKE_DIR"' EXIT
"$CAMPAIGN" smoke . --out "$SMOKE_DIR" --cases 1 --warmup 1 --measured 1 --inject-failure

step "PRIMARY_TIMING = NOT_STARTED (nothing above ran a real campaign clock)"
printf 'PRIMARY_PERFORMANCE_CAMPAIGN_FREEZE_VERIFY_DONE\n'
