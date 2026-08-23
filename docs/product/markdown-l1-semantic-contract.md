# Markit Markdown L1 Semantic Contract

Status: **Accepted for P0-02** (implementation contract). This document is
the normative definition of the Markit **Markdown L1 dialect** referenced by
ADR-004 and `docs/product/mvp-v0.1.md`. It is written **before** the parser
exists: the parser implements this contract, never the reverse.

Relationship to CommonMark: L1 is a deliberate subset of CommonMark 0.31.2
with **documented deviations** (§11). Where this document is silent and a
construct is in scope, CommonMark 0.31.2 (`spec.commonmark.org/0.31.2`) is
the tiebreaker reference. Where this document lists a deviation, the
deviation — not CommonMark — is Markit's semantics.

Hard rule: **no silent expansion of scope.** Any construct not listed here
degrades predictably to text (§7.5). "It looked like a heading" is never a
reason to parse a heading.

## 1. Scope

L1 covers exactly the `mvp-v0.1.md` construct list:

```text
block:    blank, paragraph, ATX heading, blockquote,
          unordered list, ordered list, fenced code
inline:   text, emphasis, strong, inline code, link
```

Out of scope (and explicitly degraded, §11): setext headings, thematic
breaks, indented code, reference links, images, autolinks, raw HTML
(block or inline), entities, hard line breaks, footnote/extension syntax of
any kind, and **nesting of block constructs inside containers** (§6).

## 2. Coordinate and line model

The L1 parser operates on the coordinate model of `markit-core` (P0-01):

- Coordinates are **byte** offsets (`ByteOffset` / `SourceRange`). Unicode
  correctness is byte-safety: classification only inspects ASCII prefix
  bytes and never splits a UTF-8 scalar. Multi-byte content (CJK, emoji)
  is ordinary text everywhere.
- Lines are the `Document` line index's lines: split on `'\n'` only;
  a document ending in `'\n'` has a final empty line; `LineNumber` is
  0-based.
- **CRLF terminators** (the project is Windows-first): line
  terminators are `'\n'` or `'\r\n'`. A line's **classification text**
  drops a single `'\r'` immediately before its `'\n'` — `# h\r\n` is a
  heading, `"```\r"` closes a fence, `"\r"` alone is blank, and a
  fence info string excludes the `\r`. Source bytes, source ranges,
  fingerprints, inline-run text, and saved files keep `\r\n` verbatim:
  round-trips are byte-identical and no normalization ever happens. A
  `'\r'` anywhere else — mid-line, or at end of document without a
  final newline — is an ordinary content byte.
- Line content is the line's bytes excluding its `'\n'` terminator
  (a `\r\n` line's content therefore ends with `\r`; classification
  drops it as above).

Terms used below:

- **indent** of a line: its count of leading space characters.
- **blank** line: content empty or consisting only of spaces/tabs.
- **opener**: a line-shape that starts a block (§5).

## 3. Block vocabulary

```rust,ignore
enum BlockKind { Blank, Paragraph, Heading, BlockQuote, UnorderedList, OrderedList, FencedCode }
```

`Heading` is ATX-only. `FencedCode` covers backtick and tilde fences
(the fence character and opening length are record metadata, not kinds).

### 3.1 Tiling invariant

Blocks **tile** the document: ordered by position, `blocks[0]` starts at
byte 0, `blocks[i+1]` starts exactly where `blocks[i]` ends, and the last
block ends at `len`. Every byte of the document belongs to exactly one
block, including line terminators and blank lines. This invariant is what
makes structural alignment (§9) well-defined and is asserted in tests.

A block's `line_span` is the half-open `[first_line, last_line + 1)` and
`source_range` spans `[first_byte, end_of_last_line_including_its_\n)`
(or document end for a final block without a trailing newline).

## 4. Nesting rule (fixed at depth 0)

The L1 block stream is **flat**. Containers hold text-level content only:

- a **blockquote**'s content is paragraph text; block openers inside a
  quote line (`> # h`, `> - a`, `> ```) are **literal text**, not nested
  blocks;
- a **list**'s items are text runs; indented marker lines do not nest
  (§6.5) and non-item constructs terminate the list instead of nesting.

Deeper structure arriving in L1 text therefore degrades to flat text and
renders as such. This is a product decision for v0.1 (deeper nesting is a
future dialect level, not a parser bug), and it is what keeps the restart
state (§9) two-variant small.

## 5. Block classification order

