//! H0 reference parser — BENCH-GRAMMAR-v1 implemented literally.
//!
//! Authority: `grammar/BENCH-GRAMMAR-v1.md` (frozen R3 semantics) +
//! `grammar/NORMALIZED-RESULT-v1.md` (result vocabulary). This parser is
//! the R4 semantic reference (`H0 = FULL_REBUILD`): a clean, eager,
//! total, deterministic, byte-coordinate-correct full parse. It is NOT
//! intended to be fast and contains no reuse machinery of any kind.
//!
//! Structure follows the frozen §13 shape:
//!
//! ```text
//! 1. block pass: line loop with a container stack (quotes / lists /
//!    items), total ordered dispatch (B1..B7), fenced-code raw bodies,
//!    reference-definition table built in source order (first wins).
//! 2. inline pass (AFTER the block pass): paragraphs / headings /
//!    link-text regions are scanned with the completed definition
//!    table — resolution is document-global and position-independent.
//! 3. normalized tree (oracle vocabulary).
//! ```
//!
//! All offsets are UTF-8 byte offsets into the complete document. The
//! source is always valid UTF-8 (the `Source` substrate guarantees it),
//! so byte slices at LF/space/ASCII-structure boundaries are always char
//! boundaries; multibyte scalars are only ever crossed by raw content
//! regions (fence bodies, text runs), never split.

use markit_mdbench_common::WorkSink;
use markit_mdbench_oracle::normalized::{Node, NodeKind, NormalizedDocument};

use crate::inline::{materialize, norm_label, RefTable};

/// Intermediate block skeleton (parser-private). Paragraph/heading inline
/// content is kept as byte segments and scanned in the second pass, once
/// the reference-definition table is complete (§13 ordering).
pub(crate) enum Skel {
    Para {
        start: usize,
        end: usize,
        segments: Vec<(usize, usize)>,
    },
    Heading {
        start: usize,
        end: usize,
        level: u8,
        content: (usize, usize),
    },
    Quote {
        start: usize,
        end: usize,
        children: Vec<Skel>,
    },
    List {
        start: usize,
        end: usize,
        items: Vec<Skel>,
    },
    Item {
        start: usize,
        end: usize,
        marker: u8,
        children: Vec<Skel>,
    },
    Fence {
        start: usize,
        end: usize,
        info: String,
        content: (usize, usize),
    },
    Def {
        start: usize,
        end: usize,
        label: String,
        destination: String,
    },
}

/// Open container frames (§6 blockquote, §7 list/item).
///
/// `Item.strip` is the number of spaces this frame consumes from a
/// continuation line relative to the ENCLOSING content column (the
/// absolute content indent minus what outer frames already stripped);
/// `List.indent` is the marker indent relative to the list's parent
/// content column.
enum Frame {
    Quote {
        start: usize,
        last_end: usize,
        children: Vec<Skel>,
    },
    List {
        start: usize,
        indent: usize,
        items: Vec<Skel>,
    },
    Item {
        start: usize,
        marker: u8,
        strip: usize,
        last_end: usize,
        children: Vec<Skel>,
    },
}

struct OpenPara {
    start: usize,
    last_end: usize,
    segments: Vec<(usize, usize)>,
}

struct OpenFence {
    start: usize,
    info: String,
    fence_len: usize,
    body_start: usize,
    /// End (LF position) of the last consumed body line.
    last_end: usize,
}

/// The block pass.
struct BlockParser<'a, W: WorkSink> {
    src: &'a [u8],
    frames: Vec<Frame>,
    para: Option<OpenPara>,
    fence: Option<OpenFence>,
    defs: RefTable,
    doc: Vec<Skel>,
    /// Attribution events only: per-line source-inspection ranges whose
    /// union is the complete source. Never a timer.
    sink: &'a mut W,
}

