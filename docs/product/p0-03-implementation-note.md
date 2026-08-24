# P0-03 Implementation Note — First Product Vertical Slice

Status: **complete** on `feat/p0-03-first-product-slice` (2026-08-25).
Companion docs: `roadmap.md` (P0-03), `g0-gpui-baseline.md` (frozen
baseline), `p0-01-implementation-note.md`, `p0-02-implementation-note.md`,
issue #16.

## What landed

```text
apps/markit/src/editor_slice.rs   product-only GPUI glue + disposable projection
apps/markit/tests/p0_03_product_seam.rs   headless edit→Markdown regression
apps/markit                        default feature = editor (real GPUI window)
results/evidence/p0-03/            Windows real-host smoke receipt
```

The default `markit` binary is now the first real product path:

```text
keystroke
  → platform text input (UTF-16 at the EntityInputHandler boundary only)
  → EditTransaction
  → Document revision N → N+1 with canonical per-edit EditResult regions
  → MarkdownState::update(snapshot, &EditResult)   [incremental, exact]
  → visible projection (blocks_in_lines over a viewport-bounded line
    span; each block's bytes clipped to that span's bytes, then
    snapshot.slice materialization)
  → GPUI layout/paint in the next demanded frame
  → changed pixels
```

`--core-demo` (seam demo) and `--g0-probe` (baseline probe) remain as
diagnostics. `--no-default-features` keeps a headless build so hosts
without UI system libraries can still run `cargo test --workspace`;
`markit-core` remains GPUI-free (no core code changed semantically).

## Design facts

- **Authoritative state is exactly** `Document` + `MarkdownState` + the
  input-boundary `Selection`/composition span. No shadow `String`, no
  parallel Markdown representation; IME composition text also lives in
  the `Document` (each composition update is an ordinary transaction —
  single source of truth; composition UX itself is P1-A).
- **UTF-16 exists only at the boundary**: the four conversion helpers and
  the `EntityInputHandler` methods are the only places it appears; the
  core stays byte-addressed.
