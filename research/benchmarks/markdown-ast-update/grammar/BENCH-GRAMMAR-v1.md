# BENCH-GRAMMAR-v1 — frozen shared Markdown semantics (R3)

Status: **R3 FREEZE CANDIDATE — READY_FOR_ADVERSARIAL_R3_REVIEW**
Authority: `protocol/R0-METHODOLOGY.md` §5 (construct set, FROZEN) + this
file (exact semantics R3 is allowed to freeze). This file is the single
owner of BENCH-GRAMMAR-v1 semantics. `protocol/grammar.md` remains a
pointer; `NORMALIZED-RESULT-v1.md` owns the result vocabulary, not the
grammar.

R3 mandate: make the grammar **unambiguous**, not CommonMark-complete.
Every rule below exists so that two reasonable implementations of these
rules produce byte-identical normalized trees (see
`NORMALIZED-RESULT-v1.md`), and so that an edit never changes
interpretation without this contract saying what happens.

---

## 0. What this grammar is

A deliberately small, total, deterministic Markdown subset whose only
purpose is to create the mechanism stresses required by R0 (§8–§9) and
motivated by R2. It is a benchmark input language. It is NOT:

- a production Markdown grammar;
- CommonMark (deviations are listed in §11 and are authoritative);
- extensible by horses (all horses parse this exact language, with the
  shared scanner/grammar primitives of the parity contract).

Every input admitted by the corpus contract (`corpus/CORPUS-v1.md`) is a
valid BENCH-GRAMMAR-v1 document. Parsing is **total**: every byte belongs
to exactly one normalized node subtree; there are no error nodes.

## 1. Encoding and lines

```text
Encoding        UTF-8. All offsets are UTF-8 bytes (NORMALIZED-RESULT-v1 §2).
Line terminator LF (\n = 0x0A) only. CR (0x0D) never appears in corpora;
                if present it is an ORDINARY character (text), not a
                terminator, not whitespace.
Line            a maximal run of bytes not containing LF, plus its LF
                (the final line may lack the LF only at EOF).
Blank line      a line whose content is zero or more SPACES (0x20) only.
                A TAB (0x09) is NOT whitespace: a line containing only a
                tab is NOT blank (see §3).
Indentation     SPACES only. Tabs never participate in indentation
                arithmetic anywhere in this grammar (deviation D1).
```

## 2. Block dispatch (normative order)

Parsing is line-synchronous at block boundaries. When the parser is at a
block start (document start, after a blank line, after a closed block, or
inside container content), the current line — after removing the container
prefix (§6, §7) and after removing up to **3 leading spaces relative to
the current content column** — is matched against this ordered list. The
FIRST match wins; ordering is part of the freeze:

```text
B1  blank line                (blank per §1)
B2  fenced code opener        (§8)
B3  ATX heading               (§5)
B4  blockquote marker         (§6)
B5  list marker               (§7)
B6  reference definition      (§9.4)
B7  paragraph                 (§4)
```

Consequences frozen here:

- A heading/fence/blockquote/list-marker/reference-definition line
  **interrupts** an open paragraph.
- A line that matches none of B1–B6 continues or starts a paragraph.
- Reference definitions interrupt paragraphs (like CommonMark); they are
  recognized only at a block start, never mid-line.

## 3. Paragraph (B7)

```text
starts      a line matching none of B1–B6 at the current level.
continues   while subsequent lines (at the same level) match none of
            B1–B6. Continuation lines are NOT re-indented: their leading
            spaces (if any) are ordinary content bytes, except inside a
            list item, where exactly the item's content-indent bytes are
            stripped first (§7).
ends        at a B1–B6 line, at a container-closing event (§6/§7), or EOF.
soft break  a single LF inside a paragraph stays INSIDE the paragraph
            (there is no soft-break node; the LF is interior text bytes).
```

## 4. Text and inline content

