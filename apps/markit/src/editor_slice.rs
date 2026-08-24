//! P0-03 first product vertical slice (issue #16).
//!
//! The smallest real Markit window: one keystroke travels
//!
//! ```text
//! platform text input (UTF-16 at the boundary only)
//!   -> EditTransaction
//!   -> Document (revision + canonical EditResult regions)
//!   -> MarkdownState::update(snapshot, &EditResult)
//!   -> visible projection (blocks_in_lines; each block's bytes clipped
//!      to the visible lines' byte span before snapshot.slice)
//!   -> GPUI layout/paint in the next demanded frame
//!   -> changed pixels
//! ```
//!
//! Authoritative state is exactly `Document` + `MarkdownState` + the
//! input-boundary `Selection`/composition span. There is no shadow buffer
//! and no parallel Markdown representation; the projection below is
//! disposable view state rebuilt per render from those two sources.
//!
//! Deliberately NOT here (roadmap P0-03 non-goals): scheduler, worker
//! pool, cache framework, files, undo stack, clipboard, IME composition
//! UX beyond the input-trait minimum, syntax hiding, plugins. Rendering is
//! demand-driven: edits call `cx.notify()`, nothing re-arms frames.

use std::{
    borrow::Cow,
    ops::Range,
    sync::{
        atomic::{AtomicU64, Ordering::Relaxed},
        Arc,
    },
};

use gpui::{
    actions, canvas, div, prelude::*, px, rgb, size, App, AppContext, Bounds, Context, Div,
    ElementInputHandler, EntityInputHandler, FocusHandle, Focusable, FontWeight, KeyBinding,
    Pixels, Render, UTF16Selection, Window, WindowBounds, WindowOptions,
};
use gpui_platform::application;
use markit_core::markdown::{BlockDetail, MarkdownState};
use markit_core::BlockKind;
use markit_core::{
    ByteOffset, Document, DocumentSnapshot, EditIntent, EditResult, EditTransaction, LineNumber,
    Selection, SourceRange, TextEdit,
};

actions!(editor, [DumpDiagnostics]);

/// The one in-memory P0-03 document. No file I/O in this slice.
const INITIAL_DOCUMENT: &str = "# Markit\n\nEdit **me**.\n";

/// Fixed presentation line height for the slice's viewport estimate.
const LINE_HEIGHT_PX: f32 = 24.0;
/// Tiny explicit overscan on the visible-line query.
const OVERSCAN_LINES: usize = 1;

/// Product editor state: authoritative semantic state plus the smallest
/// input-boundary bookkeeping. Nothing else is editor truth.
struct EditorSlice {
    focus_handle: FocusHandle,
    document: Document,
    markdown: MarkdownState,
    /// Caret/selection in byte offsets; converted to UTF-16 only inside
    /// the [`EntityInputHandler`] boundary methods.
    selection: Selection,
    /// Active IME composition span (bytes), `None` when not composing.
    /// The composing text lives in `document` — there is no second buffer.
    composition: Option<SourceRange>,
    /// Draw counter, incremented in the paint hook — the idle/demand
    /// evidence seam (see the G0 probe for the technique).
    draws: Arc<AtomicU64>,
    /// Last edit receipt for the status line.
    last_status: String,
}

impl EditorSlice {
    fn new(cx: &mut Context<Self>) -> Self {
        let document = Document::new(INITIAL_DOCUMENT);
        let caret = ByteOffset(document.len_bytes());
        let markdown = MarkdownState::build(&document.snapshot());
        let work = markdown.last_work();
        log::info!(
            "P0-03 initial build rev=0 lines={} blocks={} md_work[dirty_regions={} lines_scanned={} bytes_scanned={} blocks_created={}]",
            document.line_count(),
            markdown.block_count(),
            work.dirty_regions,
            work.lines_scanned,
            work.bytes_scanned,
            work.blocks_created,
        );
        Self {
            focus_handle: cx.focus_handle(),
            selection: Selection::caret(caret),
            document,
            markdown,
            composition: None,
            draws: Arc::new(AtomicU64::new(0)),
            last_status: "type to edit | F12 diagnostics".to_string(),
        }
    }

