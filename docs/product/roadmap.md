# Markit Desktop — Product Roadmap

Phase A* research naming ends here (A4 closes the research phase). The
product phases below replace it. Each phase lists goal / scope /
acceptance / non-goals.

## P0 — Product Foundation

- Goal: architecture frozen enough to implement; shared-core skeleton;
  platform interfaces; L1 pipeline prototype; capability matrix;
  Windows next-phase blockers explicit.
- Scope: docs (`docs/product/*`, ADRs 001–007), core boundaries
  (Document/LineIndex/BlockIndex/EditTransaction/Selection/Command/
  ViewModel/Platform interfaces), issue backlog.
- Acceptance: architecture doc answers the ten questions;
  performance-invariants battery defined; capability matrix evidence-
  based; issue backlog ready-to-paste. (A4 delivered this — the phase is
  the A4-P portion.)
- Non-goals: platform implementation, packaging.

## P1 — Windows Desktop MVP

- Goal: Markit Desktop v0.1 runs on Windows (`docs/product/mvp-v0.1.md`
  gates 1–7).
- Scope: markit-core refactor of the MVP guest (editor.ts/markdown.ts
  become the core modules), Windows font discovery + CJK/emoji fallback,
  text clipboard, IME model + Pinyin validation, native file dialogs,
  atomic save, undo/redo transactions, crash recovery minimal,
  Ctrl shortcuts, portable exe, regression battery in CI.
- Acceptance: mvp-v0.1 gates 1–7 PASS on Windows with evidence;
  invariants battery green.
- Non-goals: tabs, images/tables/math, installer/MSIX (later if needed),
  Linux/macOS.

## P2 — Linux Desktop MVP

- Goal: the same v0.1 on a real Linux desktop.
- Scope: Wayland host (X11 fallback), fontconfig discovery, IBus/Fcitx
  IME, xdg portal file dialogs, XDG dirs, portable binary + desktop
  entry.
- Acceptance: mvp-v0.1 gates 1–7 PASS on real Linux hardware (WSLg runs
  are smoke only).
- Non-goals: AppImage (later), Windows regression.

## P3 — macOS Desktop MVP

- Goal: the same v0.1 on macOS.
- Scope: Metal/wgpu validation, CoreText fonts, IME, clipboard, Cmd
  shortcuts, native dialogs, menu bar, window behavior, Retina, .app
  bundle.
- Acceptance: mvp-v0.1 gates 1–7 PASS on real hardware.
- Non-goals: codesign/notarization (later), store submission.

## P4 — Markdown Visual Editing L2

- Goal: Typora-like source-aware presentation (syntax hidden outside the
  active line, revealed on caret entry) without regressing the
  invariants.
- Scope: L2 view model (per-line syntax visibility), run-level
  virtualization per the Incremental View Model, bounded fence recovery
  (the R2 structural-edit cost), richer inline styling (weights/sizes
  via font slots).
- Acceptance: L2 editing measured against INV-01…07; local-edit radius
  still 1 block; fence edits bounded and documented.
- Non-goals: rich blocks (P5).

## P5 — Rich Markdown Blocks

- Goal: images, tables, math, code highlighting, as viewport-bounded
  projections (architecture.md §10).
- Scope: block-kind registry extension, lazy per-visible-block
  projection + caching, explicit invalidation.
- Acceptance: each block kind has a measured invalidation radius and
  stays out of the hot path.
- Non-goals: Mermaid/plugins (extensions, later).

## P6 — Product Hardening

- Goal: shipping quality.
- Scope: packaging (installer/MSIX, AppImage, codesign/notarization),
  crash reporting, recovery maturity, performance regression CI on all
  three platforms, accessibility basics, i18n.
- Acceptance: release-ready on all three platforms.

## Working rule (the anti-foundation rule, A4 §59)

> Once a foundation is sufficient for the next product feature, stop
> improving the foundation and build the product feature.

Real product workload is the only judge that can reopen research (e.g.
an A5-style investigation) — a synthetic benchmark alone cannot.
