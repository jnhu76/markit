#!/usr/bin/env bash
# mutation-check-r5.sh — R5 negative gate.
#
# Proves the R5 correctness suites DETECT deliberate, mechanism-specific
# corruptions of the four incremental horses. One representative mutation
# per horse plus the three R5-CORRECTIVE-1 probes is applied to the
# working tree; each must (a) still compile — a compile error is NOT
# detection — and (b) make its targeted detector test FAIL. The tree is
# restored after every mutation (and on any exit); nothing mutated is
# ever committed.
#
# Mechanism mutation classes (one per horse, each aimed at that horse's
# reuse authority):
#   H1  every terminated block "continues"  (block-local guards)
#   H2  live-side paragraph margin removed  (fragment-reuse splice)
#   H3  live-side paragraph margin removed  (old-tree cursor splice)
#   H4  convergence paragraph margin removed (restart-convergence (e);
#       detected by the adversarial differential — the in-file probes
#       exercise (e) only where the restart-boundary backup already
#       covers the join, so they stay green under its removal)
#
# Corrective probes (R5-CORRECTIVE-1 §12; D and E added by
# R5-CORRECTIVE-2, E from its reviewer round):
#   A   H1 prefix ownership move reverted to a clone while still
#       reporting nodes_reused > 0 (pseudo reuse) — caught by the
#       ownership pass-through witness
#   B   inline source-inspection event emission disabled on a fresh
#       parse path — caught by the raw-event attribution gate
#   C   completed-state QUERY bypassed back to the Pending's eager
#       result — caught by the static completed-state authority check
#   D   a prepare-phase margin inspection event removed from H4's
#       restart-boundary scan — caught by the H4 attribution test
#   E   the splice tail-byte inspection event removed from the shared
#       block scanner (`splice_to`) — caught by the shared splice
#       attribution test
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

H1=mechanisms/block-local/src/lib.rs
H2=mechanisms/fragment-reuse/src/lib.rs
H3=mechanisms/old-tree-subtree-reuse/src/lib.rs
H4=mechanisms/restart-convergence/src/lib.rs
SGI=shared-grammar/src/inline.rs
SGP=shared-grammar/src/parser.rs
H1M=mechanisms/block-local/tests/matrix_r5.rs
TARGETS=("$H1" "$H2" "$H3" "$H4" "$SGI" "$SGP" "$H1M")

restore_all() {
    for f in "${TARGETS[@]}"; do
        git checkout -- "$f" 2>/dev/null || true
    done
}

# SAFETY: the clean-tree check runs BEFORE the restore trap is installed.
# A refusal exit must never restore (revert) anything: restoring on a
# dirty tree would silently discard the caller's uncommitted work (this
# exact accident reverted corrective edits once — the trap used to be
# installed above this check).
for f in "${TARGETS[@]}"; do
    if ! git diff --quiet -- "$f"; then
        echo "refusing to run: $f has uncommitted changes" >&2
        exit 1
    fi
done
trap restore_all EXIT

# apply <file> <perl-substitution> <new-pattern-to-verify>
apply() {
    local file="$1" sub="$2" verify="$3"
    perl -0pi -e "$sub" "$file"
    if ! grep -qF "$verify" "$file"; then
        echo "MUTATION SCRIPT BUG: substitution did not apply to $file" >&2
        exit 1
    fi
}

