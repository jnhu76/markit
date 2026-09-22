//! SELECTION-CONTRACT-v1 — the frozen deterministic selectors
//! (CORRECTIVE-B §22-§36).
//!
//! Everything in this module is mechanical: fixed feature list, frozen
//! binning rule, predeclared joint cells, caps with mechanical
//! relaxation, frozen tie-break ladders, predeclared stop rules. No
//! hand-selection, no horse information, no randomness. Every decision
//! lands in the selection trace with its tie-break values.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::redundancy::RedundancyArtifact;
use crate::stats::{derive_bins, percentile_rank, FeatureBins};
use crate::{CandidateRow, SELECTION_CONFIG_SCHEMA, SELECTION_TRACE_SCHEMA};

/// The frozen selection feature list (§13). All structural features are
/// G0-scoped (`block_count` without a lane is not a fact).
pub const FEATURES: [&str; 8] = [
    "file_bytes",
    "block_count",
    "largest_block_bytes",
    "max_container_depth",
    "fence_density_per_kib",
    "code_occupancy",
    "reference_density_per_kib",
    "cjk_byte_share",
];

/// Features where zero is semantically meaningful (§23).
const ZERO_MEANINGFUL: [&str; 7] = [
    "block_count",
    "largest_block_bytes",
    "max_container_depth",
    "fence_density_per_kib",
    "code_occupancy",
    "reference_density_per_kib",
    "cjk_byte_share",
];

/// Predeclared mandatory joint cells (§24).
pub const JOINT_CELLS: [(&str, &str); 4] = [
    ("file_bytes", "block_count"),
    ("file_bytes", "largest_block_bytes"),
    ("fence_density_per_kib", "code_occupancy"),
    ("file_bytes", "reference_density_per_kib"),
];

/// The optional fifth joint cell, included only under the frozen
/// variation rule (§24).
pub const OPTIONAL_JOINT: (&str, &str) = ("max_container_depth", "block_count");

pub const PROJECT_CAP: u64 = 3;
pub const DOMAIN_CAP_SHARE: f64 = 0.35;
pub const TAIL_TOP_PERCENT: f64 = 0.01;

/// Frozen thresholds for the table sub-cells (§29).
pub const TABLE_WIDE_MIN_COLUMNS: u64 = 8;
pub const TABLE_LONG_MIN_ROWS: u64 = 20;

/// The OpenMLSys math-evidence pseudo-feature (§34): G2 is deferred, so
/// only candidate occurrence evidence exists — never an occupancy.
pub const MATH_CANDIDATE_FEATURE: &str = "__math_candidate_occurrences";

/// The serialized frozen configuration (selections/selection-config-v1.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionConfig {
    pub schema: String,
    pub features: Vec<String>,
    pub zero_meaningful: Vec<String>,
    pub joint_cells: Vec<(String, String)>,
    pub optional_joint: Option<(String, String)>,
    pub project_cap: u64,
    pub domain_cap_share: f64,
    pub tail_top_percent: f64,
    pub table_wide_min_columns: u64,
    pub table_long_min_rows: u64,
    pub population_statement: String,
    pub target_population_note: String,
    pub no_frequency_model: String,
}

pub fn frozen_config(optional_joint_included: bool) -> SelectionConfig {
    SelectionConfig {
        schema: SELECTION_CONFIG_SCHEMA.to_string(),
        features: FEATURES.iter().map(|f| f.to_string()).collect(),
        zero_meaningful: ZERO_MEANINGFUL.iter().map(|f| f.to_string()).collect(),
        joint_cells: JOINT_CELLS
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
        optional_joint: if optional_joint_included {
            Some((OPTIONAL_JOINT.0.to_string(), OPTIONAL_JOINT.1.to_string()))
        } else {
            None
        },
        project_cap: PROJECT_CAP,
        domain_cap_share: DOMAIN_CAP_SHARE,
        tail_top_percent: TAIL_TOP_PERCENT,
        table_wide_min_columns: TABLE_WIDE_MIN_COLUMNS,
        table_long_min_rows: TABLE_LONG_MIN_ROWS,
        population_statement: "The frozen sampling frame is a curated population of retrievable open-source technical Markdown/documentation from the 20 pinned sources in PR #34. It is not a random sample of all Markdown usage.".to_string(),
        target_population_note: "REPRESENTATIVE_SET is coverage-oriented: it covers major observed real-document regimes in the frozen candidate population, not a frequency-weighted sample of all Markdown users.".to_string(),
        no_frequency_model: "No edit probabilities, no weighted workload score, no average real-world speedup (CORRECTIVE-B §41).".to_string(),
    }
}

pub fn feature_value(row: &CandidateRow, feature: &str) -> f64 {
    if feature == MATH_CANDIDATE_FEATURE {
        return rank_feature_value(row, feature);
    }
    match feature {
        "file_bytes" => row.file_bytes as f64,
        "line_count" => row.line_count as f64,
        "cjk_byte_share" => row.cjk_byte_share,
        other => {
            let g0 = row.g0().expect("feature rows require a G0 lane");
            match other {
                "block_count" => g0.block_count as f64,
                "largest_block_bytes" => g0.largest_block_bytes as f64,
                "max_container_depth" => g0.max_container_depth as f64,
                "fence_density_per_kib" => g0.fence_density_per_kib,
                "code_occupancy" => g0.code_occupancy,
                "reference_density_per_kib" => g0.reference_density_per_kib,
                unknown => unreachable!("unknown feature {unknown}"),
            }
        }
    }
}

/// Rank-formula feature values, including the math-candidate occurrence
/// pseudo-feature (G2 deferred: occurrence evidence only, never a
/// fabricated occupancy).
pub fn rank_feature_value(row: &CandidateRow, feature: &str) -> f64 {
    if feature == MATH_CANDIDATE_FEATURE {
        let g1 = row.g1();
        return g1
            .map(|lane| {
                ["inline_math", "display_math"]
                    .iter()
                    .map(|kind| {
                        lane.ambiguous_kinds.get(*kind).copied().unwrap_or(0)
                            + lane.unknown_kinds.get(*kind).copied().unwrap_or(0)
                    })
                    .sum::<u64>() as f64
            })
            .unwrap_or(0.0);
    }
    feature_value(row, feature)
}

