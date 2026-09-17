//! Mutation instantiation for CORPUS-v1 sources (MUTATION-v1).
//!
//! Everything here is a pure, deterministic function of
//! `(source bytes, recipe/operation, anchor class)`. Selection ambiguity is
//! resolved exactly as MUTATION-v1 §2.1 freezes it: minimum distance wins,
//! then lower byte offset, then source order. Selectors phrased "first X
//! at/after an anchor" are source-order first and treat the line
//! CONTAINING the anchor as "at" the anchor (needed for totality of
//! M-FS-FENCE-OPEN on every applicable corpus, consistent with the frozen
//! 42-case expansion).
//!
//! These functions PRODUCE candidate edits for the frozen workload; the
//! correctness judge for every edit is the differential rule
//! `update == clean parse` enforced by the R4 test suite, never this file.

use std::collections::HashSet;

use markit_mdbench_common::PayloadShape;

/// Frozen position classes (MUTATION-v1 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorClass {
    Early,
    Middle,
    Late,
}

/// Frozen edit-size classes (MUTATION-v1 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditSize {
    Tiny,
    Small,
    Medium,
}

/// Frozen generic operations (MUTATION-v1 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenericOp {
    Insert,
    Delete,
    ReplaceEq,
    ReplaceGrow,
    ReplaceShrink,
}

/// The 13 frozen structural recipes (MUTATION-v1 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Recipe {
    LocText,
    LocUtf8Swap,
    BbParaSplit,
    BbParaMerge,
    CsItemIndent,
    CsBqNestLine,
    FsFenceOpen,
    FsFenceClose,
    IdsEmphInsert,
    IdsCodeDelim,
    IdsLinkDelim,
    SdDefReplace,
    SdDefDelete,
}

impl Recipe {
    pub fn id(self) -> &'static str {
        match self {
            Recipe::LocText => "M-LOC-TEXT",
            Recipe::LocUtf8Swap => "M-LOC-UTF8-SWAP",
            Recipe::BbParaSplit => "M-BB-PARA-SPLIT",
            Recipe::BbParaMerge => "M-BB-PARA-MERGE",
            Recipe::CsItemIndent => "M-CS-ITEM-INDENT",
            Recipe::CsBqNestLine => "M-CS-BQ-NEST-LINE",
            Recipe::FsFenceOpen => "M-FS-FENCE-OPEN",
            Recipe::FsFenceClose => "M-FS-FENCE-CLOSE",
            Recipe::IdsEmphInsert => "M-IDS-EMPH-INSERT",
            Recipe::IdsCodeDelim => "M-IDS-CODE-DELIM",
            Recipe::IdsLinkDelim => "M-IDS-LINK-DELIM",
            Recipe::SdDefReplace => "M-SD-DEF-REPLACE",
            Recipe::SdDefDelete => "M-SD-DEF-DELETE",
        }
    }

    /// The recipe's frozen selection anchor (MUTATION-v1 §5); M-FS-FENCE-OPEN
    /// takes TWO case variants (early + middle) at the call site.
    pub fn selection_anchor(self) -> AnchorClass {
        match self {
            Recipe::SdDefReplace | Recipe::SdDefDelete => AnchorClass::Early,
            _ => AnchorClass::Middle,
        }
    }
}

/// A planned canonical edit: `[start, end)` replaced by `inserted`
/// (R0 §6). `anchor_offset` records the ACTUAL byte offset after
/// dephasing + fitting + down-snapping (MUTATION-v1 §2 requires both the
/// requested class and the actual offset to be recorded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedEdit {
    pub start: usize,
    pub end: usize,
    pub inserted: Vec<u8>,
    pub anchor_class: AnchorClass,
    pub anchor_offset: usize,
}

impl PlannedEdit {
    /// Zero-width edits are INSERT; empty insertions are DELETE; equal
    /// lengths REPLACE_EQ; then grow/shrink (byte lengths are authority).
    pub fn byte_op(&self) -> &'static str {
        let removed = self.end - self.start;
        let added = self.inserted.len();
        if removed == 0 && added > 0 {
            "insert"
        } else if removed > 0 && added == 0 {
            "delete"
        } else if removed == added {
            "replace_eq"
        } else if added > removed {
            "replace_grow"
        } else {
            "replace_shrink"
        }
    }
}

/// A recipe whose frozen precondition failed on this corpus. Recorded,
/// never silently skipped (MUTATION-v1 §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotApplicable {
    pub recipe: &'static str,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Anchor machinery (MUTATION-v1 §2)
// ---------------------------------------------------------------------------

/// Raw percentile anchor: EARLY = N/4, MIDDLE = N/2, LATE = 3N/4 (floor).
pub fn raw_anchor(class: AnchorClass, n: usize) -> usize {
    match class {
        AnchorClass::Early => n / 4,
        AnchorClass::Middle => n / 2,
        AnchorClass::Late => (n / 4) * 3,
    }
}

/// The generic anchor: raw + 7 (the universal dephasing constant), BEFORE
/// range fitting and snapping.
pub fn generic_anchor(class: AnchorClass, n: usize) -> usize {
    raw_anchor(class, n) + 7
}

/// Is `pos` a UTF-8 char boundary of `src`? (Byte-level rule: the byte at
/// `pos` must not be a continuation byte.)
pub fn is_char_boundary(src: &[u8], pos: usize) -> bool {
    pos == 0 || pos >= src.len() || src[pos] & 0xC0 != 0x80
}

/// Snap DOWN to the nearest UTF-8 char boundary of `src` (step 3 of the
/// frozen pipeline). ASCII positions are unchanged.
pub fn snap_down(src: &[u8], mut pos: usize) -> usize {
    while pos > 0 && !is_char_boundary(src, pos) {
        pos -= 1;
    }
    pos
}

/// The three snapped generic anchors of the QUERY batch (ordered EARLY,
/// MIDDLE, LATE — NORMALIZED-RESULT-v1 §4).
pub fn query_anchors(src: &[u8]) -> [usize; 3] {
    [
        snap_down(src, generic_anchor(AnchorClass::Early, src.len())),
        snap_down(src, generic_anchor(AnchorClass::Middle, src.len())),
        snap_down(src, generic_anchor(AnchorClass::Late, src.len())),
    ]
}

// ---------------------------------------------------------------------------
// Generic edits (MUTATION-v1 §3)
// ---------------------------------------------------------------------------

/// Instantiate one generic `(class, size, op)` edit on `src`.
pub fn generic_edit(src: &[u8], class: AnchorClass, size: EditSize, op: GenericOp) -> PlannedEdit {
    let n = src.len();
    match size {
        EditSize::Tiny | EditSize::Small => {
            let l = match size {
                EditSize::Tiny => 3,
                _ => 32,
            };
            let mut a = generic_anchor(class, n);
            if op != GenericOp::Insert {
                a = a.min(n.saturating_sub(l)); // fit the range
            }
            let a = snap_down(src, a);
            let (start, end, inserted) = match op {
                GenericOp::Insert => (a, a, b"x".repeat(l)),
                GenericOp::Delete => (a, a + l, Vec::new()),
                GenericOp::ReplaceEq => (a, a + l, vec![b'z'; l]),
                GenericOp::ReplaceGrow => (a, a + l, vec![b'z'; 2 * l]),
                GenericOp::ReplaceShrink => (a, a + l, b"z".to_vec()),
            };
            PlannedEdit {
                start,
                end,
                inserted,
                anchor_class: class,
                anchor_offset: a,
            }
        }
        EditSize::Medium => medium_edit(src, class, op),
    }
}

