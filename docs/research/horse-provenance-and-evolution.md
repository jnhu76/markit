# Horse provenance and evolution axes

Status: **DURABLE RESEARCH NOTE / NON-NORMATIVE INPUT TO CANONICAL SYNTHESIS**

This note preserves two pieces of research context that are easy to lose when reading only the later Campaign-2 / #50 synthesis:

1. where H0–H4 came from;
2. how the post-H4 research program intends to measure an intentionally incomplete Horse-A before authorizing later improvements.

This file is **not** the Horse-A mechanism-freeze authority. Current algorithm design lives in issue #55; the evolution roadmap lives in issue #53; the broader prior-art reading map lives in issue #56. The first independent design review is preserved in `docs/research/reviews/horse-a-first-design-review-2026-09-24.md`.

The detailed prior-art authority remains:

- `research/benchmarks/markdown-ast-update/prior-art/README.md`
- `research/benchmarks/markdown-ast-update/prior-art/MECHANISM-SOURCE-MAP.md`
- `research/benchmarks/markdown-ast-update/prior-art/MECHANISM-MATRIX.md`
- the per-project extraction records under `prior-art/`

## Horse provenance

| Horse | Prior-art anchor | Mechanism abstraction tested by Markit |
|---|---|---|
| **H0 — FULL_REBUILD** | MD4C / pulldown-cmark / Comrak | clean full-document parse/build; no old parse state is consulted for incremental computation |
| **H1 — BLOCK_LOCAL_REPARSE** | `mizchi/markdown.mbt` | top-level damage mapping → bounded region reparse → splice/fallback |
| **H2 — FRAGMENT_REUSE** | Lezer (`@lezer/common`, `@lezer/lr`, `@lezer/markdown`) | fragment mapping + context-gated structural reuse / rematerialization |
| **H3 — OLD_TREE_SUBTREE_REUSE** | Tree-sitter, with Wagner & Graham / Swift overlap | edited old tree + position/state-compatible subtree consultation/reuse |
| **H4 — RESTART_CONVERGENCE** | Wagner & Graham + Swift incremental syntax, informed by Tree-sitter / Lezer convergence observations | restart before damage → forward validation → continuation equivalence → stable suffix reuse |

Required interpretation:

```text
inspired mechanism model
!= upstream reproduction
!= upstream performance claim
```

Therefore:

- H1 results are not `mizchi/markdown.mbt` product-performance measurements.
- H2 results are not Lezer benchmark results.
- H3 results are not claims about Tree-sitter product performance.
- H4 is a multi-source mechanism abstraction, not a port of one upstream implementation.
- Wagner & Graham, Swift, Tree-sitter and Lezer do not use one identical convergence authority; the compact table is provenance, not an assertion that their algorithms are equivalent.

The purpose of H0–H4 was to isolate mechanism families under one semantic core, oracle, payload/edit contract and measurement substrate.

## Post-H4 research object

The next research object is not a new Markdown grammar/parser. The shared grammar and semantic core remain the experimental semantic authority.

The new layer is:

```text
Markdown grammar / semantic core
        ↓
fresh complete AST / semantic result
        ↓
retained representation
        ↓ edit
incremental update
        ↓
new complete ready AST
```

The question is narrower than “build the best Markdown AST”:

> Under a fixed safe local edit with a preservable semantic environment and bounded restart/propagation, can the retained representation avoid work proportional to unaffected retained state?

This is the representation-locality question exposed by #50 after H4 had already achieved parser locality on the controlled local witness.

## Horse-A is still a candidate, not a frozen identity

The current design candidate in #55 intentionally tests structural locality with a small state budget.

The first independent review concluded:

```text
HORSE_A_RESEARCH_OBJECT = CORRECT
WEIGHTED_SEQUENCE_CONTRACT = ACCEPT
AVL_FIRST_REALIZATION = ACCEPT

MECHANISM_IDENTITY_READY_TO_FREEZE = NO
READY_FOR_DATA_MODEL_DESIGN = YES
READY_FOR_IMPLEMENTATION = NO
```

Five design obligations must be closed before mechanism freeze:

```text
A-01 exact source coverage / affinity contract
A-02 root restart certificate support + invalidation contract
A-03 block parse → facts → environment decision → materialization order
A-04 staging / commit frontier for unique mutable ownership
A-05 scoped R1–R6 wording
```

Do not read this note's MVP summary as stronger authority than #55 and its review record.

