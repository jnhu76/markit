//! Shared BENCH-GRAMMAR-v1 block parser — the R4 H0 block pass
//! extracted into a resumable, region-capable, splice-capable scanner.
//!
//! The parse algorithm is line-for-line the H0 reference (`§13` of
//! BENCH-GRAMMAR-v1): a line loop with a container stack (quotes / lists
//! / items), total ordered dispatch (B1..B7), fenced-code raw bodies,
//! and a reference-definition table built in source order (first wins).
//! All offsets are UTF-8 byte offsets into the COMPLETE document; a scan
//! covers `[base, end)` where both bounds are line starts (base) or
//! EOF/end-of-region (end).
//!
//! R5 additions, all of them non-semantic for a plain full parse:
//!
//! - [`ContextKey`] — the explicit benchmark context state at a
//!   block-start line (container stack + fence state). Every emitted
//!   block records the key at its entry; horses compare keys at reuse
//!   boundaries. A plain full parse never reads it back.
//! - [`Skel::Spliced`] — a placeholder the scanner emits when the
//!   HORSE-supplied splice hook takes over a byte range (the scanner
//!   never decides reuse; it only executes the take and keeps its own
//!   state — frames, paragraph, fence, spans — correct across the jump).
//!   Spliced placeholders must be replaced by the horse before
//!   materialization; [`inline::materialize`] rejects them.
//!
//! This module owns NO retained mechanism state and no reuse policy: it
//! is shared grammar semantics, identical for every horse (R5 decision
//! freeze §1).

use markit_mdbench_common::WorkSink;
use markit_mdbench_oracle::normalized::NormalizedDocument;

use crate::inline::norm_label;
use crate::inline::RefTable;

/// One container frame of the shared [`ContextKey`] (R5 decision freeze
/// §2): the state BENCH-GRAMMAR-v1 dispatch consumes at a block-start
/// line. `Item.strip` is the item's content indent relative to the
/// enclosing content column; `List.indent` is the marker indent relative
/// to the list's parent content column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameKey {
    Quote,
    List { indent: usize },
    Item { marker: u8, strip: usize },
}

/// The explicit benchmark context state at a block-start line: the open
/// container stack (outermost first) plus the fence state (the open
/// fence's opener run length, or `None`). Structural equality by value;
/// no hashing, no minimality claim (R5 decision freeze §2). Tabs are
/// ordinary characters (D1), so no tab column exists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextKey {
    pub frames: Vec<FrameKey>,
    /// `Some(run_len)` while a fenced code block is open.
    pub fence: Option<usize>,
}

/// Block skeleton (shared-grammar-private intermediate). Paragraph and
/// heading inline content is kept as byte segments and scanned in the
/// inline pass, once the reference-definition table is complete (§13
/// ordering). `ctx` is the entry [`ContextKey`] captured when the block
/// STARTED (the state before its first line was dispatched, excluding
/// the block's own frame) — parse metadata for horses, never read back
/// by a plain parse.
#[derive(Debug, Clone)]
pub enum Skel {
    Para {
        start: usize,
        end: usize,
        segments: Vec<(usize, usize)>,
        ctx: ContextKey,
    },
    Heading {
        start: usize,
        end: usize,
        level: u8,
        content: (usize, usize),
        ctx: ContextKey,
    },
    Quote {
        start: usize,
        end: usize,
        children: Vec<Skel>,
        ctx: ContextKey,
    },
    List {
        start: usize,
        end: usize,
        items: Vec<Skel>,
        ctx: ContextKey,
    },
    Item {
        start: usize,
        end: usize,
        marker: u8,
        children: Vec<Skel>,
        ctx: ContextKey,
    },
    Fence {
        start: usize,
        end: usize,
        info: String,
        content: (usize, usize),
        ctx: ContextKey,
    },
    Def {
        start: usize,
        end: usize,
        label: String,
        destination: String,
        ctx: ContextKey,
    },
    /// Placeholder for a byte range the HORSE took over through the
    /// splice hook. `slot` is the scanner-assigned index of the take
    /// (0-based, in take order); the horse maps slots to its retained
    /// pieces and replaces placeholders before materialization.
    Spliced { start: usize, end: usize, slot: u32 },
}

impl Skel {
    pub fn start(&self) -> usize {
        match self {
            Skel::Para { start, .. }
            | Skel::Heading { start, .. }
            | Skel::Quote { start, .. }
            | Skel::List { start, .. }
            | Skel::Item { start, .. }
            | Skel::Fence { start, .. }
            | Skel::Def { start, .. }
            | Skel::Spliced { start, .. } => *start,
        }
    }

    pub fn end(&self) -> usize {
        match self {
            Skel::Para { end, .. }
            | Skel::Heading { end, .. }
            | Skel::Quote { end, .. }
            | Skel::List { end, .. }
            | Skel::Item { end, .. }
            | Skel::Fence { end, .. }
            | Skel::Def { end, .. }
            | Skel::Spliced { end, .. } => *end,
        }
    }

    pub fn ctx(&self) -> &ContextKey {
        match self {
            Skel::Para { ctx, .. }
            | Skel::Heading { ctx, .. }
            | Skel::Quote { ctx, .. }
            | Skel::List { ctx, .. }
            | Skel::Item { ctx, .. }
            | Skel::Fence { ctx, .. }
            | Skel::Def { ctx, .. } => ctx,
            Skel::Spliced { .. } => panic!("Spliced placeholder has no context"),
        }
    }
}

