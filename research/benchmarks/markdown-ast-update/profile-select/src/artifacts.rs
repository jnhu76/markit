//! Artifact assembly: run the whole CORRECTIVE-B pipeline in one
//! deterministic pass and render every machine artifact and generated
//! markdown report (§42-§44).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::bias::BiasArtifact;
use crate::coverage::{CoverageArtifact, DistributionsArtifact};
use crate::redundancy::RedundancyArtifact;
use crate::selection::{frozen_config, select, SelectionConfig, SelectionOutcome};
use crate::spotcheck::SpotCheckArtifact;
use crate::universe::VerificationArtifact;
use crate::CandidateRow;

/// All generated artifacts of one full CORRECTIVE-B run, in memory.
pub struct Artifacts {
    pub verification: VerificationArtifact,
    pub distributions: DistributionsArtifact,
    pub bias: BiasArtifact,
    pub redundancy: RedundancyArtifact,
    pub config: SelectionConfig,
    pub outcome: SelectionOutcome,
    pub coverage: CoverageArtifact,
    pub spot_check: SpotCheckArtifact,
    pub rows: Vec<CandidateRow>,
    pub candidate_rows_jsonl: String,
    pub profile_jsonl_sha256: String,
    pub profile_jsonl_bytes: u64,
    pub identity: ArtifactIdentity,
    pub identities: Vec<crate::CandidateIdentity>,
    pub syntax_inventory: Vec<SyntaxTarget>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactIdentity {
    pub generator_version: String,
    pub generator_tool_sha256: String,
    pub candidate_manifest_sha256: String,
    pub source_lock_sha256: String,
    pub profiler_version: String,
    pub lane_registry_version: String,
    pub selection_contract_version: String,
    pub selection_config_sha256: String,
    pub domain_strata_sha256: String,
}

/// SHA-256 over the crate's own sources (sorted by manifest-relative
/// path) — the deterministic generator identity, computable without a
/// commit SHA. Paths are hashed relative to the crate manifest so the
/// digest is stable across checkouts at different absolute locations.
pub fn tool_source_sha256(manifest_dir: &Path) -> Result<String, String> {
    let mut names: Vec<PathBuf> = Vec::new();
    let src = manifest_dir.join("src");
    collect_rs(&src, &mut names)?;
    names.sort();
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    for path in names {
        let relative = path
            .strip_prefix(manifest_dir)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let bytes =
            fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(&bytes);
    }
    Ok(crate::sha256_hex(&hasher.finalize()))
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| format!("{}: {error}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out)?;
        } else if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(())
}

/// Run the full pipeline. `workloads_root` is the real acquisition tree
/// (input authority); artifacts are returned in memory only — the caller
/// decides where to write them.
pub fn run_all(
    workloads_root: &Path,
    manifest_dir: &Path,
    scratch_dir: &Path,
) -> Result<Artifacts, String> {
    let (identities, _per_source) = crate::universe::load_identities(workloads_root)?;

    // §9: materialization verification (hard-fails on any hash mismatch
    // are reported; selection additionally refuses non-verified rows).
    let verification = crate::universe::verify_universe(workloads_root, &identities)?;
    if verification.summary.hash_mismatches > 0 {
        return Err(format!(
            "§9 hard failure: {} hash mismatches",
            verification.summary.hash_mismatches
        ));
    }
    if verification.summary.missing_files > 0 {
        return Err(format!(
            "§9 failure: {} candidate files not materialized (run acquire.py materialize)",
            verification.summary.missing_files
        ));
    }

    // §10-§13: profile the universe (single pass, streaming jsonl into
    // scratch so the hash can be recorded).
    fs::create_dir_all(scratch_dir).map_err(|error| format!("{}: {error}", scratch_dir.display()))?;
    let profile_jsonl_path = scratch_dir.join("real-profile-v1.jsonl");
    let outcome_profile = crate::rows::profile_universe(
        workloads_root,
        &identities,
        Some(&profile_jsonl_path),
    )?;
    let profile_jsonl_bytes = fs::metadata(&profile_jsonl_path)
        .map(|metadata| metadata.len())
        .unwrap_or(outcome_profile.profile_jsonl_bytes);
    let profile_jsonl_sha256 = crate::sha256_hex(
        &fs::read(&profile_jsonl_path)
            .map_err(|error| format!("{}: {error}", profile_jsonl_path.display()))?,
    );

    // §14: syntax inventory over all rows.
    let syntax_inventory = syntax_inventory(&outcome_profile.rows);

    // §18: distributions.
    let distributions = crate::coverage::distributions(&outcome_profile.rows);

    // §19: eligibility bias (before selection for the universe-side
    // facts; the selected-set share is filled after selection).
    // §21: redundancy.
    let redundancy = crate::redundancy::analyze(workloads_root, &outcome_profile.rows)?;

    // §22-§36: selection.
    let outcome = select(&outcome_profile.rows, &redundancy)?;

    // Bias with the selected core for the §20 selected-set share.
    let selected_core: Vec<&CandidateRow> = outcome
        .representative
        .iter()
        .filter_map(|key| {
            outcome_profile
                .rows
                .iter()
                .find(|row| crate::selection::key(row) == *key)
        })
        .collect();
    let bias = crate::bias::analyze(&outcome_profile.rows, &selected_core);

    // Config frozen before membership output: thresholds derive from the
    // distributions; the config artifact records the rules and the
    // optional-joint decision made under the frozen variation rule.
    let config = frozen_config(crate::selection::optional_joint_included(
        &outcome_profile
            .rows
            .iter()
            .filter(|row| row.g0_strict_eligible())
            .collect::<Vec<_>>(),
    ));

    // §37-§39: coverage.
    let coverage = crate::coverage::build(&outcome_profile.rows, &outcome.bins, &outcome);

    // §45: spot checks.
    let spot_check = crate::spotcheck::run(workloads_root, &outcome_profile.rows, &redundancy)?;

    let candidate_rows_jsonl = outcome_profile
        .rows
        .iter()
        .map(|row| crate::canonical_json(row))
        .collect::<Vec<String>>()
        .join("\n")
        + "\n";

    let identity = ArtifactIdentity {
        generator_version: crate::CORRECTIVE_B_VERSION.to_string(),
        generator_tool_sha256: tool_source_sha256(manifest_dir)?,
        candidate_manifest_sha256: file_sha256(&workloads_root.join("manifests/candidate-universe-v1.json"))?,
        source_lock_sha256: file_sha256(&workloads_root.join("source-lock.json"))?,
        profiler_version: markit_mdbench_semantics::PROFILER_VERSION.to_string(),
        lane_registry_version: markit_mdbench_semantics::LANE_REGISTRY_VERSION.to_string(),
        selection_contract_version: "SELECTION-CONTRACT-v1".to_string(),
        selection_config_sha256: crate::sha256_hex(crate::canonical_json(&config).as_bytes()),
        domain_strata_sha256: file_sha256(&workloads_root.join("analysis/domain-strata-v1.json"))?,
    };

    Ok(Artifacts {
        verification,
        distributions,
        bias,
        redundancy,
        config,
        outcome,
        coverage,
        spot_check,
        rows: outcome_profile.rows,
        candidate_rows_jsonl,
        profile_jsonl_sha256,
        profile_jsonl_bytes,
        identity,
        identities,
        syntax_inventory,
    })
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(crate::sha256_hex(&bytes))
}

/// Syntax inventory (§14): per-target files/occurrences/grades. Separate
/// counts per recognition class — never one misleading `syntax_count`.
#[derive(Debug, Clone, Serialize)]
pub struct SyntaxTarget {
    pub target: String,
    pub owning_lane: String,
    pub evidence_grade: String,
    pub files_with_recognized_host_syntax: u64,
    pub recognized_occurrences: u64,
    pub candidate_occurrences: u64,
    pub ambiguous_occurrences: u64,
    pub unknown_occurrences: u64,
    pub non_host_context_occurrences: u64,
    pub project_count: u64,
    pub domain_count: u64,
}

pub fn syntax_inventory(rows: &[CandidateRow]) -> Vec<SyntaxTarget> {
    const TARGETS: [(&str, &str); 26] = [
        ("paragraph", "G0"),
        ("heading_atx", "G0"),
        ("list", "G0"),
        ("block_quote", "G0"),
        ("code_block_fenced", "G0"),
        ("emphasis", "G0"),
        ("code_span", "G0"),
        ("link_inline", "G0"),
        ("link_reference", "G0"),
        ("reference_definition", "G0"),
        ("table", "G1"),
        ("image", "G1"),
        ("link_autolink", "G1"),
        ("thematic_break", "G1"),
        ("code_block_indented", "G1"),
        ("heading_setext", "G1"),
        ("html_block", "G1"),
        ("raw_html_inline", "G1"),
        ("strong", "G1"),
        ("hard_break", "G1"),
        ("soft_break", "G1"),
        ("strikethrough", "G0"),
        ("task_list_item", "G0"),
        ("front_matter", "G0"),
        ("directive", "G0"),
        ("inline_math", "G1"),
    ];
    let mut targets = Vec::new();
    for (kind, lane_id) in TARGETS {
        let grammar_id = match lane_id {
            "G0" => markit_mdbench_semantics::G0_GRAMMAR_ID,
            _ => markit_mdbench_semantics::G1_GRAMMAR_ID,
        };
        fn lane_of<'a>(row: &'a CandidateRow, grammar_id: &str) -> Option<&'a crate::LaneRow> {
            row.lanes.iter().find(|lane| lane.grammar_id == grammar_id)
        }
        let mut files_recognized = 0u64;
        let mut recognized = 0u64;
        let mut candidate = 0u64;
        let mut ambiguous = 0u64;
        let mut unknown = 0u64;
        let mut nonhost = 0u64;
        let mut projects: std::collections::BTreeSet<&str> = Default::default();
        let mut domains: std::collections::BTreeSet<&str> = Default::default();
        for row in rows {
            let Some(lane) = lane_of(row, grammar_id) else { continue };
            let strict = lane.strict_kinds.get(kind).copied().unwrap_or(0);
            let declared = lane.declared_kinds.get(kind).copied().unwrap_or(0);
            let cand = lane.candidate_kinds.get(kind).copied().unwrap_or(0);
            let amb = lane.ambiguous_kinds.get(kind).copied().unwrap_or(0);
            let unk = lane.unknown_kinds.get(kind).copied().unwrap_or(0);
            let non = lane.nonhost_kinds.get(kind).copied().unwrap_or(0);
            recognized += strict + declared;
            candidate += cand;
            ambiguous += amb;
            unknown += unk;
            nonhost += non;
            if strict + declared > 0 {
                files_recognized += 1;
                projects.insert(row.identity.source_id.as_str());
                domains.insert(row.identity.domain.as_str());
            }
        }
        // Grade mirrors the frozen per-kind `construct_scopes` table in
        // `grammar/grammar-lanes-v1.json`: the G0-strict grade applies only
        // to the 12 frozen BENCH-GRAMMAR-v1 kinds; strikethrough, task list
        // items, front matter and directives are out_of_lane under both G0
        // and G1 and must never be labeled strict.
        let grade = match lane_id {
            "G0"
                if matches!(
                    kind,
                    "paragraph"
                        | "heading_atx"
                        | "list"
                        | "block_quote"
                        | "code_block_fenced"
                        | "emphasis"
                        | "code_span"
                        | "link_inline"
                        | "link_reference"
                        | "reference_definition"
                ) =>
            {
                "strict_lane_coverage".to_string()
            }
            "G1" if kind == "table" => "strict_lane_coverage".to_string(),
            "G1" if matches!(kind, "inline_math" | "display_math") => {
                "lane_deferred".to_string()
            }
            "G1" => "contract_declared_not_qualified".to_string(),
            _ => "out_of_lane_candidate".to_string(),
        };
        targets.push(SyntaxTarget {
            target: kind.to_string(),
            owning_lane: lane_id.to_string(),
            evidence_grade: grade,
            files_with_recognized_host_syntax: files_recognized,
            recognized_occurrences: recognized,
            candidate_occurrences: candidate,
            ambiguous_occurrences: ambiguous,
            unknown_occurrences: unknown,
            non_host_context_occurrences: nonhost,
            project_count: projects.len() as u64,
            domain_count: domains.len() as u64,
        });
    }
    targets
}

