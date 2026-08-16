//! ThinEditor: minimal multiline editable text surface for the Phase A0
//! feasibility spike (GPUI route).
//!
//! Deliberately thin: one flat UTF-8 document string + line-start index, a
//! cursor/selection, IME marked range, and a manual scroll offset. No Markdown
//! parser, no syntax highlighting, no plugins, no undo, no perf work.

use std::ops::Range;
use std::time::Instant;

use gpui::{
    actions, point, prelude::*, px, relative, rgb, rgba, size, App, Bounds, ClipboardItem,
    Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    FocusHandle, Focusable, Font, GlobalElementId, Hsla, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, ScrollDelta, ScrollWheelEvent,
    SharedString, ShapedLine, Style, TextRun, UTF16Selection, UnderlineStyle, Window, div, fill,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::a2::{self, EditStats};
use crate::instrument::{self, Stage};

actions!(
    editor,
    [
        Backspace,
        Delete,
        Enter,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Home,
        End,
        Copy,
        Cut,
        Paste,
        DumpTrace,
        Quit,
    ]
);

/// Fixed line height of the editor surface (px, logical).
const LINE_HEIGHT: f32 = 28.0;
/// Cursor width in px.
const CURSOR_WIDTH: f32 = 2.0;
/// Extra lines shaped beyond the strictly visible range (Phase A3-G1).
/// Covers fractional scroll offsets and the paint formula's rounding; the
/// shaped workset is `visible + overscan`, never document-proportional.
const OVERSCAN_LINES: usize = 2;

fn pxf(p: Pixels) -> f32 {
    f32::from(p)
}

pub struct ThinEditor {
    pub focus_handle: FocusHandle,

    // Document model: flat UTF-8 string, lines split on '\n'.
    pub text: String,
    /// Byte offset of each line start; len == line_count + 1; last == text.len().
    pub line_starts: Vec<usize>,

    // Selection in byte offsets. Cursor is the "moving end" of the selection.
    pub selection: Range<usize>,
    pub selection_reversed: bool,
    /// IME composition range (byte offsets).
    pub marked_range: Option<Range<usize>>,

    // Viewport state (manual scroll; no framework scroll container).
    pub scroll_y: Pixels,
    pub preferred_x: Option<Pixels>,
    pub is_selecting: bool,

    // Layout cache stashed during paint; used for hit-testing and IME bounds.
    pub font: Option<Font>,
    pub font_size: Option<Pixels>,
    pub line_height: Pixels,
    pub text_color: Hsla,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub last_viewport_h: Pixels,
}

impl ThinEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self::with_seed(cx, Self::DEFAULT_SEED)
    }

    /// Seed corpus (plain-10k style): ASCII + regular Chinese — the
    /// byte-identical twin of mvp/pocketjs/app/sample.ts.
    pub const DEFAULT_SEED: &'static str = "Markit Phase A0 - GPUI feasibility spike\n\
                    Fixed font, mouse & keyboard input, insert/delete, cursor,\n\
                    selection, scroll, resize, HiDPI baseline, Chinese text.\n\
                    \n\
                    中文文本显示验证：这是常规中文段落。\n\
                    低延迟、低卡顿的 Markdown 编辑器是最终目标。\n\
                    本行用于验证 DirectWrite 字体回退与中日韩字形。\n\
                    \n\
                    The quick brown fox jumps over the lazy dog. 0123456789\n\
                    A short line.\n\
                    Last line of the seed document.";

    pub fn with_seed(cx: &mut Context<Self>, text: &str) -> Self {
        let mut editor = Self {
            focus_handle: cx.focus_handle(),
            text: text.to_string(),
            line_starts: vec![0],
            selection: 0..0,
            selection_reversed: false,
            marked_range: None,
            scroll_y: px(0.0),
            preferred_x: None,
            is_selecting: false,
            font: None,
            font_size: None,
            line_height: px(LINE_HEIGHT),
            text_color: rgb(0x333333).into(),
            last_bounds: None,
            last_viewport_h: px(0.0),
        };
        editor.rebuild_line_starts();
        editor
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len() - 1
    }

    fn rebuild_line_starts(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);
        for (i, b) in self.text.bytes().enumerate() {
            if b == b'\n' {
                self.line_starts.push(i + 1);
            }
        }
        self.line_starts.push(self.text.len());
    }

    /// (line index, line start byte offset) for a byte offset.
    fn line_of_offset(&self, offset: usize) -> (usize, usize) {
        let idx = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        (idx, self.line_starts[idx])
    }

    /// Byte bounds of a line's content, excluding the trailing '\n'.
    fn line_bounds(&self, idx: usize) -> (usize, usize) {
        let start = self.line_starts[idx];
        let mut end = self.line_starts[idx + 1];
        if end > start && self.text.as_bytes()[end - 1] == b'\n' {
            end -= 1;
        }
        (start, end)
    }

    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selection = offset..offset;
        self.selection_reversed = false;
        self.preferred_x = None;
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selection.start = offset;
        } else {
            self.selection.end = offset;
        }
        if self.selection.end < self.selection.start {
            self.selection_reversed = !self.selection_reversed;
            self.selection = self.selection.end..self.selection.start;
        }
        self.ensure_cursor_visible();
        cx.notify();
    }

    /// Core mutation. `range` is in byte offsets.
    fn apply_edit(&mut self, range: Range<usize>, new_text: &str, cx: &mut Context<Self>) {
        // Phase A2: split the mutation into concat vs line-index rebuild.
        let t0 = Instant::now();
        self.text = format!(
            "{}{}{}",
            &self.text[..range.start],
            new_text,
            &self.text[range.end..]
        );
        let concat_us = a2::us_since(t0);
        let t1 = Instant::now();
        self.rebuild_line_starts();
        let lines_us = a2::us_since(t1);
        a2::record_edit(EditStats {
            concat_us,
            lines_us,
            scan_chars: self.text.len() as u64,
            lines_recreated: self.line_starts.len() as u64,
            doc_len: self.text.len() as u64,
        });
        self.selection = range.start + new_text.len()..range.start + new_text.len();
        self.selection_reversed = false;
        self.marked_range = None;
        self.preferred_x = None;
        instrument::record(Stage::EditApplied);
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.text.len())
    }

    fn shape_line_at(&self, line_idx: usize, window: &mut Window) -> Option<ShapedLine> {
        let font = self.font.clone()?;
        let font_size = self.font_size?;
        let (line_start, line_end) = self.line_bounds(line_idx);
        let text: SharedString = self.text[line_start..line_end].to_string().into();
        let run = TextRun {
            len: text.len(),
            font,
            color: self.text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        Some(window.text_system().shape_line(text, font_size, &[run], None))
    }

    /// Map a window-space point to a byte offset. Points outside the document
    /// clamp to the nearest line/edge.
    pub fn index_for_point(&mut self, point: Point<Pixels>, window: &mut Window) -> usize {
        if self.text.is_empty() {
            return 0;
        }
        let Some(bounds) = self.last_bounds else {
            return 0;
        };
        let lh = pxf(self.line_height);
        if lh <= 0.0 {
            return 0;
        }
        let local_y = pxf(point.y) - pxf(bounds.top()) + pxf(self.scroll_y);
        let line_idx = ((local_y / lh).floor().max(0.0) as usize).min(self.line_count() - 1);
        let (line_start, line_end) = self.line_bounds(line_idx);
        if line_start == line_end {
            return line_start;
        }
        let Some(shaped) = self.shape_line_at(line_idx, window) else {
            return line_start;
        };
        let x = pxf(point.x) - pxf(bounds.left());
        shaped.closest_index_for_x(px(x)) + line_start
    }

    fn ensure_cursor_visible(&mut self) {
        let lh = pxf(self.line_height);
        let viewport = pxf(self.last_viewport_h);
        if lh <= 0.0 || viewport <= 0.0 {
            return;
        }
        let (line_idx, _) = self.line_of_offset(self.cursor_offset());
        let y = line_idx as f32 * lh;
        let mut sy = pxf(self.scroll_y);
        if y < sy {
            sy = y;
        }
        if y + lh > sy + viewport {
            sy = y + lh - viewport;
        }
        self.scroll_y = px(sy.max(0.0));
    }

    // ---- actions ----------------------------------------------------------

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        if self.selection.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        let range = self.selection.clone();
        self.apply_edit(range, "", cx);
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        if self.selection.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        let range = self.selection.clone();
        self.apply_edit(range, "", cx);
    }

    fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        let range = self.selection.clone();
        self.apply_edit(range, "\n", cx);
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        let target = if self.selection.is_empty() {
            self.previous_boundary(self.cursor_offset())
        } else {
            self.selection.start
        };
        self.move_to(target, cx);
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        let target = if self.selection.is_empty() {
            self.next_boundary(self.cursor_offset())
        } else {
            self.selection.end
        };
        self.move_to(target, cx);
    }

    fn up(&mut self, _: &Up, window: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        self.move_vertical(-1, window, cx);
    }

    fn down(&mut self, _: &Down, window: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        self.move_vertical(1, window, cx);
    }

    fn move_vertical(&mut self, dy: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.text.is_empty() {
            return;
        }
        let (line_idx, _) = self.line_of_offset(self.cursor_offset());
        let target_line = (line_idx as isize + dy).clamp(0, self.line_count() as isize - 1) as usize;
        let x = self.preferred_x.unwrap_or_else(|| {
            let (cur_line, cur_start) = self.line_of_offset(self.cursor_offset());
            self.shape_line_at(cur_line, window)
                .map(|s| s.x_for_index(self.cursor_offset() - cur_start))
                .unwrap_or(px(0.0))
        });
        self.preferred_x = Some(x);
        let (target_start, target_end) = self.line_bounds(target_line);
        let target = if let Some(shaped) = self.shape_line_at(target_line, window) {
            target_start + shaped.closest_index_for_x(x)
        } else {
            target_start
        }
        .min(target_end);
        if self.selection.is_empty() {
            self.move_to(target, cx);
        } else {
            self.select_to(target, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        let offset = self.previous_boundary(self.cursor_offset());
        self.select_to(offset, cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        let offset = self.next_boundary(self.cursor_offset());
        self.select_to(offset, cx);
    }

    fn select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        self.move_vertical(-1, window, cx);
    }

    fn select_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        self.move_vertical(1, window, cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        self.move_to(0, cx);
        self.select_to(self.text.len(), cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        let (_, line_start) = self.line_of_offset(self.cursor_offset());
        self.move_to(line_start, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        let (line_idx, _) = self.line_of_offset(self.cursor_offset());
        let (_, line_end) = self.line_bounds(line_idx);
        self.move_to(line_end, cx);
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        if !self.selection.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.text[self.selection.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        if !self.selection.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.text[self.selection.clone()].to_string(),
            ));
            let range = self.selection.clone();
            self.apply_edit(range, "", cx);
        }
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        instrument::record(Stage::InputReceived);
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let range = self.selection.clone();
            self.apply_edit(range, &text, cx);
        }
    }

    fn dump_trace(&mut self, _: &DumpTrace, _: &mut Window, _: &mut Context<Self>) {
        instrument::dump("gpui thin editor", 200);
    }

    // ---- mouse ------------------------------------------------------------

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        instrument::record(Stage::InputReceived);
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        let offset = self.index_for_point(event.position, window);
        if event.modifiers.shift {
            self.select_to(offset, cx);
        } else {
            self.move_to(offset, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let offset = self.index_for_point(event.position, window);
            self.select_to(offset, cx);
        }
    }

    fn on_scroll_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = match event.delta {
            ScrollDelta::Pixels(p) => p.y,
            ScrollDelta::Lines(l) => px(l.y * pxf(self.line_height)),
        };
        // Wheel up (delta > 0) scrolls the viewport up.
        let content_h = px(self.line_count() as f32 * pxf(self.line_height));
        let max_scroll = (pxf(content_h) - pxf(self.last_viewport_h)).max(0.0);
        let new_y = pxf(self.scroll_y) - pxf(delta);
        self.scroll_y = px(new_y.clamp(0.0, max_scroll));
        cx.notify();
    }
}

