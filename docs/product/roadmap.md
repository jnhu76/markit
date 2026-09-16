# Markit Roadmap

Status: **benchmark-first research (#22) is the active phase**

Markit has completed a product reset and an experiment-first repository reset.
Product requirements are retained; implementation architecture remains
intentionally **not frozen**. Experiment 0 (#19, merged via PR #20) is closed
and archived as historical evidence. The attempted direct jump to architecture
synthesis (#21) is **CLOSED / SUPERSEDED**. The active phase is the
standardized Markdown AST/CST update benchmark — **#22
MARKIT-MARKDOWN-BENCHMARK-1**.

## Current order of work

```text
R0  Product truth reset / archive old authority           <- DONE
        |
        v
R1  Experiment 0: parser mechanism survey (#19/PR #20)    <- DONE, ARCHIVED
        |                                                    (evidence only)
        v
R2  Architecture synthesis attempt (#21)                  <- SUPERSEDED / CLOSED
        |
        v
R2' Experiment-first repo reset (#23)                     <- DONE
        |
        v
R3  Standardized Markdown AST/CST update benchmark (#22)  <- NOW
        |
        v
R4  Weakness Map review
        |
        v
R5  Markit-specific algorithm campaign (future issue)
        |
        v
R6  Formal/correctness work (as applicable)
        |
        v
R7  Evidence-backed architecture synthesis
        |
        v
R8  Minimal source editor / document path
        |
        v
R9  Preview + rendering pipeline
        +--> Mermaid / math
        +--> Browser / Print
        |
        v
R10 Live Mode
        |
        v
R11 Workspace / OS integration / extension seams
        |
        v
R12 v0.1 hardening
```

The order is deliberate: Markit first measures how existing Markdown
parsers/update designs behave under identical payloads, edits, and gates,
explains the weaknesses, and only then designs a Markit-specific algorithm and
architecture. UI/render architecture must consume that result rather than
dictate it.

---

## R0 — Product reset and archive — DONE

Product truth separated from the pre-reset implementation. Canonical pre-reset
archive revision:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

### Current product authority

- `docs/PRD.md` — product requirements;
- `docs/product/mvp-v0.1.md` — intended V0.1 product scope;
- `docs/product/print-browser-contract.md` — print/browser completeness contract;
- `docs/product/architecture.md` — **HOLD document only**;
- this roadmap — sequencing/status.

---

## R1 — Experiment 0: parser mechanism survey (#19) — DONE, ARCHIVED

Merged via PR #20 (`55f6326f360f77d2caf46bb60571c8f15a88de53`); external human
review verdict CORRECTIVE_PASS. Reclassified afterwards as **Experiment 0 /
mechanism reconnaissance — historical evidence only**.

Physical archive: `research/experiments/experiment-0-parser-survey/`.
Evidence: `results/summary/` inside that archive. Explicit non-identity
statements hold: `HYBRID != current architecture`, `P0-02 != current parser
candidate`, green-tree prototype != production representation, ReferenceIndex
!= production semantic index.

Its observations (locality, structural propagation, hidden O(N) metadata
work, full/incremental crossover, syntax vs semantic invalidation separation)
motivate the normalized benchmark, but do not decide anything by themselves.

---

## R2 — Architecture synthesis attempt (#21) — SUPERSEDED / CLOSED

#21 treated Experiment 0 as sufficient architecture input. That sequencing was
rejected: hypotheses produced before a normalized cross-parser benchmark are
not an architecture basis. Do not resume #21 or cite it as a gate.

---

## R2' — Experiment-first repository reset (#23) — DONE

MARKIT-EXPERIMENT-FIRST-REPO-RESET-1 restructured the active tree around the
benchmark-first pipeline:

- obsolete pre-reset implementation deleted from the active tree (Git history
  is the archive; see `docs/research/repo-reset-inventory.md`);
- Experiment 0 physically archived under `research/experiments/` and detached
  from any default workspace;
- root Cargo workspace dissolved (no active production Rust crate exists);
- authority documents frozen on #22 as the only active research campaign.

---

## R3 — Standardized Markdown AST/CST update benchmark (#22) — NOW

Area: `research/benchmarks/markdown-ast-update/`.

Fixed first-round baselines, grouped by capability (full-rebuild controls vs
incremental update subjects): MD4C, pulldown-cmark, Comrak or cmark-gfm (one),
tree-sitter-markdown, @lezer/markdown, mizchi/markdown. Full-rebuild controls
participate in structural-edit scenarios as clean-rebuild controls — that is a
control arm, not an N/A.

Method requirements (from #22):

- Swift-style incremental-vs-clean comparison per edit;
- hard correctness gates: self-equivalence, dialect-semantics oracle,
  losslessness where reconstructable;
- work-amplification metrics beyond wall-clock: bytes rescanned, nodes
  rebuilt/reused, allocations, position/index maintenance, propagation;
- recorded provenance per run: versions, commits, runtime, toolchain,
  hardware/OS, corpus version, benchmark mode.

Output: a performance surface plus a **Weakness Map** — which weaknesses are
common across designs, and which are worth a Markit-specific algorithm.

This phase does NOT select a "winning parser", does NOT write the Markit
algorithm, and does NOT unfreeze architecture.

---

## R4 — Weakness Map review

Human review of the #22 evidence: which measured weaknesses are real, common,
and consequential; which are artifacts; what a Markdown-specific algorithm
should attempt. The review decides whether a Markit algorithm campaign is
justified and scoped.

---

## R5 — Markit-specific algorithm campaign (future issue)

Only weaknesses earned by R4 enter here. The campaign produces a candidate
algorithm with measured evidence against the #22 baseline surface.

---

## R6 — Formal/correctness work (as applicable)

Formalize/verify the candidate algorithm's correctness properties to the depth
its risk warrants. This phase must exist before architecture freeze; its depth
is decided by the algorithm campaign's evidence.

---

## R7 — Evidence-backed architecture

Only after R3–R6 pass human review should `architecture.md` be replaced with a
real implementation architecture, deciding from evidence:

```text
Document/source storage
        |
        v
incremental parsing contract
        |
        v
lossless syntax / semantic representation
        |
        v
dependency / invalidation model
        |
        v
projection contract
        |
        v
render/layout scheduling
        |
        v
UI backend(s)
```

Plugin/provider seams are designed here so future extensions consume stable
semantic/query/command contracts rather than private parser/UI internals.

---

## R8–R12 — Product build-out

R8 minimal source editor, R9 preview/rendering (Mermaid, math, browser
print/PDF per `print-browser-contract.md`), R10 Live Mode, R11 workspace/OS
integration/extension seams, R12 v0.1 hardening. Scope definitions live in
`docs/PRD.md` and `docs/product/mvp-v0.1.md`.

---

## Current rule

```text
#22 is the only active Markdown research campaign.
No production parser. No Markit algorithm claims. No architecture.
Everything waits for the measured Weakness Map.
```