/// Write every artifact under `out` (which for the real run is the
/// workloads tree). Large derived data goes to the same tree where the
/// repository .gitignore excludes it.
pub fn write_all(
    out: &Path,
    artifacts: &Artifacts,
    include_large: bool,
    profile_jsonl_source: &Path,
) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    let write = |relative: &str, contents: &str, written: &mut Vec<String>| -> Result<(), String> {
        let path = out.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::write(&path, contents).map_err(|error| format!("{}: {error}", path.display()))?;
        written.push(relative.to_string());
        Ok(())
    };

    let identity_json = crate::canonical_json(&artifacts.identity);
    let identity_value: serde_json::Value = serde_json::from_str(&identity_json)
        .map_err(|error| error.to_string())?;

    // verification (§9)
    let mut verification = crate::canonical_json(&artifacts.verification);
    verification.push_str(&format!(
        "\n{{\"artifact_identity\":{identity_json}}}\n"
    ));
    write(
        "analysis/candidate-universe-verification-v1.json",
        &verification,
        &mut written,
    )?;

    // distributions (§18) + identity + profile jsonl binding
    let mut distributions_json = crate::canonical_json(&serde_json::json!({
        "schema": artifacts.distributions.schema,
        "identity": identity_value,
        "derived_data_binding": {
            "real_profile_jsonl_sha256": artifacts.profile_jsonl_sha256,
            "real_profile_jsonl_bytes": artifacts.profile_jsonl_bytes,
            "real_profile_jsonl_storage": "gitignored local derived data; regenerate with mdbench-corrective-b (§44)"
        },
        "universe": artifacts.distributions.universe,
        "g0_strict": artifacts.distributions.g0_strict,
        "per_project": artifacts.distributions.per_project,
        "per_domain": artifacts.distributions.per_domain,
        "per_eligibility_class": artifacts.distributions.per_eligibility_class,
        "feature_list": artifacts.distributions.feature_list,
    }));
    distributions_json.push('\n');
    write("profiles/distributions-v1.json", &distributions_json, &mut written)?;
    write(
        "profiles/distributions-v1.md",
        &render_distributions_md(artifacts),
        &mut written,
    )?;

    // bias (§19-§20)
    let mut bias_json = crate::canonical_json(&serde_json::json!({
        "schema": artifacts.bias.schema,
        "identity": identity_value,
        "classes": artifacts.bias.classes,
        "dimensions": artifacts.bias.dimensions,
        "per_project": artifacts.bias.per_project,
        "per_domain": artifacts.bias.per_domain,
        "proposal_concentration": artifacts.bias.proposal_concentration,
        "notes": artifacts.bias.notes,
    }));
    bias_json.push('\n');
    write("analysis/eligibility-bias-v1.json", &bias_json, &mut written)?;
    write(
        "analysis/ELIGIBILITY-BIAS-REPORT-v1.md",
        &render_bias_md(artifacts),
        &mut written,
    )?;

    // redundancy (§21)
    let mut redundancy_json = crate::canonical_json(&serde_json::json!({
        "schema": artifacts.redundancy.schema,
        "identity": identity_value,
        "method": artifacts.redundancy.method,
        "threshold": artifacts.redundancy.threshold,
        "shingle_width": artifacts.redundancy.shingle_width,
        "permutations": artifacts.redundancy.permutations,
        "exact_groups": artifacts.redundancy.exact_groups,
        "near_groups": artifacts.redundancy.near_groups,
        "notes": artifacts.redundancy.notes,
    }));
    redundancy_json.push('\n');
    write("analysis/redundancy-v1.json", &redundancy_json, &mut written)?;
    write(
        "analysis/redundancy-v1.md",
        &render_redundancy_md(artifacts),
        &mut written,
    )?;

    // selection (§22-§36)
    write(
        "selections/selection-config-v1.json",
        &(crate::canonical_json(&artifacts.config) + "\n"),
        &mut written,
    )?;
    write(
        "selections/representative-set-v1.json",
        &(render_set_json(
            crate::REPRESENTATIVE_SET_SCHEMA,
            &artifacts.outcome.representative,
            artifacts,
        )),
        &mut written,
    )?;
    write(
        "selections/extremal-set-v1.json",
        &(render_set_json(
            crate::EXTREMAL_SET_SCHEMA,
            &artifacts.outcome.extremal,
            artifacts,
        )),
        &mut written,
    )?;
    write(
        "selections/syntax-coverage-set-v1.json",
        &(render_set_json(
            crate::SYNTAX_COVERAGE_SET_SCHEMA,
            &artifacts.outcome.syntax_coverage,
            artifacts,
        )),
        &mut written,
    )?;
    write(
        "selections/full-document-set-v1.json",
        &(render_set_json(
            crate::FULL_DOCUMENT_SET_SCHEMA,
            &artifacts.outcome.full_document,
            artifacts,
        )),
        &mut written,
    )?;
    let trace_jsonl: String = artifacts
        .outcome
        .trace
        .iter()
        .map(|record| crate::canonical_json(record))
        .collect::<Vec<String>>()
        .join("\n")
        + "\n";
    write("selections/selection-trace-v1.jsonl", &trace_jsonl, &mut written)?;

    let selected_files = serde_json::json!({
        "schema": crate::SELECTED_FILES_SCHEMA,
        "identity": identity_value,
        "logical_membership_counts": artifacts.coverage.logical_membership_counts,
        "unique_physical_files": artifacts.coverage.unique_physical_files,
        "cross_set_overlap": artifacts.coverage.cross_set_overlap,
        "members": artifacts.outcome.members,
        "bins": artifacts.outcome.bins,
        "plateau_note": artifacts.outcome.plateau_note,
        "cap_relaxations": artifacts.outcome.cap_relaxations,
        "full_document_rejections": artifacts.outcome.full_document_rejections,
        "seventh_document_note": artifacts.outcome.seventh_document_note,
    });
    write(
        "selections/selected-files-v1.json",
        &(crate::canonical_json(&selected_files) + "\n"),
        &mut written,
    )?;

    let coverage_json = serde_json::json!({
        "schema": artifacts.coverage.schema,
        "identity": identity_value,
        "feature_cells": artifacts.coverage.feature_cells,
        "syntax_cells": artifacts.coverage.syntax_cells,
        "extreme_roles": artifacts.coverage.extreme_roles,
        "candidate_to_eligible_to_selected": artifacts.coverage.candidate_to_eligible_to_selected,
        "uncovered_space": artifacts.coverage.uncovered_space,
        "logical_membership_counts": artifacts.coverage.logical_membership_counts,
        "unique_physical_files": artifacts.coverage.unique_physical_files,
        "cross_set_overlap": artifacts.coverage.cross_set_overlap,
        "syntax_inventory": artifacts.syntax_inventory,
        "spot_checks": artifacts.spot_check,
    });
    write(
        "selections/coverage-v1.json",
        &(crate::canonical_json(&coverage_json) + "\n"),
        &mut written,
    )?;
    write(
        "selections/COVERAGE-REPORT-v1.md",
        &render_coverage_md(artifacts),
        &mut written,
    )?;
    write(
        "analysis/UNCOVERED-WORKLOAD-SPACE-v1.md",
        &render_uncovered_md(artifacts),
        &mut written,
    )?;

    if include_large {
        let profile_contents =
            fs::read_to_string(profile_jsonl_source).map_err(|error| {
                format!("{}: {error}", profile_jsonl_source.display())
            })?;
        write(
            "profiles/real-profile-v1.jsonl",
            &profile_contents,
            &mut written,
        )?;
        write(
            "profiles/candidate-rows-v1.jsonl",
            &artifacts.candidate_rows_jsonl,
            &mut written,
        )?;
    }
    Ok(written)
}