impl Focusable for ThinEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for ThinEditor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selection),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range.as_ref().map(|r| self.range_to_utf16(r))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        instrument::record(Stage::InputReceived);
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selection.clone());
        self.apply_edit(range, new_text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        instrument::record(Stage::InputReceived);
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selection.clone());

        // Phase A2: same split as apply_edit (IME-commit path).
        let t0 = Instant::now();
        self.text = format!(
            "{}{}{}",
            &self.text[..range.start],
            new_text,
            &self.text[range.end..]
        );
        let concat_us = a2::us_since(t0);
        let t1 = Instant::now();
        self.rebuild_line_starts();
        let lines_us = a2::us_since(t1);
        a2::record_edit(EditStats {
            concat_us,
            lines_us,
            scan_chars: self.text.len() as u64,
            lines_recreated: self.line_starts.len() as u64,
            doc_len: self.text.len() as u64,
        });
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selection = new_selected_range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.selection_reversed = false;
        self.preferred_x = None;
        instrument::record(Stage::EditApplied);
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let lh = pxf(self.line_height);
        let (start_line, start_line_start) = self.line_of_offset(range.start);
        if range.is_empty() {
            // Caret bounds for the IME composition window before text arrives.
            let shaped = self.shape_line_at(start_line, window)?;
            let x = bounds.left() + shaped.x_for_index(range.start - start_line_start);
            let top = bounds.top() + px(start_line as f32 * lh) - self.scroll_y;
            return Some(Bounds::from_corners(
                point(x, top),
                point(x + px(CURSOR_WIDTH), top + px(lh)),
            ));
        }
        let (end_line, _) = self.line_of_offset(range.end.saturating_sub(1));
        let start_shaped = self.shape_line_at(start_line, window)?;
        let end_shaped = self.shape_line_at(end_line, window)?;
        let left = bounds.left() + start_shaped.x_for_index(range.start - start_line_start);
        let right = bounds.left() + end_shaped.x_for_index(range.end - self.line_starts[end_line]);
        let top = bounds.top() + px(start_line as f32 * lh) - self.scroll_y;
        let bottom = bounds.top() + px((end_line as f32 + 1.0) * lh) - self.scroll_y;
        Some(Bounds::from_corners(point(left, top), point(right, bottom)))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let idx = self.index_for_point(point, window);
        Some(self.offset_to_utf16(idx))
    }
}