At each line start, in parser **Ground** state (§9), classification tries
in order and takes the first match:

```text
1. blank line                        -> Blank run
2. fenced-code opener                -> FencedCode
3. ATX heading opener                -> Heading
4. blockquote line ('>' form)        -> BlockQuote run
5. list marker line                  -> UnorderedList | OrderedList
6. otherwise                         -> Paragraph
```

Rules that qualify step 5:

- A line with **indent ≥ 4 is never an opener** (any kind): it is
  paragraph/list-continuation text. (No indented code blocks — §11 D3.)
- A **tab is never part of an opener prefix**: a line starting with a tab
  is not an opener (no tab expansion — §11 D4).
- **Paragraph interruption**: heading, fence, and blockquote openers
  always interrupt a paragraph. A list marker line interrupts a paragraph
  only if the item is non-empty **and**, for ordered lists, the first
  number is `1` (CommonMark rules, adopted verbatim). An interrupting
  opener ends the paragraph and opens its own block; a non-interrupting
  marker line is paragraph text.
- **Fence state suppresses everything**: while inside an unclosed fence
  (§6.7) no line is classified at all; every line is fence content until a
  matching closer or end of document.

## 6. Block semantics

Each kind below defines: opener, continuation, termination, content.

### 6.1 Blank

- Opener: a blank line.
- Continuation: immediately following blank lines join the same block
  (one `Blank` record per maximal run).
- Termination: first non-blank line or end of document.
- Content: none. No inline IR.

### 6.2 Paragraph

- Opener: a non-blank line that is no other kind's opener.
- Continuation: each following non-blank line that is not an opener
  (interruption rules of §5 apply).
- Termination: blank line; any qualifying opener (heading, fence,
  blockquote, interrupting list); end of document.
- Content: the whole span is one **inline run** (newlines inside a
  paragraph are part of the run; emphasis may span lines).

### 6.3 Heading (ATX)

- Opener: indent ≤ 3, then 1–6 `#`, then a space, tab, or end of line.
  `#######` (7+) is a paragraph. The **level** is the `#` count.
- Closing sequence: after stripping trailing spaces/tabs, if the content
  ends in a run of `#` **preceded by a space or tab**, that run (and the
  whitespace before it) is a closing sequence and is excluded from the
  content. Escaped `\#` cannot be part of a closing sequence.
- Continuation: none — a heading is exactly one line.
- Termination: end of its line.
- Content: the bytes between opener and closing sequence, one inline run.
  `# Head` level 1; `## ` empty content, level 2.

### 6.4 Blockquote

- Opener/continuation line form: indent ≤ 3, then `>` (then anything).
- Continuation: each immediately following line of the same `>` form.
- Termination: the first line **not** of the `>` form (blank or not), or
  end of document. There is **no lazy continuation**: a paragraph line
  without `>` ends the quote (§11 D5).
- Content: per quote line, strip indent, `>`, and at most **one**
  following space; the remainder is that line's content segment.
  Content segments are independent inline runs (§7.7). A `>` line with
  empty remainder contributes an empty segment; the quote is not
  terminated by it.

### 6.5 Lists (unordered, ordered)

Marker forms, with indent ≤ 3:

- unordered: one of `-`, `+`, `*`, followed by space, tab, or end of line;
- ordered: 1–9 ASCII digits, then `.` or `)`, followed by space, tab, or
  end of line.

After the marker, all immediately following spaces/tabs are stripped; the
remainder of the line starts the item's text (possibly empty).

- Opener: a marker line (subject to §5 interruption rules when following a
  paragraph).
- Item continuation: after a marker line, each following **non-blank** line
  that is not any kind of opener joins the current item's text (indent
  does not matter — §11 D8/D9).
- Termination: a blank line (§11 D7); any opener line (heading, fence,
  blockquote, or a marker line of a **different signature**); end of
  document. A same-signature marker line starts the next item instead of
  terminating.
- **Signature**: unordered — the bullet character; ordered — the delimiter
  (`.` vs `)`). A signature change terminates the list and opens a new one
  (CommonMark behavior). The ordered **start number** is the first item's
  number; later numbers are ignored (renumbering is presentation).
- Content: one inline run per item = the item's first-line text plus its
  continuation lines, contiguous in source. Delimiters do not pair across
  items (§7.7). Marker prefixes belong to no inline node.

### 6.6 Blockquote/list interplay

