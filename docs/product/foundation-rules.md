# Product Foundation Rules

Status: active product foundation documentation (Issue #106, MARKIT-FOUNDATION-0).
These rules define the already-adopted generic K0/plugin/provider foundation and are
**subordinate to `docs/product/architecture.md`**, which is the normative product
architecture authority. This document does not grant parser-research authority or
permit foundation mechanisms to override the Markdown data-plane boundaries.

## Rules

1. **K0 owns composition/reachability/lifetime.**
   `crates/markit-composition` is the only generic composition mechanism; nothing
   else keeps parallel composition truth. Domain objects may own their own concrete
   state/tasks/resources under the owner chain defined by the product architecture;
   that does not create a second generic lifecycle framework.
2. **`markit-app` is a thin composition root.** It admits components and
   desired compositions into K0, exposes diagnostics, and performs
   explicit disposal. It defines no product component and constructs no
   domain resource on behalf of feature plugins.
3. **Plugins publish stable roles through `ComponentSpec`.** Plugin
   identity is a stable product/composition role, frozen by the
   constructor convention:

   ```rust
   pub fn xxx_plugin(...) -> ComponentSpec
   ```

   The role, not the implementation brand, is the composition truth.
4. **Capabilities are contracts; concrete backends remain private.**
   Consumers bind to a `Capability`, never to a backend-private type.
5. **Product infrastructure must not depend on research.**
   `research/**` is outside the product workspace and the product
   architecture (`docs/product/repository-layout.md`,
   `tools/check_product_boundaries.py`). Product adapters may reuse an
   admitted mechanism only behind the product contract and without exposing
   research representation to consumers.
6. **Domain semantics belong to downstream domain roles/objects.** They are
   ordinary typed capabilities/objects below K0; they do not join K0 and never
   make `markit-app` domain-aware. In particular, Markdown edit/query traffic
   follows the direct typed data plane defined by `architecture.md`, not generic
   composition dispatch.
7. **UI/toolkit/backend selection is downstream of product contracts.**
   UI Slots are presentation placement only; they do not become behavioral or
   Markdown-data authorities.

## Worked convention (not an implementation obligation)

```text
markdown_plugin
    provides Markdown semantic capability
        initially backed by an H4 product adapter
        replaceable later by another compatible backend

filesystem_plugin
    (not windows_filesystem_plugin)
```

A future backend swap must not change composition identity; if the brand
itself is product semantics, the role may say so — deliberately.

## Enforcement

`tools/check_product_boundaries.py` (+ CI `.github/workflows/
product-boundary-gate.yml`) enforces the admitted crate universe, the
admitted internal dependency edges, the production→research prohibition,
exclude-list admission, and a narrow donor-contamination scan — each
fail-closed, each proven non-vacuous by reversible negative controls.

The boundary gate enforces topology. It does **not** replace domain correctness
tests for revision identity, pinned reads, stale-result rejection, Print
completeness, or UI Slot ownership; those tests belong with the concrete domain
slice that introduces each behavior.
