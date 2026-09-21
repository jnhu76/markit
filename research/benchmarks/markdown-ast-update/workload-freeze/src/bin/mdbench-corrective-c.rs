//! mdbench-corrective-c — the CORRECTIVE-C workload-freeze tool.
//!
//! Subcommands (run from the benchmark root or pass its path):
//!
//! ```text
//! generate       build all frozen workload artifacts from the frozen
//!                selection + sources (A6 + A7 + coverage + receipt)
//! verify         re-validate every frozen artifact against the sources
//!                and the registry (INVALID_PAYLOAD guard, digests, hashes)
//! determinism    generate twice into temp dirs; require byte-identical
//! dry-run        A8 correctness-only harness dispatch (H0-H4; no timing)
//! profile-export MEASUREMENT-CORRECTIVE-1 §23: mechanism-neutral G0
//!                structural facts of the frozen G0-strict surface
//!                (workloads/profiles/strict-surface-profile-v1.jsonl);
//!                the strict set comes from the frozen manifest — never
//!                reselected
//! ```

use std::path::{Path, PathBuf};

use markit_mdbench_semantics::{canonical_json_line, lane_profile};
use markit_mdbench_workload_freeze::{
    acquisition_commit_sha, applicability, artifacts, coverage, dryrun, fullread,
    load_selected_files, registry, repair, traces, write_jsonl,
};

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() < 2 {
        usage();
    }
    let command = arguments[1].as_str();
    let root: PathBuf = arguments
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let code = match command {
        "generate" => generate(&root),
        "verify" => verify(&root),
        "determinism" => determinism(&root),
        "dry-run" => dry_run(&root),
        "profile-export" => profile_export(&root),
        _ => {
            usage();
        }
    };
    std::process::exit(code);
}

fn usage() -> ! {
    eprintln!(
        "usage: mdbench-corrective-c <generate|verify|determinism|dry-run|profile-export> [benchmark_root]"
    );
    std::process::exit(2);
}

fn commit_shas(root: &Path) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    for name in [
        "cpp-core-guidelines",
        "crafting-interpreters",
        "cs231n",
        "d2l-en",
        "ethereum-eips",
        "kubernetes-keps",
        "myst-parser",
        "oci-image",
        "openmlsys",
        "owasp-cheatsheets",
        "rust-book",
        "rust-rfcs",
        "swift-evolution",
    ] {
        if let Ok(sha) = acquisition_commit_sha(root, name) {
            map.insert(name.to_string(), sha);
        }
    }
    map
}

fn generate(root: &Path) -> i32 {
    match generate_inner(root, Some(Path::new("."))) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(error) => {
            eprintln!("GENERATE_FAILED: {error}");
            1
        }
    }
}

