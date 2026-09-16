//! Survey-side line scanning and coarse syntax roles.
//!
//! This is NOT the product parser. It exists only so mutation builders can
//! find anchors ("a paragraph line near 50% of the document"). The roles
//! here intentionally mirror the L1 block vocabulary loosely; any
//! disagreement with the real parser shows up as a slightly different
//! anchor, never as a measurement.

use markit_core::{ByteOffset, SourceRange, TextEdit};

#[derive(Clone, Copy, Debug)]
pub struct LineInfo {
    /// First byte of the line.
    pub start: usize,
    /// One past the last content byte (excludes the terminator).
    pub content_end: usize,
    /// One past the terminator (`\n` or `\r\n`).
    pub end: usize,
}

/// Coarse per-line role for anchor selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Blank,
    Heading,
    Para,
    ListItem,
    Quote,
    /// Fence opener; `info` is the absolute byte span of the info string.
    FenceOpen { info: (usize, usize) },
    /// Fence body; `mermaid` marks bodies under a ```mermaid opener.
    FenceBody { mermaid: bool },
    FenceClose,
}

pub struct SurveyText {
    pub text: String,
    pub lines: Vec<LineInfo>,
    pub roles: Vec<Role>,
}

impl SurveyText {
    pub fn new(text: String) -> Self {
        let lines = scan_lines(&text);
        let roles = classify(&text, &lines);
        Self { text, lines, roles }
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// The line nearest byte position `frac * len` whose role satisfies
    /// `pred`.
    pub fn pick<F>(&self, frac: f64, pred: F) -> Option<usize>
    where
        F: Fn(&Role) -> bool,
    {
        let target = (frac.clamp(0.0, 1.0) * self.len() as f64) as usize;
        let mut best: Option<usize> = None;
        let mut best_d = usize::MAX;
        for (i, l) in self.lines.iter().enumerate() {
            if !pred(&self.roles[i]) {
                continue;
            }
            let d = l.start.abs_diff(target);
            if d < best_d {
                best_d = d;
                best = Some(i);
            }
        }
        best
    }

    /// Like [`SurveyText::pick`], but the predicate sees the line index
    /// (for neighbor-sensitive anchors such as "paragraph after a list").
    pub fn pick_index<F>(&self, frac: f64, pred: F) -> Option<usize>
    where
        F: Fn(usize) -> bool,
    {
        let target = (frac.clamp(0.0, 1.0) * self.len() as f64) as usize;
        let mut best: Option<usize> = None;
        let mut best_d = usize::MAX;
        for i in 0..self.lines.len() {
            if !pred(i) {
                continue;
            }
            let d = self.lines[i].start.abs_diff(target);
            if d < best_d {
                best_d = d;
                best = Some(i);
            }
        }
        best
    }

    pub fn line_content(&self, i: usize) -> &str {
        &self.text[self.lines[i].start..self.lines[i].content_end]
    }
}

fn scan_lines(text: &str) -> Vec<LineInfo> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let content_end = if i > start && bytes[i - 1] == b'\r' {
                i - 1
            } else {
                i
            };
            out.push(LineInfo {
                start,
                content_end,
                end: i + 1,
            });
            start = i + 1;
        }
        i += 1;
    }
    if start < bytes.len() || out.is_empty() {
        out.push(LineInfo {
            start,
            content_end: bytes.len(),
            end: bytes.len(),
        });
    }
    out
}

fn classify(text: &str, lines: &[LineInfo]) -> Vec<Role> {
    let mut roles = Vec::with_capacity(lines.len());
    let mut fence: Option<(u8, usize)> = None;
    let mut open_mermaid = false;
    for l in lines {
        let content = &text[l.start..l.content_end];
        let trimmed = content.trim_start();
        let indent = content.len() - trimmed.len();
        if let Some((marker, min_len)) = fence {
            let closes =
                !trimmed.is_empty() && trimmed.bytes().all(|b| b == marker) && trimmed.len() >= min_len;
            if closes {
                roles.push(Role::FenceClose);
                fence = None;
                open_mermaid = false;
            } else {
                roles.push(Role::FenceBody {
                    mermaid: open_mermaid,
                });
            }
            continue;
        }
        let fb = trimmed.as_bytes();
        if indent <= 3
            && fb.len() >= 3
            && (fb[0] == b'`' || fb[0] == b'~')
            && fb.iter().take_while(|&&b| b == fb[0]).count() >= 3
        {
            let marker = fb[0];
            let run = fb.iter().take_while(|&&b| b == marker).count();
            let info = (l.start + indent + run, l.content_end);
            let is_mermaid = text[info.0..info.1].trim() == "mermaid";
            roles.push(Role::FenceOpen { info });
            fence = Some((marker, run));
            open_mermaid = is_mermaid;
            continue;
        }
        if trimmed.is_empty() {
            roles.push(Role::Blank);
        } else if trimmed.starts_with('#') && indent <= 3 {
            roles.push(Role::Heading);
        } else if trimmed.starts_with('>') {
            roles.push(Role::Quote);
        } else if is_list_marker(trimmed) {
            roles.push(Role::ListItem);
        } else {
            roles.push(Role::Para);
        }
    }
    roles
}

fn is_list_marker(trimmed: &str) -> bool {
    let b = trimmed.as_bytes();
    if b.is_empty() {
        return false;
    }
    if matches!(b[0], b'-' | b'+' | b'*') {
        return b.len() == 1 || b[1] == b' ' || b[1] == b'\t';
    }
    let digits = b.iter().take_while(|x| x.is_ascii_digit()).count();
    if (1..=9).contains(&digits) && digits < b.len() && matches!(b[digits], b'.' | b')') {
        let after = digits + 1;
        return after == b.len() || b[after] == b' ' || b[after] == b'\t';
    }
    false
}

/// First ASCII alphanumeric byte at or after `from` within `[from, until)`.
pub fn ascii_byte_at(text: &str, from: usize, until: usize) -> Option<usize> {
    text.as_bytes()[from..until.max(from)]
        .iter()
        .position(|b| b.is_ascii_alphanumeric())
        .map(|i| from + i)
}

pub(crate) fn ins(at: usize, t: &str) -> Option<TextEdit> {
    Some(TextEdit::insert(ByteOffset(at), t.to_string()))
}

pub(crate) fn del(a: usize, b: usize) -> Option<TextEdit> {
    Some(TextEdit::delete(SourceRange::new(ByteOffset(a), ByteOffset(b))))
}

pub(crate) fn rep(a: usize, b: usize, t: &str) -> Option<TextEdit> {
    Some(TextEdit::replace(
        SourceRange::new(ByteOffset(a), ByteOffset(b)),
        t.to_string(),
    ))
}
