# Markit Desktop — Issue Backlog (P1+ candidates)

Ready-to-paste issue bodies for work confirmed by product requirements or
real evidence (no speculative issues). Each entry: title, labels, body.
The A4 phase spec §55 rule applies: only items the product will certainly
need or that have real evidence.

---

## P1 — Windows

### [P1] Windows system font discovery + CJK/emoji fallback chain

- Labels: `p1`, `windows`, `fonts`, `tier-0`
- Body:
  - **Why**: Markit targets Chinese Markdown editing; CJK currently
    renders as tofu on the Windows MVP (A1 gap, capability matrix FAIL).
  - **Evidence**: `docs/phase-a1-pocketjs-windows.md` (CJK DEFERRED);
    `docs/product/platform-capability-matrix.md`.
  - **Scope**: enumerate system fonts (GDI), build a Latin → CJK → emoji
    fallback chain, feed the PocketJS runtime-glyph path
    (`text.glyphs.runtime`); do NOT hardcode font paths.
  - **Acceptance**: Chinese/emoji text renders correctly; the fallback
    chain is data-driven per platform.
  - **Non-goals**: rich text shaping (U3+), font settings UI.

### [P1] Windows text clipboard (copy/cut/paste)

- Labels: `p1`, `windows`, `clipboard`, `tier-0`
- Body:
  - **Why**: copy/cut/paste is a Tier-0 product requirement; the protocol
    reserves it (`Copy`/`Cut` keys + `{t:"paste"}`) but nothing implements
    it.
  - **Evidence**: `docs/product/platform-capability-matrix.md` (clipboard
    NOT TESTED everywhere).
  - **Scope**: `ClipboardProvider` (windows impl: CF_UNICODETEXT),
    wire Ctrl+C/X/V through the host, text-only first.
  - **Acceptance**: copy/cut/paste round-trips with the OS and other
    apps.
  - **Non-goals**: rich HTML, images, custom MIME.

### [P1] Windows IME composition model + Chinese validation

- Labels: `p1`, `windows`, `ime`, `tier-0`
- Body:
  - **Why**: Chinese IME is an MVP gate; the model (ADR-007) and the wire
    contract exist, the implementation does not.
  - **Evidence**: `docs/adr/ADR-007-ime-composition-model.md`;
    capability matrix (IME NOT TESTED).
  - **Scope**: implement composition start/update/commit/cancel in the
    editor model, candidate docking at the caret rect, commit as one undo
    transaction; validate Pinyin on Windows.
  - **Acceptance**: Chinese composition works (commit, cancel, undo
    grouping); no composition text in the undo stack as keystrokes.

### [P1] Windows native file dialogs (open/save)

- Labels: `p1`, `windows`, `files`
- Body:
  - **Why**: open/save dialogs are MVP scope.
  - **Scope**: `FileDialogProvider` (windows: IFileDialog), wire
    Open/Save/Save-As commands.
  - **Acceptance**: native dialogs open/save UTF-8 files; dirty-state
    flow works.
  - **Non-goals**: custom dialog UI, recent-files UI (later).

### [P1] Atomic save + minimal crash recovery

- Labels: `p1`, `files`, `reliability`
- Body:
  - **Why**: a product must not corrupt files on crash (write → tmp →
    fsync → rename; Windows vs POSIX rename semantics).
  - **Scope**: atomic save path, periodic recovery snapshot, clean-
    shutdown marker, startup recovery prompt.
  - **Acceptance**: kill -9/TerminateProcess mid-save leaves the original
    file intact; recovery restores the last snapshot.

### [P1] Undo/redo transactions (EditTransaction)

- Labels: `p1`, `editor-model`
- Body:
  - **Why**: undo must group typing/delete/paste/IME-commit, not snapshot
    per key.
  - **Evidence**: ADR-007 (IME grouping); A4 architecture Layer 3.
  - **Scope**: `EditTransaction` in markit-core, typing/delete
    coalescing, paste and IME-commit as single transactions.
  - **Acceptance**: standard undo/redo UX for typing, deletion, paste,
    IME commits; bounded memory.

---

## P2 — Linux

### [P2] Real Linux desktop host validation (Wayland + X11 fallback)

