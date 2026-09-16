//! Mutation families (issue #19 §8) with per-case taxonomy predictions
//! (§3 influence classes I0–I6, validated — not assumed — by the run).

use markit_core::TextEdit;

use crate::text::{ascii_byte_at, del, ins, rep, Role, SurveyText};

#[derive(Clone, Copy)]
pub struct Case {
    pub id: &'static str,
    pub family: &'static str,
    pub construct: &'static str,
    /// Predicted influence class from the issue-19 taxonomy; the run's
    /// job is to confirm / split / merge / reject it.
    pub predicted: &'static str,
    /// Sweep cases run at BOF/25%/50%/75%/EOF; targeted cases run once
    /// (anchor at 50%).
    pub sweep: bool,
    /// Use a paste-intent transaction instead of typing intent.
    pub paste: bool,
    /// Fixed-position cases report where the edit actually lands instead
    /// of the sweep fraction.
    pub pos: Option<&'static str>,
    pub build: fn(&SurveyText, f64) -> Option<TextEdit>,
}

// --- Content edits -------------------------------------------------------

fn para_sub_char(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Para)?;
    let l = &st.lines[i];
    let mid = l.start + (l.content_end - l.start) / 2;
    let b = ascii_byte_at(&st.text, mid, l.content_end)
        .or_else(|| ascii_byte_at(&st.text, l.start, l.content_end))?;
    rep(b, b + 1, "X")
}

fn para_insert_char(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Para)?;
    let l = &st.lines[i];
    let mid = l.start + (l.content_end - l.start) / 2;
    let b = ascii_byte_at(&st.text, mid, l.content_end).unwrap_or(l.start);
    ins(b, "X")
}

fn para_delete_char(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Para)?;
    let l = &st.lines[i];
    let mid = l.start + (l.content_end - l.start) / 2;
    let b = ascii_byte_at(&st.text, mid, l.content_end)
        .or_else(|| ascii_byte_at(&st.text, l.start, l.content_end))?;
    del(b, b + 1)
}

fn heading_char(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Heading)?;
    let l = &st.lines[i];
    ins((l.start + 2).min(l.content_end), "X")
}

fn fence_body_char(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| matches!(r, Role::FenceBody { .. }))?;
    let l = &st.lines[i];
    ins(l.start + (l.content_end - l.start) / 2, "X")
}

fn mermaid_body_char(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    let i = st
        .roles
        .iter()
        .position(|r| matches!(r, Role::FenceBody { mermaid: true }))?;
    let l = &st.lines[i];
    ins(l.start + (l.content_end - l.start) / 2, "X")
}

// --- Delimiter edits ------------------------------------------------------

fn emphasis_delim_insert(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    for i in 0..st.roles.len() {
        if st.roles[i] != Role::Para {
            continue;
        }
        let content = st.line_content(i);
        if let Some(p) = content.find('*') {
            return ins(st.lines[i].start + p + 1, "*");
        }
    }
    None
}

fn codespan_delim_insert(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    for i in 0..st.roles.len() {
        if st.roles[i] != Role::Para {
            continue;
        }
        let content = st.line_content(i);
        if let Some(p) = content.find('`') {
            return ins(st.lines[i].start + p + 1, "`");
        }
    }
    None
}

fn emphasis_close_completion(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    for i in 0..st.roles.len() {
        if st.roles[i] != Role::Para {
            continue;
        }
        let content = st.line_content(i);
        if content.trim_start().starts_with("**") {
            return ins(st.lines[i].content_end, "**");
        }
    }
    None
}

// --- Block boundary edits -------------------------------------------------

fn blank_insert(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Blank)?;
    let l = &st.lines[i];
    if l.end >= st.len() {
        return None; // the EOF phantom is not a real blank line
    }
    ins(l.start, "\n")
}

fn blank_delete(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Blank)?;
    let l = &st.lines[i];
    if l.end >= st.len() || l.content_end != l.start {
        return None; // need a pure "\n"-only line strictly inside the doc
    }
    del(l.start, l.end)
}

fn para_split(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Para)?;
    let l = &st.lines[i];
    let mid = l.start + (l.content_end - l.start) / 2;
    // Latin: replace a space with the paragraph break. CJK lines carry no
    // spaces — split at the nearest char boundary instead.
    if let Some(b) = st.text.as_bytes()[mid..l.content_end]
        .iter()
        .position(|&c| c == b' ')
        .map(|k| mid + k)
    {
        return rep(b, b + 1, "\n\n");
    }
    let mut b = mid;
    while b > l.start && !st.text.is_char_boundary(b) {
        b -= 1;
    }
    if b == l.start {
        None
    } else {
        ins(b, "\n\n")
    }
}

