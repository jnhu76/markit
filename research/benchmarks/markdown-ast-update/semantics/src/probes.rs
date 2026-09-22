//! Frozen lexical probes for constructs a lane does not recognize.
//!
//! Authority: `workloads/profiles/PROFILER-CONTRACT-v1.md` §"Candidate
//! probes". A probe is **evidence that bytes look like a construct**, never
//! a parse fact, and never a substitute for the lane oracle. Probes exist
//! only so that "the lane did not recognize it" can be reported as
//! candidate evidence instead of disappearing as "absent".
//!
//! Two rules bound every probe:
//!
//! 1. **Host-context rule**: candidates are resolved against the lane
//!    parse's non-host intervals; a candidate inside a fence body, code
//!    span, indented code block or raw HTML region is emitted with
//!    `host_context = false` and can never be counted as host syntax.
//! 2. **Probe priority**: probes run in a frozen priority order and an
//!    already-claimed byte range suppresses later candidates, so the same
//!    bytes cannot be reported twice under two constructs.
//!
//! Extending this probe set changes the profiler contract version.

use crate::facts::{Span, SyntaxKind};

/// Frozen probe priority order (earlier wins an overlapping range).
pub const PROBE_PRIORITY: &[SyntaxKind] = &[
    SyntaxKind::FrontMatter,
    SyntaxKind::DisplayMath,
    SyntaxKind::Directive,
    SyntaxKind::Table,
    SyntaxKind::HtmlBlock,
    SyntaxKind::LinkAutolink,
    SyntaxKind::InlineMath,
    SyntaxKind::RawHtmlInline,
    SyntaxKind::HeadingSetext,
    SyntaxKind::ThematicBreak,
    SyntaxKind::Strong,
    SyntaxKind::Image,
    SyntaxKind::CodeBlockIndented,
    SyntaxKind::CodeBlockFenced,
    SyntaxKind::Strikethrough,
    SyntaxKind::TaskListItem,
];

/// One candidate found by a probe (before host-context resolution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCandidate {
    pub kind: SyntaxKind,
    pub span: Span,
    /// Whether the probe found a *complete* construct (`true`) or only an
    /// opening/partial shape whose meaning cannot be settled (`false`).
    /// A partial candidate is reported `unknown`, never classified.
    pub complete: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct Line {
    /// First byte of the line.
    start: usize,
    /// Index just past the last content byte (excludes LF).
    content_end: usize,
}

impl Line {
    fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.content_end]
    }
}

fn lines(source: &str) -> Vec<Line> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            out.push(Line {
                start,
                content_end: index,
            });
            start = index + 1;
        }
    }
    if start < bytes.len() {
        out.push(Line {
            start,
            content_end: bytes.len(),
        });
    }
    out
}

/// Run every probe and resolve overlaps by [`PROBE_PRIORITY`].
pub fn probe_candidates(source: &str) -> Vec<RawCandidate> {
    let lines = lines(source);
    let mut all = Vec::new();
    for kind in PROBE_PRIORITY {
        let mut found = match kind {
            SyntaxKind::FrontMatter => probe_front_matter(source, &lines),
            SyntaxKind::DisplayMath => probe_display_math(source),
            SyntaxKind::Directive => probe_directive(source, &lines),
            SyntaxKind::Table => probe_table(source, &lines),
            SyntaxKind::HtmlBlock => probe_html_block(source, &lines),
            SyntaxKind::LinkAutolink => probe_autolink(source),
            SyntaxKind::InlineMath => probe_inline_math(source),
            SyntaxKind::RawHtmlInline => probe_raw_html_inline(source),
            SyntaxKind::HeadingSetext => probe_setext_underline(source, &lines),
            SyntaxKind::ThematicBreak => probe_thematic_break(source, &lines),
            SyntaxKind::Strong => probe_strong(source),
            SyntaxKind::Image => probe_image(source),
            SyntaxKind::CodeBlockIndented => probe_indented_code(source, &lines),
            SyntaxKind::CodeBlockFenced => probe_tilde_fence(source, &lines),
            SyntaxKind::Strikethrough => probe_strikethrough(source),
            SyntaxKind::TaskListItem => probe_task_list_item(source, &lines),
            _ => Vec::new(),
        };
        all.append(&mut found);
    }

    // Overlap suppression in priority order: a candidate is kept only if it
    // does not intersect an already-kept candidate.
    let mut kept: Vec<RawCandidate> = Vec::new();
    for kind in PROBE_PRIORITY {
        for candidate in all.iter().filter(|candidate| candidate.kind == *kind) {
            if !kept
                .iter()
                .any(|existing| existing.span.intersects(&candidate.span))
            {
                kept.push(candidate.clone());
            }
        }
    }
    kept.sort_by_key(|candidate| (candidate.span.start, candidate.span.end));
    kept
}

