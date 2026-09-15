//! parser-survey — research harness for issue #19
//! (MARKIT-INCREMENTAL-MARKDOWN-PARSER-SURVEY-1).
//!
//! Runs the mutation battery over synthetic + adversarial corpora against
//! the current markit-core Markdown block index, measuring full-parse vs
//! incremental work, allocations, projection invalidation, and the
//! correctness oracle.
//!
//! Usage:
//!   cargo run --release -p parser-survey -- \
//!     [--sizes 1k,10k,100k,1m] [--iters N] [--warm N] [--out DIR]
//!
//! Research-only: not product code.

mod alloc;
mod corpus;
mod emit;
mod measure;
mod mutate;
mod text;

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::corpus::{adversarial, human_size, synthetic, Adv, SynthOptions};

struct Args {
    sizes: Vec<usize>,
    iters: Option<usize>,
    warm: usize,
    out: PathBuf,
}

fn parse_size(s: &str) -> Option<usize> {
    let t = s.trim().to_ascii_lowercase();
    if let Some(n) = t.strip_suffix("k") {
        n.parse::<usize>().ok().map(|n| n * 1024)
    } else if let Some(n) = t.strip_suffix("m") {
        n.parse::<usize>().ok().map(|n| n * 1024 * 1024)
    } else {
        t.parse::<usize>().ok()
    }
}

fn parse_args() -> Args {
    let mut args = Args {
        sizes: vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024],
        iters: None,
        warm: 2,
        out: PathBuf::from(format!(
            "results/raw/parser-survey/run-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        )),
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--sizes" => {
                if let Some(list) = it.next() {
                    args.sizes = list
                        .split(',')
                        .filter_map(parse_size)
                        .collect();
                }
            }
            "--iters" => args.iters = it.next().and_then(|v| v.parse().ok()),
            "--warm" => args.warm = it.next().and_then(|v| v.parse().ok()).unwrap_or(2),
            "--out" => {
                if let Some(d) = it.next() {
                    args.out = PathBuf::from(d);
                }
            }
            other => {
                eprintln!("unknown arg {other}");
                std::process::exit(2);
            }
        }
    }
    args
}

fn auto_iters(len: usize) -> usize {
    if len < 50_000 {
        9
    } else if len < 500_000 {
        7
    } else {
        5
    }
}

fn first_successful_run(
    args: &Args,
    corpus: &str,
    frac: f64,
    case: &mutate::Case,
    doc: &str,
) -> Option<measure::Row> {
    let iters = args.iters.unwrap_or_else(|| auto_iters(doc.len()));
    match measure::run_case(corpus, frac, case, doc, args.warm, iters) {
        Ok(row) => Some(row),
        Err(reason) => {
            eprintln!("skip {corpus}/{}: {reason}", case.id);
            None
        }
    }
}

fn main() {
    let args = parse_args();
    let cases = mutate::cases();
    let find = |id: &str| cases.iter().find(|c| c.id == id).unwrap();

    // Document plan: synthetic sizes + encoding variants + adversarial.
    let mut docs: Vec<(String, String)> = Vec::new();
    for &size in &args.sizes {
        docs.push((
            format!("synth-{}", human_size(size)),
            synthetic(SynthOptions {
                target_bytes: size,
                cjk: false,
                crlf: false,
            }),
        ));
    }
    let var_size = args
        .sizes
        .iter()
        .copied()
        .min_by_key(|&s| s.abs_diff(10 * 1024))
        .unwrap_or(10 * 1024);
    for (label, cjk) in [("synth-cjk", true), ("synth-crlf", false)] {
        docs.push((
            label.to_string(),
            synthetic(SynthOptions {
                target_bytes: var_size,
                cjk,
                crlf: !cjk,
            }),
        ));
    }

    let adversarial_plan: &[(Adv, &[&str])] = &[
        (Adv::UnclosedFence, &["fence_body_char"]),
        (Adv::DeepQuote, &["quote_char", "quote_remove"]),
        (Adv::LazyContinuation, &["para_insert_char"]),
        (Adv::HugeParagraph, &["para_insert_char"]),
        (Adv::ManyRefs, &["ref_def_edit"]),
        (Adv::DuplicateRefs, &["ref_def_edit"]),
        (Adv::CrlfMixed, &["para_insert_char", "blank_delete"]),
        (Adv::HalfWritten, &["emphasis_close_completion"]),
        (Adv::FenceNearBof, &["fence_body_char", "fence_closer_delete"]),
    ];
    for (kind, _case_ids) in adversarial_plan {
        let (doc, label) = adversarial(kind.clone());
        docs.push((label.to_string(), doc));
    }

    let mut rows: Vec<measure::Row> = Vec::new();
    let mut skipped = 0usize;

    // Synthetic corpora: full battery (sweep cases × 5 positions).
    let synthetic_docs = docs.len() - adversarial_plan.len();
    for (label, doc) in docs.iter().take(synthetic_docs) {
        for case in &cases {
            let fracs: &[f64] = if case.sweep {
                &[0.0, 0.25, 0.5, 0.75, 1.0]
            } else {
                &[0.5]
            };
            for &f in fracs {
                if let Some(row) = first_successful_run(&args, label, f, case, doc) {
                    rows.push(row);
                } else {
                    skipped += 1;
                }
            }
        }
    }

    // Adversarial corpora: targeted case assignments.
    for (i, (_kind, case_ids)) in adversarial_plan.iter().enumerate() {
        let (label, doc) = {
            let idx = synthetic_docs + i;
            (docs[idx].0.clone(), docs[idx].1.clone())
        };
        for cid in *case_ids {
            let case = find(cid);
            if let Some(row) = first_successful_run(&args, &label, 0.5, case, &doc) {
                rows.push(row);
            } else {
                skipped += 1;
            }
        }
    }

    // Emit.
    if let Err(e) = std::fs::create_dir_all(&args.out) {
        eprintln!("cannot create output dir {}: {e}", args.out.display());
        std::process::exit(1);
    }
    let csv = args.out.join("cases.csv");
    if let Err(e) = emit::write_csv(&rows, &csv) {
        eprintln!("cannot write {}: {e}", csv.display());
        std::process::exit(1);
    }
    let summary = emit::summarize(&rows, skipped);
    std::fs::write(args.out.join("summary.md"), &summary).ok();
    std::fs::write(args.out.join("meta.txt"), metadata(&args)).ok();

    println!("{summary}");
    eprintln!("results in {}", args.out.display());
}

fn metadata(args: &Args) -> String {
    let mut m = String::new();
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    m.push_str(&format!("utc_epoch: {secs}\n"));
    m.push_str(&format!("args: sizes={:?} iters={:?} warm={} out={}\n", args.sizes, args.iters, args.warm, args.out.display()));
    m.push_str(&format!(
        "cargo: {}\n",
        Command::new("cargo")
            .arg("--version")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".into())
    ));
    m.push_str(&format!(
        "rustc: {}\n",
        Command::new("rustc")
            .arg("-vV")
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .find(|l| l.starts_with("release:"))
                    .unwrap_or("unknown")
                    .to_string()
            })
            .unwrap_or_else(|_| "unknown".into())
    ));
    m.push_str(&format!(
        "git_sha: {}\n",
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".into())
    ));
    m.push_str("measured_impl: markit-core MarkdownState (P0-02 incremental block index)\n");
    m
}
