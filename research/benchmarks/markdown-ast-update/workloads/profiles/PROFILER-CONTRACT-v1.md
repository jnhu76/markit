# PROFILER-CONTRACT-v1 — REAL-MARKDOWN-PROFILER-v1 (CORRECTIVE-A)

Status: **CORRECTIVE-A SEMANTIC SUBSTRATE — PRE-MEASUREMENT, NO FREEZE GRANTED**
Authority: issue #35 Corrective-1 §3 (three fact classes) + #35 Workload
Construction Algorithm v1 §4 + `protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md`
§6. Executable form: `semantics/src/facts.rs`, `semantics/src/profile.rs`,
`semantics/src/probes.rs`; lane semantics in `grammar/GRAMMAR-LANES-v1.md`.
Generated schema: `workloads/profiles/schema/real-profile-v1.schema.json`.

```text
profiler_id/version   REAL-MARKDOWN-PROFILER-v1
profile schema tag    real-profile-v1
unit of output        Profile { source facts, one LaneProfile per lane }
```

---

## 0. One profile is not one number

"Do not emit one ambiguous Markdown profile" (#35 §3). Every profile is
three separable records — plus the eligibility record #35 §3 requires —
and no downstream consumer is allowed to read one as another.

```text
SOURCE_FACT      parser-independent bytes / lines / newline / hash / CJK
SYNTAX_FACT      grammar-scoped construct occurrence: kind, span,
                 recognition status, evidence grade, reason, occurrence
STRUCTURAL_FACT  grammar-scoped counting rules applied to a lane parse
ELIGIBILITY_FACT whether a source is valid AND strictly in scope for a lane
```

Every one of these records carries the `grammar_id` it was computed under
(except SOURCE_FACT, which is grammar-free by definition). There is no
grammar-free structural number anywhere in the contract: `block_count`
without a lane is not a fact.

## 1. SOURCE_FACT (parser-independent)

```text
source_sha256           SHA-256 of the exact source bytes
file_bytes              length in bytes
line_count              0 for the empty source, else
                        lf_count + (0 if the source ends with 0x0A else 1)
lf_count                number of 0x0A bytes (includes the LF of every CRLF)
cr_count                number of 0x0D bytes
crlf_count              number of "\r\n" pairs
final_line_terminated   last byte is 0x0A
newline_form            none | lf | crlf | cr | mixed
                        (crlf requires lf_count == cr_count == crlf_count)
ascii_bytes             bytes < 0x80
non_ascii_bytes         file_bytes - ascii_bytes
cjk_bytes               UTF-8 bytes of code points inside CJK_RANGES
cjk_byte_share          cjk_bytes / file_bytes  (zero-denominator rule)
utf8_valid              true by construction (the profiler takes &str)
```

`CJK_RANGES` (frozen; the list is code, not prose, so it cannot drift):

```text
U+3000-U+303F  CJK symbols and punctuation
U+3040-U+309F  Hiragana
U+30A0-U+30FF  Katakana
U+3400-U+4DBF  CJK unified ideographs extension A
U+4E00-U+9FFF  CJK unified ideographs
U+AC00-U+D7AF  Hangul syllables
U+F900-U+FAFF  CJK compatibility ideographs
U+FF00-U+FFEF  halfwidth and fullwidth forms
U+20000-U+2FFFF CJK unified ideographs extension B and beyond
```

No newline is normalized, no byte is rewritten, and no source is trimmed.
Byte spans everywhere in this contract are UTF-8 byte offsets into the
**unmodified** source.

## 2. SYNTAX_FACT

```text
grammar_id          the lane whose rules produced this fact
syntax_kind         one of the closed SyntaxKind vocabulary (snake_case)
source_start/end    half-open UTF-8 byte span, relative to the whole document
recognition_status  recognized | not_recognized | ambiguous | unknown
lane_scope_grade    strict_lane_coverage | contract_declared_not_qualified
                    | out_of_lane_candidate | lane_deferred
host_context        false when the bytes are inside a literal region
reason              machine-readable cause (see §2.2)
occurrence          stable source-order index among facts with the same
                    grammar_id + kind + status; the anchor identity
detail              kind-specific frozen detail (fence char, table columns…)
```

### 2.1 Recognition status

```text
recognized      the lane's reference oracle produced a node of this kind
not_recognized  candidate evidence exists AND the lane's rules make it
                ordinary text (or the candidate fails a frozen lane rule)
ambiguous       two declared lanes disagree (e.g. `$…$` under G1 text
                semantics vs the deferred G2 math lane); CORRECTIVE-A
                records the disagreement instead of resolving it
unknown         the construct belongs to a lane whose semantics are not
                frozen (incomplete/unterminated candidate of a deferred kind)
```

**Absence is never inferred from a non-recognition.** A fact is emitted in
exactly two situations: the lane oracle recognized the construct, or a
declared probe found candidate evidence for it. "The detector did not
recognize a table" can therefore never be read back as "there is no table";
it is either a recorded candidate with a reason, or no fact at all — and a
consumer that needs "absent" must ask a `kind_absent` predicate, which is
defined over recognized nodes only.

### 2.2 Reasons

```text
recognized_under_lane_oracle   the oracle owns this construct here
interprets_as_text_under_lane  candidate; the lane reads it as text
candidate_rejected_by_lane_rule candidate shape fails a frozen lane rule
                               (e.g. the escape-aware GFM cell count)
outside_lane_construct_set     not this lane's construct at all
non_host_context               bytes are inside a literal region
lane_semantics_deferred        owning lane has no frozen semantics (G2)
ambiguous_across_declared_lanes two declared lanes disagree
```

The vocabulary is closed. `interprets_as_text_under_lane` is the one member
no current input produces: it is reachable only for a kind a lane *declares
but does not qualify* (the G1 base constructs), and no probe emits a
candidate of such a kind — a host-context candidate of a kind the lane
recognizes is routed to `candidate_rejected_by_lane_rule` first. It is
retained as the vocabulary's defined answer for that case rather than
deleted, and this note is the record that it is currently unexercised.

### 2.3 Evidence grades and what may be counted

```text
strict_lane_coverage            the only grade that may be counted as strict
                                lane coverage or strict H0-H4 evidence
contract_declared_not_qualified lane declares it; evidence not qualified
out_of_lane_candidate           belongs to a construct set this lane does not
                                enable
lane_deferred                   owning lane has no frozen semantics yet
```

`SyntaxFact::is_strict_coverage()` is the single definition of countable
coverage: recognized AND host-context AND `strict_lane_coverage`.
`SyntaxFact::is_scope_blocker()` is the single definition of a scope
blocker: host-context AND (`out_of_lane_candidate` OR `lane_deferred`).

### 2.4 Ambiguity policy

Ambiguous and unknown candidates are never resolved, never dropped, and
never counted:

```text
complete candidate + deferred owner lane   -> ambiguous (lane_deferred)
incomplete/unterminated candidate          -> unknown   (lane_deferred)
```

Both block `strict_scope_clean`, so a file containing them can be profiled
and described but cannot enter strict comparison under that lane. This is
the honest alternative to guessing, and it is what makes "we do not know yet"
visible in the artifact instead of invisible. `p08-g1-math-ambiguous.toml`
pins both branches.

## 3. STRUCTURAL_FACT

Computed from one lane parse, with the counting rules echoed into the record
itself (`block_kinds`, `container_kinds`, `container_depth_convention`,
`largest_block_rule`, `zero_denominator_rule`,
`reference_definition_counting`, `reference_use_counting`,
`math_occupancy_note`) so a reader never has to guess which convention was
used.

```text
block_count              number of nodes whose kind ∈ block_kinds
largest_block_bytes      max (end - start) over block nodes; ties broken by
                         the lowest start offset; 0 when no block node
largest_block_span       the winning span (absent when there is no block)
max_container_depth      max over container nodes of (ancestor count + 1)
fence_density_per_kib    fenced_block_count / (file_bytes / 1024)
code_occupancy           fenced_code_content_bytes / file_bytes
reference_density_per_kib (reference_definition_count + reference_use_count)
                          / (file_bytes / 1024)
table                    {table_count, max_table_columns,
                          max_table_rows_including_header} or absent
math_occupancy           null in every current lane, with a note explaining
                         that no lane has frozen math semantics (G2 deferred)
```

### 3.1 Counting rules per lane

```text
G0 block_kinds       paragraph, heading_atx, code_block_fenced,
                     reference_definition
G0 container_kinds   block_quote, list, list_item
G1 block_kinds       paragraph, heading_atx, heading_setext,
                     code_block_fenced, code_block_indented, html_block,
                     thematic_break, table
G1 container_kinds   block_quote, list, list_item
```

```text
container_depth(node) = number of ancestors whose kind ∈ container_kinds;
                        a top-level block has depth 0; List and ListItem
                        each add 1                     (frozen convention)
zero denominator      -> 0.0, never NaN/inf; numerator and denominator byte
                         counts are reported alongside  (frozen rule)
```

Reference counting is lane-specific and therefore **not** comparable across
lanes without saying so:

```text
G0  definitions  ALL_DEFINITION_NODES: ReferenceDefinition nodes in source
                 order, duplicates counted
G0  uses         RESOLVED_REFERENCE_LINKS: ReferenceLink nodes only; an
                 unresolved candidate is literal text under BENCH-GRAMMAR-v1
                 §9.3
G1  definitions  UNIQUE_NORMALIZED_LABELS: entries of the oracle's
                 reference-definition map; duplicate labels collapse
                 (first-wins per the spec)
G1  uses         REFERENCE_LINK_EVENTS: links the oracle resolved through a
                 reference form (reference/collapsed/shortcut)
```

`fence_density` counts `code_block_fenced` **nodes** (a fence pair), not
fence lines; `code_occupancy` uses the raw content bytes between the opener
line's terminator and the closer line's first byte.

### 3.2 Table facts

Present only for lanes that recognize the table extension (G1 here). The
numbers come from recognized table nodes, never from probes:
`table_count` = recognized tables, `max_table_columns` = cells in the widest
header row, `max_table_rows_including_header` = largest row count including
the header.

### 3.3 Unknown/ambiguous handling in structural numbers

Structural numbers are computed from recognized nodes only. A document full
of math-looking bytes under G1 has `math_occupancy = null` and its math bytes
appear as ambiguous candidates — never as occupancy. This is the structural
form of the same rule as §2.4.

## 4. ELIGIBILITY_FACT

```text
grammar_id                 lane these facts belong to
lane_valid                 the source has a defined interpretation under
                           this lane; true for UTF-8 on any lane that has a
                           frozen implementation, false for a lane with no
                           semantics/oracle (G2 today)
strict_scope_clean         lane_valid AND no host-context scope blocker (§2.3)
scope_blockers             the blocking facts themselves, with spans
strict_scope_clean_rule    frozen statement of what cleanliness means here
```

`lane_valid` is not a formality about UTF-8: it says the lane can interpret
the source at all. A lane with no frozen semantics cannot, so it reports
`lane_valid = false` and `strict_scope_clean = false` together with a warning
naming the missing semantics — a deferred lane must never be readable as an
eligible one. A lane with no implementation also zero-fills its structural
record and labels that record `not implemented` rather than presenting zeros
as measurements.

Eligibility is per source. Pre and post eligibility are computed
independently (`grammar/TRANSITION-ORACLE-v1.md` §4).

The zero-byte document is a legitimate input and profiles normally: both
lanes are total over UTF-8, so `profile_g0("")` returns an empty fact list
with `lane_valid = true` rather than failing. (NORMALIZED-RESULT-v1 rejects
zero-length node spans, and the empty document is exactly one such span, so
the G0 lane skips reference-parse validation for the empty source instead of
relaxing the frozen validator.)

## 5. Host-context rule (not a heuristic)

Syntax-looking bytes inside a literal region are not host syntax. The rule
is implemented by span containment, not by re-scanning text, and the two
fact classes are resolved against different interval lists:

```text
candidate facts   CONTAINED BY a literal construct's FULL span
                  (start of its first line through its last byte;
                   equality counts)
node facts        STRICTLY INSIDE a literal construct's CONTENT interval
                  (equality excluded)
```

Both rules are containment, not intersection, and the asymmetry is two
facts about literal constructs. For a node the content interval is right,
because the construct that owns a literal region (a fenced block, a code
span, an HTML block) is itself host syntax: only what sits inside it is
not, and equality must not mark it non-host. For a candidate the full span
is right, because a lexical probe can anchor on the construct's own
delimiters — the setext probe takes the line *above* the underline and the
table probe starts at its header row's first byte, so on a fence that opens
with those bytes the candidate begins at the opener and would escape a
content-interval test. A candidate is literal only when *all* of its bytes
are, so equality counts and partial overlap never disqualifies.

Literal construct spans by lane (full span for candidates; the content
interval inside those is what nodes use):

```text
G0  fenced block (start of the opener line through the closing fence),
    code span (including its backticks)
G1  fenced block (as G0), indented code block (including its indentation,
    which the oracle's own span excludes), HTML block, raw HTML inline,
    code span
```

Consequences pinned by tests and fixtures: a table-shaped line inside a
fence is `not_recognized` + `non_host_context` (never coverage, never a
scope blocker); a document that is nothing but a fenced code block is
G0-strict; `$x$` inside a code span is not math; and a table whose header
row happens to contain a code span is host syntax, because its pipes and
row structure lie outside the code span.

The fence-node case is the reason the two lists differ, and
`semantics/tests/profiler.rs::a_candidate_anchored_on_a_fence_opener_is_still_literal_content`
pins it in both directions (candidates literal, the owning node still
host-context).

## 6. Candidate probes

Probes exist so that constructs the lane does not own can still be reported
as evidence. They are a declared, frozen, ordered vocabulary — not regexes
run at the reader's discretion:

```text
PROBE_PRIORITY (earlier wins an overlapping candidate range)
front_matter, display_math, directive, table, html_block, link_autolink,
inline_math, raw_html_inline, heading_setext, thematic_break, strong, image,
code_block_indented, code_block_fenced, strikethrough, task_list_item
```

A probe reports `complete = false` when it found only a partial shape:
unterminated display math, an unterminated `**`/`__` run, and an unclosed
`![…](` are the three such probes today. Completeness is not what
distinguishes `unknown` from `ambiguous` (§2.4) — the recognition status
does that — and a `complete = true` probe says only that the probe's own
shape was well-formed, never that the lane recognized the construct.

**Probe/oracle disagreement is recorded, not resolved.** When a probe accepts
a shape that the lane rules reject — for example a header row containing an
escaped pipe, where the escape-aware GFM cell count differs from the lexical
split — the candidate is emitted as `not_recognized` with
`candidate_rejected_by_lane_rule`. When the lane oracle *does* own the
construct, the recognized fact is the only fact: the lexical candidate is not
duplicated beside it. `semantics/tests/profiler.rs` pins both directions.

## 7. Determinism and byte-reproducibility

```text
- profiling the same source twice under the same lane produces byte-identical
  canonical JSON (sorted keys, no whitespace);
- fact order is deterministic: facts are sorted by
  `(source_start, source_end, syntax_kind, recognition_status)`, so a
  reader sees one source-position order regardless of which of the three
  emitters produced a fact;
- occurrence indices are stable and are the anchor identity used by position
  resolution and by the §11.1 tie-break;
- no timestamps, no paths, no machine facts, and no mechanism facts appear in
  a profile: a profile is a property of the bytes and the lane.
```

## 8. Validation and spot-check policy

```text
semantics/tests/profiler.rs        host-context rule (G0 code spans, G1
                                   fences, and a candidate anchored on a
                                   fence opener), empty-source totality,
                                   span evidence (char boundaries, in-range,
                                   recognized ⊆ oracle nodes), table facts
                                   from recognized tables only,
                                   ambiguous/unknown policy, probe/oracle
                                   disagreement, source facts, determinism
semantics/tests/contracts.rs       lane identity stability, generated-artifact
                                   drift guard, published artifacts carry no
                                   machine paths, profiler identity
                                   versioning, no mechanism dependency in the
                                   crate
workloads/pilots/fixtures/*.toml   hand-authored expectations, including
                                   CJK/emoji byte spans (P07) and the
                                   G0-vs-G1 setext divergence (P06)
workloads/pilots/real-source-v1.json  one real document (oci-runtime
                                   config.md, sha256-pinned) profiled under
                                   both lanes; the profile is compared against
                                   the declared lane rules, and its bytes are
                                   PR #34 acquisition material (see
                                   workloads/pilots/README.md §4)
```

A deterministic misclassifier is not evidence. Every syntax fact carries an
auditable span into the unmodified source, and the pilot fixtures assert
exact byte spans for the constructs they declare, so a reviewer can always
check a fact against the bytes.

## 9. Versioning

```text
PROFILER_VERSION bumps when a fact field, a counting rule, a probe rule, or
  the ambiguity policy changes;
PROFILE_SCHEMA (real-profile-v1) tags the record shape;
the generated schema artifact is rewritten in the same commit and guarded by
  the drift test;
lane-specific changes bump that lane's lane_version instead
  (grammar/GRAMMAR-LANES-v1.md §7).
```
