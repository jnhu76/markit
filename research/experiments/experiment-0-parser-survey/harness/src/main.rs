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
mod baseline;
mod cmoracle;
mod corpus;
mod emit;
mod green;
mod greenbench;
mod influence;
mod lezer;
mod md4c;
mod measure;
mod mutate;
mod refbench;
mod refindex;
mod text;
mod tsitter;

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::corpus::{adversarial, human_size, synthetic, Adv, SynthOptions};

struct Args {
    sizes: Vec<usize>,
    iters: Option<usize>,
    warm: usize,
    out: PathBuf,
    commonmark: Option<PathBuf>,
    md4c: bool,
    ts: bool,
    ts_deep200: bool,
    lezer: bool,
    green: bool,
    green_history: bool,
    refs: bool,
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
        commonmark: None,
        md4c: false,
        ts: false,
        ts_deep200: false,
        lezer: false,
        green: false,
        green_history: false,
        refs: false,
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
            "--commonmark" => {
                args.commonmark = it.next().map(PathBuf::from);
            }
            "--md4c" => args.md4c = true,
            "--ts" => args.ts = true,
            "--ts-deep200" => args.ts_deep200 = true,
            "--lezer" => args.lezer = true,
            "--green" => args.green = true,
            "--green-history" => args.green_history = true,
            "--refs" => args.refs = true,
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

    // RUN-4 mode: ReferenceIndex semantic dependency experiment.
    if args.refs {
        let summary = refbench::run(&args.out.join("refs")).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        println!("{summary}");
        eprintln!("results in {}", args.out.join("refs").display());
        return;
    }

    // CORRECTIVE-1 mode: G2 edit-history stability gate (review MAJOR-2).
    if args.green_history {
        let summary =
            greenbench::history_run(&args.out.join("green-history")).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
        println!("{summary}");
        eprintln!("results in {}", args.out.join("green-history").display());
        return;
    }

    // RUN-3 mode: green-tree representation prototype.
    if args.green {
        let summary = greenbench::run(&args.out.join("green")).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        println!("{summary}");
        eprintln!("results in {}", args.out.join("green").display());
        return;
    }

    // RUN-2c mode: Lezer Markdown baseline (persistent node worker).
    if args.lezer {
        let summary = lezer::run(&args.out.join("lezer")).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        println!("{summary}");
        eprintln!("results in {}", args.out.join("lezer").display());
        return;
    }

    // RUN-2b mode: tree-sitter markdown incremental baseline.
    if args.ts_deep200 {
        let summary = tsitter::run_deep200(&args.out.join("ts")).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        println!("{summary}");
        return;
    }
    if args.ts {
        let summary = tsitter::run(&args.out.join("ts")).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        println!("{summary}");
        eprintln!("results in {}", args.out.join("ts").display());
        return;
    }

    // M0-B mode: MD4C full-parse baseline (sizes sweep + CommonMark
    // cross-check).
    if args.md4c {
        let spec = args
            .commonmark
            .clone()
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/commonmark-0.31.2-spec.json")
            });
        let summary = baseline::run(&spec, &args.out.join("md4c")).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        println!("{summary}");
        eprintln!("results in {}", args.out.join("md4c").display());
        return;
    }

    // ORACLE-B mode: CommonMark dialect-semantics battery over a pinned
    // spec.json (no mutation battery).
    if let Some(spec) = args.commonmark.as_ref() {
        let summary = cmoracle::run(
            spec,
            &args
                .out
                .join("oracle-b"),
        )
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        println!("{summary}");
        eprintln!("results in {}", args.out.join("oracle-b").display());
        return;
    }

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
        (Adv::Emoji, &["para_insert_char", "eof_append_char"]),
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
