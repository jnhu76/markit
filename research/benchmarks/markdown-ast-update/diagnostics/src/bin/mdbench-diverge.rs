//! mdbench-diverge — frozen-workload correctness diagnosis (#22 closure).
//!
//! Subcommands (run from the benchmark root or pass its path):
//!
//! ```text
//! case <root> <payload_id> [horse...]   A/B/C/D isolation for one frozen payload
//! inventory <root> [out.jsonl]          every wrong dispatch, classified
//! ```
//!
//! Read-only over the frozen workload. No timing, no counters, no
//! research metric: the only outputs are the isolation class, the first
//! normalized divergence, and a source excerpt. Nothing here writes a
//! payload, manifest, or receipt.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use markit_mdbench_common::{CanonicalEdit, Source, SourceId};
use markit_mdbench_diagnostics::{
    describe_divergence as describe, isolate, render, row_json, FailureRow, Isolation,
    IsolationClass,
};
use markit_mdbench_full_rebuild::FullRebuildMechanism;
use markit_mdbench_oracle::{normalized_checksum, NormalizedDocument};
use markit_mdbench_semantics::payload::PayloadRecord;
use markit_mdbench_workload_freeze::dryrun::read_jsonl;
use markit_mdbench_workload_freeze::load_selected_files;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() < 3 {
        usage();
    }
    let command = arguments[1].as_str();
    let root = PathBuf::from(&arguments[2]);
    let code = match command {
        "case" => case_command(&root, &arguments[3..]),
        "inventory" => inventory_command(&root, arguments.get(3).map(PathBuf::from)),
        _ => usage(),
    };
    std::process::exit(code);
}

fn usage() -> ! {
    eprintln!(
        "usage: mdbench-diverge <case <root> <payload_id> [horse...] | inventory <root> [out.jsonl]>"
    );
    std::process::exit(2);
}

/// One frozen payload with its reconstructed pre/post sources.
struct Case {
    payload: PayloadRecord,
    pre: Source,
    post: Source,
    edit: CanonicalEdit,
}

fn load_case(
    root: &Path,
    payload_id: &str,
    files: &[markit_mdbench_workload_freeze::SelectedFile],
    payloads: &[PayloadRecord],
) -> Result<Case, String> {
    let payload = payloads
        .iter()
        .find(|candidate| candidate.payload_id == payload_id)
        .ok_or_else(|| format!("unknown payload id {payload_id}"))?
        .clone();
    let base = files
        .iter()
        .find(|file| file.key == payload.source_path)
        .ok_or_else(|| format!("payload source {} not materialized", payload.source_path))?;
    let _ = root;
    let pre_text = if payload.step == 0 {
        base.text.clone()
    } else {
        let step0 = payloads
            .iter()
            .find(|candidate| candidate.trace_id == payload.trace_id && candidate.step == 0)
            .ok_or_else(|| format!("trace {} has no step 0", payload.trace_id))?;
        step0
            .edit
            .apply(&base.text)
            .map_err(|error| format!("broken state: {error:?}"))?
    };
    let post_text = payload
        .edit
        .apply(&pre_text)
        .map_err(|error| format!("post source: {error:?}"))?;
    let edit = payload
        .edit
        .to_canonical()
        .map_err(|error| format!("canonical edit: {error:?}"))?;
    Ok(Case {
        payload,
        pre: Source::new(SourceId(0), pre_text),
        post: Source::new(SourceId(1), post_text),
        edit,
    })
}

fn h0_reference(bytes: &[u8]) -> NormalizedDocument {
    markit_mdbench_full_rebuild::parse_document(bytes)
}

/// Run the A/B/C/D isolation for every horse on one case, returning the
/// per-horse results in frozen order.
fn isolate_all(case: &Case) -> Vec<(&'static str, Isolation)> {
    let reference = h0_reference(case.post.as_bytes());
    let h0_pre = h0_reference(case.pre.as_bytes());
    let mut out = Vec::new();
    macro_rules! run {
        ($horse:literal, $mech:expr) => {{
            let mechanism = $mech;
            let isolation = isolate(
                &mechanism, &case.pre, &case.post, &case.edit, &reference, &h0_pre,
            );
            out.push(($horse, isolation));
        }};
    }
    run!("H0", FullRebuildMechanism::new());
    run!("H1", markit_mdbench_block_local::BlockLocalMechanism::new());
    run!(
        "H2",
        markit_mdbench_fragment_reuse::FragmentReuseMechanism::new()
    );
    run!(
        "H3",
        markit_mdbench_old_tree_subtree_reuse::OldTreeSubtreeReuseMechanism::new()
    );
    run!(
        "H4",
        markit_mdbench_restart_convergence::RestartConvergenceMechanism::new()
    );
    out
}

fn case_command(root: &Path, rest: &[String]) -> i32 {
    if rest.is_empty() {
        usage();
    }
    let payload_id = &rest[0];
    let wanted: Vec<&str> = rest[1..].iter().map(String::as_str).collect();
    let files = match load_selected_files(root) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("LOAD_FAILED: {error}");
            return 1;
        }
    };
    let payloads: Vec<PayloadRecord> =
        match read_jsonl(&root.join("workloads/payloads/edit-write-manifest-v1.jsonl")) {
            Ok(payloads) => payloads,
            Err(error) => {
                eprintln!("LOAD_FAILED: {error}");
                return 1;
            }
        };
    let case = match load_case(root, payload_id, &files, &payloads) {
        Ok(case) => case,
        Err(error) => {
            eprintln!("CASE_FAILED: {error}");
            return 1;
        }
    };
    println!(
        "CASE {} transition={} family={} step={} source={} edit=[{}, {}) insert={}B",
        case.payload.payload_id,
        case.payload.expected_transition,
        case.payload.edit_family,
        case.payload.step,
        case.payload.source_path,
        case.payload.edit.edit_start,
        case.payload.edit.edit_end,
        case.payload.edit.inserted_text.len(),
    );
    println!(
        "  pre_len={} post_len={} pre_sha={} post_sha={}",
        case.pre.as_bytes().len(),
        case.post.as_bytes().len(),
        case.payload.pre_source_sha256,
        case.payload.post_source_sha256,
    );
    for (horse, isolation) in isolate_all(&case) {
        if !wanted.is_empty() && !wanted.contains(&horse) {
            continue;
        }
        print!(
            "{}",
            render(
                horse,
                &case.payload.payload_id,
                &case.payload.expected_transition,
                &case.payload.source_path,
                &isolation,
                case.post.as_bytes(),
            )
        );
    }
    0
}

