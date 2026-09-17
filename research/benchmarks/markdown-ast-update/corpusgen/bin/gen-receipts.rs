//! gen-receipts — materialize (or verify) the 24 CORPUS-v1 generation
//! receipts under `corpus/receipt-<corpus_id>.toml`.
//!
//! Write mode (first generation): regenerates every corpus, hashes it, and
//! writes the frozen-schema receipt files. No corpus bytes are written.
//!
//! Check mode (`--check`): regenerates every corpus, re-renders each
//! receipt, and requires byte equality with the committed file — proving
//! generation determinism end to end without storing any payload.

use std::path::PathBuf;
use std::process::ExitCode;

use markit_mdbench_corpusgen::receipt;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let check = args.iter().any(|a| a == "--check");
    let dir = match args.iter().position(|a| a == "--corpus-dir") {
        Some(i) => PathBuf::from(&args[i + 1]),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus"),
    };

    let receipts = match receipt::all_receipts() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("generation failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut failures = 0;
    for r in &receipts {
        let path = dir.join(format!("receipt-{}.toml", r.corpus_id()));
        let rendered = r.render();
        if check {
            match std::fs::read(&path) {
                Ok(existing) if existing == rendered.as_bytes() => {
                    println!("ok   {}", r.corpus_id());
                }
                Ok(_) => {
                    eprintln!("DRIFT {} ({})", r.corpus_id(), path.display());
                    failures += 1;
                }
                Err(e) => {
                    eprintln!("MISSING {} ({e})", path.display());
                    failures += 1;
                }
            }
        } else {
            if let Err(e) = std::fs::write(&path, &rendered) {
                eprintln!("write failed for {}: {e}", path.display());
                failures += 1;
                continue;
            }
            println!(
                "wrote {} ({} bytes, sha256 {})",
                path.display(),
                r.actual_bytes,
                r.source_sha256
            );
        }
    }

    if failures > 0 {
        eprintln!("{failures} receipt(s) failed");
        return ExitCode::FAILURE;
    }
    if check {
        println!("R4 CORPUS RECEIPTS: all 24 verified byte-identical");
    } else {
        println!("R4 CORPUS RECEIPTS: wrote 24 receipts");
    }
    ExitCode::SUCCESS
}
