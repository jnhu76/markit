#!/usr/bin/env bash
# mutation-check-r5.sh — R5 negative gate.
#
# Proves the R5 correctness suites DETECT deliberate, mechanism-specific
# corruptions of the four incremental horses. One representative mutation
# per horse is applied to the working tree; each must (a) still compile —
# a compile error is NOT detection — and (b) make its targeted detector
# test FAIL. The tree is restored after every mutation (and on any exit);
# nothing mutated is ever committed.
#
# Mutation classes (one per horse, each aimed at that horse's reuse
# authority):
#   H1  every terminated block "continues"  (block-local guards)
#   H2  live-side paragraph margin removed  (fragment-reuse splice)
#   H3  live-side paragraph margin removed  (old-tree cursor splice)
#   H4  convergence paragraph margin removed (restart-convergence (e);
#       detected by the adversarial differential — the in-file probes
#       exercise (e) only where the restart-boundary backup already
#       covers the join, so they stay green under its removal)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

H1=mechanisms/block-local/src/lib.rs
H2=mechanisms/fragment-reuse/src/lib.rs
H3=mechanisms/old-tree-subtree-reuse/src/lib.rs
H4=mechanisms/restart-convergence/src/lib.rs
TARGETS=("$H1" "$H2" "$H3" "$H4")

restore_all() {
    for f in "${TARGETS[@]}"; do
        git checkout -- "$f" 2>/dev/null || true
    done
}
trap restore_all EXIT

for f in "${TARGETS[@]}"; do
    if ! git diff --quiet -- "$f"; then
        echo "refusing to run: $f has uncommitted changes" >&2
        exit 1
    fi
done

# apply <file> <perl-substitution> <new-pattern-to-verify>
apply() {
    local file="$1" sub="$2" verify="$3"
    perl -0pi -e "$sub" "$file"
    if ! grep -qF "$verify" "$file"; then
        echo "MUTATION SCRIPT BUG: substitution did not apply to $file" >&2
        exit 1
    fi
}

detected=0

echo "--- H1: every terminated block reports continuation (guards must catch) ---"
apply "$H1" 's/BlockFacts::Terminated => false,/BlockFacts::Terminated => true,/' 'BlockFacts::Terminated => true,'
cargo build -q -p markit-mdbench-block-local 2>/dev/null || { echo "MUTATION INVALID (compile error is not detection)" >&2; exit 1; }
if cargo test -q -p markit-mdbench-block-local --test h1_gate h1_fallback_pass_guard_probes >/tmp/r5-mutation-detector.log 2>&1; then
    echo "MUTATION SURVIVED: guard probes passed on mutated code" >&2
    tail -20 /tmp/r5-mutation-detector.log >&2 || true
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- H2: left fragment kept across a destroyed boundary (adversarial must catch) ---"
apply "$H2" 's/if !blank_before && es < first_lf \{/if false {/' 'if false {'
cargo build -q -p markit-mdbench-fragment-reuse 2>/dev/null || { echo "MUTATION INVALID (compile error is not detection)" >&2; exit 1; }
if cargo test -q -p markit-mdbench-fragment-reuse --test adversarial_r5 adversarial_small_model_differential_504_sequences >/tmp/r5-mutation-detector.log 2>&1; then
    echo "MUTATION SURVIVED: adversarial differential passed on mutated code" >&2
    tail -20 /tmp/r5-mutation-detector.log >&2 || true
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- H3: cursor splice without the live paragraph margin (adversarial must catch) ---"
apply "$H3" 's/if !sg::parser::all_spaces\(self\.post, prev_ls, pos - 1\) \{/if false {/' 'if false {'
cargo build -q -p markit-mdbench-old-tree-subtree-reuse 2>/dev/null || { echo "MUTATION INVALID (compile error is not detection)" >&2; exit 1; }
if cargo test -q -p markit-mdbench-old-tree-subtree-reuse --test adversarial_r5 adversarial_small_model_differential_504_sequences >/tmp/r5-mutation-detector.log 2>&1; then
    echo "MUTATION SURVIVED: adversarial differential passed on mutated code" >&2
    tail -20 /tmp/r5-mutation-detector.log >&2 || true
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- H4: convergence without the paragraph margin, predicate (e) (adversarial must catch) ---"
apply "$H4" 's/if !sg::parser::all_spaces\(self\.post, prev_ls, pos - 1\) \{/if false {/' 'if false {'
cargo build -q -p markit-mdbench-restart-convergence 2>/dev/null || { echo "MUTATION INVALID (compile error is not detection)" >&2; exit 1; }
if cargo test -q -p markit-mdbench-restart-convergence --test adversarial_r5 adversarial_small_model_differential_504_sequences >/tmp/r5-mutation-detector.log 2>&1; then
    echo "MUTATION SURVIVED: adversarial differential passed on mutated H4 (e) removal" >&2
    tail -20 /tmp/r5-mutation-detector.log >&2 || true
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

restore_all
echo
echo "R5 MUTATION CHECK: $detected/4 DETECTED"