    /// The whole document as `&str` (borrowed) — input only, for UTF-16
    /// boundary conversions. Never a second editable buffer.
    fn full_text(&self) -> Cow<'_, str> {
        self.document.slice(full_range_of(&self.document))
    }

    /// Applies `tx` and advances the Markdown state from the exact
    /// canonical `EditResult` — the entire semantic half of the slice.
    ///
    /// The app never reparses independently. `MarkdownState::update`
    /// fails closed on any version mismatch, and so does this path: on
    /// rejection the document keeps its new revision, the Markdown state
    /// keeps its last coherent one, and the render gate publishes no
    /// Markdown-derived pixels for the incoherent pair. There is
    /// deliberately NO automatic rebuild here — `MarkdownState::build`
    /// belongs to explicit initial-load/reset ownership, and a hot-path
    /// rebuild would mask the invariant breach instead of exposing it.
    fn apply_transaction(&mut self, tx: EditTransaction) -> Option<EditResult> {
        let intent = tx.intent();
        let applied = match tx.apply(&mut self.document) {
            Ok(applied) => applied,
            Err(err) => {
                log::warn!("P0-03 edit rejected intent={intent:?}: {err}");
                self.last_status = format!("edit rejected: {err}");
                return None;
            }
        };
        let result = applied.result;
        let snapshot = self.document.snapshot();
        if let Err(err) = self.markdown.update(&snapshot, &result) {
            // Fail closed: keep Document@N+1 + stale MarkdownState@N and go
            // INCOHERENT. `last_work()` still describes the stale state's
            // previous transition, so this branch must not log it as if it
            // were this edit's Markdown work.
            log::error!(
                "P0-03 MarkdownState::update rejected rev {}->{} ({err}); \
                 retaining Document@{} + stale MarkdownState@{} — INCOHERENT, \
                 no automatic rebuild",
                result.base_version.revision().as_u64(),
                result.new_version.revision().as_u64(),
                result.new_version.revision().as_u64(),
                self.markdown.version().revision().as_u64(),
            );
            self.last_status = format!(
                "INCOHERENT doc@{} md@{}: {err}",
                result.new_version.revision().as_u64(),
                self.markdown.version().revision().as_u64(),
            );
            return Some(result);
        }

        // P0-01 + P0-02 structural counters, end-to-end on the product path.
        // Only reached on a coherent transition, so both counters describe
        // this edit.
        let ew = result.work;
        let mw = self.markdown.last_work();
        log::info!(
            "P0-03 edit rev={} intent={intent:?} edit_work[changed_bytes={} changed_lines={} bytes_scanned={} line_entries_touched={} full_rebuilds={}] md_work[dirty_regions={} restart_line={} lines_scanned={} bytes_scanned={} blocks_examined={} blocks_reparsed={} blocks_created={} blocks_removed={} inline_blocks_reparsed={} inline_bytes_scanned={} convergence_line={}]",
            result.new_version.revision().as_u64(),
            ew.changed_bytes,
            ew.changed_lines,
            ew.bytes_scanned,
            ew.line_entries_touched,
            ew.full_rebuilds,
            mw.dirty_regions,
            mw.restart_line,
            mw.lines_scanned,
            mw.bytes_scanned,
            mw.blocks_examined,
            mw.blocks_reparsed,
            mw.blocks_created,
            mw.blocks_removed,
            mw.inline_blocks_reparsed,
            mw.inline_bytes_scanned,
            mw.convergence_line,
        );
        self.last_status = format!(
            "rev {} | doc[cb={} cl={} scan={} le={}] md[dirty={} reparsed={} lines={}]",
            result.new_version.revision().as_u64(),
            ew.changed_bytes,
            ew.changed_lines,
            ew.bytes_scanned,
            ew.line_entries_touched,
            mw.dirty_regions,
            mw.blocks_reparsed,
            mw.lines_scanned,
        );
        Some(result)
    }

    /// Committed text (keystroke or IME commit): one transaction, then the
    /// caret policy on top of the core mapping — committed text leaves the
    /// caret after the insertion (`Selection::map_over_edit` alone pins a
    /// collapsed caret before the inserted text; see selection.rs).
    fn commit_text(&mut self, replaced: SourceRange, text: &str, cx: &mut Context<Self>) {
        let intent = if self.composition.is_some() {
            EditIntent::ImeCommit
        } else {
            EditIntent::Typing
        };
        let tx = EditTransaction::new(intent).with_edit(TextEdit::replace(replaced, text));
        let Some(result) = self.apply_transaction(tx) else {
            cx.notify();
            return;
        };
        let last = result.edits.last().expect("applied transaction has edits");
        self.selection = Selection::caret(last.new_range.end);
        self.composition = None;
        cx.notify();
    }

    /// Provisional IME composition update. The minimum the GPUI input
    /// boundary requires: composition text is a real document mutation
    /// (single authoritative buffer), the marked span is tracked in
    /// bytes, and commits later replace it. Composition UX — grouping,
    /// candidate presentation — is P1-A.
    fn update_composition(
        &mut self,
        replaced: SourceRange,
        text: &str,
        caret_utf16: Option<Range<usize>>,
        cx: &mut Context<Self>,
    ) {
        let tx = EditTransaction::typing().with_edit(TextEdit::replace(replaced, text));
        let Some(result) = self.apply_transaction(tx) else {
            cx.notify();
            return;
        };
        let edit = result.edits.last().expect("applied transaction has edits");
        self.composition = if text.is_empty() {
            None
        } else {
            Some(edit.new_range)
        };
        // `caret_utf16` is relative to the NEW composition text (the
        // in-composition caret, not the marked span).
        let base = edit.new_range.start.as_usize();
        self.selection = match caret_utf16.as_ref() {
            Some(range) => {
                let start = offset_from_utf16(text, range.start);
                let end = offset_from_utf16(text, range.end).max(start);
                Selection::new(ByteOffset(base + start), ByteOffset(base + end))
            }
            None => Selection::caret(edit.new_range.end),
        };
        cx.notify();
    }

    /// Rough visible-line span for the slice: window height over a fixed
    /// line height plus one line of overscan. Element-measured viewports
    /// arrive with real scrolling in P1-A; the point here is that the
    /// Markdown query is viewport-bounded, never document-bounded.
    fn visible_line_span(&self, window: &Window) -> Range<LineNumber> {
        let height = window.bounds().size.height.as_f32();
        let rows = (((height - 120.0) / LINE_HEIGHT_PX).ceil() as usize).saturating_sub(1)
            + OVERSCAN_LINES;
        let end = rows.clamp(1, self.document.line_count());
        LineNumber(0)..LineNumber(end)
    }

    fn dump_diagnostics(&self) {
        let coherent = self.markdown.version() == self.document.version();
        log::info!(
            "P0-03 dump draws={} rev={} md_rev={} coherent={} blocks={} lines={} caret={} composition={:?} status={}",
            self.draws.load(Relaxed),
            self.document.revision().as_u64(),
            self.markdown.version().revision().as_u64(),
            coherent,
            self.markdown.block_count(),
            self.document.line_count(),
            self.selection.caret_offset().as_usize(),
            self.composition,
            self.last_status,
        );
    }
}

