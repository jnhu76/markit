# Horse provenance and evolution axes

Status: **DURABLE RESEARCH NOTE / INPUT TO CANONICAL SYNTHESIS**

This note makes two things explicit that are easy to lose when reading only the later Campaign-2 / #50 synthesis:

1. where H0-H4 came from;
2. how the next Horse-A should deliberately retain three weaknesses so that later improvements remain causally separable.

The detailed prior-art authority remains:

- `research/benchmarks/markdown-ast-update/prior-art/README.md`
- `research/benchmarks/markdown-ast-update/prior-art/MECHANISM-SOURCE-MAP.md`
- the per-project extraction records under `prior-art/`

## Horse provenance

| Horse | Prior-art anchor | Mechanism abstraction tested by Markit |
|---|---|---|
| **H0 — FULL_REBUILD** | MD4C / pulldown-cmark / Comrak | clean full-document parse/build with no retained cross-edit state |
| **H1 — BLOCK_LOCAL_REPARSE** | `mizchi/markdown.mbt` | top-level damage mapping → bounded region reparse → splice/fallback |
| **H2 — FRAGMENT_REUSE** | Lezer (`@lezer/common`, `@lezer/lr`, `@lezer/markdown`) | fragment mapping + context-gated structural reuse / rematerialization |
| **H3 — OLD_TREE_SUBTREE_REUSE** | Tree-sitter, with Wagner & Graham / Swift overlap | edited old tree + position/state-compatible subtree consultation/reuse |
| **H4 — RESTART_CONVERGENCE** | Wagner & Graham + Swift incremental syntax, informed by Tree-sitter / Lezer convergence conditions | restart before damage → forward validation → continuation equivalence → stable suffix reuse |

Interpretation rule:

```text
inspired mechanism model
!= upstream reproduction
!= upstream performance claim
```

Therefore:

- H2 results are not Lezer benchmark results.
- H3 results are not claims about Tree-sitter's product performance.
- H4 is a family-level mechanism abstraction, not a port of any one upstream implementation.

The purpose of H0-H4 was to isolate mechanism families under one semantic core, oracle, payload/edit contract and measurement substrate.

## Horse-A: test the imperfect structural-locality horse first

The next candidate should not begin as an attempted final Markit architecture.

Its first job is narrower:

> For a fixed safe local edit with unchanged semantic environment, can the retained representation remain local as total document size grows, instead of visiting or rebuilding Theta(M) unaffected records?

The proposed first identity is the reviewed `ROOT_CERTIFIED_MUTABLE_SEQUENCE_A_V1` family:

```text
included:
  single current version / unique ownership
  byte-weighted ordered retained sequence
  owner-relative spans
  complete eager AST payload
  root-certified restart/convergence
  conservative definition-environment certification
  same-target full rebuild fallback

explicitly excluded from Horse-A:
  nested continuation checkpoints
  semantic consumer postings
  stable cross-edit IDs / locator map
  COW / snapshot history
  packed/wide-node layout
  advanced cost selector
```

A mutable AVL weighted sequence is only the proposed first experimental realization. The evidence supports the required weighted sequence operations and locality target; it does **not** prove AVL is the optimal layout.

## Three deliberate weaknesses become independent research axes

```text
                         Horse-A
                           |
              structural locality only
                           |
          +----------------+----------------+
          |                |                |
          v                v                v
        A-R             Horse-B            A-P
 restart locality   semantic locality   layout locality
          |                |                |
 Tree-sitter /       Reps / attribute    B-tree / RRB /
 Lezer / Swift       evaluation          chunk / measured seq
```

### A-R — restart locality

Known Horse-A weakness:

- restart only at certified root-level safe boundaries;
- a huge list/quote/fence/paragraph owner may therefore require long forward parsing.

Question:

> Can finer continuation state shorten B/K/container propagation without recreating H4's O(M) checkpoint maintenance tax?

Prior-art families to revisit:

- Tree-sitter parser/external-scanner state compatibility;
- Lezer context hashes / fragment validity;
- Swift parser checkpoints and reuse predicates;
- Wagner & Graham restart/convergence theory.

This axis is added only if Horse-A measurements show restart granularity is a material weakness.

### Horse-B — semantic dependency locality