- Labels: `p2`, `linux`
- Body:
  - **Why**: WSLg smoke runs do not certify the Linux product runtime.
  - **Evidence**: `docs/product/platform-capability-matrix.md` (all Linux
    real-desktop rows NOT TESTED).
  - **Scope**: run the MVP on a real Wayland session (X11 fallback),
    input/scroll/resize/GPU; fix host gaps.
  - **Acceptance**: MVP gates 1–7 PASS on real Linux hardware with
    evidence recorded.

### [P2] Linux fontconfig discovery + IBus/Fcitx IME

- Labels: `p2`, `linux`, `fonts`, `ime`
- Body:
  - **Why**: Linux needs fontconfig for CJK and IBus/Fcitx for Chinese
    IME; both are product gates.
  - **Scope**: `FontProvider` (fontconfig), `ImeProvider` (IBus/Fcitx via
    the composition model), XDG config/data/cache dirs.
  - **Acceptance**: CJK renders; Chinese IME composes/commits on a real
    Linux desktop.

### [P2] Linux file dialogs + packaging (portable binary + desktop entry)

- Labels: `p2`, `linux`, `files`, `packaging`
- Body:
  - **Why**: MVP gates need open/save and a launchable install.
  - **Scope**: `FileDialogProvider` (xdg portals), portable binary +
    desktop entry (AppImage later).
  - **Acceptance**: native dialogs work; the binary launches from the
    desktop entry.

---

## P3 — macOS

### [P3] macOS validation battery

- Labels: `p3`, `macos`
- Body:
  - **Why**: PocketJS's macOS-first heritage is not product evidence;
    every capability row is NOT TESTED.
  - **Evidence**: `docs/product/platform-capability-matrix.md`.
  - **Scope**: run MVP gates 1–7 on real hardware: Metal/wgpu, CoreText
    fonts, IME, clipboard, Cmd shortcuts, native dialogs, menu bar,
    window behavior, Retina, .app bundle.
  - **Acceptance**: gates PASS with evidence; remaining gaps filed
    separately.

---

## Cross-platform

### [X] Bounded fence recovery for L1 structural edits

- Labels: `markdown`, `performance`, `p4`
- Body:
  - **Why**: A fence-boundary edit invalidates through the whole fence
    cascade (1M: 30 197 lines, 68.9 ms — measured). Correct, but too
    broad for a product hot path.
  - **Evidence**: `docs/phase-a4-final-research-closeout.md` §3.4.
  - **Scope**: bound the recovery (e.g. only treat ``` as an opener when
    a matching close exists ahead), keep the incremental rescan correct
    (differential oracle), re-measure M5 at 10K/100K/1M.
  - **Acceptance**: M5 invalidation radius bounded and documented;
    invariants battery green.

### [X] markit-desktop target identity (PocketJS upstream proposal)

- Labels: `pocketjs-upstream`, `identity`, `p4`
- Body:
  - **Why**: the Markit runtime must not impersonate `macos-widget`;
    identity and capability are separate concepts (A4 §26–§27).
  - **Evidence**: `contracts/spec/platforms.ts` (macos-widget =
    platform macos, form widget); `docs/product/architecture.md` §11.
  - **Scope**: propose a `markit-desktop` target (or desktop + runtime
    capability profile) with a Markit-independent reproduction and a
    regression test, per the upstream strategy (A4 §57).
  - **Acceptance**: vendor registry proposal reviewed upstream; Markit
    builds declare their own identity.

### [X] caretFromX click regression test

- Labels: `editor-model`, `tests`
- Body:
  - **Why**: the pre-existing click-placement bug (`lineIndex * doc.length`
    sent clicks on lines ≥ 1 to the document end) corrupted position
    cells until A4; it needs a permanent regression test.
  - **Evidence**: `docs/phase-a4-final-research-closeout.md` §11.2.
  - **Scope**: unit test for caretFromX at several line indexes + a
    headless click-caret smoke on a multi-line document.
  - **Acceptance**: test fails on the old formula, passes on the fix.

### [X] Performance regression battery in CI

- Labels: `ci`, `performance`
- Body:
  - **Why**: the invariants (`docs/product/performance-invariants.md`)
    need cheap guards: work-amplification checks, not wall-clock
    thresholds.
  - **Scope**: run `bench/run-a4.py` cells (r1-scale, r1-ops, r2-case)
    in CI on a fixed machine; assert full_scans == 0, blocks_reparsed ==
    1 for local edits, words flat across sizes.
  - **Acceptance**: green on every PR touching core/edit paths; flake
    policy documented.