/// Clean full parse of a complete BENCH-GRAMMAR-v1 document.
pub fn parse(src: &[u8]) -> NormalizedDocument {
    let mut noop = markit_mdbench_common::NoopWorkSink;
    parse_with_inspection(src, &mut noop)
}

/// Clean full parse, emitting the block-pass source-inspection events
/// into `sink`. The parser inspects every byte exactly once per pass;
/// the inline pass re-inspects subsets of already-emitted line ranges
/// (the common collector unions them, so no double count).
pub fn parse_with_inspection<W: WorkSink>(src: &[u8], sink: &mut W) -> NormalizedDocument {
    let mut bp = BlockParser {
        src,
        frames: Vec::new(),
        para: None,
        fence: None,
        defs: RefTable::new(),
        doc: Vec::new(),
        sink,
    };
    bp.run();
    bp.finish()
}

impl<'a, W: WorkSink> BlockParser<'a, W> {
    fn run(&mut self) {
        let mut pos = 0usize;
        let len = self.src.len();
        while pos < len {
            let line_start = pos;
            let line_lf = memchr_lf(self.src, line_start);
            self.sink
                .record_source_inspection(line_start as u64, (line_lf + 1).min(len) as u64);
            // 1. consume container prefixes (§6/§7); may close frames.
            let col = self.strip_prefixes(line_start, line_lf);
            // 2. classify the remainder at the (possibly new) innermost
            //    level; container pushes re-dispatch the same line.
            self.classify(line_start, line_lf, col);
            pos = if line_lf < len { line_lf + 1 } else { line_lf };
        }
        // EOF: unclosed fence runs to EOF (§8); paragraph ends; all
        // containers close.
        self.flush_fence_eof();
        self.flush_para();
        self.close_frames_from(0, None);
    }

