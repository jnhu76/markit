# AGENTS.md

This file defines the current working rules for humans and AI coding agents
contributing to Markit. It is an **authority guard**: when another document,
issue, or piece of code contradicts this file, this file wins until a reviewed
decision updates the conflict.

## 1. Mission (product truth)

Markit is a local-first Markdown editor/workspace whose document-content truth
is Markdown source. The intended product includes Source Mode, source-aware
Live Mode, workspace search, split Preview, Mermaid, LaTeX-style math, browser
preview with reliable browser Print/PDF, Windows file association / Open With,
and future plugin/provider extensibility through stable boundaries.

Product architecture is Markdown-centric: Document source + explicit revision
is the content root; Markdown semantics is derived interpretation; Preview,
Browser, Print and rich rendering are downstream projections/artifacts.

## 2. Two active lines: product and research

Markit now deliberately runs two independent lines.

### 2.1 Product integration line

```text
PRODUCT_INTEGRATION_BRANCH = dev/markit-product
PRODUCT_ARCHITECTURE       = docs/product/architecture.md
FOUNDATION                 = markit-composition + markit-app
```

Product work may proceed on branches based on `dev/markit-product` when it
stays inside the frozen architecture boundary.

The product line may implement domain seams, adapters, projections, provider
roles and UI experiments without waiting for parser-research representation
choices, provided those choices stay behind the product contract.

### 2.2 Research line

`research/**` and the active research issues continue to own parser / AST-CST /
incremental-update experiments and evidence.

Research remains authoritative for questions such as:

```text
AST/CST/hybrid representation
source storage
checkpoint/continuation representation
damage/recomputation locality
reuse identity
SemanticDelta
semantic dependency representation
fine-grained invalidation
H0-H4/Horse-A mechanism internals
performance claims
```

The product line must not silently turn one research mechanism into research
truth merely because it is useful as an initial provider.

The research line must not change its experiment protocol merely to fit a
product implementation.

## 3. Authority map

```text
docs/PRD.md
    = product goals / requirements

docs/product/architecture.md
    = normative product architecture authority

docs/product/print-browser-contract.md
    = normative Browser/Print completeness contract

docs/product/foundation-rules.md
    = K0/plugin/provider foundation conventions, subordinate to architecture.md

docs/product/repository-layout.md
    = product/research path and dependency ownership rules

crates/markit-composition + crates/markit-app
    = current generic product foundation implementation

active research issues + docs/research/** + research/**
    = research-question / experiment / mechanism authority only

archived experiments / removed implementations / Git history
    = evidence only; no residual product authority
```

A research result may replace a backend or motivate an explicit architecture
reopening. It may not silently redefine product authority boundaries through an
implementation commit.

## 4. Frozen product architecture rules

The detailed authority is `docs/product/architecture.md`. Agents must preserve
at least these invariants:

1. **One document-content root** — every active Document owns its Markdown
   source; all editing modes commit through the same authority.
2. **Explicit source commit identity** — edits identify base revision and
   coordinates; undo/redo are new commits; revision identity is not ABA-reused.
3. **One Markdown interpretation contract** — Markdown-aware projections use
   the same semantic contract/configuration; backend-private types do not
   escape to consumers.
4. **Pinned reads stay pinned** — a fixed semantic view never silently upgrades
   to the latest revision.
5. **Hot edits bypass generic buses** — per-edit data does not route through K0
   reconciliation, generic EventBus, or UI Slot lookup.
6. **Heavy rendering is downstream** — Mermaid/math/layout/browser/GPU work is
   outside ordinary Markdown parse/update critical paths and cannot mutate
   source.
7. **Freshness checks target + dependencies** — cancellation alone is never a
   correctness proof for async results.
8. **One generic composition/lifecycle framework** — K0 remains the generic
   composition authority; do not add a second generic Resource/lifecycle
   framework without a demonstrated requirement.
9. **Slots are presentation-only** — Slot registries own placement and
   contribution validity, never Document/semantic state or hot data transport.
10. **Print is pinned and complete** — Print materializes a whole pinned
    document independently of viewport history and owns the aggregate readiness
    barrier.
11. **UI toolkit is downstream** — Document/Markdown/projection/Print contracts
    do not depend on GPUI, Electron/React or another UI implementation.

These rules freeze responsibilities and semantic boundaries, not parser or UI
representation details.

## 5. Product work agents MAY do

Within a branch based on `dev/markit-product`, agents may:

- implement the Document source/revision/edit/history authority;
- define narrow product semantic/query/provider contracts;
- implement an H4-backed **product adapter/provider** to validate the seam;
- implement revision-bound read views and earned projections such as Outline;
- implement Filesystem/Mermaid/Math/Print provider roles when their slice is
  authorized;
- implement the reduced UI Slot semantics described by the product
  architecture when a real UI contribution point requires them;
- run GPUI/Electron comparative spikes without exposing toolkit types to domain
  contracts;
- add targeted correctness tests for the concrete product slice;
- update product documentation when implementation earns a narrower contract.

Initial H4 product use is seam validation, not a research verdict. Product code
must not path-depend on `research/**`; reusable mechanism code must be explicitly
admitted/extracted/adapted into a product-owned location behind the product
contract.

## 6. Product work agents MUST NOT do

Do not:

- make H4/Horse-A/private parser nodes/checkpoints/reuse identity part of a
  product consumer contract;
