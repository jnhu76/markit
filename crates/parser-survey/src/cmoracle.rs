//! ORACLE-B (DIALECT SEMANTICS) — CommonMark spec examples vs the
//! measured parser's normalized block observation (issue #19 plan §6–7).
//!
//! NOT a CST comparison. Each side is reduced to what a block-level
//! Markdown representation can honestly claim:
//!
//! markit side   — `MarkdownState` blocks, blanks skipped:
//!                 Paragraph→P, Heading{level}→H(n), BlockQuote→BQ,
//!                 List{Bullet}→UL, List{Ordered}→OL, FencedCode→CODE.
//! expected side — block-level open tags scanned from the spec's
//!                 expected HTML in document order; container interiors
//!                 tracked so nested tags can be told from flat ones.
//!
//! Verdicts:
//! - PASS           observed sequence == expected sequence
//! - FLAT_CONTAINER observed == expected with container INTERIORS
//!                  dropped (block-level skeleton agrees; the measured
//!                  representation does not represent nesting — E3)
//! - FAIL           aligned block kinds disagree (true semantic
//!                  disagreement for a claimed construct)
//! - UNSUPPORTED    expected structure needs tags outside the measured
//!                  vocabulary (thematic break `<hr>`, HTML blocks,
//!                  indented code) — a scope statement, never a FAIL
//!
//! Compared semantics: block-kind sequence + heading levels. Fence
//! closedness, list signatures and item counts are recorded but not
//! compared in v1 (noted limitation).

use std::io::Write;
use std::path::Path;

use markit_core::markdown::{BlockDetail, BlockKind, MarkdownState};
use markit_core::Document;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Norm {
    P,
    H(u8),
    BQ,
    UL,
    OL,
    CODE,
}

impl Norm {
    fn name(&self) -> String {
        match self {
            Norm::P => "P".into(),
            Norm::H(n) => format!("H{n}"),
            Norm::BQ => "BQ".into(),
            Norm::UL => "UL".into(),
            Norm::OL => "OL".into(),
            Norm::CODE => "CODE".into(),
        }
    }
}

fn seq_name(seq: &[Norm]) -> String {
    seq.iter()
        .map(|n| n.name())
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Verdict {
    Pass,
    FlatContainer,
    Fail,
    Unsupported,
}

impl Verdict {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::FlatContainer => "FLAT_CONTAINER",
            Verdict::Fail => "FAIL",
            Verdict::Unsupported => "UNSUPPORTED",
        }
    }
}

pub(crate) struct Row {
    pub(crate) example: u64,
    pub(crate) section: String,
    pub(crate) start_line: u64,
    pub(crate) verdict: Verdict,
    pub(crate) observed: String,
    pub(crate) expected: String,
    pub(crate) note: String,
}

/// Observed normalized sequence from the measured implementation.
fn observe(markdown: &str) -> Vec<Norm> {
    let doc = Document::new(markdown.to_string());
    let state = MarkdownState::build(&doc.snapshot());
    state
        .blocks()
        .filter_map(|b| {
            if b.kind() == BlockKind::Blank {
                return None;
            }
            Some(match (b.kind(), b.detail()) {
                (BlockKind::Paragraph, _) => Norm::P,
                (BlockKind::Heading, BlockDetail::Heading { level, .. }) => Norm::H(*level),
                (BlockKind::BlockQuote, _) => Norm::BQ,
                (BlockKind::UnorderedList, _) => Norm::UL,
                (BlockKind::OrderedList, _) => Norm::OL,
                (BlockKind::FencedCode, _) => Norm::CODE,
                (BlockKind::Blank, _) => return None,
                _ => return None,
            })
        })
        .collect()
}