/// MEDIUM: one line (the line containing the snapped anchor); if that line
/// is longer than 4096 bytes, a 512-byte window `[anchor, anchor+512)`
/// (snapped, clamped) replaces the line region.
fn medium_edit(src: &[u8], class: AnchorClass, op: GenericOp) -> PlannedEdit {
    let n = src.len();
    let a = snap_down(src, generic_anchor(class, n));
    let line_start = src[..a]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |p| p + 1);
    let lf = src[line_start..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(n - 1, |p| line_start + p);
    let line_len = lf + 1 - line_start; // line INCLUDING its LF

    const Q31: &[u8] = b"qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"; // "q" x 31

    let (start, end, inserted) = if line_len > 4096 {
        // Capped window form (huge_block's giant lines).
        let mut wend = (a + 512).min(n);
        while wend > a && !is_char_boundary(src, wend) {
            wend -= 1;
        }
        let w = wend - a;
        match op {
            GenericOp::Insert => (a, a, insert_medium_bytes()),
            GenericOp::Delete => (a, wend, Vec::new()),
            GenericOp::ReplaceEq => (a, wend, vec![b'z'; w]),
            GenericOp::ReplaceGrow => {
                let mut ins = vec![b'z'; w];
                ins.extend_from_slice(&Q31[..32.min(Q31.len())]);
                ins.push(b'q');
                (a, wend, ins)
            }
            GenericOp::ReplaceShrink => (a, wend, vec![b'z'; w - 1]),
        }
    } else {
        match op {
            GenericOp::Insert => (line_start, line_start, insert_medium_bytes()),
            GenericOp::Delete => (line_start, lf + 1, Vec::new()),
            GenericOp::ReplaceEq => {
                let mut ins = vec![b'z'; line_len - 1];
                ins.push(b'\n');
                (line_start, lf + 1, ins)
            }
            GenericOp::ReplaceGrow => {
                let mut ins = vec![b'z'; line_len - 1];
                ins.push(b'\n');
                ins.extend_from_slice(Q31);
                ins.push(b'\n');
                (line_start, lf + 1, ins)
            }
            GenericOp::ReplaceShrink => {
                let s = (line_len - 1).min(16);
                let mut ins = vec![b'z'; s - 1];
                ins.push(b'\n');
                (line_start, lf + 1, ins)
            }
        }
    };
    PlannedEdit {
        start,
        end,
        inserted,
        anchor_class: class,
        anchor_offset: a,
    }
}

fn insert_medium_bytes() -> Vec<u8> {
    let mut ins = vec![b'm'; 23];
    ins.push(b'\n');
    ins
}

// ---------------------------------------------------------------------------
// Source-structure scans (generator-side, faithful for CORPUS-v1 recipes)
// ---------------------------------------------------------------------------

/// Line bounds as `(start, lf_pos)`; the LF at `lf_pos` terminates the
/// line. CORPUS-v1 sources always end with LF.
pub(crate) fn line_bounds(src: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, &b) in src.iter().enumerate() {
        if b == b'\n' {
            out.push((start, i));
            start = i + 1;
        }
    }
    if start < src.len() {
        out.push((start, src.len() - 1));
    }
    out
}

fn leading_spaces(content: &[u8]) -> usize {
    content
        .iter()
        .take(3)
        .position(|&b| b != b' ')
        .unwrap_or(3)
        .min(content.len())
}

fn hash_count(rest: &[u8]) -> usize {
    rest.iter().take_while(|&&b| b == b'#').count()
}

/// One fenced region (BENCH-GRAMMAR-v1 §8): opener line, raw body span,
/// and the closer (if any).
pub(crate) struct FenceRegion {
    pub opener_start: usize,
    /// Raw body bytes `[body_start, body_end)` — between the opener's LF
    /// and the closer line's first byte (or EOF when unclosed).
    pub body: (usize, usize),
    /// `(start, end_incl_lf)` of the closing fence line when closed.
    pub closer: Option<(usize, usize)>,
}

/// All fenced regions in document order. Opener: a line whose content
/// (after at most 3 leading spaces) begins with a run of >= 3 backticks
/// whose info string contains no backtick. Closer: a later line whose
/// content is a run of the SAME length (plus optional trailing spaces).
pub(crate) fn fence_regions(src: &[u8]) -> Vec<FenceRegion> {
    let mut out = Vec::new();
    let mut open: Option<(usize, usize, usize)> = None; // (opener_start, body_start, run_len)
    for (ls, lf) in line_bounds(src) {
        let content = &src[ls..lf];
        let rest = &content[leading_spaces(content)..];
        let run = rest.iter().take_while(|&&b| b == b'`').count();
        let info = &rest[run.min(rest.len())..];
        let spaces_only = info.iter().all(|&b| b == b' ');
        match open {
            None => {
                if run >= 3 && !info.contains(&b'`') {
                    open = Some((ls, lf + 1, run));
                }
            }
            Some((ostart, bstart, orun)) => {
                if run >= 3 && run == orun && spaces_only {
                    out.push(FenceRegion {
                        opener_start: ostart,
                        body: (bstart, ls),
                        closer: Some((ls, lf + 1)),
                    });
                    open = None;
                }
            }
        }
    }
    if let Some((ostart, bstart, _)) = open {
        out.push(FenceRegion {
            opener_start: ostart,
            body: (bstart, src.len()),
            closer: None,
        });
    }
    out
}

fn body_of(regions: &[FenceRegion], pos: usize) -> bool {
    regions.iter().any(|r| pos >= r.body.0 && pos < r.body.1)
}

/// Whole-region byte spans `(opener_start, region_end)` — a fenced region
/// covers its opener line, raw body, and closer line. No inline construct
/// exists inside a covered span (fence markers are not code delimiters).
fn fence_cover(regions: &[FenceRegion]) -> Vec<(usize, usize)> {
    regions
        .iter()
        .map(|r| (r.opener_start, r.closer.map_or(r.body.1, |(_, e)| e)))
        .collect()
}

/// Jump past the covered span containing `i` (identity when uncovered).
fn skip_covered(spans: &[(usize, usize)], i: usize) -> usize {
    match spans.iter().find(|&&(s, e)| i >= s && i < e) {
        Some(&(_, e)) => e,
        None => i,
    }
}

/// Is `ls` a fence line at all (opener, closer, or inside the body)?
fn fence_line(regions: &[FenceRegion], ls: usize) -> bool {
    regions.iter().any(|r| {
        r.opener_start == ls
            || body_of(std::slice::from_ref(r), ls)
            || r.closer.is_some_and(|(cs, _)| cs == ls)
    })
}

/// Maximal ASCII-lowercase-letter runs outside fence regions, skipping
/// reference-definition lines and ATX heading lines (their bytes are not
/// paragraph Text). Returns `(start, end)` pairs.
fn text_letter_runs(src: &[u8], fences: &[FenceRegion]) -> Vec<(usize, usize)> {
    let mut excluded_lines: Vec<(usize, usize)> = Vec::new();
    for (ls, lf) in line_bounds(src) {
        let content = &src[ls..lf];
        let rest = &content[leading_spaces(content)..];
        let hashes = hash_count(rest);
        let is_heading = (1..=6).contains(&hashes) && rest.get(hashes).is_none_or(|&b| b == b' ');
        let is_refdef = rest.first() == Some(&b'[') && rest.contains(&b':');
        if is_heading || is_refdef {
            excluded_lines.push((ls, lf + 1));
        }
    }
    let mut runs = Vec::new();
    let mut i = 0;
    while i < src.len() {
        if src[i].is_ascii_lowercase() {
            let start = i;
            while i < src.len() && src[i].is_ascii_lowercase() {
                i += 1;
            }
            let excluded = excluded_lines
                .iter()
                .any(|&(ls, le)| start >= ls && start < le)
                || body_of(fences, start);
            if !excluded {
                runs.push((start, i));
            }
        } else {
            i += 1;
        }
    }
    runs
}

