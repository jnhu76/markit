# NORMALIZED-RESULT-v1 — frozen semantic vocabulary and query contract (R3)

Status: **R3 FREEZE CANDIDATE — READY_FOR_ADVERSARIAL_R3_REVIEW**
Authority: `protocol/R0-METHODOLOGY.md` §5 (normalized result contract,
FROZEN — not restated or amended here) + this file (the concrete
vocabulary R3 freezes). Single owner of the node vocabulary, span
conventions, and the NODE_PATH_AT query contract.

Correctness stays exactly R0's law:

```text
normalize(H1/H2/H3/H4 update result) == normalize(H0 clean full parse(post-edit source))
```

Compared: node kind, ordered topology, spans, the value fields below, and
reference-definition facts. NEVER compared: pointer/NodeId/fragment/Arc
identity, reuse counts, allocation addresses, mechanism state.

---

## 1. Node vocabulary (complete; closed)

Exactly these kinds exist. A horse may use richer native trees, but its
normalized result contains only these.

```text
Document             root; exactly one per result
Paragraph
Heading              field: level (1..6)
BlockQuote
List
ListItem             field: marker ("-" or "*")
FencedCode           fields: info (string, may be empty),
                             content span (start:end, may be zero-length)
Text
Emphasis
CodeSpan
Link                 field: destination (string)
ReferenceLink        fields: label (normalized), destination (resolved)
ReferenceDefinition  fields: label (normalized), destination (string)
```

No other kinds, no attributes beyond the fields listed, no error nodes,
no whitespace/trivia nodes. Node kinds correspond 1:1 to BENCH-GRAMMAR-v1
§3–§10. (Feasibility note, not semantics: Lezer's
Document/ATXHeading/FencedCode/Blockquote/BulletList/Emphasis/InlineCode/
Link/LinkReference and tree-sitter-markdown's
atx_heading/fenced_code_block/block_quote/list/list_item map onto this
vocabulary; Lezer intentionally omits reference validation upstream, so a
Lezer-anchored horse must ADD the resolution pass to produce the
`destination` fact — that is required by R0 equivalence, and its cost is
a measurement subject, not a grammar choice.)

## 2. Span conventions (frozen)

```text
coordinates     UTF-8 bytes, relative to the COMPLETE document
interval        half-open [start, end); end is exclusive
validity        0 <= start <= end <= len(document); both bounds on UTF-8
                char boundaries of the post-edit source
document        Document span is exactly [0, len)
block spans     start at the first NON-WHITESPACE byte of the construct's
                first line after the parent's consumed prefix (leading
                indentation is never inside any span); end at the last
                byte of the construct's last line, EXCLUDING that line's
                terminator
list/listitem   ListItem span starts at the marker byte; List span starts
                at its first item's marker byte
emphasis        span includes BOTH delimiter bytes
code span       span includes BOTH backtick runs
link            span includes '[', ']' and '(' ... ')' (or '[label]')
gaps            bytes between child spans inside a parent are trivia
                (whitespace/newlines/delimiter bytes). Trivia is
                represented ONLY by span gaps + the source; it has no
                nodes. Text content is the source slice of the Text span;
                a Text run is maximal within the parent's content region
                (container prefixes split runs).
fenced code     span covers the opener's first backtick through the
                closer's last backtick byte; an UNCLOSED fence is the one
                span permitted to include a final LF: it ends at
                len(document). The `content` field is the raw byte
                interval between the opener line's terminator and the
                closer line's first byte (to EOF when unclosed),
                unstripped, and may include container-prefix bytes; it
                may be zero-length. Body bytes are the `content` field,
                NOT a child node.
zero-length     normalized node spans are NEVER zero-length; the only
                zero-length interval in the vocabulary is the FencedCode
                `content` field (empty body)
children        document order (non-decreasing starts; children's spans
                lie inside the parent's span)
```

## 3. Reference-definition facts

The normalized result of a document contains the ordered sequence of
`ReferenceDefinition` nodes (source order, duplicates included, first
lookup-wins semantics per BENCH-GRAMMAR-v1 §9.3). ReferenceLink nodes
carry their RESOLVED destination as a required field; the resolution is
part of correctness equivalence. This is what makes SEMANTIC_DEPENDENCY
mutations visible: changing or deleting a definition changes the
destination field of every dependent ReferenceLink (or removes the node
entirely on fallback).