// ---- UTF-16 <-> UTF-8 (input handler contract is UTF-16) -------------------

impl ThinEditor {
    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.text.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.text.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }
}

impl Render for ThinEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0xffffff))
            .key_context("ThinEditor")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .font_family("Consolas")
            .text_size(px(18.0))
            .line_height(px(LINE_HEIGHT))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::dump_trace))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .child(EditorElement {
                editor: cx.entity(),
            })
    }
}

// ---- custom paint element --------------------------------------------------

pub struct EditorElement {
    pub editor: Entity<ThinEditor>,
}

impl IntoElement for EditorElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

pub struct PrepaintState {
    lines: Vec<(usize, ShapedLine)>,
    selection_quads: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        instrument::record(Stage::LayoutBegin);
        // Phase A3-G1: the element's layout extent is the VIEWPORT, not the
        // content. The A2 root cause was sizing this element to
        // `line_height × line_count`, which made prepaint's "visible" range
        // `[scroll_line .. document_end]` and shaped the whole tail of the
        // document every frame. The document's logical extent (total height,
        // scroll range) lives in the ThinEditor model (`line_count`,
        // `scroll_y` clamps in on_scroll_wheel / ensure_cursor_visible) and
        // is unchanged; only the paint/layout workset is viewport-bounded.
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        let id = window.request_layout(style, [], cx);
        instrument::record(Stage::LayoutEnd);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        instrument::record(Stage::RenderBegin);
        let t_pre = Instant::now();
        let editor = self.editor.read(cx);
        let text_style = window.text_style();
        let font = text_style.font();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let lh = pxf(text_style.line_height.to_pixels(text_style.font_size, window.rem_size()));
        let scroll_y = pxf(editor.scroll_y);

