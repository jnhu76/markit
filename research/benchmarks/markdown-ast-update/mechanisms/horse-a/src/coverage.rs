//! Canonical physical-first-line Owner coverage construction (spec
//! §3.2; data-model §3.1): `c0 = 0`, `c_i = p_i` (the `TopLevelStart`
//! physical line start of the i-th root block, never its semantic
//! span start), `c_k = source_len`; `Owner_i.coverage = [c_i, c_(i+1))`.
//!
//! Consequences encoded here: leading trivia belongs to the first
//! Owner; blank/interstitial bytes belong to the left Owner; trailing
//! trivia belongs to the last Owner; the union of coverage is exactly
//! `[0, source_len)` — no gaps, no overlap, no zero-length Owner.

use crate::full_build::BuildError;

/// The coverage cut sequence `c_0 .. c_k` of one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoveragePlan {
    pub cuts: Vec<usize>,
}

impl CoveragePlan {
    /// Build the frozen cuts from the observed root-level physical
    /// first-line starts. `source_len` closes the last Owner's coverage.
    pub(crate) fn build(starts: &[usize], source_len: usize) -> Result<CoveragePlan, BuildError> {
        Self::build_with_base(0, starts, source_len)
    }

    /// The same frozen cut rule for a REGION whose first Owner's coverage
    /// already begins at `base`: the local replacement path passes the
    /// restart cut as `base` and the convergence cut (or real `L_new`) as
    /// `region_end`. The first fresh block's physical start is still not a
    /// cut — the first fresh Owner swallows the region's leading trivia,
    /// exactly as the document's first Owner swallows the document's.
    /// Callers must pass `starts.len() + 1` Owner slots (the Owners the
    /// region produced plus the retained suffix's first Owner).
    pub(crate) fn build_with_base(
        base: usize,
        starts: &[usize],
        region_end: usize,
    ) -> Result<CoveragePlan, BuildError> {
        let mut prev: Option<usize> = None;
        for &p in starts {
            if p < base || p >= region_end {
                return Err(BuildError::InconsistentObservation {
                    detail: format!(
                        "TopLevelStart physical line {p} is not inside [{base}, {region_end})"
                    ),
                });
            }
            if let Some(prev_p) = prev {
                if p <= prev_p {
                    return Err(BuildError::InconsistentObservation {
                        detail: format!(
                            "TopLevelStart physical line starts are not strictly increasing: {prev_p} then {p}"
                        ),
                    });
                }
            }
            prev = Some(p);
        }
        let mut cuts = Vec::with_capacity(starts.len() + 1);
        cuts.push(base);
        // `c_i = p_i` for 0 < i < k: the FIRST block's physical start is
        // not a cut — its Owner already begins at `base` and swallows the
        // leading trivia (spec §3.2).
        cuts.extend_from_slice(&starts[1.min(starts.len())..]);
        cuts.push(region_end);
        Ok(CoveragePlan { cuts })
    }

    /// The coverage length of the Owner between cuts `i` and `i + 1`.
    pub(crate) fn coverage_len(&self, i: usize) -> usize {
        self.cuts[i + 1] - self.cuts[i]
    }
}