/// Open container frames (§6 blockquote, §7 list/item). `entry` is the
/// frame's own [`ContextKey`] captured at push time (excluding the frame
/// itself); blocks created inside get it as part of their key.
enum Frame {
    Quote {
        start: usize,
        last_end: usize,
        children: Vec<Skel>,
        entry: ContextKey,
    },
    List {
        start: usize,
        indent: usize,
        items: Vec<Skel>,
        entry: ContextKey,
    },
    Item {
        start: usize,
        marker: u8,
        strip: usize,
        last_end: usize,
        children: Vec<Skel>,
        entry: ContextKey,
    },
}

struct OpenPara {
    start: usize,
    last_end: usize,
    segments: Vec<(usize, usize)>,
    ctx: ContextKey,
}

struct OpenFence {
    start: usize,
    info: String,
    fence_len: usize,
    body_start: usize,
    /// End (LF position) of the last consumed body line.
    last_end: usize,
    ctx: ContextKey,
}

/// The splice hook a horse may install: called at every line start
/// BEFORE the line's container prefixes are consumed, with the parse
/// position and the live entry [`ContextKey`]. Returning `Some(new_pos)`
/// (with `pos < new_pos <= end`) makes the scanner flush an open
/// paragraph, emit one [`Skel::Spliced`] placeholder covering
/// `[pos, new_pos)` into the innermost frame, advance every open frame's
/// last-consumed-line bookkeeping to `new_pos`, and jump. The scanner
/// never decides reuse — the hook does.
pub type SpliceHook<'h> = dyn FnMut(usize, &ContextKey) -> Option<usize> + 'h;

/// The block pass.
struct BlockScanner<'a, 'h, W: WorkSink> {
    src: &'a [u8],
    base: usize,
    end: usize,
    frames: Vec<Frame>,
    para: Option<OpenPara>,
    fence: Option<OpenFence>,
    defs: RefTable,
    doc: Vec<Skel>,
    /// Attribution events only: per-line source-inspection ranges whose
    /// union is the inspected source. Never a timer.
    sink: &'a mut W,
    hook: Option<&'h mut SpliceHook<'h>>,
    slots: u32,
}

/// Clean full parse of a complete BENCH-GRAMMAR-v1 document (no splice
/// hook): block pass + inline pass with the completed definition table.
/// The caller owns the NORMALIZED-RESULT-v1 conformance gate. The inline
/// pass reports its inspected segments to the same sink as the block
/// pass (R5-CORRECTIVE-1, MAJOR-2).
pub fn parse_full<W: WorkSink>(src: &[u8], sink: &mut W) -> NormalizedDocument {
    let (blocks, defs) = {
        let mut scan = BlockScanner::new_region(src, 0, src.len(), sink, None);
        scan.run();
        scan.finish();
        scan.into_result()
    };
    crate::inline::finish_document_with_sink(src, blocks, &defs, sink)
}

/// One region parse result: completed top-level blocks with ABSOLUTE
/// document-coordinate spans (no substring, no re-shift), the
/// definition facts created inside the region, and whether a fenced
/// code block was still open when the region ended (an unclosed fence
/// runs to the region end).
pub struct RegionParse {
    pub blocks: Vec<Skel>,
    pub defs: RefTable,
    pub fence_open_at_end: bool,
}

/// Parse the region `[base, end)` (both line starts or document bounds)
/// with empty entry context, EOF-closing at `end`. `base == end` yields
/// an empty result.
pub fn parse_region<W: WorkSink>(src: &[u8], base: usize, end: usize, sink: &mut W) -> RegionParse {
    let mut scan = BlockScanner::new_region(src, base, end, sink, None);
    scan.run();
    let fence_open_at_end = scan.fence.is_some();
    scan.finish();
    let (blocks, defs) = scan.into_result();
    RegionParse {
        blocks,
        defs,
        fence_open_at_end,
    }
}

/// Parse the region `[base, end)` with the horse's splice hook. Same
/// EOF-close semantics as [`parse_region`]; spliced ranges appear as
/// [`Skel::Spliced`] placeholders (in take order, `slot` 0..n).
pub fn parse_region_with_hook<W: WorkSink>(
    src: &[u8],
    base: usize,
    end: usize,
    sink: &mut W,
    hook: &mut SpliceHook<'_>,
) -> (RegionParse, u32) {
    let mut scan = BlockScanner::new_region(src, base, end, sink, Some(hook));
    scan.run();
    let fence_open_at_end = scan.fence.is_some();
    scan.finish();
    let slots = scan.slots;
    let (blocks, defs) = scan.into_result();
    (
        RegionParse {
            blocks,
            defs,
            fence_open_at_end,
        },
        slots,
    )
}

impl<'a, 'h, W: WorkSink> BlockScanner<'a, 'h, W> {
    fn new_region(
        src: &'a [u8],
        base: usize,
        end: usize,
        sink: &'a mut W,
        hook: Option<&'h mut SpliceHook<'h>>,
    ) -> Self {
        debug_assert!(base <= end && end <= src.len());
        Self {
            src,
            base,
            end,
            frames: Vec::new(),
            para: None,
            fence: None,
            defs: RefTable::new(),
            doc: Vec::new(),
            sink,
            hook,
            slots: 0,
        }
    }

