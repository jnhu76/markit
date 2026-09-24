//! Statistics used by the #50 reports (Issue #50 §3, §14).
//!
//! - per session, per point: p50 and p95 over the 30 measured values with
//!   the frozen nearest-rank rule (p50 = 15th smallest, p95 = 29th
//!   smallest);
//! - the case estimate is the **median of the three session p50s**, never
//!   a statistic over 90 pooled samples treated as independent;
//! - the empirical A/A resolution is the p95 of the per-round absolute
//!   relative difference between the two identical A0 control slots, taken
//!   per session, and the most conservative of the three sessions is used;
//! - the practical effect threshold is `max(5%, noise)`.

/// Sorted-copy percentile with the nearest-rank rule:
/// `index = ceil(p * n)` (1-based), clamped into range.
pub fn nearest_rank(values: &[u64], p: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_unstable();
    let n = v.len();
    let rank = (p * n as f64).ceil() as usize;
    let idx = rank.clamp(1, n) - 1;
    Some(v[idx])
}

/// p50 / p95 pair.
pub fn p50_p95(values: &[u64]) -> Option<(u64, u64)> {
    Some((
        nearest_rank(values, 0.50)?,
        nearest_rank(values, 0.95)?,
    ))
}

/// Median of three session p50s (the case estimate).
pub fn median3(mut values: [u64; 3]) -> u64 {
    values.sort_unstable();
    values[1]
}

/// Median of an arbitrary (non-empty) slice.
pub fn median(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_unstable();
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2
    })
}

/// The empirical A/A noise for one session: the p95 of the per-round
/// absolute relative difference between the two identical A0 control
/// slots' measured values.
///
/// `pairs` holds `(value_of_A0, value_of_A0_duplicate)` for one round.
/// Rounds where either value is zero are skipped (no defined relative
/// difference) and the count of skipped rounds is returned alongside.
pub fn aa_noise(pairs: &[(u64, u64)]) -> (Option<f64>, usize) {
    let mut rel: Vec<u64> = Vec::new();
    let mut skipped = 0usize;
    for (a, b) in pairs {
        if *a == 0 || *b == 0 {
            skipped += 1;
            continue;
        }
        let diff = (*a as f64 - *b as f64).abs() / (*a.max(b) as f64);
        // Store as parts-per-billion to reuse the integer percentile.
        rel.push((diff * 1e9) as u64);
    }
    if rel.is_empty() {
        return (None, skipped);
    }
    nearest_rank(&rel, 0.95).map(|v| (Some(v as f64 / 1e9), skipped)).unwrap_or((None, skipped))
}

/// `max(5%, noise)`, the practical effect threshold (Issue #50 §14).
pub fn effect_threshold(noise: Option<f64>) -> f64 {
    0.05f64.max(noise.unwrap_or(0.0))
}