Because L1 containers are flat, block openers inside container content do
not nest; they either terminate the container (when at line level, §6.5)
or are literal text (inside quote lines, §4).

### 6.7 Fenced code

- Opener: indent ≤ 3, then a run of ≥ 3 backticks or ≥ 3 tildes (never
  mixed). The rest of the line is the **info string**, trimmed of leading
  and trailing spaces/tabs. A **backtick** fence's info string must not
  contain a backtick — otherwise the line is not an opener (paragraph).
  A tilde fence's info string has no restriction (CommonMark 0.31.2).
- Continuation: every following line is content, verbatim. Leading
  indentation of content lines is **not stripped in the IR** — stripping
  up to the opener's indentation is a presentation rule applied
  downstream (same rendered result as CommonMark).
- Closing fence: a line with indent ≤ 3, a run of the **same** fence
  character with length ≥ the opening run's length, followed only by
  spaces/tabs. Nothing else closes the fence.
- Termination: closing fence, or end of document. An **unclosed fence
  runs to end of document** — this is deliberate (issue #12 / ADR-004):
  fence semantics are never bent to bound invalidation. The structural
  cost (rescan to EOF for edits inside an unclosed fence) is honest and
  reported through the work counters (§9).
- Content: none at inline level (no inline IR; the whole block is opaque).

## 7. Inline model

### 7.1 Text and escapes

A backslash before an ASCII punctuation character is an escape: the pair
is literal text containing that punctuation, and the escaped character
never opens/closes any construct. A backslash elsewhere (or at run end)
is literal. Block openers are line-prefix-based, so `\# x`, `\- x`, `\> x`
are paragraphs whose inline text begins with an escape — no block-level
escape machinery exists or is needed.

### 7.2 Inline code

A **backtick string** of length *n* opens a code span; it closes at the
next backtick string of exactly length *n*. Content between is opaque:
escapes, emphasis, links, and backtick strings of other lengths are
literal. CommonMark's presentation rules (newlines → spaces; strip one
leading/trailing space when both present and content is not all spaces)
apply downstream; the IR records source ranges only. With no matching
closing string, the opener is literal text.

### 7.3 Emphasis and strong

Delimiter runs are maximal runs of `*` or `_` whose first character is not
escaped. Flanking (simplified, §11 D11):

- **can open** (`*`): the character after the run is not whitespace;
- **can close** (`*`): the character before the run is not whitespace;
- `_` additionally requires word boundaries: can open only if the run is
  at run-start or preceded by whitespace, and can close only if at
  run-end or followed by whitespace (no intraword `_` emphasis).

Run boundaries are whitespace per Rust `char::is_whitespace` (Unicode).
Construct edges (code spans, links) count as non-whitespace neighbors.

Pairing (CommonMark's algorithm, minus the rule of 3 — §11 D10):
closers are processed left to right; each matches the nearest earlier
same-character opener that can open; if both runs have ≥ 2 delimiters
left, 2 pair into **strong**, else 1 pairs into **emphasis**; leftover
delimiters stay available. Delimiters inside code spans or link
destinations do not participate. Unmatched delimiters are literal text.
Emphasis may span line boundaries inside one inline run (§7.7).

### 7.4 Links

Inline links only: `[text](destination)` with an optional title:
`[text](destination "title")` (title also in `'…'` or `(…)`).

- `text` may contain balanced brackets and is parsed for inline
  constructs; if it contains a link, the **outer** link is not a link
  (links do not nest) and all of it stays literal except the inner link.
- `destination` is either `<…>` (no unescaped `<`/`>`/newline inside) or
  bare (no ASCII whitespace, balanced unescaped parentheses). Escapes
  apply inside both.
- `title`: optional, separated by whitespace (newlines allowed), quoted;
  may not contain its own unescaped quote. Paren titles must balance.
- Anything malformed (missing `(`, unbalanced, invalid destination) makes
  the `[` literal text; scanning resumes inside the brackets.
- `!` is ordinary text: with images out of scope, `![alt](url)` renders
  as literal `!` followed by a normal link (§11 D12).

### 7.5 Predictable degradation

Every unsupported construct is **text**: setext underlines, thematic
breaks, indented code, reference links (`[a]: url` definitions and
`[a][b]` uses), images (as above), autolinks (including `<…>`), raw HTML
(block or inline), entities (`&amp;`), hard line breaks (trailing spaces
or backslash are text). Degradation is total and local: an unsupported
construct never changes the classification of surrounding lines (this is
why setext is rejected — §11 D1).

### 7.6 Inline scope by block kind

- Paragraph: one run over the whole block.
- Heading: one run over the content range.
- Blockquote: one run per quote line's content segment.
- Lists: one run per item.
- Fenced code, blank: no inline IR.

### 7.7 Run independence

Each inline run (and each segment of blockquote/list content) is parsed
independently: delimiters never pair across run boundaries — not across
list items, not across quote lines. Runs do pair across plain line
boundaries inside one paragraph or one list item.

## 8. Records, ranges, fingerprints

For each block the parser produces a `BlockRecord`:

- `id` (§10), `kind`, `source_range`, `line_span`;
- `state_before` / `state_after` (§9);
- kind metadata: heading level; fence character/length/info range; list
  signature and ordered start; quote segments; per-item marker ranges;
- `fingerprint`: FNV-1a-64 over the block's source bytes.

Fingerprints are diagnostics and identity-pairing evidence; **soundness of
incremental updates never depends on them** (§9.3). Inline IR (nodes with
source ranges into the run; emphasis/strong have children; code/link
carry their operand ranges) is derived state stored per block and
reparsed only for blocks whose records changed.

## 9. Restart and convergence semantics

### 9.1 Restart state

```rust,ignore
enum BlockParseState { Ground, InFence { fence_char: u8, fence_len: usize } }
```

- Every block boundary with state `Ground` is a **safe restart
  boundary**: parsing the suffix from that boundary is deterministic and
  independent of everything before it.
- A block boundary inside an open fence is not safe; the enclosing
  `FencedCode` record's start is the boundary (its `state_before` is
  `Ground`).
