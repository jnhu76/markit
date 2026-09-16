# Markit Roadmap

Status: **architecture synthesis next**

Markit has completed a product reset. The product requirements are retained and implementation architecture remains intentionally **not frozen**. The parser research experiment (#19) is **CLOSED** — evidence merged via PR #20, external review verdict **CORRECTIVE_PASS** — and the next phase is architecture synthesis (MARKIT-MARKDOWN-ARCHITECTURE-1).

## Current order of work

```text
R0  Product truth reset / archive old authority
        |
        v
R1  Markdown parser research (#19)               <- CLOSED (PR #20; direction: HYBRID)
        |
        v
R2  Research-question revision + parser verdict  <- DONE (question REFINED)
        |
        v
R3  Evidence-backed architecture                 <- NOW (MARKIT-MARKDOWN-ARCHITECTURE-1)
        |
        v
R4  Minimal source editor / document path
        |
        v
R5  Preview + rendering pipeline
        |
        +--> Mermaid / math
        +--> Browser / Print
        |
        v
R6  Live Mode
        |
        v
R7  Workspace / OS integration / extension seams
        |
        v
R8  v0.1 hardening
```

The order is deliberate: Markit should first understand how arbitrary Markdown edits propagate through parsing and semantic state. UI/render architecture must consume that result rather than dictate it.

---

## R0 — Product reset and archive

### Goal

Separate product truth from the previous experimental implementation and documents.

### Current product authority

- `docs/PRD.md` — product requirements;
- `docs/product/mvp-v0.1.md` — intended V0.1 product scope;
- `docs/product/print-browser-contract.md` — print/browser completeness contract;
- `docs/product/architecture.md` — **HOLD document only**, not a frozen implementation architecture;
- this roadmap — sequencing/status.

### Archive authority

The pre-reset repository is preserved at:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

Old ADRs, research-first product documents, GPUI/PocketJS experiments, the old Markdown implementation, benchmarks, and implementation notes are historical/reference evidence.

They must not silently become requirements for the new architecture.

---

## R1 — Incremental Markdown parser research (#19) — CLOSED

Completed and merged via PR #20; external human review verdict **CORRECTIVE_PASS** (MARKIT-19-CORRECTIVE-1: all four MAJOR findings fixed and re-measured). Parser direction: **HYBRID** = Markdown-aware restart → reparse → earliest-safe convergence + position-free history-stable persistent syntax + separate semantic dependency indexes + coherent full-rebuild escape hatch (predictor unvalidated). FROZEN / NOT-FROZEN lists: `results/summary/parser-survey-final.md`; research log: `docs/research/parser-survey-1.md`.

### Root research question — provisional

> **For lossless Markdown editing under arbitrary edits, how can Markit minimize reparse radius, tree reconstruction, memory movement, and downstream render invalidation while preserving correctness?**

This wording was provisional at kickoff. Outcome: the final report **REFINED** it into four independent cost axes with one mechanism per axis; see `results/summary/parser-survey-final.md`.

### Research map

```text
Theory
  Wagner & Graham
  incremental parsing / reuse

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

### Candidate Markit hypothesis

```text
arbitrary edit
    |
    v
small affected source region
    |
    v
block / inline parsing with boundary state
    |
    v
earliest safe convergence
    |
    v
reuse unaffected syntax
    |
    v
separate semantic dependency invalidation
```

The hypothesis is not architecture. It must survive the mutation corpus and correctness oracle.

### Measurements

Do not optimize only wall-clock parse time. At minimum inspect:

- bytes / lines rescanned;
- propagation radius;
- syntax regions or blocks reparsed;
- nodes rebuilt / reused;
- allocations / allocated bytes where measurable;
- memory movement / copy amplification where measurable;
- offset/index maintenance work;
- semantic dependency fan-out;
- equality with a clean parse.

### Important distinction

```text
syntax invalidation
!=
semantic dependency invalidation
!=
render/presentation invalidation
```

### Stop gate

No production parser replacement in R1.

Satisfied: #19 ended with evidence and a parser-direction verdict (HYBRID), then passed human review (CORRECTIVE_PASS). Implementation architecture is written next, in R3.

---

## R2 — Revise the research question and choose parser direction — DONE

Completed inside #19. Question verdict: **REFINED**. Parser direction verdict: **HYBRID** with explicitly unfrozen details. The refined formulation in `results/summary/parser-survey-final.md` is the research north star for later rendering/layout work.

Allowed question verdicts:

```text
KEEP
REFINE
REPLACE
```

The parser direction must then be one of:

```text
ADOPT_EXISTING
ADAPT_EXISTING
BUILD_MARKIT_SPECIFIC
INSUFFICIENT_EVIDENCE
```

If the question is refined, the new formulation becomes the research north star for later rendering/layout work.

---

## R3 — Evidence-backed architecture

Only after R1/R2 passes human review should `architecture.md` be replaced with a real implementation architecture.

It should decide, from evidence:

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

Questions such as Rope vs Piece Table, CST vs another representation, parser library choice, block checkpoints, semantic indexes, and render-delta shape belong here **after** the experiment.

Plugin/provider seams should be designed so future extensions consume stable semantic/query/command contracts rather than private parser/UI internals.

---

## R4 — Minimal source editor

After parser/storage architecture is selected, build the smallest trustworthy user path:

```text
open Markdown
  -> authoritative source
  -> Source Mode edit
  -> revision/change
  -> parse/semantic update
  -> save
```

Requirements include:

- lossless Markdown source editing;
- undo/redo;
- UTF-8/CJK/emoji;
- Windows IME;
- open/save/save-as;
- mode-independent document authority.

Existing `markit-core` code is a candidate reference only. Audit each component as `ADOPT`, `ADAPT`, `REPLACE`, or `DELETE`.

---

## R5 — Preview and rendering

UI/rendering work begins only after the parser/semantic contract is credible.

Study the second half of the end-to-end cost chain:

```text
SemanticDelta
     -> projection invalidation
     -> layout invalidation
     -> shaping
     -> paint
     -> pixels
```

The key future question is not merely GPU speed, but how parser-local changes propagate into the minimum correct amount of downstream work.

This phase includes:

- split Source + Preview;
- progressive publication where appropriate;
- Mermaid;
- LaTeX-style math;
- Browser Preview;
- browser Print/PDF according to `print-browser-contract.md`.

Heavy Mermaid/math rendering must remain outside the Markdown parser critical path.

---

## R6 — Live Mode

Add source-aware WYSIWYG editing only after source/syntax/semantic/visual mappings are understood.

Hard invariant:

```text
Live Mode is a projection/editing surface over the same Markdown source.
It is not a second rich-text document synchronized back to Markdown.
```

Mode switching alone must be source-byte neutral.

---

## R7 — Workspace, OS integration, extension seams

Product requirements include:

- workspace file tree;
- workspace text search;
- Windows `.md` Open With/file association;
- browser launch;
- future plugin/provider extensibility.

These features must not force parser or UI internals into public extension contracts.

A general plugin runtime is not required for V0.1; clean semantic seams are.

---

## R8 — V0.1 hardening

Validate the complete product on the real Windows host:

- correctness under arbitrary edits;
- large-document behavior;
- CJK/IME/path handling;
- workspace search;
- Mermaid/math failure and stale-result cases;
- print completeness independent of scroll history;
- memory and work-amplification bounds;
- packaging/file association.

---

## Current rule

Issue #19 is complete and human-reviewed. The rule becomes:

> **Do not implement a parser before the MARKIT-MARKDOWN-ARCHITECTURE-1 contract passes architecture review.**

The current task is architecture synthesis: turn the earned constraints (FROZEN / NOT-FROZEN lists in `results/summary/parser-survey-final.md`) into module responsibilities, data flow, entry/exit points, and coherence boundaries for the Markdown core. No further parser experiments.