fn generate_inner(root: &Path, out_dir: Option<&Path>) -> Result<String, String> {
    let files = load_selected_files(root)?;
    let shas = commit_shas(root);
    let profiles = applicability::build_profile_map(&files);
    let workload = applicability::build_frozen_workload(&files, &shas, &profiles)?;

    // Set-level syntax-coverage check: no required transition may end
    // without any applicable cell anywhere (repair decision input).
    // Restore legs are reached through their BREAK entries; they are not
    // independent cells.
    let uncovered = coverage::uncovered_transitions(&workload);
    if !uncovered.is_empty() {
        if std::env::var("C3_DEBUG_UNCOVERED").is_ok() {
            for row in &workload.rows {
                if uncovered.contains(&row.transition_id)
                    && row.source_key.ends_with("0404-change-prefer-dynamic.md")
                    || row.status != applicability::STATUS_APPLICABLE
                        && uncovered.contains(&row.transition_id)
                {
                    eprintln!(
                        "UNCOVERED_CELL {} {} {} anchors={} rejections={:?}",
                        row.transition_id,
                        row.source_key,
                        row.status,
                        row.candidate_anchor_count,
                        &row.rejections[..row.rejections.len().min(4)],
                    );
                }
            }
        }
        return Err(format!(
            "REQUIRED_CELL_UNCOVERED: no applicable cell anywhere for {uncovered:?}; \
             syntax-coverage repair evaluation required before freeze"
        ));
    }

    let full_read = fullread::build_full_read(&files, &profiles, &shas);
    let trace_records = traces::build_traces(&workload.payloads);
    // Surface the deterministic SYNTAX_COVERAGE_SET repair (if one is
    // recorded) in the final coverage report.
    let repairs = match repair::load_syntax_coverage_repair(root)? {
        Some(record) => vec![format!(
            "{}: added {} to SYNTAX_COVERAGE_SET ({})",
            repair::SYNTAX_COVERAGE_REPAIR_SCHEMA,
            record.selected_key,
            record
                .basis
                .uncovered_cell
                .get("finding")
                .and_then(|value| value.as_str())
                .unwrap_or("required cell uncovered"),
        )],
        None => Vec::new(),
    };
    let report = coverage::build_coverage(&workload, &files, &full_read, repairs);

    let payloads_dir = match out_dir {
        Some(dir) => dir.join("workloads/payloads"),
        None => root.join("workloads/payloads"),
    };
    let mut digests: Vec<(String, String)> = Vec::new();

    let registry_json = markit_mdbench_semantics::canonical_json(&registry::registry_artifact());
    digests.push((
        "transition-registry-v1.json".to_string(),
        artifacts::write_artifact(&payloads_dir, "transition-registry-v1.json", &registry_json)?,
    ));
    let matrix = matrix_text(&workload.rows);
    digests.push((
        "applicability-matrix-v1.jsonl".to_string(),
        artifacts::write_artifact(&payloads_dir, "applicability-matrix-v1.jsonl", &matrix)?,
    ));
    let full_read_text = full_read
        .iter()
        .map(|record| markit_mdbench_semantics::canonical_json_line(record))
        .collect::<String>();
    digests.push((
        "full-read-manifest-v1.jsonl".to_string(),
        artifacts::write_artifact(
            &payloads_dir,
            "full-read-manifest-v1.jsonl",
            &full_read_text,
        )?,
    ));
    let payloads_text = workload
        .payloads
        .iter()
        .map(|payload| markit_mdbench_semantics::canonical_json_line(payload))
        .collect::<String>();
    digests.push((
        "edit-write-manifest-v1.jsonl".to_string(),
        artifacts::write_artifact(
            &payloads_dir,
            "edit-write-manifest-v1.jsonl",
            &payloads_text,
        )?,
    ));
    let trace_text = trace_records
        .iter()
        .map(|record| markit_mdbench_semantics::canonical_json_line(record))
        .collect::<String>();
    digests.push((
        "trace-manifest-v1.jsonl".to_string(),
        artifacts::write_artifact(&payloads_dir, "trace-manifest-v1.jsonl", &trace_text)?,
    ));
    let coverage_json = markit_mdbench_semantics::canonical_json(&report);
    digests.push((
        "coverage-final-v1.json".to_string(),
        artifacts::write_artifact(&payloads_dir, "coverage-final-v1.json", &coverage_json)?,
    ));

    let receipt = artifacts::freeze_receipt(root, &digests, &workload, &full_read, &report);
    let receipt_text = markit_mdbench_semantics::canonical_json(&receipt);
    digests.push((
        "freeze-receipt-v1.json".to_string(),
        artifacts::write_artifact(&payloads_dir, "freeze-receipt-v1.json", &receipt_text)?,
    ));

    Ok(format!(
        "GENERATE_OK files={} matrix_rows={} full_read={} edit_write={} break_restore_pairs={}",
        files.len(),
        workload.rows.len(),
        full_read.len(),
        workload.payloads.len(),
        workload.break_restore_reports.len(),
    ))
}

fn matrix_text(rows: &[applicability::MatrixRow]) -> String {
    artifacts::matrix_lines(rows).concat()
}

fn verify(root: &Path) -> i32 {
    match verify_inner(root) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(error) => {
            eprintln!("VERIFY_FAILED: {error}");
            1
        }
    }
}

