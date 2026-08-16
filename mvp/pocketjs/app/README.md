# mvp/pocketjs/app — Markit core seed & guest modules

Module map against the A4-P product layers (`docs/product/architecture.md`).
These modules are framework-free (pure TS, bun-testable) — they are the
seed of `markit-core`; the PocketJS UI layer consumes them and never
stores editor state in components.

| Layer | Module | Contents |
|-------|--------|----------|
| L1 Text Storage | `editor.ts` | `Document` (string), incremental `LineIndex` (A3-P1), caret/selection math, edit functions with explicit changed-range propagation (`EditResult.change`) |
| L2 Markdown Structure | `markdown.ts` | `BlockIndex` (incremental L1 rescan, stable-boundary stop, fence state), `scanBlocksFull` (oracle), `parseInline` (styled runs), `classifyLine` |
| L3 Editor Model | `app.tsx` / `md-app.tsx` (UI-side glue) | caret/selection state, key/mouse handling, commands; EditTransaction (undo/redo) is a P1 product item (ADR-007) |
| L4 View Model | visible-range memos in `app.tsx`/`md-app.tsx` | visible line range (viewport formula + overscan), styled-run slicing, stable-item discipline (A4-R1) |
| L5 PocketJS Presentation | JSX in `app.tsx`/`md-app.tsx` | components → ui.* ops → DrawList; nothing here is the document database |
| L6 Platform Adapter | `platform.ts` (contracts), `svc.ts` (host protocol), `src/main.rs` (host) | Clipboard/Font/Ime/FileDialog/Shortcut/Paths contracts; svc wire; host implementation |

Diagnostic bundles (A4-R, not product): `cf-boot.ts`/`cf.ts`/
`cf-notext.ts` (R1 counterfactual — byte-identical DrawList), `main-ops.tsx`/
`cf-ops.ts` (op-count runs), `md-app.tsx`/`md.tsx` (R2 L1 pipeline
measurement surface).

Tests: `line-index.test.ts` (38), `markdown.test.ts` (10, incl. 5000-edit
randomized differential vs the full-scan oracle).

## Product rule (from A4-R1)

Visible-list items must carry **stable identity** (cached by position)
and every doc-dependent read must be hoisted into **item-scoped memos** —
fresh item objects per render re-mount the whole visible list per edit
(~90 native node creations per edit, measured). This is the seed of the
product's Incremental View Model (Layer 4).
