//! Phase A2 instrumentation (Markit-owned) — GPUI edit/render work counters
//! and JSONL emission.
//!
//! Kept out of the A1 ring-buffer trace (instrument.rs) so the A1 stages and
//! dump format stay byte-identical. One JSON line per render frame plus one
//! line per edit, consumed by bench/parse-a2.py. All fields are counters or
//! durations; nothing here changes editor behavior.

use std::sync::Mutex;
use std::time::Instant;

#[derive(Default, Clone)]
pub struct EditStats {
    /// format! concat duration, us
    pub concat_us: u64,
    /// rebuild_line_starts duration, us
    pub lines_us: u64,
    /// bytes scanned by rebuild_line_starts (== document length)
    pub scan_chars: u64,
    /// line entries (re)created
    pub lines_recreated: u64,
    /// document byte length after the edit
    pub doc_len: u64,
}

#[derive(Default, Clone)]
pub struct RenderStats {
    /// total prepaint duration, us
    pub prepaint_us: u64,
    /// visible-range line shaping loop, us
    pub shape_us: u64,
    /// lines shaped (non-empty lines in the visited range)
    pub lines_shaped: u64,
    /// glyphs across shaped lines (sum of shaped line lengths)
    pub glyphs: u64,
    /// first line index visited
    pub first: u64,
    /// viewport-derived visible count (element-bounds based; A3-G1: the
    /// element is sized to the viewport, so this is ~viewport lines)
    pub visible: u64,
    /// last line index visited (exclusive)
    pub last: u64,
    /// total document lines
    pub lines_total: u64,
    /// overscan lines added beyond the visible range (A3-G1)
    pub overscan: u64,
    /// lines visited by prepaint (last - first; the workset size)
    pub lines_visited: u64,
    /// lines painted (completed in paint)
    pub lines_painted: u64,
    /// selection quads built
    pub quads: u64,
    /// paint loop duration, us
    pub paint_us: u64,
    /// the edit that caused this frame, if any
    pub edit: Option<EditStats>,
}

static LAST_EDIT: Mutex<Option<EditStats>> = Mutex::new(None);
static PENDING_RENDER: Mutex<Option<RenderStats>> = Mutex::new(None);
/// JSONL emission gate (GPUI_A2=1). Counter collection always runs (cheap);
/// only the per-frame println is gated, so ON/OFF share one binary.
static ENABLED: Mutex<bool> = Mutex::new(false);

pub fn set_enabled(on: bool) {
    if let Ok(mut g) = ENABLED.lock() {
        *g = on;
    }
}

/// Record an edit (called from apply_edit / replace_and_mark).
pub fn record_edit(stats: EditStats) {
    if let Ok(mut g) = LAST_EDIT.lock() {
        *g = Some(stats);
    }
}

/// Prepaint stashes its counters; paint completes and emits them.
pub fn set_pending(stats: RenderStats) {
    if let Ok(mut g) = PENDING_RENDER.lock() {
        *g = Some(stats);
    }
}

/// Take the pending render stats (consumed by paint).
pub fn take_pending() -> Option<RenderStats> {
    if let Ok(mut g) = PENDING_RENDER.lock() {
        g.take()
    } else {
        None
    }
}

/// Emit one JSON line for a rendered frame (called at RenderEnd).
pub fn emit_render(stats: RenderStats) {
    if !is_enabled() {
        return;
    }
    let edit_json = match &stats.edit {
        Some(e) => format!(
            ",\"concat_us\":{},\"lines_us\":{},\"scan_chars\":{},\"lines_recreated\":{},\"doc_len\":{}",
            e.concat_us, e.lines_us, e.scan_chars, e.lines_recreated, e.doc_len
        ),
        None => String::new(),
    };
    println!(
        "{{\"perf\":1,\"prepaint_us\":{},\"shape_us\":{},\"lines_shaped\":{},\"glyphs\":{},\"first\":{},\"visible\":{},\"last\":{},\"lines_total\":{},\"overscan\":{},\"lines_visited\":{},\"lines_painted\":{},\"quads\":{},\"paint_us\":{}{}}}",
        stats.prepaint_us,
        stats.shape_us,
        stats.lines_shaped,
        stats.glyphs,
        stats.first,
        stats.visible,
        stats.last,
        stats.lines_total,
        stats.overscan,
        stats.lines_visited,
        stats.lines_painted,
        stats.quads,
        stats.paint_us,
        edit_json,
    );
}

/// Take the pending edit stats (consumed by the next render).
pub fn take_edit() -> Option<EditStats> {
    if let Ok(mut g) = LAST_EDIT.lock() {
        g.take()
    } else {
        None
    }
}

/// Duration helper (us).
pub fn us_since(t0: Instant) -> u64 {
    t0.elapsed().as_micros() as u64
}

fn is_enabled() -> bool {
    if let Ok(g) = ENABLED.lock() {
        *g
    } else {
        false
    }
}
