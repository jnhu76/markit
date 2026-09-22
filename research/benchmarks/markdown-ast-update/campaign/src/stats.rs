//! Statistical output contract (task §29-§34, §51-§52).
//!
//! Pure functions over latency samples. In THIS freeze they are
//! exercised only on synthetic data — no real primary timing value
//! exists in this crate. The future RUN task feeds campaign envelopes
//! through the same functions; the contract is frozen now, before any
//! timing is seen.
//!
//! Frozen rules:
//!
//! - **Quantiles** are deterministic NEAREST-RANK per `case × horse ×
//!   session` from exactly 30 measured observations, sorted ascending,
//!   1-indexed, no interpolation: p50 rank = ceil(0.50 × 30) = 15,
//!   p95 rank = ceil(0.95 × 30) = 29. With n = 30, p95 is a descriptive
//!   tail indicator, not a high-confidence tail model.
//! - **No pooling**: the 90 observations from three sessions are never
//!   pooled and presented as 90 independent replications (task §29).
//! - **Case point estimate** = median of the 3 session p50 values (and
//!   of the 3 session p95 values); all three constituents are retained.
//! - **H0-relative speedup** preserves pairing FIRST:
//!   `speedup(C,H,S) = p50_T_total(C,H0,S) / p50_T_total(C,H,S)` per
//!   session, then the geometric mean of the 3 session ratios. Never
//!   `mean(H0) / mean(Hx)` after aggregation (task §31).
//! - **Warmup rows** are excluded from summaries; **no outlier
//!   deletion** of any kind (all successful measured rows remain)
//!   (task §17, §38).
//! - **Absolute latency** distributions are retained per
//!   case/file/project; geometric means of nanoseconds are never
//!   presented as physical throughput (task §34).
//! - **Experimental hierarchy** (task §32): iteration → session →
//!   case/trace → file → project. PROJECT_MACRO aggregates ratios by
//!   geometric mean per level with equal weights at the top (task §33);
//!   FAMILY_MACRO is secondary, equal family weight.

use std::collections::BTreeMap;

/// Frozen nearest-rank p50 with n = 30 (1-indexed rank 15).
pub const P50_RANK: usize = 15;
/// Frozen nearest-rank p95 with n = 30 (1-indexed rank 29).
pub const P95_RANK: usize = 29;

/// Diagnostic-only session-instability flag threshold (task §52): flag
/// when the max/min ratio of the three session p50s exceeds this. A
/// flag NEVER deletes or downweights data; all three sessions stay
/// visible.
pub const SESSION_UNSTABLE_RATIO: f64 = 1.5;

/// Nearest-rank quantile index for n samples and probability p:
/// `ceil(p * n)`, 1-indexed.
pub fn nearest_rank(n: usize, p: f64) -> usize {
    debug_assert!(p > 0.0 && p < 1.0);
    let rank = (p * n as f64).ceil() as usize;
    rank.clamp(1, n)
}

/// Quantile of ascending-sorted values by 1-indexed rank (no
/// interpolation).
pub fn quantile_by_rank(sorted: &[u64], rank: usize) -> Option<u64> {
    if rank == 0 || rank > sorted.len() {
        return None;
    }
    sorted.get(rank - 1).copied()
}