Paragraph and heading content is scanned for inline constructs
(§9, §10). Text is everything not inside an inline construct: **maximal
runs** of source bytes between/around inline constructs, one Text node per
run (a run may span several lines and contain LFs). Runs are maximal
WITHIN the parent block's content region: a container prefix and the LF
preceding it are trivia BETWEEN runs, so a paragraph inside a quote or
list item yields one Text per contiguous content run (§6, §7). No escapes
exist: backslash is an ordinary character (deviation D2). Trailing spaces
of a line are ordinary text bytes. Leading indentation of a block's FIRST
line is never part of any node span; interior/trailing bytes are.

## 5. ATX heading (B3)

```text
lexical form   1–6 '#' characters, then (a) end-of-line, or (b) one or
               more SPACES, then content = the rest of the line.
level          the number of '#' (1–6).
not a heading  a '#' run of length > 6, or '#' not followed by space/EOL
               ("#foo") — the line is a paragraph (fixture F006).
closing seq    NOT stripped: trailing '#' characters are content
               ("# a #" has content "a #", fixture F008; deviation D3).
children       the content is inline-scanned (§4, §9, §10); content may
               be empty (no children).
termination    always exactly one line (never continues).
```

## 6. Block quote (B4)

```text
marker        up to 3 leading spaces, then '>'.
content       the rest of the line after '>' and ONE optional space.
              Content may be empty ("'>" or "> " alone = a blank line
              INSIDE the quote; it does not close it and contributes no
              block).
nesting       the content region is re-dispatched from B1; a content
              starting with '>' nests (">> a" and "> > a" are the same
              tree, different spans).
continuation  a blockquote continues only while its lines carry the '>'
              prefix. A line without '>' at the quote's level CLOSES the
              quote (no lazy continuation; deviation D4). An outer blank
              line closes it.
interrupts    B4 may interrupt a paragraph.
inner blocks  paragraphs, headings, fences, lists, nested quotes,
              reference definitions — full B1–B7 dispatch on content.
```

## 7. List (B5) — the one frozen list form

Unordered only; no ordered lists, no tight/loose distinction (deviation D5).

```text
marker set    '-' or '*'. (No '+'.)
item opening  a line whose leading indent s (spaces, relative to the
              current content column after container prefix) satisfies
              0 <= s <= 3 at the level where the list is being opened,
              followed by the marker, followed by 1..4 spaces — OR the
              marker alone at end-of-line (EMPTY ITEM, no children;
              its content indent is the marker column + 1).
content indent  c = (marker column) + 1 (marker width) + k, where
              k = number of spaces after the marker, capped: if k > 4,
              c = marker column + 2 and the remaining spaces are content
              bytes (CommonMark rule, frozen).
first line    item content = rest of line after marker + spaces.
continuation  a non-blank line with indent >= c belongs to the item; its
              first c (absolute) columns are container trivia (stripped,
              never in spans); the rest is content. Lines with
              indent >= c that have a marker with (indent - c) in [0,3]
              are NOT continuation text: they open/continue a NESTED
              list inside the item (§7 nesting).
item end      first line with indent < c (at the item's container level),
              or a blank line, or EOF.
list end      blank line (tight-only: a blank line ALWAYS ends the whole
              list at its level; deviation D5), or a line that is neither
              a valid continuation nor a new item at an enclosing level,
              or EOF.
nesting       unlimited depth; a nested list is a child of the item whose
              content region contains it. Indent jumps of more than 3
              beyond c make the line ordinary continuation text (no
              deep-indent list).
sibling rule  after an item ends via indent < c, a marker line at the
              outer list's indent continues the SAME List with a new
              ListItem; any other line ends the List.
empty item    the marker-alone form above; it contributes an empty
              ListItem (no children, fixture-free — corpora never emit
              it).
interrupts    B5 may interrupt a paragraph.
no lazy mode  a non-indented line after an item NEVER continues it
              (deviation D4); it is parsed at the outer level.
```

The experimental property of this section is **container-state
propagation**: the interpretation of a line depends on the open container
stack (indent arithmetic, marker state), which edits can break at a
distance. That is the point of the DEEP_CONTAINER shape and the
CONTAINER_STATE mutation family — not CommonMark completeness.

## 8. Fenced code block (B2)

