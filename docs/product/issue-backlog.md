# Markit — Issue Backlog (P0/P1+ candidates)

Ready-to-paste issue bodies for work confirmed by product requirements or
real evidence (no speculative issues). Each entry: title, labels, body.
The A4 phase spec §55 rule applies: only items the product will certainly
need or that have real evidence. Items are written for the direct-GPUI
architecture (ADR-008) and the real-time execution model in
`docs/product/realtime-execution-model.md`.

---

## P0 — Core execution semantics

### [P0] Revision identity + precise dirty propagation in markit-core

- Labels: `p0`, `editor-model`, `performance`, `correctness`
- Body:
  - **Why**: changed-range propagation exists as a product invariant, but
    the real-time execution model also requires explicit revision identity
    and precise downstream invalidation so deferred work can be made safe.
  - **Scope**: define the smallest core semantics needed to distinguish
    local edit / append / delete / paste / structural edit / document
    replacement / viewport/style changes where those distinctions affect
    invalidation; propagate dirty regions through LineIndex, BlockIndex,
    Markdown IR, view model; add document/derived-state revision identity.
  - **Acceptance**: local edits invalidate only semantically affected
    blocks; append is not treated as document replacement; tests can
    observe dirty ranges/revisions without GPUI; old revision results are
    rejectable by construction; no speculative ECS/general dependency
    framework.
  - **Non-goals**: worker pool, numeric frame budget, final buffer
    structure.

### [P0] Work-amplification + revision instrumentation seams

- Labels: `p0`, `performance`, `instrumentation`
- Body:
  - **Why**: the execution laws are not useful if hidden fan-out cannot be
    measured.
  - **Scope**: expose counters/events for changed bytes/lines/blocks,
    blocks rescanned/reparsed, revision, and derived-result acceptance /
    stale rejection in headless/core tests.
  - **Acceptance**: regression tests can assert INV-01/05/08/10/13 without
    wall-clock thresholds; instrumentation can be disabled or made cheap
    outside performance builds.

---

## P1 — Windows (direct GPUI)

### [P1] Demand-driven frame scheduler + user-observable priority

- Labels: `p1`, `performance`, `gpui`, `scheduler`, `tier-0`
- Body:
  - **Why**: viewport-bounded rendering alone does not prevent syntax,
    layout, indexing, or other derived work from monopolizing the UI path.
    P1 requires bounded/cooperative work with current interaction and the
    visible viewport ahead of background completion.
  - **Scope**: implement the smallest GPUI-facing scheduler seam that can
    request frames on demand, stop/yield deferrable work at a measured
    deadline/quota, resume later, and expose critical/high/near/background
    priority. No permanent fixed-rate tick.
  - **Evidence**: `realtime-execution-model.md`; Markstream is an external
    reference for adaptive/incremental scheduling discipline, not for
    numeric defaults.
  - **Acceptance**: idle editor does not continuously request frames;
    sustained typing keeps caret/visible text ahead of background work;
    frame work/yield/queue-depth counters are observable; exact budget is
    calibrated on real Windows/GPUI evidence.
  - **Non-goals**: generic game engine, ECS, copied 6 ms/8 ms constants.

### [P1] Revision-safe deferred jobs + coherent presentation commit

- Labels: `p1`, `correctness`, `performance`, `scheduler`
- Body:
  - **Why**: background/deferred parse, highlight, layout, indexing, and
    later rich-block work can finish out of order. Stale results must never
    overwrite newer visible state.
  - **Scope**: define job revision/dependency identity, cancellation or
    stale-result rejection, and a logical committed-presentation boundary.
    Build a deterministic test harness that deliberately completes old
    jobs after new edits.
  - **Acceptance**: stale results cannot commit; compatible cached results
    can be reused; visible state cannot silently mix incompatible
    document/parse/layout/highlight revisions; cancellation/rejection count
    is observable.

### [P1] Viewport / Document LOD seam

- Labels: `p1`, `view-model`, `performance`, `virtualization`
- Body:
  - **Why**: ADR-005 bounds presentation by the viewport; the real-time
    model also needs far/near/visible priority so distant derived work does
    not become synchronous typing cost.
  - **Scope**: represent enough view-model state to distinguish far / near
    / visible materialization; preserve logical scroll extent; visible
    content gets exact layout/shaping, near content may prefetch, far
    content remains lightweight.
  - **Acceptance**: 10K→1M normal frame materialization remains viewport-
    bounded; far content does not force exact layout; counters report
    far/near/visible materialization; no visible scroll jump in the basic
    P1 path.
  - **Non-goals**: final height-estimation algorithm (P2 hardening).