// ---------------------------------------------------------------------------
// Block probes
// ---------------------------------------------------------------------------

/// Front matter: the first line is exactly `---` (or `+++`) and a later
/// line is exactly `---`.
fn probe_front_matter(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let Some(first) = lines.first() else {
        return Vec::new();
    };
    let opener = first.text(source).trim_end_matches('\r');
    let is_yaml = opener == "---";
    let is_pluses = opener == "+++";
    if !is_yaml && !is_pluses {
        return Vec::new();
    }
    let closer_token = if is_yaml { "---" } else { "+++" };
    for line in lines.iter().skip(1) {
        if line.text(source).trim_end_matches('\r') == closer_token {
            return vec![RawCandidate {
                kind: SyntaxKind::FrontMatter,
                span: Span::new(0, line.content_end),
                complete: true,
                detail: Some(if is_yaml {
                    "yaml_style_a".to_string()
                } else {
                    "pluses_style".to_string()
                }),
            }];
        }
    }
    Vec::new()
}

/// Display math: a `$$` run closed by a later `$$` run; an unterminated
/// opener is reported as a candidate spanning the rest of its line.
fn probe_display_math(source: &str) -> Vec<RawCandidate> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'$' {
            let run_start = index;
            while index < bytes.len() && bytes[index] == b'$' {
                index += 1;
            }
            let run_len = index - run_start;
            if run_len < 2 {
                continue;
            }
            // find the closing run
            let mut cursor = index;
            let mut closed = None;
            while cursor < bytes.len() {
                if bytes[cursor] == b'$' {
                    let close_start = cursor;
                    while cursor < bytes.len() && bytes[cursor] == b'$' {
                        cursor += 1;
                    }
                    if cursor - close_start >= 2 {
                        closed = Some((close_start, cursor));
                        break;
                    }
                } else {
                    cursor += 1;
                }
            }
            match closed {
                Some((close_start, close_end)) => {
                    out.push(RawCandidate {
                        kind: SyntaxKind::DisplayMath,
                        span: Span::new(run_start, close_end),
                        complete: true,
                        detail: Some("display_dollars".to_string()),
                    });
                    index = close_end;
                    let _ = close_start;
                }
                None => {
                    let line_end = source[run_start..]
                        .find('\n')
                        .map(|offset| run_start + offset)
                        .unwrap_or(bytes.len());
                    out.push(RawCandidate {
                        kind: SyntaxKind::DisplayMath,
                        span: Span::new(run_start, line_end),
                        complete: false,
                        detail: Some("unterminated".to_string()),
                    });
                    index = line_end;
                }
            }
        } else {
            index += 1;
        }
    }
    out
}

