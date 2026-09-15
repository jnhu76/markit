# Markdown Parser Research

Status: **ACTIVE**

Primary campaign: [Issue #19 — Incremental Markdown parsing: locality, convergence, and invalidation](https://github.com/jnhu76/markit/issues/19)

This directory is the durable documentation anchor for Markit's current research phase.

## Provisional root question

> **For lossless Markdown editing under arbitrary edits, how can Markit minimize reparse radius, tree reconstruction, memory movement, and downstream render invalidation while preserving correctness?**

This is a research question, not a frozen architecture statement.

At the end of Issue #19, the question itself must receive one verdict:

```text
KEEP
REFINE
REPLACE
```

If experimental evidence shows that the important cost model is different—for example because source storage, offset/index maintenance, semantic dependency fan-out, cache locality, or downstream layout dominates—the question must be rewritten.

## Comparison map

```text
Theory
  Wagner & Graham
  incremental parsing / optimal reuse

Systems
  Tree-sitter
  Lezer
  Roslyn
  rust-analyzer / rowan

Markdown-specific
  @lezer/markdown
  tree-sitter-markdown
  mizchi/markdown
  MD4C
```

These are comparison/reference families, not predetermined dependencies.

The experiment should ask what mechanism each system uses, what information it preserves, what authority it claims, and where work is amplified after an arbitrary edit.

## Candidate Markit hypothesis

A candidate model worth testing is:

```text
Document
   |
   v
arbitrary edit
   |
   v
small affected source region
   |
   v
Block / Inline parser
   |
   v
boundary state / context summary
   |
   v
earliest safe convergence
   |
   v
reuse unaffected syntax
   |
   v
semantic dependency update
```

The useful unit may be a block, subtree, fragment, checkpoint, green node, or something else. The experiment must decide rather than assuming a block-local CST is automatically correct.

## Cost model under investigation

The benchmark should look beyond elapsed parse time.

Potential dimensions include:

```text
source mutation / copy cost
bytes and lines rescanned
reparse radius
syntax nodes rebuilt
syntax nodes reused
allocation / transient memory
memory movement
metadata / offset maintenance
semantic dependency fan-out
projection invalidation
future layout / shaping invalidation
```

The distinction below is fundamental:

```text
syntax invalidation
!=
semantic dependency invalidation
!=
downstream presentation invalidation
```

A global semantic relationship must not automatically imply global syntax reparsing.

## What this phase does not decide

Do not use this research phase to freeze:

- GPUI or another UI backend;
- final editor layout;
- Live Mode visual behavior;
- rendering scheduler topology;
- plugin runtime/transport;
- Rope vs Piece Table without evidence;
- AST vs CST without evidence;
- Mermaid or math rendering implementation.

Mermaid and math may appear in mutation cases because their syntax affects Markdown classification, but heavy rendering remains outside the parser critical path.

## Evidence rule

A fast incremental result that differs from a clean authoritative parse is a failure.

A parser that reparses one block but performs hidden `O(N)` metadata shifts is not proven local.

A parser that wins one microbenchmark but causes larger downstream invalidation is not automatically the best Markit parser.

The desired result is the lowest **correct end-to-end work amplification** that the experiment can justify.

## Exit

This research phase ends only after Issue #19 provides:

- reproducible mutation corpus;
- comparison evidence;
- correctness oracle;
- work-amplification measurements;
- research-question verdict;
- parser-direction verdict.

Only then should Markit write the next real architecture document.