fn render_set_json(schema: &str, keys: &[String], artifacts: &Artifacts) -> String {
    let members: Vec<&crate::selection::SelectedMember> = keys
        .iter()
        .filter_map(|key| artifacts.outcome.members.get(key))
        .collect();
    crate::canonical_json(&serde_json::json!({
        "schema": schema,
        "identity": serde_json::from_str::<serde_json::Value>(&crate::canonical_json(&artifacts.identity))
            .unwrap_or(serde_json::Value::Null),
        "logical_count": keys.len(),
        "keys": keys,
        "members": members,
    })) + "\n"
}

fn render_distributions_md(artifacts: &Artifacts) -> String {
    let mut out = String::new();
    out.push_str("# Distributions-v1 — candidate universe (CORRECTIVE-B)\n\n");
    out.push_str(&format!(
        "Derived data binding: `real-profile-v1.jsonl` sha256 `{}` ({} bytes, gitignored local regeneration).\n\n",
        artifacts.profile_jsonl_sha256[..16].to_string(),
        artifacts.profile_jsonl_bytes
    ));
    out.push_str("## Universe (all 3,970 candidates unless verification says otherwise)\n\n");
    out.push_str("| feature | count | zeros | min | p25 | p50 | p75 | p95 | p99 | max |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
    for (feature, stats) in &artifacts.distributions.universe {
        out.push_str(&format!(
            "| {} | {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |\n",
            feature, stats.count, stats.zero_count, stats.min, stats.p25, stats.p50, stats.p75, stats.p95, stats.p99, stats.max
        ));
    }
    out.push_str("\n## G0 strict-scope-clean subset\n\n");
    out.push_str("| feature | count | zeros | min | p25 | p50 | p75 | p95 | p99 | max |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
    for (feature, stats) in &artifacts.distributions.g0_strict {
        out.push_str(&format!(
            "| {} | {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |\n",
            feature, stats.count, stats.zero_count, stats.min, stats.p25, stats.p50, stats.p75, stats.p95, stats.p99, stats.max
        ));
    }
    out.push_str("\n## Per project / per domain / eligibility classes\n\n");
    out.push_str("Per-project and per-domain counts and eligibility classes are in `distributions-v1.json` (machine authority; this file is the human view).\n");
    out
}

fn render_bias_md(artifacts: &Artifacts) -> String {
    let mut out = String::new();
    out.push_str("# ELIGIBILITY-BIAS-REPORT-v1 (CORRECTIVE-B §19-§20)\n\n");
    out.push_str("## Eligibility classes per lane\n\n");
    out.push_str("| lane | all | lane_valid | strict | blocker-bearing | deferred | strict bytes |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for class in &artifacts.bias.classes {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            class.grammar_id, class.all_candidates, class.lane_valid, class.strict_scope_clean,
            class.realism_only_blocker_bearing, class.deferred_or_unknown, class.strict_bytes
        ));
    }
    out.push_str("\n## Universe vs G0-strict subset, per dimension\n\n");
    out.push_str("| feature | universe p50 | strict p50 | universe p95 | strict p95 | universe max | strict max |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for dimension in &artifacts.bias.dimensions {
        out.push_str(&format!(
            "| {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |\n",
            dimension.feature,
            dimension.universe.p50, dimension.strict.p50,
            dimension.universe.p95, dimension.strict.p95,
            dimension.universe.max, dimension.strict.max
        ));
    }
    let proposal = &artifacts.bias.proposal_concentration;
    out.push_str(&format!(
        "\n## Proposal/RFC concentration (§20)\n\n- {} account for {} files ({:.2}%) and {} bytes ({:.2}%) of the candidate universe.\n- Selected-set share: {}.\n- This is a bias fact about the sampling frame, not an automatic corpus failure.\n",
        proposal.sources.join(", "),
        proposal.candidate_files,
        100.0 * proposal.file_share,
        proposal.candidate_bytes,
        100.0 * proposal.byte_share,
        proposal
            .selected_core_share
            .map(|share| format!("{:.2}%", 100.0 * share))
            .unwrap_or_else(|| "not yet selected".to_string()),
    ));
    out.push_str("\n## Per-project eligibility (top blockers)\n\n");
    out.push_str("| project | domain | candidates | G0 strict | G1 strict | top G0 blockers |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for project in &artifacts.bias.per_project {
        let blockers = project
            .g0_top_blockers
            .iter()
            .map(|(kind, count)| format!("{kind}×{count}"))
            .collect::<Vec<String>>()
            .join(", ");
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            project.source_id, project.domain, project.candidates, project.g0_strict, project.g1_strict, blockers
        ));
    }
    out.push_str("\nNotes:\n");
    for note in &artifacts.bias.notes {
        out.push_str(&format!("- {note}\n"));
    }
    out
}

