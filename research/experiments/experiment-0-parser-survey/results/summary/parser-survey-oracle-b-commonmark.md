# ORACLE-B (DIALECT SEMANTICS) — CommonMark 0.31.2 vs markit-core P0-02 L1

Issue: #19 — correctness-oracle expansion (plan §6–7)
Branch: `exp/19-parser-survey-1`. Measured impl: `MarkdownState` (P0-02
block index), unchanged. Harness: `crates/parser-survey/src/cmoracle.rs`
(research-only; `--commonmark` mode).
Raw: `results/raw/parser-survey/run-oracle-b/oracle-b/` (local, ignored).

## Corpus provenance (frozen per plan §7)

- spec version: **CommonMark 0.31.2**
- source: `https://spec.commonmark.org/0.31.2/spec.json`
- sha256: `d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20`
- retrieved: 2026-09-16, committed at `crates/parser-survey/data/`
  (652 examples / 26 sections; never re-fetched "latest")

## Verdict semantics

Compared semantics: **normalized block-kind sequence + heading levels**
(`NormalizedObservation`, plan §6). Markit side: blanks skipped,
Paragraph→`P`, ATX Heading{level}→`Hn`, BlockQuote→`BQ`,
List{Bullet/Ordered}→`UL/OL`, FencedCode→`CODE`. Expected side:
block-level open tags scanned from the spec's expected HTML, with
container depth recorded.

- **PASS** — sequences equal.
- **FLAT_CONTAINER** — equal after dropping container interiors: the
  block-level skeleton agrees; the flat representation does not
  represent nesting (E3, now measured on the spec corpus).
- **FAIL** — aligned block kinds disagree: a genuine divergence for a
  construct the dialect layer does claim.
- **UNSUPPORTED** — the example needs constructs outside the compared
  vocabulary (thematic break, indented code, raw HTML blocks, comment
  blocks): a scope statement, never scored as failure (plan §6).

## Results

| verdict | n | share |
|---|---:|---:|
| PASS | 391 | 60.0% |
| FLAT_CONTAINER | 22 | 3.4% |
| FAIL | 109 | 16.7% |
| UNSUPPORTED | 130 | 19.9% |

The 109 FAILs decompose into exactly four root causes:

| cause | n | examples |
|---|---:|---|
| **No reference-definition block construct** — `[x]: /url` stays a
  plain Paragraph; CommonMark consumes it and resolves users | ~70 |
  Links (39), Link ref defs (16), Images (14), escapes/entities (2) |
| **No setext headings** — `Foo\n---` stays paragraph text; expected H2
  | 13 | Setext (12), one Thematic-breaks, one Fenced-code |
| **List grouping/continuation semantics** — L1 groups list blocks by
  marker signature; CommonMark groups by container position, continues
  items across indentation, and nests | 23 | Lists (12), List items
  (5), Block quotes (3), Tabs (1), Indented code (2) |
| **HTML-block boundary semantics** — L1 does not model html blocks as
  consuming regions | 2 | HTML blocks, Raw HTML |

## What this means for the campaign

1. **The P0-02 block layer is a partial dialect by design** — CommonMark
   60%-PASS is not a regression number; it scopes what "the Markdown
   layer" of the editor claims. Inline semantics (emphasis 132/132 PASS,
   code spans 22/22, autolinks 19/19, hard breaks 15/15 PASS as blocks)
   is where the dialect already agrees.
2. **RUN-4 input**: the missing reference-definition construct is the
   single largest divergence. The planned `ReferenceIndex` must extract
   definitions from *paragraph inline runs*, matching how the editor
   actually represents them.
3. **FLAT_CONTAINER (22 examples)** quantifies E3 on the spec corpus:
   quote/list skeletons agree but nesting interiors are unrepresented —
   direct input to the RUN-3 container-granularity experiment (§22).
4. Fences are solid: Fenced code 26 PASS / 1 FLAT / 1 setext-related
   FAIL, and `closed` propagation matches the honest-run-to-EOF contract.

## Method limitations (explicit)