/// Re-validate every frozen artifact against the sources + registry.
fn verify_inner(root: &Path) -> Result<String, String> {
    let files = load_selected_files(root)?;
    let payloads_dir = root.join("workloads/payloads");

    let payloads: Vec<markit_mdbench_semantics::payload::PayloadRecord> =
        dryrun::read_jsonl(&payloads_dir.join("edit-write-manifest-v1.jsonl"))?;
    let full_read: Vec<fullread::FullReadRecord> =
        dryrun::read_jsonl(&payloads_dir.join("full-read-manifest-v1.jsonl"))?;

    let registry_entries = registry::transition_registry_v1();
    let mut verified = 0usize;
    for payload in &payloads {
        let source = files
            .iter()
            .find(|file| file.key == payload.source_path)
            .ok_or_else(|| format!("payload {}: unknown source", payload.payload_id))?;
        let pre_source = if payload.step == 0 {
            source.text.clone()
        } else {
            let step0 = payloads
                .iter()
                .find(|candidate| candidate.trace_id == payload.trace_id && candidate.step == 0)
                .ok_or_else(|| format!("payload {}: trace has no step 0", payload.payload_id))?;
            if step0.base_source_sha256 != source.sha256 {
                return Err(format!(
                    "payload {}: trace base disagrees with source identity",
                    payload.payload_id
                ));
            }
            step0.edit.apply(&source.text).map_err(|error| {
                format!("payload {}: broken state: {error:?}", payload.payload_id)
            })?
        };
        let validation =
            markit_mdbench_semantics::validate_payload(payload, &pre_source, &source.text);
        if !validation.valid {
            return Err(format!(
                "payload {}: {:?}",
                payload.payload_id, validation.failure_codes
            ));
        }
        let entry = registry::registry_entry(&registry_entries, &payload.expected_transition)
            .ok_or_else(|| {
                format!(
                    "payload {}: unknown transition {}",
                    payload.payload_id, payload.expected_transition
                )
            })?;
        registry::payload_agrees_with_registry(
            &payload.grammar_id,
            payload.syntax_target.name(),
            &payload.expected_transition,
            &payload.edit_family,
            &payload.operation_variant,
            &payload.expected_pre,
            &payload.expected_post,
            entry,
        )
        .map_err(|error| format!("payload {}: {error}", payload.payload_id))?;
        verified += 1;
    }

    // FULL_READ identities must match the sources byte-for-byte.
    let mut full_read_verified = 0usize;
    for record in &full_read {
        let source = files
            .iter()
            .find(|file| file.key == record.source_key)
            .ok_or_else(|| format!("full-read {}: unknown source", record.source_key))?;
        if record.source_sha256 != source.sha256 || record.file_bytes != source.file_bytes {
            return Err(format!(
                "full-read {}: identity disagrees with materialized bytes",
                record.source_key
            ));
        }
        full_read_verified += 1;
    }

    // Receipt digests must match the artifacts on disk.
    let receipt_raw = std::fs::read_to_string(payloads_dir.join("freeze-receipt-v1.json"))
        .map_err(|error| format!("read freeze receipt: {error}"))?;
    let receipt: serde_json::Value = serde_json::from_str(&receipt_raw)
        .map_err(|error| format!("parse freeze receipt: {error}"))?;
    for entry in receipt["artifact_sha256"]
        .as_array()
        .ok_or("receipt artifact list")?
    {
        let name = entry["artifact"].as_str().ok_or("artifact name")?;
        let digest = entry["sha256"].as_str().ok_or("artifact digest")?;
        let on_disk = std::fs::read_to_string(payloads_dir.join(name))
            .map_err(|error| format!("receipt artifact {name}: {error}"))?;
        let actual = markit_mdbench_semantics::sha256_hex(on_disk.as_bytes());
        if actual != digest {
            return Err(format!(
                "receipt digest mismatch for {name}: {actual} != {digest}"
            ));
        }
    }

    Ok(format!(
        "VERIFY_OK payloads={verified} full_read_records={full_read_verified} \
         receipt_artifacts={}",
        receipt["artifact_sha256"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0)
    ))
}

