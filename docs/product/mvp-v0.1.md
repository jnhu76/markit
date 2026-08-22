# Markit — MVP v0.1 Scope

The first shippable product milestone: a single-document, L1-styled
Markdown editor on Windows, built in **Rust + direct GPUI** (ADR-008).
Windows is the first product platform; Linux and macOS follow in later
phases, each gated by the same acceptance on real hardware.

The functional MVP scope below is carried over from the A4-era product
definition; only the substrate changed (from the PocketJS foundation to
direct GPUI). The execution model is defined by
`docs/product/realtime-execution-model.md`: incremental, viewport-bounded,
revision-safe, demand-driven, and non-blocking for deferrable work.

The MVP does **not** ship a general plugin runtime, but its boundaries must
remain compatible with the future extension model in
`docs/product/plugin-compatibility-contract.md`. Built-in export/print or
other extension-like features should not force plugins to depend on
`markit-core` internals, GPUI entities, concrete Markdown IR memory layout,
or private scheduler/cache structures.

## In scope

```text
Platform:            Windows (first); Linux/macOS later, same core
Editing:             single document, UTF-8 (BOM detection if needed)
Files:               open / save / save-as, dirty state, atomic save
Markdown:            L1 styled editing (heading, paragraph, bold, emphasis,
                     inline code, link, blockquote, ul/ol list, fenced
                     code) with syntax visible
Editing primitives:  caret, selection, scroll, undo/redo (transactions,
                     typing/delete coalescing, IME-commit grouping)
Text:                Latin + CJK + emoji fallback (system font discovery)
IME:                 Chinese IME (composition model, candidate docking);
                     JA/KO architecture present, Chinese validated
Clipboard:           text copy/cut/paste
Shortcuts:           Copy/Paste/Undo/Redo/SelectAll/Save/Open/Find
                     (Ctrl on Win/Linux, Cmd on macOS via ShortcutPolicy)
Window:              resize, HiDPI, window-state restore (basic)
Execution:           explicit changed range + revision identity,
                     viewport-bounded materialization, demand rendering,
                     bounded/cooperative deferrable work, stale-result
                     rejection, coherent publication
Extension seam:      explicit commands/transactions, stable document/block
                     identity where exposed, coherent snapshot/revision
                     semantics; no public dependency on internal Rust/GPUI
                     representation
Stability:           large documents (1M+ measured flat), crash recovery
                     (minimal: periodic recovery snapshot + clean-shutdown
                     marker + startup recovery)
```

## Not in scope (v0.1)

```text
tabs / workspace / file tree   general plugin runtime / marketplace
images / tables / math / Mermaid
cloud sync / collaboration / Git integration
PDF export marketplace / extension store
rich HTML clipboard / images / custom MIME
transparent windows
legacy encodings (UTF-8 only)
full Typora-style syntax hiding (L2 is a later phase)
generic ECS / archetype / game-engine framework
permanent fixed-rate render/update loop
final worker-pool topology or hard-coded scheduler tuning copied from references
plugin transport choice (Rust dylib / Wasm / subprocess IPC / wire encoding)
```

## Acceptance gates (Windows first, in order)

1. Launch, render, resize, HiDPI — PASS on real hardware (no WSLg-only
   certification for Linux later).
2. Open/save/save-as round-trip byte-identical for UTF-8 (incl. BOM),
   atomic save (no torn file on kill -9 / TerminateProcess).
3. L1 editing: type, backspace, enter, arrows, home/end, selection,
   scroll, undo/redo — smoke-verified byte-identical state sequences.
4. CJK: Chinese text renders (system font discovery + fallback), Chinese
   IME composes, commits and cancels correctly; composition never enters
   the undo stack as keystrokes.
5. Clipboard: copy/cut/paste round-trip with the OS.
6. Performance invariants: `docs/product/performance-invariants.md`
   battery passes (work-amplification + scheduling/revision checks;
   calibrated real-host timing, not arbitrary CI wall-clock SLAs).
7. 1M-document stability: normal local-edit work stays effectively flat
   vs 10K except known/owned structural cases; GPUI presentation remains
   viewport-bound; idle editor does not continuously request frames.
8. Realtime scheduling: deferrable work can yield/resume without blocking
   caret/input/visible-text progress; current interaction and visible
   viewport outrank distant/background completion.
9. Revision safety: deliberately delayed/out-of-order background or
   deferred results cannot overwrite a newer document/presentation state.
10. Publication coherence: a visible frame cannot silently combine
    incompatible document/parse/layout/highlight revisions unless reused
    artifacts prove compatibility.
11. Observability: real-host performance runs expose p50/p95/p99 where
    statistically meaningful, max/long-frame counts, changed-region and
    viewport work, frame-work/yield counters, and stale/cancel/reject
    counts needed to diagnose scheduling failures.
12. Extension-boundary preservation: no MVP feature requires a future
    plugin to borrow mutable editor internals or link against GPUI/private
    `markit-core` representation; extension-like operations can be expressed
    through explicit snapshot/query + command/result seams compatible with
    the plugin compatibility contract.

## Exit criteria

- P1 (Windows): gates 1–12 on Windows with evidence, installer-free
  portable exe, crash recovery smoke.
- The exact numeric frame budget, batch size, worker count, overscan, and
  cache sizes are recorded as measured implementation parameters, not as
  architectural constants.
- Plugin runtime/transport remains a later evidence-driven decision; MVP
  freezes only the semantic boundary discipline, not an ABI.
- Later platforms: the same semantic gates on a real Linux desktop and on
  macOS, each in its own phase (roadmap P5), with platform-appropriate
  timing/presentation evidence.