    fn run(&mut self) {
        let mut pos = self.base;
        let end = self.end;
        while pos < end {
            // Splice point: the horse may take over the range starting
            // at this line start. (Never fired while a fence is open —
            // a fence body has no block boundaries to align with.)
            if self.fence.is_none() {
                let key = self.state_key();
                let take = self.hook.as_deref_mut().and_then(|h| h(pos, &key));
                if let Some(new_pos) = take {
                    debug_assert!(pos < new_pos && new_pos <= end);
                    self.splice_to(pos, new_pos);
                    pos = new_pos;
                    continue;
                }
            }
            let line_start = pos;
            let line_lf = memchr_lf(self.src, line_start);
            self.sink
                .record_source_inspection(line_start as u64, (line_lf + 1).min(self.end) as u64);
            // 1. consume container prefixes (§6/§7); may close frames.
            let col = self.strip_prefixes(line_start, line_lf);
            // 2. classify the remainder at the (possibly new) innermost
            //    level; container pushes re-dispatch the same line.
            self.classify(line_start, line_lf, col);
            pos = if line_lf < end { line_lf + 1 } else { line_lf };
        }
        // EOF: unclosed fence runs to the region end (§8); paragraph
        // ends; all containers close. (The caller's finish() does this
        // so it can observe pre-close state first.)
    }

    /// Flush an open paragraph, emit one Spliced placeholder covering
    /// `[pos, new_pos)`, and carry every open frame's last-consumed-line
    /// bookkeeping to `new_pos`. Under the horse's vouching, every line
    /// in `[pos, new_pos)` carried the prefixes of every still-open
    /// frame, so each frame's last consumed line is the line before
    /// `new_pos`. The placeholder span is bookkeeping only — the horse
    /// replaces placeholders with its retained pieces before
    /// materialization.
    ///
    /// The carried check reads the taken range's last byte — the only
    /// source byte this method inspects, and one the per-line reports
    /// never cover because the taken range is skipped outright — so it
    /// is reported to the sink (source-inspection closure).
    fn splice_to(&mut self, pos: usize, new_pos: usize) {
        self.flush_para();
        let slot = self.slots;
        self.slots += 1;
        let spliced = Skel::Spliced {
            start: pos,
            end: new_pos,
            slot,
        };
        self.push_into_innermost(spliced);
        let prev = new_pos.saturating_sub(1);
        let carried = if new_pos > 0 {
            self.sink
                .record_source_inspection(prev as u64, new_pos as u64);
            self.src.get(prev) == Some(&b'\n')
        } else {
            false
        };
        if carried {
            for frame in &mut self.frames {
                match frame {
                    Frame::Quote { last_end, .. } | Frame::Item { last_end, .. } => {
                        *last_end = prev;
                    }
                    Frame::List { .. } => {}
                }
            }
        }
    }

    /// The live entry [`ContextKey`]: open container stack + fence
    /// state. (A splice/entry decision never happens mid-paragraph in a
    /// way that matters: the key describes the state before the line is
    /// dispatched, which is exactly the block-entry semantics of §2 of
    /// the R5 decision freeze.)
    fn state_key(&self) -> ContextKey {
        ContextKey {
            frames: self
                .frames
                .iter()
                .map(|f| match f {
                    Frame::Quote { .. } => FrameKey::Quote,
                    Frame::List { indent, .. } => FrameKey::List { indent: *indent },
                    Frame::Item { marker, strip, .. } => FrameKey::Item {
                        marker: *marker,
                        strip: *strip,
                    },
                })
                .collect(),
            fence: self.fence.as_ref().map(|f| f.fence_len),
        }
    }

    /// The key for a block about to START now (state before dispatch,
    /// excluding the block's own frame-to-be).
    fn entry_key(&self) -> ContextKey {
        self.state_key()
    }

    /// Consume quote/list-item prefixes for one line. Returns the content
    /// column. Lines that can no longer carry an open container's prefix
    /// close that container (and everything inside it) and the line is
    /// re-dispatched at the surviving level (no lazy continuation, D4).
    /// Frames BELOW the closed one already carried their prefixes on this
    /// line, so prefix consumption stops there; the one exception is the
    /// closed item's parent List, which still owes the sibling-vs-close
    /// decision (§7) at the current column.
    fn strip_prefixes(&mut self, line_start: usize, line_lf: usize) -> usize {
        let src = self.src;
        let mut col = line_start;
        let mut idx = 0usize;
        while idx < self.frames.len() {
            let tag = match self.frames.get(idx) {
                None => break,
                Some(Frame::Quote { .. }) => 0,
                Some(Frame::Item { .. }) => 1,
                Some(Frame::List { .. }) => 2,
            };
            match tag {
                // blockquote: up to 3 leading spaces, then '>' (§6 marker
                // rule), then one optional space.
                0 => {
                    let s = count_spaces(src, col, line_lf);
                    let marker_col = col + s.min(3);
                    let carries = marker_col < line_lf && src[marker_col] == b'>';
                    if carries {
                        if let Some(Frame::Quote { last_end, .. }) = self.frames.get_mut(idx) {
                            *last_end = line_lf;
                        }
                        col = marker_col + 1;
                        if col < line_lf && src[col] == b' ' {
                            col += 1;
                        }
                        idx += 1;
                    } else {
                        self.close_frames_from(idx, Some(line_start));
                        break;
                    }
                }
                // list item: exactly the item's content-indent bytes are
                // stripped when the line carries them (§7 continuation)
                1 => {
                    let strip = match self.frames.get(idx) {
                        Some(Frame::Item { strip, .. }) => *strip,
                        _ => unreachable!("tag mismatch"),
                    };
                    let s = count_spaces(src, col, line_lf);
                    if s >= strip {
                        if let Some(Frame::Item { last_end, .. }) = self.frames.get_mut(idx) {
                            *last_end = line_lf;
                        }
                        col += strip;
                        idx += 1;
                    } else {
                        self.close_frames_from(idx, Some(line_start));
                        match self.frames.last() {
                            Some(Frame::List { .. }) => idx = idx.saturating_sub(1),
                            _ => break,
                        }
                    }
                }
                // list: transparent for prefixes. While its own item is
                // still open above it, the list decides nothing. Once the
                // item has been popped for this line, the list either
                // takes the line as a sibling item at exactly its marker
                // indent (§7 sibling rule) or ends.
                _ => {
                    if idx + 1 < self.frames.len() {
                        idx += 1;
                        continue;
                    }
                    let indent = match self.frames.get(idx) {
                        Some(Frame::List { indent, .. }) => *indent,
                        _ => unreachable!("tag mismatch"),
                    };
                    let s = count_spaces(src, col, line_lf);
                    let mut sibling = None;
                    if s == indent {
                        sibling = parse_marker(src, col + s, line_lf);
                    }
                    match sibling {
                        Some((marker, delta)) => {
                            let entry = self.entry_key();
                            self.frames.push(Frame::Item {
                                start: col + s,
                                marker,
                                strip: s + delta,
                                last_end: line_lf,
                                children: Vec::new(),
                                entry,
                            });
                            col = col + s + delta;
                            break;
                        }
                        None => {
                            self.close_frames_from(idx, Some(line_start));
                            break;
                        }
                    }
                }
            }
        }
        col
    }