/// Block-level open tags in the expected HTML with their container
/// depth (0 = top level). Returns Err(unmapped-tag) when the example
/// needs constructs outside the compared vocabulary.
fn expected_tags(html: &str) -> Result<Vec<(Norm, usize)>, String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Block(Norm),
        Container(Norm),
        Ignorable,
    }
    fn classify(name: &str) -> Option<Kind> {
        Some(match name {
            "p" => Kind::Block(Norm::P),
            "h1" => Kind::Block(Norm::H(1)),
            "h2" => Kind::Block(Norm::H(2)),
            "h3" => Kind::Block(Norm::H(3)),
            "h4" => Kind::Block(Norm::H(4)),
            "h5" => Kind::Block(Norm::H(5)),
            "h6" => Kind::Block(Norm::H(6)),
            "blockquote" => Kind::Container(Norm::BQ),
            "ul" => Kind::Container(Norm::UL),
            "ol" => Kind::Container(Norm::OL),
            "pre" => Kind::Block(Norm::CODE),
            // block-level machinery and inline vocabulary inside expected
            // HTML: present but not compared semantics.
            "li" | "code" | "em" | "strong" | "a" | "img" | "br" | "del" | "input"
            | "span" | "s" => Kind::Ignorable,
            _ => return None,
        })
    }

    let b = html.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;
    // Whether any compared block tag has opened. An inline-class tag
    // opening while `in_block` is false is a RAW HTML block (e.g. an
    // `<a href>` line the spec passes through verbatim) — not inline
    // machinery inside a compared block.
    let mut in_block = false;
    let mut seq = Vec::new();
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        if html[i..].starts_with("<!--") {
            // A degenerate comment like `<!-->` closes with `-->` that
            // overlaps the opener — search from `<!`, not past it.
            match html[i + 2..].find("-->") {
                Some(off) => i += 2 + off + 3,
                None => i = b.len(),
            }
            continue;
        }
        let close = b.get(i + 1) == Some(&b'/');
        let start = if close { i + 2 } else { i + 1 };
        let name_end = b[start..]
            .iter()
            .position(|c| !c.is_ascii_alphanumeric())
            .map(|p| start + p)
            .unwrap_or(b.len());
        let name = &html[start..name_end];
        if name.eq_ignore_ascii_case("hr") && !close {
            return Err("hr (thematic break)".into());
        }
        match (classify(&name.to_ascii_lowercase()), close) {
            (Some(Kind::Block(n)), false) => {
                in_block = true;
                seq.push((n, depth));
            }
            (Some(Kind::Block(_)), true) => {}
            (Some(Kind::Container(n)), false) => {
                in_block = true;
                depth += 1;
                seq.push((n, depth - 1));
            }
            (Some(Kind::Container(_)), true) => depth = depth.saturating_sub(1),
            (Some(Kind::Ignorable), false) if !in_block => {
                return Err(format!("raw html block (<{name}>)"));
            }
            (Some(Kind::Ignorable), _) => {}
            // open tag outside the vocabulary: raw HTML blocks, tables,
            // whatever the dialect cannot honestly claim; close tags of
            // unmapped names are just ends of ignored regions
            (None, true) => {}
            (None, false) => return Err(format!("<{name}>")),
        }
        i = name_end;
        // skip attributes to the tag's '>'
        while i < b.len() && b[i] != b'>' {
            i += 1;
        }
        i += 1;
    }
    Ok(seq)
}

/// Depth-aware flattening: drop container INTERIOR tags — what a flat
/// block representation could at best produce.
fn flatten(tagged: &[(Norm, usize)]) -> Vec<Norm> {
    tagged
        .iter()
        .filter(|(_, d)| *d == 0)
        .map(|(n, _)| *n)
        .collect()
}

fn judge(markdown: &str, html: &str) -> (Verdict, String, String, String) {
    let observed = observe(markdown);
    judge_observed(&observed, markdown, html, false)
}

/// Judgement against a precomputed observed sequence. `indented_code`:
/// whether the measured implementation claims indented code blocks (it
/// controls the indented-code UNSUPPORTED conversion — MD4C claims
/// them, markit L1 does not).
pub(crate) fn judge_observed(
    observed: &[Norm],
    markdown: &str,
    html: &str,
    indented_code: bool,
) -> (Verdict, String, String, String) {
    match expected_tags(html) {
        Err(tag) => (
            Verdict::Unsupported,
            seq_name(observed),
            String::new(),
            format!("vocabulary: {tag}"),
        ),
        Ok(tagged) => {
            // Indented code blocks: expected `<pre><code>` whose source
            // does not START with a fence opener means indented code —
            // a construct outside the measured vocabulary (plan §6:
            // unsupported, not failed) unless the implementation claims it.
            let fenced = markdown
                .lines()
                .find(|l| !l.trim().is_empty())
                .map(|l| {
                    let t = l.trim_start();
                    t.starts_with("```") || t.starts_with("~~~")
                })
                .unwrap_or(false);
            if !indented_code && !fenced && tagged.iter().any(|(n, _)| *n == Norm::CODE) {
                return (
                    Verdict::Unsupported,
                    seq_name(&observed),
                    String::new(),
                    "vocabulary: indented code block".into(),
                );
            }
            // Expected output that consumed everything with no compared
            // block while the source opens a tag: a raw HTML block that
            // produced no compared structure.
            if tagged.is_empty() && markdown.trim_start().starts_with('<') {
                return (
                    Verdict::Unsupported,
                    seq_name(&observed),
                    String::new(),
                    "vocabulary: raw html block".into(),
                );
            }
            let full: Vec<Norm> = tagged.iter().map(|(n, _)| *n).collect();
            let flat = flatten(&tagged);
            let obs = seq_name(observed);
            let exp = seq_name(&full);
            if observed == full {
                (Verdict::Pass, obs, exp, String::new())
            } else if observed == flat {
                (Verdict::FlatContainer, obs, exp, String::new())
            } else {
                (Verdict::Fail, obs, exp, String::new())
            }
        }
    }
}