fn render_redundancy_md(artifacts: &Artifacts) -> String {
    let mut out = String::new();
    out.push_str("# Redundancy-v1 (CORRECTIVE-B §21)\n\n");
    out.push_str(&format!(
        "Method: `{}` (normalized word-5-gram shingles + deterministic 64-permutation MinHash, threshold {:.2}; no embeddings, no seed). Near-duplicate groups are diagnostic only — nothing is removed automatically.\n\n",
        artifacts.redundancy.method, artifacts.redundancy.threshold
    ));
    out.push_str(&format!(
        "Exact duplicate groups: **{}** (byte-identical by source SHA-256; canonical member = lexically smallest; acquisition registry cross-checked).\n\n",
        artifacts.redundancy.exact_groups.len()
    ));
    for group in artifacts.redundancy.exact_groups.iter().take(20) {
        out.push_str(&format!(
            "- `{}` ({} members: {})\n",
            group.sha256[..12].to_string(),
            group.members.len(),
            group.members.iter().take(4).cloned().collect::<Vec<String>>().join(", ")
        ));
    }
    if artifacts.redundancy.exact_groups.len() > 20 {
        out.push_str(&format!(
            "- … {} more groups (see redundancy-v1.json)\n",
            artifacts.redundancy.exact_groups.len() - 20
        ));
    }
    out.push_str(&format!(
        "\nNear-duplicate candidate groups: **{}**.\n\n",
        artifacts.redundancy.near_groups.len()
    ));
    for group in artifacts.redundancy.near_groups.iter().take(20) {
        out.push_str(&format!(
            "- estimated Jaccard ≥ {:.2}: {}\n",
            group.estimated_jaccard,
            group.members.join(", ")
        ));
    }
    if artifacts.redundancy.near_groups.len() > 20 {
        out.push_str(&format!(
            "- … {} more groups (see redundancy-v1.json)\n",
            artifacts.redundancy.near_groups.len() - 20
        ));
    }
    out
}

