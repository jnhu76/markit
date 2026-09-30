# Product Foundation Rules

Status: active product documentation (Issue #106, MARKIT-FOUNDATION-0,
foundation harvest slice). This is a boundary and convention record only;
it grants no implementation authority and reopens no architecture
decision. `AGENTS.md` remains the authority guard.

## Rules

1. **K0 owns composition/reachability/lifetime.**
   `crates/markit-composition` is the only composition mechanism; nothing
   else keeps parallel composition truth.
2. **`markit-app` is a thin composition root.** It admits components and
   desired compositions into K0, exposes diagnostics, and performs
   explicit disposal. It defines no product component and constructs no
   resource.
3. **Plugins publish stable roles through `ComponentSpec`.** Plugin
   identity is a stable product/composition role, frozen by the
   constructor convention:

   ```rust
   pub fn xxx_plugin(...) -> ComponentSpec
   ```

   The role, not the implementation brand, is the composition truth.
4. **Capabilities are contracts; concrete backends remain private.**
   Consumers bind to a `Capability`, never to a backend type.
5. **Product infrastructure must not depend on research.**
   `research/**` is outside the product workspace and the product
   architecture (`docs/product/repository-layout.md`,
   `tools/check_product_boundaries.py`).
6. **Domain semantics belong to future domain crates/plugins.** They will
   be ordinary downstream capabilities; they do not join K0 and never
   make `markit-app` domain-aware.
7. **UI/toolkit/backend selection is downstream of product contracts.**

## Worked convention (not an implementation obligation)

```text
markdown_plugin
    provides MarkdownEngineCapability
        backed today by one engine, later by another

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