/// Nearest candidate by §2.1: minimum `|candidate_start − anchor|`, ties
/// to the lower offset.
fn nearest<T>(
    candidates: impl IntoIterator<Item = T>,
    anchor: usize,
    key: impl Fn(&T) -> usize,
) -> Option<T> {
    let mut best: Option<(usize, usize, T)> = None; // (distance, start, item)
    for c in candidates {
        let start = key(&c);
        let dist = start.abs_diff(anchor);
        let better = match &best {
            None => true,
            Some((bd, bs, _)) => dist < *bd || (dist == *bd && start < *bs),
        };
        if better {
            best = Some((dist, start, c));
        }
    }
    best.map(|(_, _, c)| c)
}

/// All CJK scalar start positions (the corpus-restricted set: 中 E4 B8 AD
/// and 文 E6 96 87; these byte triples occur as exactly those scalars in
/// CORPUS-v1).
fn cjk_positions(src: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < src.len() {
        if (src[i] == 0xE4 && src[i + 1] == 0xB8 && src[i + 2] == 0xAD)
            || (src[i] == 0xE6 && src[i + 1] == 0x96 && src[i + 2] == 0x87)
        {
            out.push(i);
            i += 3;
        } else {
            i += 1;
        }
    }
    out
}

/// Interior paragraph-blank-paragraph separators: positions `i` with
/// `src[i..i+2] == "\n\n"`, a non-empty paragraph line before, a non-empty
/// paragraph line after, outside fence bodies ("two adjacent paragraphs
/// separated by exactly one blank line").
fn blank_separators(src: &[u8], fences: &[FenceRegion]) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 1..src.len().saturating_sub(2) {
        if src[i] == b'\n' && src[i + 1] == b'\n' && src[i - 1] != b'\n' && src[i + 2] != b'\n' {
            let after = i + 2;
            let before_line_start = src[..i]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |p| p + 1);
            if is_paragraph_line(src, fences, before_line_start)
                && is_paragraph_line(src, fences, after)
            {
                out.push(i);
            }
        }
    }
    out
}