- Each record's `state_before`/`state_after` make the state at any block
  boundary queryable without reparsing.

### 9.2 Incremental update obligation

An update consumes the P0-01 canonical per-edit regions (`EditResult`
→ each `AppliedEdit`'s `old_line_span`/`new_line_span` — **never**
`covering_*`). For each edit island, in document order:

1. **Rewind**: find the last old block whose `state_after` is `Ground`
   and which starts at or before the island's first affected old line;
   restart parsing there. Old blocks intersecting the island's old line
   span are **dead** (candidates for identity pairing only). An edit
   landing on a block's first line rewinds one block further when the
   previous block is a paragraph, list, quote, **or blank run** — a
   continued or blanked line extends the block above (blank runs are
   maximal), and the reparse must include it or the survivor below
   would keep a shape a full rebuild would merge away.
2. **Reparse forward** over the new document, emitting records.
3. **Converge** at the first subsequent point where all hold:
   parser state is `Ground`; the next parse position equals the
   (delta-shifted) start of the next surviving old block — an old block
   whose old line span lies entirely after the island; that block's
   `state_before` is `Ground`. Two refinements for the *empty* survivor
   (the final-empty-line block): it may not converge when the new
   document no longer ends with a terminator (deleting the final
   newline deletes that line's existence), nor when the reparse's last
   block is a blank run (maximal runs already consumed the final empty
   line; the survivor would duplicate it). Then splice: new records
   replace the dead ones, and every surviving old block is kept, its
   ranges shifted by the accumulated byte/line delta of preceding
   edits — the shift uses the per-edit exact `line_delta`
   (newlines in minus newlines out), not line-span arithmetic, whose
   exclusive-end semantics undercount replacements ending in a
   terminator.
4. If convergence never happens before end of document, the document ends
   (honest unclosed-fence propagation, §6.7).

Islands are processed **sparsely**: two distant one-line edits are two
islands with independent restart/convergence points. Islands merge only
when their rewind–convergence spans actually overlap (semantic overlap,
never distance-based coalescing).

### 9.3 Why convergence is sound

Surviving old blocks' bytes are untouched by construction (the edit
affected only island lines); parsing is deterministic and depends only on
state + suffix bytes; therefore a fresh full parse from the convergence
point produces exactly the surviving old records. Convergence is
**parser state + structural alignment**, not "same line count" or "same
kind": it cannot fire inside an open fence, and position equality is
checked exactly (fingerprints are asserted equal in tests as a
strengthening check, never trusted for the decision).

## 10. Block identity (`InternalBlockId`)