fn render_coverage_md(artifacts: &Artifacts) -> String {
    let mut out = String::new();
    out.push_str("# COVERAGE-REPORT-v1 (CORRECTIVE-B §37-§38)\n\n");
    let ces = &artifacts.coverage.candidate_to_eligible_to_selected;
    out.push_str(&format!(
        "Candidate {} files / {} bytes → G0 strict {} files / {} bytes → selected core (representative) {} files / {} bytes; realism/syntax/full adds {} files / {} bytes.\n\n",
        ces.candidate_files, ces.candidate_bytes,
        ces.g0_strict_files, ces.g0_strict_bytes,
        ces.selected_core_representative_files, ces.selected_core_bytes,
        ces.selected_realism_syntax_full_files, ces.selected_realism_syntax_full_bytes
    ));
    out.push_str("## Feature-cell retention (candidate / eligible / selected)\n\n");
    out.push_str("| cell | candidates | eligible | selected |\n");
    out.push_str("|---|---|---|---|\n");
    for cell in &artifacts.coverage.feature_cells {
        out.push_str(&format!("| {} | {} | {} | {} |\n", cell.cell, cell.candidate_count, cell.eligible_count, cell.selected_count));
    }
    out.push_str("\n## Syntax/context cell coverage\n\n");
    out.push_str("| cell | required | candidate evidence | selected | best grade | gap |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for cell in &artifacts.coverage.syntax_cells {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            cell.cell,
            cell.evidence_grade_required,
            cell.candidate_evidence_count,
            cell.selected_evidence_count,
            cell.best_evidence_grade.as_deref().unwrap_or("none"),
            cell.remaining_gap.as_deref().unwrap_or("-")
        ));
    }
    out.push_str("\n## Extreme-role coverage\n\n");
    out.push_str("| dimension | observed max | max file | selected max | tail replicate |\n");
    out.push_str("|---|---|---|---|---|\n");
    for extreme in &artifacts.coverage.extreme_roles {
        out.push_str(&format!(
            "| {} | {:.3} | {} | {:.3} | {} |\n",
            extreme.dimension,
            extreme.observed_max,
            extreme.observed_max_file,
            extreme.selected_max,
            extreme.tail_replicate.as_deref().unwrap_or("NONE_AVAILABLE")
        ));
    }
    out.push_str(&format!(
        "\n## Syntax inventory (§14)\n\n| target | lane | grade | files | recognized | candidate | ambiguous | unknown | non-host | projects | domains |\n|---|---|---|---|---|---|---|---|---|---|---|\n"
    ));
    for target in &artifacts.syntax_inventory {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            target.target, target.owning_lane, target.evidence_grade,
            target.files_with_recognized_host_syntax, target.recognized_occurrences,
            target.candidate_occurrences, target.ambiguous_occurrences, target.unknown_occurrences,
            target.non_host_context_occurrences, target.project_count, target.domain_count
        ));
    }
    out.push_str(&format!(
        "\n## Spot checks (§45)\n\nall_pass = {} ({} checks; details in coverage-v1.json)\n",
        artifacts.spot_check.all_pass,
        artifacts.spot_check.checks.len()
    ));
    out
}

fn render_uncovered_md(artifacts: &Artifacts) -> String {
    let mut out = String::new();
    out.push_str("# UNCOVERED-WORKLOAD-SPACE-v1 (CORRECTIVE-B §39)\n\n");
    out.push_str("Uncovered space is a result, not a defect to fix by adding files in this task.\n\n");
    for fact in &artifacts.coverage.uncovered_space {
        out.push_str(&format!("- **{}**: {}\n", fact.kind, fact.detail));
    }
    if let Some(note) = &artifacts.outcome.plateau_note {
        out.push_str(&format!("\n- **selection_note**: {note}\n"));
    }
    out
}
