//! Eligibility-bias report (CORRECTIVE-B §19): candidate universe vs
//! lane-strict subsets, measured on every structural dimension, per
//! project and per domain. Bias facts are reported, never hidden.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::stats::{feature_stats, FeatureStats};
use crate::{CandidateRow, ELIGIBILITY_BIAS_SCHEMA};

#[derive(Debug, Clone, Serialize)]
pub struct EligibilityClass {
    pub grammar_id: String,
    pub all_candidates: u64,
    pub lane_valid: u64,
    pub strict_scope_clean: u64,
    pub realism_only_blocker_bearing: u64,
    pub deferred_or_unknown: u64,
    pub strict_bytes: u64,
    pub all_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BiasDimension {
    pub feature: String,
    pub universe: FeatureStats,
    pub strict: FeatureStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectEligibility {
    pub source_id: String,
    pub domain: String,
    pub candidates: u64,
    pub g0_strict: u64,
    pub g1_strict: u64,
    pub g0_top_blockers: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BiasArtifact {
    pub schema: String,
    pub classes: Vec<EligibilityClass>,
    pub dimensions: Vec<BiasDimension>,
    pub per_project: Vec<ProjectEligibility>,
    pub per_domain: Vec<(String, u64, u64)>,
    pub proposal_concentration: ProposalConcentration,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalConcentration {
    pub sources: Vec<String>,
    pub candidate_files: u64,
    pub candidate_bytes: u64,
    pub file_share: f64,
    pub byte_share: f64,
    pub selected_core_share: Option<f64>,
}

fn strict_value(row: &CandidateRow, feature: &str) -> f64 {
    let Some(g0) = row.g0() else {
        return 0.0;
    };
    match feature {
        "file_bytes" => row.file_bytes as f64,
        "block_count" => g0.block_count as f64,
        "largest_block_bytes" => g0.largest_block_bytes as f64,
        "max_container_depth" => g0.max_container_depth as f64,
        "fence_density_per_kib" => g0.fence_density_per_kib,
        "code_occupancy" => g0.code_occupancy,
        "reference_density_per_kib" => g0.reference_density_per_kib,
        "cjk_byte_share" => row.cjk_byte_share,
        "line_count" => row.line_count as f64,
        _ => 0.0,
    }
}

pub const BIAS_DIMENSIONS: [&str; 9] = [
    "file_bytes",
    "line_count",
    "block_count",
    "largest_block_bytes",
    "max_container_depth",
    "fence_density_per_kib",
    "code_occupancy",
    "reference_density_per_kib",
    "cjk_byte_share",
];

/// Compute the eligibility-bias artifact over all rows. `selected_core`
/// is the final REPRESENTATIVE_SET membership (empty before selection);
/// it is used only for the selected-set proposal-share fact (§20).
pub fn analyze(rows: &[CandidateRow], selected_core: &[&CandidateRow]) -> BiasArtifact {
    let mut classes = Vec::new();
    for grammar_id in [
        markit_mdbench_semantics::G0_GRAMMAR_ID,
        markit_mdbench_semantics::G1_GRAMMAR_ID,
        markit_mdbench_semantics::G2_GRAMMAR_ID,
    ] {
        let lane_rows: Vec<&CandidateRow> = rows
            .iter()
            .filter(|row| row.lanes.iter().any(|lane| lane.grammar_id == grammar_id))
            .collect();
        let strict: Vec<&CandidateRow> = lane_rows
            .iter()
            .copied()
            .filter(|row| {
                row.lanes
                    .iter()
                    .any(|lane| lane.grammar_id == grammar_id && lane.strict_scope_clean)
            })
            .collect();
        classes.push(EligibilityClass {
            grammar_id: grammar_id.to_string(),
            all_candidates: rows.len() as u64,
            lane_valid: lane_rows
                .iter()
                .filter(|row| {
                    row.lanes
                        .iter()
                        .any(|lane| lane.grammar_id == grammar_id && lane.lane_valid)
                })
                .count() as u64,
            strict_scope_clean: strict.len() as u64,
            realism_only_blocker_bearing: lane_rows
                .iter()
                .filter(|row| {
                    row.lanes.iter().any(|lane| {
                        lane.grammar_id == grammar_id && lane.lane_valid && !lane.strict_scope_clean
                    })
                })
                .count() as u64,
            deferred_or_unknown: if grammar_id == markit_mdbench_semantics::G2_GRAMMAR_ID {
                rows.len() as u64
            } else {
                0
            },
            strict_bytes: strict.iter().map(|row| row.file_bytes).sum(),
            all_bytes: rows.iter().map(|row| row.file_bytes).sum(),
        });
    }

    let g0_strict_rows: Vec<&CandidateRow> =
        rows.iter().filter(|row| row.g0_strict_eligible()).collect();

    let dimensions = BIAS_DIMENSIONS
        .iter()
        .map(|feature| BiasDimension {
            feature: feature.to_string(),
            universe: feature_stats(
                &rows
                    .iter()
                    .map(|row| universe_value(row, feature))
                    .collect::<Vec<f64>>(),
            ),
            strict: feature_stats(
                &g0_strict_rows
                    .iter()
                    .map(|row| strict_value(row, feature))
                    .collect::<Vec<f64>>(),
            ),
        })
        .collect();

    let mut per_project: Vec<ProjectEligibility> = Vec::new();
    let mut by_project: BTreeMap<&str, Vec<&CandidateRow>> = BTreeMap::new();
    for row in rows {
        by_project
            .entry(row.identity.source_id.as_str())
            .or_default()
            .push(row);
    }
    for (source_id, project_rows) in by_project {
        let mut blockers: BTreeMap<String, u64> = BTreeMap::new();
        for row in &project_rows {
            if let Some(g0) = row.g0() {
                for (kind, count) in &g0.blocker_kinds {
                    *blockers.entry(kind.clone()).or_insert(0) += count;
                }
            }
        }
        let mut top_blockers: Vec<(String, u64)> = blockers.into_iter().collect();
        top_blockers.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        top_blockers.truncate(5);
        per_project.push(ProjectEligibility {
            source_id: source_id.to_string(),
            domain: project_rows[0].identity.domain.clone(),
            candidates: project_rows.len() as u64,
            g0_strict: project_rows
                .iter()
                .filter(|row| row.g0_strict_eligible())
                .count() as u64,
            g1_strict: project_rows
                .iter()
                .filter(|row| {
                    row.g1()
                        .map(|lane| lane.lane_valid && lane.strict_scope_clean)
                        .unwrap_or(false)
                })
                .count() as u64,
            g0_top_blockers: top_blockers,
        });
    }

    let mut per_domain: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for row in rows {
        let entry = per_domain
            .entry(row.identity.domain.clone())
            .or_insert((0, 0));
        entry.0 += 1;
        if row.g0_strict_eligible() {
            entry.1 += 1;
        }
    }
    let per_domain: Vec<(String, u64, u64)> = per_domain
        .into_iter()
        .map(|(domain, (candidates, strict))| (domain, candidates, strict))
        .collect();

    let proposal_sources = [
        "ethereum-eips",
        "kubernetes-keps",
        "rust-rfcs",
        "swift-evolution",
    ];
    let proposal_files = rows
        .iter()
        .filter(|row| proposal_sources.contains(&row.identity.source_id.as_str()))
        .count() as u64;
    let proposal_bytes = rows
        .iter()
        .filter(|row| proposal_sources.contains(&row.identity.source_id.as_str()))
        .map(|row| row.file_bytes)
        .sum();

    let mut notes = Vec::new();
    let g0_class = &classes[0];
    if g0_class.strict_scope_clean * 2 < g0_class.all_candidates {
        notes.push(format!(
            "G0 eligibility materially changes the corpus: {}/{} candidates ({:.1}%) are G0 strict-scope clean; all bias dimensions below compare the universe against this reduced strict subset",
            g0_class.strict_scope_clean,
            g0_class.all_candidates,
            100.0 * g0_class.strict_scope_clean as f64 / g0_class.all_candidates as f64
        ));
    }
    notes.push(
        "G1 strict evidence is table-scoped only (the single semantically qualified G1 construct); G1 base CommonMark kinds are contract_declared_not_qualified and are never counted as strict coverage".to_string(),
    );
    notes.push(
        "G2 lane_valid is false for every candidate: math semantics are deferred, so every math-looking candidate is ambiguous/unknown evidence, never occupancy".to_string(),
    );

    BiasArtifact {
        schema: ELIGIBILITY_BIAS_SCHEMA.to_string(),
        classes,
        dimensions,
        per_project,
        per_domain,
        proposal_concentration: ProposalConcentration {
            sources: proposal_sources
                .iter()
                .map(|source| source.to_string())
                .collect(),
            candidate_files: proposal_files,
            candidate_bytes: proposal_bytes,
            file_share: proposal_files as f64 / rows.len() as f64,
            byte_share: proposal_bytes as f64
                / rows.iter().map(|row| row.file_bytes).sum::<u64>() as f64,
            selected_core_share: if selected_core.is_empty() {
                None
            } else {
                let selected_proposal = selected_core
                    .iter()
                    .filter(|row| proposal_sources.contains(&row.identity.source_id.as_str()))
                    .count() as f64;
                Some(selected_proposal / selected_core.len() as f64)
            },
        },
        notes,
    }
}

fn universe_value(row: &CandidateRow, feature: &str) -> f64 {
    match feature {
        "file_bytes" => row.file_bytes as f64,
        "line_count" => row.line_count as f64,
        // Structural features under G0 even for non-strict files: the G0
        // parse is total over UTF-8, so its structural record is defined
        // for every profiled candidate.
        _ => row.g0().map(|_| strict_value(row, feature)).unwrap_or(0.0),
    }
}

/// Public wrapper used by the distributions artifact.
pub fn universe_feature(row: &CandidateRow, feature: &str) -> f64 {
    universe_value(row, feature)
}
