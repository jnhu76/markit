#!/usr/bin/env bash
# verify-r5.sh — R5 horse correctness + parity gate (CORRECTNESS ONLY).
#
# Validates the four incremental mechanism horses (H1 BLOCK_LOCAL_REPARSE,
# H2 FRAGMENT_REUSE, H3 OLD_TREE_SUBTREE_REUSE, H4 RESTART_CONVERGENCE)
# against the frozen R5 contract: formatting, lints, the full workspace
# test suite (43/43 golden fixtures, per-horse gate suites, adversarial
# small-model differentials), the frozen 370-slot CASE-MATRIX-v1
# differential for every horse (release profile), the R1/R3/R4
# regression gates, the static completed-state QUERY authority check,
# and the negative mutation check (deliberate mechanism-specific
# corruptions plus the R5-CORRECTIVE-1 probes must be DETECTED;
# temporary, always restored, never committed).
#
# This script runs NO benchmark campaign, times NOTHING, and produces NO
# research data.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

HORSES="block-local fragment-reuse old-tree-subtree-reuse restart-convergence"

step() { printf '\n=== %s ===\n' "$1"; }

step "cargo fmt --all -- --check"
cargo fmt --all -- --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "cargo test --workspace (43/43 fixtures + H1-H4 gate suites + adversarial differentials)"
cargo test --workspace

step "eager completed-state projection + QUERY law (H1-H4; R5-CORRECTIVE-1)"
cargo test -q -p markit-mdbench-block-local --test h1_gate h1_counters_and_eager_completion
cargo test -q -p markit-mdbench-fragment-reuse --test h2_gate h2_counters_and_eager_completion
cargo test -q -p markit-mdbench-old-tree-subtree-reuse --test h3_gate h3_counters_and_eager_completion
cargo test -q -p markit-mdbench-restart-convergence --test h4_gate h4_counters_and_eager_completion
printf 'H1_EAGER_COMPLETION_PASS\n'
printf 'H2_EAGER_COMPLETION_PASS\n'
printf 'H3_EAGER_COMPLETION_PASS\n'
printf 'H4_EAGER_COMPLETION_PASS\n'
printf 'EAGER_COMPLETION_VALIDATION_PASS\n'

for h in $HORSES; do
    step "frozen 370-slot case matrix differential: $h (release profile; ignored in debug)"
    cargo test --release -p "markit-mdbench-$h" --test matrix_r5 -- --ignored --nocapture --test-threads=8
done

step "R1 regression: harness substrate gate"
bash scripts/verify-r1.sh

step "R3 regression: freeze artifact gate"
bash scripts/verify-r3.sh

step "R4 regression: H0 reference gate"
bash scripts/verify-r4.sh

step "completed-state QUERY authority (static; R5-CORRECTIVE-1 MAJOR-3): every matrix helper projects from done.state.normalize_v1"
static_failed=0
for h in $HORSES; do
    f="mechanisms/$h/tests/matrix_r5.rs"
    count=$(grep -c "state\.normalize_v1()" "$f" || true)
    if [ "$count" -lt 3 ]; then
        echo "STATIC completed-state QUERY authority violated in $f (occurrences: $count)"
        static_failed=1
    fi
done
if [ "$static_failed" -ne 0 ]; then
    exit 1
fi

step "R5 negative gate: temporary mechanism-specific mutations are detected"
bash scripts/mutation-check-r5.sh

printf '\nR5 HORSE CORRECTNESS + PARITY GATE: PASS\n'
printf 'Correctness only: no benchmark campaign was run and no timing was recorded.\n'