        // Visible line range (paint is clipped; skip off-screen lines to
        // avoid shaping work outside the viewport). A3-G1: `bounds` is the
        // viewport (request_layout sizes the element to it), so this range
        // is viewport-bounded plus a small overscan — never
        // [first .. document end].
        let line_count = editor.line_count();
        let first = (scroll_y / lh).floor().max(0.0) as usize;
        let visible = (pxf(bounds.size.height) / lh).ceil() as usize + 1;
        let last = (first + visible + OVERSCAN_LINES).min(line_count);
        let lines_visited = last - first;

        let mut lines = Vec::new();
        let t_shape = Instant::now();
        let mut glyphs = 0u64;
        for idx in first..last {
            let (line_start, line_end) = editor.line_bounds(idx);
            let text: SharedString = editor.text[line_start..line_end].to_string().into();
            if text.is_empty() {
                continue;
            }
            glyphs += text.len() as u64;
            let runs = if let Some(marked) = editor.marked_range.as_ref() {
                let m_start = marked.start.saturating_sub(line_start).min(text.len());
                let m_end = marked.end.saturating_sub(line_start).min(text.len());
                if m_start < m_end {
                    vec![
                        TextRun {
                            len: m_start,
                            ..base_run(&text, &font, text_style.color)
                        },
                        TextRun {
                            len: m_end - m_start,
                            underline: Some(UnderlineStyle {
                                color: Some(text_style.color),
                                thickness: px(1.0),
                                wavy: false,
                            }),
                            ..base_run(&text, &font, text_style.color)
                        },
                        TextRun {
                            len: text.len() - m_end,
                            ..base_run(&text, &font, text_style.color)
                        },
                    ]
                    .into_iter()
                    .filter(|run| run.len > 0)
                    .collect()
                } else {
                    vec![base_run(&text, &font, text_style.color)]
                }
            } else {
                vec![base_run(&text, &font, text_style.color)]
            };
            let line = window.text_system().shape_line(text, font_size, &runs, None);
            lines.push((idx, line));
        }
        let shape_us = a2::us_since(t_shape);