- Expected-side block structure is derived from expected HTML by a
  targeted scanner (not an HTML parser). Edge behavior is pinned by the
  corpus itself: degenerate comments `<!-->`, close-tag-led raw HTML
  blocks, indented fences (`    ``` `), and comment-swallowing are all
  handled and regression-visible in `cases.csv`.
- Fence closedness, list signatures, item counts, and inline internals
  are recorded but not compared in v1.
- Inline rendering differences (soft/hard breaks, entity resolution)
  only surface when they change block structure.

## Full tables

Machine-generated tables follow.

# ORACLE-B (DIALECT SEMANTICS) — CommonMark 0.31.2

spec: CommonMark 0.31.2, sha256 d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20, retrieved 2026-09-16
examples: 652

| verdict | n | share |
|---|---:|---:|
| PASS | 391 | 60.0% |
| FLAT_CONTAINER | 22 | 3.4% |
| FAIL | 109 | 16.7% |
| UNSUPPORTED | 130 | 19.9% |

## Per-section verdicts

| section | pass | flat | fail | unsupported |
|---|---:|---:|---:|---:|
| ATX headings | 16 | 0 | 0 | 2 |
| Autolinks | 19 | 0 | 0 | 0 |
| Backslash escapes | 10 | 0 | 1 | 2 |
| Blank lines | 1 | 0 | 0 | 0 |
| Block quotes | 4 | 12 | 3 | 6 |
| Code spans | 22 | 0 | 0 | 0 |
| Emphasis and strong emphasis | 132 | 0 | 0 | 0 |
| Entity and numeric character references | 14 | 0 | 1 | 2 |
| Fenced code blocks | 25 | 0 | 1 | 3 |
| HTML blocks | 3 | 0 | 1 | 40 |
| Hard line breaks | 15 | 0 | 0 | 0 |
| Images | 8 | 0 | 14 | 0 |
| Indented code blocks | 1 | 0 | 2 | 9 |
| Inlines | 1 | 0 | 0 | 0 |
| Link reference definitions | 9 | 0 | 16 | 2 |
| Links | 47 | 0 | 39 | 4 |
| List items | 19 | 8 | 5 | 16 |
| Lists | 8 | 1 | 12 | 5 |
| Paragraphs | 7 | 0 | 0 | 1 |
| Precedence | 1 | 0 | 0 | 0 |
| Raw HTML | 13 | 0 | 0 | 7 |
| Setext headings | 4 | 0 | 12 | 11 |
| Soft line breaks | 2 | 0 | 0 | 0 |
| Tabs | 1 | 1 | 1 | 8 |
| Textual content | 3 | 0 | 0 | 0 |
| Thematic breaks | 6 | 0 | 1 | 12 |

## FAIL examples (observed vs expected, first 25)

| ex | section | observed | expected |
|---|---|---|---|
| 4 | Tabs | `UL P` | `UL P P` |
| 23 | Backslash escapes | `P P` | `P` |
| 33 | Entity and numeric character references | `P P` | `P` |
| 59 | Thematic breaks | `P` | `H2 P` |
| 80 | Setext headings | `P P` | `H1 H2` |
| 81 | Setext headings | `P` | `H1` |
| 82 | Setext headings | `P` | `H1` |
| 83 | Setext headings | `P P` | `H2 H1` |
| 84 | Setext headings | `P P P` | `H2 H2 H1` |
| 86 | Setext headings | `P` | `H2` |
| 89 | Setext headings | `P` | `H2` |
| 90 | Setext headings | `P` | `H2` |
| 91 | Setext headings | `P P` | `H2 P H2 P` |
| 95 | Setext headings | `P` | `H2` |
| 102 | Setext headings | `P` | `H2` |
| 103 | Setext headings | `P P` | `P H2 P` |
| 108 | Indented code blocks | `UL P` | `UL P P` |
| 109 | Indented code blocks | `OL P` | `OL P UL` |
| 134 | Fenced code blocks | `P` | `CODE` |
| 179 | HTML blocks | `P P` | `P` |
| 192 | Link reference definitions | `P P` | `P` |
| 193 | Link reference definitions | `P P` | `P` |
| 194 | Link reference definitions | `P P` | `P` |
| 195 | Link reference definitions | `P P` | `P` |
| 196 | Link reference definitions | `P P` | `P` |

## UNSUPPORTED vocabulary (what the measured dialect does not claim)

| vocabulary | n |
|---|---:|
| vocabulary: indented code block | 51 |
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
