//! Run the SEMANTIC_PILOT_SET and emit the pilot manifest and results.
//!
//! Usage (from `research/benchmarks/markdown-ast-update`):
//!
//! ```text
//! cargo run -p markit-mdbench-semantics --bin mdbench-semantic-pilot            # summary only
//! cargo run -p markit-mdbench-semantics --bin mdbench-semantic-pilot -- --write # write artifacts
//! ```
//!
//! Written artifacts (canonical JSON, byte-reproducible):
//!
//! ```text
//! workloads/pilots/semantic-pilot-manifest-v1.json
//! workloads/pilots/semantic-pilot-results-v1.json
//! ```
//!
//! This pilot validates contracts; it does not select a workload, freeze a
//! corpus, or measure anything.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use markit_mdbench_semantics::artifacts::pilot_artifacts;
use markit_mdbench_semantics::pilot::{
    load_fixtures, real_source_spec, run_pilot, PilotFixture, PilotResults, PILOT_SET_ID,
};

fn bench_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate lives inside the benchmark root")
        .to_path_buf()
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let write = args.iter().any(|arg| arg == "--write");
    let root = args
        .iter()
        .position(|arg| arg == "--bench-root")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .unwrap_or_else(bench_root);

    let fixtures: Vec<PilotFixture> =
        load_fixtures(&root).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let real_source = real_source_spec(&root);
    let results = run_pilot(&root, &fixtures, real_source.as_ref());

    print_summary(&results);

    if write {
        // The artifacts are rendered by the library so the committed outputs
        // and the drift guard cannot diverge from what the driver produced.
        let artifacts = pilot_artifacts(&root)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        for (relative, contents) in artifacts {
            let path = root.join(relative);
            fs::write(&path, contents)?;
            println!("wrote {}", path.display());
        }
    }

    if !results.pass {
        std::process::exit(1);
    }
    Ok(())
}

fn print_summary(results: &PilotResults) {
    println!("=== {PILOT_SET_ID} ===");
    for fixture in &results.fixtures {
        println!(
            "[{}] {} ({}) lanes={:?}",
            if fixture.pass { "PASS" } else { "FAIL" },
            fixture.id,
            fixture.name,
            fixture.lanes
        );
        for check in &fixture.checks {
            if !check.passed {
                println!("    {} {} — {}", check.status(), check.check, check.detail);
            }
        }
    }
    match &results.real_source {
        Some(real) => {
            println!(
                "[{}] real source {} (available={})",
                if real.pass { "PASS" } else { "FAIL" },
                real.spec.source_id,
                real.available
            );
            for check in &real.checks {
                println!("    {} {} — {}", check.status(), check.check, check.detail);
            }
            for profile in &real.profiles {
                println!(
                    "    lane {} strict_scope_clean={} blocks={} depth={} tables={} strict_facts={}",
                    profile.lane_id,
                    profile.strict_scope_clean,
                    profile.structural.block_count,
                    profile.structural.max_container_depth,
                    profile
                        .structural
                        .table
                        .as_ref()
                        .map(|table| table.table_count)
                        .unwrap_or(0),
                    profile.recognized_strict_count
                );
            }
        }
        None => println!("[SKIP] no real-source case declared"),
    }
    println!(
        "pilot verdict: {}",
        if results.pass { "PASS" } else { "FAIL" }
    );
}