- **The app never reparses.** Every successful transaction calls
  `MarkdownState::update` with the exact `EditResult`. `update` fails
  closed on any version mismatch, and so does the transaction path: on
  rejection the slice retains `Document@N+1` with the stale
  `MarkdownState@N`, marks itself INCOHERENT, publishes no
  Markdown-derived pixels, and performs **no automatic rebuild** —
  `MarkdownState::build` is owned by explicit initial load (and a future
  explicit reset/reload action), never by typing recovery. The breach
  stays loud instead of being masked (PR #17 review).
- **Coherent publication**: the projection styles through the Markdown
  state only when `markdown.version() == document.version()`; a mixture
  is never painted.
- **Viewport-bounded visible work — including materialization**: the
  render queries `blocks_in_lines(0..visible_rows)` where `visible_rows`
  is a window-height-over-fixed-line-height estimate + 1 line overscan
  (element-measured viewports arrive with real scrolling in P1-A). A
  block may span far beyond the viewport (one paragraph can be the whole
  document), so the projection derives the byte span of the requested
  lines, intersects every returned block's `source_range` with it, and
  materializes only the intersection — visible text is bounded by the
  viewport span (+ explicit overscan), never by block/document size
  (regression: `projection_materializes_only_the_requested_span_for_huge_blocks`).
- **One Markdown semantic → one visual fact (minimum)**: heading level →
  size/bold/color; fenced code → distinct color. Markdown markers remain
  visible — no syntax hiding (P3).
- **Demand-driven rendering**: edits call `cx.notify()`; the paint hook
  counts draws and attaches the input handler (the proven G0 pattern)
  but never re-arms a frame. No timers, no `on_next_frame`, no
  scheduler, no worker pool, no caches.
- **Counters end-to-end**: every transaction logs and displays P0-01
  `EditWork` (changed bytes/lines, bytes scanned, line entries touched,
  full rebuilds) and P0-02 `MarkdownWork` (dirty regions, restart line,
  lines/bytes scanned, blocks examined/reparsed/created/removed, inline
  work, convergence line). F12 dumps full diagnostics on demand.

## Windows real-host smoke evidence

Receipt: `results/evidence/p0-03/` (log, before/after PNGs, meta, driver
script), captured at commit `0eb0806` (the post-review-remediation
build; viewport clipping + fail-closed incoherence). Setup: the G0 host
(AMD Ryzen 7 5800H, Windows 11, 2560x1440@60,
scale 100%), native Windows release build of the pinned baseline, real
input via `SendKeys` after `AppActivate`, DPI-aware window capture, idle
CPU/RSS via `Process` counters. The host's default input method is
Microsoft Pinyin, so the scripted key exercised the honest IME path: `x`
opened a composition (provisional transaction, `Typing` intent) and
`ENTER` committed the raw letter (`replace_text_in_range`,
`ImeCommit` intent).

Facts from the receipt (`p0-03-smoke.log`):

```text
P0-03 editor slice start pid=28848
P0-03 initial build rev=0 lines=4 blocks=4 md_work[dirty_regions=1
  lines_scanned=4 bytes_scanned=23 blocks_created=4]
P0-03 composition update text="x" …            ← real key 'x'
P0-03 edit rev=1 intent=Typing edit_work[changed_bytes=1 changed_lines=1
  bytes_scanned=1 line_entries_touched=0 full_rebuilds=0]
  md_work[dirty_regions=1 restart_line=2 lines_scanned=2 bytes_scanned=14
  blocks_examined=2 blocks_reparsed=1 blocks_created=0 blocks_removed=1
  inline_blocks_reparsed=1 inline_bytes_scanned=16 convergence_line=4]
P0-03 ime commit text="x" …                     ← ENTER commits
P0-03 edit rev=2 intent=ImeCommit edit_work[changed_bytes=2 …
  full_rebuilds=0] md_work[dirty_regions=1 blocks_reparsed=1 …]
P0-03 dump draws=5 rev=2 md_rev=2 coherent=true blocks=3 lines=4
  caret=24 composition=None …
P0-03 dump draws=6 rev=2 md_rev=2 coherent=true …   (2 s later, idle)
```

- window opens; initial Markdown visibly styled (`p0-03-before.png`:
  `# Markit` large/bold/blue vs ordinary paragraph text);
- real input changes the `Document` as two transactions, each advancing
  exactly one revision: the `x` keypress opened a Pinyin composition
  (transaction 1: provisional insert, `Typing` intent, rev 0→1) and
  `ENTER` committed it (transaction 2: `replace_text_in_range`,
  `ImeCommit` intent, rev 1→2). The pair 0→1→2 is key+commit, not "one
  key"; `MarkdownState` reaches the same version as the document
  (`rev=2 md_rev=2 coherent=true`);
- edited pixels visibly change: before/after window captures differ
  (SHA256 in `p0-03-meta.txt`); a pixel-diff of the two captures
  localizes the change to exactly the expected bands — the appended `x`
  glyph line inside the joined paragraph (~52 px), the fully rewritten
  status line (counters/rev), and ~40 px of glyph-antialiasing noise in
  the heading band;
- P0-01/P0-02 counters emitted per edit (above) and visible in the
  status bar;
- idle has no application update loop: across the 2 s between the two
  F12 dumps, draws advanced 5→6 — the +1 is the first dump's own
  `cx.notify()` repaint; a frame loop would have added ~120. Idle CPU
  ≈ 0.4–0.8 % across runs (0.02–0.06 CPU-seconds over the 4 s sampling
  window; the baseline's documented vsync-thread wakeup floor; G0
  report §4), 38 MB WS, no growth.

Incidental observation (log not retained): an earlier run of this same
build on the live host was contaminated by real human typing through
the still-focused window — ~120 additional IME transactions (compositions
plus multi-character sentence commits, revs 3–121). Every one stayed in
lockstep (`coherent=true` at each dump, zero update rejections) with
Markdown work local throughout (`dirty_regions=1`, `blocks_reparsed=1`,
`lines_scanned=2`). Not a controlled benchmark; recorded only because
it is the largest sustained-real-IME sample so far.

Observation vs inference (the log is observation; the attribution of the
+1 draw to the dump's own notify is inference from the demand-driven code
path — no other notify source exists in the slice).

## Headless product-seam regression

`apps/markit/tests/p0_03_product_seam.rs` (GPUI-free):

- `paragraph_keystroke_is_local_and_semantically_visible` — fixture edit
  inside `Edit **me**.`: version lockstep, one canonical region, P0-01
  counters (`full_rebuilds=0`, `bytes_scanned=1`, `changed_lines=1`),
  fixture-locality P0-02 counters (island = paragraph + blank boundary
  neighbor: `blocks_reparsed=2`, `lines_scanned=2`), and the semantic
  result through the same `blocks_in_lines` + `slice` surface the
  projection uses (heading level/content intact, paragraph text grown,
  strong emphasis survived);
- `eof_keystrokes_stay_in_lockstep` — the Windows smoke path (appends on
  the final line; consecutive non-blank lines honestly join into one
  paragraph), plus the inverse (undo-seam) transaction staying in
  lockstep;
- `version_mismatch_fails_closed_at_the_product_seam` — stale /
  non-matching results are rejected (`SnapshotNotAtNewRevision`,
  `StaleBase`) with the state untouched; `MarkdownState::build` stays
  owned by explicit initial load/reset — the app keeps
  `Document@N+1` + stale `MarkdownState@N` and renders INCOHERENT
  rather than rebuilding.

Plus three windowless unit tests inside
`apps/markit/src/editor_slice.rs` (they link GPUI but open no window):
`line_span_bytes_covers_exactly_the_requested_lines` (span derivation,
EOF clamp, degenerate ranges), `projection_full_document_viewport_is_
unclipped` (clipping changes nothing on the normal full-document
path), and the adversarial `projection_materializes_only_the_requested_
span_for_huge_blocks`: one ~50 KB single-paragraph block (400 joined
lines) queried through a 2-line viewport deep inside it — materialized
bytes must stay ≤ the requested span's bytes (~100× smaller than the
block/document) and be exactly the two requested lines. This is the PR
#17 review regression: the pre-fix projection materialized
`slice(block.source_range())`, i.e. O(block) text per frame.

`blocks_reparsed == 1/2` here is fixture evidence, not a law (issue #12
R6): structural edits may honestly propagate further, and the counters
report it when they do.

## Limitations / honest non-coverage

- No plain-key (non-IME) commit was captured in this smoke because the
  host default input method is Microsoft Pinyin; the commit path
  (`replace_text_in_range`, `ImeCommit` intent) is exercised via the
  pinyin ENTER-commit, and the non-IME branch of the same method is the
  identical code path (`Typing` intent). A future English-IME host run
  can add that receipt; P1-A's IME work will cover input-method matrices
  properly.
- The viewport estimate is window-height-based, not element-measured;
  correct for the slice (no scrolling), replaced when scrolling lands.
- No pixel-precise caret, no selection rendering, no scroll, no file
  I/O, no undo stack, no clipboard, no plugin surface — all explicit
  P0-03 non-goals (issue #16 subtraction list), all preserved.
- The slice is intentionally synchronous; there is no deferred/cancellable
  job to steal results. When a real job exceeds the interaction budget
  (issue #12 R3 trigger), it gets revision-safe machinery then.