fn determinism(root: &Path) -> i32 {
    let temp = std::env::temp_dir().join("mdbench-corrective-c-determinism");
    let _ = std::fs::remove_dir_all(&temp);
    let run_a = temp.join("a");
    let run_b = temp.join("b");
    std::fs::create_dir_all(&run_a).expect("temp a");
    std::fs::create_dir_all(&run_b).expect("temp b");
    if let Err(error) = generate_inner(root, Some(&run_a)) {
        eprintln!("DETERMINISM_FAILED(first run): {error}");
        return 1;
    }
    if let Err(error) = generate_inner(root, Some(&run_b)) {
        eprintln!("DETERMINISM_FAILED(second run): {error}");
        return 1;
    }
    let dir_a = run_a.join("workloads/payloads");
    let dir_b = run_b.join("workloads/payloads");
    let mut names: Vec<String> = std::fs::read_dir(&dir_a)
        .expect("artifact dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    let mut identical = true;
    for name in &names {
        let a = std::fs::read(dir_a.join(name)).expect("artifact a");
        let b = std::fs::read(dir_b.join(name)).expect("artifact b");
        if a != b {
            eprintln!("DETERMINISM_DRIFT: {name}");
            identical = false;
        }
    }
    if identical {
        println!("DETERMINISM_OK artifacts={} byte-identical", names.len());
        0
    } else {
        1
    }
}

fn dry_run(root: &Path) -> i32 {
    match dry_run_inner(root) {
        Ok((summary, rows)) => {
            let payloads_dir = root.join("workloads/payloads");
            let lines = rows
                .iter()
                .map(|row| markit_mdbench_semantics::canonical_json_line(row))
                .collect::<String>();
            if write_jsonl(&payloads_dir.join("dry-run-cases-v1.jsonl"), &[]).is_err() {
                return 1;
            }
            std::fs::write(payloads_dir.join("dry-run-cases-v1.jsonl"), lines)
                .expect("write dry-run cases");
            let report_text = markit_mdbench_semantics::canonical_json(&summary);
            std::fs::write(payloads_dir.join("dry-run-report-v1.json"), report_text)
                .expect("write dry-run report");
            println!("{summary_text}", summary_text = summarize(&summary));
            if summary
                .edit_write
                .per_horse
                .values()
                .any(|counts| counts.wrong_result > 0 || counts.execution_failed > 0)
                || summary.full_read.failed > 0
            {
                eprintln!("DRY_RUN_CORRECTNESS_FAILURES_PRESENT");
                return 1;
            }
            println!("DRY_RUN_OK correctness-only; no timing was performed");
            0
        }
        Err(error) => {
            eprintln!("DRY_RUN_FAILED: {error}");
            1
        }
    }
}

fn summarize(report: &dryrun::DryRunReport) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    let _ = writeln!(
        text,
        "DRY_RUN_REPORT full_read (clean parse + native-state construction): files={} g0_strict={} horse_dispatches={} pass={} failed={}",
        report.full_read.files,
        report.full_read.g0_strict_cases,
        report.full_read.horse_dispatches,
        report.full_read.pass,
        report.full_read.failed
    );
    for (horse, counts) in &report.full_read.per_horse {
        let _ = writeln!(
            text,
            "DRY_RUN_REPORT full_read horse {horse}: pass={} wrong_result={} execution_failed={}",
            counts.pass, counts.wrong_result, counts.execution_failed
        );
    }
    let _ = writeln!(
        text,
        "DRY_RUN_REPORT edit_write: g0_cases={} dispatches={} g1_semantic_skipped={}",
        report.edit_write.g0_cases,
        report.edit_write.horse_dispatches,
        report.edit_write.g1_semantic_skipped
    );
    for (horse, counts) in &report.edit_write.per_horse {
        let _ = writeln!(
            text,
            "DRY_RUN_REPORT horse {horse}: pass={} wrong_result={} execution_failed={}",
            counts.pass, counts.wrong_result, counts.execution_failed
        );
    }
    text
}