```text
opener        0..3 leading spaces (relative to content column), then a
              run of >= 3 BACKTICKS (0x60) — no tilde fences (D6) — then
              an optional info string: the rest of the line. The info
              string may not contain a backtick; if it does, the line is
              NOT a fence opener (it is paragraph text).
closing fence a line with 0..3 leading spaces, a run of backticks of
              length >= the opener run length, optional leading/trailing
              SPACES only, nothing else.
body          raw bytes; NO inline scanning, no escapes, no constructs
              (fixture F022). Fence characters inside the body are body
              bytes (a backtick run that does not satisfy the closing
              rule is body).
body/containers  inside a container, body lines still carry the container
              prefix (consumed for fence-boundary detection per §2/§7),
              but the body BYTES of the block are the raw interval from
              the opener line's LF to the closer line's first byte (to
              EOF when unclosed), unstripped, in document coordinates —
              container-prefix bytes inside the body are raw body bytes.
unclosed      an unclosed fence runs to EOF (fixture F019).
termination   the closer line ends the block; the fence may interrupt a
              paragraph (B2 ordering).
forward state this construct is the carrier of FORWARD_STATE propagation:
              whether a line is body or structure depends on an opener
              arbitrarily far back.
```

## 9. Links, references, definitions

These semantics are load-bearing for REFERENCE_FANOUT and the
SEMANTIC_DEPENDENCY family: reference resolution is **document-global and
position-independent** (a use may precede its definition — fixture F034).

### 9.1 Label normalization (normative)

```text
norm(label_bytes):
  1. replace every run of one or more SPACES with one space
  2. trim leading/trailing spaces
  3. ASCII-case-fold (A-Z -> a-z)
Unicode case folding is NOT applied (deviation D7). Tabs, being ordinary
characters, do not collapse.
```

### 9.2 Link / reference-link text scanning

At an inline scan, a '[' begins a candidate:

```text
text region   bytes up to the first ']' that is NOT inside an open code
              span (code spans win, fixture: backtick-aware scan);
              nested '[' inside the text is literal; nested links are
              NOT recognized.
inline link   immediately after ']': '(' then destination then ')'.
              destination = the bytes up to the FIRST ')'; the construct
              fails (-> literal text) if the destination region contains
              a SPACE, a '[', or no ')' follows on the same line
              (fixture F033). Empty destination "()" is valid.
reference     immediately after ']': '[' then label bytes then ']' with
  link        NO whitespace between the two bracket groups. Empty label
              "[]" = collapsed form: the label is the TEXT bytes.
              Any other byte after ']' -> the candidate fails (shortcut
              references "[text]" are NOT links; deviation D8).
failure       a failed candidate decomposes into ordinary text bytes and
              scanning resumes after its '[' (one byte forward).
```

### 9.3 Reference resolution (semantic fact)

```text
a ReferenceLink whose normalized label has a definition in the document's
reference-definition table is RESOLVED: its destination field is the
destination bytes of the definition.
a ReferenceLink with no matching definition is NOT a ReferenceLink: the
construct is literal text (fixture F036). This is the frozen fallback
semantics (CommonMark-compatible).
duplicate definitions: the FIRST definition of a normalized label wins;
later duplicates still parse as ReferenceDefinition nodes, they only lose
lookup (fixture F035).
```

### 9.4 Reference definition (B6)

```text
lexical form  at a block start (may interrupt a paragraph), 0..3 leading
              spaces, then '[' label ']' ':' then one or more SPACES,
              then destination = bytes up to the first SPACE or EOL
              (destination itself contains no space), then only optional
              SPACES to EOL.
              Anything else on the line -> NOT a definition (paragraph).
single line   definitions never continue to the next line (deviation D9).
no titles     title strings are not part of the grammar (D9).
destination   may be empty (label with empty destination is legal).
facts         a definition contributes the fact
              (norm(label) -> destination bytes) to the document-global
              table, in source order.
```

## 10. Emphasis and code spans (inline delimiter state)

### 10.1 Code span

```text
opener   a maximal run of n >= 1 backticks.
closer   the next run of EXACTLY n backticks (fixture F028: a shorter run
         inside is content; a longer run does not close an n-run).
content  the exact source bytes between opener and closer. NO space
         stripping, NO line-ending normalization (deviation D10).
raw      inside a code span nothing is scanned (no emphasis, no links).
failure  an unclosed backtick run is literal text.
```