    /// Dispatch one line at the given content column. Container pushes
    /// (B4/B5) recurse into the remainder of the SAME line.
    fn classify(&mut self, line_start: usize, line_lf: usize, mut col: usize) {
        loop {
            let len = self.end;
            // Fenced-code mode: only the closer ends the block (§8); blank
            // lines and structure lines are raw body bytes.
            if self.fence.is_some() {
                let s = count_spaces(self.src, col, line_lf);
                let cls = col + s.min(3);
                if self.is_closer_line(cls, line_lf) {
                    self.close_fence(line_start);
                } else if let Some(f) = self.fence.as_mut() {
                    f.last_end = line_lf;
                }
                return;
            }
            let s = count_spaces(self.src, col, line_lf);
            let cls = col + s.min(3);
            // B1: blank line at this level (spaces only; a tab is NOT
            // whitespace, §1).
            if all_spaces(self.src, cls, line_lf) {
                self.flush_para();
                match self.frames.last() {
                    // a blank line inside a blockquote does not close it
                    Some(Frame::Quote { .. }) => {}
                    // blank ends every open list/item up to the nearest
                    // enclosing quote (D5 tight-only)
                    _ => {
                        let mut keep = self.frames.len();
                        while keep > 0
                            && matches!(
                                self.frames[keep - 1],
                                Frame::Item { .. } | Frame::List { .. }
                            )
                        {
                            keep -= 1;
                        }
                        self.close_frames_from(keep, Some(line_start));
                    }
                }
                return;
            }
            // B2: fenced code opener
            if let Some((run_len, info)) = self.fence_opener_at(cls, line_lf) {
                self.flush_para();
                let ctx = self.entry_key();
                self.fence = Some(OpenFence {
                    start: cls,
                    info,
                    fence_len: run_len,
                    body_start: if line_lf < len { line_lf + 1 } else { line_lf },
                    last_end: line_lf,
                    ctx,
                });
                return;
            }
            // B3: ATX heading
            if let Some((level, content_start)) = heading_at(self.src, cls, line_lf) {
                self.flush_para();
                let ctx = self.entry_key();
                let skel = Skel::Heading {
                    start: cls,
                    end: line_lf,
                    level,
                    content: (content_start, line_lf),
                    ctx,
                };
                self.push_into_innermost(skel);
                return;
            }
            // B4: blockquote marker — pushes a frame and re-dispatches
            // the rest of the line inside it.
            if cls < line_lf && self.src[cls] == b'>' {
                self.flush_para();
                let ctx = self.entry_key();
                self.frames.push(Frame::Quote {
                    start: cls,
                    last_end: line_lf,
                    children: Vec::new(),
                    entry: ctx,
                });
                col = cls + 1;
                if col < line_lf && self.src[col] == b' ' {
                    col += 1;
                }
                continue;
            }
            // B5: list marker
            if cls < line_lf && (self.src[cls] == b'-' || self.src[cls] == b'*') {
                if let Some((marker, delta)) = parse_marker(self.src, cls, line_lf) {
                    self.flush_para();
                    // relative indents: list markers sit at `s` spaces
                    // past the current content column; the item strips
                    // `s + delta` columns from continuation lines
                    let ctx = self.entry_key();
                    self.frames.push(Frame::List {
                        start: cls,
                        indent: cls - col,
                        items: Vec::new(),
                        entry: ctx.clone(),
                    });
                    let item_ctx = ContextKey {
                        frames: {
                            let mut f = ctx.frames.clone();
                            f.push(FrameKey::List { indent: cls - col });
                            f
                        },
                        fence: None,
                    };
                    self.frames.push(Frame::Item {
                        start: cls,
                        marker,
                        strip: (cls - col) + delta,
                        last_end: line_lf,
                        children: Vec::new(),
                        entry: item_ctx,
                    });
                    col = cls + delta;
                    continue;
                }
            }
            // B6: reference definition (may interrupt a paragraph)
            if cls < line_lf && self.src[cls] == b'[' {
                if let Some((start, end, label, destination)) = refdef_at(self.src, cls, line_lf) {
                    self.flush_para();
                    let ctx = self.entry_key();
                    self.defs.define(label.clone(), destination.clone());
                    self.push_into_innermost(Skel::Def {
                        start,
                        end,
                        label,
                        destination,
                        ctx,
                    });
                    return;
                }
            }
            // B7: paragraph (start or continuation)
            if let Some(p) = self.para.as_mut() {
                // continuation: leading spaces beyond container prefixes
                // are ordinary content bytes (§3). A continuation line
                // separated from the previous segment by ONLY its LF is
                // contiguous content (document-level soft break); a
                // container prefix between them splits segments (§4).
                if let Some(last) = p.segments.last_mut() {
                    if col > last.1 && col == last.1 + 1 {
                        last.1 = line_lf;
                    } else {
                        p.segments.push((col, line_lf));
                    }
                }
                p.last_end = line_lf;
            } else {
                // first line: leading indentation is never part of any
                // span (NORMALIZED-RESULT-v1 §2)
                let ctx = self.entry_key();
                self.para = Some(OpenPara {
                    start: col + s,
                    last_end: line_lf,
                    segments: vec![(col + s, line_lf)],
                    ctx,
                });
            }
            return;
        }
    }

