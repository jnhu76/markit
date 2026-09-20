//! Coverage-retention data and UNCOVERED-WORKLOAD-SPACE inputs
//! (CORRECTIVE-B §37-§39). No silent zeroes: every required cell reports
//! candidate / eligible / selected counts.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::selection::{syntax_cell, SelectionOutcome, SYNTAX_CELLS, FEATURES};
use crate::stats::{bin_of, feature_stats, FeatureBins};
use crate::{CandidateRow, COVERAGE_SCHEMA};

#[derive(Debug, Clone, Serialize)]
pub struct FeatureCellCoverage {
    pub cell: String,
    pub candidate_count: u64,
    pub eligible_count: u64,
    pub selected_count: u64,
    pub selected_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyntaxCellCoverage {
    pub cell: String,
    pub evidence_grade_required: String,
    pub candidate_evidence_count: u64,
    pub selected_evidence_count: u64,
    pub best_evidence_grade: Option<String>,
    pub selected_files: Vec<String>,
    pub remaining_gap: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtremeRoleCoverage {
    pub dimension: String,
    pub observed_max: f64,
    pub observed_max_file: String,
    pub selected_max: f64,
    pub tail_replicate: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UncoveredSpaceFact {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageArtifact {
    pub schema: String,
    pub feature_cells: Vec<FeatureCellCoverage>,
    pub syntax_cells: Vec<SyntaxCellCoverage>,
    pub extreme_roles: Vec<ExtremeRoleCoverage>,
    pub candidate_to_eligible_to_selected: CandidateEligibleSelected,
    pub uncovered_space: Vec<UncoveredSpaceFact>,
    pub logical_membership_counts: BTreeMap<String, u64>,
    pub unique_physical_files: u64,
    pub cross_set_overlap: Vec<(String, String, u64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateEligibleSelected {
    pub candidate_files: u64,
    pub candidate_bytes: u64,
    pub g0_strict_files: u64,
    pub g0_strict_bytes: u64,
    pub selected_core_representative_files: u64,
    pub selected_core_bytes: u64,
    pub selected_realism_syntax_full_files: u64,
    pub selected_realism_syntax_full_bytes: u64,
    pub project_share_table: Vec<ProjectShare>,
    pub domain_share_table: Vec<DomainShare>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectShare {
    pub source_id: String,
    pub candidate_share: f64,
    pub eligible_share: f64,
    pub selected_core_share: f64,
    pub selected_realism_share: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainShare {
    pub domain: String,
    pub candidate_share: f64,
    pub eligible_share: f64,
    pub selected_core_share: f64,
    pub selected_realism_share: f64,
}

/// Build the coverage artifact from rows, bins and the selection outcome.
pub fn build(
    rows: &[CandidateRow],
    bins: &[FeatureBins],
    outcome: &SelectionOutcome,
) -> CoverageArtifact {
    let eligible: Vec<&CandidateRow> = rows.iter().filter(|row| row.g0_strict_eligible()).collect();
    let usable: Vec<&CandidateRow> = rows
        .iter()
        .filter(|row| row.materialized && row.hash_match && row.profile_failure.is_none())
        .collect();

    // Feature/joint cells from the outcome's attainable list: split on
    // '×' to distinguish joint cells.
    let mut feature_cells = Vec::new();
    for cell in &outcome.attainable_cells {
        if cell.contains('×') {
            continue;
        }
        let (feature, label) = cell.split_once('=').expect("cell form feature=label");
        let bin = bins.iter().find(|bin| bin.feature == feature).expect("bin");
        let candidate_count = rows
            .iter()
            .filter(|row| row.g0().is_some() && bin_of(bin, crate::selection::feature_value(row, feature)) == label)
            .count() as u64;
        let eligible_count = eligible
            .iter()
            .filter(|row| bin_of(bin, crate::selection::feature_value(row, feature)) == label)
            .count() as u64;
        let selected_files: Vec<String> = outcome
            .members
            .iter()
            .filter(|(_, member)| member.memberships.iter().any(|m| m == "representative"))
            .filter(|(key, _)| {
                let row = rows.iter().find(|row| crate::selection::key(row) == **key);
                row.map(|row| bin_of(bin, crate::selection::feature_value(row, feature)) == label)
                    .unwrap_or(false)
            })
            .map(|(key, _)| key.clone())
            .collect();
        feature_cells.push(FeatureCellCoverage {
            cell: cell.clone(),
            candidate_count,
            eligible_count,
            selected_count: selected_files.len() as u64,
            selected_files,
        });
    }

    let mut syntax_cells = Vec::new();
    for cell in SYNTAX_CELLS {
        let candidate_evidence_count = usable
            .iter()
            .filter(|row| syntax_cell(row, cell).is_some())
            .count() as u64;
        let selected_files: Vec<String> = outcome
            .members
            .iter()
            .filter(|(key, _)| {
                usable
                    .iter()
                    .find(|row| crate::selection::key(row) == **key)
                    .map(|row| syntax_cell(row, cell).is_some())
                    .unwrap_or(false)
            })
            .map(|(key, _)| key.clone())
            .collect();
        let best_evidence_grade = usable
            .iter()
            .filter_map(|row| syntax_cell(row, cell))
            .max_by_key(|grade| match *grade {
                "strict" => 4,
                "declared" => 3,
                "candidate" => 2,
                _ => 1,
            });
        let remaining_gap = if candidate_evidence_count == 0 {
            Some("no evidence anywhere in the frozen universe".to_string())
        } else if selected_files.is_empty() {
            Some("observed in candidates but not covered by any selected file".to_string())
        } else {
            None
        };
        let required = if cell.starts_with("core:") || cell.starts_with("syntax:") || cell.starts_with("table:") {
            "required"
        } else {
            "observed_extra"
        };
        syntax_cells.push(SyntaxCellCoverage {
            cell: cell.to_string(),
            evidence_grade_required: required.to_string(),
            candidate_evidence_count,
            selected_evidence_count: selected_files.len() as u64,
            best_evidence_grade: best_evidence_grade.map(str::to_string),
            selected_files,
            remaining_gap,
        });
    }

    // Extreme roles (§38): observed max vs selected max per dimension.
    let mut extreme_roles = Vec::new();
    for feature in FEATURES {
        let mut observed: Vec<(f64, &CandidateRow)> = usable
            .iter()
            .map(|row| (crate::selection::feature_value(row, feature), *row))
            .collect();
        observed.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap()
                .then_with(|| a.1.lexical_key().cmp(&b.1.lexical_key()))
        });
        let max = observed.first().cloned();
        let selected_max = outcome
            .members
            .iter()
            .filter(|(_, member)| member.memberships.contains(&"extremal".to_string()))
            .filter_map(|(key, _)| {
                usable
                    .iter()
                    .find(|row| crate::selection::key(row) == **key)
                    .map(|row| crate::selection::feature_value(row, feature))
            })
            .fold(None::<f64>, |acc, value| {
                Some(acc.map_or(value, |current: f64| current.max(value)))
            });
        let replicate = outcome
            .members
            .iter()
            .filter(|(_, member)| {
                member
                    .extreme_roles
                    .iter()
                    .any(|role| role == &format!("{feature}:TAIL_REPLICATE"))
            })
            .map(|(key, _)| key.clone())
            .next();
        extreme_roles.push(ExtremeRoleCoverage {
            dimension: feature.to_string(),
            observed_max: max.as_ref().map(|(value, _)| *value).unwrap_or(0.0),
            observed_max_file: max
                .map(|(_, row)| crate::selection::key(row))
                .unwrap_or_else(|| "NONE".to_string()),
            selected_max: selected_max.unwrap_or(0.0),
            tail_replicate: replicate,
        });
    }

    // Candidate -> eligible -> selected comparison (§37).
    let selected_core: Vec<&CandidateRow> = outcome
        .representative
        .iter()
        .filter_map(|key| rows.iter().find(|row| crate::selection::key(row) == *key))
        .collect();
    let selected_realism: Vec<&CandidateRow> = outcome
        .members
        .keys()
        .filter_map(|key| rows.iter().find(|row| crate::selection::key(row) == *key))
        .filter(|row| !outcome.representative.contains(&crate::selection::key(row)))
        .collect();

    let total_files = rows.len() as u64;
    let total_bytes = rows.iter().map(|row| row.file_bytes).sum::<u64>();
    let mut project_share_table = Vec::new();
    let mut by_project: BTreeMap<&str, Vec<&CandidateRow>> = BTreeMap::new();
    for row in rows {
        by_project.entry(row.identity.source_id.as_str()).or_default().push(row);
    }
    for (source_id, project_rows) in &by_project {
        project_share_table.push(ProjectShare {
            source_id: source_id.to_string(),
            candidate_share: project_rows.len() as f64 / total_files as f64,
            eligible_share: project_rows
                .iter()
                .filter(|row| row.g0_strict_eligible())
                .count() as f64
                / eligible.len().max(1) as f64,
            selected_core_share: selected_core
                .iter()
                .filter(|row| row.identity.source_id == *source_id)
                .count() as f64
                / selected_core.len().max(1) as f64,
            selected_realism_share: selected_realism
                .iter()
                .filter(|row| row.identity.source_id == *source_id)
                .count() as f64
                / selected_realism.len().max(1) as f64,
        });
    }
    let mut domain_share_table = Vec::new();
    let mut by_domain: BTreeMap<&str, Vec<&CandidateRow>> = BTreeMap::new();
    for row in rows {
        by_domain.entry(row.identity.domain.as_str()).or_default().push(row);
    }
    for (domain, domain_rows) in &by_domain {
        domain_share_table.push(DomainShare {
            domain: domain.to_string(),
            candidate_share: domain_rows.len() as f64 / total_files as f64,
            eligible_share: domain_rows
                .iter()
                .filter(|row| row.g0_strict_eligible())
                .count() as f64
                / eligible.len().max(1) as f64,
            selected_core_share: selected_core
                .iter()
                .filter(|row| row.identity.domain == *domain)
                .count() as f64
                / selected_core.len().max(1) as f64,
            selected_realism_share: selected_realism
                .iter()
                .filter(|row| row.identity.domain == *domain)
                .count() as f64
                / selected_realism.len().max(1) as f64,
        });
    }

    // Uncovered space (§39).
    let mut uncovered_space = Vec::new();
    for cell in &syntax_cells {
        if let Some(gap) = &cell.remaining_gap {
            if cell.candidate_evidence_count == 0 {
                uncovered_space.push(UncoveredSpaceFact {
                    kind: "syntax_absent_from_universe".to_string(),
                    detail: format!("{}: {gap}", cell.cell),
                });
            }
        }
    }
    for cell in &syntax_cells {
        if cell.cell.starts_with("syntax:math") && cell.candidate_evidence_count > 0 {
            uncovered_space.push(UncoveredSpaceFact {
                kind: "syntax_observed_lane_deferred".to_string(),
                detail: format!(
                    "{}: math semantics deferred to G2; evidence is ambiguous/unknown candidates only",
                    cell.cell
                ),
            });
        }
    }
    for domain in crate::domains::DOMAIN_STRATA {
        let present = rows.iter().any(|row| row.identity.domain == domain);
        if !present {
            uncovered_space.push(UncoveredSpaceFact {
                kind: "domain_missing_from_sampling_frame".to_string(),
                detail: format!("{domain}: no acquisition source maps to this stratum"),
            });
        }
    }
    for extreme in &extreme_roles {
        if extreme.tail_replicate.is_none() {
            uncovered_space.push(UncoveredSpaceFact {
                kind: "extreme_without_cross_project_replicate".to_string(),
                detail: format!("{}: NONE_AVAILABLE replicate", extreme.dimension),
            });
        }
    }
    let ambiguous_total: u64 = rows
        .iter()
        .filter_map(|row| row.g1())
        .map(|lane| lane.ambiguous_kinds.values().sum::<u64>())
        .sum();
    let unknown_total: u64 = rows
        .iter()
        .filter_map(|row| row.g1())
        .map(|lane| lane.unknown_kinds.values().sum::<u64>())
        .sum();
    if ambiguous_total > 0 || unknown_total > 0 {
        uncovered_space.push(UncoveredSpaceFact {
            kind: "ambiguous_unknown_facts".to_string(),
            detail: format!(
                "{ambiguous_total} ambiguous and {unknown_total} unknown host-context facts exist; they block strict scope and are never counted as coverage"
            ),
        });
    }

    // Logical membership counts + cross-set overlap (§36).
    let mut logical_membership_counts = BTreeMap::new();
    logical_membership_counts.insert(
        "REPRESENTATIVE_SET".to_string(),
        outcome.representative.len() as u64,
    );
    logical_membership_counts.insert("EXTREMAL_SET".to_string(), outcome.extremal.len() as u64);
    logical_membership_counts.insert(
        "SYNTAX_COVERAGE_SET".to_string(),
        outcome.syntax_coverage.len() as u64,
    );
    logical_membership_counts.insert(
        "FULL_DOCUMENT_SET".to_string(),
        outcome.full_document.len() as u64,
    );
    let sets = [
        ("REPRESENTATIVE_SET", &outcome.representative),
        ("EXTREMAL_SET", &outcome.extremal),
        ("SYNTAX_COVERAGE_SET", &outcome.syntax_coverage),
        ("FULL_DOCUMENT_SET", &outcome.full_document),
    ];
    let mut cross_set_overlap = Vec::new();
    for (i, (name_a, set_a)) in sets.iter().enumerate() {
        for (name_b, set_b) in sets.iter().skip(i + 1) {
            let overlap = set_a.iter().filter(|key| set_b.contains(key)).count() as u64;
            cross_set_overlap.push((name_a.to_string(), name_b.to_string(), overlap));
        }
    }

    CoverageArtifact {
        schema: COVERAGE_SCHEMA.to_string(),
        feature_cells,
        syntax_cells,
        extreme_roles,
        candidate_to_eligible_to_selected: CandidateEligibleSelected {
            candidate_files: total_files,
            candidate_bytes: total_bytes,
            g0_strict_files: eligible.len() as u64,
            g0_strict_bytes: eligible.iter().map(|row| row.file_bytes).sum(),
            selected_core_representative_files: selected_core.len() as u64,
            selected_core_bytes: selected_core.iter().map(|row| row.file_bytes).sum(),
            selected_realism_syntax_full_files: selected_realism.len() as u64,
            selected_realism_syntax_full_bytes: selected_realism.iter().map(|row| row.file_bytes).sum(),
            project_share_table,
            domain_share_table,
        },
        uncovered_space,
        logical_membership_counts,
        unique_physical_files: outcome.members.len() as u64,
        cross_set_overlap,
    }
}

/// Distribution artifact body (per-feature stats over the full universe,
/// the G0-strict subset, per project and per domain).
#[derive(Debug, Clone, Serialize)]
pub struct DistributionsArtifact {
    pub schema: String,
    pub universe: BTreeMap<String, crate::stats::FeatureStats>,
    pub g0_strict: BTreeMap<String, crate::stats::FeatureStats>,
    pub per_project: BTreeMap<String, u64>,
    pub per_domain: BTreeMap<String, u64>,
    pub per_eligibility_class: BTreeMap<String, u64>,
    pub feature_list: Vec<String>,
}

pub fn distributions(rows: &[CandidateRow]) -> DistributionsArtifact {
    let eligible: Vec<&CandidateRow> = rows.iter().filter(|row| row.g0_strict_eligible()).collect();
    let universe = crate::bias::BIAS_DIMENSIONS
        .iter()
        .map(|feature| {
            (
                feature.to_string(),
                feature_stats(
                    &rows
                        .iter()
                        .map(|row| crate::bias::universe_feature(row, feature))
                        .collect::<Vec<f64>>(),
                ),
            )
        })
        .collect();
    let g0_strict = crate::bias::BIAS_DIMENSIONS
        .iter()
        .map(|feature| {
            (
                feature.to_string(),
                feature_stats(
                    &eligible
                        .iter()
                        .map(|row| crate::selection::feature_value(row, feature))
                        .collect::<Vec<f64>>(),
                ),
            )
        })
        .collect();
    let mut per_project = BTreeMap::new();
    for row in rows {
        *per_project.entry(row.identity.source_id.clone()).or_insert(0) += 1;
    }
    let mut per_domain = BTreeMap::new();
    for row in rows {
        *per_domain.entry(row.identity.domain.clone()).or_insert(0) += 1;
    }
    let mut per_eligibility_class = BTreeMap::new();
    *per_eligibility_class
        .entry("all_candidates".to_string())
        .or_insert(0) += rows.len() as u64;
    *per_eligibility_class
        .entry("g0_strict_scope_clean".to_string())
        .or_insert(0) += eligible.len() as u64;
    *per_eligibility_class
        .entry("g1_strict_scope_clean".to_string())
        .or_insert(0)
        += rows
            .iter()
            .filter(|row| {
                row.g1()
                    .map(|lane| lane.lane_valid && lane.strict_scope_clean)
                    .unwrap_or(false)
            })
            .count() as u64;
    *per_eligibility_class
        .entry("g2_deferred".to_string())
        .or_insert(0) += rows.len() as u64;
    DistributionsArtifact {
        schema: crate::DISTRIBUTIONS_SCHEMA.to_string(),
        universe,
        g0_strict,
        per_project,
        per_domain,
        per_eligibility_class,
        feature_list: crate::bias::BIAS_DIMENSIONS
            .iter()
            .map(|f| f.to_string())
            .collect(),
    }
}