/// One materialized visible block — disposable presentation state derived
/// per render from (snapshot, markdown state). It carries no semantic
/// identity; deleting it loses nothing but pixels.
struct VisibleBlock {
    kind: BlockKind,
    heading_level: Option<u8>,
    /// Block source text, line terminators trimmed. Markdown markers stay
    /// visible: syntax hiding is explicitly not P0-03.
    text: String,
}

/// The byte span covering exactly `lines` (clamped to the document):
/// first byte of the first requested line through the last requested
/// line's terminator. This is the hard bound on what the projection may
/// materialize — the caller's explicit overscan is part of `lines`.
fn line_span_bytes(snapshot: &DocumentSnapshot<'_>, lines: &Range<LineNumber>) -> SourceRange {
    let last_line = snapshot.line_count().saturating_sub(1);
    let start = snapshot
        .line_range(LineNumber(lines.start.as_usize().min(last_line)))
        .start;
    if lines.start >= lines.end {
        return SourceRange::new(start, start);
    }
    let end_line = (lines.end.as_usize() - 1).min(last_line);
    let end = if end_line + 1 < snapshot.line_count() {
        snapshot.line_range(LineNumber(end_line + 1)).start
    } else {
        ByteOffset(snapshot.len_bytes())
    };
    SourceRange::new(start, end.max(start))
}

