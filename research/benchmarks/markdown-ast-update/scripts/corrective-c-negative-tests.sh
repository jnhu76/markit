#!/usr/bin/env bash
# CORRECTIVE-C negative tests (task §53): every failure-closed path of the
# freeze must actually fail when its authority is violated. Run from the
# benchmark root after `generate`. No test mutates the repository: every
# tamper happens on a temporary copy, restored before the next test.
set -u
cd "$(dirname "$0")/.."

BIN="cargo run -q -p markit-mdbench-workload-freeze --bin mdbench-corrective-c --"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
fails=0

expect_ok() {
    local label="$1"; shift
    if "$@" >"$TMP/out.txt" 2>"$TMP/err.txt"; then
        echo "NEGATIVE-SETUP-OK: $label"
    else
        echo "NEGATIVE-SETUP-FAILED (expected success): $label"
        cat "$TMP/err.txt"
        fails=$((fails+1))
    fi
}

expect_fail() {
    local label="$1"; shift
    if "$@" >/dev/null 2>"$TMP/err.txt"; then
        echo "NEGATIVE-TEST-FAILED (expected nonzero exit): $label"
        fails=$((fails+1))
    elif grep -q "GENERATE_FAILED\|VERIFY_FAILED\|DRY_RUN_FAILED" "$TMP/err.txt"; then
        echo "NEGATIVE-TEST-OK: $label"
    else
        echo "NEGATIVE-TEST-FAILED (unexpected error): $label"
        cat "$TMP/err.txt"
        fails=$((fails+1))
    fi
}

# Isolated copy of the benchmark inputs (sources, selections, frozen
# payloads) without the acquisition cache/build noise.
mkdir -p "$TMP/root"
cp -r workloads "$TMP/root/workloads"
rm -rf "$TMP/root/workloads/_cache"

expect_ok "pristine copy generates" $BIN generate "$TMP/root"

REPAIR_FILE="$TMP/root/workloads/sources/opentelemetry-spec/files/specification/library-layout.md"

# 1. Source-byte tamper: SOURCE_AUTHORITY must fail closed.
cp "$REPAIR_FILE" "$TMP/backup.md"
printf 'x' >> "$REPAIR_FILE"
expect_fail "tampered source byte (sha mismatch)" $BIN verify "$TMP/root"
cp "$TMP/backup.md" "$REPAIR_FILE"

# 2. Payload tamper: a mutated edit coordinate must fail re-validation.
python3 - "$TMP/root" <<'EOF'
import json, sys
path = sys.argv[1] + "/workloads/payloads/edit-write-manifest-v1.jsonl"
rows = [json.loads(l) for l in open(path)]
for r in rows:
    if r["step"] == 0:
        r["edit"]["edit_start"] += 1
        break
with open(path, "w") as fh:
    for r in rows:
        fh.write(json.dumps(r, sort_keys=True) + "\n")
EOF
expect_fail "tampered payload edit offset" $BIN verify "$TMP/root"

# 3. Receipt tamper: digest binding must fail.
python3 - "$TMP/root" <<'EOF'
import json, sys
path = sys.argv[1] + "/workloads/payloads/freeze-receipt-v1.json"
receipt = json.load(open(path))
receipt["artifact_sha256"][0]["sha256"] = "0" * 64
json.dump(receipt, open(path, "w"), sort_keys=True)
EOF
expect_fail "tampered freeze receipt digest" $BIN verify "$TMP/root"

# 4. Repair authority: a changed selection identity must fail the overlay.
python3 - "$TMP/root" <<'EOF'
import json, sys
path = sys.argv[1] + "/workloads/selections/syntax-coverage-repair-v1.json"
repair = json.load(open(path))
repair["basis"]["selection_identity"]["source_lock_sha256"] = "0" * 64
json.dump(repair, open(path, "w"), indent=1, sort_keys=True)
EOF
expect_fail "repair identity drift" $BIN generate "$TMP/root"

# 5. Repair source identity: a drifted universe hash must fail the add.
python3 - "$TMP/root" <<'EOF'
import json, sys
path = sys.argv[1] + "/workloads/selections/syntax-coverage-repair-v1.json"
repair = json.load(open(path))
for candidate in repair["evaluated_candidates"]:
    if candidate["key"] == repair["selected_key"]:
        candidate["sha256"] = "1" * 64
json.dump(repair, open(path, "w"), indent=1, sort_keys=True)
EOF
expect_fail "repair candidate hash drift" $BIN generate "$TMP/root"

# 6. Repair membership relabel: a tampered membership_added that would
#    silently move the added file into another logical set must fail.
python3 - "$TMP/root" <<'EOF'
import json, sys
path = sys.argv[1] + "/workloads/selections/syntax-coverage-repair-v1.json"
repair = json.load(open(path))
repair["membership_added"] = "representative"
json.dump(repair, open(path, "w"), indent=1, sort_keys=True)
EOF
expect_fail "repair membership relabel (set boundary)" $BIN generate "$TMP/root"

if [ "$fails" -eq 0 ]; then
    echo "NEGATIVE_TESTS_OK all failure-closed paths held"
    exit 0
fi
exit 1