/// Directives: a fence opener whose info string starts with `{`, a `:::`
/// container line, or an inline `{name}` role token.
fn probe_directive(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for line in lines {
        let text = line.text(source);
        let trimmed = text.trim_start_matches(' ');
        let indent = text.len() - trimmed.len();
        if indent <= 3 {
            if trimmed.starts_with(":::") {
                out.push(RawCandidate {
                    kind: SyntaxKind::Directive,
                    span: Span::new(line.start, line.content_end),
                    complete: true,
                    detail: Some("colon_fence".to_string()),
                });
                continue;
            }
            let fence_char = trimmed.chars().next();
            if matches!(fence_char, Some('`') | Some('~')) {
                let fence = fence_char.unwrap();
                let fence_len = trimmed.chars().take_while(|c| *c == fence).count();
                if fence_len >= 3 {
                    let info = trimmed[fence_len..].trim_start();
                    if info.starts_with('{') {
                        out.push(RawCandidate {
                            kind: SyntaxKind::Directive,
                            span: Span::new(line.start, line.content_end),
                            complete: true,
                            detail: Some("fence_directive".to_string()),
                        });
                        continue;
                    }
                }
            }
        }
    }
    // Inline roles: `{name}` where name is lowercase/digits/dash/underscore.
    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'{' {
            let start = index;
            let mut cursor = index + 1;
            let mut name_len = 0usize;
            while cursor < bytes.len() {
                let byte = bytes[cursor];
                if byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || byte == b'-'
                    || byte == b'_'
                {
                    name_len += 1;
                    cursor += 1;
                } else {
                    break;
                }
            }
            if name_len >= 2 && cursor < bytes.len() && bytes[cursor] == b'}' {
                let span = Span::new(start, cursor + 1);
                if !out.iter().any(|existing| existing.span.intersects(&span)) {
                    out.push(RawCandidate {
                        kind: SyntaxKind::Directive,
                        span,
                        complete: true,
                        detail: Some("inline_role".to_string()),
                    });
                }
                index = cursor + 1;
                continue;
            }
        }
        index += 1;
    }
    out
}

/// Table: a line containing `|` followed immediately by a delimiter row
/// with the same cell count (GFM §4.10 shape).
fn probe_table(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for window in lines.windows(2) {
        let header = window[0].text(source);
        let delimiter = window[1].text(source);
        if !header.contains('|') || header.trim().is_empty() {
            continue;
        }
        let header_cells = split_row(header);
        let delimiter_cells = split_row(delimiter);
        if header_cells.is_empty() || header_cells.len() != delimiter_cells.len() {
            continue;
        }
        if !delimiter_cells.iter().all(|cell| is_delimiter_cell(cell)) {
            continue;
        }
        out.push(RawCandidate {
            kind: SyntaxKind::Table,
            span: Span::new(window[0].start, window[1].content_end),
            complete: true,
            detail: Some(format!("columns={}", header_cells.len())),
        });
    }
    out
}

/// Thematic break: a line of >= 3 of the same marker char (`-`, `*`, `_`)
/// with nothing else but spaces, indented by at most 3 spaces.
///
/// BENCH-GRAMMAR-v1 has no thematic break (D13), so under G0 these bytes are
/// paragraph text; the candidate is what makes that divergence visible.
fn probe_thematic_break(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for line in lines {
        let text = line.text(source);
        let trimmed = text.trim_start_matches(' ');
        if text.len() - trimmed.len() > 3 {
            continue;
        }
        let mut marker: Option<char> = None;
        let mut count = 0usize;
        let mut ok = !trimmed.is_empty();
        for ch in trimmed.chars() {
            if ch == ' ' {
                continue;
            }
            if !matches!(ch, '-' | '*' | '_') {
                ok = false;
                break;
            }
            match marker {
                None => marker = Some(ch),
                Some(existing) if existing != ch => {
                    ok = false;
                    break;
                }
                Some(_) => {}
            }
            count += 1;
        }
        if !ok || count < 3 {
            continue;
        }
        out.push(RawCandidate {
            kind: SyntaxKind::ThematicBreak,
            span: Span::new(line.start + (text.len() - trimmed.len()), line.content_end),
            complete: true,
            detail: Some(format!("marker={}", marker.unwrap_or('-'))),
        });
    }
    out
}