fn dry_run_inner(
    root: &Path,
) -> Result<(dryrun::DryRunReport, Vec<dryrun::DryRunCaseRow>), String> {
    let files = load_selected_files(root)?;
    dryrun::run_dry_run(root, &files)
}

/// One row of `strict-surface-profile-v1.jsonl`: mechanism-neutral,
/// grammar-lane facts of one G0-strict file plus its frozen identity.
/// No timing, no counters, no horse fields — this export describes the
/// WORKLOAD, never a mechanism.
#[derive(serde::Serialize)]
struct StrictSurfaceProfileRow<'a> {
    schema: &'static str,
    profiler_version: &'static str,
    source_key: &'a str,
    source_sha256: &'a str,
    file_bytes: u64,
    grammar_id: &'static str,
    structural: markit_mdbench_semantics::StructuralFacts,
}

/// MEASUREMENT-CORRECTIVE-1 §23 — export the strict-file profile.
///
/// The authority for WHICH files is the frozen `full-read-manifest-v1.jsonl`
/// (every record carrying a `G0_STRICT_FULL_READ` lane): the strict
/// surface is never reselected here. The facts are the G0 grammar lane's
/// frozen `StructuralFacts` (block/container counting rules echoed in the
/// record) — the same values the selection and applicability work already
/// consume, re-exported as one flat artifact for workload comprehension.
fn profile_export(root: &Path) -> i32 {
    match profile_export_inner(root) {
        Ok(summary) => {
            println!("{summary}");
            0
        }
        Err(error) => {
            eprintln!("PROFILE_EXPORT_FAILED: {error}");
            1
        }
    }
}

fn profile_export_inner(root: &Path) -> Result<String, String> {
    // Source authority: materialize + hash-verify exactly as everywhere
    // else (load_selected_files fails closed on any byte drift).
    let files = load_selected_files(root)?;
    let mut sources = std::collections::BTreeMap::new();
    for file in &files {
        sources.insert(file.key.clone(), file);
    }

    // The strict surface: the frozen manifest, not a new selection.
    let manifest_path = root.join("workloads/payloads/full-read-manifest-v1.jsonl");
    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let mut strict_keys: Vec<String> = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let record: fullread::FullReadRecord = serde_json::from_str(line)
            .map_err(|e| format!("parse full-read manifest line: {e}"))?;
        if record
            .lanes
            .iter()
            .any(|lane| lane.case_class == "G0_STRICT_FULL_READ")
        {
            strict_keys.push(record.source_key);
        }
    }
    strict_keys.sort();
    strict_keys.dedup();

    let mut rows = Vec::new();
    for key in &strict_keys {
        let file = sources.get(key).ok_or_else(|| {
            format!("G0-strict manifest key {key} is not a materialized selected source")
        })?;
        let profile = lane_profile(&file.text, markit_mdbench_semantics::G0_GRAMMAR_ID)
            .ok_or_else(|| "G0 lane missing from the frozen registry".to_string())?;
        rows.push(StrictSurfaceProfileRow {
            schema: "strict-surface-profile-v1",
            profiler_version: markit_mdbench_semantics::PROFILER_VERSION,
            source_key: key,
            source_sha256: &file.sha256,
            file_bytes: file.file_bytes,
            grammar_id: "G0",
            structural: profile.structural,
        });
    }
    let out_dir = root.join("workloads/profiles");
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("create {}: {e}", out_dir.display()))?;
    let lines = rows
        .iter()
        .map(|row| canonical_json_line(row))
        .collect::<String>();
    let out_path = out_dir.join("strict-surface-profile-v1.jsonl");
    std::fs::write(&out_path, lines).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    Ok(format!(
        "PROFILE_EXPORT_OK strict_files={} rows={} path=workloads/profiles/strict-surface-profile-v1.jsonl (mechanism-neutral facts; the strict surface was NOT reselected)",
        strict_keys.len(),
        rows.len()
    ))
}
