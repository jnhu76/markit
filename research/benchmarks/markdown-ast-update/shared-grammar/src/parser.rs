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
#[derive(Debug, Clone, PartialEq, Eq)]
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
struct BlockScanner<'a, 'h, 'o, W: WorkSink> {
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
    observer: Option<&'o mut dyn RegionObserver>,
    /// The LF byte this scan consumed to enter the current physical line,
    /// or `None` while the current line is still the scan's first. This is
    /// the only source of `RootBlankEvent::preceding_lf`: line provenance
    /// the scan actually established, never a re-read of the source.
    prev_lf: Option<usize>,
    /// Cut of the certified root blank barrier the observer stopped at.
    /// `Some` means the parse ended there, sealed, without EOF closure.
    stop_requested: Option<usize>,
    slots: u32,
}

/// Clean full parse of a complete BENCH-GRAMMAR-v1 document (no splice
/// hook): block pass + inline pass with the completed definition table.
/// The caller owns the NORMALIZED-RESULT-v1 conformance gate. The inline
/// pass reports its inspected segments to the same sink as the block
/// pass (R5-CORRECTIVE-1, MAJOR-2).
pub fn parse_full<W: WorkSink>(src: &[u8], sink: &mut W) -> NormalizedDocument {
    let (blocks, defs) = {
        let mut scan = BlockScanner::new_region(src, 0, src.len(), sink, None, None);
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
#[derive(Debug, PartialEq, Eq)]
pub struct RegionParse {
    pub blocks: Vec<Skel>,
    pub defs: RefTable,
    pub fence_open_at_end: bool,
}

/// A root-level top-level block start (Horse-A §5.4), issued at the
/// actual dispatch that opens the block's first physical line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopLevelEvent {
    /// First byte of the block's first PHYSICAL line. Never the semantic
    /// span start: a span starts after leading spaces and container
    /// prefixes, the physical line starts before them.
    pub physical_line_start: usize,
}

/// A root-safe blank barrier (Horse-A §5.1), issued at the end of a real
/// physical root `SPACES* LF` blank line whose grammar processing left no
/// live root state, so every byte before `cut` is already sealed.
///
/// This is parser EVIDENCE. I1 retains nothing: turning it into a
/// persisted restart certificate (with its support range) is a later
/// slice's concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootBlankEvent {
    /// Physical start of the blank line.
    pub line_start: usize,
    /// The LF byte actually consumed for this physical line.
    pub line_lf: usize,
    /// `line_lf + 1`: the end of the blank line, and the region boundary a
    /// local stop returns at.
    pub cut: usize,
    /// The LF byte that establishes `line_start` as a physical line start,
    /// or `None` at BOF (`line_start == 0`). Support of a future
    /// certificate is `{preceding_lf}` + `[line_start, cut)`.
    ///
    /// Events are issued only where this antecedent holds: a scan that
    /// begins AT a blank line never consumed its establishing LF, so it
    /// issues no barrier for it rather than claiming BOF support.
    pub preceding_lf: Option<usize>,
}

/// The observer's bounded control request, answerable only at a
/// [`RootBlankEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverControl {
    Continue,
    Stop,
}

/// Receives parser transition observations. The observer owns no grammar
/// state and cannot mutate the parse: the only thing it can ask for is a
/// bounded early stop at a certified root blank barrier.
pub trait RegionObserver {
    /// A new root-level top-level block begins at this physical line.
    fn on_top_level_start(&mut self, ev: TopLevelEvent);

    /// The parser just sealed a root-safe blank barrier; the observer may
    /// ask to end the region parse at its cut.
    fn on_root_blank_barrier(&mut self, ev: RootBlankEvent) -> ObserverControl;
}

/// How an observed region parse ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionOutcome {
    /// The scan consumed the whole region; ordinary EOF closure applies.
    RanToEnd,
    /// The observer stopped at a certified root blank barrier. The region
    /// was already sealed there, so this is a successful parse of
    /// `[base, cut)` — not a parser error and not an EOF completion.
    StoppedAtCertifiedCut { cut: usize },
}