/// Strong emphasis: a run of >= 2 identical delimiter chars (`*` or `_`)
/// closed by a later run of the same char whose length is at least the
/// opening length. Unterminated runs are reported incomplete.
///
/// BENCH-GRAMMAR-v1 has single-char `*` LIFO emphasis only (D11), so `**x**`
/// is not strong there; a `__x__` run is ordinary text.
fn probe_strong(source: &str) -> Vec<RawCandidate> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'*' && byte != b'_' {
            index += 1;
            continue;
        }
        let run_start = index;
        while index < bytes.len() && bytes[index] == byte {
            index += 1;
        }
        let run_len = index - run_start;
        if run_len < 2 {
            continue;
        }
        // Look for a closing run of the same char, at least as long.
        let mut cursor = index;
        let mut closer = None;
        while cursor < bytes.len() {
            if bytes[cursor] != byte {
                cursor += 1;
                continue;
            }
            let close_start = cursor;
            while cursor < bytes.len() && bytes[cursor] == byte {
                cursor += 1;
            }
            if cursor - close_start >= run_len {
                closer = Some(cursor);
                break;
            }
        }
        match closer {
            Some(end) => out.push(RawCandidate {
                kind: SyntaxKind::Strong,
                span: Span::new(run_start, end),
                complete: true,
                detail: Some(format!("delimiter={}", byte as char)),
            }),
            None => out.push(RawCandidate {
                kind: SyntaxKind::Strong,
                span: Span::new(run_start, index),
                complete: false,
                detail: Some(format!("delimiter={} unterminated", byte as char)),
            }),
        }
    }
    out
}

/// Image: `![` ... `](` ... `)` with a matching close, reported incomplete
/// when the `![` opener has no closing `)`.
///
/// BENCH-GRAMMAR-v1 has no image kind (D13): under G0 `![alt](url)` is an
/// exclamation mark followed by an inline link.
fn probe_image(source: &str) -> Vec<RawCandidate> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index + 1 < bytes.len() {
        if bytes[index] != b'!' || bytes[index + 1] != b'[' {
            index += 1;
            continue;
        }
        let start = index;
        let mut cursor = index + 2;
        let mut closer = None;
        while cursor < bytes.len() {
            if bytes[cursor] == b'\\' {
                cursor += 2;
                continue;
            }
            if bytes[cursor] == b'\n' {
                break;
            }
            if bytes[cursor] == b')' {
                closer = Some(cursor + 1);
                break;
            }
            cursor += 1;
        }
        match closer {
            Some(end) => out.push(RawCandidate {
                kind: SyntaxKind::Image,
                span: Span::new(start, end),
                complete: true,
                detail: Some("image".to_string()),
            }),
            None => out.push(RawCandidate {
                kind: SyntaxKind::Image,
                span: Span::new(start, bytes.len()),
                complete: false,
                detail: Some("unterminated image".to_string()),
            }),
        }
        index = cursor.max(start + 2);
    }
    out
}

fn split_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let trimmed = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed)
        .strip_suffix('|')
        .unwrap_or(trimmed);
    if trimmed.trim().is_empty() {
        return Vec::new();
    }
    trimmed
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn is_delimiter_cell(cell: &str) -> bool {
    let stripped = cell.strip_prefix(':').unwrap_or(cell);
    let stripped = stripped.strip_suffix(':').unwrap_or(stripped);
    !stripped.is_empty() && stripped.chars().all(|c| c == '-')
}

/// HTML block: a line whose first non-space byte is `<` followed by a
/// letter, `/` or `!`.
fn probe_html_block(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for line in lines {
        let text = line.text(source);
        let trimmed = text.trim_start_matches(' ');
        if text.len() - trimmed.len() > 3 {
            continue;
        }
        let mut chars = trimmed.chars();
        if chars.next() != Some('<') {
            continue;
        }
        let Some(next) = chars.next() else { continue };
        if next.is_ascii_alphabetic() || next == '/' || next == '!' || next == '?' {
            out.push(RawCandidate {
                kind: SyntaxKind::HtmlBlock,
                span: Span::new(line.start, line.content_end),
                complete: true,
                detail: Some("html_block_line".to_string()),
            });
        }
    }
    out
}

/// Indented code: a non-blank line with >= 4 leading spaces, at document
/// start or directly after a blank line (the CommonMark start condition
/// that does not depend on lazy continuation).
fn probe_indented_code(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    let mut previous_blank = true;
    for line in lines {
        let text = line.text(source);
        let blank = text.trim_matches(' ').is_empty();
        let indent = text.len() - text.trim_start_matches(' ').len();
        if !blank && previous_blank && indent >= 4 {
            out.push(RawCandidate {
                kind: SyntaxKind::CodeBlockIndented,
                span: Span::new(line.start, line.content_end),
                complete: true,
                detail: Some("indent4_after_blank".to_string()),
            });
        }
        previous_blank = blank;
    }
    out
}