### 10.2 Emphasis

One unambiguous delimiter contract; NOT the CommonMark algorithm (D11).

```text
delimiter      the single character '*' (underscore is never a delimiter,
               D11).
can open       next byte exists and is not a SPACE and not an LF.
can close      previous byte exists and is not a SPACE and not an LF.
empty-emphasis an opener and a closer that are ADJACENT (opener_pos + 1
  ban          == closer_pos) create an EMPTY Emphasis and are forbidden:
               a '*' is not closable against an opener immediately next
               to it. (This is what makes "**a**" one nested pair instead
               of two empty pairs.)
matching       scan inline content left to right with a stack:
                 - at a '*' that can close (non-adjacent opener on the
                   stack): CLOSE the most recent such opener (LIFO); the
                   Emphasis span covers [opener '*', closer '*'] inclusive
                   and its content is re-scanned recursively as inline
                   content;
                 - else if it can open: PUSH;
                 - else: literal text byte.
               Tie-break (a '*' both open- and close-able): CLOSE wins
               whenever a legal (non-adjacent) opener exists, else OPEN.
result         unmatched delimiters are literal Text bytes (F025, F026).
emphases do not span blocks; each paragraph/heading/link-text region is
scanned independently.
```

Worked examples pinned by fixtures:

```text
"a *b* c"      -> Text, Emphasis(b), Text
"**a**"        -> Emphasis(Emphasis(a))          (F024)
"*a**"         -> Emphasis(a), Text("*")          (F026)
"*a"           -> Text("*a")                      (F025)
```

## 11. Frozen deviations from CommonMark (authoritative list)

```text
D1  tabs are ordinary characters; no tab-stop arithmetic
D2  no backslash escapes anywhere
D3  ATX closing sequences are not stripped
D4  no lazy continuation for blockquotes or list items
D5  lists: unordered (-,*) only; tight-only (blank line ends the list);
    no loose-list merging across blanks
D6  no tilde fences; backtick fences only
D7  label matching: ASCII case-fold only (no Unicode folding)
D8  no shortcut reference links
D9  reference definitions are single-line, no titles
D10 code spans: exact bytes (no space stripping, no newline->space)
D11 emphasis: single-char '*' LIFO contract, close-preferred tie-break;
    not the CommonMark delimiter-run algorithm
D12 no indented code blocks; an over-indented line is container content
    or paragraph text per §7
D13 no setext headings, no HTML blocks/inline HTML, no autolinks, no
    hard line breaks, no tables, no footnotes, no task lists
```

Anything not listed in this file is not in the grammar and is ordinary
text. CommonMark behavior for such inputs is IRRELEVANT to the benchmark.

## 12. Ambiguity closure (R3 §7 questions answered)

For each construct the contract fixes the one interpretation:

```text
can two implementations differ?      No: dispatch order (§2) is total and
                                     ordered; delimiter tie-breaks are
                                     fixed (§10); label normalization is
                                     algorithmic (§9.1); first-def-wins
                                     is fixed (§9.3).
can an edit change interpretation    Yes — by design — but the resulting
without this contract saying what    interpretation is always defined by
to do?                               §2–§10 (e.g. a newly unclosed fence
                                     swallows following blocks exactly
                                     per §8).
do we depend on unspecified          No: §11 enumerates every deliberate
CommonMark behavior?                 deviation; corpus text avoids
                                     constructs outside the grammar
                                     (CORPUS-v1 recipes use only §3–§10
                                     forms).
```

## 13. Parse algorithm shape (normative summary)

```text
1. line loop with container stack (document, quotes, list items):
   strip container prefix, classify by §2, update stack, emit blocks.
2. reference-definition table built during the SAME single pass, in
   source order (first wins).
3. inline scan (§4, §9, §10) per paragraph/heading/link-text region,
   AFTER the block pass, using the completed definition table —
   resolution is global, never position-dependent.
```

This ordering (blocks -> table -> inlines) matches the H0 anchors
(md4c/pulldown-cmark/comrak all fix it; MECHANISM-SOURCE-MAP H0) and is
required so that `normalize(H0 clean parse)` is well defined.