## Current Horse-A MVP boundary

The candidate currently keeps:

```text
byte-weighted ordered retained sequence
owner-relative spans
eager complete semantic payload
certified root-level restart/convergence
same-target full rebuild
local structural splice
single externally current version
mutable retained representation
```

The following are intentionally deferred from the first identity:

```text
nested continuation checkpoints
consumer dependency postings
winner index
stable cross-edit IDs / persistent locator map
COW / historical roots / snapshot readers
packed/chunked wide-node layout
advanced calibrated selector
global subtree/fragment reuse index
```

Interpretation:

```text
NOT IN HORSE-A MVP
!=
PERMANENTLY FORBIDDEN
```

A deferred mechanism may enter a later variant only after measured evidence shows that it addresses a real Horse-A weakness and earns its construction, memory, update and retirement cost.

## Three independently attributable evolution directions

These are research directions that can be investigated with controlled attribution. They are **not proven fully orthogonal knobs**, and their benefits must not be added together without a new controlled comparison.

```text
                         Horse-A
                           |
              structural locality first
                           |
          +----------------+----------------+
          |                |                |
          v                v                v
        A-R             Horse-B            A-P
 restart locality   semantic locality   layout locality
```

### A-R — restart / required retention granularity

Known weakness:

- root-only certified restart can reparse a long distance inside a huge list/quote/fence/paragraph or other coarse Owner.

Question:

> Can finer continuation state shorten restart/propagation enough to justify its persistent state and maintenance cost without recreating H4's global checkpoint tax?

Prior-art families include Tree-sitter state compatibility, Lezer context validity, Swift checkpoints and Wagner/Graham restart/convergence theory.

Important coupling:

- a finer restart point may not help if the retained Owner/payload granularity still forces rebuilding the entire huge Owner. A-R may therefore require a later controlled retention-granularity change.

### Horse-B — semantic dependency locality

Known weakness:

- Horse-A conservatively selects same-target full rebuild whenever complete replacement-region definition facts differ or preservation cannot be proven.
- a shadowed definition edit can therefore rebuild even when the effective winner did not change.

Suggested later decomposition:

```text
B1: effective environment
    normalized label
      → ordered definition occurrences
      → effective winner/value

B2: selective semantic repair
    changed semantic answer
      → affected consumer Owners
      → include unresolved lookup attempts
```

Important coupling:

- B2 may require additional owner addressing and retained rematerialization input; those costs are part of the experiment, not free infrastructure.

### A-P — packed/layout locality

Known weakness:

- the accepted first AVL realization may pay allocation, pointer-chasing, cache and construction costs even if retained-state work is structurally local.

Question:

> Holding restart, semantics and ownership policy fixed, does a wider/chunked weighted sequence improve economics without reintroducing global maintenance?

Candidate families include B-tree-like weighted sequences, chunked vector/tree hybrids, RRB-style wide relaxed trees, measured/finger-tree ideas and rope-style split/concat.

Important coupling:

- packing changes relocation, metadata aggregation and boundary maintenance costs; it must be measured as a distinct representation variant.

## Correct research order after the first design review

```text
1. close A-01..A-05 through data-model design
2. revise #55 / durable design artifact
3. fresh independent review
4. only if approved: freeze Horse-A mechanism identity
5. freeze a failure-first structural-locality experiment
6. implement Horse-A with W-A1/W-A2/W-A3 intact
7. correctness first
8. work-locality kill gate
9. if work passes: timing / allocation / construction / memory
10. then broader H0–H4–Horse-A comparison
11. diagnose the actual limiting weakness
12. authorize one later axis only if evidence earns it
```

The first narrow witness remains #50-shaped in intent:

```text
N = 128 KiB / 1 MiB / 16 MiB
fixed safe local paragraph edit
no definitions / references / fences
H0 / H4 / Horse-A
```

The first decision is structural work, not latency. A candidate that still touches unaffected retained records proportional to M does not pass simply because it is faster on one machine.

## Reading map

The detailed “why read this?” map now lives in issue #56:

```text
HORSE-A-PRIOR-ART-GAP-1
```

It is organized by the actual research gaps rather than by chronology:

```text
Horse-A core  → representation / coordinate locality
A-R           → restart / continuation locality
Horse-B       → semantic dependency locality
A-P           → sequence/layout locality
```

That issue is a reading/interpretation map, not mechanism authority.
