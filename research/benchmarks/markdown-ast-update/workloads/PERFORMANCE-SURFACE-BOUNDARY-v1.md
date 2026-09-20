# PERFORMANCE-SURFACE-BOUNDARY-v1 — real workload ≠ controlled scaling workload

Status: **authority boundary contract** (issue #35, PR #38 settlement).
This document freezes a methodological boundary; it grants no gate, freezes
no payload, and authorizes no measurement. It complements — it does not
amend — `protocol/R0-METHODOLOGY.md`, `protocol/R6-REAL-WORKLOAD-AUTHORITY-AMENDMENT.md`,
and `selections/SELECTION-CONTRACT-v1.md`.

## 0. The boundary being frozen

The CORRECTIVE-B selected real workload (36 unique physical files) and the
#22/#31 controlled scaling workload are **two different workloads with two
different roles**. Neither substitutes for the other:

```text
REAL COMPLETE FILE WORKLOAD  !=  CONTROLLED SCALING WORKLOAD
```

Every future performance claim must state which workload (evidence surface)
supports it. A claim supported by neither is unmeasured; a claim supported
by the wrong surface is misattributed.

## 1. The three evidence surfaces

```text
SURFACE R — REAL

    complete selected real Markdown files
    (CORRECTIVE-B selections: REPRESENTATIVE / EXTREMAL /
    SYNTAX_COVERAGE / FULL_DOCUMENT sets, 36 unique files)

    establishes: realism, regime reproduction, external validity,
    real edit-anchor provenance, real tails


SURFACE S — SCALE

    controlled synthetic/generated sweeps
    (#22 CONTROLLED_PROTOCOL_SET shapes, frozen scale points)

    establishes: algorithmic scaling, crossover, cliff localization,
    causal attribution


SURFACE X — BOUNDARY

    grammar/state edge cases (frozen BENCH-GRAMMAR-v1 pathological
    shapes and oracle-defined boundary mutations)

    establishes: cliffs, pathological propagation, failure regimes
```

Asymmetries that are part of this contract:

```text
R does not replace S.
S does not establish real-world prevalence.
X does not establish ordinary frequency.
```

## 2. What the real complete-file workload answers — and does not

### 2.1 SURFACE R answers

```text
What does the frozen candidate population actually contain?

Do mechanism rankings seen in controlled experiments reproduce
on complete real documents?

Which real structures correspond to observed weakness regimes?
```

### 2.2 SURFACE R does NOT answer

```text
What is the asymptotic/scaling law?

At what document size does Hx cross over H0?

How does cost grow with one isolated structural variable?

What is the maximum propagation cliff?

Does an algorithm remain flat when N grows to 16 MiB?
```

Those questions belong to SURFACE S (and its boundary cases to SURFACE X)
under the #22/#31 controlled protocol. This document neither generates nor
executes that protocol.

## 3. Controlled scaling dimensions — N / B / L / K / F

These are the future controlled-sweep dimensions, frozen as **semantic
definitions only**. No sweep, generator, or scale point is created or
executed by this document.

### N — document size

```text
N = total source bytes
```

Detects whole-document/global overhead: full-rebuild scaling, tree/global
index/navigation scaling. A future N sweep holds local edit semantics and
affected block size approximately constant where feasible.

### B — affected block/region size

```text
B = size of the syntactic block or region whose local state
    must be reconsidered
```

Measures block-local reparsing efficiency. A future B sweep holds N
approximately fixed where feasible.

### L — state propagation distance

```text
L = source distance from the edit/restart point until grammar
    state becomes equivalent/stable again, under the frozen
    controlled definition of that equivalence
```

Examples: broken fence closer, container state, forward delimiter state.

```text
L is a controlled semantic/state dimension.

L is NOT automatically H4 convergence_distance.
```

`convergence_distance` as recorded by the H4 horse is horse-specific work
evidence produced under the measurement contract. It may be *compared* to
a controlled L sweep; the two identifiers must never be conflated, because
one is a measured output and the other is an experiment design variable.

### K — structural cardinality

```text
K = relevant block/node/container population
```

For navigation, tree consultation, metadata indexing, candidate lookup.
Example: same N, different number of blocks.

### F — semantic dependency fanout

```text
F = number of semantic consumers/dependents affected by one
    dependency source
```

Example: one reference definition consumed by many reference uses
(F = the count of dependent uses). Measures semantic
dependency maintenance scaling.

## 4. Small real files — explicit misinterpretation warning

A small real file can be valid and important evidence. A 4 KiB real
Markdown file may legitimately represent an observed real structural
regime. But:

```text
H0 ≈ H4 on a 4 KiB real file

must NOT be promoted into:

    "incremental parsing provides no benefit"
```

without controlled scaling evidence (SURFACE S). Likewise, one 841 KiB
real file does not establish behavior at 16 MiB, nor any asymptotic law.
The selected corpus's aggregate size (~2.0 MiB across 36 files; median
6,575 B) is a fact about the real population, not a cap on the questions
the benchmark may ask.