/// An observed region parse: the [`RegionParse`] plus how it ended. On
/// [`RegionOutcome::StoppedAtCertifiedCut`] the result is the sealed parse
/// of `[base, cut)`.
#[derive(Debug, PartialEq, Eq)]
pub struct ObservedRegionParse {
    pub region: RegionParse,
    pub outcome: RegionOutcome,
}

/// Parse the region `[base, end)` with an observer installed. This is the
/// observed sibling of [`parse_region`]: Horse-A's local parsing uses this
/// entry point and never a [`SpliceHook`].
///
/// Installing an observer may not change the parse. A no-op observer —
/// one that ignores every event and always continues — yields exactly the
/// [`RegionParse`] and the exactly the same sink event stream as
/// [`parse_region`], because both entries run the same grammar code.
///
/// The region base is this scan's BOF authority: it is never itself treated
/// as a root blank barrier, because the scan cannot know (without reading
/// outside the bytes it reports as inspected) what precedes it.
pub fn parse_region_observed<W: WorkSink>(
    src: &[u8],
    base: usize,
    end: usize,
    sink: &mut W,
    observer: &mut dyn RegionObserver,
) -> ObservedRegionParse {
    let mut scan = BlockScanner::new_region(src, base, end, sink, None, Some(observer));
    scan.run();
    let outcome = match scan.stop_requested {
        Some(cut) => RegionOutcome::StoppedAtCertifiedCut { cut },
        None => RegionOutcome::RanToEnd,
    };
    let fence_open_at_end = scan.fence.is_some();
    match outcome {
        // A certified stop is already sealed (no frames, no paragraph, no
        // fence), so EOF closure is not run: real EOF is a separate
        // completion path, and running it here would blur the two.
        RegionOutcome::StoppedAtCertifiedCut { .. } => debug_assert!(
            scan.frames.is_empty() && scan.para.is_none() && scan.fence.is_none(),
            "a certified stop must leave an empty root"
        ),
        RegionOutcome::RanToEnd => scan.finish(),
    }
    let (blocks, defs) = scan.into_result();
    ObservedRegionParse {
        region: RegionParse {
            blocks,
            defs,
            fence_open_at_end,
        },
        outcome,
    }
}

