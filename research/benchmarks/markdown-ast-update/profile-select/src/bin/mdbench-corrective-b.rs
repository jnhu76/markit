//! mdbench-corrective-b — CORRECTIVE-B driver (#35: profile + select).
//!
//! Usage (from `research/benchmarks/markdown-ast-update`):
//!
//! ```text
//! cargo run -p markit-mdbench-profile-select --bin mdbench-corrective-b -- run            # artifacts into workloads/
//! cargo run -p markit-mdbench-profile-select --bin mdbench-corrective-b -- run --dry      # print summaries only
//! cargo run -p markit-mdbench-profile-select --bin mdbench-corrective-b -- determinism    # §44 double-run check
//! cargo run -p markit-mdbench-profile-select --bin mdbench-corrective-b -- verify         # §9 only
//! ```
//!
//! Requires the PR #34 acquisition bytes to be materialized locally
//! (`python3 workloads/tools/acquire.py materialize`). No network, no
//! timing, no horse.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn bench_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate lives inside the benchmark root")
        .to_path_buf()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("run");
    let workloads_root = args
        .iter()
        .position(|arg| arg == "--workloads-root")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| bench_root().join("workloads"));
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let result = match command {
        "verify" => cmd_verify(&workloads_root),
        "run" => cmd_run(&workloads_root, &manifest_dir, args.iter().any(|arg| arg == "--dry")),
        "determinism" => cmd_determinism(&workloads_root, &manifest_dir),
        other => Err(format!("unknown command {other} (verify | run | determinism)")),
    };
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(" mdbench-corrective-b: {error}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_verify(workloads_root: &Path) -> Result<String, String> {
    let (identities, _) = markit_mdbench_profile_select::universe::load_identities(workloads_root)?;
    let verification =
        markit_mdbench_profile_select::universe::verify_universe(workloads_root, &identities)?;
    let summary = &verification.summary;
    if summary.hash_mismatches > 0 || summary.missing_files > 0 {
        return Err(format!(
            "candidate-universe verification FAILED: {} missing, {} hash mismatches",
            summary.missing_files, summary.hash_mismatches
        ));
    }
    Ok(format!(
        "VERIFY PASS: {} expected / {} materialized / 0 missing / 0 mismatches, {} bytes",
        summary.expected_files, summary.materialized_files, summary.byte_total_expected
    ))
}

fn cmd_run(workloads_root: &Path, manifest_dir: &Path, dry: bool) -> Result<String, String> {
    let scratch = std::env::temp_dir().join(format!(
        "corrective-b-scratch-{}",
        std::process::id()
    ));
    let artifacts = markit_mdbench_profile_select::artifacts::run_all(
        workloads_root,
        manifest_dir,
        &scratch,
    )?;
    print_summary(&artifacts);
    if dry {
        let _ = std::fs::remove_dir_all(&scratch);
        return Ok("DRY RUN: no artifacts written".to_string());
    }
    let written = markit_mdbench_profile_select::artifacts::write_all(
        workloads_root,
        &artifacts,
        true,
        &scratch.join("real-profile-v1.jsonl"),
    )?;
    let _ = std::fs::remove_dir_all(&scratch);
    Ok(format!("wrote {} artifacts", written.len()))
}