/// Is the line starting at `ls` an ordinary paragraph line (non-blank,
/// not a container/heading/refdef/fence line, outside fence bodies)?
/// Faithful for CORPUS-v1 (all container markers live at line starts).
fn is_paragraph_line(src: &[u8], fences: &[FenceRegion], ls: usize) -> bool {
    let lf = src[ls..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(src.len(), |p| ls + p);
    let content = &src[ls..lf];
    if content.is_empty() || body_of(fences, ls) {
        return false;
    }
    let rest = &content[leading_spaces(content)..];
    let first = rest.first().copied().unwrap_or(b' ');
    let hashes = hash_count(rest);
    let heading = first == b'#' && (1..=6).contains(&hashes);
    let marker = (first == b'-' || first == b'*') && rest.get(1) == Some(&b' ');
    let quote = first == b'>';
    let refdef = first == b'[' && rest.contains(&b':');
    let fence = rest.iter().take_while(|&&b| b == b'`').count() >= 3;
    !(heading || marker || quote || refdef || fence)
}

/// Reference definitions: `(line_start, line_end_incl_lf, normalized
/// label, dest (start, end))` in document order. First-wins resolution is
/// the caller's concern; here every definition line is reported.
fn refdefs(src: &[u8]) -> Vec<(usize, usize, String, (usize, usize))> {
    let mut out = Vec::new();
    for (ls, lf) in line_bounds(src) {
        let content = &src[ls..lf];
        let rest_at = ls + leading_spaces(content);
        let rest = &content[rest_at - ls..];
        if rest.first() != Some(&b'[') {
            continue;
        }
        let Some(close) = rest.iter().position(|&b| b == b']') else {
            continue;
        };
        if rest.get(close + 1) != Some(&b':') {
            continue;
        }
        let label = String::from_utf8_lossy(&rest[1..close]).to_ascii_lowercase();
        let mut dest_start = close + 2;
        while dest_start < rest.len() && rest[dest_start] == b' ' {
            dest_start += 1;
        }
        out.push((ls, lf + 1, label, (rest_at + dest_start, lf)));
    }
    out
}

fn norm(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_ascii_lowercase()
}

/// Reference-link label set (first-wins per BENCH-GRAMMAR-v1 §9.2).
fn definition_table(src: &[u8]) -> HashSet<String> {
    let mut table = HashSet::new();
    for (_, _, label, _) in refdefs(src) {
        table.insert(label);
    }
    table
}

fn run_len(src: &[u8], at: usize, b: u8) -> usize {
    src[at..].iter().take_while(|&&x| x == b).count()
}

/// Next run of EXACTLY `n` backticks at or after `from`.
fn find_run_exact(src: &[u8], from: usize, n: usize) -> Option<usize> {
    let mut i = from;
    while i < src.len() {
        if src[i] == b'`' {
            let run = run_len(src, i, b'`');
            if run == n {
                return Some(i);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

/// Link candidate at `at` (`src[at] == b'['`): `Some(end)` when the bytes
/// form an inline link or a RESOLVED reference link.
fn link_candidate(src: &[u8], at: usize, defs: &HashSet<String>) -> Option<usize> {
    let text_close = src[at + 1..].iter().position(|&b| b == b']')? + at + 1;
    let after = text_close + 1;
    match src.get(after)? {
        b'(' => {
            // inline destination: bytes to the first ')', failing on
            // SPACE, '[' or no ')' before the next LF (§9.2)
            let mut p = after + 1;
            loop {
                let b = *src.get(p)?;
                if b == b')' {
                    return Some(p + 1);
                }
                if b == b' ' || b == b'[' || b == b'\n' {
                    return None;
                }
                p += 1;
            }
        }
        b'[' => {
            // reference: label to the next ']'; empty label = collapsed
            let label_close = src[after + 1..].iter().position(|&b| b == b']')? + after + 1;
            let label_bytes = &src[after + 1..label_close];
            let label = if label_bytes.is_empty() {
                norm(&src[at + 1..text_close])
            } else {
                norm(label_bytes)
            };
            if defs.contains(&label) {
                Some(label_close + 1)
            } else {
                None // unresolved: literal + resume after '['
            }
        }
        _ => None, // shortcut form is not a link (D8)
    }
}

/// Walk links, collecting `(span, normalized label)` for reference links
/// and `(span, None)` for inline links. Code spans are skipped first
/// (their brackets are literal). Unresolved reference candidates fall
/// back to literal and do NOT count.
fn walk_links(src: &[u8], fences: &[FenceRegion]) -> Vec<((usize, usize), Option<String>)> {
    let defs = definition_table(src);
    let cover = fence_cover(fences);
    let mut out = Vec::new();
    let mut i = 0;
    while i < src.len() {
        i = skip_covered(&cover, i);
        if i >= src.len() {
            break;
        }
        match src[i] {
            b'`' => {
                let run = run_len(src, i, b'`');
                i = match find_run_exact(src, i + run, run) {
                    Some(c) => c + run,
                    None => i + run,
                };
            }
            b'[' => match link_candidate(src, i, &defs) {
                Some(end) => {
                    // recover the label for reference forms
                    let text_close = src[i + 1..].iter().position(|&b| b == b']').unwrap() + i + 1;
                    let label = if src.get(text_close + 1) == Some(&b'[') {
                        let lc = src[text_close + 2..]
                            .iter()
                            .position(|&b| b == b']')
                            .map_or(text_close + 1, |p| p + text_close + 2);
                        let bytes = &src[text_close + 2..lc];
                        Some(if bytes.is_empty() {
                            norm(&src[i + 1..text_close])
                        } else {
                            norm(bytes)
                        })
                    } else {
                        None
                    };
                    out.push(((i, end), label));
                    i = end;
                }
                None => i += 1,
            },
            _ => i += 1,
        }
    }
    out
}

/// Link constructs `(start, end)` in document order.
fn link_spans(src: &[u8], fences: &[FenceRegion]) -> Vec<(usize, usize)> {
    walk_links(src, fences)
        .into_iter()
        .map(|(s, _)| s)
        .collect()
}

/// Emphasis spans `(open, close_end)` under the frozen single-'*' LIFO
/// walk (close-preferred, empty-content adjacency ban), skipping code
/// spans. Mirrors the H0 inline pass's delimiter decisions.
fn emphasis_spans(src: &[u8], fences: &[FenceRegion]) -> Vec<(usize, usize)> {
    let cover = fence_cover(fences);
    let mut out = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < src.len() {
        i = skip_covered(&cover, i);
        if i >= src.len() {
            break;
        }
        match src[i] {
            b'`' => {
                let run = run_len(src, i, b'`');
                i = match find_run_exact(src, i + run, run) {
                    Some(c) => c + run,
                    None => i + run,
                };
            }
            b'*' => {
                let prev_space = i == 0 || src[i - 1] == b' ' || src[i - 1] == b'\n';
                let can_open = i + 1 < src.len() && src[i + 1] != b' ' && src[i + 1] != b'\n';
                let top_adjacent = stack.last().is_some_and(|&p| p + 1 == i);
                let closable = !prev_space
                    && match stack.len() {
                        0 => false,
                        1 => !top_adjacent,
                        _ => true,
                    };
                if closable {
                    if top_adjacent {
                        stack.pop();
                    }
                    let opener = stack.pop().expect("closable implies opener");
                    out.push((opener, i + 1));
                    i += 1;
                } else if can_open {
                    stack.push(i);
                    i += 1;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// Code spans `(open, close_end)`; `open` is the first byte of the
/// opening run.
fn code_spans(src: &[u8], fences: &[FenceRegion]) -> Vec<(usize, usize)> {
    let cover = fence_cover(fences);
    let mut out = Vec::new();
    let mut i = 0;
    while i < src.len() {
        i = skip_covered(&cover, i);
        if i >= src.len() {
            break;
        }
        if src[i] == b'`' {
            let run = run_len(src, i, b'`');
            if let Some(c) = find_run_exact(src, i + run, run) {
                out.push((i, c + run));
                i = c + run;
            } else {
                i += run;
            }
        } else {
            i += 1;
        }
    }
    out
}

/// M-CS-ITEM-INDENT simulator: walk list-marker lines with the §7
/// (marker_indent, content_indent) frame stack; every non-marker line in
/// CORPUS-v1 sits at indent 0 and therefore closes every open frame
/// (blank lines, quote lines, headings, refdefs, fences, paragraph
/// lines). Raw fence body lines are skipped entirely. Returns the FIRST
/// marker line at/after `anchor` that continues an existing list as a
/// sibling, as `(line_start, marker column)`.
fn first_sibling_item_at_or_after(
    src: &[u8],
    fences: &[FenceRegion],
    anchor: usize,
) -> Option<(usize, usize)> {
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut skip_until: Option<usize> = None;
    for (ls, lf) in line_bounds(src) {
        if let Some(end) = skip_until {
            if ls < end {
                continue; // raw fence body: no list semantics
            }
            skip_until = None;
        }
        if let Some(r) = fences.iter().find(|r| r.opener_start == ls) {
            stack.clear();
            skip_until = Some(r.body.1);
            continue;
        }
        if fences
            .iter()
            .any(|r| r.closer.is_some_and(|(cs, _)| cs == ls))
        {
            stack.clear();
            continue;
        }
        let content = &src[ls..lf];
        let indent = content
            .iter()
            .position(|&b| b != b' ')
            .unwrap_or(content.len());
        let rest = &content[indent.min(content.len())..];
        let is_marker = (rest.first() == Some(&b'-') || rest.first() == Some(&b'*'))
            && rest.get(1) == Some(&b' ');
        if !is_marker {
            stack.clear();
            continue;
        }
        // §7 sibling walk (mirrors scripts/verify_r3.py check_deep_depths)
        let mut placed = false;
        while indent < stack.last().map_or(0, |&(_, c)| c) {
            let cand = stack.pop().expect("nonempty by loop condition");
            if indent == cand.0 {
                stack.push((cand.0, indent + 2));
                placed = true;
                break;
            }
        }
        if placed {
            if ls >= anchor {
                return Some((ls, indent));
            }
        } else {
            stack.push((indent, indent + 2));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Structural recipes (MUTATION-v1 §5)
// ---------------------------------------------------------------------------

/// Instantiate one structural recipe on `src` with the recipe's frozen
/// selection anchor. `Err(NotApplicable)` records a failed precondition.
pub fn structural_edit(
    src: &[u8],
    recipe: Recipe,
    anchor: AnchorClass,
) -> Result<PlannedEdit, NotApplicable> {
    let n = src.len();
    let a = snap_down(src, generic_anchor(anchor, n));
    let fences = fence_regions(src);
    let recipe_id = recipe.id();
    let na = |reason: String| NotApplicable {
        recipe: recipe_id,
        reason,
    };
    let edit = |start: usize, end: usize, inserted: Vec<u8>| PlannedEdit {
        start,
        end,
        inserted,
        anchor_class: anchor,
        anchor_offset: a,
    };

    match recipe {
        Recipe::LocText => {
            let runs: Vec<(usize, usize)> = text_letter_runs(src, &fences)
                .into_iter()
                .filter(|&(s, e)| e - s >= 32)
                .collect();
            let Some((rs, re)) = nearest(runs, a, |&(s, _)| s) else {
                return Err(na(
                    "no run of >= 32 ASCII letters outside fences/refdefs/headings".into(),
                ));
            };
            let start = snap_down(src, rs + (re - rs) / 2 - 16);
            Ok(edit(start, start + 32, vec![b'z'; 32]))
        }
        Recipe::LocUtf8Swap => {
            let positions = cjk_positions(src);
            let Some(p) = nearest(positions, a, |&p| p) else {
                return Err(na("no CJK scalar in corpus".into()));
            };
            Ok(edit(p, p + 3, b"zzz".to_vec()))
        }
        Recipe::BbParaSplit => {
            let run = nearest(
                text_letter_runs(src, &fences)
                    .into_iter()
                    .filter(|&(s, e)| e - s >= 4),
                a,
                |&(s, _)| s,
            );
            let Some((rs, re)) = run else {
                return Err(na(
                    "no paragraph text run of >= 4 letters near the anchor".into()
                ));
            };
            let mid = rs + (re - rs) / 2;
            Ok(edit(mid, mid, b"\n\n".to_vec()))
        }
        Recipe::BbParaMerge => {
            let seps = blank_separators(src, &fences);
            let Some(i) = nearest(seps, a, |&i| i) else {
                return Err(na(
                    "no two adjacent paragraphs separated by exactly one blank line".into(),
                ));
            };
            Ok(edit(i, i + 2, Vec::new()))
        }
        Recipe::CsItemIndent => {
            let Some((ls, marker_col)) = first_sibling_item_at_or_after(src, &fences, a) else {
                return Err(na(
                    "no list item with a previous sibling at the same indent \
                     at/after the anchor"
                        .into(),
                ));
            };
            Ok(edit(ls + marker_col, ls + marker_col, b"  ".to_vec()))
        }
        Recipe::CsBqNestLine => {
            // "the first such line at/after the anchor" — source order;
            // the line containing the anchor counts as "at".
            let hit = line_bounds(src)
                .into_iter()
                .filter(|&(ls, lf)| lf >= a && !body_of(&fences, ls))
                .find(|&(ls, lf)| {
                    let content = &src[ls..lf];
                    content.starts_with(b"> ") || content == b">"
                })
                .map(|(ls, _)| ls);
            let Some(ls) = hit else {
                return Err(na("no single-level blockquote line outside fence bodies \
                     at/after the anchor"
                    .into()));
            };
            Ok(edit(ls, ls, b">".to_vec()))
        }
        Recipe::FsFenceOpen => {
            // First non-blank line that is not part of any fence, at/after
            // the anchor (the containing line counts as "at").
            let hit = line_bounds(src)
                .into_iter()
                .filter(|&(ls, lf)| lf >= a && lf > ls && !fence_line(&fences, ls))
                .map(|(ls, _)| ls)
                .next();
            let Some(ls) = hit else {
                return Err(na(
                    "no non-blank line outside fence regions at/after the anchor".into(),
                ));
            };
            Ok(edit(ls, ls, b"```\n".to_vec()))
        }
        Recipe::FsFenceClose => {
            let region = fences.iter().find(|r| {
                r.closer.is_some()
                    && src[r.body.0..r.body.1]
                        .iter()
                        .filter(|&&b| b == b'\n')
                        .count()
                        >= 2
            });
            let Some(region) = region else {
                return Err(na(
                    "no fence with >= 2 body lines and a closing fence".into()
                ));
            };
            let body = &src[region.body.0..region.body.1];
            // interior LF: the last LF before the body's final byte (the
            // final byte is the LF ending the last body line)
            let interior = &body[..body.len() - 1];
            let body1_lf = interior
                .iter()
                .rposition(|&b| b == b'\n')
                .expect(">= 2 body lines implies an interior LF");
            let body2_start = region.body.0 + body1_lf + 1;
            let (cstart, cend) = region.closer.expect("checked above");
            let mut inserted = src[cstart..cend].to_vec();
            inserted.extend_from_slice(&src[body2_start..cstart]);
            Ok(edit(body2_start, cend, inserted))
        }
        Recipe::IdsEmphInsert => {
            let spans = emphasis_spans(src, &fences);
            let Some((_, close_end)) = nearest(spans, a, |&(open, _)| open) else {
                return Err(na("no emphasis span in corpus".into()));
            };
            Ok(edit(close_end - 1, close_end - 1, b"*".to_vec()))
        }
        Recipe::IdsCodeDelim => {
            let spans = code_spans(src, &fences);
            let Some((open, _)) = nearest(spans, a, |&(open, _)| open) else {
                return Err(na("no code span in corpus".into()));
            };
            Ok(edit(open, open + 1, Vec::new()))
        }
        Recipe::IdsLinkDelim => {
            let spans = link_spans(src, &fences);
            let Some((open, _)) = nearest(spans, a, |&(open, _)| open) else {
                return Err(na("no link construct in corpus".into()));
            };
            Ok(edit(open, open + 1, Vec::new()))
        }
        Recipe::SdDefReplace | Recipe::SdDefDelete => {
            let used: HashSet<String> = walk_links(src, &fences)
                .into_iter()
                .filter_map(|(_, label)| label)
                .collect();
            let defs: Vec<(usize, usize, (usize, usize))> = refdefs(src)
                .into_iter()
                .filter(|(_, _, label, _)| used.contains(label))
                .map(|(ls, le, _, dest)| (ls, le, dest))
                .collect();
            let Some((ls, le, dest)) = nearest(defs, a, |&(ls, _, _)| ls) else {
                return Err(na(
                    "no reference definition whose label is used by a link".into()
                ));
            };
            if recipe == Recipe::SdDefReplace {
                Ok(edit(dest.0, dest.1, vec![b'z'; dest.1 - dest.0]))
            } else {
                Ok(edit(ls, le, Vec::new()))
            }
        }
    }
}

/// `structural_edit` gated by the FROZEN case matrix: a combo the matrix
/// declares NOT_APPLICABLE is refused even if a byte scan could find a
/// candidate (the declared applicable lists and case counts are frozen
/// authority — e.g. reference_fanout carries 8-letter CJK-tail runs but is
/// declared outside M-BB-PARA-SPLIT's workload). This is the entry point
/// case instantiation must use.
pub fn structural_edit_for(
    src: &[u8],
    recipe: Recipe,
    anchor: AnchorClass,
    shape: PayloadShape,
    size: usize,
) -> Result<PlannedEdit, NotApplicable> {
    if !frozen_applicable(recipe, shape, size) {
        return Err(NotApplicable {
            recipe: recipe.id(),
            reason: "declared NOT_APPLICABLE by the frozen case matrix".into(),
        });
    }
    structural_edit(src, recipe, anchor)
}

// ---------------------------------------------------------------------------
// Application + frozen validity invariants (MUTATION-v1 §6 / CORPUS-v1 §6)
// ---------------------------------------------------------------------------

/// Host-side byte-exact application: `apply(old, edit) == post`.
pub fn apply(src: &[u8], edit: &PlannedEdit) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() - (edit.end - edit.start) + edit.inserted.len());
    out.extend_from_slice(&src[..edit.start]);
    out.extend_from_slice(&edit.inserted);
    out.extend_from_slice(&src[edit.end..]);
    out
}

/// Frozen mutation validity invariants (MUTATION-v1 §6). `post` must be
/// the byte-exact application of `edit` to `src`.
pub fn validate(src: &[u8], edit: &PlannedEdit, post: &[u8]) -> Result<(), String> {
    if edit.start > edit.end || edit.end > src.len() {
        return Err(format!(
            "range [{}, {}) out of order or beyond {}",
            edit.start,
            edit.end,
            src.len()
        ));
    }
    for pos in [edit.start, edit.end] {
        if !is_char_boundary(src, pos) {
            return Err(format!("boundary {pos} splits a UTF-8 scalar"));
        }
    }
    if edit.end == edit.start && edit.inserted.is_empty() {
        return Err("degenerate edit changes no byte".into());
    }
    if apply(src, edit) != post {
        return Err("apply(old, edit) != post".into());
    }
    if std::str::from_utf8(post).is_err() {
        return Err("post-edit source is not valid UTF-8".into());
    }
    Ok(())
}

/// The frozen applicability declarations (MUTATION-v1 §5 / CASE-MATRIX-v1
/// §5), as a pure table. Structural instantiation MUST agree with this.
pub fn frozen_applicable(recipe: Recipe, shape: PayloadShape, size: usize) -> bool {
    use PayloadShape::*;
    match recipe {
        Recipe::LocText => matches!(shape, Plain | HugeBlock | DeepContainer | Mixed),
        Recipe::LocUtf8Swap => !matches!(shape, Mixed),
        Recipe::BbParaSplit => {
            matches!(
                shape,
                Plain | ManyBlocks | HugeBlock | DeepContainer | Mixed
            )
        }
        Recipe::BbParaMerge => match shape {
            FenceHeavy | DeepContainer => false,
            HugeBlock => size != crate::SIZE_64K,
            Plain | ManyBlocks | InlineDense | ReferenceFanout | Mixed => true,
        },
        Recipe::CsItemIndent | Recipe::CsBqNestLine => matches!(shape, DeepContainer | Mixed),
        Recipe::FsFenceOpen => !matches!(shape, FenceHeavy),
        Recipe::FsFenceClose => matches!(shape, FenceHeavy | Mixed),
        Recipe::IdsEmphInsert | Recipe::IdsCodeDelim => matches!(shape, InlineDense | Mixed),
        Recipe::IdsLinkDelim => matches!(shape, InlineDense | ReferenceFanout | Mixed),
        Recipe::SdDefReplace | Recipe::SdDefDelete => {
            matches!(shape, ReferenceFanout | Mixed)
        }
    }
}

/// The 13 recipes in frozen order.
pub const ALL_RECIPES: [Recipe; 13] = [
    Recipe::LocText,
    Recipe::LocUtf8Swap,
    Recipe::BbParaSplit,
    Recipe::BbParaMerge,
    Recipe::CsItemIndent,
    Recipe::CsBqNestLine,
    Recipe::FsFenceOpen,
    Recipe::FsFenceClose,
    Recipe::IdsEmphInsert,
    Recipe::IdsCodeDelim,
    Recipe::IdsLinkDelim,
    Recipe::SdDefReplace,
    Recipe::SdDefDelete,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, size_label, ALL_SHAPES, SIZE_16M, SIZE_1M, SIZE_64K};

    fn gen(shape: PayloadShape, size: usize) -> Vec<u8> {
        generate(shape, size).expect("generate")
    }

    // ---- anchor machinery -------------------------------------------------

    #[test]
    fn raw_anchors_are_floor_percentiles() {
        let n = 1_000;
        assert_eq!(raw_anchor(AnchorClass::Early, n), 250);
        assert_eq!(raw_anchor(AnchorClass::Middle, n), 500);
        assert_eq!(raw_anchor(AnchorClass::Late, n), 750);
        assert_eq!(generic_anchor(AnchorClass::Early, n), 257);
    }

    #[test]
    fn snap_down_never_splits_a_scalar() {
        let src = b"ab\xE4\xB8\xADc"; // ab中c: boundaries 0,1,2,5,6
        assert_eq!(snap_down(src, 2), 2); // start of 中
        assert_eq!(snap_down(src, 3), 2); // continuation -> snap down
        assert_eq!(snap_down(src, 4), 2); // continuation -> snap down
        assert_eq!(snap_down(src, 5), 5); // after 中
    }

    #[test]
    fn query_anchors_are_snapped_generic_anchors() {
        let src = gen(PayloadShape::DeepContainer, SIZE_64K);
        // all three raw anchors are 65536k -> mountain 8k+0 offset 7, i.e.
        // inside 文 of line 0; each snaps down to that scalar's first byte
        assert_eq!(query_anchors(&src), [16_389, 32_773, 49_157]);
    }

    // ---- generic edits ----------------------------------------------------

    #[test]
    fn generic_edit_byte_classes_hold() {
        for shape in [
            PayloadShape::Plain,
            PayloadShape::FenceHeavy,
            PayloadShape::Mixed,
        ] {
            let src = gen(shape, SIZE_64K);
            for class in [AnchorClass::Early, AnchorClass::Middle, AnchorClass::Late] {
                for size in [EditSize::Tiny, EditSize::Small, EditSize::Medium] {
                    for op in [
                        GenericOp::Insert,
                        GenericOp::Delete,
                        GenericOp::ReplaceEq,
                        GenericOp::ReplaceGrow,
                        GenericOp::ReplaceShrink,
                    ] {
                        let e = generic_edit(&src, class, size, op);
                        let post = apply(&src, &e);
                        validate(&src, &e, &post).unwrap_or_else(|err| {
                            panic!("{shape:?} {class:?} {size:?} {op:?}: {err}")
                        });
                        let removed = e.end - e.start;
                        let added = e.inserted.len();
                        match op {
                            GenericOp::Insert => {
                                assert_eq!(removed, 0);
                                assert!(added > 0);
                            }
                            GenericOp::Delete => {
                                assert!(removed > 0);
                                assert_eq!(added, 0);
                            }
                            GenericOp::ReplaceEq => assert_eq!(removed, added),
                            GenericOp::ReplaceGrow => assert!(added > removed),
                            GenericOp::ReplaceShrink => assert!(added < removed),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn medium_window_cap_fires_on_huge_block() {
        let src = gen(PayloadShape::HugeBlock, SIZE_64K);
        let e = generic_edit(
            &src,
            AnchorClass::Middle,
            EditSize::Medium,
            GenericOp::Delete,
        );
        // the giant line (65534 B) exceeds 4096 -> 512-byte window
        assert_eq!(e.anchor_offset, 32_775);
        assert_eq!(e.start, 32_775);
        assert_eq!(e.end, 32_775 + 512);
        let eq = generic_edit(
            &src,
            AnchorClass::Middle,
            EditSize::Medium,
            GenericOp::ReplaceEq,
        );
        assert_eq!(eq.inserted.len(), 512);
        assert_eq!(eq.end - eq.start, 512);
    }

    #[test]
    fn medium_line_forms_on_plain() {
        let src = gen(PayloadShape::Plain, SIZE_64K);
        let e = generic_edit(
            &src,
            AnchorClass::Early,
            EditSize::Medium,
            GenericOp::ReplaceEq,
        );
        // line containing 16391: [16385, 16448) incl LF (62 content + LF)
        assert_eq!(e.start, 16_385);
        assert_eq!(e.end, 16_448);
        assert_eq!(e.inserted.len(), 63);
        assert_eq!(e.inserted[62], b'\n');
        let del = generic_edit(
            &src,
            AnchorClass::Early,
            EditSize::Medium,
            GenericOp::Delete,
        );
        assert_eq!(del.start, 16_385);
        assert_eq!(del.end, 16_448);
        let ins = generic_edit(
            &src,
            AnchorClass::Early,
            EditSize::Medium,
            GenericOp::Insert,
        );
        assert_eq!(ins.start, 16_385); // START of the anchor's line
        assert_eq!(ins.inserted.len(), 24);
    }

    // ---- structural recipe applicability matrix ---------------------------

    #[test]
    fn structural_applicability_matches_the_frozen_matrix() {
        for shape in ALL_SHAPES {
            for size in [SIZE_64K, SIZE_1M] {
                let src = gen(shape, size);
                for recipe in ALL_RECIPES {
                    let want = frozen_applicable(recipe, shape, size);
                    // Every DECLARED-applicable combo must instantiate by
                    // scan (the frozen case counts depend on it).
                    if want {
                        structural_edit(&src, recipe, recipe.selection_anchor()).unwrap_or_else(
                            |na| {
                                panic!(
                                    "{} on {}-{}: declared applicable but scan refuses: {na:?}",
                                    recipe.id(),
                                    shape.canonical_name(),
                                    size_label(size)
                                )
                            },
                        );
                    }
                    // The gated entry point honors the matrix both ways.
                    assert_eq!(
                        structural_edit_for(&src, recipe, recipe.selection_anchor(), shape, size)
                            .is_ok(),
                        want,
                        "{} on {}-{}: gated instantiation disagrees with the matrix",
                        recipe.id(),
                        shape.canonical_name(),
                        size_label(size)
                    );
                }
            }
        }
    }

    #[test]
    fn huge_block_64k_para_merge_is_not_applicable_but_1m_is() {
        let small = gen(PayloadShape::HugeBlock, SIZE_64K);
        assert!(structural_edit(&small, Recipe::BbParaMerge, AnchorClass::Middle).is_err());
        let big = gen(PayloadShape::HugeBlock, SIZE_1M);
        assert!(structural_edit(&big, Recipe::BbParaMerge, AnchorClass::Middle).is_ok());
    }

    // ---- structural edits: validity + determinism -------------------------

    #[test]
    fn structural_edits_are_valid_deterministic_and_apply() {
        for shape in ALL_SHAPES {
            let src = gen(shape, SIZE_64K);
            for recipe in ALL_RECIPES {
                let first = structural_edit(&src, recipe, recipe.selection_anchor());
                let second = structural_edit(&src, recipe, recipe.selection_anchor());
                assert_eq!(
                    first,
                    second,
                    "{}: nondeterministic on {shape:?}",
                    recipe.id()
                );
                if let Ok(e) = first {
                    let post = apply(&src, &e);
                    validate(&src, &e, &post)
                        .unwrap_or_else(|err| panic!("{} on {shape:?}: {err}", recipe.id()));
                }
            }
        }
    }

    #[test]
    fn fence_open_has_both_early_and_middle_variants_on_seven_shapes() {
        // M-FS-FENCE-OPEN: EARLY and MIDDLE are separate case variants
        // (7 shapes x 3 sizes x 2 anchors = 42 frozen cases).
        for shape in ALL_SHAPES {
            let src = gen(shape, SIZE_64K);
            for anchor in [AnchorClass::Early, AnchorClass::Middle] {
                let got = structural_edit(&src, Recipe::FsFenceOpen, anchor);
                assert_eq!(
                    got.is_ok(),
                    !matches!(shape, PayloadShape::FenceHeavy),
                    "FsFenceOpen {anchor:?} on {shape:?}"
                );
            }
        }
    }

    // ---- frozen landing properties (MUTATION-v1 §2, formula-derived) ------

    #[test]
    fn generic_landings_match_the_formula_table() {
        // plain: inside the anchored CJK variant unit's filler bytes
        let plain = gen(PayloadShape::Plain, SIZE_64K);
        let a = snap_down(&plain, generic_anchor(AnchorClass::Early, plain.len()));
        assert_eq!(a, 16_391); // unit 256 (CJK), filler byte 0
        assert!(plain[a].is_ascii_lowercase());

        // many_blocks: inside the anchored CJK variant line's filler
        let mb = gen(PayloadShape::ManyBlocks, SIZE_64K);
        let a = snap_down(&mb, generic_anchor(AnchorClass::Early, mb.len()));
        assert_eq!(a, 16_391); // unit 1024 (CJK), filler byte 1
        assert!(mb[a].is_ascii_lowercase());

        // huge_block: interior of the giant line
        let hb = gen(PayloadShape::HugeBlock, SIZE_64K);
        let a = snap_down(&hb, generic_anchor(AnchorClass::Early, hb.len()));
        assert_eq!(a, 16_391);
        assert!(hb[a].is_ascii_lowercase());

        // deep_container: line 0 of an even (list) mountain, content
        // region; byte 7 falls inside 文 -> down-snap to its first byte.
        let dc = gen(PayloadShape::DeepContainer, SIZE_64K);
        let raw = generic_anchor(AnchorClass::Early, dc.len());
        assert_eq!(raw, 16_391);
        assert_eq!(raw % 2048, 7);
        assert_eq!(dc[16_384], b'-'); // list mountain line 0
        let a = snap_down(&dc, raw);
        assert_eq!(a, 16_389); // first byte of the second CJK scalar

        // fence_heavy: inside body1 of the anchored ASCII fence unit
        let fh = gen(PayloadShape::FenceHeavy, SIZE_64K);
        let a = snap_down(&fh, generic_anchor(AnchorClass::Early, fh.len()));
        assert_eq!(a, 16_391); // unit 32 (even -> ASCII), body1 byte 2
        assert!(fh[a].is_ascii_lowercase());

        // mixed: inside the tile heading's text
        let mx = gen(PayloadShape::Mixed, SIZE_64K);
        let a = snap_down(&mx, generic_anchor(AnchorClass::Early, mx.len()));
        assert_eq!(a, 16_391); // tile 4, offset 7
        assert_eq!(&mx[16_384..16_394], b"## section");

        // reference_fanout: offset 7 of an ASCII link unit (formula
        // landing; the §2 prose "CJK variant line" drift is recorded in
        // the R4 stage record — the formula is the authority).
        let rf = gen(PayloadShape::ReferenceFanout, SIZE_64K);
        let a = snap_down(&rf, generic_anchor(AnchorClass::Early, rf.len()));
        assert_eq!(a, 16_391); // unit 248 (8 mod 16 -> ASCII), label '['
        assert_eq!(rf[a], b'[');
    }

    // ---- recipe behavior spot checks --------------------------------------

    #[test]
    fn para_merge_deletes_the_blank_line_bytes() {
        let src = gen(PayloadShape::Plain, SIZE_64K);
        let e = structural_edit(&src, Recipe::BbParaMerge, AnchorClass::Middle).unwrap();
        assert_eq!(e.end - e.start, 2);
        assert_eq!(&src[e.start..e.end], b"\n\n");
        assert!(src[e.start - 1].is_ascii_lowercase());
        assert_ne!(src[e.start + 2], b'\n');
    }

    #[test]
    fn para_split_inserts_two_lfs_at_run_midpoint() {
        let src = gen(PayloadShape::Plain, SIZE_64K);
        let e = structural_edit(&src, Recipe::BbParaSplit, AnchorClass::Middle).unwrap();
        assert_eq!(e.inserted, b"\n\n");
        // nearest run starts AT the MIDDLE anchor (filler of the anchored
        // CJK unit at 32775); midpoint = run_start + 56/2
        assert_eq!(e.start, 32_775 + 28);
    }

    #[test]
    fn cs_item_indent_nests_the_beta_item_in_mixed() {
        let src = gen(PayloadShape::Mixed, SIZE_64K);
        let e = structural_edit(&src, Recipe::CsItemIndent, AnchorClass::Middle).unwrap();
        assert_eq!(e.inserted, b"  ");
        let ls = src[..e.start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |p| p + 1);
        // the marker line is "- item beta" of the anchor's tile (alpha has
        // no previous sibling, so it fails the precondition)
        assert_eq!(&src[ls..ls + 11], b"- item beta");
    }

    #[test]
    fn cs_bq_nest_targets_a_single_level_quote() {
        let src = gen(PayloadShape::Mixed, SIZE_64K);
        let e = structural_edit(&src, Recipe::CsBqNestLine, AnchorClass::Middle).unwrap();
        assert_eq!(e.inserted, b">");
        let ls = src[..e.start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |p| p + 1);
        assert_eq!(&src[ls..ls + 13], b"> quoted aaaa");
        let post = apply(&src, &e);
        assert_eq!(&post[ls..ls + 14], b">> quoted aaaa");
    }

    #[test]
    fn fs_fence_close_moves_the_closer_in_front_of_body2() {
        let src = gen(PayloadShape::FenceHeavy, SIZE_64K);
        let e = structural_edit(&src, Recipe::FsFenceClose, AnchorClass::Middle).unwrap();
        // first fence = unit 0: [256, 511) -> "```\n" + body2(251)
        assert_eq!(e.start, 256);
        assert_eq!(e.end, 511);
        assert_eq!(e.inserted.len(), 255);
        assert_eq!(&e.inserted[..4], b"```\n");
        assert_eq!(&e.inserted[4..6], b"ab");
        let post = apply(&src, &e);
        let text = std::str::from_utf8(&post[..512]).unwrap();
        let lines: Vec<&str> = text.split('\n').collect();
        assert_eq!(lines[0], "```x");
        assert_eq!(lines[1].len(), 250);
        assert_eq!(lines[2], "```"); // closer moved up
        assert_eq!(lines[3].len(), 250); // released body2
    }

    #[test]
    fn ids_emph_insert_lands_before_the_closing_delimiter() {
        let src = gen(PayloadShape::InlineDense, SIZE_64K);
        let e = structural_edit(&src, Recipe::IdsEmphInsert, AnchorClass::Middle).unwrap();
        assert_eq!(e.inserted, b"*");
        // nearest span to the anchor is *bb* of the anchored CJK unit:
        // insert position e.start sits immediately before its closer
        assert_eq!(src[e.start], b'*'); // the original closing delimiter
        assert_eq!(src[e.start - 1], b'b');
        let post = apply(&src, &e);
        assert_eq!(post[e.start], b'*'); // inserted
        assert_eq!(post[e.start + 1], b'*'); // original shifted right
    }

    #[test]
    fn ids_code_delim_deletes_the_first_opening_backtick() {
        let src = gen(PayloadShape::InlineDense, SIZE_64K);
        let e = structural_edit(&src, Recipe::IdsCodeDelim, AnchorClass::Middle).unwrap();
        assert_eq!(e.end - e.start, 1);
        assert_eq!(src[e.start], b'`');
    }

    #[test]
    fn ids_link_delim_deletes_the_opening_bracket() {
        for shape in [PayloadShape::InlineDense, PayloadShape::ReferenceFanout] {
            let src = gen(shape, SIZE_64K);
            let e = structural_edit(&src, Recipe::IdsLinkDelim, AnchorClass::Middle).unwrap();
            assert_eq!(src[e.start], b'[', "{shape:?}");
            assert_eq!(e.end - e.start, 1);
        }
    }

    #[test]
    fn sd_def_replace_rewrites_the_destination_bytes() {
        let src = gen(PayloadShape::ReferenceFanout, SIZE_64K);
        let e = structural_edit(&src, Recipe::SdDefReplace, AnchorClass::Early).unwrap();
        // §2.1 min-distance fallback: no defs after EARLY -> nearest overall
        // is the last header def (r15) at [480, 512); dest at [487, 511).
        assert_eq!(e.start, 487);
        assert_eq!(e.end, 511);
        assert_eq!(e.inserted, vec![b'z'; 24]);
        assert_eq!(&src[480..487], b"[r15]: ");
    }

    #[test]
    fn sd_def_delete_removes_the_whole_definition_line() {
        let src = gen(PayloadShape::ReferenceFanout, SIZE_64K);
        let e = structural_edit(&src, Recipe::SdDefDelete, AnchorClass::Early).unwrap();
        assert_eq!(e.start, 480);
        assert_eq!(e.end, 512);
        assert!(e.inserted.is_empty());
        let post = apply(&src, &e);
        assert_eq!(post.len(), SIZE_64K - 32);
        assert!(post.starts_with(b"[r00]: /"));
    }

    #[test]
    fn mixed_sd_defs_select_the_anchor_tile_header() {
        let src = gen(PayloadShape::Mixed, SIZE_64K);
        let e = structural_edit(&src, Recipe::SdDefReplace, AnchorClass::Early).unwrap();
        // EARLY 16391 -> tile 4 header: "[rx0]: /" at [16396, 16404),
        // destination bytes [16404, 16428).
        assert_eq!(e.start, 16_403); // the '/' byte
        assert_eq!(e.end, 16_427);
        assert_eq!(e.inserted, vec![b'z'; 24]);
        assert_eq!(&src[16_396..16_404], b"[rx0]: /");
    }

    #[test]
    fn loc_text_replaces_interior_run_bytes() {
        let src = gen(PayloadShape::Plain, SIZE_64K);
        let e = structural_edit(&src, Recipe::LocText, AnchorClass::Middle).unwrap();
        assert_eq!(e.inserted, vec![b'z'; 32]);
        // the selected run contains the edit window as an interior slice
        let mut rs = e.start;
        while src[rs - 1].is_ascii_lowercase() {
            rs -= 1;
        }
        let mut re = e.start + 32;
        while src[re].is_ascii_lowercase() {
            re += 1;
        }
        assert!(re - rs >= 32);
        assert_eq!(e.start + 16, rs + (re - rs) / 2); // midpoint placement
    }

    #[test]
    fn loc_utf8_swap_replaces_exactly_three_bytes_of_one_scalar() {
        let src = gen(PayloadShape::HugeBlock, SIZE_1M);
        let e = structural_edit(&src, Recipe::LocUtf8Swap, AnchorClass::Middle).unwrap();
        assert_eq!(e.inserted, b"zzz");
        assert_eq!(&src[e.start..e.end], "文".as_bytes());
    }

    #[test]
    fn fence_open_prepends_an_opener_to_the_containing_line() {
        let src = gen(PayloadShape::Plain, SIZE_64K);
        let e = structural_edit(&src, Recipe::FsFenceOpen, AnchorClass::Early).unwrap();
        assert_eq!(e.inserted, b"```\n");
        let ls = src[..e.start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |p| p + 1);
        assert!(ls <= 16_391);
        // the anchor's line is the CJK filler line of unit 256
        assert_eq!(&src[ls..ls + 3], "中".as_bytes());
        let post = apply(&src, &e);
        assert_eq!(&post[e.start..e.start + 4], b"```\n");
    }

    #[test]
    fn cs_item_indent_selects_the_ladder_sibling_on_deep() {
        let src = gen(PayloadShape::DeepContainer, SIZE_64K);
        let e = structural_edit(&src, Recipe::CsItemIndent, AnchorClass::Middle).unwrap();
        // anchor at mountain 8 line 0; the first sibling line is line
        // index 16 (the descending d=15 step, indent 28, re-opens the
        // d=15 frame opened at line index 14)
        let mountain = (e.start / 2048) * 2048;
        assert_eq!(e.start, mountain + 16 * 64 + 28);
        assert_eq!(e.inserted, b"  ");
    }

    #[test]
    fn all_16m_applicable_recipes_instantiate_and_validate() {
        // Scaling sanity at the top size: scans are linear and recipes
        // anchor locally, so this stays fast.
        for shape in [PayloadShape::HugeBlock, PayloadShape::ReferenceFanout] {
            let src = gen(shape, SIZE_16M);
            for recipe in [
                Recipe::LocUtf8Swap,
                Recipe::SdDefReplace,
                Recipe::FsFenceOpen,
            ] {
                if !frozen_applicable(recipe, shape, SIZE_16M) {
                    continue;
                }
                let e = structural_edit(&src, recipe, recipe.selection_anchor())
                    .unwrap_or_else(|na| panic!("{} on {shape:?} 16m: {na:?}", recipe.id()));
                let post = apply(&src, &e);
                validate(&src, &e, &post).unwrap();
            }
        }
    }
}