Known Horse-A weakness:

- exact local definition facts differ → conservative same-target full rebuild;
- a shadowed definition edit may therefore rebuild even when the effective winner is unchanged.

Suggested separation:

```text
B1: effective environment tracking
    normalized label
      -> ordered definition occurrences
      -> effective winner/value

B2: selective consumer repair
    changed effective label
      -> affected consumer owners
      -> include unresolved lookup attempts
```

Prior-art family:

- incremental attribute evaluation / incremental context-dependent analysis (Reps, Teitelbaum, Demers and related work).

Do not add B1 and B2 to Horse-A before the structural-locality hypothesis is measured.

### A-P — packed/layout locality

Known Horse-A weakness:

- one AVL node per owner may create allocation, pointer-chasing, cache and construction costs even if structural work is logarithmic/local.

Question:

> Holding restart, semantics and ownership constant, does a wide/chunked weighted sequence improve economics without reintroducing global maintenance?

Candidate families:

- B-tree-like weighted sequences;
- chunked vector/tree hybrids;
- RRB-style wide relaxed trees;
- measured/finger-tree ideas;
- rope-style weighted split/concat.

A-P is a representation-layout experiment, not an excuse to also change restart or semantic policy.

## Experimental order

```text
1. freeze Horse-A mechanism identity
2. freeze a failure-first structural-locality experiment
3. implement Horse-A with the three weaknesses intact
4. correctness first
5. work-locality kill gate
6. if work passes: timing / allocation / construction / memory
7. then full H0-H4-Horse-A comparison
8. diagnose the actual limiting weakness
9. authorize one next axis only if the evidence earns it
```

The first narrow witness should be #50-shaped:

```text
N = 128 KiB / 1 MiB / 16 MiB
fixed safe local paragraph edit
H0 / H4 / Horse-A
```

Immediate structural failure includes:

```text
untouched retained records touched proportional to M
whole suffix coordinate/checkpoint/locator rewrite
whole-definition recollection
whole-old-state retirement
hidden retained-tree scan
large-N fallback on the fixed local witness
incorrect normalized result
hidden deferred semantic/query work
```

A faster latency does not rescue a candidate that still performs Theta(M) unaffected-state work.

## Reading guide

### Restart / continuation locality

1. Wagner & Graham, **Efficient and Flexible Incremental Parsing**, TOPLAS 1998 — DOI `10.1145/293677.293678`.
2. Tree-sitter advanced parsing / `ts_tree_edit` / old-tree reuse; pair with this repository's `prior-art/tree-sitter.md` and `tree-sitter-markdown.md`.
3. Swift, **Incremental syntax parsing** proposal; pair with `prior-art/swift-incremental-syntax.md`.
4. Lezer system guide and `@lezer/markdown`; pair with `prior-art/lezer.md`.

### Semantic dependency locality

5. Reps, Teitelbaum, Demers, **Incremental Context-Dependent Analysis for Language-Based Editors**, TOPLAS 1983, DOI `10.1145/2166.357218`.
6. Reps, **Optimal-time incremental semantic analysis for syntax-directed editors**, POPL 1982.
7. Reps, **Incremental evaluation for attribute grammars with unrestricted movement between tree modifications**, Acta Informatica 1988.

### Sequence/layout locality

8. Hinze & Paterson, **Finger Trees: A Simple General-purpose Data Structure**, JFP 2006, DOI `10.1017/S0956796805005769`.
9. Bagwell & Rompf, **RRB-Trees: Efficient Immutable Vectors**.
10. Boehm, Atkinson, Plass, **Ropes: An Alternative to Strings**, 1995, DOI `10.1002/spe.4380251203`.
11. Bender, Demaine, Farach-Colton, **Cache-Oblivious B-Trees**, SIAM J. Comput., DOI `10.1137/S0097539701389956`.

Recommended short reading order:

```text
1. prior-art/MECHANISM-SOURCE-MAP.md
2. Wagner & Graham 1998
3. prior-art/tree-sitter.md + tree-sitter-markdown.md
4. Reps et al. 1983
5. Finger Trees
6. RRB-Trees
```

Then revisit whether Horse-A should freeze exactly as proposed, and which of A-R / B1 / A-P deserves to be the first post-A experiment.