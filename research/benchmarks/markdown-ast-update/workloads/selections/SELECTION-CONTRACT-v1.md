# SELECTION-CONTRACT-v1 — frozen deterministic workload selection (CORRECTIVE-B)

Status: **CORRECTIVE-B PROFILE + SELECT — NO FREEZE GRANTED**.
Executable form: `profile-select/src/selection.rs` (crate
`markit-mdbench-profile-select`). Configuration artifact:
`selections/selection-config-v1.json`. This document is the frozen rule
statement; where prose and code could disagree, the code plus its tests
are authoritative and this file must be updated in the same commit.

Inputs the selectors may consume (§40 performance-blind guard):

```text
PR #34 acquisition manifests + materialized source bytes
CORRECTIVE-A profiler outputs (REAL-MARKDOWN-PROFILER-v1)
DOMAIN-STRATA-v1 committed mapping
this contract + selection-config-v1.json
```

No `mechanisms/*`, no runner/timing fields, no allocation results, no
restart/convergence, no reuse counters, no horse identity. The crate's
dependency graph is the structural guard; a test re-asserts it.

## 1. Feature list (§13)

All structural features are G0-scoped (`block_count` without a lane is
not a fact):

```text
file_bytes  block_count  largest_block_bytes  max_container_depth
fence_density_per_kib  code_occupancy  reference_density_per_kib
cjk_byte_share
```

`math_occupancy` is NOT a feature: G2 is deferred, so it is `null` in
every lane and no fabricated structural measurement exists.

## 2. Binning rule (§23)

```text
if zero is semantically meaningful (all features above except file_bytes):
    ZERO = exactly 0
for positive values:
    LOW    = <= positive-population Q33.333
    MEDIUM = > Q33.333 and <= Q66.667
    HIGH   = > Q66.667
```

Quantiles are computed over the **G0-strict-eligible** population. When
quantile boundaries collapse because many values are equal, empty or
identical bins collapse to the actually attainable set; values are never
jittered. All actual thresholds are recorded in
`selections/selected-files-v1.json` (`bins`).

## 3. Joint-coverage cells (§24)

Mandatory (predeclared):

```text
file_bytes × block_count
file_bytes × largest_block_bytes
fence_density_per_kib × code_occupancy
file_bytes × reference_density_per_kib
```

Optional fifth `max_container_depth × block_count`, included only under
the frozen variation rule: at least 3 distinct observed depth values
among eligible candidates AND p90(depth) >= 2. The executed decision is
recorded in `selection-config-v1.json`. No combination may be added after
seeing H0-H4 results (there are none in B).

## 4. REPRESENTATIVE_SET (§25)

Eligible input: `G0 strict_scope_clean == true` with verified
materialization and a successful profile. Hierarchical diversity caps:

```text
per-project representative cap = 3
no domain stratum > 35% of the set (ceil-guarded, minimum 1)
```

Caps may be relaxed only mechanically: a cap-violating candidate is
admitted only when no cap-compliant candidate covers any remaining
mandatory cell, and every relaxation is logged in the trace. No
relaxation occurred in the executed run.

Greedy iteration ordering (§25.2), all ties broken deterministically:

```text
1. largest number of newly covered mandatory cells (+1 for a new domain stratum)
2. candidate from the currently least-represented domain
3. candidate from the currently least-represented project
4. fewer potential-near-duplicate conflicts with selected files
5. lexical (source_id, snapshot_path)
```

Stop rule (§25.3): all attainable mandatory cells covered, or no
remaining candidate adds a new cell. Counts 20-30 are guidance, not
quotas: a plateau below 20 reports `UNDER_TARGET_DUE_TO_COVERAGE_PLATEAU`
(no padding), an overrun above 30 is allowed with per-file unique-cell
evidence in the trace.

## 5. EXTREMAL_SET (§27)

For each of the 8 features, over materialized, hash-verified, profiled
candidates: `OBSERVED_MAXIMUM` (ties broken lexically) plus one
`TAIL_REPLICATE` — a distinct project within the top 1% (min 2) of the
same dimension, lexically first. `NONE_AVAILABLE` is recorded when no
replicate exists; none is manufactured. Files deduplicate physically
across roles via `extreme_roles`.

## 6. SYNTAX_COVERAGE_SET (§28-§31)

Cells and evidence grades are frozen in code (`SYNTAX_CELLS`, 33 cells):
10 core G0-strict cells, 10 required real-world syntax cells, 6 table
sub-cells, 7 observed extras. Only `strict` evidence may be counted as
strict coverage; `declared` (G1 CommonMark base kinds), `candidate`
(out-of-lane), and `ambiguous_or_unknown` (math, G2 deferred) evidence
retains its grade. Seeded from REPRESENTATIVE ∪ EXTREMAL ∪
FULL_DOCUMENT coverage; greedy-add by (§31):

```text
1. stronger evidence grade   2. more new cells   3. new project
4. new domain                5. lexical
```

Files whose selection value is only non-strict evidence carry
`REALISM_ONLY` plus `LANE_DEFERRED` (math) and/or
`GRAMMAR_EXTENSION_REQUIRED` (out-of-lane extensions).

Table thresholds (§29): wide = max_table_columns >= 8; long =
max_table_rows_including_header >= 20; alignment marker = any non-None
G1 oracle alignment; escaped pipe = a table probe candidate rejected by
the frozen lane rule; inline-inside-table = a recognized host-context
inline fact contained in a recognized table span.

## 7. FULL_DOCUMENT_SET (§33-§35)

Named hard candidates (validated, never hand-replaced):

```text
FULL-CPP-CORE          cpp-core-guidelines CppCoreGuidelines.md
FULL-NODE-FS           node doc/api/fs.md
FULL-D2L-LINEAR-ALGEBRA d2l-en chapter_preliminaries/linear-algebra.md
```

Mechanical ranks (equal-weight within-source percentile ranks, published
per component; lexical tie-break):

```text
OpenMLSys: file_bytes, cjk_byte_share, fence_density_per_kib,
           math-candidate occurrences (G2 deferred: occurrence/file
           presence evidence only — no fabricated occupancy)
KEP:       file_bytes, block_count, max_container_depth, fence_density_per_kib
Rust RFC:  file_bytes, block_count, reference_density_per_kib
```

Optional seventh document only when a domain stratum or required syntax
cell remains uncovered by the first six; the deterministic choice is the
largest-bytes file covering the trigger. Otherwise stop at six and
record why.

## 8. Cross-set dedup (§36)

Four logical sets; physical files stored once with `memberships` +
`extreme_roles`. `selected-files-v1.json` reports logical counts, unique
physical count, and pairwise overlap.

## 9. Trace (§26)

Every iteration records: iteration, selection, new cells, duplicated
cells, domain/project counts before/after, near-duplicate flags,
tie-break values, remaining uncovered cells, cap relaxations, and a
machine-derived `why` string. Prose like "looks representative" is never
selection authority.

## 10. Stop-before rules (§46-§47)

No final edit bytes: no `edit_start`/`edit_end`/`inserted bytes`/
`post_source_sha256`/`payload_id`/`CaseId` are emitted. Transition
taxonomy stays deferred to CORRECTIVE-C; B records only boolean
transition opportunities (`g0_atx_heading_to_paragraph`,
`g1_table_delimiter_transition`).