fn inventory_command(root: &Path, out_path: Option<PathBuf>) -> i32 {
    let files = match load_selected_files(root) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("LOAD_FAILED: {error}");
            return 1;
        }
    };
    let payloads: Vec<PayloadRecord> =
        match read_jsonl(&root.join("workloads/payloads/edit-write-manifest-v1.jsonl")) {
            Ok(payloads) => payloads,
            Err(error) => {
                eprintln!("LOAD_FAILED: {error}");
                return 1;
            }
        };
    let mut by_trace: BTreeMap<&str, Vec<&PayloadRecord>> = BTreeMap::new();
    for payload in &payloads {
        by_trace
            .entry(payload.trace_id.as_str())
            .or_default()
            .push(payload);
    }

    let mut rows: Vec<FailureRow> = Vec::new();
    let mut classes: BTreeMap<(String, String), u64> = BTreeMap::new();
    for (trace_id, group) in by_trace {
        let _ = trace_id;
        let base = group[0];
        if base.grammar_id != markit_mdbench_workload_freeze::G0_GRAMMAR_ID
            || !base
                .memberships
                .iter()
                .any(|m| m == markit_mdbench_workload_freeze::MEMBERSHIP_G0_PRIMARY)
        {
            continue;
        }
        let mut sorted = group.clone();
        sorted.sort_by_key(|payload| payload.step);
        for payload in sorted {
            let case = match load_case(root, &payload.payload_id, &files, &payloads) {
                Ok(case) => case,
                Err(error) => {
                    eprintln!("CASE_FAILED {}: {error}", payload.payload_id);
                    return 1;
                }
            };
            for (horse, isolation) in isolate_all(&case) {
                let class = isolation.class();
                *classes
                    .entry((horse.to_string(), class.name().to_string()))
                    .or_insert(0) += 1;
                if class == IsolationClass::Pass {
                    continue;
                }
                let (divergence, phase) = match class {
                    IsolationClass::OldStateWrong => (&isolation.hx_full_pre, "C_full_parse_pre"),
                    IsolationClass::FullParseWrong => {
                        (&isolation.hx_full_post, "B_full_parse_post")
                    }
                    _ => (&isolation.hx_update, "D_update"),
                };
                rows.push(FailureRow {
                    payload_id: payload.payload_id.clone(),
                    case_id: String::new(),
                    horse: horse.to_string(),
                    transition_id: payload.expected_transition.clone(),
                    step: payload.step,
                    edit_family: payload.edit_family.clone(),
                    operation_variant: payload.operation_variant.clone(),
                    source_id: payload.source_id.clone(),
                    source_path: payload.source_path.clone(),
                    pre_source_sha256: payload.pre_source_sha256.clone(),
                    post_source_sha256: payload.post_source_sha256.clone(),
                    edit_start: payload.edit.edit_start,
                    edit_end: payload.edit.edit_end,
                    inserted_bytes: payload.edit.inserted_text.len() as u64,
                    requested_positions: payload
                        .position
                        .as_ref()
                        .map(|position| {
                            position
                                .requested_positions
                                .iter()
                                .map(|requested| format!("{requested:?}").to_lowercase())
                                .collect()
                        })
                        .unwrap_or_default(),
                    actual_anchor_byte: payload
                        .position
                        .as_ref()
                        .map(|position| position.actual_anchor_byte)
                        .unwrap_or(0),
                    isolation_class: format!("{}:{phase}", class.name()),
                    first_divergence: divergence
                        .as_ref()
                        .map(describe)
                        .unwrap_or_else(|| "none".to_string()),
                    divergence_excerpt: markit_mdbench_diagnostics::excerpt(
                        case.post.as_bytes(),
                        divergence.as_ref().and_then(|d| d.source_span),
                    ),
                    reference_sha: format!(
                        "{:016x}",
                        normalized_checksum(&h0_reference(case.post.as_bytes()))
                    ),
                });
            }
        }
    }

    println!("ISOLATION_CLASS_COUNTS (horse, class) -> dispatches");
    for ((horse, class), count) in &classes {
        println!("  {horse} {class} {count}");
    }
    println!("WRONG_DISPATCHES {}", rows.len());
    for row in &rows {
        println!(
            "  {} {} {} {} {}",
            row.horse, row.transition_id, row.isolation_class, row.payload_id, row.source_path
        );
        println!("    {}", row.first_divergence);
        if !row.divergence_excerpt.is_empty() {
            println!(
                "    excerpt: {}",
                row.divergence_excerpt.replace('\n', "\\n")
            );
        }
    }

    if let Some(path) = out_path {
        let lines: Vec<String> = rows.iter().map(row_json).collect();
        if let Err(error) = markit_mdbench_workload_freeze::write_jsonl(&path, &lines) {
            eprintln!("WRITE_FAILED: {error}");
            return 1;
        }
        println!(
            "INVENTORY_WRITTEN {} rows -> {}",
            rows.len(),
            path.display()
        );
    }
    0
}