pub fn key(row: &CandidateRow) -> String {
    format!("{}/{}", row.identity.source_id, row.identity.snapshot_path)
}

/// One selection-trace record (§26).
#[derive(Debug, Clone, Serialize)]
pub struct TraceRecord {
    pub schema: String,
    pub set: String,
    pub iteration: u64,
    pub selected: String,
    pub new_cells: Vec<String>,
    pub duplicated_cells: u64,
    pub domain_count_before: u64,
    pub domain_count_after: u64,
    pub project_count_before: u64,
    pub project_count_after: u64,
    pub near_duplicate_flags: u64,
    pub tie_break: Vec<(String, String)>,
    pub remaining_uncovered: u64,
    pub cap_relaxation: Option<String>,
    pub rejected: Option<String>,
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SelectedMember {
    pub key: String,
    pub memberships: Vec<String>,
    pub extreme_roles: Vec<String>,
    pub why_this_file: Vec<String>,
    pub domain: String,
    pub evidence_labels: Vec<String>,
    pub transition_opportunity: BTreeMap<String, bool>,
    pub g0_strict_eligible: bool,
    pub g1_strict_eligible: bool,
    pub file_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SelectionOutcome {
    pub representative: Vec<String>,
    pub extremal: Vec<String>,
    pub syntax_coverage: Vec<String>,
    pub full_document: Vec<String>,
    pub trace: Vec<TraceRecord>,
    pub members: BTreeMap<String, SelectedMember>,
    pub bins: Vec<FeatureBins>,
    pub attainable_cells: Vec<String>,
    pub uncovered_cells: Vec<String>,
    pub eligible_domains: Vec<String>,
    pub plateau_note: Option<String>,
    pub cap_relaxations: Vec<String>,
    pub full_document_rejections: Vec<String>,
    pub seventh_document_note: Option<String>,
}

/// Syntax/context coverage cells (§28-§30) with their frozen evidence
/// grades over a candidate row.
pub fn syntax_cell(row: &CandidateRow, cell: &str) -> Option<&'static str> {
    let g0 = row.g0()?;
    let g1 = row.g1()?;
    let strict_has = |kind: &str| g0.strict_kinds.get(kind).copied().unwrap_or(0) > 0;
    let declared_has = |kind: &str| g1.declared_kinds.get(kind).copied().unwrap_or(0) > 0;
    let math_candidates = |kind: &str| {
        g1.ambiguous_kinds.get(kind).copied().unwrap_or(0)
            + g1.unknown_kinds.get(kind).copied().unwrap_or(0)
            > 0
    };
    let candidate_has = |kind: &str| {
        g0.candidate_kinds.get(kind).copied().unwrap_or(0)
            + g1.candidate_kinds.get(kind).copied().unwrap_or(0)
            > 0
    };
    match cell {
        "core:paragraph" => strict_has("paragraph").then_some("strict"),
        "core:heading_atx" => strict_has("heading_atx").then_some("strict"),
        "core:list" => strict_has("list").then_some("strict"),
        "core:block_quote" => strict_has("block_quote").then_some("strict"),
        "core:code_block_fenced" => strict_has("code_block_fenced").then_some("strict"),
        "core:emphasis" => strict_has("emphasis").then_some("strict"),
        "core:code_span" => strict_has("code_span").then_some("strict"),
        "core:link_inline" => strict_has("link_inline").then_some("strict"),
        "core:link_reference" => strict_has("link_reference").then_some("strict"),
        "core:reference_definition" => strict_has("reference_definition").then_some("strict"),
        "syntax:table" => (g1.table_count > 0).then_some("strict"),
        "syntax:image" => declared_has("image").then_some("declared"),
        "syntax:link_autolink" => declared_has("link_autolink").then_some("declared"),
        "syntax:thematic_break" => declared_has("thematic_break").then_some("declared"),
        "syntax:code_block_indented" => declared_has("code_block_indented").then_some("declared"),
        "syntax:heading_setext" => declared_has("heading_setext").then_some("declared"),
        "syntax:html_block" => declared_has("html_block").then_some("declared"),
        "syntax:raw_html_inline" => declared_has("raw_html_inline").then_some("declared"),
        "syntax:math_inline_candidate" => {
            math_candidates("inline_math").then_some("ambiguous_or_unknown")
        }
        "syntax:math_display_candidate" => {
            math_candidates("display_math").then_some("ambiguous_or_unknown")
        }
        "table:ordinary" => (g1.table_count > 0).then_some("strict"),
        "table:wide" => (g1.max_table_columns >= TABLE_WIDE_MIN_COLUMNS).then_some("strict"),
        "table:long" => {
            (g1.max_table_rows_including_header >= TABLE_LONG_MIN_ROWS).then_some("strict")
        }
        "table:alignment_marker" => g1.table_alignment_marker.then_some("strict"),
        "table:escaped_pipe_candidate" => g1.table_candidate_rejected.then_some("candidate"),
        "table:inline_inside_table" => g1.inline_inside_table.then_some("strict"),
        "extra:strong" => declared_has("strong").then_some("declared"),
        "extra:hard_break" => declared_has("hard_break").then_some("declared"),
        "extra:soft_break" => declared_has("soft_break").then_some("declared"),
        "extra:strikethrough" => candidate_has("strikethrough").then_some("candidate"),
        "extra:task_list_item" => candidate_has("task_list_item").then_some("candidate"),
        "extra:front_matter" => candidate_has("front_matter").then_some("candidate"),
        "extra:directive" => candidate_has("directive").then_some("candidate"),
        _ => None,
    }
}

pub const SYNTAX_CELLS: [&str; 33] = [
    "core:paragraph",
    "core:heading_atx",
    "core:list",
    "core:block_quote",
    "core:code_block_fenced",
    "core:emphasis",
    "core:code_span",
    "core:link_inline",
    "core:link_reference",
    "core:reference_definition",
    "syntax:table",
    "syntax:image",
    "syntax:link_autolink",
    "syntax:thematic_break",
    "syntax:code_block_indented",
    "syntax:heading_setext",
    "syntax:html_block",
    "syntax:raw_html_inline",
    "syntax:math_inline_candidate",
    "syntax:math_display_candidate",
    "table:ordinary",
    "table:wide",
    "table:long",
    "table:alignment_marker",
    "table:escaped_pipe_candidate",
    "table:inline_inside_table",
    "extra:strong",
    "extra:hard_break",
    "extra:soft_break",
    "extra:strikethrough",
    "extra:task_list_item",
    "extra:front_matter",
    "extra:directive",
];

fn grade_rank(grade: &str) -> u64 {
    match grade {
        "strict" => 4,
        "declared" => 3,
        "candidate" => 2,
        "ambiguous_or_unknown" => 1,
        _ => 0,
    }
}

fn grade_rank_label(grade: u64) -> &'static str {
    match grade {
        4 => "strict",
        3 => "declared",
        2 => "candidate",
        1 => "ambiguous_or_unknown",
        _ => "none",
    }
}