    /// Consume quote/list-item prefixes for one line. Returns the content
    /// column. Lines that can no longer carry an open container's prefix
    /// close that container (and everything inside it) and the line is
    /// re-dispatched at the outer level (no lazy continuation, D4) — the
    /// frame BELOW the closed one is re-examined against the same line.
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
                        idx = idx.saturating_sub(1);
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
                        idx = idx.saturating_sub(1);
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
                            self.frames.push(Frame::Item {
                                start: col + s,
                                marker,
                                strip: s + delta,
                                last_end: line_lf,
                                children: Vec::new(),
                            });
                            col = col + s + delta;
                            break;
                        }
                        None => {
                            self.close_frames_from(idx, Some(line_start));
                            idx = idx.saturating_sub(1);
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
            let len = self.src.len();
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
                self.fence = Some(OpenFence {
                    start: cls,
                    info,
                    fence_len: run_len,
                    body_start: if line_lf < len { line_lf + 1 } else { line_lf },
                    last_end: line_lf,
                });
                return;
            }
            // B3: ATX heading
            if let Some((level, content_start)) = heading_at(self.src, cls, line_lf) {
                self.flush_para();
                let skel = Skel::Heading {
                    start: cls,
                    end: line_lf,
                    level,
                    content: (content_start, line_lf),
                };
                self.push_into_innermost(skel);
                return;
            }
            // B4: blockquote marker — pushes a frame and re-dispatches
            // the rest of the line inside it.
            if cls < line_lf && self.src[cls] == b'>' {
                self.flush_para();
                self.frames.push(Frame::Quote {
                    start: cls,
                    last_end: line_lf,
                    children: Vec::new(),
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
                    self.frames.push(Frame::List {
                        start: cls,
                        indent: cls - col,
                        items: Vec::new(),
                    });
                    self.frames.push(Frame::Item {
                        start: cls,
                        marker,
                        strip: (cls - col) + delta,
                        last_end: line_lf,
                        children: Vec::new(),
                    });
                    col = cls + delta;
                    continue;
                }
            }
            // B6: reference definition (may interrupt a paragraph)
            if cls < line_lf && self.src[cls] == b'[' {
                if let Some((start, end, label, destination)) = refdef_at(self.src, cls, line_lf) {
                    self.flush_para();
                    self.defs.define(label.clone(), destination.clone());
                    self.push_into_innermost(Skel::Def {
                        start,
                        end,
                        label,
                        destination,
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
                self.para = Some(OpenPara {
                    start: col + s,
                    last_end: line_lf,
                    segments: vec![(col + s, line_lf)],
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
        self.push_into_innermost(Skel::Fence {
            start: f.start,
            end: n,
            info: f.info,
            content: (f.body_start, closer_line_start),
        });
    }

    /// Unclosed fence at EOF: the one span permitted to include a final
    /// LF — it ends at len(document); content runs to EOF (§8).
    fn flush_fence_eof(&mut self) {
        if let Some(f) = self.fence.take() {
            let len = self.src.len();
            self.push_into_innermost(Skel::Fence {
                start: f.start,
                end: len,
                info: f.info,
                content: (f.body_start, len),
            });
        }
    }

    /// Fence truncated by a closing container: the block ends at the last
    /// consumed body line (excluding that line's terminator); the raw
    /// content ends where the interrupted line begins.
    fn flush_fence_truncated(&mut self, trunc: usize) {
        if let Some(f) = self.fence.take() {
            self.push_into_innermost(Skel::Fence {
                start: f.start,
                end: f.last_end,
                info: f.info,
                content: (f.body_start, trunc),
            });
        }
    }

    // -- paragraph / frame helpers ----------------------------------------

    fn flush_para(&mut self) {
        if let Some(p) = self.para.take() {
            self.push_into_innermost(Skel::Para {
                start: p.start,
                end: p.last_end,
                segments: p.segments,
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
                } => Skel::Quote {
                    start,
                    end: last_end,
                    children,
                },
                Frame::List { start, items, .. } => {
                    let end = items.last().map(skel_end).unwrap_or(start);
                    Skel::List { start, end, items }
                }
                Frame::Item {
                    start,
                    marker,
                    last_end,
                    children,
                    ..
                } => Skel::Item {
                    start,
                    end: last_end,
                    marker,
                    children,
                },
            };
            self.push_into_innermost(skel);
        }
    }

    fn finish(mut self) -> NormalizedDocument {
        let len = self.src.len();
        let doc_skels = std::mem::take(&mut self.doc);
        let defs = self.defs;
        let children = materialize(self.src, doc_skels, &defs);
        let mut root = Node::new(NodeKind::Document, 0, len);
        root.children = children;
        NormalizedDocument::new(root)
    }
}

fn skel_end(s: &Skel) -> usize {
    match s {
        Skel::Para { end, .. }
        | Skel::Heading { end, .. }
        | Skel::Quote { end, .. }
        | Skel::List { end, .. }
        | Skel::Item { end, .. }
        | Skel::Fence { end, .. }
        | Skel::Def { end, .. } => *end,
    }
}

// ---------------------------------------------------------------------------
// lexical helpers (all byte-based; source is guaranteed UTF-8)
// ---------------------------------------------------------------------------

fn memchr_lf(src: &[u8], from: usize) -> usize {
    src[from..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| from + p)
        .unwrap_or(src.len())
}

fn count_spaces(src: &[u8], from: usize, to: usize) -> usize {
    let mut n = from;
    while n < to && src[n] == b' ' {
        n += 1;
    }
    n - from
}

fn all_spaces(src: &[u8], from: usize, to: usize) -> bool {
    src[from..to].iter().all(|&b| b == b' ')
}

/// ATX heading at `cls` (§5): 1..=6 '#', then EOL or 1+ spaces then
/// content. Returns `(level, content_start)`.
fn heading_at(src: &[u8], cls: usize, line_lf: usize) -> Option<(u8, usize)> {
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
fn parse_marker(src: &[u8], p: usize, line_lf: usize) -> Option<(u8, usize)> {
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
fn refdef_at(src: &[u8], cls: usize, line_lf: usize) -> Option<(usize, usize, String, String)> {
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