fn cmd_determinism(workloads_root: &Path, manifest_dir: &Path) -> Result<String, String> {
    let scratch_a = std::env::temp_dir().join(format!("corrective-b-det-a-{}", std::process::id()));
    let scratch_b = std::env::temp_dir().join(format!("corrective-b-det-b-{}", std::process::id()));
    let out_a = std::env::temp_dir().join(format!("corrective-b-out-a-{}", std::process::id()));
    let out_b = std::env::temp_dir().join(format!("corrective-b-out-b-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch_a);
    let _ = std::fs::remove_dir_all(&scratch_b);
    let _ = std::fs::remove_dir_all(&out_a);
    let _ = std::fs::remove_dir_all(&out_b);

    let artifacts_a = markit_mdbench_profile_select::artifacts::run_all(
        workloads_root, manifest_dir, &scratch_a,
    )?;
    let mut written_a = markit_mdbench_profile_select::artifacts::write_all(
        &out_a, &artifacts_a, true, &scratch_a.join("real-profile-v1.jsonl"),
    )?;
    let artifacts_b = markit_mdbench_profile_select::artifacts::run_all(
        workloads_root, manifest_dir, &scratch_b,
    )?;
    let written_b = markit_mdbench_profile_select::artifacts::write_all(
        &out_b, &artifacts_b, true, &scratch_b.join("real-profile-v1.jsonl"),
    )?;

    if written_a != written_b {
        return Err(format!(
            "artifact lists differ between runs: {:?} vs {:?}",
            written_a, written_b
        ));
    }
    let mut mismatches = Vec::new();
    for relative in &written_a {
        let a = std::fs::read(out_a.join(relative)).map_err(|e| e.to_string())?;
        let b = std::fs::read(out_b.join(relative)).map_err(|e| e.to_string())?;
        if a != b {
            mismatches.push(relative.clone());
        }
    }
    let _ = std::fs::remove_dir_all(&scratch_a);
    let _ = std::fs::remove_dir_all(&scratch_b);
    let _ = std::fs::remove_dir_all(&out_a);
    let _ = std::fs::remove_dir_all(&out_b);
    written_a.sort();
    if mismatches.is_empty() {
        Ok(format!(
            "DETERMINISM PASS: {} artifacts byte-identical across two clean runs",
            written_a.len()
        ))
    } else {
        Err(format!("DETERMINISM FAIL: differing artifacts: {mismatches:?}"))
    }
}

fn print_summary(artifacts: &markit_mdbench_profile_select::artifacts::Artifacts) {
    let summary = &artifacts.verification.summary;
    println!(
        "verification: {} expected / {} materialized / {} missing / {} mismatches",
        summary.expected_files, summary.materialized_files, summary.missing_files, summary.hash_mismatches
    );
    println!(
        "profiles: {} rows, {} failures, {} ambiguous + {} unknown facts; profile jsonl {} bytes sha256 {}…",
        artifacts.rows.len(),
        artifacts.rows.iter().filter(|row| row.profile_failure.is_some()).count(),
        artifacts.rows.iter().filter_map(|row| row.g1()).map(|lane| lane.ambiguous_kinds.values().sum::<u64>()).sum::<u64>(),
        artifacts.rows.iter().filter_map(|row| row.g1()).map(|lane| lane.unknown_kinds.values().sum::<u64>()).sum::<u64>(),
        artifacts.profile_jsonl_bytes,
        &artifacts.profile_jsonl_sha256[..12.min(artifacts.profile_jsonl_sha256.len())]
    );
    for class in &artifacts.bias.classes {
        println!(
            "eligibility {}: strict {}/{} blocker-bearing {}",
            class.grammar_id, class.strict_scope_clean, class.all_candidates, class.realism_only_blocker_bearing
        );
    }
    println!(
        "redundancy: {} exact groups, {} near groups",
        artifacts.redundancy.exact_groups.len(),
        artifacts.redundancy.near_groups.len()
    );
    let counts = &artifacts.coverage.logical_membership_counts;
    println!(
        "sets: representative={} extremal={} syntax={} full={} unique={}",
        counts.get("REPRESENTATIVE_SET").copied().unwrap_or(0),
        counts.get("EXTREMAL_SET").copied().unwrap_or(0),
        counts.get("SYNTAX_COVERAGE_SET").copied().unwrap_or(0),
        counts.get("FULL_DOCUMENT_SET").copied().unwrap_or(0),
        artifacts.coverage.unique_physical_files
    );
    if let Some(note) = &artifacts.outcome.plateau_note {
        println!("note: {note}");
    }
    println!("spot checks: all_pass={}", artifacts.spot_check.all_pass);
}
