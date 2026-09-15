# Markit pre-product-reset archive

This directory marks the authority boundary for **MARKIT-PRODUCT-RESET-0**.

The pre-reset repository state is preserved by Git at:

```text
d7837fcfa95a58d8cf3a6063bc0f7d6ce5f9e91e
```

That revision contains the previous product/research documents, GPUI experiments, benchmark work, and the then-current implementation. Nothing from that revision is deleted as evidence. It is retained as **historical/experimental input**, not as current product authority.

In particular, the pre-reset versions of these documents are historical records:

- `README.md`
- `docs/PRD.md`
- `docs/product/architecture.md`
- `docs/product/mvp-v0.1.md`
- `docs/product/roadmap.md`
- the A0-A4 research documents, benchmark results, and GPUI/PocketJS comparison material

The existing implementation at the archive revision is also treated as an **experimental/reference implementation**. Components may be reused only after they are checked against the post-reset product contracts; prior existence is not sufficient justification for inclusion.

## Post-reset authority

After this reset, product intent is defined in this order:

```text
docs/PRD.md
  -> docs/product/architecture.md
  -> docs/product/print-browser-contract.md
  -> docs/product/mvp-v0.1.md
  -> docs/product/roadmap.md
  -> implementation issues / code
```

`README.md` is an entry point, not an independent source of product truth.

Any older document that conflicts with the post-reset authority above is historical until it is explicitly re-adopted.