/// Session cell summary: nearest-rank p50/p95 from exactly 30 measured
/// observations. Fails closed on the wrong sample count.
pub fn session_cell_summary(values: &[u64]) -> Result<(u64, u64), String> {
    if values.len() != crate::manifest::QUANTILE_SAMPLE_SIZE {
        return Err(format!(
            "session cell has {} measured samples != 30",
            values.len()
        ));
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let p50 =
        quantile_by_rank(&sorted, P50_RANK).ok_or_else(|| "p50 rank out of bounds".to_string())?;
    let p95 =
        quantile_by_rank(&sorted, P95_RANK).ok_or_else(|| "p95 rank out of bounds".to_string())?;
    Ok((p50, p95))
}

/// Median of exactly three values (the case-level point estimate of the
/// three session summaries; task §30).
pub fn median_of_three(a: u64, b: u64, c: u64) -> u64 {
    let mut values = [a, b, c];
    values.sort_unstable();
    values[1]
}

/// H0-relative speedup for one case/horse/session: pairing first — the
/// ratio of the H0 session p50 to this horse's session p50 (task §31).
/// `> 1.0` = H faster than H0; `= 1.0` = parity; `< 1.0` = slower.
pub fn session_speedup(h0_session_p50: u64, horse_session_p50: u64) -> Result<f64, String> {
    if horse_session_p50 == 0 {
        return Err("speedup denominator (horse session p50) is zero".to_string());
    }
    Ok(h0_session_p50 as f64 / horse_session_p50 as f64)
}

/// Geometric mean of strictly positive ratios (task §31: the per-case
/// speedup across sessions is the geometric mean of the 3 positive
/// session ratios).
pub fn geometric_mean(values: &[f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err("geometric mean of an empty set".to_string());
    }
    let mut log_sum = 0.0f64;
    for value in values {
        if *value <= 0.0 || !value.is_finite() {
            return Err(format!("non-positive or non-finite ratio {value}"));
        }
        log_sum += value.ln();
    }
    Ok((log_sum / values.len() as f64).exp())
}

/// Session-instability diagnostic (task §52): flag — never discard.
pub fn session_unstable(session_p50s: &[u64; 3]) -> Option<f64> {
    let min = session_p50s.iter().min().copied().unwrap_or(0);
    let max = session_p50s.iter().max().copied().unwrap_or(0);
    if min == 0 {
        return None;
    }
    let ratio = max as f64 / min as f64;
    (ratio > SESSION_UNSTABLE_RATIO).then_some(ratio)
}

// ---------------------------------------------------------------------------
// Observation-frame analysis over the minimal latency fields
// ---------------------------------------------------------------------------

/// The minimal latency facts the statistical contract consumes (the
/// future analysis layer maps campaign envelopes onto this; synthetic
/// tests construct it directly — no real timing input exists here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencySample {
    pub case: &'static str,
    pub horse: &'static str,
    pub session: u32,
    pub measured: bool,
    pub prepare_ns: u64,
    pub native_ns: u64,
    pub total_ns: u64,
    pub execution_pass: bool,
    pub correctness_pass: bool,
}