    // -- fence helpers ---------------------------------------------------

    /// `Some((backtick_run_len, info))` if a fence opener starts at
    /// `cls` (§8): run of >= 3 backticks, info string may not contain a
    /// backtick.
    fn fence_opener_at(&self, cls: usize, line_lf: usize) -> Option<(usize, String)> {
        let src = self.src;
        if cls >= line_lf || src[cls] != b'`' {
            return None;
        }
        let mut n = cls;
        while n < line_lf && src[n] == b'`' {
            n += 1;
        }
        let run = n - cls;
        if run < 3 {
            return None;
        }
        let info_bytes = &src[n..line_lf];
        if info_bytes.contains(&b'`') {
            return None;
        }
        let info = String::from_utf8_lossy(info_bytes).into_owned();
        Some((run, info))
    }

    /// Closing fence at `cls`: a run of backticks of length >= the
    /// opener's, then only optional SPACES to end of line (§8).
    fn is_closer_line(&self, cls: usize, line_lf: usize) -> bool {
        let src = self.src;
        let fence_len = match self.fence.as_ref() {
            Some(f) => f.fence_len,
            None => return false,
        };
        if cls >= line_lf || src[cls] != b'`' {
            return false;
        }
        let mut n = cls;
        while n < line_lf && src[n] == b'`' {
            n += 1;
        }
        n - cls >= fence_len && all_spaces(src, n, line_lf)
    }

    /// Finalize a fence at its closing line: span covers the opener's
    /// first backtick through the closer's last backtick byte; the raw
    /// content interval is [opener LF + 1, closer line's first byte).
    fn close_fence(&mut self, closer_line_start: usize) {
        let f = self.fence.take().expect("fence open");
        let src = self.src;
        // locate the closer's backtick run from the physical line start
        let mut n = closer_line_start;
        while n < src.len() && src[n] != b'`' {
            n += 1;
        }
        while n < src.len() && src[n] == b'`' {
            n += 1;
        }
        let ctx = f.ctx;
        self.push_into_innermost(Skel::Fence {
            start: f.start,
            end: n,
            info: f.info,
            content: (f.body_start, closer_line_start),
            ctx,
        });
    }

    /// Unclosed fence at the region end: the one span permitted to
    /// include a final LF — it ends at the region end; content runs to
    /// the region end (§8). A fence nested inside open container frames
    /// ends at the innermost such frame's last consumed line instead: a
    /// child block may never escape its parent's span (the fence's own
    /// last-end bookkeeping already equals the innermost frame's — every
    /// consumed body line carried the frames' prefixes — so the clamp
    /// only bites at the region boundary).
    fn flush_fence_eof(&mut self) {
        if let Some(f) = self.fence.take() {
            let mut end = self.end;
            for frame in &self.frames {
                if let Frame::Quote { last_end, .. } | Frame::Item { last_end, .. } = frame {
                    end = (*last_end).min(end);
                }
            }
            let ctx = f.ctx;
            // The raw-content interval clamps with the span; a fence
            // truncated at its container's extent has an empty content
            // interval at the span end (FencedCode is the one node kind
            // permitted a zero-length content interval).
            let content = (f.body_start.min(end), end);
            self.push_into_innermost(Skel::Fence {
                start: f.start,
                end,
                info: f.info,
                content,
                ctx,
            });
        }
    }

    /// Fence truncated by a closing container: the block ends at the last
    /// consumed body line (excluding that line's terminator); the raw
    /// content ends where the interrupted line begins, clamped into the
    /// span (the interrupting line starts after the span; a fence with no
    /// consumed body line has an empty content interval at the span end).
    fn flush_fence_truncated(&mut self, trunc: usize) {
        if let Some(f) = self.fence.take() {
            let ctx = f.ctx;
            let content = (f.body_start.min(f.last_end), trunc.min(f.last_end));
            self.push_into_innermost(Skel::Fence {
                start: f.start,
                end: f.last_end,
                info: f.info,
                content,
                ctx,
            });
        }
    }