## 4. QUERY — NODE_PATH_AT (frozen contract)

R0 froze `QUERY` as an operation but left its semantics open. Issue #22
defines no more specific contract (checked 2026-09-17); R9 names the
surface "offset -> enclosing region". Frozen here, one minimal,
representation-independent contract:

```text
name       NODE_PATH_AT(offset)
input      one UTF-8 byte offset with 0 <= offset <= len(source)
output     the ordered path root -> deepest node of the normalized tree
           where every node n on the path satisfies
             n.start <= offset < n.end
           and each consecutive pair is parent/child.
           Every reported node carries: kind, [start, end).
containment  half-open: start <= offset < end. An offset exactly at a
             node's end is NOT contained (it belongs to whatever follows;
             at whitespace gaps the path is just [Document]).
EOF        offset == len(source): containment fails for every node
           (end is exclusive), so the frozen result is the single-node
           path [Document] — the only end-inclusive special case.
zero-length none exist in the vocabulary (§2), so no zero-length rule is
           needed beyond the FencedCode content field, which is not a
           node and never appears in a path.
overlap    child spans nest strictly inside parents; among siblings,
           spans never overlap, so "deepest containing" is always unique.
correctness  defined over the NORMALIZED tree only. Whether a mechanism
           answers by traversal, index, checkpoint, or fragment metadata
           is a later implementation/parity decision (R0 §7.3 fairness
           rule applies to whichever it chooses). NO horse-specific index
           is defined here.
```

First-round QUERY cases (see `cases/CASE-MATRIX-v1.md`): one QUERY case
per corpus, answered at three frozen anchors of the OLD source:
EARLY = floor(N/4), MIDDLE = floor(N/2), LATE = floor(3N/4), each snapped
DOWN to the nearest UTF-8 char boundary. QUERY carries no edit
(CaseKeyV1 rejects edit fields for `query`), so post-edit querying is
explicitly DEFERRED (later stage; R9), not defined here.

## 5. Golden semantic fixtures — format and role

`grammar/fixtures/*.toml` prove BENCH-GRAMMAR-v1 + this vocabulary are
unambiguous BEFORE any H0 implementation exists. They are hand-authored
authority: the expected tree in a fixture is TRUE BY DECLARATION under
this document pair. We ship 43 focused fixtures (target band 20–40 —
the §9 minimum-coverage list plus one fixture per pinned rule, with zero
deliberate redundancy), including CJK / multibyte UTF-8 in several,
covering every §1 kind and every §11 deviation that a fixture can
exhibit.

File format (single owner: this section):

```toml
schema        = "r3-fixture-v1"        # literal
id            = "F001"                 # unique, F + 3 digits
name          = "single paragraph"
covers        = ["paragraph", "text"]  # subset of COVER_TAGS below
source        = '''
...exact source bytes; TOML multiline-literal: the newline right after
the opening delimiter is trimmed, the final newline before the closing
delimiter is KEPT...
'''
source_sha256 = "<hex sha256 of the exact source bytes>"
expected_tree = '''
(Document 0 12
  (Paragraph 0 11
    (Text 0 11)))
'''
```

Tree syntax: `(Kind start end)` with optional `key=value` fields
(`level`, `marker`, `info`, `content` as `start:end`, `label`,
`destination`) between end and children; children indented one space per
depth. Kinds are the §1 set; spans follow §2.

```text
COVER_TAGS = paragraph text heading blockquote list listitem
             fenced-code unclosed-fence emphasis code-span
             inline-link reference-link reference-definition
             reference-fanout mixed utf8 cjk emoji deviation
```

Machine checks (scripts/verify-r3.sh; deliberately NOT a re-parse —
semantic truth is the hand-authored tree): unique ids, schema literal,
`covers` ⊆ COVER_TAGS, sha256(source) matches, tree parses, kinds/fields
valid, spans within [0, len(source)] and on char boundaries, root is
`(Document 0 len)`, parent containment, sibling order non-overlapping,
block spans exclude the trailing LF. These are internal-consistency
invariants of the artifacts, not Markdown semantics.
