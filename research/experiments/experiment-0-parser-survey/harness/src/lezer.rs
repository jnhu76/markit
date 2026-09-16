//! RUN-2c — Lezer Markdown baseline (issue #19 plan §11).
//!
//! The most important Markdown-specific baseline: Lezer reuses BLOCK-
//! LEVEL fragments via per-block hashes (its fragment cursor matches
//! fragments by block hash, not generic subtree equality).
//!
//! Plan §11 timing rule, implemented literally:
//! - the Node process is spawned ONCE and stays warm;
//! - the JS worker times the parser inside the runtime and returns the
//!   aggregate (`native_us`), never through a wall-clock taken in Rust;
//! - transport (`ipc_us` = Rust wall − worker native) is reported
//!   separately and never merged into the parser number;
//! - fragment adjustment (`TreeFragment.applyChanges`) is timed inside
//!   JS too (`adjust_us`) — it is the representation-maintenance step.
//!
//! Pinned: @lezer/markdown 1.7.2, @lezer/common 1.5.2 (scripts/
//! package.json), Node runtime recorded at run time.
//!
//! Limitations (stated, not hidden): Lezer exposes neither changed
//! ranges nor node-reuse counters, so reuse evidence is the
//! inc-vs-full native-time ratio on the same document plus the
//! observable fragment flow; no fabricated reuse numbers.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Instant;

use serde_json::json;

use crate::corpus::{adversarial, synthetic, Adv, SynthOptions};
use crate::mutate;
use crate::text::SurveyText;

struct Worker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, r#"{{"cmd":"stop"}}"#);
        let _ = self.child.wait();
    }
}

fn find_node() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("LEZER_NODE") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let c = Path::new(dir).join("node");
            if c.is_file() {
                return Ok(c);
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let nvm = Path::new(&home).join(".nvm/versions/node");
        let mut vers: Vec<_> = std::fs::read_dir(&nvm)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.join("bin/node").is_file())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        vers.sort();
        if let Some(latest) = vers.last() {
            return Ok(latest.join("bin/node"));
        }
    }
    Err("no node executable found (set LEZER_NODE)".into())
}

