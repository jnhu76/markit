# AGENTS.md

This file defines the current working rules for humans and AI coding agents
contributing to Markit. It is an **authority guard**: when any other document,
issue, or piece of code contradicts this file, this file wins until a reviewed
decision says otherwise.

## 1. Mission (product truth)

Markit is a local-first Markdown editor/workspace whose document truth is
Markdown source. The intended product includes Source Mode, source-aware Live
Mode, workspace search, split Preview, Mermaid, LaTeX-style math, browser
preview with reliable browser Print/PDF, Windows file association / Open With,
and future plugin/provider extensibility through stable boundaries.

The PRD (`docs/PRD.md`) and `docs/product/**` define **what** the product must
do. They grant **no implementation authority**.

## 2. Current active campaign

```text
CURRENT_ACTIVE_CAMPAIGN = #22 MARKIT-MARKDOWN-BENCHMARK-1
```

Issue #22 builds a **standardized Markdown AST/CST update benchmark**: the
same payloads, the same edits, the same correctness gates, and the same
measurement model across existing parsers / AST-CST update designs. Its
purpose is a performance surface and a **Weakness Map** — which weaknesses are
common, and which are worth solving with a Markit-specific algorithm.

The research pipeline is fixed for this phase:

```text
existing Markdown parser / AST-CST update algorithms
        ↓ standardized benchmark (#22)          <- NOW
        ↓ Weakness Map
        ↓ Markit-specific algorithm (future issue)
        ↓ formal/correctness work (as applicable)
        ↓ architecture synthesis
        ↓ production implementation
```

## 3. What agents must NOT do

Do not:

- implement a production Markdown parser or editor;
- resume #21 (MARKIT-MARKDOWN-ARCHITECTURE-1) — it is CLOSED / SUPERSEDED;
- promote Experiment 0 (#19) prototypes into product or architecture claims;
- assume `markit-core` (now archived under
  `research/experiments/experiment-0-parser-survey/baseline/`) is a future
  product core or an implicit benchmark baseline — it is an Experiment 0
  measurement subject only;
- assume GPUI (removed from the active tree) is the future frontend;
- start formal proofs of a not-yet-existing Markit algorithm;
- write the final architecture document or unfreeze
  `docs/product/architecture.md`;
- modify the Experiment 0 archive except relocation-compatible fixes that do
  not change experiment semantics;
- treat old benchmark numbers, corpora, or tooling as current evidence without
  re-measuring under the #22 protocol.

### Experiment authorization is a hard gate

Do not use benchmark code, prototype implementation, instrumentation, workload
selection, or exploratory optimization to obtain an early answer for a research
question that has not yet been authorized and frozen by the current research
contract.

“Only exploratory”, “just checking whether it is promising”, “just a smoke
benchmark”, or “we will formalize it later” does not bypass this gate. Before the
relevant experiment is authorized, code may only serve work already permitted by
the current campaign: benchmark-substrate construction, oracle validation,
correctness characterization, instrument validation, or another explicitly
authorized preparatory task. It must not silently exercise a future Markit
algorithm or produce decision-bearing comparative evidence for it.

If pre-authorization measurements are produced accidentally, mark them
non-decision-bearing. They must not be used to choose the favored mechanism,
freeze Hx, select the corpus/edit subset, choose metrics, set thresholds, or
justify promotion to the next research stage. Any later authorized experiment
must be independently preregistered and rerun under its frozen protocol.

## 4. What agents MAY do (this phase)

Allowed work is benchmark research only:

- benchmark harnesses and baseline adapters for #22's fixed subject list
  (MD4C, pulldown-cmark, Comrak or cmark-gfm, tree-sitter-markdown,
  @lezer/markdown, mizchi/markdown);
- corpus / mutation definitions under the #22 protocol;
- measurement instrumentation and reuse/work counters;
- correctness oracles (self-equivalence, dialect semantics, losslessness);
- weakness attribution and result records;
- documentation that records evidence under `docs/research/` and
  `research/benchmarks/markdown-ast-update/`.

Do not conflate "can install a dependency" with "start the experiment" — #22
owns experiment execution; setup-only preparation must stop before measuring.

## 5. Authority map

```text
docs/PRD.md, docs/product/**
    = product requirements authority (no implementation authority)

issue #22 + research/benchmarks/markdown-ast-update/
    = the ONLY active Markdown parser research authority

research/experiments/experiment-0-parser-survey/
    = ARCHIVED Experiment 0 (#19 / PR #20) — historical evidence only

issue #21
    = SUPERSEDED / CLOSED — must not be used as a gate

docs/product/architecture.md
    = HOLD / invariant boundary only

docs/product/print-browser-contract.md
    = print/browser completeness contract

docs/product/mvp-v0.1.md
    = intended V0.1 product scope

docs/product/roadmap.md
    = work sequencing

Git history
    = the archive of all removed implementations and documents
```

Key historical revisions:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e   complete pre-reset repository
(before MARKIT-EXPERIMENT-FIRST-REPO-RESET-1)  old apps/crates/bench trees
55f6326f360f77d2caf46bb60571c8f15a88de53   Experiment 0 merge (PR #20)
```

## 6. Frozen product invariants

These hold regardless of research outcome (details in
`docs/product/architecture.md`):

1. **One document truth** — Markdown source is authoritative; every mode,
   preview, print, search, and cache is derived state. No second rich document
   that must be synchronized back.
2. **Losslessness** — do not choose a representation that silently loses
   Markdown markers or normalizes unrelated source.
3. **Heavy rendering is outside the parser critical path** — Mermaid, math,
   layout, shaping, browser generation, and GPU work must not run synchronously
   inside ordinary parsing.
4. **Print completeness is separate from interactive laziness** — print must
   represent the complete coherent document, independent of scroll history
   (`docs/product/print-browser-contract.md`).
5. **Extension seams stay possible** — private parser nodes, memory layouts,
   or caches must not become the future plugin contract by accident.

## 7. Benchmark integrity

A fast wrong parser fails. For every mutation where the comparison is defined:

```text
incremental result == clean authoritative parse
```

Record more than wall-clock where practical: bytes/lines rescanned, nodes
rebuilt/reused, allocations, memory movement, offset/index maintenance,
propagation distance, semantic dependents changed. Separate measurement
overhead from the measured metric. Record toolchain, commit SHA, hardware/OS,
corpus version, and benchmark mode with every performance claim.

Do not justify a mechanism with "this should be faster", "native is faster",
"all editors do this", or "the old Markit code already had it". All of those
are hypotheses to measure.

### 7.1 Failure-first benchmark discipline

An authorized benchmark mechanism must be capable of losing. Before implementing,
tuning, or materially changing the treatment, freeze enough of the experiment to
state:

```text
payload / corpus
edit or mutation
clean authoritative parse oracle
normalized result contract
expected correctness failures
expected degradation modes
measured costs
measurement boundary
falsification / weakening conditions
and the decision the result can change
```

Correctness evidence comes before performance interpretation. No timing result is
decision-bearing for an edit whose incremental result has not passed the frozen
authoritative equivalence gate.

Do not implement an optimization, inspect favorable cases, and then redefine the
workload, edit selection, metric, exclusion rule, threshold, or correctness gate
around those cases. A material protocol change after observing treatment results
creates a new experiment version; all mechanisms needed for the comparison must
be rerun under the new frozen protocol.

Unit tests are appropriate when they independently establish an invariant that
the benchmark oracle does not directly expose. Do not add implementation-shaped
tests merely to increase coverage of a mechanism already determined by its own
code. For this research phase, E2E is not the sole or primary testing mechanism:
the experimental variable must remain isolated enough to attribute cost.
End-to-end editor tests become primary evidence only when a later authorized
claim crosses parser/update boundaries into actual editor behavior.

Every decision-bearing run should produce or reference replayable evidence
containing, where applicable:

```text
repository commit
toolchain
hardware / OS
corpus identity and content hash
mutation/edit manifest
mechanism configuration
normalized correctness results or hashes
raw benchmark samples
allocation / reuse / rescan / propagation counters
measurement metadata
exact execution command
analysis output
```

Derived charts and summary tables must be regenerable from retained raw evidence.
During mechanism development run narrow correctness and smoke checks; run the
frozen comparison campaign at the designated measurement boundary rather than
continuously executing the whole corpus merely because the harness can do so.

A benchmark that shows no advantage, exposes a new weakness, or falsifies Hx is
successful research. Do not modify the benchmark to preserve the preferred
algorithm.

## 8. Unicode and source coordinates

Anything production-facing must remain compatible with Unicode/CJK/emoji. Do
not conflate byte offsets, Unicode scalar positions, grapheme boundaries,
logical source positions, visual positions, or platform UTF-16 coordinates.
Any persistent position/index design must state its coordinate system and its
edit-shift cost.

## 9. Documentation rules

- Active product docs live under `docs/product/`; active research records
  under `docs/research/` and `research/benchmarks/markdown-ast-update/`;
  closed experiments under `research/experiments/`.
- Do not resurrect superseded documents as authority. Write a new
  evidence-backed document instead.
- When research changes the root question, direction, or authority boundary,
  update `docs/research/README.md`, `docs/product/architecture.md`,
  `docs/product/roadmap.md`, `README.md`, and this file as applicable.

## 10. Stop rule

When uncertain, prefer:

```text
measure the Markdown edit (under the #22 protocol)
```

over:

```text
design the editor around an unproven parser
```

The objective is not a repository that looks finished. It is a measured
Weakness Map that earns the Markit algorithm — and only then an architecture.
