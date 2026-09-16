//! RUN-2b — tree-sitter markdown incremental baseline (issue #19 plan
//! §10).
//!
//! Answers: what does a GENERIC incremental CST engine pay for the same
//! edits markit's block index pays for? Uses the real incremental API
//! exclusively (plan §10 forbids full-parse masquerading):
//!
//! ```text
//! old source → old tree (timed, cold)
//! edit       → old_tree.edit(InputEdit)          [true incremental API]
//! new source → parser.parse(new, Some(old_tree)) [timed]
//! old'/new   → Tree::changed_ranges              [reuse proxy]
//! ```
//!
//! tree-sitter does not expose nodes-rebuilt/nodes-reused, so per plan
//! §10 the reuse evidence is the observable changed-range/source
//! coverage with that limitation stated — never fabricated counters.
//!
//! Pinned versions: tree-sitter 0.19.5 runtime + tree-sitter-markdown
//! 0.7.1 grammar (the grammar pins that runtime; recorded in the
//! summary).

use std::io::Write;
use std::path::Path;

use markit_core::TextEdit;
use tree_sitter::{InputEdit, Parser, Point};

use crate::alloc::timed;
use crate::corpus::{adversarial, synthetic, Adv, SynthOptions};
use crate::mutate;
use crate::text::SurveyText;

fn point_of(text: &str, byte: usize) -> Point {
    let clamped = byte.min(text.len());
    let row = text[..clamped].bytes().filter(|&b| b == b'\n').count();
    let line_start = text[..clamped]
        .rfind('\n')
        .map(|p| p + 1)
        .unwrap_or(0);
    Point {
        row,
        column: clamped - line_start,
    }
}

fn apply_edit(text: &str, edit: &TextEdit) -> String {
    let s = edit.range.start.as_usize();
    let e = edit.range.end.as_usize();
    let mut out = String::with_capacity(text.len() + edit.new_text.len());
    out.push_str(&text[..s]);
    out.push_str(&edit.new_text);
    out.push_str(&text[e..]);
    out
}

struct TsRow {
    corpus: &'static str,
    case_id: &'static str,
    position: &'static str,
    doc_bytes: u64,
    changed_bytes: u64,
    ts_full_us: f64,
    ts_inc_us: f64,
    changed_ranges: u64,
    changed_bytes_cov: u64,
    coverage_pct: f64,
    alloc_bytes_inc: u64,
    has_error: bool,
}

fn measure(
    corpus: &'static str,
    case_id: &'static str,
    position: &'static str,
    doc: &str,
    case: &mutate::Case,
    frac: f64,
) -> Result<TsRow, String> {
    let survey = SurveyText::new(doc.to_string());
    let edit: TextEdit = (case.build)(&survey, frac)
        .ok_or_else(|| format!("no anchor for {case_id}"))?;

    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_markdown::language())
        .map_err(|e| format!("set_language: {e:?}"))?;

    // Old tree, cold full parse (median of 3 + 1 warm).
    let mut full_us = Vec::new();
    let mut old_tree = None;
    for it in 0..4 {
        let (t, us, _) = timed(|| parser.parse(doc, None));
        let t = t.ok_or("cold parse failed")?;
        if it > 0 {
            full_us.push(us);
        }
        old_tree = Some(t);
    }
    let old_tree = old_tree.unwrap();
    if old_tree.root_node().has_error() {
        // Grammar-level errors are acceptable (lossy markdown) but must
        // be visible in the row.
    }

    // Apply the edit. The edited clone `t` (old tree adjusted to the NEW
    // document's coordinates via edit()) must be kept: the
    // changed_ranges contract compares the EDITED old tree to the new
    // tree. CORRECTIVE-1: run-2b originally compared the UNEDITED
    // old_tree here, which measured coordinate offset, not syntax change
    // (retracted; see results/summary/parser-survey-2-baselines-treesitter.md).
    let new_source = apply_edit(doc, &edit);
    let (parsed, inc_us, alloc_inc) = timed(|| {
        let mut t = old_tree.clone();
        t.edit(&InputEdit {
            start_byte: edit.range.start.as_usize(),
            old_end_byte: edit.range.end.as_usize(),
            new_end_byte: edit.range.start.as_usize() + edit.new_text.len(),
            start_position: point_of(doc, edit.range.start.as_usize()),
            old_end_position: point_of(doc, edit.range.end.as_usize()),
            new_end_position: point_of(
                &new_source,
                edit.range.start.as_usize() + edit.new_text.len(),
            ),
        });
        let new = parser.parse(new_source.as_bytes(), Some(&t));
        (new, t)
    });
    let (new_tree_parsed, edited_old) = parsed;
    let new_tree = new_tree_parsed.ok_or("incremental parse failed")?;

    // Reuse proxy: changed ranges between the edited old tree and the
    // new tree (both in new-document coordinates).
    let (ranges_n, cov) = {
        let mut n = 0u64;
        let mut bytes = 0u64;
        for r in edited_old.changed_ranges(&new_tree) {
            n += 1;
            bytes += (r.end_byte - r.start_byte) as u64;
        }
        (n, bytes)
    };

    let ts_full_us = {
        full_us.sort_by(|a, b| a.partial_cmp(b).unwrap());
        full_us[full_us.len() / 2]
    };

    Ok(TsRow {
        corpus,
        case_id,
        position,
        doc_bytes: doc.len() as u64,
        changed_bytes: edit.new_text.len() as u64
            + edit.range.end.as_usize().saturating_sub(edit.range.start.as_usize()) as u64,
        ts_full_us,
        ts_inc_us: inc_us,
        changed_ranges: ranges_n,
        changed_bytes_cov: cov,
        coverage_pct: 100.0 * cov as f64 / new_source.len() as f64,
        alloc_bytes_inc: alloc_inc.bytes,
        has_error: new_tree.root_node().has_error(),
    })
}