/// Outcome of summarizing one full set of samples for one surface.
#[derive(Debug, Clone, PartialEq)]
pub enum SummariesOutcome {
    /// Per `case × horse × session` p50/p95 for T_prepare / T_native /
    /// T_total (task §30), plus per-case point estimates (medians of
    /// the three session values).
    Complete {
        cells: BTreeMap<(&'static str, &'static str, u32), CellSummary>,
        case_estimates: BTreeMap<(&'static str, &'static str), CaseEstimate>,
        unstable_sessions: Vec<(&'static str, &'static str)>,
    },
    /// At least one measured sample failed verification: the campaign
    /// is PRIMARY_CAMPAIGN_INVALID for headline comparison; no summary
    /// is produced (task §36 — the failure stays in raw evidence).
    Invalid { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellSummary {
    pub prepare_p50: u64,
    pub prepare_p95: u64,
    pub native_p50: u64,
    pub native_p95: u64,
    pub total_p50: u64,
    pub total_p95: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaseEstimate {
    pub prepare_p50: u64,
    pub prepare_p95: u64,
    pub native_p50: u64,
    pub native_p95: u64,
    pub total_p50: u64,
    pub total_p95: u64,
    /// Diagnostic-only flags (task §52): retained, never acted on.
    pub unstable: bool,
}

/// Build per-session cell summaries and case-level point estimates.
/// Warmups are excluded (task §17); every measured sample must be a
/// pass; exactly 3 sessions × 30 measured samples per `case × horse`
/// are required, and no measured sample may be substituted (task §9,
/// §36).
pub fn summarize(samples: &[LatencySample]) -> SummariesOutcome {
    // Warmup failures are campaign failures too (task §17/§37): a
    // wrong warmup invalidates the campaign, so check ALL samples before
    // excluding warmups from the summaries.
    if let Some(failed) = samples
        .iter()
        .find(|sample| !sample.execution_pass || !sample.correctness_pass)
    {
        return SummariesOutcome::Invalid {
            reason: format!(
                "{} sample case {} horse {} session {} did not pass (execution {} correctness {})",
                if failed.measured {
                    "measured"
                } else {
                    "warmup"
                },
                failed.case,
                failed.horse,
                failed.session,
                failed.execution_pass,
                failed.correctness_pass
            ),
        };
    }
    // Warmup exclusion (task §17).
    let measured: Vec<&LatencySample> = samples.iter().filter(|sample| sample.measured).collect();
    // Invalid-row propagation (task §36): one failed qualified sample
    // invalidates the headline campaign — summaries are not produced.
    if let Some(failed) = measured
        .iter()
        .find(|sample| !sample.execution_pass || !sample.correctness_pass)
    {
        return SummariesOutcome::Invalid {
            reason: format!(
                "sample case {} horse {} session {} did not pass (execution {} correctness {})",
                failed.case,
                failed.horse,
                failed.session,
                failed.execution_pass,
                failed.correctness_pass
            ),
        };
    }
    let mut grouped: BTreeMap<(&'static str, &'static str, u32), Vec<&LatencySample>> =
        BTreeMap::new();
    for sample in &measured {
        grouped
            .entry((sample.case, sample.horse, sample.session))
            .or_default()
            .push(sample);
    }
    let mut cells: BTreeMap<(&'static str, &'static str, u32), CellSummary> = BTreeMap::new();
    let mut by_case_horse: BTreeMap<(&'static str, &'static str), BTreeMap<u32, CellSummary>> =
        BTreeMap::new();
    for ((case, horse, session), group) in &grouped {
        if group.len() != crate::manifest::QUANTILE_SAMPLE_SIZE {
            return SummariesOutcome::Invalid {
                reason: format!(
                    "case {case} horse {horse} session {session}: {} measured samples != 30",
                    group.len()
                ),
            };
        }
        let prepares: Vec<u64> = group.iter().map(|s| s.prepare_ns).collect();
        let natives: Vec<u64> = group.iter().map(|s| s.native_ns).collect();
        let totals: Vec<u64> = group.iter().map(|s| s.total_ns).collect();
        let summary = match (
            session_cell_summary(&prepares),
            session_cell_summary(&natives),
            session_cell_summary(&totals),
        ) {
            (Ok((a, b)), Ok((c, d)), Ok((e, f))) => CellSummary {
                prepare_p50: a,
                prepare_p95: b,
                native_p50: c,
                native_p95: d,
                total_p50: e,
                total_p95: f,
            },
            _ => unreachable!("cell length was checked above"),
        };
        by_case_horse
            .entry((case, horse))
            .or_default()
            .insert(*session, summary);
        cells.insert((*case, *horse, *session), summary);
    }
    let mut case_estimates: BTreeMap<(&'static str, &'static str), CaseEstimate> = BTreeMap::new();
    let mut unstable_sessions = Vec::new();
    for ((case, horse), session_map) in &by_case_horse {
        if session_map.len() != 3 {
            return SummariesOutcome::Invalid {
                reason: format!(
                    "case {case} horse {horse}: {} sessions with summaries != 3",
                    session_map.len()
                ),
            };
        }
        let sessions: Vec<&CellSummary> = session_map.values().collect();
        let total_p50s = [
            sessions[0].total_p50,
            sessions[1].total_p50,
            sessions[2].total_p50,
        ];
        let unstable = session_unstable(&total_p50s).is_some();
        if unstable {
            unstable_sessions.push((*case, *horse));
        }
        case_estimates.insert(
            (*case, *horse),
            CaseEstimate {
                prepare_p50: median_of_three(
                    sessions[0].prepare_p50,
                    sessions[1].prepare_p50,
                    sessions[2].prepare_p50,
                ),
                prepare_p95: median_of_three(
                    sessions[0].prepare_p95,
                    sessions[1].prepare_p95,
                    sessions[2].prepare_p95,
                ),
                native_p50: median_of_three(
                    sessions[0].native_p50,
                    sessions[1].native_p50,
                    sessions[2].native_p50,
                ),
                native_p95: median_of_three(
                    sessions[0].native_p95,
                    sessions[1].native_p95,
                    sessions[2].native_p95,
                ),
                total_p50: median_of_three(total_p50s[0], total_p50s[1], total_p50s[2]),
                total_p95: median_of_three(
                    sessions[0].total_p95,
                    sessions[1].total_p95,
                    sessions[2].total_p95,
                ),
                unstable,
            },
        );
    }
    SummariesOutcome::Complete {
        cells,
        case_estimates,
        unstable_sessions,
    }
}

// ---------------------------------------------------------------------------
// Hierarchical PROJECT_MACRO aggregation (task §33)
// ---------------------------------------------------------------------------

/// Grouping index for one EDIT_WRITE case (the experimental hierarchy:
/// case/trace → file → project; task §32).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchyRef {
    pub case: String,
    pub trace: String,
    pub file: String,
    pub project: String,
}

/// PROJECT_MACRO equal-weight hierarchical geometric-mean aggregation of
/// H0-relative speedups:
///
/// ```text
/// session ratios -> geometric mean per CaseId
/// BREAK/RESTORE steps of one trace -> geometric mean per trace
/// trace groups within file -> geometric mean per file
/// files within project -> geometric mean per project
/// projects -> equal-weight geometric mean
/// ```
pub fn project_macro(
    per_case: &BTreeMap<String, f64>,
    hierarchy: &[HierarchyRef],
) -> Result<f64, String> {
    let case_to_project: BTreeMap<&str, &HierarchyRef> = hierarchy
        .iter()
        .map(|entry| (entry.case.as_str(), entry))
        .collect();
    let mut case_ratios: BTreeMap<&str, f64> = BTreeMap::new();
    for (case, ratio) in per_case {
        if !case_to_project.contains_key(case.as_str()) {
            return Err(format!("case {case} missing from the hierarchy index"));
        }
        case_ratios.insert(case.as_str(), *ratio);
    }
    // trace level
    let mut trace_ratios: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for (case, ratio) in &case_ratios {
        let entry = case_to_project[*case];
        trace_ratios
            .entry(entry.trace.as_str())
            .or_default()
            .push(*ratio);
    }
    let mut trace_value: BTreeMap<&str, f64> = BTreeMap::new();
    let mut trace_file: BTreeMap<&str, &str> = BTreeMap::new();
    for case in case_ratios.keys() {
        let entry = case_to_project[*case];
        trace_file.insert(entry.trace.as_str(), entry.file.as_str());
    }
    for (trace, ratios) in trace_ratios {
        trace_value.insert(trace, geometric_mean(&ratios)?);
    }
    // file level
    let mut file_ratios: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for (trace, value) in &trace_value {
        file_ratios
            .entry(trace_file[*trace])
            .or_default()
            .push(*value);
    }
    let mut file_value: BTreeMap<&str, f64> = BTreeMap::new();
    let mut file_project: BTreeMap<&str, &str> = BTreeMap::new();
    for case in case_ratios.keys() {
        let entry = case_to_project[*case];
        file_project.insert(entry.file.as_str(), entry.project.as_str());
    }
    for (file, ratios) in file_ratios {
        file_value.insert(file, geometric_mean(&ratios)?);
    }
    // project level
    let mut project_ratios: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for (file, value) in &file_value {
        project_ratios
            .entry(file_project[*file])
            .or_default()
            .push(*value);
    }
    let mut project_values = Vec::new();
    for (_project, ratios) in project_ratios {
        project_values.push(geometric_mean(&ratios)?);
    }
    // equal-weight projects
    geometric_mean(&project_values)
}

/// FAMILY_MACRO (secondary; task §33 C): equal weight for the E1-E6
/// families after their project-macro results.
pub fn family_macro(family_project_macro: &BTreeMap<String, f64>) -> Result<f64, String> {
    let values: Vec<f64> = family_project_macro.values().copied().collect();
    geometric_mean(&values)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic synthetic latency stream (multiplier-based, no
    /// wall-clock input of any kind).
    fn synthetic_samples() -> Vec<LatencySample> {
        let mut samples = Vec::new();
        // Two cases, two horses, three sessions, 10 warmup + 30 measured.
        for case in ["caseA", "caseB"] {
            for horse in ["H0", "H1"] {
                for session in 0u32..3 {
                    for iteration in 0..40 {
                        let measured = iteration >= 10;
                        let base = match (case, horse) {
                            ("caseA", "H0") => 1000u64,
                            ("caseA", "H1") => 500u64,
                            ("caseB", "H0") => 2000u64,
                            _ => 2000u64,
                        };
                        // Session drift + iteration noise, deterministic.
                        let noise = ((iteration as u64 * 7 + session as u64 * 13) % 11) * 3;
                        let total = base + noise;
                        samples.push(LatencySample {
                            case,
                            horse,
                            session,
                            measured,
                            prepare_ns: total / 4,
                            native_ns: total - total / 4,
                            total_ns: total,
                            execution_pass: true,
                            correctness_pass: true,
                        });
                    }
                }
            }
        }
        samples
    }

    #[test]
    fn nearest_rank_definitions_are_exact() {
        assert_eq!(nearest_rank(30, 0.50), 15);
        assert_eq!(nearest_rank(30, 0.95), 29);
        // No interpolation: values are taken as-is.
        let sorted = (1u64..=30).collect::<Vec<_>>();
        assert_eq!(quantile_by_rank(&sorted, 15), Some(15));
        assert_eq!(quantile_by_rank(&sorted, 29), Some(29));
        assert_eq!(quantile_by_rank(&sorted, 30), Some(30));
        assert_eq!(quantile_by_rank(&sorted, 31), None);
    }

    #[test]
    fn p50_p95_of_30_samples() {
        let values: Vec<u64> = (1u64..=30).collect();
        let (p50, p95) = session_cell_summary(&values).unwrap();
        assert_eq!(p50, 15);
        assert_eq!(p95, 29);
        // Wrong sample count fails closed.
        assert!(session_cell_summary(&values[..29]).is_err());
    }

    #[test]
    fn median_of_three_is_the_middle_value() {
        assert_eq!(median_of_three(3, 1, 2), 2);
        assert_eq!(median_of_three(10, 10, 20), 10);
    }

    #[test]
    fn speedup_formula_preserves_parity_semantics() {
        // > 1.0 = faster than H0; = 1.0 parity; < 1.0 slower.
        assert!(session_speedup(1000, 500).unwrap() > 1.0);
        assert_eq!(session_speedup(500, 500).unwrap(), 1.0);
        assert!(session_speedup(500, 1000).unwrap() < 1.0);
        assert!(session_speedup(1000, 0).is_err());
    }

    #[test]
    fn geometric_mean_basics() {
        let value = geometric_mean(&[2.0, 8.0]).unwrap();
        assert!((value - 4.0).abs() < 1e-12);
        assert!((geometric_mean(&[1.0, 1.0, 1.0]).unwrap() - 1.0).abs() < 1e-12);
        assert!(geometric_mean(&[0.0]).is_err());
        assert!(geometric_mean(&[-1.0]).is_err());
        assert!(geometric_mean(&[]).is_err());
    }

    #[test]
    fn aggregation_pairs_before_averaging() {
        // Task §31: mean(H0)/mean(Hx) after aggregation is WRONG.
        // H0 = [100, 1000], H1 = [50, 500] in two sessions: every
        // session ratio is 2.0, so the paired answer is exactly 2.0.
        let r1 = session_speedup(100, 50).unwrap();
        let r2 = session_speedup(1000, 500).unwrap();
        let paired = geometric_mean(&[r1, r2]).unwrap();
        assert!((paired - 2.0).abs() < 1e-12);
        // In the symmetric case above the two coincide; this distorted
        // case shows the difference (the reason pairing is frozen).
        let r1 = session_speedup(100, 50).unwrap();
        let r2 = session_speedup(1000, 100).unwrap();
        let paired = geometric_mean(&[r1, r2]).unwrap();
        let wrong = ((100.0 + 1000.0) / 2.0) / ((50.0 + 100.0) / 2.0);
        assert!(
            (paired - wrong).abs() > 1e-6,
            "paired {paired} vs unpaired {wrong}"
        );
    }

    #[test]
    fn project_macro_prevents_high_case_count_domination() {
        let mut per_case = BTreeMap::new();
        let mut hierarchy = Vec::new();
        // Project A: 10 cases, ratio 1.0. Project B: 2 cases, ratio 4.0.
        for i in 0..10 {
            let case = format!("a{i}");
            per_case.insert(case.clone(), 1.0);
            hierarchy.push(HierarchyRef {
                case,
                trace: format!("a-trace-{i}"),
                file: format!("a-file-{i}"),
                project: "projA".to_string(),
            });
        }
        for i in 0..2 {
            let case = format!("b{i}");
            per_case.insert(case.clone(), 4.0);
            hierarchy.push(HierarchyRef {
                case,
                trace: format!("b-trace-{i}"),
                file: format!("b-file-{i}"),
                project: "projB".to_string(),
            });
        }
        let value = project_macro(&per_case, &hierarchy).unwrap();
        // Equal-weight projects: geometric mean of (1.0, 4.0) = 2.0.
        assert!((value - 2.0).abs() < 1e-12);
        // The case-count-weighted answer would be (10*1 + 2*4)/12 ~= 1.5.
    }

    #[test]
    fn family_macro_equal_weights_families() {
        let mut families = BTreeMap::new();
        families.insert("E1_LOCAL_TEXT".to_string(), 2.0);
        families.insert("E6_REFERENCE_DEFINITION".to_string(), 8.0);
        let value = family_macro(&families).unwrap();
        assert!((value - 4.0).abs() < 1e-12);
    }

    #[test]
    fn warmups_are_excluded_and_failures_invalidate() {
        let samples = synthetic_samples();
        // Case-level summarize: full set must be complete.
        let outcome = summarize(&samples);
        assert!(matches!(outcome, SummariesOutcome::Complete { .. }));

        // One failed measured sample -> Invalid, no summaries (task §36).
        let mut poisoned = samples.clone();
        if let Some(sample) = poisoned.iter_mut().find(|s| s.measured) {
            sample.correctness_pass = false;
        }
        assert!(matches!(
            summarize(&poisoned),
            SummariesOutcome::Invalid { .. }
        ));

        // Missing one measured sample (e.g. a substitution attempt) ->
        // Invalid: no measured sample may be substituted (task §9).
        let mut short = samples.clone();
        let index = short.iter().position(|s| s.measured).unwrap();
        short.remove(index);
        assert!(matches!(
            summarize(&short),
            SummariesOutcome::Invalid { .. }
        ));

        // A failed WARMUP also invalidates (task §17/§37): warmups are
        // not disposable correctness.
        let mut warmup_failed = synthetic_samples();
        if let Some(sample) = warmup_failed.iter_mut().find(|s| !s.measured) {
            sample.correctness_pass = false;
        }
        assert!(matches!(
            summarize(&warmup_failed),
            SummariesOutcome::Invalid { .. }
        ));

        // All warmups removed must not change the summaries (exclusion
        // semantics — they were already excluded).
        let measured_only: Vec<LatencySample> =
            samples.iter().filter(|s| s.measured).copied().collect();
        assert_eq!(summarize(&samples), summarize(&measured_only));
    }

    #[test]
    fn case_estimates_are_medians_of_three_sessions() {
        let samples = synthetic_samples();
        let SummariesOutcome::Complete {
            case_estimates,
            unstable_sessions,
            ..
        } = summarize(&samples)
        else {
            panic!("synthetic set must summarize");
        };
        // H1 on caseA halves H0's latency: every session ratio is ~2.0
        // (deterministic noise may shift it slightly, never the sign).
        let h0 = case_estimates.get(&("caseA", "H0")).expect("H0 estimate");
        let h1 = case_estimates.get(&("caseA", "H1")).expect("H1 estimate");
        // H1 halves H0's latency modulo the bounded deterministic noise
        // (up to 30ns per sample on a 500ns base).
        assert!(h1.total_p50 * 2 <= h0.total_p50 + 60);
        // caseB is parity: p50s are equal.
        let b0 = case_estimates.get(&("caseB", "H0")).expect("b0");
        let b1 = case_estimates.get(&("caseB", "H1")).expect("b1");
        assert_eq!(b0.total_p50, b1.total_p50);
        assert!(unstable_sessions.is_empty(), "stable synthetic set");
    }

    #[test]
    fn session_instability_is_flag_only() {
        assert!(session_unstable(&[100, 100, 100]).is_none());
        assert!(session_unstable(&[100, 100, 200]).is_some());
        // The flag never removes data: the function returns a ratio,
        // callers retain all three sessions.
    }
}
