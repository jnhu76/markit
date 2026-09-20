# REVIEW-CORRECTIVE-B-ADVERSARIAL-v1 — §48 adversarial review record

```text
stage reviewed   CORRECTIVE-B-PROFILE-SELECT-v1 (issue #35)
reviewer         fresh-context reviewer agent; no part in building the stage;
                 read-only (no edits, no merges, no future-corrective work)
when              after generation + self-audit fixes, before the stage commit
method           read the full profile-select crate, semantics profiler
                 modules, contract inputs and all 19 artifacts; independently
                 recomputed bins, cells, caps, winners, hashes and every
                 headline report number from candidate-rows-v1.jsonl /
                 artifacts with python; ran `verify` (PASS), both crate test
                 suites (green), and a non-writing `run --dry` which
                 reproduced the recorded profile JSONL byte count and sha256
                 (cfc3854500e0…, 1,255,213,034 bytes) from current sources
```

## Verdicts

```text
Q1  Did any H0-H4 timing/work/reuse fact influence candidate membership?
    SOUND — crate deps exactly semantics/serde/serde_json/sha2 (test-enforced
    guard); membership computed solely from G0 structural facts; grep for
    horse/timing/throughput identifiers over crate + artifacts clean.

Q2  Can a non-recognized/candidate syntax fact be accidentally counted as
    strict coverage?
    SOUND — is_strict_coverage() requires Recognized + host_context +
    StrictLaneCoverage; probes land in candidate/ambiguous/unknown/nonhost
    buckets only; grammar-lanes-v1.json construct_scopes is the grade
    authority and artifacts.rs mirrors it post-fix.

Q3  Can a Markdown-looking construct inside fence/code/raw HTML be counted
    as host syntax?
    SOUND — two-rule discipline (candidates fully inside literal spans;
    recognized nodes host only when not strictly inside a non-host span);
    empirically: table-shaped text inside fences has table_count=0 and
    non-host candidates recorded; non-host math (384) separated from host
    ambiguous math (10,076).

Q4  Did the selector call the 3,970-source population representative of all
    Markdown?
    SOUND — selection-config population_statement: "not a random sample of
    all Markdown usage"; report repeats it and never generalizes.

Q5  Did EIP/KEP/RFC/Swift proposal concentration still dominate the selected
    representative set despite domain controls?
    SOUND — RFC 72.17% of candidates → 22.22% of the representative core
    (4/18), 38.89% realism; recomputed and matches the report.

Q6  Can one project consume excessive representative slots?
    SOUND — PROJECT_CAP=3 enforced before admission (max observed: 3);
    domain cap holds (RFC 4/18 ≤ 6); cap_relaxations = [].

Q7  Does REPRESENTATIVE_SET actually cover its declared feature/joint cells?
    SOUND — all 28 feature cells with eligible>0 have selected>0; reviewer
    recomputed all 76 attainable cells (28 single + 48 joint incl. the
    triggered optional fifth) as covered; trace shows 63→0 over 18 picks.

Q8  Are EXTREMAL_SET values genuine source tails rather than
    generated/corrupt artifacts?
    SOUND — verified against real bytes (cpp 841,419 B; eip-7643 single
    ~138 KB csv fence; openmlsys 552/561 CJK bytes; keps 783 real 21,288 B;
    the 76/89 B MyST include-stubs are genuine files).

Q9  Does SYNTAX_COVERAGE_SET cover syntax/context cells rather than merely
    files containing punctuation?
    SOUND — cells defined over parser-recognized kinds, structural table
    facts, span containment, and graded probe evidence; probes are never
    parse facts; both syntax-set additions principled (table:wide strict;
    escaped_pipe candidate-only).

Q10 Were unsupported/deferred syntax cases clearly marked REALISM_ONLY /
    deferred rather than strict?
    SOUND — eip-3198 carries REALISM_ONLY + GRAMMAR_EXTENSION_REQUIRED;
    math cells ambiguous_or_unknown + LANE_DEFERRED; the four extension
    targets graded out_of_lane_candidate with 0 recognized occurrences,
    matching grammar-lanes-v1.json (regression test passes).

Q11 Were complete real files preserved byte-for-byte?
    SOUND — all 36 selected members re-hashed against materialized bytes:
    0 mismatches; rows require hash_match && materialized for eligibility.

Q12 Did any near-duplicate analysis silently delete candidate evidence?
    SOUND — diagnostic only by code (no removal path; tie-break input only);
    all 3,970 rows retained; exact groups hard-checked vs acquisition
    registry; no two selected members share a sha256.

Q13 Can two runs produce different set membership or selection order?
    SOUND — no HashMap/HashSet in the crate; all orderings BTree/sorts with
    lexical tie-breaks; determinism command byte-compares all 19 artifacts
    across two clean runs; reviewer's `run --dry` reproduced the recorded
    profile JSONL sha256 and all pipeline numbers.

Q14 Did CORRECTIVE-B accidentally generate final canonical edits/payloads?
    SOUND — payload-field grep hits only the contract's prohibition text;
    payloads/cases trees untouched; outputs carry only boolean
    transition_opportunity flags.

Q15 Did any full-document choice rely on hand-picking after seeing the
    selector output?
    SOUND — three named hard candidates + percentile formulas frozen in the
    contract and matching code; reviewer mechanically recomputed all three
    winners and the mechanical seventh document from the contract rules.

Q16 Are candidate→eligible→selected biases explicitly reported rather than
    hidden?
    SOUND — full 9-dimension bias artifact + share tables; report's
    distortion table matches artifacts exactly; re-weighting stated in prose.

Q17 Are logical membership and physical-file dedup kept separate?
    SOUND — 39 logical memberships vs 36 physical files; overlap pairs
    recomputed and match.

Q18 Does any metric depend on an undefined/deferred grammar semantic,
    especially math occupancy?
    SOUND — math_occupancy is None in every lane record (by construction
    with an explicit note), absent from features/bias/contract; openmlsys
    uses math as G1 occurrence counts only.

Q19 Does any selected G1 Table file get treated as H0-H4 performance-ready
    even though horses are not implemented for G1?
    SOUND — every G1 horse is NotImplemented (registry + dedicated test);
    no artifact or report claims H0-H4 readiness for any selected file.

Q20 Did this task accidentally begin #31 performance execution?
    SOUND — results trees contain only pre-stage READMEs; no timing fields
    anywhere; no runner/instrumentation dependency; scope statement explicit.
```