/// Whether the optional fifth joint cell is included, under the frozen
/// variation rule: at least three distinct observed `max_container_depth`
/// values among G0-strict-eligible candidates AND p90 depth >= 2.
pub fn optional_joint_included(eligible: &[&CandidateRow]) -> bool {
    let mut depths: Vec<f64> = eligible
        .iter()
        .map(|row| {
            row.g0()
                .map(|lane| lane.max_container_depth as f64)
                .unwrap_or(0.0)
        })
        .collect();
    depths.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let distinct: BTreeSet<u64> = depths.iter().map(|depth| *depth as u64).collect();
    let p90 = crate::stats::quantile_sorted(&depths, 0.90);
    distinct.len() >= 3 && p90 >= 2.0
}

struct CellState {
    cell: String,
    members: BTreeSet<usize>,
}

/// Run the full four-set selection over the profiled universe (§25-§36).
// The ranking bookkeeping tuples are local to this frozen algorithm;
// factoring them out would churn the selection code for lint aesthetics.
#[allow(clippy::type_complexity)]
pub fn select(
    rows: &[CandidateRow],
    redundancy: &RedundancyArtifact,
) -> Result<SelectionOutcome, String> {
    let eligible: Vec<&CandidateRow> = rows.iter().filter(|row| row.g0_strict_eligible()).collect();
    if eligible.is_empty() {
        return Err(
            "no G0-strict-eligible candidates: representative selection impossible".to_string(),
        );
    }
    let usable: Vec<&CandidateRow> = rows
        .iter()
        .filter(|row| row.materialized && row.hash_match && row.profile_failure.is_none())
        .collect();

    // ---- bins over the eligible population (frozen rule, §23) ----
    let mut bins = Vec::new();
    for feature in FEATURES {
        let values: Vec<f64> = eligible
            .iter()
            .map(|row| feature_value(row, feature))
            .collect();
        bins.push(derive_bins(
            feature,
            ZERO_MEANINGFUL.contains(&feature),
            &values,
        ));
    }
    // ---- attainable mandatory cells (§24-§25) ----
    let include_optional = optional_joint_included(&eligible);
    let mut joints: Vec<(&str, &str)> = JOINT_CELLS.to_vec();
    if include_optional {
        joints.push(OPTIONAL_JOINT);
    }
    let mut mandatory: Vec<CellState> = Vec::new();
    for bin in &bins {
        for label in &bin.bins {
            let members: BTreeSet<usize> = eligible
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    crate::stats::bin_of(bin, feature_value(row, &bin.feature)) == *label
                })
                .map(|(index, _)| index)
                .collect();
            if !members.is_empty() {
                mandatory.push(CellState {
                    cell: format!("{}={}", bin.feature, label),
                    members,
                });
            }
        }
    }
    for (feature_a, feature_b) in &joints {
        let bin_a = bins.iter().find(|bin| bin.feature == *feature_a).unwrap();
        let bin_b = bins.iter().find(|bin| bin.feature == *feature_b).unwrap();
        for label_a in &bin_a.bins {
            for label_b in &bin_b.bins {
                let members: BTreeSet<usize> = eligible
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| {
                        crate::stats::bin_of(bin_a, feature_value(row, feature_a)) == *label_a
                            && crate::stats::bin_of(bin_b, feature_value(row, feature_b))
                                == *label_b
                    })
                    .map(|(index, _)| index)
                    .collect();
                if !members.is_empty() {
                    mandatory.push(CellState {
                        cell: format!("{feature_a}={label_a}×{feature_b}={label_b}"),
                        members,
                    });
                }
            }
        }
    }
    let eligible_domains: BTreeSet<&str> = eligible
        .iter()
        .map(|row| row.identity.domain.as_str())
        .collect();
    let cells_of = |index: usize| -> Vec<String> {
        mandatory
            .iter()
            .filter(|state| state.members.contains(&index))
            .map(|state| state.cell.clone())
            .collect()
    };
    let total_mandatory = mandatory.len();
    let attainable_cells: Vec<String> = mandatory.iter().map(|state| state.cell.clone()).collect();

    let mut trace: Vec<TraceRecord> = Vec::new();
    let mut cap_relaxations: Vec<String> = Vec::new();

    // ---- representative greedy (§25.2-§25.3) ----
    let mut selected_rep: Vec<usize> = Vec::new();
    let mut covered: BTreeSet<String> = BTreeSet::new();
    let mut covered_domains: BTreeSet<String> = BTreeSet::new();

    let project_count = |selected: &[usize], project: &str| -> u64 {
        selected
            .iter()
            .filter(|index| eligible[**index].identity.source_id == project)
            .count() as u64
    };
    let domain_count = |selected: &[usize], domain: &str| -> u64 {
        selected
            .iter()
            .filter(|index| eligible[**index].identity.domain == domain)
            .count() as u64
    };

    let mut iteration: u64 = 0;
    loop {
        iteration += 1;
        // Two passes: cap-compliant candidates first; cap-violating
        // candidates only when no compliant candidate covers a new cell
        // (mechanical relaxation, §25.1). Ordering (§25.2) is a single
        // comparable tuple: more new cells, then least-represented
        // domain, then least-represented project, then fewer
        // near-duplicate conflicts, then lexical.
        let selected_before: Vec<&CandidateRow> =
            selected_rep.iter().map(|index| eligible[*index]).collect();
        type RepKey = (Reverse<usize>, u64, u64, u64, (String, String));
        let rep_key = |index: usize, gain: usize| -> RepKey {
            let row = eligible[index];
            (
                Reverse(gain),
                domain_count(&selected_rep, &row.identity.domain),
                project_count(&selected_rep, &row.identity.source_id),
                crate::redundancy::near_conflict_count(redundancy, row, &selected_before),
                (
                    row.identity.source_id.clone(),
                    row.identity.snapshot_path.clone(),
                ),
            )
        };
        let mut best_compliant: Option<(RepKey, usize, Vec<String>)> = None;
        let mut best_violating: Option<(RepKey, usize, Vec<String>, String)> = None;
        for (index, row) in eligible.iter().enumerate() {
            if selected_rep.contains(&index) {
                continue;
            }
            let cells = cells_of(index);
            let new_cells: Vec<String> = cells
                .iter()
                .filter(|cell| !covered.contains(*cell))
                .cloned()
                .collect();
            let new_domain = !covered_domains.contains(row.identity.domain.as_str());
            let gain = new_cells.len() + usize::from(new_domain);
            if gain == 0 {
                continue;
            }
            let ordering = rep_key(index, gain);
            let n_after = selected_rep.len() + 1;
            let domain_allowed = (domain_count(&selected_rep, &row.identity.domain) + 1) as f64
                <= (DOMAIN_CAP_SHARE * n_after as f64).floor().max(1.0);
            let project_allowed =
                project_count(&selected_rep, &row.identity.source_id) < PROJECT_CAP;
            if domain_allowed && project_allowed {
                if best_compliant
                    .as_ref()
                    .map(|(best, _, _)| &ordering < best)
                    .unwrap_or(true)
                {
                    best_compliant = Some((ordering, index, new_cells));
                }
            } else {
                let violation = if !project_allowed {
                    format!(
                        "project cap {PROJECT_CAP} exceeded for {}",
                        row.identity.source_id
                    )
                } else {
                    format!(
                        "domain cap {:.0}% exceeded for {}",
                        DOMAIN_CAP_SHARE * 100.0,
                        row.identity.domain
                    )
                };
                let better = best_violating
                    .as_ref()
                    .map(|(best, _, _, _)| &ordering < best)
                    .unwrap_or(true);
                if better {
                    best_violating = Some((ordering, index, new_cells, violation));
                }
            }
        }

        let (index, new_cells, relaxation) = match (best_compliant, best_violating) {
            (Some((_, index, cells)), _) => (index, cells, None),
            (None, Some((_, index, cells, violation))) => {
                let note = format!(
                    "cap relaxation at iteration {iteration}: {} ({violation}) admitted because no cap-compliant candidate covers a remaining mandatory cell",
                    key(eligible[index])
                );
                cap_relaxations.push(note.clone());
                (index, cells, Some(note))
            }
            (None, None) => break,
        };

        let row = eligible[index];
        let domain_before = covered_domains.len() as u64;
        let project_before = selected_rep
            .iter()
            .map(|index| eligible[*index].identity.source_id.clone())
            .collect::<BTreeSet<String>>()
            .len() as u64;
        let near_flags_before = crate::redundancy::near_conflict_count(
            redundancy,
            row,
            &selected_rep
                .iter()
                .map(|index| eligible[*index])
                .collect::<Vec<_>>(),
        );
        for cell in &new_cells {
            covered.insert(cell.clone());
        }
        let new_domain = covered_domains.insert(row.identity.domain.clone());
        selected_rep.push(index);
        trace.push(TraceRecord {
            schema: SELECTION_TRACE_SCHEMA.to_string(),
            set: "representative".to_string(),
            iteration,
            selected: key(row),
            duplicated_cells: cells_of(index).len() as u64 - new_cells.len() as u64,
            new_cells: new_cells.clone(),
            domain_count_before: domain_before,
            domain_count_after: covered_domains.len() as u64,
            project_count_before: project_before,
            project_count_after: selected_rep
                .iter()
                .map(|index| eligible[*index].identity.source_id.clone())
                .collect::<BTreeSet<String>>()
                .len() as u64,
            near_duplicate_flags: near_flags_before,
            tie_break: vec![
                ("new_cells".to_string(), new_cells.len().to_string()),
                (
                    "domain_representation".to_string(),
                    domain_count(
                        &selected_rep[..selected_rep.len() - 1],
                        &row.identity.domain,
                    )
                    .to_string(),
                ),
                (
                    "project_representation".to_string(),
                    project_count(
                        &selected_rep[..selected_rep.len() - 1],
                        &row.identity.source_id,
                    )
                    .to_string(),
                ),
                ("lexical".to_string(), row.identity.snapshot_path.clone()),
            ],
            remaining_uncovered: (total_mandatory - covered.len()) as u64,
            cap_relaxation: relaxation,
            rejected: None,
            why: format!(
                "covers {} new mandatory cell(s){}; domain {} project {}",
                new_cells.len(),
                if new_domain {
                    " + 1 new domain stratum"
                } else {
                    ""
                },
                row.identity.domain,
                row.identity.source_id
            ),
        });
        if covered.len() >= total_mandatory {
            break;
        }
    }

    let plateau_note = if selected_rep.len() < 20 {
        Some(format!(
            "UNDER_TARGET_DUE_TO_COVERAGE_PLATEAU: representative selection stopped at {} files (< 20); all attainable mandatory cells covered: {}",
            selected_rep.len(),
            covered.len() == total_mandatory
        ))
    } else if selected_rep.len() > 30 {
        Some(format!(
            "TARGET_OVERRUN_JUSTIFIED: {} files selected; the trace shows each file added at least one new mandatory cell",
            selected_rep.len()
        ))
    } else {
        None
    };

    // ---- extremal set (§27) ----
    let mut extremal_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut extremal_records: Vec<TraceRecord> = Vec::new();
    let mut extremal_iteration = 0u64;
    for feature in FEATURES {
        let mut ranked: Vec<&CandidateRow> = usable.clone();
        ranked.sort_by(|a, b| {
            feature_value(b, feature)
                .partial_cmp(&feature_value(a, feature))
                .unwrap()
                .then_with(|| a.lexical_key().cmp(&b.lexical_key()))
        });
        let winner = ranked[0];
        extremal_iteration += 1;
        extremal_members
            .entry(key(winner))
            .or_default()
            .push(format!("{feature}:OBSERVED_MAXIMUM"));
        extremal_records.push(TraceRecord {
            schema: SELECTION_TRACE_SCHEMA.to_string(),
            set: "extremal".to_string(),
            iteration: extremal_iteration,
            selected: key(winner),
            new_cells: vec![format!("{feature}:OBSERVED_MAXIMUM")],
            duplicated_cells: 0,
            domain_count_before: 0,
            domain_count_after: 0,
            project_count_before: 0,
            project_count_after: 0,
            near_duplicate_flags: 0,
            tie_break: vec![(
                "observed_maximum".to_string(),
                format!("{}", feature_value(winner, feature)),
            )],
            remaining_uncovered: 0,
            cap_relaxation: None,
            rejected: None,
            why: format!(
                "{feature} observed maximum over the materialized, hash-verified, profiled candidate universe"
            ),
        });
        let top_n = ((TAIL_TOP_PERCENT * usable.len() as f64).ceil() as usize).max(2);
        let replicate = ranked[..top_n.min(ranked.len())]
            .iter()
            .find(|candidate| candidate.identity.source_id != winner.identity.source_id)
            .copied();
        match replicate {
            Some(replicate_row) => {
                extremal_iteration += 1;
                extremal_members
                    .entry(key(replicate_row))
                    .or_default()
                    .push(format!("{feature}:TAIL_REPLICATE"));
                extremal_records.push(TraceRecord {
                    schema: SELECTION_TRACE_SCHEMA.to_string(),
                    set: "extremal".to_string(),
                    iteration: extremal_iteration,
                    selected: key(replicate_row),
                    new_cells: vec![format!("{feature}:TAIL_REPLICATE")],
                    duplicated_cells: 0,
                    domain_count_before: 0,
                    domain_count_after: 0,
                    project_count_before: 0,
                    project_count_after: 0,
                    near_duplicate_flags: 0,
                    tie_break: vec![(
                        "top_percentile_rank_position".to_string(),
                        format!("{}", ranked.iter().position(|r| std::ptr::eq(*r, replicate_row)).unwrap_or(0)),
                    )],
                    remaining_uncovered: 0,
                    cap_relaxation: None,
                    rejected: None,
                    why: format!(
                        "distinct-project replicate within the top {TAIL_TOP_PERCENT:.0}% of {feature} (project {})",
                        replicate_row.identity.source_id
                    ),
                });
            }
            None => {
                extremal_records.push(TraceRecord {
                    schema: SELECTION_TRACE_SCHEMA.to_string(),
                    set: "extremal".to_string(),
                    iteration: 0,
                    selected: format!("NONE_AVAILABLE({feature})"),
                    new_cells: vec![],
                    duplicated_cells: 0,
                    domain_count_before: 0,
                    domain_count_after: 0,
                    project_count_before: 0,
                    project_count_after: 0,
                    near_duplicate_flags: 0,
                    tie_break: vec![],
                    remaining_uncovered: 0,
                    cap_relaxation: None,
                    rejected: None,
                    why: format!(
                        "no distinct-project replicate within the top {TAIL_TOP_PERCENT:.0}% of {feature}"
                    ),
                });
            }
        }
    }

    // ---- full-document set (§33-§35) ----
    let mut full_document: Vec<String> = Vec::new();
    let mut full_document_rejections: Vec<String> = Vec::new();
    let hard_candidates: [(&str, &str, &str); 3] = [
        (
            "cpp-core-guidelines",
            "files/CppCoreGuidelines.md",
            "FULL-CPP-CORE",
        ),
        ("node", "files/doc/api/fs.md", "FULL-NODE-FS"),
        (
            "d2l-en",
            "files/chapter_preliminaries/linear-algebra.md",
            "FULL-D2L-LINEAR-ALGEBRA",
        ),
    ];
    let mut full_doc_records: Vec<TraceRecord> = Vec::new();
    for (source_id, snapshot_path, label) in hard_candidates {
        match usable.iter().find(|row| {
            row.identity.source_id == source_id && row.identity.snapshot_path == snapshot_path
        }) {
            Some(row) => {
                full_document.push(key(row));
                full_doc_records.push(TraceRecord {
                    schema: SELECTION_TRACE_SCHEMA.to_string(),
                    set: "full_document".to_string(),
                    iteration: full_document.len() as u64,
                    selected: key(row),
                    new_cells: vec![format!("{label}:named_hard_candidate")],
                    duplicated_cells: 0,
                    domain_count_before: 0,
                    domain_count_after: 0,
                    project_count_before: 0,
                    project_count_after: 0,
                    near_duplicate_flags: 0,
                    tie_break: vec![("named_candidate".to_string(), label.to_string())],
                    remaining_uncovered: 0,
                    cap_relaxation: None,
                    rejected: None,
                    why: format!(
                        "{label}: named hard candidate, present, byte-valid and profiled; G0 eligible = {}, G1 eligible = {}",
                        row.g0_strict_eligible(),
                        row.g1().map(|lane| lane.strict_scope_clean).unwrap_or(false)
                    ),
                });
            }
            None => {
                full_document_rejections.push(format!(
                    "{label}: {source_id}/{snapshot_path} not present or not byte-valid in the frozen universe"
                ));
                full_doc_records.push(TraceRecord {
                    schema: SELECTION_TRACE_SCHEMA.to_string(),
                    set: "full_document".to_string(),
                    iteration: full_document.len() as u64,
                    selected: format!("REJECTED({label})"),
                    new_cells: vec![],
                    duplicated_cells: 0,
                    domain_count_before: 0,
                    domain_count_after: 0,
                    project_count_before: 0,
                    project_count_after: 0,
                    near_duplicate_flags: 0,
                    tie_break: vec![],
                    remaining_uncovered: 0,
                    cap_relaxation: None,
                    rejected: Some(format!(
                        "{source_id}/{snapshot_path} absent or not byte-valid"
                    )),
                    why: format!("{label}: named candidate rejected by the frozen validity rule"),
                });
            }
        }
    }

    // Mechanical full documents: frozen percentile-rank formulas (§34).
    let rank_file = |source_rows: &[&CandidateRow],
                     row: &CandidateRow,
                     features: &[&str]|
     -> (f64, Vec<(String, f64)>) {
        let mut total = 0.0;
        let mut components = Vec::new();
        for feature in features {
            let mut values: Vec<f64> = source_rows
                .iter()
                .map(|candidate| rank_feature_value(candidate, feature))
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let component = percentile_rank(&values, rank_feature_value(row, feature));
            components.push((feature.to_string(), component));
            total += component;
        }
        (total / features.len() as f64, components)
    };
    let mut add_mechanical_full_doc = |source_id: &str,
                                       features: &[&str],
                                       label: &str,
                                       math_note: bool|
     -> Result<(), String> {
        let rows_for_source: Vec<&CandidateRow> = usable
            .iter()
            .filter(|row| row.identity.source_id == source_id)
            .copied()
            .collect();
        if rows_for_source.is_empty() {
            // An absent source is recorded as a rejection, never a
            // silent skip and never a hard failure of the whole
            // selection.
            full_document_rejections.push(format!(
                "{label}: no usable candidates from {source_id} in this universe"
            ));
            full_doc_records.push(TraceRecord {
                schema: SELECTION_TRACE_SCHEMA.to_string(),
                set: "full_document".to_string(),
                iteration: full_document.len() as u64,
                selected: format!("REJECTED({label})"),
                new_cells: vec![],
                duplicated_cells: 0,
                domain_count_before: 0,
                domain_count_after: 0,
                project_count_before: 0,
                project_count_after: 0,
                near_duplicate_flags: 0,
                tie_break: vec![],
                remaining_uncovered: 0,
                cap_relaxation: None,
                rejected: Some(format!("no usable candidates from {source_id}")),
                why: format!("{label}: mechanical rank could not run — source absent"),
            });
            return Ok(());
        }
        let mut ranked: Vec<(&CandidateRow, f64, Vec<(String, f64)>)> = rows_for_source
            .iter()
            .map(|row| {
                let (score, components) = rank_file(&rows_for_source, row, features);
                (*row, score, components)
            })
            .collect();
        ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap()
                .then_with(|| a.0.lexical_key().cmp(&b.0.lexical_key()))
        });
        let (winner, score, components) = ranked[0].clone();
        full_document.push(key(winner));
        full_doc_records.push(TraceRecord {
                schema: SELECTION_TRACE_SCHEMA.to_string(),
                set: "full_document".to_string(),
                iteration: full_document.len() as u64,
                selected: key(winner),
                new_cells: vec![format!("{label}:mechanical_rank")],
                duplicated_cells: 0,
                domain_count_before: 0,
                domain_count_after: 0,
                project_count_before: 0,
                project_count_after: 0,
                near_duplicate_flags: 0,
                tie_break: components
                    .iter()
                    .map(|(feature, component)| (feature.clone(), format!("{component:.6}")))
                    .chain(std::iter::once(("total_score".to_string(), format!("{score:.6}"))))
                    .collect(),
                remaining_uncovered: 0,
                cap_relaxation: None,
                rejected: None,
                why: if math_note {
                    format!(
                        "{label}: equal-weight percentile rank over {}; math evidence is G1 ambiguous/unknown candidate occurrences only (G2 deferred — no fabricated occupancy)",
                        features.join(", ")
                    )
                } else {
                    format!(
                        "{label}: equal-weight percentile rank over {} (component ranks in tie_break)",
                        features.join(", ")
                    )
                },
            });
        Ok(())
    };

    add_mechanical_full_doc(
        "openmlsys",
        &[
            "file_bytes",
            "cjk_byte_share",
            "fence_density_per_kib",
            MATH_CANDIDATE_FEATURE,
        ],
        "FULL-OPENMLSYS",
        true,
    )?;
    add_mechanical_full_doc(
        "kubernetes-keps",
        &[
            "file_bytes",
            "block_count",
            "max_container_depth",
            "fence_density_per_kib",
        ],
        "FULL-KEP",
        false,
    )?;
    add_mechanical_full_doc(
        "rust-rfcs",
        &["file_bytes", "block_count", "reference_density_per_kib"],
        "FULL-RUST-RFC",
        false,
    )?;

    // Optional seventh document (§35): only when it covers a material
    // regime the first six do not — mechanically, a domain stratum or a
    // required syntax cell still uncovered.
    let mut seventh_document_note: Option<String> = None;
    let six_keys: BTreeSet<&String> = full_document.iter().collect();
    let six_rows: Vec<&CandidateRow> = usable
        .iter()
        .filter(|row| six_keys.contains(&key(row)))
        .copied()
        .collect();
    let six_domains: BTreeSet<&str> = six_rows
        .iter()
        .map(|row| row.identity.domain.as_str())
        .collect();
    let uncovered_domain: Option<&str> = usable
        .iter()
        .map(|row| row.identity.domain.as_str())
        .collect::<BTreeSet<&str>>()
        .into_iter()
        .find(|domain| !six_domains.contains(domain));
    let six_syntax: BTreeSet<&str> = SYNTAX_CELLS
        .iter()
        .filter(|cell| six_rows.iter().any(|row| syntax_cell(row, cell).is_some()))
        .copied()
        .collect();
    let uncovered_syntax: Option<&str> = SYNTAX_CELLS
        .iter()
        .find(|cell| {
            !six_syntax.contains(*cell) && usable.iter().any(|row| syntax_cell(row, cell).is_some())
        })
        .copied();
    if let Some(trigger) = uncovered_domain.or(uncovered_syntax) {
        // Candidate = largest-bytes usable row covering the trigger.
        let mut candidates: Vec<&&CandidateRow> = usable
            .iter()
            .filter(|row| match uncovered_domain {
                Some(domain) => row.identity.domain == domain,
                None => true,
            })
            .filter(|row| match uncovered_domain {
                Some(_) => true,
                None => syntax_cell(row, trigger).is_some(),
            })
            .collect();
        candidates.sort_by(|a, b| {
            b.file_bytes
                .cmp(&a.file_bytes)
                .then_with(|| a.lexical_key().cmp(&b.lexical_key()))
        });
        if let Some(seventh) = candidates.first() {
            let seventh = **seventh;
            if !six_keys.contains(&key(seventh)) {
                full_document.push(key(seventh));
                full_doc_records.push(TraceRecord {
                    schema: SELECTION_TRACE_SCHEMA.to_string(),
                    set: "full_document".to_string(),
                    iteration: full_document.len() as u64,
                    selected: key(seventh),
                    new_cells: vec![format!("seventh:uncovered_regime:{trigger}")],
                    duplicated_cells: 0,
                    domain_count_before: six_domains.len() as u64,
                    domain_count_after: six_domains
                        .iter()
                        .chain(std::iter::once(&seventh.identity.domain.as_str()))
                        .collect::<BTreeSet<&&str>>()
                        .len() as u64,
                    project_count_before: 0,
                    project_count_after: 0,
                    near_duplicate_flags: 0,
                    tie_break: vec![
                        ("trigger".to_string(), trigger.to_string()),
                        ("file_bytes".to_string(), seventh.file_bytes.to_string()),
                    ],
                    remaining_uncovered: 0,
                    cap_relaxation: None,
                    rejected: None,
                    why: format!(
                        "seventh document justified by uncovered regime {trigger}; largest-bytes deterministic choice"
                    ),
                });
                seventh_document_note = Some(format!(
                    "seventh document added for uncovered regime {trigger}"
                ));
            }
        }
    } else {
        seventh_document_note = Some(
            "seventh document not justified: the six documents already cover every observed domain stratum and every attainable required syntax cell".to_string(),
        );
    }

    // ---- syntax set-cover (§31), seeded from rep + extremal + full ----
    let mut covered_syntax: BTreeMap<String, String> = BTreeMap::new();
    let mut seed: Vec<&CandidateRow> = Vec::new();
    for index in &selected_rep {
        seed.push(eligible[*index]);
    }
    let extremal_keys: BTreeSet<String> = extremal_members.keys().cloned().collect();
    for row in &usable {
        if extremal_keys.contains(&key(row)) {
            seed.push(row);
        }
    }
    for row in &usable {
        if full_document.contains(&key(row)) {
            seed.push(row);
        }
    }
    for row in &seed {
        for cell in SYNTAX_CELLS {
            if syntax_cell(row, cell).is_some() {
                covered_syntax
                    .entry(cell.to_string())
                    .or_insert_with(|| key(row));
            }
        }
    }

    let mut syntax_selected: Vec<String> = Vec::new();
    let mut syntax_records: Vec<TraceRecord> = Vec::new();
    let mut syntax_iteration = 0u64;
    loop {
        syntax_iteration += 1;
        let seed_keys: BTreeSet<String> = seed.iter().map(|row| key(row)).collect();
        let chosen_so_far: BTreeSet<String> = syntax_selected.iter().cloned().collect();
        let known_projects: BTreeSet<String> = seed
            .iter()
            .map(|row| row.identity.source_id.clone())
            .chain(syntax_selected.iter().map(|key| {
                let (project, _) = key.split_once('/').expect("key form source/path");
                project.to_string()
            }))
            .collect();
        let known_domains: BTreeSet<String> =
            seed.iter().map(|row| row.identity.domain.clone()).collect();
        // §31 ordering: stronger evidence grade, more new cells, new
        // project, new domain, lexical. Stored per candidate as a single
        // comparable tuple.
        let mut best: Option<(
            (
                Reverse<u64>,
                Reverse<usize>,
                Reverse<u64>,
                Reverse<u64>,
                (String, String),
            ),
            &CandidateRow,
            Vec<String>,
            u64,
            bool,
            bool,
        )> = None;
        for row in &usable {
            let row_key = key(row);
            if chosen_so_far.contains(&row_key) || seed_keys.contains(&row_key) {
                continue;
            }
            let new_cells: Vec<String> = SYNTAX_CELLS
                .iter()
                .filter(|cell| {
                    !covered_syntax.contains_key(**cell) && syntax_cell(row, cell).is_some()
                })
                .map(|cell| cell.to_string())
                .collect();
            if new_cells.is_empty() {
                continue;
            }
            let best_grade = new_cells
                .iter()
                .filter_map(|cell| syntax_cell(row, cell))
                .map(grade_rank)
                .max()
                .unwrap_or(0);
            let new_project = !known_projects.contains(&row.identity.source_id);
            let new_domain = !known_domains.contains(&row.identity.domain);
            let ordering = (
                Reverse(best_grade),
                Reverse(new_cells.len()),
                Reverse(new_project as u64),
                Reverse(new_domain as u64),
                (
                    row.identity.source_id.clone(),
                    row.identity.snapshot_path.clone(),
                ),
            );
            let better = best
                .as_ref()
                .map(|(current, _, _, _, _, _)| &ordering < current)
                .unwrap_or(true);
            if better {
                best = Some((
                    ordering,
                    row,
                    new_cells,
                    best_grade,
                    new_project,
                    new_domain,
                ));
            }
        }
        let Some((_, row, new_cells, best_grade, new_project, new_domain)) = best else {
            break;
        };
        for cell in &new_cells {
            covered_syntax.insert(cell.clone(), key(row));
        }
        syntax_selected.push(key(row));
        syntax_records.push(TraceRecord {
            schema: SELECTION_TRACE_SCHEMA.to_string(),
            set: "syntax_coverage".to_string(),
            iteration: syntax_iteration,
            selected: key(row),
            new_cells: new_cells.clone(),
            duplicated_cells: 0,
            domain_count_before: 0,
            domain_count_after: 0,
            project_count_before: 0,
            project_count_after: 0,
            near_duplicate_flags: 0,
            tie_break: vec![
                (
                    "evidence_grade".to_string(),
                    grade_rank_label(best_grade).to_string(),
                ),
                ("new_cells".to_string(), new_cells.len().to_string()),
                ("new_project".to_string(), new_project.to_string()),
                ("new_domain".to_string(), new_domain.to_string()),
                ("lexical".to_string(), row.identity.snapshot_path.clone()),
            ],
            remaining_uncovered: SYNTAX_CELLS
                .iter()
                .filter(|cell| !covered_syntax.contains_key(**cell))
                .count() as u64,
            cap_relaxation: None,
            rejected: None,
            why: format!(
                "adds {} new syntax/context cell(s) at evidence grade {}",
                new_cells.len(),
                grade_rank_label(best_grade)
            ),
        });
    }

    // ---- assemble members (§36: physical dedup across sets) ----
    let mut members: BTreeMap<String, SelectedMember> = BTreeMap::new();
    fn ensure<'m>(
        row: &CandidateRow,
        members: &'m mut BTreeMap<String, SelectedMember>,
    ) -> &'m mut SelectedMember {
        let value = SelectedMember {
            key: key(row),
            memberships: Vec::new(),
            extreme_roles: Vec::new(),
            why_this_file: Vec::new(),
            domain: row.identity.domain.clone(),
            evidence_labels: Vec::new(),
            transition_opportunity: BTreeMap::new(),
            g0_strict_eligible: row.g0_strict_eligible(),
            g1_strict_eligible: row
                .g1()
                .map(|lane| lane.lane_valid && lane.strict_scope_clean)
                .unwrap_or(false),
            file_bytes: row.file_bytes,
            sha256: row.source_sha256.clone(),
        };
        members.entry(key(row)).or_insert(value)
    }
    for index in &selected_rep {
        let row = eligible[*index];
        let record_why = trace
            .iter()
            .find(|record| record.set == "representative" && record.selected == key(row))
            .map(|record| format!("representative: {}", record.why))
            .unwrap_or_else(|| "representative".to_string());
        let member = ensure(row, &mut members);
        member.memberships.push("representative".to_string());
        member.why_this_file.push(record_why);
    }
    for (member_key, roles) in &extremal_members {
        let row = usable
            .iter()
            .find(|row| key(row) == *member_key)
            .ok_or_else(|| format!("extremal member {member_key} not found"))?;
        let member = ensure(row, &mut members);
        member.memberships.push("extremal".to_string());
        member.extreme_roles.extend(roles.iter().cloned());
        member
            .why_this_file
            .push(format!("extremal roles: {}", roles.join(", ")));
    }
    for row_key in &full_document {
        let row = usable
            .iter()
            .find(|row| key(row) == *row_key)
            .ok_or_else(|| format!("full-document member {row_key} not found"))?;
        let record_why = full_doc_records
            .iter()
            .find(|record| record.selected == *row_key)
            .map(|record| format!("full_document: {}", record.why))
            .unwrap_or_else(|| "full_document".to_string());
        let member = ensure(row, &mut members);
        member.memberships.push("full_document".to_string());
        member.why_this_file.push(record_why);
    }
    for row_key in &syntax_selected {
        let row = usable
            .iter()
            .find(|row| key(row) == *row_key)
            .ok_or_else(|| format!("syntax member {row_key} not found"))?;
        let record = syntax_records
            .iter()
            .find(|record| record.selected == *row_key)
            .cloned();
        let member = ensure(row, &mut members);
        member.memberships.push("syntax_coverage".to_string());
        member.why_this_file.push(format!(
            "syntax_coverage: {}",
            record
                .as_ref()
                .map(|record| record.why.clone())
                .unwrap_or_default()
        ));
        // Evidence labels (§30): a file whose selection value is only
        // non-strict evidence is REALISM_ONLY; deferred math adds
        // LANE_DEFERRED, out-of-lane extensions add
        // GRAMMAR_EXTENSION_REQUIRED.
        if let Some(record) = &record {
            let grades: Vec<&str> = record
                .new_cells
                .iter()
                .filter_map(|cell| syntax_cell(row, cell))
                .collect();
            let non_strict_only =
                !grades.is_empty() && grades.iter().all(|grade| *grade != "strict");
            if non_strict_only {
                member.evidence_labels.push("REALISM_ONLY".to_string());
                if grades.contains(&"ambiguous_or_unknown") {
                    member.evidence_labels.push("LANE_DEFERRED".to_string());
                }
                if grades.contains(&"candidate") {
                    member
                        .evidence_labels
                        .push("GRAMMAR_EXTENSION_REQUIRED".to_string());
                }
            }
        }
    }
    // Transition opportunities (§32): booleans only, never edits.
    let keys: Vec<String> = members.keys().cloned().collect();
    for member_key in keys {
        if let Some(row) = usable.iter().find(|row| key(row) == member_key) {
            let member = members.get_mut(&member_key).unwrap();
            member.transition_opportunity.insert(
                "g0_atx_heading_to_paragraph".to_string(),
                row.g0()
                    .map(|lane| lane.strict_kinds.get("heading_atx").copied().unwrap_or(0) > 0)
                    .unwrap_or(false),
            );
            member.transition_opportunity.insert(
                "g1_table_delimiter_transition".to_string(),
                row.g1().map(|lane| lane.table_count > 0).unwrap_or(false),
            );
        }
    }

    let mut final_trace = trace;
    final_trace.extend(extremal_records);
    final_trace.extend(full_doc_records);
    final_trace.extend(syntax_records);

    let uncovered_cells: Vec<String> = mandatory
        .iter()
        .map(|state| state.cell.clone())
        .filter(|cell| !covered.contains(cell))
        .collect();

    Ok(SelectionOutcome {
        representative: selected_rep
            .iter()
            .map(|index| key(eligible[*index]))
            .collect(),
        extremal: extremal_keys.into_iter().collect(),
        syntax_coverage: syntax_selected,
        full_document,
        trace: final_trace,
        members,
        bins,
        attainable_cells,
        uncovered_cells,
        eligible_domains: eligible_domains.into_iter().map(str::to_string).collect(),
        plateau_note,
        cap_relaxations,
        full_document_rejections,
        seventh_document_note,
    })
}