### [P1] GPUI/DirectWrite CJK + emoji fallback validation for Markit

- Labels: `p1`, `windows`, `fonts`, `tier-0`
- Body:
  - **Why**: Markit targets Chinese Markdown editing; CJK must render
    correctly through the chosen GPUI baseline on Windows.
  - **Evidence**: `mvp/gpui` (Phase A0) verified Chinese via DirectWrite
    fallback in the feasibility prototype; product acceptance requires a
    validation on the selected GPUI baseline.
  - **Scope**: system font discovery + fallback chain (Latin → CJK →
    emoji) through GPUI's text system; do NOT hardcode font paths;
    document any GPUI text-atlas / fallback gaps found.
  - **Acceptance**: Chinese/emoji text renders correctly; the fallback
    behavior is data-driven and recorded; any GPUI gap is filed upstream
    with a Markit workaround note.
  - **Non-goals**: rich text shaping (U3+), font settings UI.

### [P1] Windows text clipboard (copy/cut/paste)

- Labels: `p1`, `windows`, `clipboard`, `tier-0`
- Body:
  - **Why**: copy/cut/paste is a Tier-0 product requirement; GPUI
    exposes clipboard on Windows but the Markit path (commands →
    GPUI clipboard → OS) must be validated end-to-end.
  - **Evidence**: `docs/product/platform-capability-matrix.md`
    (clipboard NOT TESTED on the product path).
  - **Scope**: wire Copy/Paste/Cut commands through GPUI's Windows
    clipboard; text-only first.
  - **Acceptance**: copy/cut/paste round-trips with the OS and other
    apps on the pinned GPUI baseline.
  - **Non-goals**: rich HTML, images, custom MIME.

### [P1] Windows IME composition model + Chinese validation

- Labels: `p1`, `windows`, `ime`, `tier-0`
- Body:
  - **Why**: Chinese IME is an MVP gate; the model (ADR-007) is defined,
    the GPUI Windows IME path must be validated end-to-end.
  - **Evidence**: `docs/adr/ADR-007-ime-composition-model.md`;
    capability matrix (IME NOT TESTED).
  - **Scope**: implement composition start/update/commit/cancel in
    `markit-core`, candidate docking at the caret rect, commit as one
    undo transaction; validate Pinyin on Windows through GPUI's IME
    integration (IMM32/TSF).
  - **Acceptance**: Chinese composition works (commit, cancel, undo
    grouping); no composition text in the undo stack as keystrokes.

### [P1] Windows native file dialogs (open/save)

- Labels: `p1`, `windows`, `files`
- Body:
  - **Why**: open/save dialogs are MVP scope; GPUI's dialog story must be
    validated on Windows (or a thin platform adapter added at the GPUI
    edge — not a Markit-private OS layer).
  - **Scope**: wire Open/Save/Save-As commands to native dialogs; if
    GPUI does not provide them on Windows, a minimal `FileDialogProvider`
    adapter at the GPUI edge.
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
  - **Evidence**: ADR-007 (IME grouping); architecture Layer (core).
  - **Scope**: `EditTransaction` in markit-core, typing/delete
    coalescing, paste and IME-commit as single transactions.
  - **Acceptance**: standard undo/redo UX for typing, deletion, paste,
    IME commits; bounded memory.

### [P1] Markit core + realtime regression battery in CI

- Labels: `ci`, `performance`, `editor-model`
- Body:
  - **Why**: `docs/product/performance-invariants.md` now covers both
    work amplification and scheduling/revision correctness.
  - **Scope**: core unit tests + invariant battery; assert full scans == 0,
    blocks_reparsed == 1 for local edits, presentation work flat across
    sizes, stale-result rejection/coherent publication, and idle demand
    rendering where deterministic host semantics permit. Keep fragile real
    wall-clock gates out of generic CI.
  - **Acceptance**: green on every PR touching core/edit paths; real-host
    timing battery kept separate; flake policy documented.

---

## P2 / cross-platform core

### [P2] Cache dependency + invalidation contracts

