//! RUN-2 external baselines (issue #19 plan §8–12). This module: M0-B,
//! the MD4C full-parse baseline.
//!
//! Answers the plan's question — "what does a minimal high-throughput
//! full Markdown parse actually cost?" — on the SAME corpora the
//! markit-core control uses, split into:
//!
//! - `markit_build`   — `MarkdownState::build` (the M0 control itself;
//!   includes the implementation's own block/inline normalization);
//! - `md4c_count`     — MD4C parser-native cost (event counting only);
//! - `md4c_norm`      — MD4C parse + normalization into the ORACLE-B
//!   block vocabulary; `md4c_norm − md4c_count` is the adapter cost,
//!   never merged into the native number (plan §9).
//!
//! Plus the CommonMark 0.31.2 cross-check: MD4C's normalized block
//! sequence per spec example, judged by the same ORACLE-B rules (with
//! indented code counted as claimed vocabulary — MD4C parses it).

use std::io::Write;
use std::path::Path;

use markit_core::markdown::MarkdownState;
use markit_core::Document;

use crate::alloc::timed;
use crate::cmoracle::{self, Row};
use crate::corpus::{human_size, synthetic, SynthOptions};
use crate::md4c;

const SIZES: &[usize] = &[
    1024,
    10 * 1024,
    100 * 1024,
    1024 * 1024,
    10 * 1024 * 1024,
];

fn median(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn iters_for(len: usize) -> usize {
    if len < 100_000 {
        9
    } else if len < 2_000_000 {
        7
    } else {
        3
    }
}

/// Full-parse cost sweep over the synthetic corpus family.
fn sweep(out: &Path) -> Result<String, String> {
    let mut s = String::new();
    s.push_str("# M0-B — MD4C full-parse baseline (plan §9)\n\n");
    s.push_str("MD4C release-0.5.3, vendored, flags=0 (no extensions). ");
    s.push_str("Medians; same machine/build as the run-1.1 markit control.\n\n");
    s.push_str("| corpus | bytes | markit_build | md4c_count | md4c_norm | norm−count | events | ev/KB | ns/B markit | ns/B md4c |\n\
        |---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");

    let mut csv = String::from(
        "corpus,bytes,markit_build_us,md4c_count_us,md4c_norm_us,md4c_events,alloc_bytes_md4c_count,alloc_bytes_markit\n",
    );
    for &size in SIZES {
        let text = synthetic(SynthOptions {
            target_bytes: size,
            cjk: false,
            crlf: false,
        });
        let doc = Document::new(text.clone());

        let n = iters_for(size);
        let mut mb = Vec::with_capacity(n);
        let mut mc = Vec::with_capacity(n);
        let mut mn = Vec::with_capacity(n);
        let mut events = 0u64;
        let mut alloc_md4c = 0u64;
        let mut alloc_markit = 0u64;
        for it in 0..n + 1 {
            let (ev, us, al) = timed(|| md4c::count_events(&text));
            let _ = ev.map_err(|e| format!("md4c error {e}"))?;
            let (_, us2, _al2) = timed(|| md4c::normalize(&text));
            let doc_ref = &doc;
            let (_state, us3, _al3) = timed(|| {
                let snap = doc_ref.snapshot();
                MarkdownState::build(&snap)
            });
            if it == 0 {
                events = ev.unwrap_or(0);
                alloc_md4c = al.bytes;
                alloc_markit = _al3.bytes;
                continue; // warm-up
            }
            mc.push(us);
            mn.push(us2);
            mb.push(us3);
        }
        let (mc, mn, mb) = (median(&mut mc), median(&mut mn), median(&mut mb));
        s.push_str(&format!(
            "| synth-{} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {} | {:.1} | {:.1} | {:.1} |\n",
            human_size(size),
            text.len(),
            mb,
            mc,
            mn,
            mn - mc,
            events,
            events as f64 / (text.len() as f64 / 1024.0),
            mb * 1000.0 / text.len() as f64,
            mc * 1000.0 / text.len() as f64,
        ));
        csv.push_str(&format!(
            "synth-{},{},{:.1},{:.1},{:.1},{},{},{}\n",
            human_size(size),
            text.len(),
            mb,
            mc,
            mn,
            events,
            alloc_md4c,
            alloc_markit,
        ));
    }

    std::fs::create_dir_all(out).map_err(|e| format!("mkdir: {e}"))?;
    let csv_path = out.join("md4c-sizes.csv");
    std::fs::write(&csv_path, &csv).map_err(|e| format!("write csv: {e}"))?;
    s.push_str("\nColumn notes: `md4c_count` = parser-native (event counting only); \
        `md4c_norm` = native + normalization into the ORACLE-B vocabulary; the delta is \
        adapter cost. `markit_build` = MarkdownState::build (includes markit's own \
        block/inline normalization — NOT directly comparable to `md4c_count`; the \
        comparison for §34 is per-column, never a single ranking).\n");
    Ok(s)
}

/// CommonMark 0.31.2 cross-check: MD4C block observation vs expected,
/// same ORACLE-B rules, indented code counted as claimed.
fn commonmark(spec_path: &Path, out: &Path) -> Result<String, String> {
    let raw =
        std::fs::read_to_string(spec_path).map_err(|e| format!("read spec: {e}"))?;
    let arr = serde_json::from_str::<serde_json::Value>(&raw)
        .map_err(|e| format!("spec parse: {e}"))?
        .as_array()
        .ok_or("spec: top-level array expected")?
        .clone();

    let mut rows: Vec<Row> = Vec::new();
    for ex in &arr {
        let get = |k: &str| ex.get(k).and_then(|v| v.as_str()).unwrap_or_default();
        let markdown = get("markdown");
        let html = get("html");
        let observed = md4c::normalize(markdown).map_err(|e| format!("md4c error {e}"))?;
        let (verdict, obs, exp, note) =
            cmoracle::judge_observed(&observed, markdown, html, true);
        rows.push(Row {
            example: ex.get("example").and_then(|v| v.as_u64()).unwrap_or(0),
            section: get("section").to_string(),
            start_line: ex.get("start_line").and_then(|v| v.as_u64()).unwrap_or(0),
            verdict,
            observed: obs,
            expected: exp,
            note,
        });
    }

    std::fs::create_dir_all(out).map_err(|e| format!("mkdir: {e}"))?;
    let csv = out.join("md4c-commonmark.csv");
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(&csv).map_err(|e| format!("create: {e}"))?,
    );
    writeln!(f, "example,section,start_line,verdict,observed,expected,note")
        .map_err(|e| format!("write: {e}"))?;
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
        .map_err(|e| format!("write: {e}"))?;
    }
    f.flush().ok();
    drop(f);
    Ok(cmoracle::summarize(&rows))
}

/// Runs the whole M0-B baseline; writes artifacts under `out`,
/// returns the combined summary.
pub fn run(spec_path: &Path, out: &Path) -> Result<String, String> {
    let mut s = sweep(out)?;
    s.push_str("\n\n---\n\n");
    s.push_str(&commonmark(spec_path, &out.join("commonmark"))?);
    Ok(s)
}
