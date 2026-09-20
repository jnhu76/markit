# SEMANTIC_PILOT_SET — contract-validation pilot (CORRECTIVE-A)

Status: **CORRECTIVE-A SEMANTIC SUBSTRATE — PILOT ONLY, NO FREEZE GRANTED**
Authority: issue #35 Corrective-1 §11 (`CORRECTIVE-A: semantic substrate`) +
`protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md`. This directory owns the
pilot fixtures and their two generated artifacts.

```text
SEMANTIC_PILOT_SET is a contract-validation pilot, not a workload: it is not
the representative set, the extremal set, the syntax-coverage set or the
full-document set, and no representativeness claim may be derived from it.
```

That sentence is carried verbatim in `semantic-pilot-manifest-v1.json` so it
travels with the data.

---

## 1. What the pilot is for

The pilot answers one question: **do the frozen contracts actually hold on
real bytes, including the bytes that are designed to break naive readings?**
It is deliberately small, hand-authored, and adversarial. It is not a
sample, and it does not select anything.

```text
fixtures          8 hand-authored cases, expectations written by hand
real source       1 complete real document, sha256-pinned, used byte-for-byte
outputs           manifest (inputs + expectations) and results (observations)
verdict           PASS when every fixture check and the real-source check hold
```

## 2. Fixture set

```text
P01  transition   G0  ATX heading -> paragraph (BREAK/RESTORE)
                  family ATX_HEADING_TOGGLE, exact restore, single-anchor
                  position dedup (EARLY/MIDDLE/LATE -> one anchor)
P02  transition   G1  table delimiter break/restore
                  family TABLE_DELIMITER_EDIT: Table -> non-Table proven by
                  the lane oracle, then restored to the exact original bytes
P03  transition   G1  content-only control + chained trace semantics
                  family E1_LOCAL_TEXT: topology_equal + text_content_differs,
                  three-step local_burst trace in per-step coordinates
P04  transition   G0  post-ineligibility: the edit exposes host syntax
                  outside G0 scope, so the post state is not strict-eligible
P05  profiler     G1  host-context exclusion: table- and math-looking bytes
                  inside a fence and a code span are literal content
P06  profiler     G0+G1  known lane divergence (setext heading) + partial
                  position dedup (EARLY -> anchor 0, MIDDLE/LATE -> anchor 1)
P07  profiler     G0+G1  UTF-8/CJK/emoji byte spans in heading, emphasis,
                  code span, and a table cell
P08  profiler     G0+G1  math candidates: ambiguous (complete, deferred lane)
                  and unknown (unterminated)
```

Fixture file format (`semantic-pilot-fixture-v1`, TOML) declares the source,
its sha256, the lanes, the expected facts with exact byte spans, the expected
structural numbers, the transition predicates, and — where applicable —
position and trace expectations. Expectations are hand-authored authority;
the pilot compares observations against them and never regenerates them.

## 3. Real source

```text
source_id            oci-runtime
path                 workloads/sources/oci-runtime/files/config.md
source_sha256        3d99d67f4e1a32c2581aee44cf3cbee51f5b335fa74af191d1f9df495f92ee0b
acquisition_commit   6999a89a76a0329f440d5740497bedb9dd431297
lanes                G0, G1
```

It is a complete real technical document from the PR #34 acquisition
universe (VENDORED, ~57 KiB) that exercises the pilot's qualified G1
construct (three real GFM tables) together with fenced code, inline raw HTML
anchors, and reference-free inline links. It is used byte-for-byte, exactly
as materialized: no trimming, no newline normalization, no rewriting.

Its purpose is to show that the lane contracts survive a document nobody
authored for them — including the honest result that the same file is
**G0-strict-ineligible** (`strict_scope_clean = false`, out-of-lane
constructs present) while being **G1 scope-clean** with three recognized
tables. That divergence is the point: one file, two lanes, two different
eligibility answers, both recorded.

What the pilot checks for this source is its **identity** (bytes present,
declared sha256, valid UTF-8). The lane observations are recorded, not
asserted: `real-source-v1.json` declares no expectations, so changing a lane
rule would change the recorded numbers without failing the pilot. Asserting
a real file's construct counts belongs to the selection stage, which owns
the question of which files are representative — this corrective deliberately
does not answer it, and the artifact says `verified` rather than implying
the numbers were checked against a contract.

It is a pilot fixture. It is not a member of any workload set, and nothing
about it may be generalized into a selection claim.

## 4. Real-source bytes are PR #34 material (abstention rule)

The materialized snapshot bytes are acquisition material owned by PR #34:
they are rebuilt locally by PR #34's acquisition tooling
(`python3 workloads/tools/acquire.py materialize`, which lives on PR #34's
branch rather than on this one) and are never tracked in Git. The pilot therefore treats them as follows:

```text
bytes present + hash matches declared   -> PASS   (gating check)
bytes present + hash differs            -> FAIL   (gating check, fail closed)
bytes absent                            -> ABSTAIN (non-gating, recorded)
```

`ABSTAIN` is not a failure and not a pass: the observation is recorded, the
declared identity stays on record, and the pilot verdict depends only on
gating checks. The lane profiles for the real source are produced only when
the bytes are available; `real_source.profiles` is empty otherwise, and the
results artifact says so explicitly rather than pretending the source was
profiled.

Because `real_source.pass` is a gating verdict, it must never be read as
"the real source was checked". The artifact therefore names the state:
`real_source.status` is `verified` when the bytes were present and every
gating check ran, and `abstained_absent_bytes` when nothing about the source
was evaluated. The per-check `gating` flags carry the same information at
finer grain (`gating = false` marks an abstained check).

This is what makes the pilot reproducible in a fresh clone of a branch that
does not carry PR #34's materialization, while still failing closed the
moment a present source contradicts its pinned identity.

## 5. Running the pilot

```text
cargo run -p markit-mdbench-semantics --bin mdbench-semantic-pilot
        runs the pilot, prints per-fixture verdicts, exits non-zero on FAIL
cargo run -p markit-mdbench-semantics --bin mdbench-semantic-pilot -- --write
        additionally rewrites the two artifacts in this directory
cargo run -p markit-mdbench-semantics --bin mdbench-gen-semantic-schema
        rewrites grammar/grammar-lanes-v1.json and the three JSON Schemas
```

Both outputs are byte-reproducible: canonical JSON (sorted keys, no
whitespace), no timestamps, no machine facts, no paths outside the
repository-relative source path. `semantics/tests/contracts.rs` fails if a
committed artifact no longer matches what the models generate.

## 6. What the pilot deliberately does not do

```text
- no H0-H4 mechanism is loaded, imported, or timed;
- no file selection, no coverage-set membership, no representativeness claim;
- no semantic-interference metric (a later attribution contract, see
  workloads/attribution/SEMANTIC-INTERFERENCE-CONTRACT-v1.md);
- no render-ready, whole-project, or long-session measurement;
- no claim that G1 is CommonMark/GFM complete: only the table construct is
  semantically qualified in this corrective.
```

The results artifact carries the same list under `deferred`, so a reader of
the data cannot mistake the pilot's scope for the campaign's.
