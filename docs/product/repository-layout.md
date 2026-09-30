# Repository Layout — Ownership and Boundary Rules

Status: active product documentation (Issue #106, MARKIT-FOUNDATION-0).
This is a structural boundary document only; it grants no implementation
authority and reopens no architecture decision. `AGENTS.md` remains the
authority guard.

## Directory ownership

```text
research/            owns experimental mechanisms, benchmark evidence,
                     preregistration, workloads, and research artifacts.
                     Self-contained research workspaces with their own
                     frozen manifests, profiles, and toolchain pins. They
                     are NOT members of the product workspace and must
                     never become so (the root Cargo.toml excludes them).

crates/              owns production/reusable Markit crates
                     (currently: markit-composition, the generic
                     composition kernel; future document/markdown-api/
                     workbench/platform crates belong here as authorized).

apps/                owns executable application composition roots
                     (reserved; not created until an application exists).

docs/product/        owns product contracts and requirements.

docs/research/       owns research authority records.

scripts/             owns repository automation.
```

Do not create speculative empty directories; create a root only when its
first real occupant arrives. Do not introduce repository-root directories
whose ownership is ambiguous (`core/`, `common/`, `shared/`, `utils/`,
`engine/`, `runtime/`, `misc/`, `prototype/`, …) or catch-all crates
(`markit-core`, `markit-common`, `markit-utils`, `markit-shared`).

## Dependency direction

```text
generic product infrastructure (markit-composition, …)
        must not depend on research mechanisms

product Markdown adapter
        may depend on a selected backend (behind the product contract)

UI/workbench
        depends on product contracts,
        never on research representations directly
```

Concretely:

- `crates/markit-composition` depends on nothing but std. It must not
  depend on H0–H4/Horse-A, benchmark tooling, Markdown semantics, UI
  toolkits (GPUI/Electron), Mermaid/KaTeX, platform implementations, or
  any future application crate. `tests/dependency_firewall.rs` enforces
  this mechanically.
- Research code stays under `research/` even when a mechanism is later
  selected as a product backend: product code reaches it only through a
  dedicated adapter behind a product contract, and that adapter is NOT
  this crate's concern.
- Donor-provenance material for transplanted product code lives with the
  crate (e.g. `crates/markit-composition/PROVENANCE.md`), never under
  `research/`.

## Workspace rules

The root `Cargo.toml` is the product workspace: `members = ["crates/*"]`,
`exclude = ["research"]`. Adding a research tree as a member, or letting a
product crate path-depend on `research/**`, is a boundary violation. Any
workspace change must leave research dependency resolution, feature
selection, optimization profiles, and toolchain pins byte-identical.

## Reading a path

A future engineer must be able to classify any file from its path alone:

```text
research/**            → research evidence (historical or active campaign)
crates/**, apps/**     → production product code
docs/product/**        → product-facing contracts
docs/research/**       → research records
scripts/**             → repository automation
```

If a path does not make that clear, the layout — not the reader — is wrong.