    // -- paragraph / frame helpers ----------------------------------------

    fn flush_para(&mut self) {
        if let Some(p) = self.para.take() {
            let ctx = p.ctx;
            self.push_into_innermost(Skel::Para {
                start: p.start,
                end: p.last_end,
                segments: p.segments,
                ctx,
            });
        }
    }

    fn push_into_innermost(&mut self, skel: Skel) {
        match self.frames.last_mut() {
            Some(Frame::Quote { children, .. }) => children.push(skel),
            Some(Frame::Item { children, .. }) => children.push(skel),
            Some(Frame::List { items, .. }) => items.push(skel),
            None => self.doc.push(skel),
        }
    }

    /// Close every frame from index `keep` upward (innermost first),
    /// flushing any open fence/paragraph into the innermost closing
    /// frame. `trunc` is the current line's start when the close is
    /// triggered by a prefix failure (fence truncation point).
    fn close_frames_from(&mut self, keep: usize, trunc: Option<usize>) {
        if let Some(ts) = trunc {
            self.flush_fence_truncated(ts);
        }
        self.flush_para();
        while self.frames.len() > keep {
            let frame = self.frames.pop().expect("frame");
            let skel = match frame {
                Frame::Quote {
                    start,
                    last_end,
                    children,
                    entry,
                } => Skel::Quote {
                    start,
                    end: last_end,
                    children,
                    ctx: entry,
                },
                Frame::List {
                    start,
                    items,
                    entry,
                    ..
                } => {
                    let end = items.last().map(skel_end).unwrap_or(start);
                    Skel::List {
                        start,
                        end,
                        items,
                        ctx: entry,
                    }
                }
                Frame::Item {
                    start,
                    marker,
                    last_end,
                    children,
                    entry,
                    ..
                } => Skel::Item {
                    start,
                    end: last_end,
                    marker,
                    children,
                    ctx: entry,
                },
            };
            self.push_into_innermost(skel);
        }
    }

    /// EOF-close at the region end: unclosed fence, open paragraph, all
    /// frames.
    fn finish(&mut self) {
        self.flush_fence_eof();
        self.flush_para();
        self.close_frames_from(0, None);
    }

    fn into_result(self) -> (Vec<Skel>, RefTable) {
        (self.doc, self.defs)
    }
}

fn skel_end(s: &Skel) -> usize {
    s.end()
}

// ---------------------------------------------------------------------------
// Lexical line classification (shared, pure) — used by horses for
// soundness gates on UNPARSED bytes. Same rules as the dispatch in
// `classify` (B1-B7), evaluated without touching scanner state.
// ---------------------------------------------------------------------------

/// Lexical class of the line `[line_start, line_lf)` evaluated at the
/// top level (empty container context, no open fence) with the content
/// column at `line_start`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineClass {
    /// Spaces only (§1 blank).
    Blank,
    /// Fenced code opener: `(run_len, has_info_bytes)`.
    FenceOpener { run_len: usize },
    /// ATX heading with level 1..=6.
    Heading { level: u8 },
    /// Blockquote marker line ('>' after <= 3 spaces).
    QuoteMarker,
    /// List marker: `(marker_byte, first-line content delta)` (§7).
    ListMarker { marker: u8, delta: usize },
    /// Reference definition (§9.4).
    RefDef,
    /// None of B1-B6: paragraph text (starts or continues a paragraph).
    ParagraphText,
}

pub fn classify_top_level_line(src: &[u8], line_start: usize, line_lf: usize) -> LineClass {
    let s = count_spaces(src, line_start, line_lf);
    let cls = line_start + s.min(3);
    if all_spaces(src, cls, line_lf) {
        return LineClass::Blank;
    }
    if let Some((run_len, _)) = fence_opener_at(src, cls, line_lf) {
        return LineClass::FenceOpener { run_len };
    }
    if let Some((level, _)) = heading_at(src, cls, line_lf) {
        return LineClass::Heading { level };
    }
    if src[cls] == b'>' {
        return LineClass::QuoteMarker;
    }
    if src[cls] == b'-' || src[cls] == b'*' {
        if let Some((marker, delta)) = parse_marker(src, cls, line_lf) {
            return LineClass::ListMarker { marker, delta };
        }
    }
    if src[cls] == b'[' && refdef_at(src, cls, line_lf).is_some() {
        return LineClass::RefDef;
    }
    LineClass::ParagraphText
}

/// The fence-opener check without scanner state (§8).
pub fn fence_opener_at(src: &[u8], cls: usize, line_lf: usize) -> Option<(usize, String)> {
    if cls >= line_lf || src[cls] != b'`' {
        return None;
    }
    let mut n = cls;
    while n < line_lf && src[n] == b'`' {
        n += 1;
    }
    let run = n - cls;
    if run < 3 {
        return None;
    }
    let info_bytes = &src[n..line_lf];
    if info_bytes.contains(&b'`') {
        return None;
    }
    let info = String::from_utf8_lossy(info_bytes).into_owned();
    Some((run, info))
}

// ---------------------------------------------------------------------------
// lexical helpers (all byte-based; source is guaranteed UTF-8)
// ---------------------------------------------------------------------------

pub fn memchr_lf(src: &[u8], from: usize) -> usize {
    src[from..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| from + p)
        .unwrap_or(src.len())
}