/// Overlap of `range` with `to`; `None` when they share no bytes.
fn clip_range(range: SourceRange, to: SourceRange) -> Option<SourceRange> {
    let start = range.start.0.max(to.start.0);
    let end = range.end.0.min(to.end.0);
    (start < end).then(|| SourceRange::new(ByteOffset(start), ByteOffset(end)))
}

/// The tiny P0-03 projection: query only the visible line span, then
/// materialize each block's text clipped to that span's bytes. A block
/// may span far beyond the viewport (one paragraph can be the whole
/// document), so materializing a block's full `source_range` would make
/// visible work O(block) or worse; the clip keeps it bounded by the
/// requested span while `BlockKind`/`BlockDetail` stay the semantic and
/// style source. Heading level is the one Markdown semantic fact that
/// becomes a visual style fact.
fn project_visible_blocks(
    snapshot: &DocumentSnapshot<'_>,
    markdown: &MarkdownState,
    lines: Range<LineNumber>,
) -> Vec<VisibleBlock> {
    let visible_bytes = line_span_bytes(snapshot, &lines);
    markdown
        .blocks_in_lines(lines)
        .filter(|view| view.kind() != BlockKind::Blank)
        .filter_map(|view| {
            let heading_level = match view.detail() {
                BlockDetail::Heading { level, .. } => Some(*level),
                _ => None,
            };
            let clipped = clip_range(view.source_range(), visible_bytes)?;
            let text = snapshot
                .slice(clipped)
                .trim_end_matches(['\n', '\r'])
                .to_string();
            Some(VisibleBlock {
                kind: view.kind(),
                heading_level,
                text,
            })
        })
        .collect()
}

/// Markdown semantics -> style facts: heading level becomes size/weight,
/// fenced code gets a distinct dim color; everything else renders as
/// ordinary text. Raw Markdown markers stay visible (no syntax hiding).
fn style_block(block: &VisibleBlock) -> Div {
    match block.heading_level {
        Some(level) => {
            let size = match level {
                1 => 30.0,
                2 => 26.0,
                3 => 22.0,
                _ => 19.0,
            };
            div()
                .text_size(px(size))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0x89b4fa))
                .child(block.text.clone())
        }
        None if block.kind == BlockKind::FencedCode => div()
            .text_size(px(15.0))
            .text_color(rgb(0xa6e3a1))
            .child(block.text.clone()),
        None => div().text_size(px(16.0)).child(block.text.clone()),
    }
}

// ---- UTF-16 boundary conversions (platform edge only) --------------------
//
// Same contract as the G0 probe: every range crossing
// `EntityInputHandler` is UTF-16; the document is byte-addressed and the
// conversion happens only in these helpers and the trait methods below.

fn offset_from_utf16(text: &str, offset: usize) -> usize {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;
    for ch in text.chars() {
        if utf16_count >= offset {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }
    utf8_offset
}

fn offset_to_utf16(text: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for ch in text.chars() {
        if utf8_count >= offset {
            break;
        }
        utf8_count += ch.len_utf8();
        utf16_offset += ch.len_utf16();
    }
    utf16_offset
}

fn range_from_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = offset_from_utf16(text, range.start);
    start..offset_from_utf16(text, range.end).max(start)
}

