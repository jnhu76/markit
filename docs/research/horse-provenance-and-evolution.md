# Horse provenance and evolution axes

Status: **DURABLE RESEARCH NOTE / NON-NORMATIVE INPUT TO CANONICAL SYNTHESIS**

This note preserves three pieces of research context that are easy to lose when reading only the later Campaign-2 / #50 synthesis:

1. where H0–H4 came from;
2. why Horse-A is intentionally incomplete;
3. how later restart / semantic / layout improvements remain evidence-triggered rather than silently folded into the first horse.

This file is **not** the Horse-A mechanism-freeze authority.

Current roles are:

```text
#53  = evolution roadmap / later-axis evidence triggers
#55  = current Horse-A logical mechanism candidate
#56  = prior-art reading / interpretation map; non-authoritative
PR #54 branch = durable provenance + review/design artifacts
canonical synthesis = evidence / lessons / provenance / next question
```

The first independent design review is preserved at:

`docs/research/reviews/horse-a-first-design-review-2026-09-24.md`

The current concrete logical-data-model candidate is preserved as a fixed review bundle at:

`docs/research/horse-a-logical-data-model/`

Its README defines the review order and gate. The live #55 body carries the same logical candidate as one continuous document.

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

## Horse-A gate after logical-data-model design

The first independent review originally concluded:

```text
P0 = 0
P1 = 5
P2 = 4
P3 = 2

MECHANISM_IDENTITY_READY_TO_FREEZE = NO
READY_FOR_DATA_MODEL_DESIGN = YES
READY_FOR_IMPLEMENTATION = NO
```

It required closure of:

```text
A-01 exact source coverage / affinity contract
A-02 root restart certificate support + invalidation contract
A-03 block parse → facts → environment decision → materialization order
A-04 staging / commit frontier for unique mutable ownership
A-05 scoped R1–R6 wording
```

The new logical-data-model candidate in #55 and `docs/research/horse-a-logical-data-model/` now provides explicit contracts for all five. Its **author-side assessment** is:

```text
COVERAGE_CONTRACT = PASS
READYDOCUMENT_CONTRACT = PASS
OWNERSEQ_CONTRACT = PASS
OWNER_CONTRACT = PASS
ASTPAYLOAD_CONTRACT = PASS
REFTABLE_RELATION = PASS
RESTART_CERTIFICATE = PASS
CONVERGENCE_CONTRACT = PASS
STAGING_COMMIT_MODEL = PASS
FULL_BUILD_EQUIVALENCE = PASS
R1_R6_MAPPING = PASS

P0 = 0
P1 = 0
P2 = 0
P3 = 0

MECHANISM_IDENTITY_READY_TO_FREEZE = YES
READY_TO_FREEZE_FALSIFICATION_CONTRACT = YES
READY_FOR_IMPLEMENTATION = NO
```

These YES values mean only:

> the candidate is now concrete enough to submit to a **fresh independent mechanism-freeze review** and to design/review a falsification contract if that review authorizes it.

They do **not** mean:

```text
MECHANISM IDENTITY FROZEN = YES
FALSIFICATION CONTRACT FROZEN = YES
IMPLEMENTATION AUTHORIZED = YES
```

Current live authority remains:

```text
FRESH INDEPENDENT REVIEW = REQUIRED / NOT YET RECORDED
MECHANISM IDENTITY FROZEN = NO
FALSIFICATION CONTRACT FROZEN = NO
HORSE-A IMPLEMENTATION AUTHORIZED = NO
HORSE-A PERFORMANCE COLLECTION AUTHORIZED = NO
```

## What changed in the concrete candidate

The data-model pass deliberately resolved the five review obligations without adding the deferred mechanisms.

Key choices now fixed in the candidate include:

```text
coverage:
  split by physical top-level-block first-line starts
  gaps / blank separators belong to the left Owner
  first Owner absorbs leading trivia
  last Owner absorbs trailing trivia
  empty source = empty OwnerSeq
  all SPACES/LF-only source = one TriviaOnly Owner

retention:
  one complete top-level syntax subtree per Owner
  giant container remains one Owner in Horse-A

restart:
  only parser-derived pre-EOF root-blank-barrier evidence may certify a restart cut
  ContextKey::default() alone is insufficient
  ordinary EOF completion is not a persistent restart certificate
  a conservative unchanged left guard block is included in the replacement region

semantic preservation:
  parse complete replacement block region
  extract exact source-ordered definition facts
  compare old/new replacement facts
  only if equal, eager-materialize fresh payload under the retained RefTable
  otherwise select same-target full build

commit:
  all fallible parse / facts / materialization / allocation / resource preparation before frontier
  after frontier: structural ownership transfer/install + retirement only
  no algorithm fallback after frontier

representation:
  first realization = one-record-per-node mutable AVL weighted sequence
  this is a falsifiable realization, not an optimality claim
```

The candidate also fixes the cost vocabulary so structural locality cannot hide parser propagation, semantic lookup, removed payload, retirement or full-build costs.

## Current Horse-A MVP boundary

The candidate keeps:

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

The following remain intentionally deferred from the first identity:

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

- root-only certified restart plus a coarse top-level Owner can require long forward parsing and payload rebuild inside a huge list/quote/fence/paragraph.

Question:

> Can finer continuation state shorten restart/propagation enough to justify its persistent state and maintenance cost without recreating H4's global checkpoint tax?

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

Important coupling:

- packing changes relocation, metadata aggregation and boundary maintenance costs; it must be measured as a distinct representation variant.

## Correct research order now

```text
1. concrete logical-data-model candidate recorded in #55
2. fixed review bundle recorded on PR #54 branch
3. fresh independent review of that fixed candidate
4. only if PASS: explicit Horse-A mechanism-freeze decision
5. freeze a failure-first structural-locality falsification contract
6. only after that contract is frozen: implement Horse-A with W-A1/W-A2/W-A3 intact
7. correctness first
8. work-locality kill gate
9. if work passes: timing / allocation / construction / memory
10. broader H0–H4–Horse-A comparison
11. diagnose the actual limiting weakness
12. authorize one later axis only if evidence earns it
```

The fresh review should explicitly attack at least:

```text
guard-block coverage closure
restart-certificate induction / migration
EOF and empty/all-trivia exceptions
old/new replacement-region fact completeness
no-fail split/join/retirement resource preparation after commit frontier
same-target full-builder equivalence
counter units / parser-observation boundary
```

The first narrow falsification witness remains #50-shaped in intent, but its exact workload, thresholds, counters and result-to-action rules are **not frozen by this note**.

## Reading map

The detailed “why read this?” map lives in issue #56:

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