        // Selection quads per visible line.
        let selection = editor.selection.clone();
        let selection_reversed = editor.selection_reversed;
        let cursor_offset = editor.cursor_offset();
        let mut selection_quads = Vec::new();
        let mut cursor = None;
        let mut quads = 0u64;
        if !selection.is_empty() {
            let (sel_start, sel_end) = if selection_reversed {
                (selection.end, selection.start)
            } else {
                (selection.start, selection.end)
            };
            for (idx, line) in &lines {
                let (line_start, line_end) = editor.line_bounds(*idx);
                let lo = sel_start.saturating_sub(line_start);
                let hi = sel_end.saturating_sub(line_start);
                if hi <= 0 || lo >= line_end - line_start {
                    continue;
                }
                let left = line.x_for_index(lo.clamp(0, line.len()));
                let right = line.x_for_index(hi.clamp(0, line.len()));
                let top = bounds.top() + px(*idx as f32 * lh) - px(scroll_y);
                selection_quads.push(fill(
                    Bounds::from_corners(
                        point(bounds.left() + left, top),
                        point(bounds.left() + right, top + px(lh)),
                    ),
                    rgba(0x3311ff30),
                ));
                quads += 1;
            }
        } else if editor.focus_handle.is_focused(window) {
            let (cur_line, cur_start) = editor.line_of_offset(cursor_offset);
            if let Some(line) = lines.iter().find(|(idx, _)| *idx == cur_line) {
                let x = line.1.x_for_index(cursor_offset - cur_start);
                let top = bounds.top() + px(cur_line as f32 * lh) - px(scroll_y);
                cursor = Some(fill(
                    Bounds::new(
                        point(bounds.left() + x, top),
                        size(px(CURSOR_WIDTH), px(lh)),
                    ),
                    gpui::blue(),
                ));
            }
        }

        let prepaint_us = a2::us_since(t_pre);
        // Phase A2/A3: remember render counters for the frame's JSONL line.
        a2::set_pending(a2::RenderStats {
            prepaint_us,
            shape_us,
            lines_shaped: lines.len() as u64,
            glyphs,
            first: first as u64,
            visible: visible as u64,
            last: last as u64,
            lines_total: line_count as u64,
            overscan: OVERSCAN_LINES as u64,
            lines_visited: lines_visited as u64,
            lines_painted: 0,
            quads,
            paint_us: 0,
            edit: a2::take_edit(),
        });
        PrepaintState {
            lines,
            selection_quads,
            cursor,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let t_paint = Instant::now();
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );

        let text_style = window.text_style();
        let lh = pxf(text_style.line_height.to_pixels(text_style.font_size, window.rem_size()));
        let scroll_y = pxf(self.editor.read(cx).scroll_y);

        for quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(quad);
        }
        let lines_painted = prepaint.lines.len() as u64;
        for (idx, line) in prepaint.lines.drain(..) {
            let origin = point(
                bounds.left(),
                bounds.top() + px(idx as f32 * lh) - px(scroll_y),
            );
            line.paint(origin, px(lh), window, cx).unwrap();
        }
        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }

        // Stash layout cache for hit-testing / IME bounds. The viewport height
        // is the window client height, NOT this element's height (which tracks
        // content height when the document is taller than the window).
        self.editor.update(cx, |editor, _| {
            editor.font = Some(text_style.font());
            editor.font_size = Some(text_style.font_size.to_pixels(window.rem_size()));
            editor.line_height = px(lh);
            editor.text_color = text_style.color;
            editor.last_bounds = Some(bounds);
            editor.last_viewport_h = window.viewport_size().height;
        });

        // Phase A2/A3: complete and emit the frame's JSONL line.
        if let Some(mut stats) = a2::take_pending() {
            stats.paint_us = a2::us_since(t_paint);
            stats.lines_painted = lines_painted;
            a2::emit_render(stats);
        }

        // Phase A3-M: first usable frame = application-level frame-ready
        // (seed document painted, editor wired for input). See instrument.rs.
        instrument::first_usable_frame();
        instrument::record(Stage::RenderEnd);
    }
}

fn base_run(text: &SharedString, font: &Font, color: Hsla) -> TextRun {
    TextRun {
        len: text.len(),
        font: font.clone(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}
