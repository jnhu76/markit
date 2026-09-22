# Result Row Schema (v2) — MARKIT-MARKDOWN-BENCHMARK-1

Status: **R1 MATERIALIZED / SCHEMA v2 (MEASUREMENT-CORRECTIVE-1)**
Source of truth: `runner/src/result.rs` (the row type keeps its historical
`ResultRowV1` name; it stamps `RESULT_SCHEMA_VERSION_V2 = 2`).
Generated artifact: `protocol/result-schema-v2.json` (JSON Schema 2020-12,
produced by `cargo run -p markit-mdbench-runner --bin mdbench-gen-schema --
protocol/result-schema-v2.json`). A test in the runner fails if the
checked-in JSON drifts from the Rust model — there are not two
independently maintained authorities. v1
(`protocol/result-schema-v1.json`) is retired and removed; v1 payloads do
not deserialize as v2 (the version field is checked).

Rows are emitted as JSONL (one JSON object per line).

## Identity and versioning

| field              | value / meaning                                        |
|--------------------|--------------------------------------------------------|
| `schema_version`   | `2` (`RESULT_SCHEMA_VERSION_V2`; the Rust row struct keeps its historical `ResultRowV1` name) |
| `protocol_version` | `"R0-FROZEN-V1"` — points at `protocol/R0-METHODOLOGY.md` |
| `case_id`          | hex `SHA256(canonical_encode(CaseKeyV1))`; algorithm id `sha256-of-casekey-v1`; contains no mechanism/lane/run-order facts |
| `seed`             | the recorded case-order seed                           |
| `mechanism_id`     | e.g. `"__r1_null__"` (R1-only)                          |

## Build identity

`build_identity` (captured at compile time by the runner build script):
`runner_git_commit`, `rustc`, `target`, `build_profile_id`
(`release-primary-v1` for release builds; `debug-non-research` for
development/test builds, which are never research measurements),
`cargo_lock_sha256`.

## Payload and edit

- `payload`: `payload_id`, `shape` (R0 §9 snake_case names), `size_bytes`.
- `edit`: `start_byte`, `end_byte`, `inserted_sha256` (hex SHA256 of the
  exact inserted UTF-8 bytes). Built by the validated
  `edit_meta(operation, edit)` constructor, which uses the SAME operation
  contract as `CaseKeyV1` and rejects contradictory combinations:

```text
FULL_PARSE / QUERY:  all null
DELETE:              range present, inserted_sha256 = null
INSERT / REPLACE_EQ / REPLACE_GROW / REPLACE_SHRINK / STRUCTURAL_EDIT:
                     range present, inserted_sha256 present
                     (empty insertions hash the empty string)
```

## Statuses (kept orthogonal)

- `execution_status`: `pass` | `unsupported` | `timeout` | `oom` |
  `stack_overflow` | `crash` | `instrumentation_unavailable`.
  A panic inside a mechanism phase is recorded as `crash`.
- `correctness_status`: `not_checked` | `pass` | `wrong_result`.
  Failed executions are `not_checked` — never silently `pass`, never
  dropped.
- `result_checksum`: hex of the deterministic scalar checksum of the
  completed result; `null` when the run did not complete. A correctness
  fact, never a performance claim.

## Measurement (one lane per row)

```json
{ "lane": "timing", "metrics": { "prepare_ns": 11, "native_ns": 23, "total_ns": 34 } }
{ "lane": "memory", "metrics": { "allocated_bytes": ..., "allocation_count": ...,
                                 "peak_bytes": ..., "retained_bytes": ... } }
{ "lane": "attribution", "metrics": { <WorkCounters slots> } }
```

- Exactly one lane per row; there is no composite/headline lane. Metric
  structs use `deny_unknown_fields`, so a timing payload carrying
  attribution keys (or vice versa) is schema-invalid and rejected.
- `timing`: integer nanoseconds. `prepare_ns` is `NOT_APPLICABLE` for
  `FULL_PARSE`; `total_ns` is always the arithmetic sum
  (`prepare + native`, or `native` when prepare is `NOT_APPLICABLE`) —
  never an independently measured enclosing interval. Failed runs and
  timing-arithmetic overflow record `"UNKNOWN"` values.
- `memory`: per-case window facts (one `begin_case`/`end_case` window per
  run). `peak_bytes` (maximum live bytes IN the window) and
  `retained_bytes` (bytes still held at window close) are DISTINCT
  metrics and are never conflated into a `peak_retained_bytes` field.
- `attribution` slots (ATTRIBUTION-SCHEMA-v2, MEASUREMENT-CORRECTIVE-1
  §18/§19) are three-valued: `Known(v)` serializes as the number, absent
  instrumentation as `"UNKNOWN"`, structurally inapplicable as
  `"NOT_APPLICABLE"`. `Known(0)` is therefore always distinguishable from
  unknown/inapplicable. The v1 derived slots
  (`unique_source_intervals_inspected` /
  `unique_source_bytes_inspected`, single offset union) are RETIRED and
  do not deserialize as v2. The v2 derived slots are computed by the
  common collector from `record_source_inspection` events, which carry
  explicit OLD/POST source-version provenance; the two coordinate spaces
  are never unioned with each other:

```text
unique_old_source_intervals / unique_old_source_bytes
    distinct byte intervals/bytes of the OLD source inspected
unique_post_source_intervals / unique_post_source_bytes
    distinct byte intervals/bytes of the POST source inspected
unique_source_intervals / unique_source_bytes
    the COMBINED primary unique-coverage quantity: the SUM of the two
    per-version unions (a byte touched in both versions counts once per
    version)
source_bytes_inspected_total
    cumulative inspection EFFORT: every event contributes its full
    length, repeated scans included
```

  Unique source coverage (how much distinct source territory was
  touched) is therefore NOT cumulative inspection effort (how much
  source-reading work actually occurred): repeated scans increase
  `source_bytes_inspected_total` but not the unique slots. The derived
  slots stay `"UNKNOWN"` for runs that did not complete. Parse
  Amplification is derived by the common analysis layer as
  `unique_source_bytes / logical_edited_bytes` (combined per-version
  union sum in the numerator), with repeated effort reported separately
  as `source_bytes_inspected_total / logical_edited_bytes` — never by
  horses, and never conflated.

## Provenance

- `environment_ref`: e.g. `manifest/environment.toml#<environment_id>`.
- `provenance_ref`: provenance tag of the run. Harness validation rows
  MUST carry `R1_SMOKE_ONLY/NON_RESEARCH_RESULT`.

## Banned fields

Raw rows preserve facts only. `winner`, `rank`, `score`, weighted/speedup
or quality-conclusion fields must never be added (a guard test enforces
this at the schema level). Conclusions belong to later report stages.