- path-depend from product crates into `research/**`;
- move research files into `crates/**` merely to avoid the firewall;
- route source edits through a universal EventBus, generic message protocol,
  Slot registry, or repeated K0 reconciliation;
- introduce a second generic lifecycle/Resource framework on top of K0 without
  a concrete missing requirement;
- create a Redux-like/global application authority or monolithic Workbench
  model that copies Document/semantic state;
- let Preview/Print independently reinterpret Markdown with their own dialect;
- make Print traverse only the currently materialized interactive UI tree;
- synchronously render Mermaid/math/layout/browser output inside ordinary
  Markdown parser/update work;
- freeze `SemanticDelta`, cross-revision node identity, AST/CST shape, source
  storage, fine-grained invalidation or scheduling merely because the product
  slice would be easier with a specific choice;
- assume GPUI or Electron is selected before the comparative decision;
- create dynamic plugin loading, marketplace, WASM ABI, untrusted-extension
  sandboxing, keyed/chain/shadow Slot machinery or another generic framework
  without an earned V1 requirement;
- modify unrelated research evidence, benchmark manifests, frozen protocols or
  result records from a product branch.

## 7. Research authorization is a hard gate

Do not use benchmark code, prototype implementation, instrumentation, workload
selection, or exploratory optimization to obtain an early decision-bearing
answer for a research question that has not been authorized and frozen by its
current research contract.

“Only exploratory”, “just checking whether it is promising”, “just a smoke
benchmark”, or “we will formalize it later” does not bypass this gate.

Before a decision-bearing experiment, freeze enough to state:

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
the decision the result can change
```

If pre-authorization measurements are produced accidentally, mark them
non-decision-bearing. They must not be used to select a favored mechanism,
freeze a treatment, tune the corpus/metrics/thresholds around observed results,
or justify promotion to the next research stage.

## 8. Benchmark integrity

A fast wrong parser fails. For every mutation where the comparison is defined:

```text
incremental result == clean authoritative parse
```

Record more than wall-clock where practical: bytes/lines rescanned, nodes
rebuilt/reused, allocations, memory movement, offset/index maintenance,
propagation distance, semantic dependents changed. Separate measurement
overhead from the measured metric.

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

Correctness evidence comes before performance interpretation. A material
protocol change after observing treatment results creates a new experiment
version; all mechanisms needed for the comparison must be rerun under the new
frozen protocol.

A benchmark that shows no advantage, exposes a weakness, or falsifies the
preferred hypothesis is successful research.

## 9. Product correctness discipline

Product tests should establish distinct product contracts rather than maximize
test count.

Prefer:

```text
one risk -> one strong witness
```

over overlapping defensive suites.

Examples of slice-specific contracts that deserve direct tests when introduced:

- stale base-revision edit rejection;
- pinned read consistency across later edits;
- provider substitution without leaking backend-private types;
- late/stale async result rejection;
- provider failure cannot mutate source or promote stale state;
- Print required-set completeness and `Ready | FailedVisible` barrier;
- Slot owner/contributor lifetime and stale authorization;
- product→research dependency firewall.

Do not duplicate K0's already-proven lifecycle suites in every domain crate.
Do not add fuzz/property/formal/negative-control machinery merely because it
sounds safer; each mechanism must defend a concrete risk not already covered by
a cheaper/stronger witness.

## 10. Unicode and source coordinates

Anything production-facing must remain compatible with Unicode/CJK/emoji.
Do not conflate:

```text
byte offsets
Unicode scalar positions
grapheme boundaries
logical source positions
visual positions
platform UTF-16 coordinates
```

Every persisted or cross-layer source location must state its coordinate
system and revision binding. UI adapters may translate platform coordinates;
that translation does not redefine Document source coordinates.

## 11. Repository and dependency boundaries

The root product workspace owns `crates/**`; `research/**` is excluded and
self-contained.

Rules in `docs/product/repository-layout.md` and
`tools/check_product_boundaries.py` apply.

In particular:

```text
markit-composition -> no domain/research/UI dependencies
markit-app         -> markit-composition only
product domain     -> product contracts/providers only
product            -X-> research/**
UI                 -X-> research representation
```

Do not create root-level catch-all ownership such as `core/`, `common/`,
`shared/`, `utils/`, `runtime/`, or catch-all crates such as `markit-core` /
`markit-common` without an explicit reviewed architecture decision.

## 12. Documentation rules

- Active product architecture/contracts live under `docs/product/`.
- Active research authority/evidence lives under `docs/research/` and
  `research/**`.
- Closed experiments stay under `research/experiments/`.
- Do not resurrect superseded/removed implementations as authority.
- When research changes a mechanism behind an existing product seam, update
  research authority/evidence but do not rewrite product architecture unless
  the seam itself must change.
- When a real product requirement cannot fit ARCH-01..ARCH-11, treat it as an
  explicit product-architecture reopening event; do not smuggle it through an
  implementation PR.

## 13. Stop rules

For **product work**, prefer:

```text
small typed domain seam
+ coarse-but-correct revision publication
+ explicit owner/lifetime
```

over:

```text
new generic infrastructure
+ speculative extension machinery
+ parser-specific consumer coupling
```

For **research work**, prefer:

```text
measure the Markdown edit under the active frozen protocol
```

over:

```text
design the experiment around a preferred product outcome
```

The objective is a product whose stable Markdown-centric seams can survive
better research results, while the research remains free to improve the engine
without being contaminated by product convenience.