/// Tilde fence opener: 0..3 leading spaces then >= 3 `~`.
fn probe_tilde_fence(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for line in lines {
        let text = line.text(source);
        let trimmed = text.trim_start_matches(' ');
        if text.len() - trimmed.len() > 3 {
            continue;
        }
        let run = trimmed.chars().take_while(|c| *c == '~').count();
        if run >= 3 {
            out.push(RawCandidate {
                kind: SyntaxKind::CodeBlockFenced,
                span: Span::new(line.start, line.content_end),
                complete: true,
                detail: Some("tilde_fence".to_string()),
            });
        }
    }
    out
}

/// Setext underline: a line of only `=` (>=1) or only `-` (>=1) whose
/// preceding line is non-blank; the candidate spans both lines.
fn probe_setext_underline(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for index in 1..lines.len() {
        let text = lines[index].text(source);
        let trimmed = text.trim_end_matches(' ');
        if trimmed.is_empty() {
            continue;
        }
        let all_equals = trimmed.chars().all(|c| c == '=');
        let all_dashes = trimmed.chars().all(|c| c == '-');
        if !all_equals && !all_dashes {
            continue;
        }
        let previous = lines[index - 1].text(source);
        if previous.trim_matches(' ').is_empty() {
            continue;
        }
        out.push(RawCandidate {
            kind: SyntaxKind::HeadingSetext,
            span: Span::new(lines[index - 1].start, lines[index].content_end),
            complete: true,
            detail: Some(if all_equals {
                "underline_equals".to_string()
            } else {
                "underline_dashes".to_string()
            }),
        });
    }
    out
}

/// Task list item: a list marker line whose first content token is a
/// checkbox.
fn probe_task_list_item(source: &str, lines: &[Line]) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for line in lines {
        let text = line.text(source);
        let trimmed = text.trim_start_matches(' ');
        if text.len() - trimmed.len() > 3 {
            continue;
        }
        let rest = if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            rest
        } else {
            let digits = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits == 0 {
                continue;
            }
            let after = &trimmed[digits..];
            match after
                .strip_prefix(". ")
                .or_else(|| after.strip_prefix(") "))
            {
                Some(rest) => rest,
                None => continue,
            }
        };
        let checkbox = rest.starts_with("[ ] ")
            || rest.starts_with("[x] ")
            || rest.starts_with("[X] ")
            || rest == "[ ]"
            || rest == "[x]"
            || rest == "[X]";
        if checkbox {
            out.push(RawCandidate {
                kind: SyntaxKind::TaskListItem,
                span: Span::new(line.start, line.content_end),
                complete: true,
                detail: Some("task_marker".to_string()),
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Inline probes
// ---------------------------------------------------------------------------

/// Autolink: `<scheme:...>` or `<local@domain>` with no spaces inside.
fn probe_autolink(source: &str) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for (start, end) in angle_regions(source) {
        let inner = &source[start + 1..end];
        if inner.is_empty() || inner.contains(' ') || inner.contains('<') {
            continue;
        }
        let looks_like_uri = match inner.find(':') {
            Some(colon) => {
                let scheme = &inner[..colon];
                (2..=32).contains(&scheme.len())
                    && scheme
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic())
                    && scheme
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
                    && inner.len() > colon + 1
            }
            None => false,
        };
        let looks_like_email = inner.contains('@')
            && inner
                .split('@')
                .all(|part| !part.is_empty() && !part.contains('@'));
        if looks_like_uri || looks_like_email {
            out.push(RawCandidate {
                kind: SyntaxKind::LinkAutolink,
                span: Span::new(start, end + 1),
                complete: true,
                detail: Some(if looks_like_uri {
                    "uri_autolink".to_string()
                } else {
                    "email_autolink".to_string()
                }),
            });
        }
    }
    out
}

/// Inline math: a single-`$` pair on one line with non-empty content that
/// does not start or end with a space (the common `$…$` shape). An
/// unclosed `$` is not reported: it cannot be distinguished from prose.
fn probe_inline_math(source: &str) -> Vec<RawCandidate> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        let run_start = index;
        while index < bytes.len() && bytes[index] == b'$' {
            index += 1;
        }
        if index - run_start != 1 {
            continue;
        }
        let content_start = index;
        let mut cursor = index;
        let mut closer = None;
        while cursor < bytes.len() {
            match bytes[cursor] {
                b'\n' => break,
                b'$' => {
                    closer = Some(cursor);
                    break;
                }
                _ => cursor += 1,
            }
        }
        let Some(closer) = closer else { continue };
        // The closer must be a single `$` run.
        let mut after = closer;
        while after < bytes.len() && bytes[after] == b'$' {
            after += 1;
        }
        if after - closer != 1 {
            index = after;
            continue;
        }
        let content = &source[content_start..closer];
        if !content.is_empty() && !content.starts_with(' ') && !content.ends_with(' ') {
            out.push(RawCandidate {
                kind: SyntaxKind::InlineMath,
                span: Span::new(run_start, closer + 1),
                complete: true,
                detail: Some("single_dollar".to_string()),
            });
        }
        index = closer + 1;
    }
    out
}