fn range_to_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    offset_to_utf16(text, range.start)..offset_to_utf16(text, range.end)
}

fn full_range_of(document: &Document) -> SourceRange {
    SourceRange::new(ByteOffset::ZERO, ByteOffset(document.len_bytes()))
}

fn source_range(range: Range<usize>) -> SourceRange {
    SourceRange::new(ByteOffset(range.start), ByteOffset(range.end))
}

impl EntityInputHandler for EditorSlice {
    // Boundary contract (pinned gpui `input.rs` + canonical
    // `examples/input.rs`): every range crossing this trait is UTF-16.

    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.full_text();
        let range = range_from_utf16(&text, &range_utf16);
        *actual_range = Some(range_to_utf16(&text, &range));
        Some(text.get(range)?.to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let text = self.full_text();
        let range = self.selection.to_range();
        Some(UTF16Selection {
            range: range_to_utf16(&text, &(range.start.as_usize()..range.end.as_usize())),
            reversed: self.selection.is_reversed(),
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let text = self.full_text();
        self.composition
            .as_ref()
            .map(|r| range_to_utf16(&text, &(r.start.as_usize()..r.end.as_usize())))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Windows commits through `replace_text_in_range` (GCS_RESULTSTR)
        // instead; this path is for platforms with an explicit unmark step.
        if self.composition.take().is_some() {
            log::info!("P0-03 composition unmark");
        }
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // `None` means "replace the active composition, else the
        // selection" (canonical example). Windows IMM32 commits the result
        // string with `None`, so honoring this makes commits REPLACE the
        // marked span instead of appending after it.
        let replaced = match range_utf16.as_ref() {
            Some(range_utf16) => {
                let full = self.full_text();
                source_range(range_from_utf16(&full, range_utf16))
            }
            None => self
                .composition
                .unwrap_or_else(|| self.selection.to_range()),
        };
        let was_composition = self.composition.is_some();
        if was_composition {
            log::info!("P0-03 ime commit text={text:?} replaced={replaced:?}");
        } else {
            log::info!(
                "P0-03 keystroke text={text:?} replaced={replaced:?} caret_before={}",
                self.selection.caret_offset().as_usize(),
            );
        }
        self.commit_text(replaced, text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let replaced = match range_utf16.as_ref() {
            Some(range_utf16) => {
                let full = self.full_text();
                source_range(range_from_utf16(&full, range_utf16))
            }
            None => self
                .composition
                .unwrap_or_else(|| self.selection.to_range()),
        };
        log::info!(
            "P0-03 composition update text={new_text:?} replaced={replaced:?} selected_utf16={new_selected_range_utf16:?}",
        );
        self.update_composition(replaced, new_text, new_selected_range_utf16, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // Candidate-window docking rect: bottom edge of the text area.
        Some(Bounds {
            origin: gpui::Point {
                x: element_bounds.origin.x,
                y: element_bounds.origin.y + element_bounds.size.height - px(24.0),
            },
            size: gpui::Size {
                width: element_bounds.size.width,
                height: px(24.0),
            },
        })
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let text = self.full_text();
        Some(offset_to_utf16(
            &text,
            self.selection.caret_offset().as_usize(),
        ))
    }

    fn set_selected_text_range(
        &mut self,
        range_utf16: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = self.full_text();
        self.selection = Selection::new(
            ByteOffset(offset_from_utf16(&text, range_utf16.start)),
            ByteOffset(offset_from_utf16(&text, range_utf16.end)),
        );
        cx.notify();
    }

    fn text_length_utf16(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let text = self.full_text();
        Some(offset_to_utf16(&text, text.len()))
    }
}

impl Focusable for EditorSlice {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorSlice {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Coherent publication: style through the Markdown state only when
        // it describes exactly the current document version. The two
        // advance in lockstep in `apply_transaction`; a mismatch here is
        // the fail-closed INCOHERENT state's business (update rejected,
        // no automatic rebuild) and must not paint a doc-vN + markdown-vM
        // mixture.
        let coherent = self.markdown.version() == self.document.version();
        let blocks = if coherent {
            let snapshot = self.document.snapshot();
            project_visible_blocks(&snapshot, &self.markdown, self.visible_line_span(window))
        } else {
            Vec::new()
        };

        let draws = self.draws.clone();
        let entity = cx.entity();
        let focus_handle = self.focus_handle.clone();
        let status = format!(
            "{} | rev={} md={} blocks={} draws={}{}",
            self.last_status,
            self.document.revision().as_u64(),
            self.markdown.version().revision().as_u64(),
            self.markdown.block_count(),
            self.draws.load(Relaxed),
            if coherent { "" } else { " | INCOHERENT" },
        );

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(rgb(0x1e1e2e))
            .p_6()
            .flex()
            .flex_col()
            .text_size(px(16.0))
            .text_color(rgb(0xcdd6f4))
            .on_action(cx.listener(|this, _: &DumpDiagnostics, _, cx| {
                this.dump_diagnostics();
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(blocks.iter().map(style_block)),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x7f849c))
                    .child(status),
            )
            .child(canvas(
                move |bounds: Bounds<Pixels>, _, _| bounds,
                move |_, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App| {
                    // Paint hook: the draw moment for this frame, and the
                    // proven G0 pattern for attaching the input handler
                    // with current bounds. Nothing here re-arms a frame:
                    // no notify, no on_next_frame — redraw stays
                    // demand-driven.
                    draws.fetch_add(1, Relaxed);
                    window.handle_input(
                        &focus_handle,
                        ElementInputHandler::new(bounds, entity.clone()),
                        cx,
                    );
                },
            ))
    }
}

pub fn run() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_micros()
        .try_init();
    log::info!("P0-03 editor slice start pid={}", std::process::id());

    application().run(move |cx: &mut App| {
        // Bind keys BEFORE opening the window (Phase A0 prototype finding:
        // otherwise the keymap is empty on Windows). F12 only — printable
        // keys must reach the text-input path, not the keymap.
        cx.bind_keys([KeyBinding::new("f12", DumpDiagnostics, None)]);

        let bounds = Bounds::centered(None, size(px(720.0), px(480.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(EditorSlice::new),
            )
            .expect("P0-03: open window");
        cx.activate(true);

        let focus = window
            .update(cx, |slice, _, cx| {
                let focus = slice.focus_handle.clone();
                (cx.entity(), focus)
            })
            .expect("P0-03: entity");
        window
            .update(cx, |_, window, cx| window.focus(&focus.1, cx))
            .ok();
    });
}

#[cfg(test)]
mod tests {
    //! Structural viewport-boundedness regressions for the projection
    //! (PR #17 review): the materialized text must be bounded by the
    //! REQUESTED line span, never by block or document size. These run
    //! windowless; only the projection's pure functions are exercised.

    use super::*;

    #[test]
    fn line_span_bytes_covers_exactly_the_requested_lines() {
        // Fixture bytes: "# Markit\n" 0..9, blank "\n" at 9, "Edit **me**.\n"
        // 10..23, final empty line 3 at 23 (23 bytes total).
        let doc = Document::new(INITIAL_DOCUMENT);
        let snapshot = doc.snapshot();
        assert_eq!(snapshot.len_bytes(), 23);
        assert_eq!(snapshot.line_count(), 4);

        let span = |a: usize, b: usize| line_span_bytes(&snapshot, &(LineNumber(a)..LineNumber(b)));

        // One line including its terminator.
        assert_eq!(
            span(2, 3),
            SourceRange::new(ByteOffset(10), ByteOffset(23)),
            "line 2 through its '\\n'"
        );
        // Two lines from the document start.
        assert_eq!(span(0, 2), SourceRange::new(ByteOffset(0), ByteOffset(10)));
        // The final (empty) line has no terminator: empty span at EOF.
        assert_eq!(span(3, 4), SourceRange::new(ByteOffset(23), ByteOffset(23)));
        // Clamped past EOF, and degenerate/empty requests stay well-formed.
        assert_eq!(
            span(9, 20),
            SourceRange::new(ByteOffset(23), ByteOffset(23))
        );
        assert_eq!(span(2, 2), SourceRange::new(ByteOffset(10), ByteOffset(10)));
        assert_eq!(span(3, 1), SourceRange::new(ByteOffset(23), ByteOffset(23)));
    }

    #[test]
    fn projection_full_document_viewport_is_unclipped() {
        // Guard: with the viewport covering the whole fixture, clipping
        // must not change the normal-path output (markers stay visible).
        let doc = Document::new(INITIAL_DOCUMENT);
        let snapshot = doc.snapshot();
        let markdown = MarkdownState::build(&snapshot);

        let visible = project_visible_blocks(
            &snapshot,
            &markdown,
            LineNumber(0)..LineNumber(snapshot.line_count()),
        );

        let summary: Vec<(BlockKind, Option<u8>, &str)> = visible
            .iter()
            .map(|b| (b.kind, b.heading_level, b.text.as_str()))
            .collect();
        assert_eq!(
            summary,
            vec![
                (BlockKind::Heading, Some(1), "# Markit"),
                (BlockKind::Paragraph, None, "Edit **me**."),
            ],
            "blank runs are filtered; heading keeps its level; markers visible"
        );
    }

    #[test]
    fn projection_materializes_only_the_requested_span_for_huge_blocks() {
        // Adversarial shape (PR #17 review blocker 1): one paragraph of
        // 400 joined lines (~50 KB) is ONE block spanning the whole
        // document, queried through a 2-line viewport deep inside it.
        // Pre-fix behavior materialized `slice(block.source_range())` —
        // O(block) text per frame; the bound must be the requested span.
        let line = "w".repeat(120);
        let text = (0..400)
            .map(|i| format!("{line} {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let doc = Document::new(&text);
        let snapshot = doc.snapshot();
        let markdown = MarkdownState::build(&snapshot);

        // The adversarial premise holds: the entire document is one block.
        assert_eq!(markdown.block_count(), 1);
        let block = markdown
            .blocks_in_lines(LineNumber(0)..LineNumber(1))
            .next()
            .expect("the one block");
        assert_eq!(block.kind(), BlockKind::Paragraph);
        assert_eq!(block.source_range().len(), text.len());

        let viewport = LineNumber(150)..LineNumber(152);
        let span = line_span_bytes(&snapshot, &viewport);
        assert!(
            span.len() * 100 < text.len(),
            "span {} must be tiny against the {} byte document",
            span.len(),
            text.len()
        );

        let visible = project_visible_blocks(&snapshot, &markdown, viewport);
        assert_eq!(visible.len(), 1, "the huge block, clipped");
        let materialized: usize = visible.iter().map(|b| b.text.len()).sum();

        // Bounded by the requested span's bytes (the caller asked for
        // exactly two lines; overscan would be the caller's explicit
        // addition to `viewport`, not something the projection adds).
        assert!(
            materialized <= span.len(),
            "materialized {materialized} bytes exceeds the {span_len} byte span",
            span_len = span.len()
        );
        assert!(
            materialized * 100 < text.len(),
            "materialized {materialized} bytes scales with the document ({} bytes)",
            text.len()
        );

        // And it is the RIGHT text: exactly the two requested lines, not
        // a block-aligned surrogate.
        assert_eq!(visible[0].kind, BlockKind::Paragraph);
        assert_eq!(visible[0].heading_level, None);
        assert_eq!(visible[0].text, format!("{line} 150\n{line} 151"));
    }
}
