#!/usr/bin/env bash
# verify-r3.sh — R3 freeze gate (static artifacts only).
#
# Validates the R3 freeze documents/manifests/fixtures. It runs NO
# benchmark, times NOTHING, and implements NO Markdown semantics.
# Complement of scripts/verify-r1.sh (the R1 regression guard).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== R3 gate: static artifact validation =="
python3 scripts/verify_r3.py

echo
echo "== R3 gate: no horse-outcome language in R3 artifacts =="
# R3 may state workload semantics; it may not predict winners.
if grep -RInE '(H[0-4] (wins|loses|should win))|(should (be faster|defeat))|(expected winner)' \
      grammar/ corpus/ mutations/ cases/ protocol/R3-GRAMMAR-CORPUS-MUTATION-FREEZE.md 2>/dev/null; then
  echo "R3 FREEZE GATE: FAIL (horse-outcome language found)"
  exit 1
fi
echo "clean"

echo
echo "== R3 gate: scope proof (no parser/mechanism code in R3 trees) =="
if find grammar corpus mutations cases -name '*.rs' -o -name '*.c' -o -name '*.mbt' | grep -q .; then
  echo "R3 FREEZE GATE: FAIL (implementation files in R3 artifacts)"
  exit 1
fi
echo "clean"

echo
echo "R3 FREEZE GATE: PASS"