/// Raw HTML inline: `<!-- … -->` or a `<tag …>` region on one line.
fn probe_raw_html_inline(source: &str) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for (start, end) in angle_regions(source) {
        let inner = &source[start + 1..end];
        if inner.starts_with("!--") || inner.starts_with('!') || inner.starts_with('?') {
            out.push(RawCandidate {
                kind: SyntaxKind::RawHtmlInline,
                span: Span::new(start, end + 1),
                complete: true,
                detail: Some("declaration".to_string()),
            });
            continue;
        }
        let name: String = inner
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if name.is_empty() || !name.chars().next().unwrap().is_ascii_alphabetic() {
            continue;
        }
        let after = inner.trim_start_matches('/');
        let plausible = after.starts_with(&name)
            && (after.len() == name.len() || after[name.len()..].starts_with([' ', '/', '\t']));
        if plausible {
            out.push(RawCandidate {
                kind: SyntaxKind::RawHtmlInline,
                span: Span::new(start, end + 1),
                complete: true,
                detail: Some("tag".to_string()),
            });
        }
    }
    out
}

/// Strikethrough: `~~…~~` on one line, non-empty content.
fn probe_strikethrough(source: &str) -> Vec<RawCandidate> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index + 1 < bytes.len() {
        if bytes[index] == b'~' && bytes[index + 1] == b'~' {
            let start = index;
            let mut cursor = index + 2;
            let mut closer = None;
            while cursor + 1 < bytes.len() {
                match bytes[cursor] {
                    b'\n' => break,
                    b'~' if bytes[cursor + 1] == b'~' => {
                        closer = Some(cursor);
                        break;
                    }
                    _ => cursor += 1,
                }
            }
            if let Some(closer) = closer {
                let content = &source[start + 2..closer];
                if !content.is_empty() && !content.starts_with(' ') && !content.ends_with(' ') {
                    out.push(RawCandidate {
                        kind: SyntaxKind::Strikethrough,
                        span: Span::new(start, closer + 2),
                        complete: true,
                        detail: Some("double_tilde".to_string()),
                    });
                    index = closer + 2;
                    continue;
                }
            }
        }
        index += 1;
    }
    out
}

/// `<` … `>` regions that stay on one line.
fn angle_regions(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'<' {
            let start = index;
            let mut cursor = index + 1;
            let mut closed = None;
            while cursor < bytes.len() {
                match bytes[cursor] {
                    b'\n' => break,
                    b'>' => {
                        closed = Some(cursor);
                        break;
                    }
                    b'<' => break,
                    _ => cursor += 1,
                }
            }
            if let Some(end) = closed {
                out.push((start, end));
                index = end + 1;
                continue;
            }
        }
        index += 1;
    }
    out
}