impl Worker {
    fn spawn() -> Result<Worker, String> {
        let node = find_node()?;
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/lezer-worker.js");
        let mut child = Command::new(&node)
            .arg("--expose-gc")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn node ({}): {e}", node.display()))?;
        let stdin = child.stdin.take().ok_or("worker stdin")?;
        let stdout = child.stdout.take().ok_or("worker stdout")?;
        Ok(Worker {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// One request/response; `Err` means the worker died or misbehaved.
    fn request(&mut self, msg: &serde_json::Value) -> Result<serde_json::Value, String> {
        writeln!(self.stdin, "{msg}").map_err(|e| format!("worker write: {e}"))?;
        self.stdin.flush().ok();
        let mut line = String::new();
        let n = self
            .stdout
            .read_line(&mut line)
            .map_err(|e| format!("worker read: {e}"))?;
        if n == 0 {
            return Err("worker died (EOF)".into());
        }
        serde_json::from_str(&line).map_err(|e| format!("worker reply: {e}"))
    }
}

struct LezerRow {
    corpus: String,
    case_id: String,
    position: String,
    doc_bytes: u64,
    changed_bytes: u64,
    full_native_us: f64,
    inc_native_us: f64,
    adjust_native_us: f64,
    ipc_us: f64,
    node_count: u64,
    worker_died: bool,
}

fn scenario(
    worker: &mut Worker,
    corpus: &str,
    case_id: &str,
    position: &str,
    doc: &str,
    case: &mutate::Case,
    frac: f64,
) -> Result<LezerRow, String> {
    let survey = SurveyText::new(doc.to_string());
    let edit = (case.build)(&survey, frac)
        .ok_or_else(|| format!("no anchor for {case_id}"))?;
    let s = edit.range.start.as_usize();
    let e = edit.range.end.as_usize();
    let new_doc = format!("{}{}{}", &doc[..s], edit.new_text, &doc[e..]);

    let iters = 5;
    let t_wall = Instant::now();
    let full = worker
        .request(&json!({"cmd":"full","doc":new_doc,"iters":iters}))
        .map_err(|e| format!("full: {e}"))?;
    let inc = worker
        .request(&json!({
            "cmd":"inc",
            "old_doc": doc,
            "new_doc": new_doc,
            "fromA": s, "toA": e,
            "fromB": s, "toB": s + edit.new_text.len(),
            "iters": iters
        }))
        .map_err(|e| format!("inc: {e}"));
    let wall_us = t_wall.elapsed().as_secs_f64() * 1e6;
    let inc = inc?;

    if !full["ok"].as_bool().unwrap_or(false) || !inc["ok"].as_bool().unwrap_or(false) {
        return Err(format!(
            "worker error: {} {}",
            full["error"], inc["error"]
        ));
    }
    let native_full = full["native_us"].as_f64().unwrap_or(0.0);
    let native_inc = inc["native_us"].as_f64().unwrap_or(0.0);
    Ok(LezerRow {
        corpus: corpus.into(),
        case_id: case_id.into(),
        position: position.into(),
        doc_bytes: new_doc.len() as u64,
        changed_bytes: (edit.new_text.len() + e - s) as u64,
        full_native_us: native_full,
        inc_native_us: native_inc,
        adjust_native_us: inc["adjust_us"].as_f64().unwrap_or(0.0),
        ipc_us: wall_us - native_full - native_inc,
        node_count: inc["node_count"].as_u64().unwrap_or(0),
        worker_died: false,
    })
}

pub fn run(out: &Path) -> Result<String, String> {
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
    let (deep_quote, quote_label) = adversarial(Adv::DeepQuote);

    let plan: Vec<(&'static str, &'static str, &'static str, String, &'static str, f64)> = vec![
        ("synth-100k", "para_insert_char", "bof", synth_100k.clone(), "para_insert_char", 0.0),
        ("synth-100k", "para_insert_char", "mid", synth_100k.clone(), "para_insert_char", 0.5),
        ("synth-100k", "para_insert_char", "eof", synth_100k.clone(), "para_insert_char", 1.0),
        ("synth-100k", "para_sub_char", "mid", synth_100k.clone(), "para_sub_char", 0.5),
        ("synth-100k", "blank_delete", "mid", synth_100k.clone(), "blank_delete", 0.5),
        ("synth-100k", "fence_len_grow", "mid", synth_100k.clone(), "fence_len_grow", 0.5),
        ("synth-1m", "para_insert_char", "bof", synth_1m.clone(), "para_insert_char", 0.0),
        ("synth-1m", "para_insert_char", "mid", synth_1m.clone(), "para_insert_char", 0.5),
        ("synth-1m", "para_insert_char", "eof", synth_1m.clone(), "para_insert_char", 1.0),
        (huge_label, "para_insert_char", "mid", huge_para.clone(), "para_insert_char", 0.5),
        (quote_label, "quote_char", "mid", deep_quote.clone(), "quote_char", 0.5),
    ];

    let mut worker = Worker::spawn()?;
    let mut rows: Vec<LezerRow> = Vec::new();
    for (corpus, case_id, position, doc, case_id2, frac) in &plan {
        let case = mutate::cases()
            .into_iter()
            .find(|c| c.id == *case_id2)
            .ok_or("case missing")?;
        match scenario(&mut worker, corpus, case_id, position, &doc, &case, *frac) {
            Ok(r) => rows.push(r),
            Err(reason) => {
                // Worker death on a scenario is a recorded finding;
                // respawn and continue with the rest.
                eprintln!("scenario {corpus}/{case_id} failed: {reason}");
                rows.push(LezerRow {
                    corpus: corpus.to_string(),
                    case_id: case_id.to_string(),
                    position: position.to_string(),
                    doc_bytes: doc.len() as u64,
                    changed_bytes: 0,
                    full_native_us: 0.0,
                    inc_native_us: 0.0,
                    adjust_native_us: 0.0,
                    ipc_us: 0.0,
                    node_count: 0,
                    worker_died: true,
                });
                worker = Worker::spawn()?;
            }
        }
    }
    let node_version = find_node()
        .ok()
        .and_then(|p| {
            Command::new(p)
                .arg("--version")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into());

    std::fs::create_dir_all(out).map_err(|e| format!("mkdir: {e}"))?;
    let csv = out.join("lezer.csv");
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(&csv).map_err(|e| format!("create: {e}"))?,
    );
    writeln!(
        f,
        "corpus,case_id,position,doc_bytes,changed_bytes,full_native_us,inc_native_us,adjust_native_us,ipc_us,node_count,worker_died"
    )
    .map_err(|e| format!("write: {e}"))?;
    for r in &rows {
        writeln!(
            f,
            "{},{},{},{},{},{:.1},{:.1},{:.1},{:.1},{},{}",
            r.corpus,
            r.case_id,
            r.position,
            r.doc_bytes,
            r.changed_bytes,
            r.full_native_us,
            r.inc_native_us,
            r.adjust_native_us,
            r.ipc_us,
            r.node_count,
            r.worker_died,
        )
        .map_err(|e| format!("write: {e}"))?;
    }
    f.flush().ok();
    drop(f);

    let mut s = String::new();
    s.push_str("# RUN-2c — Lezer Markdown baseline (plan §11)\n\n");
    s.push_str(&format!(
        "@lezer/markdown **1.7.2** + @lezer/common **1.5.2**, Node {node_version} \
        (persistent worker, plan §11 timing rule: native times measured inside JS; \
        fragment adjustment timed separately; Rust wall-clock reported only as \
        transport and never merged into parser numbers).\n\n\
        | corpus | case | pos | doc_B | chg_B | full_native | inc_native | adjust_native | inc/full | ipc_wall |\n\
        |---|---|---|---:|---:|---:|---:|---:|---:|---:|\n"
    ));
    for r in &rows {
        let inc_full = if r.worker_died || r.full_native_us <= 0.0 {
            String::from("-")
        } else {
            format!("{:.2}×", r.inc_native_us / r.full_native_us)
        };
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.1} | {:.1} | {:.1} | {} | {:.1} |\n",
            r.corpus,
            r.case_id,
            r.position,
            r.doc_bytes,
            r.changed_bytes,
            r.full_native_us,
            r.inc_native_us,
            r.adjust_native_us,
            inc_full,
            r.ipc_us,
        ));
    }
    s.push_str("\nNotes: times are µs medians inside the JS runtime (5 measured iterations, \
        1 warm-up, GC between loops). `inc/full` = incremental parse with block-hash \
        fragments over a cold full parse of the SAME post-edit document. Lezer exposes \
        no changed-range/node-reuse counters — reuse evidence is the ratio and the \
        fragment flow (addTree → applyChanges → parse(new, fragments)); no fabricated \
        reuse numbers (plan §10/§12). `ipc_wall` includes ~2 MB of JSON transport for \
        1 MB corpora and is NOT a parser metric.\n");
    std::fs::write(out.join("summary.md"), &s).map_err(|e| format!("write: {e}"))?;
    Ok(s)
}
