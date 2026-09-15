# RUN-2a — M0-B external baseline: MD4C full parse (plan §9)

Issue: #19, branch `exp/19-parser-survey-1`. Research-only adapter over
vendored MD4C **release-0.5.3** (provenance + per-file sha256 in
`crates/parser-survey/vendor/md4c/PROVENANCE.md`), `flags=0` (pure
CommonMark-style dialect, no extensions).
Raw: `results/raw/parser-survey/run-2-baselines/md4c/` (local).

## Size sweep (medians; same machine/build as the run-1.1 control)

| corpus | bytes | markit_build | md4c_count | md4c_norm | norm−count | events | ev/KB | ns/B markit | ns/B md4c |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| synth-1k | 1 153 | 8.0 µs | 3.4 µs | 3.3 µs | −0.1 | 93 | 82.6 | 6.9 | 2.9 |
| synth-10k | 10 295 | 55.1 | 15.0 | 17.7 | 2.7 | 398 | 39.6 | 5.4 | 1.5 |
| synth-100k | 102 493 | 590.1 | 118.9 | 117.5 | −1.4 | 3 433 | 34.3 | 5.8 | 1.2 |
| synth-1m | 1 048 633 | 4 277.5 | 1 182.5 | 1 186.4 | 3.9 | 34 173 | 33.4 | 4.1 | 1.1 |
| synth-10m | 10 485 893 | 60 800.5 | 12 754.3 | 12 517.3 | −237.0 | 336 853 | 32.9 | 5.8 | 1.2 |

Per plan §9 the two parsers' costs are **never merged into one number**:

- `md4c_count` is parser-native cost only (event-counting callbacks);
  **MD4C full parse ≈ 1.1–1.2 ns/B** on this corpus family.
- `md4c_norm − md4c_count` isolates the adapter/normalization cost:
  ≈ 0 µs at every size within noise — building a block-kind sequence
  from MD4C events is effectively free.
- `markit_build` (4.1–5.8 ns/B) includes markit's own block+inline
  normalization, so the honest §34 statement is: *the entire current
  markit full pipeline is ≈ 4–5× the MD4C native event parse on the
  same corpus*, with markit's extra work buying inline IR + fence state
  + tiling block records.

H6 crossover context (run-1): markit's incremental path on a fence
cascade at 1 MB costs ≈ 1.9–2.1 ms — **above MD4C's 1 MB full parse
(1.18 ms)**. A full-parse fallback does not have to be the same parser;
this is the first measured support for a *cheap external full-parse
reference point*.

## CommonMark 0.31.2 cross-check (same ORACLE-B rules as markit)

| verdict | n | share |
|---|---:|---:|
| PASS | 572 | 87.7% |
| FLAT_CONTAINER | 0 | 0.0% |
| FAIL | 1 | 0.2% |
| UNSUPPORTED | 79 | 12.1% |

- **PASS 87.7% vs markit's 60.0%** on the identical corpus and identical
  expected-side scanner — the comparison is scanner-bias-controlled;
  the gap is dialect completeness, not oracle artifact.
- **FLAT_CONTAINER 0**: MD4C is a true container tree — nested
  blockquote/list examples match the full expected sequence, while
  markit's flat L1 matched only the skeleton on 22 examples.
- The single FAIL (ex 169) is an HTML-block boundary edge
  (`<pre>`-led block swallowing the next line) — vocabulary-external in
  our comparison.
- UNSUPPORTED (79) is dominated by HTML blocks (38) and thematic
  breaks (11) — constructs the comparison vocabulary does not map —
  plus 11 setext examples whose expected HTML mixes `<hr>` (the same
  11 are UNSUPPORTED for markit; scanner-limited, consistently for both
  implementations).

## Method notes

- Same synthetic corpus family, same sizes, same medians protocol as
  run-1.1; allocator counting is available in `md4c-sizes.csv` but not
  used for latency claims (allocator-instrumented timing is support
  evidence only).
- MD4C events: ~33/KB steady-state above 100 KB (≈ 1 event per 30 bytes
  of this corpus).
- The ORACLE-B expected-side rules are byte-identical between the two
  implementations' runs (only the indented-code vocabulary flag
  differs, because MD4C claims indented code and markit L1 does not).

## Full machine tables

# M0-B — MD4C full-parse baseline (plan §9)

MD4C release-0.5.3, vendored, flags=0 (no extensions). Medians; same machine/build as the run-1.1 markit control.

