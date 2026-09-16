# Markit research

This directory contains **active post-reset research**.

Historical A0-A4 / GPUI / PocketJS material is preserved at the pre-reset archive revision:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

It is no longer duplicated in the active documentation tree.

## Active research

```text
markdown-parser/
```

The most recent campaign was Issue #19 — **CLOSED** (external review verdict CORRECTIVE_PASS; evidence merged via PR #20):

> Incremental Markdown parsing: locality, convergence, and invalidation.

See `markdown-parser/README.md` for the provisional research question, comparison map, cost model, and exit conditions; see `results/summary/parser-survey-final.md` for the final verdicts and the FROZEN / NOT-FROZEN lists.

## Rule

Research documents may propose mechanisms, but they do not become architecture authority automatically.

The lifecycle is:

```text
question
  -> experiment
  -> evidence
  -> verdict
  -> reviewed architectural decision
```

Old experiments remain useful evidence within their original setup, but old recommendations are not inherited into the new Markit architecture by default.