`InternalBlockId` is a monotonically increasing counter owned by the
Markdown state. It is **internal product identity** — used by the future
view model for stable block references — and is **not** plugin/API
identity (issue #12 R9; `docs/product/plugin-compatibility-contract.md`).
Retired ids are never reused.

Rules, in order:

- **(A) Untouched blocks keep their ids.** Surviving blocks (§9.2 step 3)
  keep ids unconditionally — including blocks far from the edit and
  blocks after an unclosed-fence rescan.
- **(B) 1:1 structural correspondence keeps the id.** Dead old blocks
  pair with newly parsed blocks by greedy in-order matching on equal
  `kind`: pairs keep the old id (even when text changed — e.g. typing
  inside a paragraph, editing fence content, editing a heading's text).
- **(C) Ambiguous transforms mint fresh ids.** Kind changes
  (paragraph→heading), splits (old block pairs with the first new block;
  extras mint), merges (surplus old ids retire), and fence
  reinterpretations follow from the same deterministic pairing — no
  lineage graph, no heuristics.

The pairing is total and deterministic: same input always yields the same
id assignment (pinned by tests).

## 11. Deviations from CommonMark 0.31.2

| # | Topic | CommonMark | Markit L1 | Why |
|---|-------|-----------|-----------|-----|
| D1 | Setext headings | `===`/`---` under text make a heading | paragraph text | setext reinterprets a **preceding** block — non-local invalidation, poor incremental behavior |
| D2 | Thematic breaks | `---` etc. are `<hr>` | paragraph text | interacts with D1/list ambiguity; minimal v0.1 vocabulary |
| D3 | Indented code | 4-space indent = code | text | removes paragraph-continuation/code ambiguity; fenced code is the v0.1 code form |
| D4 | Tabs | expanded to 4-column stops | never expanded; leading tab disqualifies openers | no column arithmetic in the hot path |
| D5 | Blockquote laziness | paragraph continuation may omit `>` | quote ends at first non-`>` line | termination stays local & restart-safe |
| D6 | Containers nest | quotes/lists contain blocks | flat; container content is text (§4) | v0.1 vocabulary; two-variant restart state |
| D7 | Loose lists | blank line inside list keeps one list | blank line terminates the list; later marker starts a new block | stateless single-pass termination |
| D8 | List indentation | content indent defines nesting | indent never nests; marker lines at ≤3 join the enclosing list | flat model (D6) |
| D9 | List item continuation | requires content-column indent (lazy for paragraphs) | any non-blank non-opener line continues the item | single-pass, no column tracking |
| D10 | Emphasis rule of 3 | run-length mod-3 restriction | not implemented | rare edge; simplified pairing stays testable |
| D11 | Flanking classes | Unicode punctuation-aware | whitespace-only; `_` word-boundary rule as §7.3 | no Unicode category tables in a zero-dependency core; micro-deviations fixture-covered |
| D12 | Images/autolinks/HTML/entities/reference links/hard breaks | parsed | literal text (§7.5); `!` is text so `![a](b)` = `!` + link | out of v0.1 vocabulary; degradation is local and predictable |
| D13 | Link titles | supported | supported (`"…"`, `'…'`, `(…)`) | alignment, not deviation (listed to pin it) |
| D14 | Info strings | backtick fence info must not contain backticks; tilde unrestricted | identical | alignment (listed to pin it) |

Everything not listed: L1 matches CommonMark 0.31.2 for in-scope
constructs (fence open/close rules, ATX form, interruption rules, marker
signatures, ordered start, code-span matching, link grammar).

## 12. Test obligations (normative for P0-02)

- **Golden fixtures** per construct, each covering: canonical form,
  boundary cases, malformed input, CJK/emoji/multi-byte content, EOF, and
  missing final newline. CRLF line endings get their own fixtures —
  heading, blank, list, quote, fenced code (closer and info string),
  mixed endings, EOF without a final newline, mid-line `\r` — all
  asserting that classification matches the LF behavior while block
  bytes keep `\r\n` verbatim. CommonMark-derived cases cite the spec
  example; deviation cases cite their D-number.
- **Differential oracle**: incremental update == full rebuild after every
  edit in randomized runs (deterministic seeds; compare kinds, ranges,
  line spans, states, fingerprints, inline IR, version — **not** ids).
- **Identity battery**: the §10 rules as regression cases (untouched,
  text edit, kind change, split, merge, fence content edit, unclosed
  fence, sparse two-location transaction, blank-run edit).
- **Structural work tests**: local edits must not rescan the document
  (counters, not milliseconds); sparsity must survive multi-edit
  transactions (1M-line document, two distant edits, middle untouched).
