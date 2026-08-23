# ADR-008 — Direct GPUI as the Markit desktop product substrate

Status: accepted (2026-08-18).

Supersedes: ADR-001 (PocketJS as primary UI/runtime substrate).

## Prior state

A1–A4 compared PocketJS and GPUI as candidate desktop editor substrates
on parity MVPs with a shared workload contract.

PocketJS had been selected as the product foundation (ADR-001, A4) because
of:

- guest-side product logic (editor behavior in JS/TS on the PocketJS stack);
- cross-platform runtime control via one retained UI tree + DrawList contract;
- understood rendering costs after A2–A4 decomposition (Solid residual
  reduced to ~1.0–1.5 ms, lower stack ~0.3 ms);
- one runtime abstraction across Windows/Linux/macOS.

GPUI was kept as the reference/performance oracle. Its measured advantages
(active-edit latency, startup, memory) were documented, not treated as a
product argument at that time.

## New information

After the PocketJS desktop audit (A1 pocketjs-windows phase) and the
subsequent product review, the following became clear:

- PocketJS's modern desktop path itself uses GPUI for the platform work:
  window, rendering, native text, keyboard, pointer, IME and clipboard
  integration ultimately go through GPUI.
- Markit does not need PocketJS's QuickJS / DrawList / companion /
  capability runtime layers merely to build a native Markdown editor.
- Those intermediate abstractions increase architecture surface and
  attribution complexity: every latency question has to be traced through
  guest → reactive layer → retained tree → DrawList → host → GPUI instead
  of directly through the GPUI pipeline.
- Markit's primary product principle is:

```text
Nothing gets between your input and the next frame.
```

An intermediate runtime between Markit and the platform works against
that principle when the platform path it abstracts is itself GPUI.

## Decision

```text
Markit product = Rust + direct GPUI
```

with:

```text
Rust-native editor core (markit-core)
+
GPUI UI/platform layer (markit-gpui/app)
```

Windows remains the first product platform.

This is an architecture decision, not a claim that GPUI wins every
benchmark. The A1–A4 measurements remain valid within their original
setup; this ADR changes the product abstraction boundary, not the
historical evidence.

## Consequences

### Positive

- smaller runtime stack (no QuickJS, no retained UI tree, no DrawList
  contract, no host/svc bridge);
- fewer intermediate representations between document and pixels;
- easier profiling and causal attribution;
- direct access to GPUI native text/input/IME/clipboard/window semantics;
- closer relationship to Zed's real editor workload (Zed remains a
  reference implementation, not proof — see AGENTS.md);
- PocketJS no longer blocks Markit in any roadmap sense.

### Negative

- Rust iteration cost for editor logic;
- GPUI is pre-1.0; upstream churn may require migration;
- Markit must own its editor architecture directly (no guest-side
  iteration shortcut);
- multi-platform portability depends more directly on GPUI maturity;
- the incremental Markdown / view-model knowledge from A4 must be
  re-expressed in the Rust core (see `docs/research/`).

## Dependency policy

Markit SHOULD pin an explicitly tested GPUI version/revision once a
baseline-selection experiment establishes it.

Do NOT inherit GPUI 0.2.2 merely because the PocketJS host used 0.2.2.

The current `mvp/gpui` prototype (gpui 0.2.2) is evidence of Windows
feasibility, not the final product dependency. GPUI baseline selection is
the next experiment (roadmap G0); at this ADR's acceptance the product
GPUI baseline was NOT YET FROZEN (superseded by the status update below).

### Status update (2026-08-22, G0 closed)

The baseline is now **frozen**: `zed-industries/zed` rev
`eb8e1c8b5502b7007465fbbc465f4a736fa39210` (Zed v1.16.1), pinned in the
workspace root `Cargo.toml`. Evidence, capability audit, provenance, and
the update policy live in `docs/product/g0-gpui-baseline.md`. Note: the
pinned revision's `gpui` crate still reports version string "0.2.2" while
being an entirely different implementation (platform code split into
`gpui_platform`/`gpui_windows`/…) — version strings are not baseline
identity; the commit SHA is. The frozen baseline is validated on a real
Windows desktop host (G0 smoke matrix, `results/summary/g0/`): build,
window, resize, HiDPI@1.0, Latin/CJK/emoji rendering, keyboard/mouse,
IME composition pipeline (partial), clipboard both directions, demand
redraw, idle behavior (0 draws / 0 % CPU), startup, and memory all PASS
or honestly PARTIAL as recorded.

## Relationship to ADR-001

ADR-001 remains a valid historical decision record: it truthfully
records that A4 selected PocketJS at that time. It is superseded by this
ADR because the underlying assumption (PocketJS provides the platform
abstraction Markit needs) no longer holds — PocketJS itself delegates
that abstraction to GPUI.