## 5. Scale points

The frozen #22 first-round scale points remain authoritative where
applicable:

```text
64 KiB · 1 MiB · 16 MiB
```

(`protocol/R0-METHODOLOGY.md` §9.) This document does not change them.

Future causal sweeps **may** require intermediate points (e.g.
64 KiB → 256 KiB → 1 MiB → 4 MiB → 16 MiB) — but only under the existing
R0 rule:

```text
observed crossover/cliff → explanatory intermediate samples
```

added as an analysis follow-up, not a rewrite of earlier results. No
intermediate point is added preemptively by this settlement.

## 6. Real transition → controlled scaling mapping (future workflow)

The future pre-performance step maps frozen real transitions onto
controlled dimensions:

```text
REAL TRANSITION
        ↓
candidate mechanism hypothesis
        ↓
controlled dimension
        ↓
frozen sweep
```

Illustrative examples only — **not** a frozen final matrix, and not
proposed vocabulary: the transition names below are placeholders carried
from the settlement task; CORRECTIVE-C owns the taxonomy and may name
transitions differently.

```text
LOCAL_TEXT                    → N / B sweep
FENCE_CLOSE_BREAK             → L sweep
HUGE_PARAGRAPH_LOCAL_EDIT     → B sweep
MANY_BLOCKS_LOCAL_EDIT        → K sweep
REFERENCE_DEFINITION_CHANGE   → F and dependency-distance sweep
```

## 7. PERFORMANCE_MATRIX_FREEZE — reserved future gate

A gate between CORRECTIVE-C and #31 timing is reserved:

```text
CORRECTIVE-C
        ↓
PERFORMANCE_MATRIX_FREEZE        (future; NOT granted here)
        ↓
#31 timing
```

Its future job: take the frozen real transition taxonomy, map each relevant
transition to N/B/L/K/F controlled sweeps, freeze scale points, freeze
controlled generators, freeze result joins. It must run **before** H0-H4
timing is inspected. Suggested gate name when granted:

```text
PERFORMANCE_MATRIX_FREEZE_PASS
```

This settlement does **not** grant it.

## 8. FULL_DOCUMENT_SET role (CORRECTIVE-B scope note)

`FULL_DOCUMENT_SET` is the **complete-real-document macro surface**. It may
later support full parse/state construction, real incremental edits, and
macro sanity checks. It is not the scaling surface, not the asymptotic
surface, and not the worst-case surface.

> Full-document selection maximizes realism and structural diversity
> within the frozen real acquisition universe; it is not designed to
> manufacture large inputs for complexity analysis.

## 9. What this document does not authorize

```text
NO transition taxonomy            (CORRECTIVE-C)
NO canonical edits / payloads     (CORRECTIVE-C)
NO synthetic N/B/L/K/F cases      (PERFORMANCE_MATRIX_FREEZE)
NO H0-H4 timing                   (#31, post-freeze gates)
NO change to the 36-file selection membership
NO change to frozen #22 scale points or shapes
```
