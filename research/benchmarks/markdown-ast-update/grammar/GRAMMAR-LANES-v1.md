# GRAMMAR-LANES-v1 — frozen grammar-lane registry (CORRECTIVE-A)

Status: **CORRECTIVE-A SEMANTIC SUBSTRATE — PRE-MEASUREMENT, NO FREEZE GRANTED**
Authority: issue #35 Corrective-1 §2 (grammar lanes must be explicit before
selection) + #35 Workload Construction Algorithm v1 §3 + `protocol/
R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` §3. This file owns the lane
identities, their exact semantic authority, and the rules that make a fact
count as lane coverage. `grammar/BENCH-GRAMMAR-v1.md` remains the single
owner of G0 semantics; `grammar/NORMALIZED-RESULT-v1.md` owns the G0 result
vocabulary. The machine-readable form of this registry is the generated
artifact `grammar/grammar-lanes-v1.json`.

---

## 0. Why lanes exist

A real Markdown file parsed by a simplified total grammar is not a
real-Markdown semantic benchmark. Three questions must never be answered by
one ambiguous word "Markdown":

```text
1. which language is the file being read in?
2. can the claimed lane actually interpret every host-syntax byte in it?
3. which H0-H4 horses are correct enough to be timed on that lane?
```

Questions 1-2 are the **semantic** lane contract (§1-§5). Question 3 is the
**implementation** contract (§6). They are independent gates: a lane may be
semantically frozen for profiling/realism long before any horse implements
it, and a horse may be correct on a lane whose semantic evidence is still
`declared_not_qualified`.

```text
semantic qualification     != horse implementation qualification
profiling / realism use    != strict H0-H4 comparison evidence
```

Only a fact whose `lane_scope_grade` is `StrictLaneCoverage` may be counted
as strict lane coverage or as strict H0-H4 comparison evidence
(`workloads/profiles/PROFILER-CONTRACT-v1.md` §4).

---

## 1. Lane registry

```text
registry_version  GRAMMAR-LANES-v1
lane_id           G0
  grammar_id      BENCH-GRAMMAR-v1
  lane_version    v1
  semantic_status qualified
  scope           the complete BENCH-GRAMMAR-v1 construct set (12 frozen kinds)
  oracle          BENCH-GRAMMAR-v1-REFERENCE-PARSE-v1 (repository reference
                  implementation: markit-mdbench-shared-grammar::parse_full
                  + markit-mdbench-oracle::validate_normalized; no options)

lane_id           G1
  grammar_id      COMMONMARK-0.31.2+GFM-TABLES-0.29-gfm-v1
  lane_version    v1
  semantic_status qualified — for the TABLE construct only (pilot scope)
  scope           CommonMark 0.31.2 base + GFM tables extension
                  (4 frozen kinds: table, table_header_row, table_row, table_cell;
                   22 declared_not_qualified base kinds;
                    4 out_of_lane disabled extensions;
                    2 deferred math kinds owned by G2)
  oracle          G1-ORACLE-pulldown-cmark-0.13.4
                  (pulldown-cmark =0.13.4, default features off,
                   Options::ENABLE_TABLES only)

lane_id           G2
  grammar_id      MARKIT-EXT-MATH-v1
  lane_version    v1
  semantic_status deferred — identity reserved, semantics NOT frozen
  scope           inline/display dollar math (identity only)
  oracle          none
```

`grammar-lanes-v1.json` carries the exhaustive per-kind `construct_scopes`
table (status + rationale per kind, per lane) and is regenerated from the
same Rust constants the profiler uses, so the doc and the code cannot drift
silently: `cargo run -p markit-mdbench-semantics --bin
mdbench-gen-semantic-schema` rewrites it, and `semantics/tests/contracts.rs`
fails if the committed artifact differs from the models.

## 2. G0 — `BENCH-GRAMMAR-v1`

The controlled synthetic lane of #22/R3. Its semantics are owned by
`grammar/BENCH-GRAMMAR-v1.md` (construct set, deviations D1-D13) and
`grammar/NORMALIZED-RESULT-v1.md` (13 node kinds, span conventions,
`NODE_PATH_AT`). This corrective does not restate, soften, or "fix" any of
it: the CommonMark deviations D1-D13 are authoritative here.

```text
semantic qualification   complete for the BENCH-GRAMMAR-v1 construct set
                         (12 frozen kinds: paragraph, text, heading_atx,
                          block_quote, list, list_item, code_block_fenced,
                          emphasis, code_span, link_inline, link_reference,
                          reference_definition)