/// Parse the region `[base, end)` (both line starts or document bounds)
/// with empty entry context, EOF-closing at `end`. `base == end` yields
/// an empty result.
pub fn parse_region<W: WorkSink>(src: &[u8], base: usize, end: usize, sink: &mut W) -> RegionParse {
    let mut scan = BlockScanner::new_region(src, base, end, sink, None, None);
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
    let mut scan = BlockScanner::new_region(src, base, end, sink, Some(hook), None);
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

impl<'a, 'h, 'o, W: WorkSink> BlockScanner<'a, 'h, 'o, W> {
    fn new_region(
        src: &'a [u8],
        base: usize,
        end: usize,
        sink: &'a mut W,
        hook: Option<&'h mut SpliceHook<'h>>,
        observer: Option<&'o mut dyn RegionObserver>,
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
            observer,
            prev_lf: None,
            stop_requested: None,
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
            self.sink.record_source_inspection(
                markit_mdbench_common::SourceVersion::Post,
                line_start as u64,
                (line_lf + 1).min(self.end) as u64,
            );
            // 1. consume container prefixes (§6/§7); may close frames.
            let col = self.strip_prefixes(line_start, line_lf);
            // 2. classify the remainder at the (possibly new) innermost
            //    level; container pushes re-dispatch the same line.
            self.classify(line_start, line_lf, col);
            // An accepted stop ends the region parse here: this physical
            // line is fully processed and everything before its cut is
            // sealed, so the next physical line is never dispatched.
            if self.stop_requested.is_some() {
                return;
            }
            // This line's terminator, if it has one, is what establishes
            // the next line start (and the next line's barrier support).
            self.prev_lf = if line_lf < end { Some(line_lf) } else { None };
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
            self.sink.record_source_inspection(
                markit_mdbench_common::SourceVersion::Post,
                prev as u64,
                new_pos as u64,
            );
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
        // A take that ends on an LF leaves the next line start established
        // by that LF; one that ends mid-line establishes nothing.
        self.prev_lf = if carried { Some(prev) } else { None };
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

    // -- observation seam ------------------------------------------------

    /// Issue a [`TopLevelEvent`] when this dispatch begins a new ROOT-level
    /// top-level block.
    ///
    /// The offset is the block's first PHYSICAL line start, which is not
    /// its semantic span start: leading spaces and container prefixes are
    /// excluded from the span but are part of the line. Nothing is issued
    /// while a container frame is live — every descendant block (nested
    /// quotes/lists, text inside them) stays inside the Owner opened by
    /// its root start — and a paragraph continuation line issues nothing
    /// because its block is already open.
    fn observe_top_level_start(&mut self, physical_line_start: usize) {
        // No observer installed: the unobserved path does no observation
        // work at all, and no descendant start may ever be issued.
        if self.observer.is_none() || !self.frames.is_empty() {
            return;
        }
        if let Some(observer) = self.observer.as_deref_mut() {
            observer.on_top_level_start(TopLevelEvent {
                physical_line_start,
            });
        }
    }

    /// Issue a [`RootBlankEvent`] for a physical root `SPACES* LF` blank
    /// line. Called from B1 only, i.e. after the grammar's own paragraph
    /// flush and container closure have run, because the frozen condition
    /// is about the LIVE post-B1 root state: an entry-state context key
    /// being empty says nothing (a blank line inside a quote has an empty
    /// content-level state yet must not certify), and with the paragraph
    /// still open the output before the cut would not be sealed.
    ///
    /// The barrier is support-carrying evidence, so it is issued only when
    /// the scan can name the LF that establishes this line as a physical
    /// line start. A scan whose own base is the blank line never consumed
    /// that LF and therefore issues nothing rather than fabricating BOF
    /// support for it.
    fn observe_root_blank_barrier(&mut self, line_start: usize, line_lf: usize) {
        // The unobserved path issues nothing and evaluates none of this.
        if self.observer.is_none() {
            return;
        }
        // A spaces-only tail that never had an LF consumed is not an
        // interior blank line; real EOF is a separate completion path.
        if line_lf >= self.end {
            return;
        }
        if !self.frames.is_empty() || self.para.is_some() || self.fence.is_some() {
            return;
        }
        let preceding_lf = if line_start == 0 {
            None
        } else {
            match self.prev_lf {
                Some(lf) => {
                    debug_assert_eq!(lf, line_start - 1, "line provenance drift");
                    Some(lf)
                }
                None => return,
            }
        };
        let ev = RootBlankEvent {
            line_start,
            line_lf,
            cut: line_lf + 1,
            preceding_lf,
        };
        if let Some(observer) = self.observer.as_deref_mut() {
            if let ObserverControl::Stop = observer.on_root_blank_barrier(ev) {
                self.stop_requested = Some(ev.cut);
            }
        }
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
                // Only now — real prefixes consumed, real B1 flush and
                // closure done, completed blocks emitted — is the live
                // root state meaningful to observe.
                self.observe_root_blank_barrier(line_start, line_lf);
                return;
            }
            // B2: fenced code opener
            if let Some((run_len, info)) = self.fence_opener_at(cls, line_lf) {
                self.flush_para();
                self.observe_top_level_start(line_start);
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
                self.observe_top_level_start(line_start);
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
                self.observe_top_level_start(line_start);
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
                    self.observe_top_level_start(line_start);
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
                    self.observe_top_level_start(line_start);
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
            if self.para.is_none() {
                // A paragraph START is a new top-level block; a continuation
                // line belongs to the paragraph already open.
                self.observe_top_level_start(line_start);
            }
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
    line_start_of_reported_in(markit_mdbench_common::SourceVersion::Post, src, pos, sink)
}

/// [`line_start_of_reported`] with an EXPLICIT source version
/// (MEASUREMENT-CORRECTIVE-1 §18): mechanism consultations of the OLD
/// retained source report `Old`; substrate scans of the source being
/// parsed report `Post`.
pub fn line_start_of_reported_in<W: WorkSink>(
    version: markit_mdbench_common::SourceVersion,
    src: &[u8],
    pos: usize,
    sink: &mut W,
) -> usize {
    let found = src[..pos].iter().rposition(|&b| b == b'\n');
    sink.record_source_inspection(version, found.map_or(0, |p| p) as u64, pos as u64);
    found.map_or(0, |p| p + 1)
}

/// [`memchr_lf`] with attribution: reports the bytes the scan actually
/// inspected — `[from, lf]` including the terminator when found,
/// `[from, len)` otherwise.
pub fn memchr_lf_reported<W: WorkSink>(src: &[u8], from: usize, sink: &mut W) -> usize {
    memchr_lf_reported_in(markit_mdbench_common::SourceVersion::Post, src, from, sink)
}

/// [`memchr_lf_reported`] with an EXPLICIT source version
/// (MEASUREMENT-CORRECTIVE-1 §18).
pub fn memchr_lf_reported_in<W: WorkSink>(
    version: markit_mdbench_common::SourceVersion,
    src: &[u8],
    from: usize,
    sink: &mut W,
) -> usize {
    let found = src[from..].iter().position(|&b| b == b'\n');
    sink.record_source_inspection(
        version,
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
///
/// A marker must be a byte INSIDE the physical line: `p >= line_lf` is
/// no marker (bounds guard, #66 — at an unterminated EOF line
/// `line_lf == src.len()`, so reading `src[p]` there was out of bounds).
/// When an LF does exist at `line_lf` it is never '-'/'*', so this guard
/// changes no grammar decision — it only makes the exhausted-line answer
/// explicit instead of panicking.
pub fn parse_marker(src: &[u8], p: usize, line_lf: usize) -> Option<(u8, usize)> {
    if p >= line_lf {
        return None;
    }
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
    use markit_mdbench_common::{CounterSink, NoopWorkSink, SourceVersion, WorkCounters};
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

    /// Regression (SHARED-GRAMMAR-EOF-MARKER-1, #66 — surfaced by the
    /// independent Horse-A I2 review): a container-marker-only final
    /// line with no LF used to reach the list sibling test with a
    /// position AT the physical line bound, and `parse_marker` indexed
    /// `src[p]` with `p == line_lf == src.len()` → index out of bounds.
    ///
    /// The bounds guard makes the sibling test answer "no marker" —
    /// exactly the decision the terminated twin makes (the byte at
    /// `line_lf`, when one exists, is the LF and never a marker) — so
    /// the unterminated tail flows through the same frozen closure:
    /// sibling fails → the list closes → the blank settles inside the
    /// still-open quote → EOF finish closes the quote. All spans below
    /// are hand-derived from the byte layouts and that control flow.
    #[test]
    fn an_unterminated_marker_only_eof_tail_parses_like_its_terminated_twin() {
        // "> - x\n>" (len 7): quote carries '>' at 6; the item dies
        // (nothing left to strip); the list's sibling test sees an empty
        // remainder at EOF and finds no marker → list closes inside the
        // quote. Quote last consumed line is [.., 7).
        let mut noop = NoopWorkSink;
        let doc = parse_full(b"> - x\n>", &mut noop);
        assert_eq!(doc.root.kind, NodeKind::Document);
        assert_eq!((doc.root.start, doc.root.end), (0, 7));
        assert_eq!(doc.root.children.len(), 1);
        let quote = &doc.root.children[0];
        assert_eq!(quote.kind, NodeKind::BlockQuote);
        assert_eq!((quote.start, quote.end), (0, 7));
        let list = &quote.children[0];
        assert_eq!(list.kind, NodeKind::List);
        assert_eq!((list.start, list.end), (2, 5));
        let item = &list.children[0];
        assert_eq!(item.kind, NodeKind::ListItem);
        assert_eq!((item.start, item.end), (2, 5));
        assert_eq!(item.marker.as_deref(), Some("-"));
        let para = &item.children[0];
        assert_eq!(para.kind, NodeKind::Paragraph);
        assert_eq!((para.start, para.end), (4, 5));
        let text = &para.children[0];
        assert_eq!(text.kind, NodeKind::Text);
        assert_eq!((text.start, text.end), (4, 5));
        markit_mdbench_oracle::validate_root(&doc.root, Some(b"> - x\n>"))
            .expect("unterminated quote tail violates NORMALIZED-RESULT-v1");

        // "> - x\n> " (len 8): identical walk; the trailing space is
        // consumed as the quote's optional post-marker space, so the
        // quote runs to 8.
        let doc = parse_full(b"> - x\n> ", &mut noop);
        assert_eq!((doc.root.start, doc.root.end), (0, 8));
        let quote = &doc.root.children[0];
        assert_eq!((quote.start, quote.end), (0, 8));
        let list = &quote.children[0];
        assert_eq!((list.start, list.end), (2, 5));
        let item = &list.children[0];
        assert_eq!((item.start, item.end), (2, 5));
        let para = &item.children[0];
        assert_eq!((para.start, para.end), (4, 5));
        markit_mdbench_oracle::validate_root(&doc.root, Some(b"> - x\n> "))
            .expect("unterminated quote-space tail violates NORMALIZED-RESULT-v1");

        // The terminated twins make the identical structural decisions:
        // "> - x\n>\n" — the LF after '>' is not a marker either.
        let terminated = parse_full(b"> - x\n>\n", &mut noop);
        assert_eq!(terminated.root.children.len(), 1);
        let twin = &terminated.root.children[0];
        assert_eq!(twin.kind, NodeKind::BlockQuote);
        assert_eq!((twin.start, twin.end), (0, 7));
        let terminated = parse_full(b"> - x\n> \n", &mut noop);
        let twin = &terminated.root.children[0];
        assert_eq!((twin.start, twin.end), (0, 8));
    }

    /// Same defect without any quote: a spaces-only EOF tail that the
    /// outer item carries but the inner item cannot — the inner list's
    /// sibling test ran off the line bound and panicked.
    #[test]
    fn a_spaces_only_eof_tail_after_a_nested_list_parses() {
        // "- - x\n  " (len 8): the two spaces satisfy the outer item's
        // strip (2) but not the inner item's after the outer strip, so
        // the inner item closes and the inner list's sibling test sees
        // the empty EOF remainder — no marker → inner list closes; the
        // blank closes the outer list at EOF.
        let mut noop = NoopWorkSink;
        let doc = parse_full(b"- - x\n  ", &mut noop);
        assert_eq!(doc.root.children.len(), 1);
        let outer = &doc.root.children[0];
        assert_eq!(outer.kind, NodeKind::List);
        let outer_item = &outer.children[0];
        assert_eq!(outer_item.kind, NodeKind::ListItem);
        let inner = &outer_item.children[0];
        assert_eq!(inner.kind, NodeKind::List);
        assert_eq!((inner.start, inner.end), (2, 5));
        let inner_item = &inner.children[0];
        assert_eq!((inner_item.start, inner_item.end), (2, 5));
        let para = &inner_item.children[0];
        assert_eq!((para.start, para.end), (4, 5));
        markit_mdbench_oracle::validate_root(&doc.root, Some(b"- - x\n  "))
            .expect("nested-list spaces tail violates NORMALIZED-RESULT-v1");
    }

    /// Nearby marker-tail forms that must keep their existing behavior:
    /// a real marker byte present at EOF opens a sibling item (empty
    /// item, §7's "marker alone at end-of-line" rule).
    #[test]
    fn a_marker_byte_present_at_eof_still_opens_an_empty_sibling_item() {
        let mut noop = NoopWorkSink;
        for (src, marker) in [(b"> - x\n> -".as_slice(), "-"), (b"> - x\n> *", "*")] {
            let doc = parse_full(src, &mut noop);
            let quote = &doc.root.children[0];
            assert_eq!(quote.kind, NodeKind::BlockQuote);
            let list = &quote.children[0];
            assert_eq!(list.kind, NodeKind::List);
            assert_eq!(list.children.len(), 2, "sibling item for {src:?}");
            assert_eq!(list.children[1].marker.as_deref(), Some(marker));
            assert!(
                list.children[1].children.is_empty(),
                "empty item for {src:?}"
            );
            markit_mdbench_oracle::validate_root(&doc.root, Some(src))
                .expect("marker tail violates NORMALIZED-RESULT-v1");
        }
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
        // is reused untouched — no event may OVERLAP it (an event that
        // merely pokes into it, e.g. ending at 6 or starting at 7, is
        // just as much an unreported mechanism-work read).
        let events = sink.inspections();
        assert!(
            events.contains(&(SourceVersion::Post, 8, 9)),
            "splice tail byte (8, 9) not reported; events: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|&(v, s, e)| v == SourceVersion::Post && s < 8 && e > 5),
            "reused-range interior [5, 8) must stay uninspected (no overlapping event); events: {events:?}"
        );
    }

    #[test]
    fn context_keys_distinguish_container_state() {
        let src = b"> q1\n> q2\n\ntop\n";
        let mut noop = NoopWorkSink;
        let (blocks, _) = {
            let mut scan = BlockScanner::new_region(src, 0, src.len(), &mut noop, None, None);
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

    /// Barrier support is line PROVENANCE, and a hook take moves the scan
    /// position without reading a line. Only the internal seam can install
    /// a hook and an observer together, so this invariant has no public
    /// path: a take ending ON an LF establishes the next line start (and
    /// its support LF), a take ending mid-line establishes nothing, so the
    /// blank-looking position right after it does not certify.
    #[test]
    fn barrier_line_provenance_is_kept_across_a_splice_jump() {
        struct Rec(Vec<RootBlankEvent>);
        impl RegionObserver for Rec {
            fn on_top_level_start(&mut self, _ev: TopLevelEvent) {}
            fn on_root_blank_barrier(&mut self, ev: RootBlankEvent) -> ObserverControl {
                self.0.push(ev);
                ObserverControl::Continue
            }
        }

        // "a\n\nb\n\nc\n": bytes 0..8. Taking [0, 5) ends on the LF at 4,
        // so byte 5 is established as a line start and the blank line there
        // carries support {4} + [5, 6).
        let src = b"a\n\nb\n\nc\n";
        let mut noop = NoopWorkSink;
        let mut rec = Rec(Vec::new());
        let mut taken = false;
        let mut hook = |pos: usize, _key: &ContextKey| -> Option<usize> {
            if !taken && pos == 0 {
                taken = true;
                Some(5)
            } else {
                None
            }
        };
        {
            let hook: &mut SpliceHook<'_> = &mut hook;
            let mut scan =
                BlockScanner::new_region(src, 0, src.len(), &mut noop, Some(hook), Some(&mut rec));
            scan.run();
        }
        assert_eq!(
            rec.0,
            vec![RootBlankEvent {
                line_start: 5,
                line_lf: 5,
                cut: 6,
                preceding_lf: Some(4),
            }]
        );

        // Taking [0, 4) ends mid-line: the LF the take consumed is not
        // this line's, so position 4 issues nothing, and the scan resumes
        // its own provenance at the next real blank line.
        let mut rec = Rec(Vec::new());
        let mut taken = false;
        let mut hook = |pos: usize, _key: &ContextKey| -> Option<usize> {
            if !taken && pos == 0 {
                taken = true;
                Some(4)
            } else {
                None
            }
        };
        {
            let hook: &mut SpliceHook<'_> = &mut hook;
            let mut scan =
                BlockScanner::new_region(src, 0, src.len(), &mut noop, Some(hook), Some(&mut rec));
            scan.run();
        }
        assert_eq!(
            rec.0,
            vec![RootBlankEvent {
                line_start: 5,
                line_lf: 5,
                cut: 6,
                preceding_lf: Some(4),
            }]
        );
    }
}