fn setext_add(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Para)?;
    let l = &st.lines[i];
    ins(l.content_end, "\n---")
}

fn heading_from_para(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Para)?;
    ins(st.lines[i].start, "## ")
}

// --- Container edits ------------------------------------------------------

fn list_indent_add(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::ListItem)?;
    ins(st.lines[i].start, "  ")
}

fn list_marker_add(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick_index(frac, |i| {
        i > 0 && st.roles[i] == Role::Para && st.roles[i - 1] == Role::ListItem
    })?;
    ins(st.lines[i].start, "- ")
}

fn list_marker_remove(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::ListItem)?;
    let l = &st.lines[i];
    let content = st.line_content(i);
    let indent = content.len() - content.trim_start().len();
    let b = l.start + indent;
    let bytes = st.text.as_bytes();
    if bytes.get(b + 1) == Some(&b' ') {
        del(b, b + 2)
    } else {
        del(b, b + 1)
    }
}

fn quote_char(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Quote)?;
    let l = &st.lines[i];
    ins((l.start + 2).min(l.content_end), "X")
}

fn quote_add(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick_index(frac, |i| {
        i > 0 && st.roles[i] == Role::Para && st.roles[i - 1] == Role::Quote
    })?;
    ins(st.lines[i].start, "> ")
}

fn quote_remove(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::Quote)?;
    let l = &st.lines[i];
    if st.line_content(i).starts_with("> ") {
        del(l.start, l.start + 2)
    } else {
        None
    }
}

// --- State-propagating edits ----------------------------------------------

fn fence_opener(st: &SurveyText, frac: f64) -> Option<usize> {
    st.pick(frac, |r| matches!(r, Role::FenceOpen { .. }))
}

fn fence_opener_break(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = fence_opener(st, frac)?;
    let l = &st.lines[i];
    // Break the fence by destroying one backtick — the third char of the
    // line (NOT the last char: "```rust"'s last byte is info text).
    let b = l.start + 2;
    if st.text.as_bytes().get(b) == Some(&b'`') {
        rep(b, b + 1, "X")
    } else {
        None
    }
}

fn fence_opener_delete(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = fence_opener(st, frac)?;
    let l = &st.lines[i];
    del(l.start, l.end)
}

fn fence_closer_delete(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = st.pick(frac, |r| *r == Role::FenceClose)?;
    let l = &st.lines[i];
    del(l.start, l.end)
}

fn fence_len_grow(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = fence_opener(st, frac)?;
    let l = &st.lines[i];
    ins(l.content_end, "`")
}

fn fence_info_edit(st: &SurveyText, frac: f64) -> Option<TextEdit> {
    let i = fence_opener(st, frac)?;
    let Role::FenceOpen { info: (s, e) } = st.roles[i] else {
        return None;
    };
    if e > s {
        rep(s, e, "golang")
    } else {
        None
    }
}

// --- Semantic-global edits --------------------------------------------------

fn ref_def_add(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    ins(st.len(), "\n[x]: /url\n")
}

fn ref_def_edit(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    for i in 0..st.roles.len() {
        if st.roles[i] != Role::Para {
            continue;
        }
        let content = st.line_content(i);
        if content.trim_start().starts_with('[') && content.contains("]: /") {
            return ins(st.lines[i].content_end, "x");
        }
    }
    None
}

fn ref_user_edit(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    for i in 0..st.roles.len() {
        if st.roles[i] != Role::Para {
            continue;
        }
        let content = st.line_content(i);
        if let Some(p) = content.find("[link](/target)") {
            let b = st.lines[i].start + p + 1;
            return rep(b, b + 4, "links");
        }
    }
    None
}

// --- Structural stress ------------------------------------------------------

fn bof_insert_char(_st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    ins(0, "X")
}

fn bof_insert_newline(_st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    ins(0, "\n")
}

fn bof_delete_char(_st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    del(0, 1)
}

fn eof_append_char(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    ins(st.len(), "X")
}

fn large_paste_1k(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    let i = st.pick(0.5, |r| *r == Role::Para)?;
    let mut paste = String::with_capacity(64 * 1000);
    for j in 0..1000 {
        paste.push_str(&format!("paste line {j} carries ordinary survey corpus words.\n"));
    }
    ins(st.lines[i].start, &paste)
}