strict_scope_clean       no host-context candidate whose kind is outside
                         that set (thematic break, strong, image, tables,
                          HTML, math, setext headings, indented code,
                          autolinks, strikethrough, task lists, front matter,
                          directives)
lane_valid               always true (the grammar is total over UTF-8)
horse qualification      h0-h4 qualified (protocol/R5-HORSE-CORRECTNESS-PARITY.md,
                         protocol/R5-HORSES-STAGE-RECORD.md)
```

That enumeration is illustrative; `grammar/grammar-lanes-v1.json` carries the
exhaustive per-kind `construct_scopes` table and is the authoritative list.

`code_block_fenced` is in G0's frozen set, so a tilde fence is **not** a
scope blocker: G0's grammar defines backtick fences, and its reading of
`~~~` bytes is ordinary paragraph text. The bytes are still recorded — the
tilde-fence probe reports a `code_block_fenced` candidate that the lane
oracle declined, with `candidate_rejected_by_lane_rule` — so the divergence
is visible in the artifact even though it does not make the file
ineligible. A file that leans on tilde fences is a poor G0 coverage subject
for reasons the profile shows, not for reasons `strict_scope_clean`
encodes.

Consequence for real files: a real file that contains host-context thematic
breaks, strong emphasis, images, tables, HTML, math, setext headings,
indented code, autolinks, strikethrough, task lists, front matter, or
directives is **not** G0-strict. Its G0 profile records those bytes as
`out_of_lane` candidates with `strict_scope_clean = false`; the file remains
usable for realism/parse coverage only, and must never be counted as G0
strict coverage.

## 3. G1 — `COMMONMARK-0.31.2+GFM-TABLES-0.29-gfm-v1`

The real-Markdown lane. Identity is frozen to exact specification versions
rather than to a family name:

```text
base dialect      CommonMark 0.31.2, published 2024-01-28
                  https://spec.commonmark.org/0.31.2/
extension         GFM 0.29-gfm, published 2019-04-06, §4.10 Tables ONLY
                  https://github.github.com/gfm/
precedence        the base dialect is CommonMark; the GFM text contributes
                  ONLY the table construct, so an extension/base conflict
                  cannot silently redefine the base
oracle            pulldown-cmark 0.13.4, default features off,
                  Options::ENABLE_TABLES only
```

The oracle is a **pinned independent implementation**, not the semantic
authority: the spec texts are the authority, the pilot fixtures declare the
expected interpretation independently, and the oracle is what makes the
declared interpretation executable. Both are checked
(`workloads/pilots/README.md` §3).

### 3.1 Enabled / disabled

```text
ENABLED     CommonMark 0.31.2 base constructs
            GFM §4.10 tables
DISABLED    GFM strikethrough, GFM task list items, GFM autolink literals,
            GFM tagfilter, footnotes, smart punctuation, YAML/+++ metadata,
            heading attributes, definition lists, super/subscript, wikilinks,
            MyST directives and other project-specific extensions,
            math (owned by G2)
```

### 3.2 What is actually qualified in this corrective

```text
TABLE                    SEMANTIC_QUALIFIED (frozen semantics + oracle
                         configuration + normalized table vocabulary +
                         pre/post transition predicates)
                         -> the only kind that may be counted as strict G1
                            coverage in CORRECTIVE-A
BASE CONSTRUCTS          SEMANTIC_CONTRACT_FROZEN / EVIDENCE_NOT_QUALIFIED:
                         the CommonMark 0.31.2 spec text is the frozen
                         authority, but this corrective does not qualify
                         oracle/coverage evidence for them beyond the pilot.
                         Their facts carry `contract_declared_not_qualified`
                         and must not be counted as strict lane coverage.
MATH                     owned by G2; under G1, `$...$` / `$$...$$` bytes are
                         reported `ambiguous` (complete candidate) or
                         `unknown` (incomplete/unterminated candidate) with
                         grade `lane_deferred` — never silently classified as
                         text, never counted as coverage.
```

This is exactly the `IMPLEMENTATION_NOT_QUALIFIED` outcome the corrective
allows: a lane may be semantically specified while its horses are not
qualified. It does **not** permit rewriting real files to make a lane pass.

### 3.3 Precedence rules (frozen)

```text
- the base dialect is CommonMark 0.31.2; GFM contributes only §4.10 tables
- table recognition needs a header row immediately followed by a delimiter
  row whose cell count matches (GFM §4.10)
- a table cannot interrupt a paragraph (GFM §4.10)
- pipes inside code spans do not split cells (GFM §4.10)
- syntax bytes inside fences / code spans / raw HTML are literal content,
  never host constructs