- Labels: `p2`, `performance`, `correctness`, `cache`
- Body:
  - **Why**: parse/highlight/layout/shaping caches are useful only if
    stable work is reused and stale work can never leak into presentation.
  - **Scope**: for each cache used by P1/P2, document key, dependency set,
    invalidation trigger, revision compatibility, memory bound/eviction,
    and hit/miss instrumentation. Add tests showing an unrelated local edit
    preserves reusable artifacts while affected artifacts invalidate.
  - **Acceptance**: no opaque cache; stale cache result cannot commit;
    memory bound documented; unrelated cache reuse verified.

### [P2] Document LOD height correction + scroll-drift hardening

- Labels: `p2`, `performance`, `scroll`, `virtualization`
- Body:
  - **Why**: keeping far content lightweight may require estimated extents;
    a CPU win is not acceptable if exact layout causes visible scroll
    jumps later.
  - **Scope**: measure far/near/visible height estimation, correction,
    anchor preservation, fast-scroll behavior, and after-large-navigation
    cache state.
  - **Acceptance**: drift/jump metric defined; correction remains within
    accepted bounds under large documents and rapid navigation; no hidden
    full-document layout.

### [X] Bounded fence recovery for L1 structural edits

- Labels: `markdown`, `performance`, `p2`
- Body:
  - **Why**: A fence-boundary edit invalidates through the whole fence
    cascade (measured at 1M: 30 197 lines, 68.9 ms). Correct, but too
    broad for a product hot path. The worst case must not propagate
    with unbounded document size.
  - **Evidence**: `docs/phase-a4-final-research-closeout.md` §3.4
    (historical measurement).
  - **Scope**: bound the recovery — candidates are parser checkpoints
    (every N blocks) + parser restart state at the checkpoint + scan
    until the state converges, and/or only treating ``` as an opener
    when a matching close exists ahead. Keep the incremental rescan
    correct (differential oracle); **do not optimize the honest
    structural propagation away** — fence delimiters legitimately change
    later structure, so the fix is *bounding* the cascade, not hiding it.
    Re-measure at 10K/100K/1M.
  - **Acceptance**: invalidation radius bounded (no O(document) worst
    case at 1M) and documented; differential oracle still green;
    invariants battery green; before/after recorded honestly.

### [X] Markdown L1 conformance golden fixtures

- Labels: `markdown`, `tests`, `p0`
- Body:
  - **Why**: the differential oracle proves *incremental invalidation
    correctness* (incremental == full scan of the same parser), not
    Markdown/CommonMark conformance — both sides could parse wrong
    together. The parser is the Markit L1 subset (heading, paragraph,
    bold, emphasis, inline code, link, blockquote, ul/ol list, fenced
    code), not a general Markdown parser.
  - **Evidence**: `docs/phase-a4-final-research-closeout.md` §3.2
    (conformance scope).
  - **Scope**: supported-syntax golden fixtures (input → expected
    blocks/runs), CommonMark-derived cases where applicable, a list of
    known deviations.
  - **Acceptance**: fixtures committed and green in the regression
    battery; deviations documented, not silently "fixed".

### [X] caretFromX click regression test

- Labels: `editor-model`, `tests`
- Body:
  - **Why**: the pre-existing click-placement bug (`lineIndex * doc.length`
    sent clicks on lines ≥ 1 to the document end) corrupted position
    cells until A4; it needs a permanent regression test in the Rust
    core.
  - **Evidence**: `docs/phase-a4-final-research-closeout.md` §11.2
    (historical record).
  - **Scope**: unit test for caretFromX at several line indexes + a
    headless click-caret smoke on a multi-line document.
  - **Acceptance**: test fails on the old formula, passes on the fix.

### [X] Viewport identity discipline for the GPUI presentation layer

- Labels: `view-model`, `memory`, `p3`
- Body:
  - **Why**: the A4-R1 finding (fresh per-render visible-list identity
    re-materialized the whole visible list per edit) transfers as a
    principle to whatever per-line presentation the GPUI layer
    materializes: per-line presentation must derive statelessly from the
    model's visible range, and any stateful line widget must be keyed by
    a stable block/content ID, not the absolute line number (which
    shifts on edits above the viewport).
  - **Evidence**: `docs/research/pocketjs-mvp-knowledge-transfer.md` §3.
  - **Scope**: establish and test the identity discipline in the
    markit-gpui layer once per-line presentation exists.
  - **Acceptance**: no per-edit re-materialization of the visible
    presentation; identity is stable and documented.