fn large_delete_mid(st: &SurveyText, _frac: f64) -> Option<TextEdit> {
    let a = st.pick(0.4, |r| *r == Role::Para)?;
    let b = st.pick(0.6, |r| *r == Role::Para)?;
    if b <= a {
        return None;
    }
    del(st.lines[a].start, st.lines[b].start)
}

/// The battery. Order is stable so CSV diffs stay reviewable.
pub fn cases() -> Vec<Case> {
    vec![
        // Sweep cases (BOF / 25% / 50% / 75% / EOF).
        case("para_sub_char", "ContentEdit", "paragraph-same-length", "I0", true, para_sub_char),
        case("para_insert_char", "ContentEdit", "paragraph-insert-1b", "I0", true, para_insert_char),
        case("para_delete_char", "ContentEdit", "paragraph-delete-1b", "I0", true, para_delete_char),
        case("heading_char", "ContentEdit", "heading-text", "I0", true, heading_char),
        case("fence_body_char", "ContentEdit", "fence-body", "I0", true, fence_body_char),
        case("blank_insert", "BlockBoundary", "blank-line-insert", "I2", true, blank_insert),
        case("blank_delete", "BlockBoundary", "blank-line-delete-merge", "I2", true, blank_delete),
        // Targeted cases.
        case("mermaid_body_char", "ContentEdit", "mermaid-body", "I6", false, mermaid_body_char),
        case("emphasis_delim_insert", "DelimiterEdit", "emphasis-star", "I1", false, emphasis_delim_insert),
        case("codespan_delim_insert", "DelimiterEdit", "codespan-backtick", "I1", false, codespan_delim_insert),
        case("emphasis_close_completion", "DelimiterEdit", "unclosed-emphasis-complete", "I1", false, emphasis_close_completion),
        case("para_split", "BlockBoundary", "paragraph-split", "I2", false, para_split),
        case("setext_add", "BlockBoundary", "setext-underline-trap", "I2", false, setext_add),
        case("heading_from_para", "BlockBoundary", "para-to-heading", "I2", false, heading_from_para),
        case("list_indent_add", "Container", "list-indent", "I3", false, list_indent_add),
        case("list_marker_add", "Container", "list-marker-add", "I3", false, list_marker_add),
        case("list_marker_remove", "Container", "list-marker-remove", "I3", false, list_marker_remove),
        case("quote_char", "Container", "quote-content", "I3", false, quote_char),
        case("quote_add", "Container", "quote-marker-add", "I3", false, quote_add),
        case("quote_remove", "Container", "quote-marker-remove", "I3", false, quote_remove),
        case("fence_opener_break", "StatePropagating", "fence-opener-char", "I4", false, fence_opener_break),
        case("fence_opener_delete", "StatePropagating", "fence-opener-delete", "I4", false, fence_opener_delete),
        case("fence_closer_delete", "StatePropagating", "fence-closer-delete", "I4", false, fence_closer_delete),
        case("fence_len_grow", "StatePropagating", "fence-length-grow", "I4", false, fence_len_grow),
        case("fence_info_edit", "StatePropagating", "fence-info-string", "I1", false, fence_info_edit),
        case("ref_def_add", "SemanticGlobal", "reference-def-add", "I5", false, ref_def_add),
        case("ref_def_edit", "SemanticGlobal", "reference-def-edit", "I5", false, ref_def_edit),
        case("ref_user_edit", "SemanticGlobal", "reference-user-edit", "I5", false, ref_user_edit),
        case("bof_insert_char", "Stress", "bof-insert-1b", "-", false, bof_insert_char),
        case("bof_insert_newline", "Stress", "bof-insert-newline", "-", false, bof_insert_newline),
        case("bof_delete_char", "Stress", "bof-delete-1b", "-", false, bof_delete_char),
        case("eof_append_char", "Stress", "eof-append-1b", "-", false, eof_append_char),
        case("large_paste_1k", "Stress", "paste-1000-lines", "-", true, large_paste_1k),
        case("large_delete_mid", "Stress", "delete-20pct", "-", false, large_delete_mid),
    ]
}

fn case(
    id: &'static str,
    family: &'static str,
    construct: &'static str,
    predicted: &'static str,
    sweep: bool,
    build: fn(&SurveyText, f64) -> Option<TextEdit>,
) -> Case {
    Case {
        id,
        family,
        construct,
        predicted,
        sweep,
        paste: id == "large_paste_1k",
        pos: match id {
            "bof_insert_char" | "bof_insert_newline" | "bof_delete_char" => Some("bof"),
            "eof_append_char" => Some("eof"),
            _ => None,
        },
        build,
    }
}