# static_query_authority — Probe C's detector (also part of verify-r5.sh):
# every matrix helper must project the QUERY authority from the
# COMPLETED state (done.state.normalize_v1), never from the eager
# Pending result. Three occurrences per file: completed_doc, the update
# case, and the chained follow-up.
static_query_authority() {
    local failed=0
    for h in block-local fragment-reuse old-tree-subtree-reuse restart-convergence; do
        local f="mechanisms/$h/tests/matrix_r5.rs"
        local count
        count=$(grep -c "state\.normalize_v1()" "$f" || true)
        if [ "$count" -lt 3 ]; then
            echo "STATIC completed-state QUERY authority violated in $f (occurrences: $count)" >&2
            failed=1
        fi
    done
    return $failed
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

echo "--- CORRECTIVE PROBE A: H1 prefix ownership move reverted to clone (ownership witness must catch) ---"
apply "$H1" 's/moved_prefix\.push\(e\);/moved_prefix.push(e.clone());/' 'moved_prefix.push(e.clone());'
cargo build -q -p markit-mdbench-block-local 2>/dev/null || { echo "MUTATION INVALID (compile error is not detection)" >&2; exit 1; }
if cargo test -q -p markit-mdbench-block-local --test h1_gate h1_prefix_reuse_is_ownership_pass_through >/tmp/r5-mutation-detector.log 2>&1; then
    echo "MUTATION SURVIVED: ownership witness passed on clone-based pseudo reuse" >&2
    tail -20 /tmp/r5-mutation-detector.log >&2 || true
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- CORRECTIVE PROBE B: inline inspection event emission disabled (attribution gate must catch) ---"
apply "$SGI" 's/sink\.record_source_inspection\(\s*\n\s*markit_mdbench_common::SourceVersion::Post,\s*\n\s*ss as u64,\s*\n\s*se as u64,\s*\n\s*\);/let _ = (ss, se);/' 'let _ = (ss, se);'
cargo build -q -p markit-mdbench-full-rebuild 2>/dev/null || { echo "MUTATION INVALID (compile error is not detection)" >&2; exit 1; }
if cargo test -q -p markit-mdbench-full-rebuild --test inline_inspection >/tmp/r5-mutation-detector.log 2>&1; then
    echo "MUTATION SURVIVED: attribution gate passed with inline emission disabled" >&2
    tail -20 /tmp/r5-mutation-detector.log >&2 || true
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- CORRECTIVE PROBE C: completed-state QUERY bypassed to the Pending result (static gate must catch) ---"
apply "$H1M" 's/    let doc = done\.state\.normalize_v1\(\);\n    assert_eq!\(\n        normalized_checksum\(&doc\),/    let doc = eager.clone();\n    assert_eq!(\n        normalized_checksum(&doc),/' 'let doc = eager.clone();'
if static_query_authority; then
    echo "MUTATION SURVIVED: static completed-state QUERY authority passed on the bypassed helper" >&2
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- CORRECTIVE PROBE D: H4 restart-boundary margin inspection event removed (attribution test must catch) ---"
apply "$H4" 's/cx\.sink\.record_source_inspection\(\s*\n\s*markit_mdbench_common::SourceVersion::Old,\s*\n\s*sep_lo as u64,\s*\n\s*sep_hi as u64,\s*\n\s*\);/let _ = (sep_lo, sep_hi);/' 'let _ = (sep_lo, sep_hi);'
cargo build -q -p markit-mdbench-restart-convergence 2>/dev/null || { echo "MUTATION INVALID (compile error is not detection)" >&2; exit 1; }
if cargo test -q -p markit-mdbench-restart-convergence --test h4_gate h4_prepare_margins_report_source_inspection >/tmp/r5-mutation-detector.log 2>&1; then
    echo "MUTATION SURVIVED: H4 attribution test passed with the margin event removed" >&2
    tail -20 /tmp/r5-mutation-detector.log >&2 || true
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- CORRECTIVE PROBE E: shared splice_to tail-byte inspection event removed (attribution test must catch) ---"
apply "$SGP" 's/self\.sink\.record_source_inspection\(\s*\n\s*markit_mdbench_common::SourceVersion::Post,\s*\n\s*prev as u64,\s*\n\s*new_pos as u64,\s*\n\s*\);/let _ = (prev, new_pos);/' 'let _ = (prev, new_pos);'
cargo build -q -p markit-mdbench-shared-grammar 2>/dev/null || { echo "MUTATION INVALID (compile error is not detection)" >&2; exit 1; }
if cargo test -q -p markit-mdbench-shared-grammar --lib splice_take_reports_its_tail_byte_source_inspection >/tmp/r5-mutation-detector.log 2>&1; then
    echo "MUTATION SURVIVED: shared splice attribution test passed with the tail-byte event removed" >&2
    tail -20 /tmp/r5-mutation-detector.log >&2 || true
    exit 1
fi
restore_all; detected=$((detected + 1)); echo "detected"

restore_all
echo
echo "R5 MUTATION CHECK: $detected/9 DETECTED"