fn cases_find(id: &str) -> mutate::Case {
    let mut c = mutate
    ::cases()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("case {id}"));
    c.sweep = false;
    c
}

/// `--ts-deep200`: measure ONLY the 200-deep quote scenario. Run as a
/// child process by [`run`]; expected to die in the C layer on this
/// grammar version, which the parent records as a finding.
pub fn run_deep200(out: &Path) -> Result<String, String> {
    let (deep200, label) = adversarial(Adv::DeepQuote);
    let case = cases_find("quote_char");
    let row = measure(label, "quote_char", "mid", &deep200, &case, 0.5)?;
    std::fs::create_dir_all(out).map_err(|e| format!("mkdir: {e}"))?;
    let line = format!(
        "{},{},{},{},{},{:.1},{:.1},{},{},{:.2},{},{}\n",
        row.corpus,
        row.case_id,
        row.position,
        row.doc_bytes,
        row.changed_bytes,
        row.ts_full_us,
        row.ts_inc_us,
        row.changed_ranges,
        row.changed_bytes_cov,
        row.coverage_pct,
        row.alloc_bytes_inc,
        row.has_error,
    );
    std::fs::write(out.join("ts-deep200.csv"), line)
        .map_err(|e| format!("write: {e}"))?;
    Ok("deep200 survived".into())
}

fn subprocess_deep200_probe(out: &Path) -> String {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return format!("probe spawn failed: {e}"),
    };
    match std::process::Command::new(exe)
        .arg("--ts-deep200")
        .arg("--out")
        .arg(out)
        .output()
    {
        Ok(o) if o.status.code() == Some(0) => {
            "SURVIVED (see ts-deep200.csv)".to_string()
        }
        Ok(o) => format!(
            "**FATAL** — tree-sitter-markdown 0.7.1 aborts (exit code {:?}) \
             on the 200-deep quote document; no row is recorded",
            o.status.code()
        ),
        Err(e) => format!("probe spawn failed: {e}"),
    }
}

