//! Deterministic distribution statistics (CORRECTIVE-B §18, §23).

use serde::Serialize;

/// Sorted-copy quantile by nearest-rank interpolation on the given sorted
/// values. `q` is a fraction in `[0, 1]`. Deterministic: no sampling, no
/// jitter.
pub fn quantile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let position = q * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = position - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

/// Nearest-rank quantile on integer values (exact, no interpolation).
pub fn quantile_rank(sorted_u64: &[u64], q: f64) -> u64 {
    if sorted_u64.is_empty() {
        return 0;
    }
    let position = (q * sorted_u64.len() as f64).ceil() as usize;
    let index = position.clamp(1, sorted_u64.len()) - 1;
    sorted_u64[index]
}

/// The frozen summary statistics block for one numeric feature.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureStats {
    pub count: u64,
    pub zero_count: u64,
    pub min: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

/// Compute [`FeatureStats`] over raw values.
pub fn feature_stats(values: &[f64]) -> FeatureStats {
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("values must not be NaN"));
    FeatureStats {
        count: sorted.len() as u64,
        zero_count: sorted.iter().filter(|value| **value == 0.0).count() as u64,
        min: sorted.first().copied().unwrap_or(0.0),
        p25: quantile_sorted(&sorted, 0.25),
        p50: quantile_sorted(&sorted, 0.50),
        p75: quantile_sorted(&sorted, 0.75),
        p95: quantile_sorted(&sorted, 0.95),
        p99: quantile_sorted(&sorted, 0.99),
        max: sorted.last().copied().unwrap_or(0.0),
    }
}

/// The frozen binning rule result for one feature (§23).
///
/// Rule: `ZERO` is the exact-0 bin when zero is semantically meaningful;
/// positive values are split at the positive-population tertiles
/// Q33.333/Q66.667 into LOW/MEDIUM/HIGH. Bins whose boundaries collapse
/// (because too many values are equal) are collapsed to the actually
/// attainable set; values are never jittered.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeatureBins {
    pub feature: String,
    pub zero_meaningful: bool,
    /// Tertile thresholds over positive values (present when there are
    /// at least two distinct positive values).
    pub low_max: Option<f64>,
    pub medium_max: Option<f64>,
    /// The attainable ordered bin labels.
    pub bins: Vec<String>,
}

/// Derive bins for one feature per the frozen rule.
pub fn derive_bins(feature: &str, zero_meaningful: bool, values: &[f64]) -> FeatureBins {
    let mut labels = Vec::new();
    let mut low_max: Option<f64> = None;
    let mut medium_max: Option<f64> = None;

    let has_zero = values.iter().any(|value| *value == 0.0);
    if zero_meaningful && has_zero {
        labels.push("ZERO".to_string());
    }

    let mut positive: Vec<f64> = values
        .iter()
        .copied()
        .filter(|value| *value > 0.0)
        .collect();
    positive.sort_by(|a, b| a.partial_cmp(b).expect("values must not be NaN"));
    if positive.is_empty() {
        return FeatureBins {
            feature: feature.to_string(),
            zero_meaningful,
            low_max,
            medium_max,
            bins: labels,
        };
    }

    let t1 = quantile_sorted(&positive, 1.0 / 3.0);
    let t2 = quantile_sorted(&positive, 2.0 / 3.0);

    // Collapse rule: a boundary only yields a separate bin when values
    // actually exist strictly above the previous boundary. LOW always
    // exists if any positive value exists.
    let above_t1 = positive.iter().any(|value| *value > t1);
    let above_t2 = positive.iter().any(|value| *value > t2);

    if above_t1 {
        low_max = Some(t1);
        labels.push("LOW".to_string());
        if above_t2 {
            medium_max = Some(t2);
            labels.push("MEDIUM".to_string());
            labels.push("HIGH".to_string());
        } else {
            // No value above the upper tertile: MEDIUM and HIGH collapse.
            labels.push("MEDIUM".to_string());
        }
    } else {
        // All positive values are equal (or all <= t1 with t1 == max):
        // a single collapsed positive bin.
        labels.push("LOW".to_string());
    }

    FeatureBins {
        feature: feature.to_string(),
        zero_meaningful,
        low_max,
        medium_max,
        bins: labels,
    }
}

/// Assign a value to its bin label under derived bins.
pub fn bin_of(bins: &FeatureBins, value: f64) -> String {
    if bins.zero_meaningful && value == 0.0 && bins.bins.iter().any(|b| b == "ZERO") {
        return "ZERO".to_string();
    }
    if !bins
        .bins
        .iter()
        .any(|b| b == "LOW" || b == "MEDIUM" || b == "HIGH")
    {
        // No positive bin exists; the only attainable label is ZERO.
        return "ZERO".to_string();
    }
    match (bins.low_max, bins.medium_max) {
        (Some(low), Some(medium)) => {
            if value <= low {
                "LOW".to_string()
            } else if value <= medium {
                "MEDIUM".to_string()
            } else {
                "HIGH".to_string()
            }
        }
        (Some(low), None) => {
            if value <= low {
                "LOW".to_string()
            } else {
                "MEDIUM".to_string()
            }
        }
        _ => "LOW".to_string(),
    }
}

/// Percentile rank of `value` within `sorted_values` (fraction of values
/// strictly less than `value`, plus half of equals; deterministic and
/// tie-stable). Used by the frozen full-document rank formulas (§34).
pub fn percentile_rank(sorted_values: &[f64], value: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let less = sorted_values.iter().filter(|other| **other < value).count() as f64;
    let equal = sorted_values
        .iter()
        .filter(|other| **other == value)
        .count() as f64;
    (less + 0.5 * equal) / sorted_values.len() as f64
}