| corpus | bytes | markit_build | md4c_count | md4c_norm | norm−count | events | ev/KB | ns/B markit | ns/B md4c |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| synth-1k | 1153 | 8.2 | 3.4 | 3.3 | -0.1 | 93 | 82.6 | 7.1 | 2.9 |
| synth-10k | 10295 | 45.8 | 14.3 | 14.3 | 0.0 | 398 | 39.6 | 4.4 | 1.4 |
| synth-100k | 102493 | 584.7 | 121.7 | 120.3 | -1.4 | 3433 | 34.3 | 5.7 | 1.2 |
| synth-1m | 1048633 | 4803.6 | 1322.3 | 1315.5 | -6.9 | 34173 | 33.4 | 4.6 | 1.3 |
| synth-10m | 10485893 | 62529.1 | 13707.9 | 13464.6 | -243.2 | 336853 | 32.9 | 6.0 | 1.3 |

Column notes: `md4c_count` = parser-native (event counting only); `md4c_norm` = native + normalization into the ORACLE-B vocabulary; the delta is adapter cost. `markit_build` = MarkdownState::build (includes markit's own block/inline normalization — NOT directly comparable to `md4c_count`; the comparison for §34 is per-column, never a single ranking).


---

# ORACLE-B (DIALECT SEMANTICS) — CommonMark 0.31.2

spec: CommonMark 0.31.2, sha256 d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20, retrieved 2026-09-16
examples: 652

| verdict | n | share |
|---|---:|---:|
| PASS | 572 | 87.7% |
| FLAT_CONTAINER | 0 | 0.0% |
| FAIL | 1 | 0.2% |
| UNSUPPORTED | 79 | 12.1% |

## Per-section verdicts

| section | pass | flat | fail | unsupported |
|---|---:|---:|---:|---:|
| ATX headings | 17 | 0 | 0 | 1 |
| Autolinks | 19 | 0 | 0 | 0 |
| Backslash escapes | 12 | 0 | 0 | 1 |
| Blank lines | 1 | 0 | 0 | 0 |
| Block quotes | 23 | 0 | 0 | 2 |
| Code spans | 22 | 0 | 0 | 0 |
| Emphasis and strong emphasis | 132 | 0 | 0 | 0 |
| Entity and numeric character references | 16 | 0 | 0 | 1 |
| Fenced code blocks | 29 | 0 | 0 | 0 |
| HTML blocks | 5 | 0 | 1 | 38 |
| Hard line breaks | 15 | 0 | 0 | 0 |
| Images | 22 | 0 | 0 | 0 |
| Indented code blocks | 11 | 0 | 0 | 1 |
| Inlines | 1 | 0 | 0 | 0 |
| Link reference definitions | 26 | 0 | 0 | 1 |
| Links | 86 | 0 | 0 | 4 |
| List items | 48 | 0 | 0 | 0 |
| Lists | 26 | 0 | 0 | 0 |
| Paragraphs | 8 | 0 | 0 | 0 |
| Precedence | 1 | 0 | 0 | 0 |
| Raw HTML | 13 | 0 | 0 | 7 |
| Setext headings | 16 | 0 | 0 | 11 |
| Soft line breaks | 2 | 0 | 0 | 0 |
| Tabs | 10 | 0 | 0 | 1 |
| Textual content | 3 | 0 | 0 | 0 |
| Thematic breaks | 8 | 0 | 0 | 11 |

## FAIL examples (observed vs expected, first 25)

| ex | section | observed | expected |
|---|---|---|---|
| 169 | HTML blocks | `P` | `CODE P` |

## UNSUPPORTED vocabulary (what the measured dialect does not claim)

| vocabulary | n |
|---|---:|
| vocabulary: hr (thematic break) | 27 |
| vocabulary: <div> | 16 |
| vocabulary: <> | 6 |
| vocabulary: <table> | 5 |
| vocabulary: raw html block (<a>) | 3 |
| vocabulary: <style> | 3 |
| vocabulary: <bar> | 3 |
| vocabulary: raw html block | 2 |
| vocabulary: raw html block (<del>) | 2 |
| vocabulary: <script> | 2 |
| vocabulary: <b2> | 2 |
| vocabulary: <DIV> | 1 |
| vocabulary: <Warning> | 1 |
| vocabulary: <i> | 1 |
| vocabulary: <textarea> | 1 |
| vocabulary: <foo> | 1 |
| vocabulary: <b> | 1 |
| vocabulary: <bab> | 1 |
| vocabulary: <responsive> | 1 |

Verdict semantics: PASS = normalized block sequence equal; FLAT_CONTAINER = equal after dropping container interiors (the measured representation is flat — E3); FAIL = aligned block kinds disagree; UNSUPPORTED = example needs constructs outside the measured vocabulary (scope statement, not a failure). Compared semantics: block-kind sequence + heading levels; fence closedness / list signatures recorded but not compared in v1.

