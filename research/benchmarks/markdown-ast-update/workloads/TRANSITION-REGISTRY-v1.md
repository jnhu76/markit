# TRANSITION-REGISTRY-v1

Status: FROZEN by CORRECTIVE-C (PR #39, issue #35). Machine-readable form:
`workloads/payloads/transition-registry-v1.json` (schema
`transition-registry-v1`). This document is the human-readable authority
for the same 23 entries; the JSON artifact is generated from the same
frozen Rust source (`workload-freeze/src/registry.rs`) and the two must
agree (enforced by generation + `verify`).

## 1. Purpose and scope

The registry is the closed vocabulary of edit/transition semantics the
frozen real-file workload exercises. It exists so that:

- every frozen payload names exactly one registry entry and must agree
  with it (field equality + floor-containment; disagreement is
  `INVALID_PAYLOAD`);
- the A6 applicability matrix has a finite, predeclared cell set
  (file × BREAK-side entry × requested position; `exact_restore_leg`
  entries are realized through their BREAK entries, not as cells) with
  no silent gaps;
- workload extension is a REVIEWED registry change, never an ad-hoc
  payload.

The registry is deliberately small. It is not a taxonomy of Markdown
editing; it is the minimal surface that exercises the E1–E6 headline
families plus the G1 table pilot on real files.

## 2. Entry fields

```text
transition_id      frozen identifier ("G0-..." / "G1-...")
grammar_id, lane   BENCH-GRAMMAR-v1/G0 or COMMONMARK-0.31.2+GFM-TABLES-0.29-gfm-v1/G1
syntax_target      anchor kind under the lane reference parse
edit_family_label  E1_LOCAL_TEXT | E2_PARAGRAPH_SPLIT_MERGE | E3_CONTAINER_DEPTH
                   | E4_FENCE_OPEN_CLOSE | E5_INLINE_DELIMITER
                   | E6_REFERENCE_DEFINITION | ATX_HEADING_TOGGLE | TABLE_SEMANTIC
operation_variant  constructor shape
expected_pre/post  FLOOR predicates (document-independent minimums; payloads
                   add derived exact-count predicates at generation time)
restore_policy     none | paired_transition | exact_restore_leg
qualification      g0_strict | g1_semantic_only
coverage_status    active_edit_covered
```

Floor predicates are intentionally weak: they must hold for ANY document
where the transition applies. Exact counts are derived per payload from
the actual pre/post parses and re-proven by TRANSITION-ORACLE-v1.

## 3. The 23 entries

### G0 (BENCH-GRAMMAR-v1; g0_strict; 18 entries)

| transition_id | target | family | operation | restore |
|---|---|---|---|---|
| G0-LOCAL-TEXT-REPLACE-EQ | text | E1_LOCAL_TEXT | same-length 'z' run inside a text run | none |
| G0-PARAGRAPH-SPLIT | paragraph | E2_PARAGRAPH_SPLIT_MERGE | interior space -> blank line | none |
| G0-PARAGRAPH-MERGE | paragraph | E2_PARAGRAPH_SPLIT_MERGE | blank line -> space (merge) | none |
| G0-ATX-TO-PARAGRAPH | heading_atx | ATX_HEADING_TOGGLE | remove '#'+space | paired |
| G0-PARAGRAPH-TO-ATX | heading_atx | ATX_HEADING_TOGGLE | insert '#'+space | exact_restore_leg |
| G0-LIST-ITEM-INDENT | list_item | E3_CONTAINER_DEPTH | +2 spaces on a sibling-adjacent marker line | paired |
| G0-LIST-ITEM-DEDENT | list_item | E3_CONTAINER_DEPTH | remove the indent | exact_restore_leg |
| G0-BQ-NEST-LINE | block_quote | E3_CONTAINER_DEPTH | insert '>' before a quote marker | paired |
| G0-BQ-UNNEST-LINE | block_quote | E3_CONTAINER_DEPTH | remove the inserted '>' | exact_restore_leg |
| G0-FENCE-CLOSER-REMOVE | code_block_fenced | E4_FENCE_OPEN_CLOSE | delete the real closer run (fence -> EOF) | paired |
| G0-FENCE-CLOSER-RESTORE | code_block_fenced | E4_FENCE_OPEN_CLOSE | re-insert the closer | exact_restore_leg |
| G0-EMPH-DELIM-BREAK | emphasis | E5_INLINE_DELIMITER | delete the closing '*' | paired |
| G0-EMPH-DELIM-RESTORE | emphasis | E5_INLINE_DELIMITER | re-insert the '*' | exact_restore_leg |
| G0-CODESPAN-DELIM-BREAK | code_span | E5_INLINE_DELIMITER | delete one opening backtick | paired |
| G0-CODESPAN-DELIM-RESTORE | code_span | E5_INLINE_DELIMITER | re-insert the backtick | exact_restore_leg |
| G0-REFDEF-REMOVE | reference_definition | E6_REFERENCE_DEFINITION | delete a USED definition line | paired |
| G0-REFDEF-RESTORE | reference_definition | E6_REFERENCE_DEFINITION | re-insert the line (uses resolve) | exact_restore_leg |
| G0-LINK-DEST-BREAK | link_inline | E5_INLINE_DELIMITER | space into destination (G0 §9.2 break) | none |

Notes frozen here:

- **Container-depth truth** (E3): the payload must increase the LOCAL
  container depth around the anchor (window over the anchor span), not
  merely a document-global maximum; trivia bytes (`>`, indent spaces)
  never move the text fingerprint, so the proof is the derived
  container kind-count change.
- **List indent precondition**: a marker line whose immediately
  preceding line is a sibling marker at the same indent (G0 §7
  tight-only lists: anything else does not nest).
- **Reference-dependency truth** (E6): the definition must have a
  resolved use; the LinkReference recognized-count drop is the proof.
- **Destination edit is the break form**: a pure destination byte
  replacement leaves the link a link and is invisible to every
  PREDICATE-v1 predicate; the registry's destination edit inserts a
  space so the construct fails into literal text (G0 §9.2).

### G1 (COMMONMARK-0.31.2+GFM-TABLES-0.29-gfm-v1; g1_semantic_only; 5 entries)

| transition_id | target | operation | restore |
|---|---|---|---|
| G1-TABLE-DELIM-BREAK | table | first '-' of the delimiter row -> 'n' (valid table -> non-table) | paired |
| G1-TABLE-DELIM-RESTORE | table | restore the '-' | exact_restore_leg |
| G1-TABLE-CELL-EDIT | table_cell | same-length 'z' in a real cell | none |
| G1-TABLE-ROW-DELETE | table_row | delete a real body-row line | none |
| G1-TABLE-HEADER-PIPE-REMOVE | table | delete one header CELL-SEPARATOR pipe (never a leading/trailing boundary pipe: that would leave a valid table) | none |

G1 payloads are SEMANTIC_ONLY and NOT_HORSE_QUALIFIED: they validate the
G1 lane semantics (pulldown-cmark 0.13.4 ENABLE_TABLES reference) and are
NEVER dispatched into H0–H4.

## 4. Agreement rule (payload ↔ registry)

A payload agrees with its entry iff:

1. grammar_id, syntax_target, expected_transition, edit_family,
   operation_variant match the entry exactly; and
2. the payload's declared pre/post predicates CONTAIN the entry's floor
   predicates (set containment); derived exact-count predicates may
   strengthen but never contradict a floor.

Violation at generation or verify time is `INVALID_PAYLOAD` (fail
closed).

## 5. Extension rule

Adding a transition requires: a new registry entry with floor predicates,
a deterministic constructor, and regeneration. There is no path from a
frozen payload to a new payload ("POST_HOC_* work cannot silently mutate
the workload").