/// `line_start_of`-shaped backward scan with attribution: reports the
/// bytes the scan actually inspected — `[found_lf, pos)` (or `[0, pos)`
/// before the first line). Mechanism-work source reads use this form so
/// every inspected byte reaches the attribution lane
/// (R5-CORRECTIVE-2, source-inspection closure); contexts that report
/// their own covering range may keep the plain form.
pub fn line_start_of_reported<W: WorkSink>(src: &[u8], pos: usize, sink: &mut W) -> usize {
    let found = src[..pos].iter().rposition(|&b| b == b'\n');
    sink.record_source_inspection(found.map_or(0, |p| p) as u64, pos as u64);
    found.map_or(0, |p| p + 1)
}

/// [`memchr_lf`] with attribution: reports the bytes the scan actually
/// inspected — `[from, lf]` including the terminator when found,
/// `[from, len)` otherwise.
pub fn memchr_lf_reported<W: WorkSink>(src: &[u8], from: usize, sink: &mut W) -> usize {
    let found = src[from..].iter().position(|&b| b == b'\n');
    sink.record_source_inspection(
        from as u64,
        found.map_or(src.len(), |p| from + p + 1) as u64,
    );
    found.map_or(src.len(), |p| from + p)
}

pub fn count_spaces(src: &[u8], from: usize, to: usize) -> usize {
    let mut n = from;
    while n < to && src[n] == b' ' {
        n += 1;
    }
    n - from
}

pub fn all_spaces(src: &[u8], from: usize, to: usize) -> bool {
    src[from..to].iter().all(|&b| b == b' ')
}

/// ATX heading at `cls` (§5): 1..=6 '#', then EOL or 1+ spaces then
/// content. Returns `(level, content_start)`.
pub fn heading_at(src: &[u8], cls: usize, line_lf: usize) -> Option<(u8, usize)> {
    if cls >= line_lf || src[cls] != b'#' {
        return None;
    }
    let mut n = cls;
    while n < line_lf && src[n] == b'#' {
        n += 1;
    }
    let run = n - cls;
    if run > 6 {
        return None;
    }
    if n == line_lf {
        return Some((run as u8, line_lf));
    }
    if src[n] != b' ' {
        return None;
    }
    while n < line_lf && src[n] == b' ' {
        n += 1;
    }
    Some((run as u8, n))
}

/// List marker at `p` (§7): '-' or '*', then 1..=4 spaces, or the marker
/// alone at end-of-line (empty item). Returns
/// `(marker_byte, delta)` where the item's first-line content column is
/// `p + delta`; `k > 4` caps the delta at 2 (frozen CommonMark rule —
/// the remaining spaces are content bytes).
pub fn parse_marker(src: &[u8], p: usize, line_lf: usize) -> Option<(u8, usize)> {
    let marker = src[p];
    if marker != b'-' && marker != b'*' {
        return None;
    }
    let mut n = p + 1;
    while n < line_lf && src[n] == b' ' {
        n += 1;
    }
    let k = n - (p + 1);
    if k == 0 {
        if n == line_lf {
            // empty item: content indent = marker column + 1
            return Some((marker, 1));
        }
        return None;
    }
    if k <= 4 {
        Some((marker, 1 + k))
    } else {
        Some((marker, 2))
    }
}

/// Reference definition at `cls` (§9.4): `[label]:` + 1..=N spaces +
/// destination (no space) + only optional spaces to EOL. Returns
/// `(node_start, node_end, normalized_label, destination)`.
#[allow(clippy::type_complexity)]
pub fn refdef_at(src: &[u8], cls: usize, line_lf: usize) -> Option<(usize, usize, String, String)> {
    if cls >= line_lf || src[cls] != b'[' {
        return None;
    }
    let mut n = cls + 1;
    while n < line_lf && src[n] != b']' {
        n += 1;
    }
    if n >= line_lf {
        return None;
    }
    let label_raw = &src[cls + 1..n];
    if n + 1 >= line_lf || src[n + 1] != b':' {
        return None;
    }
    let mut d = n + 2;
    let spaces = count_spaces(src, d, line_lf);
    if spaces == 0 {
        return None; // one or more SPACES required after ':'
    }
    d += spaces;
    let dest_start = d;
    while d < line_lf && src[d] != b' ' {
        d += 1;
    }
    let destination_bytes = &src[dest_start..d];
    if !all_spaces(src, d, line_lf) {
        return None; // only optional spaces allowed after destination
    }
    let label = norm_label(label_raw);
    let destination = String::from_utf8_lossy(destination_bytes).into_owned();
    // span ends at the destination's last byte (empty destination:
    // after the ':')
    let end = if destination_bytes.is_empty() {
        dest_start
    } else {
        d
    };
    Some((cls, end, label, destination))
}

#[cfg(test)]
mod tests {
    use super::*;
    use markit_mdbench_common::{CounterSink, NoopWorkSink, WorkCounters};
    use markit_mdbench_oracle::normalized::NodeKind;

    #[test]
    fn full_parse_shapes_a_simple_document() {
        let src = b"# t\n\npara *b* [l](/u)\n";
        let mut noop = NoopWorkSink;
        let doc = parse_full(src, &mut noop);
        assert_eq!(doc.root.kind, NodeKind::Document);
        assert_eq!(doc.root.children.len(), 2);
        assert_eq!(doc.root.children[0].kind, NodeKind::Heading);
        assert_eq!(doc.root.children[1].kind, NodeKind::Paragraph);
    }

