# SEMANTIC-INTERFERENCE-CONTRACT-v1 — requirements contract (NOT IMPLEMENTED)

Status: **DOCUMENTATION ONLY — DEFERRED ATTRIBUTION CONTRACT**
Authority: issue #35 Corrective-1 §6 (separate semantic interference from
mechanism work) + `protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md` §5
(semantic interference is an attribution model, not a naive AST diff) + the
#35 Current Authority Notice ("semantic-interference quantification is a
later attribution contract; CORRECTIVE-A must not implement it as a naive
AST diff").

```text
CORRECTIVE-A IMPLEMENTS NOTHING IN THIS FILE.
There is no semantic-interference metric, field, or code path in the
semantics crate. This document freezes the REQUIREMENTS a future
quantitative metric must satisfy before it may be promoted, so that the
question is not answered by accident later.
```

---

## 0. Why this is deferred

The research chain is:

```text
EDIT
  -> semantic effect under the frozen grammar lane
  -> mechanism work (H0-H4)
  -> latency / memory
  -> causal attribution
```

CORRECTIVE-A closes the leftmost link (lanes, eligibility, oracle, predicates,
profiler semantics). The second link is a separate experimental object with
its own contract, and #35 §6 makes the separation a core methodology rule:

```text
horse-independent ground truth   semantic interference
                                 (from clean pre vs clean post state under
                                  the same frozen lane)
horse-dependent ground truth     mechanism work
                                 (collected separately per H0-H4)
DO NOT DERIVE ONE FROM THE OTHER.
```

## 1. Candidate facts (horse-independent side)

From #35 §6, to be defined — not measured — by the future contract:

```text
syntax_changed_ranges
changed_syntax_bytes
changed_blocks
changed_nodes
furthest_affected_offset
forward-propagation span
dependency-affected consumers / fanout
```

The horse-dependent side is likewise listed there and belongs to #31:

```text
bytes inspected
blocks reparsed
nodes rebuilt
nodes reused
metadata/range records touched
fallback
restart distance
convergence distance
mechanism-specific consultations where defined
```

## 2. Requirements before any quantity is promoted

Every one of these must be frozen in the future contract, explicitly, before
a number is published:

```text
1. pre/post coordinate mapping through the canonical edit
   (how a pre-state position is expressed in post-state coordinates, and
    vice versa, for arbitrary edits — not only same-length replacements)
2. node/content correspondence rule
   (when is a post node "the same node" as a pre node: kind + position +
    content + ancestry; the rule decides everything downstream)
3. change classes
   CONTENT_CHANGE / STRUCTURE_CHANGE / DEPENDENCY_RESOLUTION_CHANGE /
   POSITION_SHIFT_ONLY — the distinctions the metric must preserve
4. pure span/position shift handling
   a source insertion near the document start must NOT turn every shifted
   span into semantic change merely because absolute offsets moved
5. ancestor double-count policy
   (whether an affected child also marks all ancestors, and how the union
    avoids counting the same bytes twice)
6. non-contiguous affected-range representation
   (a set of ranges, not one interval)
7. union size vs enclosing span
   a distant reference-definition edit may affect a non-contiguous set of
   consumers; the union of affected ranges and the enclosing span that
   contains them are different quantities and must never be conflated
```

Three further rules that are already frozen:

```text
- a same-length text replacement must not disappear merely because node
  kind/topology/spans remained stable;
- any resulting quantity must be described as
  "observed semantic impact under the frozen grammar and correspondence rule"
  — never as the parser's mathematically minimal required work;
- internal parser-state propagation is a SEPARATE concept: if studied it
  needs an independently frozen reference-state definition and must not be
  conflated with H4 restart/convergence distance.
```

## 3. Relationship to what CORRECTIVE-A does implement

CORRECTIVE-A provides the substrate a future metric would consume, and
deliberately nothing more:

```text
available now
  lane identity + exact semantics          grammar/GRAMMAR-LANES-v1.md
  lane oracle + eligibility (pre/post)     grammar/TRANSITION-ORACLE-v1.md
  syntax facts with auditable spans        workloads/profiles/PROFILER-CONTRACT-v1.md
  minimal change classification
    CONTENT_CHANGE / STRUCTURE_CHANGE      transition reports, with
    (+ two RESERVED classes)               change_class_note stating what
                                           it is NOT

NOT available, by design
  any semantic-interference quantity
  any correspondence rule between pre and post nodes
  any union/span aggregation of affected ranges
  any attribution of latency to semantic impact
```

`TransitionReport.change_class` is an observed effect under PREDICATE-v1. It
is explicitly not interference: every report carries a note saying so, and
`DEPENDENCY_RESOLUTION_CHANGE` / `POSITION_SHIFT_ONLY` remain RESERVED
precisely because deriving them requires the correspondence rule this
contract has not yet frozen.

## 4. Gate

This contract is not part of the G1-G8 freeze gates of #35 §10. Semantic
interference is required for the **attribution** stage (#31 execution, #33
synthesis), not for `REAL_WORKLOAD_FREEZE_PASS` or
`CORE_REAL_WORKLOAD_FREEZE_PASS`. It must be frozen before any published
claim attributes performance differences to semantic impact.
