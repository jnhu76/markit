# Markit — Product Roadmap

Phase A* research naming ends with A4 (research phase closed). The
product phases below replace it. Each phase lists goal / scope /
acceptance / non-goals.

```text
Architecture Pivot / Repository Cleanup   ← this change (ADR-008)
        ↓
G0  GPUI Baseline Selection
        ↓
P0  Markit Rust Core
        ↓
P1  Windows Editor MVP + Markdown L1 (v0.1, direct GPUI)
        ↓
P2  Markdown L1 hardening
        ↓
P3  Typora-style L2
        ↓
P4  Rich blocks
        ↓
P5  Cross-platform / hardening
```

## G0 — GPUI Baseline Selection (next)

- Goal: select and pin a GPUI baseline suitable for Windows development.
- Scope: evaluate the current `mvp/gpui` prototype (gpui 0.2.2) and a
  current Zed GPUI revision on:

  ```text
  build
  release build
  window
  native text
  CJK
  IME
  clipboard
  resize
  HiDPI
  startup
  RSS
  basic latency
  ```

- Do not choose merely because newer is newer. Record evidence per item.
- The pinned GPUI revision is a dependency of the GPUI-facing crates
  (`markit-gpui` / the app crate, via the workspace dependency) — never
  of `markit-core` (P0 acceptance: core has no GPUI dependency).
- Acceptance: a documented, evidence-based GPUI baseline selection with
  build + Windows runtime evidence; the product's GPUI dependency (in
  `markit-gpui` / the app crate, not `markit-core`) is frozen to the
  selected revision.
- Non-goals: editor features, Linux/macOS hosts.

## P0 — Markit Rust Core

- Goal: framework-independent core built as a Rust library.
- Scope: `Document`, `LineIndex` (ADR-003), `Selection`,
  `EditTransaction` (undo/redo), `Commands` (ADR-006), `BlockIndex`,
  Markdown L1 (ADR-004), view model, explicit changed-range propagation,
  Unicode-aware coordinate semantics (AGENTS.md §8), and the regression
  battery (work-amplification invariants).
- Acceptance: core tests green (incl. randomized differentials vs
  full-scan oracles, caretFromX regression); invariants battery defined;
  core has no GPUI dependency.
- Non-goals: platform integration, packaging.

## P1 — Windows Editor MVP + Markdown L1 (v0.1)

- Goal: Markit Desktop v0.1 runs on Windows, built directly on GPUI: a
  single-document, L1-styled Markdown editor (`mvp-v0.1.md`).
- Scope (from `docs/product/mvp-v0.1.md`): window, editing, Markdown L1
  styled pipeline (ADR-004: heading, paragraph, bold, emphasis, inline
  code, link, blockquote, ul/ol list, fenced code), IME (Chinese
  composition model, ADR-007), clipboard (text), files (open/save/atomic
  save, crash recovery minimal), CJK + emoji fallback, undo/redo,
  shortcuts (Ctrl), resize/HiDPI, large-document stability (1M+ flat),
  regression battery in CI.
- Acceptance: `docs/product/mvp-v0.1.md` gates PASS on Windows with
  evidence; invariants battery green; no GPUI code leaked into
  `markit-core`.
- Non-goals: tabs, images/tables/math, installer/MSIX (later if needed),
  Linux/macOS.

## P2 — Markdown L1 Hardening

- Goal: harden the L1 pipeline shipped in P1 to product-grade conformance
  and robustness; the L1 feature set itself is already complete and
  shipped at P1 exit (v0.1).
- Scope: L1 conformance golden fixtures (CommonMark-derived where
  applicable), bounded fence recovery + parser checkpoints (issue
  backlog: bound the cascade, do not hide honest structural
  propagation), incremental-parser robustness across large structural
  edits.
- Acceptance: local-edit radius 1 block at any size; fence edits bounded
  and documented; differential oracle green; conformance fixtures green.
- Non-goals: syntax hiding (P3).

## P3 — Typora-style L2

- Goal: source-aware presentation (syntax hidden outside the active
  line, revealed on caret entry) without regressing the invariants.
- Scope: per-line syntax visibility, run-level virtualization,
  richer inline styling (weights/sizes).
- Acceptance: L2 editing measured against the invariants; local-edit
  radius still 1 block.
- Non-goals: rich blocks (P4).

## P4 — Rich Markdown Blocks

- Goal: images, tables, math, code highlighting as viewport-bounded
  projections (architecture.md §8).
- Scope: block-kind registry extension, lazy per-visible-block
  projection + caching, explicit invalidation.
- Acceptance: each block kind has a measured invalidation radius and
  stays out of the hot path.
- Non-goals: Mermaid/plugins (extensions, later).

## P5 — Cross-platform / hardening

- Goal: shipping quality on Windows, then Linux/macOS.
- Scope: Windows hardening first (packaging, crash reporting, recovery
  maturity, performance regression CI, accessibility basics, i18n); then
  Linux (Wayland/X11, fontconfig, IBus/Fcitx) and macOS (CoreText, IME,
  clipboard, Cmd, .app bundle) — each gated by the same MVP acceptance
  on real hardware.
- Acceptance: release-ready on each platform it claims; Linux/macOS are
  not blocked by Windows hardening, but each platform must pass its own
  MVP gates on real hardware.
- Non-goals: store submission specifics until a platform is release-ready.

## Working rule (the anti-foundation rule, from A4 §59)

> Once a foundation is sufficient for the next product feature, stop
> improving the foundation and build the product feature.

Real product workload is the only judge that can reopen research (e.g.
an A5-style investigation) — a synthetic benchmark alone cannot. Do not
let platform work prevent building the editor once the Windows foundation
is adequate.
