#!/usr/bin/env bash
# mutation-check-r4.sh — R4 negative gate.
#
# Proves the R4 correctness suites DETECT deliberate corruptions of the
# H0 reference mechanism. Five temporary mutations are applied one at a
# time to the working tree; each must (a) still compile — a compile
# error is NOT detection, and (b) make its targeted detector test FAIL.
# The tree is restored after every mutation (and on any exit); nothing
# mutated is committed.
#
# R5 AMENDMENT (explicit, not silent): since the R5 shared-substrate
# extraction, H0's parser and inline semantics live in the shared
# shared-grammar crate (mechanisms/full-rebuild is the H0 wrapper) and
# this script's M1-M4 targets moved accordingly. The mutation classes,
# the corrupted expressions, and the detector (full-rebuild's own suites
# exercising H0 end-to-end through the shared crate) are unchanged.
# The full-rebuild crate itself no longer contains parser.rs/inline.rs,
# which is what the pre-amendment script pointed at.
#
# Mutation classes (one representative each):
#   M1  span off-by-one            (paragraph start, parser.rs)
#   M2  byte/Unicode-fold drop     (inline.rs, label normalization D7)
#   M3  refdef lookup last-wins    (inline.rs, §9.3 first-wins)
#   M4  unclosed-fence span wrong  (parser.rs, §8 EOF rule)
#   M5  update consults old source (lib.rs, H0 must parse the POST source)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PARSER=shared-grammar/src/parser.rs
INLINE=shared-grammar/src/inline.rs
LIB=mechanisms/full-rebuild/src/lib.rs
TARGETS=("$PARSER" "$INLINE" "$LIB")

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

# run_detector <test-args...>; fails the gate if the test PASSES
# (mutation survived) or fails to BUILD (invalid mutation).
run_detector() {
    if cargo build -q -p markit-mdbench-full-rebuild 2>/dev/null; then
        if cargo test -q -p markit-mdbench-full-rebuild -- "$@" >/tmp/r4-mutation-detector.log 2>&1; then
            echo "MUTATION SURVIVED: $* passed on mutated code" >&2
            tail -20 /tmp/r4-mutation-detector.log >&2 || true
            exit 1
        fi
    else
        echo "MUTATION INVALID (compile error is not detection)" >&2
        exit 1
    fi
}

detected=0

echo "--- M1: paragraph span off-by-one (fixtures must catch) ---"
apply "$PARSER" 's/self\.para = Some\(OpenPara \{\n                    start: col \+ s,/self.para = Some(OpenPara {\n                    start: col + s + 1,/' 'start: col + s + 1,'
run_detector --test fixtures
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- M2: drop ASCII case-fold in label normalization (F036) ---"
apply "$INLINE" 's/out\.push\(b\.to_ascii_lowercase\(\) as char\);/out.push(b as char);/' 'out.push(b as char);'
run_detector --test fixtures
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- M3: reference lookup last-wins (F033) ---"
apply "$INLINE" 's/self\.entries\n            \.iter\(\)/self.entries.iter().rev()/' 'self.entries.iter().rev()'
run_detector --test fixtures
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- M4: unclosed-fence EOF span wrong (F017) ---"
apply "$PARSER" 's/let mut end = self\.end;/let mut end = f.start;/' 'let mut end = f.start;'
run_detector --test fixtures
restore_all; detected=$((detected + 1)); echo "detected"

echo "--- M5: update re-parses the OLD source (differential must catch) ---"
apply "$LIB" 's/parse_with_attribution\(post_source\.as_bytes\(\), cx\)/parse_with_attribution(_old_source.as_bytes(), cx)/' 'parse_with_attribution(_old_source.as_bytes(), cx)'
run_detector --test corpus_differential -- update_result_is_independent_of_the_old_state
restore_all; detected=$((detected + 1)); echo "detected"

restore_all
echo
echo "R4 MUTATION CHECK: $detected/5 DETECTED"