Overall: **REVIEW: no MAJOR findings.**

## MINOR findings and dispositions

1. **Report number errors in CORRECTIVE-B-REPORT-1.md** — EXTREMAL_SET is
   8 OBSERVED_MAXIMUM + **7** TAIL_REPLICATE roles (not 5; over 6 files,
   node/fs carries two); replicates cover **7 of 8** dimensions (not 5 of
   8); the smallest selected margin is cjk=MEDIUM/HIGH only
   (reference_density=LOW has 2). All three understate achieved coverage.
   **FIXED** in the report before the PASS verdict was recorded.
2. **Run-header / freeze-order evidence** — at review time no CORRECTIVE-B
   commit existed and the report claimed a clean PR head and a
   "committed before membership" contract freeze, which would be
   unfalsifiable in a single commit. **FIXED**: the freeze claim now points
   at verifiable evidence (mtime ordering, contract↔code↔config match,
   sha256 binding in artifact identity, reviewer recompute); the
   worktree-status claim is true of the PR head this report is committed
   in.
3. **`generator_tool_sha256` was not checkout-stable** (hashed absolute
   source paths; recorded digest did not reproduce from a different
   checkout). **FIXED**: artifacts.rs now hashes manifest-relative paths +
   contents; artifacts regenerated and determinism re-verified.

## Notes recorded (non-blocking, left standing)

- `table:inline_inside_table` labels the cell grade "strict" when the
  contained inline fact is G1-declared — defensible (strictness belongs to
  the qualified table), noted as a grade-conflation.
- Joint-cell winners are tabulated only in the selection trace, not in
  coverage-v1.json (the report now points at the trace).
- The code's seventh-document syntax trigger is broader than the contract
  wording ("any uncovered syntax cell" vs "required syntax cell"); no
  effect in the executed run (the domain trigger fired first).
- `derive_bins` can retain an unattainable MEDIUM label in degenerate
  distributions (harmless: cells are built from non-empty member sets).
- `selection-config-v1.json` is a regenerated echo of code-frozen rules,
  not a consumed input.
- "Selection membership unchanged by the fence fix" is an observation over
  this checkout's runs (pre-fix artifacts were overwritten).

## Post-review state

All MINORs fixed; artifacts regenerated with the checkout-stable tool
identity; determinism re-verified; `cargo test --workspace` green. The
stage verdict `CORRECTIVE_B_PROFILE_SELECT_PASS` is recorded with this
review attached.