/// Runs the battery over a pinned spec.json; writes cases.csv +
/// summary.md under `out` and returns the summary text.
pub fn run(spec_path: &Path, out: &Path) -> Result<String, String> {
    let raw = std::fs::read_to_string(spec_path)
        .map_err(|e| format!("cannot read {}: {e}", spec_path.display()))?;
    let examples: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("spec.json parse: {e}"))?;
    let arr = examples
        .as_array()
        .ok_or("spec.json: expected top-level array")?;

    let mut rows: Vec<Row> = Vec::new();
    for ex in arr {
        let get = |k: &str| ex.get(k).and_then(|v| v.as_str()).unwrap_or_default();
        let markdown = get("markdown");
        let html = get("html");
        let (verdict, observed, expected, note) = judge(markdown, html);
        rows.push(Row {
            example: ex.get("example").and_then(|v| v.as_u64()).unwrap_or(0),
            section: get("section").to_string(),
            start_line: ex.get("start_line").and_then(|v| v.as_u64()).unwrap_or(0),
            verdict,
            observed,
            expected,
            note,
        });
    }

    std::fs::create_dir_all(out).map_err(|e| format!("mkdir {}: {e}", out.display()))?;
    let csv = out.join("cases.csv");
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(&csv).map_err(|e| format!("create {}: {e}", csv.display()))?,
    );
    writeln!(f, "example,section,start_line,verdict,observed,expected,note")
        .map_err(|e| format!("write csv: {e}"))?;
    for r in &rows {
        writeln!(
            f,
            "{},{},{},{},{},{},{}",
            r.example,
            r.section.replace(',', ";"),
            r.start_line,
            r.verdict.name(),
            r.observed.replace(',', ";"),
            r.expected.replace(',', ";"),
            r.note.replace(',', ";"),
        )
        .map_err(|e| format!("write csv: {e}"))?;
    }
    f.flush().ok();
    drop(f);

    let summary = summarize(&rows);
    std::fs::write(out.join("summary.md"), &summary)
        .map_err(|e| format!("write summary: {e}"))?;
    Ok(summary)
}

pub(crate) fn summarize(rows: &[Row]) -> String {
    let mut s = String::new();
    s.push_str("# ORACLE-B (DIALECT SEMANTICS) — CommonMark 0.31.2\n\n");
    s.push_str("spec: CommonMark 0.31.2, sha256 d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20, retrieved 2026-09-16\n");
    s.push_str(&format!("examples: {}\n\n", rows.len()));

    let mut counts: Vec<(&str, usize)> = vec![
        (Verdict::Pass.name(), 0),
        (Verdict::FlatContainer.name(), 0),
        (Verdict::Fail.name(), 0),
        (Verdict::Unsupported.name(), 0),
    ];
    for r in rows {
        let slot = counts
            .iter_mut()
            .find(|(n, _)| *n == r.verdict.name())
            .unwrap();
        slot.1 += 1;
    }
    s.push_str("| verdict | n | share |\n|---|---:|---:|\n");
    for (n, c) in &counts {
        s.push_str(&format!(
            "| {n} | {c} | {:.1}% |\n",
            100.0 * *c as f64 / rows.len().max(1) as f64
        ));
    }

    // Per-section breakdown.
    let mut sections: Vec<&str> = rows.iter().map(|r| r.section.as_str()).collect();
    sections.sort_unstable();
    sections.dedup();
    s.push_str("\n## Per-section verdicts\n\n\
        | section | pass | flat | fail | unsupported |\n|---|---:|---:|---:|---:|\n");
    for sec in &sections {
        let rs: Vec<&Row> = rows.iter().filter(|r| r.section == *sec).collect();
        let c = |v: Verdict| rs.iter().filter(|r| r.verdict == v).count();
        s.push_str(&format!(
            "| {sec} | {} | {} | {} | {} |\n",
            c(Verdict::Pass),
            c(Verdict::FlatContainer),
            c(Verdict::Fail),
            c(Verdict::Unsupported),
        ));
    }

    // First failing examples with sequences (the interesting rows).
    s.push_str("\n## FAIL examples (observed vs expected, first 25)\n\n\
        | ex | section | observed | expected |\n|---|---|---|---|\n");
    for r in rows.iter().filter(|r| r.verdict == Verdict::Fail).take(25) {
        s.push_str(&format!(
            "| {} | {} | `{}` | `{}` |\n",
            r.example, r.section, r.observed, r.expected
        ));
    }

    s.push_str("\n## UNSUPPORTED vocabulary (what the measured dialect does not claim)\n\n\
        | vocabulary | n |\n|---|---:|\n");
    let mut vocab: Vec<(String, usize)> = Vec::new();
    for r in rows.iter().filter(|r| r.verdict == Verdict::Unsupported) {
        match vocab.iter_mut().find(|(v, _)| *v == r.note) {
            Some((_, c)) => *c += 1,
            None => vocab.push((r.note.clone(), 1)),
        }
    }
    vocab.sort_by(|a, b| b.1.cmp(&a.1));
    for (v, n) in &vocab {
        s.push_str(&format!("| {v} | {n} |\n"));
    }

    s.push_str("\nVerdict semantics: PASS = normalized block sequence equal; \
        FLAT_CONTAINER = equal after dropping container interiors (the measured \
        representation is flat — E3); FAIL = aligned block kinds disagree; \
        UNSUPPORTED = example needs constructs outside the measured vocabulary \
        (scope statement, not a failure). Compared semantics: block-kind sequence \
        + heading levels; fence closedness / list signatures recorded but not \
        compared in v1.\n");
    s
}