    #[test]
    fn region_parse_matches_full_parse_on_a_suffix_region() {
        let src = b"# h\n\ntext one\n\ntext two\n";
        let mut noop = NoopWorkSink;
        // byte 15 is the start of the line "text two"
        let region = parse_region(src, 15, src.len(), &mut noop);
        assert_eq!(region.blocks.len(), 1);
        assert!(!region.fence_open_at_end);
        // absolute spans, no re-shift needed
        assert_eq!(region.blocks[0].start(), 15);
    }

    /// Regression (R5 adversarial small-model generator, ContainerState
    /// family): an unclosed fence nested in a container EOF-closes at the
    /// container's extent — never past its parent's span. Before the fix
    /// the fence ran to the REGION end and the child escaped the quote,
    /// tripping the NORMALIZED-RESULT-v1 parent-containment check.
    #[test]
    fn eof_fence_inside_a_container_never_escapes_its_parent() {
        let mut noop = NoopWorkSink;
        // Quote opens on the marker line and the fence opens inside it;
        // EOF closes both. The fence must end at the quote's last
        // consumed line, not at the document end.
        for src in [
            &b"> ```\n\nx\n"[..],
            &b"> ```\n"[..],
            &b"- > ```\n\nx\n"[..],
        ] {
            let region = parse_region(src, 0, src.len(), &mut noop);
            let doc = crate::inline::finish_document(src, region.blocks, &region.defs);
            markit_mdbench_oracle::validate_root(&doc.root, Some(src))
                .expect("child fence escapes its container");
        }
        // A TOP-LEVEL unclosed fence still runs to the region end and may
        // include the final LF (§8's one permitted span).
        let src = b"para\n\n```\nbody\n";
        let region = parse_region(src, 0, src.len(), &mut noop);
        assert!(region.fence_open_at_end);
        let fence = region.blocks.last().expect("fence block");
        assert_eq!(fence.end(), src.len(), "top-level fence ends at region end");
    }

    #[test]
    fn splice_hook_runs_and_produces_placeholders() {
        let src = b"aaa\n\nbbb\n\nccc\n";
        let mut noop = NoopWorkSink;
        let mut taken = 0usize;
        let mut hook = |pos: usize, key: &ContextKey| -> Option<usize> {
            assert!(key.fence.is_none());
            if pos == 5 {
                taken += 1;
                Some(9) // take the "bbb" line including its LF
            } else {
                None
            }
        };
        let (region, slots) = parse_region_with_hook(src, 0, src.len(), &mut noop, &mut hook);
        assert_eq!(taken, 1);
        assert_eq!(slots, 1);
        let spliced: Vec<_> = region
            .blocks
            .iter()
            .filter(|b| matches!(b, Skel::Spliced { .. }))
            .collect();
        assert_eq!(spliced.len(), 1);
        assert!(matches!(
            region.blocks.iter().find(|b| b.start() == 5).unwrap(),
            Skel::Spliced {
                start: 5,
                end: 9,
                slot: 0
            }
        ));
        // the surrounding paragraphs still parsed around the take
        assert_eq!(region.blocks.len(), 3);
    }

    /// Attribution regression (R5-CORRECTIVE-2 reviewer round): a
    /// successful take skips `[pos, new_pos)` outright — the per-line
    /// inspection reports never cover it — yet `splice_to` itself reads
    /// the range's last byte to carry frame bookkeeping. That read must
    /// be visible as an exact EVENT, or the mechanism work the take
    /// replaced goes unreported and `unique_source_bytes_inspected`
    /// understates real source work.
    #[test]
    fn splice_take_reports_its_tail_byte_source_inspection() {
        let src = b"aaa\n\nbbb\n\nccc\n";
        let mut counters = WorkCounters::all_unknown();
        let mut sink = CounterSink::new(&mut counters);
        let mut hook = |pos: usize, _key: &ContextKey| -> Option<usize> {
            if pos == 5 {
                Some(9) // take "bbb\n" = [5, 9)
            } else {
                None
            }
        };
        let (region, slots) = parse_region_with_hook(src, 0, src.len(), &mut sink, &mut hook);
        assert_eq!(slots, 1);
        assert!(matches!(
            region.blocks.iter().find(|b| b.start() == 5).unwrap(),
            Skel::Spliced {
                start: 5,
                end: 9,
                slot: 0
            }
        ));
        // splice_to's carried-LF read is the tail byte of the taken
        // range: exactly the event (8, 9). The range's interior [5, 8)
        // is reused untouched — no event may cover it.
        let events = sink.inspections();
        assert!(
            events.contains(&(8, 9)),
            "splice tail byte (8, 9) not reported; events: {events:?}"
        );
        assert!(
            !events.iter().any(|&(s, e)| s >= 5 && e <= 8),
            "reused-range interior bytes must stay uninspected; events: {events:?}"
        );
    }

    #[test]
    fn context_keys_distinguish_container_state() {
        let src = b"> q1\n> q2\n\ntop\n";
        let mut noop = NoopWorkSink;
        let (blocks, _) = {
            let mut scan = BlockScanner::new_region(src, 0, src.len(), &mut noop, None);
            scan.run();
            scan.finish();
            scan.into_result()
        };
        assert_eq!(blocks.len(), 2);
        let quote = &blocks[0];
        assert!(quote.ctx().frames.is_empty());
        match quote {
            Skel::Quote { children, .. } => {
                assert_eq!(children[0].ctx().frames, vec![FrameKey::Quote]);
            }
            _ => panic!("expected quote"),
        }
        assert!(blocks[1].ctx().frames.is_empty());
    }
}
