#!/usr/bin/env bash
# verify-r4.sh — R4 H0 reference gate (CORRECTNESS ONLY).
#
# Validates the H0 = FULL_REBUILD control horse against the frozen R4
# contract: formatting, lints, CORPUS-v1 receipt reproducibility, the
# full workspace test suite (43/43 golden fixtures + the corpus
# differential/invariant/QUERY/eager/attribution suites), the R1 and R3
# regression gates, and the negative mutation check (deliberate
# corruptions of H0 must be DETECTED by the suites).
#
# This script runs NO benchmark campaign, times NOTHING, and produces NO
# research data. Mutation testing here is temporary and always restored;
# no mutated source is ever committed.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

step() { printf '\n=== %s ===\n' "$1"; }

step "cargo fmt --all -- --check"
cargo fmt --all -- --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "CORPUS-v1 receipts regenerate byte-identically"
cargo run -q -p markit-mdbench-corpusgen --bin gen-receipts -- --check

step "cargo test --workspace (43/43 fixtures + differential + invariants)"
cargo test --workspace

step "R1 regression: harness substrate gate"
bash scripts/verify-r1.sh

step "R3 regression: freeze artifact gate"
bash scripts/verify-r3.sh

step "R4 negative gate: temporary H0 mutations are detected"
bash scripts/mutation-check-r4.sh

printf '\nR4 H0 REFERENCE GATE: PASS\n'
printf 'Correctness only: no benchmark campaign was run and no timing was recorded.\n'