pub fn run(out: &Path) -> Result<String, String> {
    let mut rows: Vec<TsRow> = Vec::new();

    let synth_100k = synthetic(SynthOptions {
        target_bytes: 100 * 1024,
        cjk: false,
        crlf: false,
    });
    let synth_1m = synthetic(SynthOptions {
        target_bytes: 1024 * 1024,
        cjk: false,
        crlf: false,
    });
    let (huge_para, huge_label) = adversarial(Adv::HugeParagraph);
    // NOTE: the run-1 adversarial deep-quote document (200 nesting
    // levels) ABORTS this grammar (tree-sitter-markdown 0.7.1 raises a
    // fatal runtime error in the C layer — recorded as a finding, not
    // skipped silently). A 50-level variant provides the partial
    // nesting datapoint.
    let mut deep_quote = String::new();
    for i in 1..=50 {
        deep_quote.push_str(&"> ".repeat(i));
        deep_quote.push_str(&format!("depth {i}\n"));
    }
    deep_quote.push_str("after paragraph\n");
    let quote_label = "adv-deep-quote-50";

    let plan: Vec<(&'static str, &'static str, &'static str, &String, &'static str, f64)> = vec![
        ("synth-100k", "para_insert_char", "bof", &synth_100k, "para_insert_char", 0.0),
        ("synth-100k", "para_insert_char", "mid", &synth_100k, "para_insert_char", 0.5),
        ("synth-100k", "para_insert_char", "eof", &synth_100k, "para_insert_char", 1.0),
        ("synth-100k", "para_sub_char", "mid", &synth_100k, "para_sub_char", 0.5),
        ("synth-100k", "blank_delete", "mid", &synth_100k, "blank_delete", 0.5),
        ("synth-100k", "para_split", "mid", &synth_100k, "para_split", 0.5),
        ("synth-100k", "fence_len_grow", "mid", &synth_100k, "fence_len_grow", 0.5),
        ("synth-1m", "para_insert_char", "bof", &synth_1m, "para_insert_char", 0.0),
        ("synth-1m", "para_insert_char", "mid", &synth_1m, "para_insert_char", 0.5),
        ("synth-1m", "para_insert_char", "eof", &synth_1m, "para_insert_char", 1.0),
        (huge_label, "para_insert_char", "mid", &huge_para, "para_insert_char", 0.5),
        (quote_label, "quote_char", "mid", &deep_quote, "quote_char", 0.5),
    ];

    // The 200-deep document crashes this grammar version; probe it via
    // a child process (the C-layer abort is fatal, unwinding cannot
    // contain it) so the rest of the battery survives and the crash is
    // recorded explicitly rather than silently.
    let deep200_result = subprocess_deep200_probe(out);

    for (corpus, case_id, position, doc, _same, frac) in &plan {
        let case = cases_find(_same);
        eprintln!("ts: {corpus}/{case_id}@{position} ...");
        match measure(corpus, case_id, position, doc, &case, *frac) {
            Ok(r) => rows.push(r),
            Err(reason) => eprintln!("skip {corpus}/{case_id}: {reason}"),
        }
    }

    // Emit.
    std::fs::create_dir_all(out).map_err(|e| format!("mkdir: {e}"))?;
    let csv = out.join("ts.csv");
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(&csv).map_err(|e| format!("create: {e}"))?,
    );
    writeln!(
        f,
        "corpus,case_id,position,doc_bytes,changed_bytes,ts_full_us,ts_inc_us,changed_ranges,changed_bytes_cov,coverage_pct,alloc_bytes_inc,has_error"
    )
    .map_err(|e| format!("write: {e}"))?;
    for r in &rows {
        writeln!(
            f,
            "{},{},{},{},{},{:.1},{:.1},{},{},{:.2},{},{}",
            r.corpus,
            r.case_id,
            r.position,
            r.doc_bytes,
            r.changed_bytes,
            r.ts_full_us,
            r.ts_inc_us,
            r.changed_ranges,
            r.changed_bytes_cov,
            r.coverage_pct,
            r.alloc_bytes_inc,
            r.has_error,
        )
        .map_err(|e| format!("write: {e}"))?;
    }
    f.flush().ok();
    drop(f);

    let mut s = String::new();
    s.push_str("# RUN-2b — tree-sitter markdown incremental baseline (plan §10)\n\n");
    s.push_str("tree-sitter runtime **0.19.5** + tree-sitter-markdown grammar **0.7.1** \
        (the grammar pins that runtime). True incremental API only: old_tree.edit + \
        parse(new, Some(old)); reuse evidence = Tree::changed_ranges (the grammar does \
        not expose nodes-rebuilt/reused — not fabricated).\n\n\
        | corpus | case | pos | doc_B | chg_B | ts_full | ts_inc | chg_ranges | cov_B | cov% | inc/full |\n\
        |---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for r in &rows {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.1} | {:.1} | {} | {} | {:.2} | {:.1}× |\n",
            r.corpus,
            r.case_id,
            r.position,
            r.doc_bytes,
            r.changed_bytes,
            r.ts_full_us,
            r.ts_inc_us,
            r.changed_ranges,
            r.changed_bytes_cov,
            r.coverage_pct,
            r.ts_inc_us / r.ts_full_us.max(0.001),
        ));
    }
    s.push_str("\n200-deep quote probe (run-1 adversarial corpus): ");
    s.push_str(&deep200_result);
    s.push_str("\n\nNotes: `cov%` = changed source coverage of the changed ranges over the \
        new document (the §34 reuse proxy). `inc/full` below 1.0 means the incremental \
        path beat a cold full parse of the same document. Errors: rows with `has_error` \
        carry grammar-level error nodes (lossy markdown is expected to parse lossily).\n");
    std::fs::write(out.join("summary.md"), &s).map_err(|e| format!("write summary: {e}"))?;
    Ok(s)
}
