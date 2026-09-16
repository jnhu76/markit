# Markit pre-product-reset archive

> **SUPERSEDED authority note.** This is a point-in-time boundary record for
> MARKIT-PRODUCT-RESET-0. The "Active post-reset authority" and "Code status"
> sections below are historical: the authority map now lives in `docs/README.md`
> (active campaign: #22), and the old implementation trees it describes were
> later removed from the active tree / archived by
> MARKIT-EXPERIMENT-FIRST-REPO-RESET-1 (`docs/research/repo-reset-inventory.md`).

This directory marks the historical authority boundary for the Markit product reset.

## Canonical archive revision

The exact pre-reset repository is preserved by Git at:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

That immutable revision is the canonical archive. Historical files do not need to be duplicated into the active documentation tree merely to preserve them.

Use that revision when exact historical wording, benchmark code, ADRs, implementation notes, or old source behavior is needed.

## Material archived by that revision

It contains, among other things:

- the previous `README.md`;
- the previous `docs/PRD.md`;
- the previous product architecture, roadmap, and MVP documents;
- ADR-001 through ADR-008;
- the GPUI/PocketJS substrate and feasibility work;
- A0-A4 research and intervention reports;
- the previous realtime execution model;
- performance invariants;
- plugin compatibility design;
- platform capability matrix;
- issue backlog;
- P0-01 / P0-02 implementation notes;
- the previous Markdown semantic contract;
- benchmark/results material;
- the pre-reset source implementation.

These are retained as **historical/experimental evidence**, not current product or architecture authority.

## Code status

The current branch still carries the old implementation in its existing paths so that:

- experiments remain reproducible;
- Issue #19 can use the current Markdown implementation as a comparison baseline;
- useful components can be audited rather than discarded blindly.

That does **not** make the current code architecture authoritative.

Every component that might survive the parser research must later be classified explicitly as:

```text
ADOPT
ADAPT
REPLACE
DELETE
```

Moving or rewriting source directories before the parser experiment would add noise and could destroy the comparison baseline, so code cleanup is intentionally semantic-first rather than directory-churn-first.

## Active post-reset authority

During the parser-research phase:

```text
docs/PRD.md
    = product requirements

Issue #19 + docs/research/markdown-parser/README.md
    = current parser research authority

docs/product/architecture.md
    = architecture HOLD / invariant boundary only

docs/product/print-browser-contract.md
    = print/browser output contract

docs/product/mvp-v0.1.md
    = intended product scope

docs/product/roadmap.md
    = sequencing
```

A full implementation architecture is intentionally absent until Issue #19 completes and its verdict is reviewed.

## Re-adoption rule

Historical material does not become current merely because it existed before the reset.

A historical decision may be re-adopted only when new evidence explicitly earns it back.
