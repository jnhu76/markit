#!/usr/bin/env bash
# scripts/verify-r1.sh — R1 acceptance gate ONLY.
#
# Validates the harness substrate: formatting, compilation, tests, lints,
# the null mechanism end-to-end through the real runner, and schema
# validity of the emitted smoke rows.
#
# This script runs NO benchmark campaign and produces NO research data.
# Smoke output is temporary and labeled NON_RESEARCH_RESULT.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

step() { printf '\n=== %s ===\n' "$1"; }

step "cargo fmt --all -- --check"
cargo fmt --all -- --check

step "cargo check --workspace --all-targets"
cargo check --workspace --all-targets

step "cargo test --workspace"
cargo test --workspace

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "null mechanism end-to-end through the real runner (R1_SMOKE_ONLY / NON_RESEARCH_RESULT)"
SMOKE_OUT="$(mktemp -t mdbench-r1-smoke.XXXXXX.jsonl)"
cargo run -q -p markit-mdbench-runner --bin r1-smoke -- "$SMOKE_OUT"

step "supervised worker end-to-end (R1-CORRECTIVE-1 failure isolation, NON_RESEARCH_RESULT)"
WORKER_OUT="$(mktemp -t mdbench-r1-worker.XXXXXX.jsonl)"
WORKER_JOB="$(mktemp -t mdbench-r1-worker.XXXXXX.json)"
printf '{"mode":"null_smoke_update"}' > "$WORKER_JOB"
cargo run -q -p markit-mdbench-runner --bin mdbench-worker -- "$WORKER_JOB" "$WORKER_OUT"
printf '%s' "$(cat "$WORKER_OUT")" | python3 -c '
import json, sys
row = json.loads(sys.stdin.read())
assert row["provenance_ref"] == "R1_SMOKE_ONLY/NON_RESEARCH_RESULT", row
assert row["execution_status"] == "pass", row
assert row["correctness_status"] == "pass", row
assert row["measurement"]["lane"] == "timing", row
m = row["measurement"]["metrics"]
assert m["total_ns"] == m["prepare_ns"] + m["native_ns"], row
'
rm -f "$WORKER_JOB" "$WORKER_OUT"

step "smoke rows are schema-valid JSONL"
ROW_COUNT=0
while IFS= read -r line; do
    ROW_COUNT=$((ROW_COUNT + 1))
    printf '%s' "$line" | python3 -c '
import json, sys
row = json.load(sys.stdin)
assert row["schema_version"] == 1, row
assert row["protocol_version"] == "R0-FROZEN-V1", row
assert row["provenance_ref"] == "R1_SMOKE_ONLY/NON_RESEARCH_RESULT", row
assert row["execution_status"] == "pass", row
assert row["correctness_status"] == "pass", row
assert row["measurement"]["lane"] == "timing", row
m = row["measurement"]["metrics"]
prep, nat, tot = m["prepare_ns"], m["native_ns"], m["total_ns"]
if prep == "NOT_APPLICABLE":
    assert tot == nat, row
else:
    assert isinstance(prep, int) and tot == prep + nat, row
'
done < "$SMOKE_OUT"
[ "$ROW_COUNT" -eq 2 ] || { echo "expected exactly 2 smoke rows, got $ROW_COUNT"; exit 1; }
rm -f "$SMOKE_OUT"

step "result-schema-v1.json matches the Rust model (drift guard)"
GENERATED="$(mktemp -t mdbench-schema.XXXXXX.json)"
cargo run -q -p markit-mdbench-runner --bin mdbench-gen-schema -- "$GENERATED" >/dev/null
if ! diff -q "$GENERATED" protocol/result-schema-v1.json >/dev/null; then
    echo "protocol/result-schema-v1.json has drifted from ResultRowV1; regenerate it" >&2
    exit 1
fi
rm -f "$GENERATED"

printf '\n=== R1 ACCEPTANCE GATE: PASS ===\n'
printf 'HARNESS_SUBSTRATE_PASS conditions verified. No benchmark campaign was run.\n'