```

The last rule is the **host-context rule**. It is a lane-semantic rule, not
a heuristic: a table-shaped line inside a fence is body text, and a `$` inside
a code span is not math. `workloads/profiles/PROFILER-CONTRACT-v1.md` §5
defines exactly how facts record it.

## 4. G2 — `MARKIT-EXT-MATH-v1` (identity reserved, semantics deferred)

No semantics, no normalized vocabulary, no oracle, no eligibility rule
exist for G2 in this corrective, and none are invented here. G2 exists in
the registry so that:

```text
- math-looking bytes are attributed to a named, reserved owner instead of
  being silently folded into "G1 text" or "G0 text";
- downstream selection can report "this file needs G2" as a fact rather
  than as an absence;
- freezing G2 later is an additive versioned event, not a re-interpretation
  of already-published facts.
```

`math_occupancy` is therefore `null` in every current profile, with an
explicit `math_occupancy_note` saying why (see the profiler contract §3.3).
#35 §4.2 requires `MATH_INLINE` / `MATH_DISPLAY` as real-world syntax
targets; in CORRECTIVE-A they are **candidate** targets only, and no payload
may claim them as coverage.

## 5. Eligibility: `lane_valid` and `strict_scope_clean`

G0 and G1 are total grammars: every UTF-8 byte sequence has a defined
interpretation under them, so for those two lanes `lane_valid` is true for
any valid UTF-8 input and is **not** the interesting property. What decides
whether a file/edit may enter strict comparison is scope cleanliness:

```text
lane_valid(lane, source) =
    the lane has frozen semantics and an oracle, and therefore interprets
    the source at all
strict_scope_clean(lane, source) =
    lane_valid AND no host-context candidate fact whose lane_scope_grade is
    out_of_lane_candidate or lane_deferred
```

`lane_valid` is defined this way rather than as "the grammar is total" so
that a lane with no semantics cannot be read as an eligible one: G2 has no
frozen semantics and no oracle, so every G2 profile reports
`lane_valid = false` and `strict_scope_clean = false` together with a warning
naming the missing semantics. The zero-filled structural record of such a
lane is labelled `not implemented` instead of presenting zeros as
measurements.

Rules:

- candidates inside fences / code spans / raw regions never block
  cleanliness (they are not host syntax);
- a construct the lane *declares but does not qualify* (G1 base constructs)
  does not block cleanliness either — it is reported with
  `contract_declared_not_qualified` so that coverage is not overclaimed;
- eligibility is evaluated **independently for the pre source and the post
  source**. An edit that exposes host syntax outside the lane makes the post
  state ineligible; that is recorded per source and never inherited from the
  pre state (`grammar/TRANSITION-ORACLE-v1.md` §4).

## 6. Horse implementation qualification (separate gate)

```text
G0   h0-full-rebuild, h1-block-local-reparse, h2-fragment-reuse,
     h3-old-tree-subtree-reuse, h4-restart-convergence  -> qualified
G1   all five horses                                  -> not_implemented
G2   all five horses                                  -> not_applicable
```

A G1 fact can be profiled, reviewed, and used for realism/syntax-coverage
decisions today. It may **not** be used for H0-H4 timing, ranking, or
attribution until the horses are implemented and pass the lane oracle. No
H0-H4 performance measurement is authorized in CORRECTIVE-A at all.

## 7. Versioning rules

```text
- lane_version bumps when a lane's construct scopes, precedence, or oracle
  configuration change;
- registry_version (GRAMMAR-LANES-v1) bumps when the registry record shape
  or the qualification rule changes;
- grammar_id is the identity consumed by payloads, profiles, and predicates;
  renaming one is an identity break, not a documentation edit;
- every change regenerates grammar-lanes-v1.json in the same commit, and the
  drift guard in semantics/tests/contracts.rs must pass.
```

## 8. Validation coverage required of this registry

The lane registry is only trustworthy if its claims are checkable against
bytes. The pilot fixtures in `workloads/pilots/fixtures/` must cover, for
the lanes they claim:

```text
ordinary construct under its own lane                P01, P02
construct-shaped bytes inside a fence / code span    P05
ambiguous dialect case (two declared lanes differ)   P08
known G0-vs-G1 divergence                            P06 (setext)
UTF-8 / CJK / emoji spans                            P07
post-state ineligibility caused by the edit          P04
```

`semantics/tests/profiler.rs` additionally pins the host-context rule, the
ambiguous/unknown policy, span evidence, and the probe/oracle disagreement
path; `semantics/tests/transitions.rs` pins the lane-bound predicate rule
and the unknown-lane rejection.
