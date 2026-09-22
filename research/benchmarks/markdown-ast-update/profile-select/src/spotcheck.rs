//! Deterministic validation samples (CORRECTIVE-B §45): pick one file per
//! declared category by a frozen rule, re-profile it, and verify sampled
//! fact spans against the actual source bytes.

use std::path::Path;

use serde::Serialize;

use crate::rows::span_evidence_ok;
use crate::{CandidateIdentity, CandidateRow, SPOT_CHECK_SCHEMA};

#[derive(Debug, Clone, Serialize)]
pub struct SpotCheckFact {
    pub syntax_kind: String,
    pub grammar_id: String,
    pub span: (usize, usize),
    pub result: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpotCheck {
    pub category: String,
    pub pick_rule: String,
    pub key: String,
    pub sha256_match: bool,
    pub facts_checked: u64,
    pub facts_failed: u64,
    pub failures: Vec<SpotCheckFact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpotCheckArtifact {
    pub schema: String,
    pub checks: Vec<SpotCheck>,
    pub all_pass: bool,
}

const MAX_FACTS_PER_FILE: usize = 12;

fn make_check(
    workloads_root: &Path,
    category: &str,
    pick_rule: &str,
    identity: &CandidateIdentity,
) -> Result<SpotCheck, String> {
    let bytes = std::fs::read(crate::universe::materialized_path(workloads_root, identity))
        .map_err(|error| format!("{}: {error}", identity.snapshot_path))?;
    let digest = crate::sha256_hex(&bytes);
    let sha256_match = digest == identity.sha256;
    let mut failures = Vec::new();
    let mut facts_checked = 0u64;
    if let Ok(source) = std::str::from_utf8(&bytes) {
        // Sample facts deterministically: re-profile and walk lanes in
        // order, taking up to MAX_FACTS_PER_FILE facts spread across
        // kinds (one per kind first, then by source order).
        let full = markit_mdbench_semantics::profile(source, &crate::PROFILED_GRAMMAR_IDS);
        let mut sampled: Vec<&markit_mdbench_semantics::SyntaxFact> = Vec::new();
        let mut seen_kinds = std::collections::BTreeSet::new();
        for lane in &full.lanes {
            for fact in &lane.syntax_facts {
                let kind_key = format!("{}:{}", lane.grammar_id, fact.syntax_kind.name());
                if seen_kinds.insert(kind_key) {
                    sampled.push(fact);
                }
            }
        }
        for lane in &full.lanes {
            for fact in &lane.syntax_facts {
                if sampled.len() >= MAX_FACTS_PER_FILE {
                    break;
                }
                if !sampled.iter().any(|existing| {
                    existing.source_start == fact.source_start
                        && existing.source_end == fact.source_end
                        && existing.grammar_id == lane.grammar_id
                }) {
                    sampled.push(fact);
                }
            }
        }
        for fact in sampled {
            facts_checked += 1;
            if let Err(error) = span_evidence_ok(source, fact) {
                failures.push(SpotCheckFact {
                    syntax_kind: fact.syntax_kind.name().to_string(),
                    grammar_id: fact.grammar_id.clone(),
                    span: (fact.source_start, fact.source_end),
                    result: error,
                });
            }
        }
    }
    Ok(SpotCheck {
        category: category.to_string(),
        pick_rule: pick_rule.to_string(),
        key: format!("{}/{}", identity.source_id, identity.snapshot_path),
        sha256_match,
        facts_checked,
        facts_failed: failures.len() as u64,
        failures,
    })
}

fn g0(row: &CandidateRow) -> &crate::LaneRow {
    row.g0().expect("usable rows carry a G0 lane")
}

/// Run the frozen spot-check suite (§45 categories).
pub fn run(
    workloads_root: &Path,
    rows: &[CandidateRow],
    redundancy: &crate::redundancy::RedundancyArtifact,
) -> Result<SpotCheckArtifact, String> {
    let usable: Vec<&CandidateRow> = rows
        .iter()
        .filter(|row| row.materialized && row.hash_match && row.profile_failure.is_none())
        .collect();
    let strict: Vec<&CandidateRow> = rows.iter().filter(|row| row.g0_strict_eligible()).collect();

    fn best_by<'r>(
        pool: &[&'r CandidateRow],
        rule: &dyn Fn(&CandidateRow) -> f64,
    ) -> &'r CandidateRow {
        let mut sorted: Vec<&&'r CandidateRow> = pool.iter().collect();
        sorted.sort_by(|a, b| {
            rule(b)
                .partial_cmp(&rule(a))
                .unwrap()
                .then_with(|| a.lexical_key().cmp(&b.lexical_key()))
        });
        sorted[0]
    }

    let mut checks = Vec::new();

    fn pick(
        workloads_root: &Path,
        checks: &mut Vec<SpotCheck>,
        category: &str,
        rule_text: &str,
        row: &CandidateRow,
    ) -> Result<(), String> {
        checks.push(make_check(
            workloads_root,
            category,
            rule_text,
            &row.identity,
        )?);
        Ok(())
    }

    // 1. ordinary G0-strict prose doc: most blocks among strict files
    //    with zero fences and zero CJK.
    let prose_pool: Vec<&CandidateRow> = strict
        .iter()
        .copied()
        .filter(|row| g0(row).fence_density_per_kib == 0.0 && row.cjk_byte_share == 0.0)
        .collect();
    if !prose_pool.is_empty() {
        pick(
            workloads_root,
            &mut checks,
            "ordinary_g0_strict_prose_doc",
            "max block_count among G0-strict, fence-free, CJK-free",
            best_by(&prose_pool, &|row| g0(row).block_count as f64),
        )?;
    }
    // 2. fence-heavy technical doc.
    pick(
        workloads_root,
        &mut checks,
        "fence_heavy_doc",
        "max G0 fence_density over usable rows",
        best_by(&usable, &|row| g0(row).fence_density_per_kib),
    )?;
    // 3. reference-heavy doc.
    pick(
        workloads_root,
        &mut checks,
        "reference_heavy_doc",
        "max G0 reference_density over usable rows",
        best_by(&usable, &|row| g0(row).reference_density_per_kib),
    )?;
    // 4. deep container doc.
    pick(
        workloads_root,
        &mut checks,
        "deep_container_doc",
        "max G0 max_container_depth over usable rows",
        best_by(&usable, &|row| g0(row).max_container_depth as f64),
    )?;
    // 5. CJK doc.
    pick(
        workloads_root,
        &mut checks,
        "cjk_doc",
        "max cjk_byte_share over usable rows",
        best_by(&usable, &|row| row.cjk_byte_share),
    )?;
    // 6. recognized G1 Table doc.
    let table_pool: Vec<&CandidateRow> = usable
        .iter()
        .copied()
        .filter(|row| row.g1().map(|lane| lane.table_count > 0).unwrap_or(false))
        .collect();
    if !table_pool.is_empty() {
        pick(
            workloads_root,
            &mut checks,
            "recognized_g1_table_doc",
            "max G1 table_count over usable rows",
            best_by(&table_pool, &|row| {
                row.g1().map(|lane| lane.table_count as f64).unwrap_or(0.0)
            }),
        )?;
    }
    // 7. table-looking text inside a fence: non-host table facts, no
    //    host tables.
    let fenced_table_pool: Vec<&CandidateRow> = usable
        .iter()
        .copied()
        .filter(|row| {
            row.g1()
                .map(|lane| {
                    lane.nonhost_kinds.get("table").copied().unwrap_or(0) > 0
                        && lane.table_count == 0
                })
                .unwrap_or(false)
        })
        .collect();
    if !fenced_table_pool.is_empty() {
        pick(
            workloads_root,
            &mut checks,
            "table_looking_text_inside_fence",
            "max G1 non-host table candidates among rows with zero host tables",
            best_by(&fenced_table_pool, &|row| {
                row.g1()
                    .map(|lane| lane.nonhost_kinds.get("table").copied().unwrap_or(0) as f64)
                    .unwrap_or(0.0)
            }),
        )?;
    }
    // 8. raw HTML-bearing doc.
    let html_pool: Vec<&CandidateRow> = usable
        .iter()
        .copied()
        .filter(|row| {
            row.g1()
                .map(|lane| lane.declared_kinds.get("html_block").copied().unwrap_or(0) > 0)
                .unwrap_or(false)
        })
        .collect();
    if !html_pool.is_empty() {
        pick(
            workloads_root,
            &mut checks,
            "raw_html_bearing_doc",
            "max G1 recognized html_block count over usable rows",
            best_by(&html_pool, &|row| {
                row.g1()
                    .map(|lane| lane.declared_kinds.get("html_block").copied().unwrap_or(0) as f64)
                    .unwrap_or(0.0)
            }),
        )?;
    }
    // 9. image/autolink-bearing doc.
    let link_pool: Vec<&CandidateRow> = usable
        .iter()
        .copied()
        .filter(|row| {
            row.g1()
                .map(|lane| {
                    lane.declared_kinds.get("image").copied().unwrap_or(0)
                        + lane
                            .declared_kinds
                            .get("link_autolink")
                            .copied()
                            .unwrap_or(0)
                        > 0
                })
                .unwrap_or(false)
        })
        .collect();
    if !link_pool.is_empty() {
        pick(
            workloads_root,
            &mut checks,
            "image_autolink_bearing_doc",
            "max G1 image+autolink count over usable rows",
            best_by(&link_pool, &|row| {
                row.g1()
                    .map(|lane| {
                        (lane.declared_kinds.get("image").copied().unwrap_or(0)
                            + lane
                                .declared_kinds
                                .get("link_autolink")
                                .copied()
                                .unwrap_or(0)) as f64
                    })
                    .unwrap_or(0.0)
            }),
        )?;
    }
    // 10. math-candidate doc.
    let math_pool: Vec<&CandidateRow> = usable
        .iter()
        .copied()
        .filter(|row| {
            row.g1()
                .map(|lane| {
                    ["inline_math", "display_math"].iter().any(|kind| {
                        lane.ambiguous_kinds.get(*kind).copied().unwrap_or(0)
                            + lane.unknown_kinds.get(*kind).copied().unwrap_or(0)
                            > 0
                    })
                })
                .unwrap_or(false)
        })
        .collect();
    if !math_pool.is_empty() {
        pick(
            workloads_root,
            &mut checks,
            "math_candidate_doc",
            "max G1 ambiguous/unknown math candidates over usable rows",
            best_by(&math_pool, &|row| {
                row.g1()
                    .map(|lane| {
                        ["inline_math", "display_math"]
                            .iter()
                            .map(|kind| {
                                lane.ambiguous_kinds.get(*kind).copied().unwrap_or(0)
                                    + lane.unknown_kinds.get(*kind).copied().unwrap_or(0)
                            })
                            .sum::<u64>() as f64
                    })
                    .unwrap_or(0.0)
            }),
        )?;
    }
    // 11. G0-ineligible large doc.
    let ineligible: Vec<&CandidateRow> = usable
        .iter()
        .copied()
        .filter(|row| !row.g0_strict_eligible())
        .collect();
    if !ineligible.is_empty() {
        pick(
            workloads_root,
            &mut checks,
            "g0_ineligible_large_doc",
            "max file_bytes among G0-ineligible usable rows",
            best_by(&ineligible, &|row| row.file_bytes as f64),
        )?;
    }
    // 12. one exact duplicate group.
    if let Some(group) = redundancy.exact_groups.first() {
        let member = group.members[0].clone();
        if let Some(row) = rows.iter().find(|row| {
            format!("{}/{}", row.identity.source_id, row.identity.snapshot_path) == member
        }) {
            checks.push(make_check(
                workloads_root,
                "exact_duplicate_group",
                &format!(
                    "first exact-duplicate group ({}, {} members)",
                    &group.sha256[..12],
                    group.members.len()
                ),
                &row.identity,
            )?);
        }
    }
    // 13. one near-duplicate group.
    if let Some(group) = redundancy.near_groups.first() {
        let member = group.members[0].clone();
        if let Some(row) = rows.iter().find(|row| {
            format!("{}/{}", row.identity.source_id, row.identity.snapshot_path) == member
        }) {
            checks.push(make_check(
                workloads_root,
                "near_duplicate_group",
                &format!(
                    "first near-duplicate group (estimated Jaccard {:.3})",
                    group.estimated_jaccard
                ),
                &row.identity,
            )?);
        }
    }

    let all_pass = checks
        .iter()
        .all(|check| check.sha256_match && check.facts_failed == 0 && check.facts_checked > 0);
    Ok(SpotCheckArtifact {
        schema: SPOT_CHECK_SCHEMA.to_string(),
        checks,
        all_pass,
    })
}
