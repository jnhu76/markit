//! Deterministic case ordering (R1 task §11).
//!
//! ```text
//! same manifest + same seed -> same case set -> same CaseIds -> same order
//! ```
//!
//! Pipeline: sort by CaseId bytes (total order, input-order independent)
//! then a Fisher-Yates shuffle driven by SplitMix64-v1 with the recorded
//! seed. No `HashMap` iteration ever determines order.

use markit_mdbench_common::CaseId;
use markit_mdbench_common::Seed;

/// Identifier of the recorded ordering algorithm (PRNG + shuffle).
///
/// History (identifiers are never reused or silently changed):
///
/// - `splitmix64-v1+fisher-yates-mulshift-v1`: single widening
///   multiply-high index draws — documented as "unbiased", which was an
///   overclaim (without rejection the mapping is not strictly uniform for
///   arbitrary bounds). Superseded by v2 in R1-CORRECTIVE-1.
/// - `splitmix64-v1+fisher-yates-lemire-rejection-v2` (current): same
///   SplitMix64-v1 stream and Fisher–Yates frame, but bounded draws use
///   Lemire multiply-shift WITH rejection of the biased zone, which is
///   exactly uniform over `0..n`.
pub const SHUFFLE_ALGORITHM_ID: &str = "splitmix64-v1+fisher-yates-lemire-rejection-v2";

/// Explicitly versioned SplitMix64 (Steele et al. variant used by
/// splitmix64.c), fixed constants; stream of u64 words.
pub struct SplitMix64V1 {
    state: u64,
}

impl SplitMix64V1 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform draw in `0..n` via 64-bit widening multiplication with
    /// REJECTION of the biased zone (Lemire's bounded sampling). The
    /// rejection loop re-draws whenever the raw draw falls inside the
    /// final partial interval, so every index has probability exactly
    /// `1/n`; at most one rejection is expected on average
    /// (`2^64 mod n < n` bounds the biased zone).
    ///
    /// Deterministic for a fixed PRNG stream; changing this method
    /// changes [`SHUFFLE_ALGORITHM_ID`] — golden vectors pin its behavior.
    fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        let n64 = n as u64;
        // The biased zone is the final `2^64 mod n` values of the u64
        // range; `(0 - n) % n` computes `2^64 mod n` without 128-bit math.
        let threshold = 0u64.wrapping_sub(n64) % n64;
        loop {
            let wide = (self.next_u64() as u128) * (n as u128);
            let low = wide as u64;
            if low >= threshold {
                return (wide >> 64) as usize;
            }
        }
    }
}

/// Sort `items` by CaseId, then apply the seeded deterministic
/// Fisher-Yates shuffle. Deterministic in both the seed and the input
/// enumeration order.
pub fn order_cases<T, F>(items: &mut [T], seed: Seed, case_id_of: F)
where
    F: Fn(&T) -> CaseId,
{
    items.sort_by(|a, b| case_id_of(a).as_bytes().cmp(case_id_of(b).as_bytes()));
    let mut rng = SplitMix64V1::new(seed.0);
    for i in (1..items.len()).rev() {
        let j = rng.below(i + 1);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markit_mdbench_common::CaseKeyV1;
    use markit_mdbench_common::OperationKind;
    use markit_mdbench_common::PayloadShape;

    fn synthetic_id(i: u64) -> CaseId {
        let key = CaseKeyV1 {
            payload_id: format!("case-{i:04}"),
            payload_shape: PayloadShape::Plain,
            payload_size_bytes: 64 * 1024,
            old_source_sha256: [i as u8; 32],
            operation: OperationKind::FullParse,
            edit_start_byte: None,
            edit_end_byte: None,
            inserted_text_sha256: None,
            generator_id: Some("r1-ordering-test".to_string()),
            generator_seed: Some(i),
        }
        .validated()
        .expect("valid key");
        CaseId::from_key(&key)
    }

    #[derive(Clone)]
    struct Entry {
        tag: u64,
        id: CaseId,
    }

    fn build_set(permutation: &[usize]) -> Vec<Entry> {
        permutation
            .iter()
            .map(|&i| Entry {
                tag: i as u64,
                id: synthetic_id(i as u64),
            })
            .collect()
    }

    #[test]
    fn same_seed_same_order() {
        let identity: Vec<usize> = (0..24).collect();
        let scrambled: Vec<usize> = vec![
            23, 7, 11, 0, 19, 4, 15, 8, 3, 20, 12, 6, 17, 1, 9, 22, 5, 14, 10, 2, 18, 13, 21, 16,
        ];
        let mut a = build_set(&identity);
        let mut b = build_set(&scrambled);
        order_cases(&mut a, Seed(42), |e| e.id);
        order_cases(&mut b, Seed(42), |e| e.id);
        assert_eq!(a.len(), b.len());
        for (ea, eb) in a.iter().zip(b.iter()) {
            assert_eq!(ea.tag, eb.tag);
            assert_eq!(ea.id, eb.id);
        }
    }

    #[test]
    fn different_seed_normally_different_order() {
        let identity: Vec<usize> = (0..24).collect();
        let mut a = build_set(&identity);
        let mut b = build_set(&identity);
        order_cases(&mut a, Seed(1), |e| e.id);
        order_cases(&mut b, Seed(2), |e| e.id);
        let same_positions = a
            .iter()
            .zip(b.iter())
            .filter(|(x, y)| x.tag == y.tag)
            .count();
        assert!(
            same_positions < 24,
            "different seeds produced identical permutations"
        );
    }

    #[test]
    fn unordered_enumeration_yields_same_final_order() {
        let identity: Vec<usize> = (0..24).collect();
        let mut forward = build_set(&identity);
        let mut reverse = build_set(&identity);
        reverse.reverse();
        order_cases(&mut forward, Seed(7), |e| e.id);
        order_cases(&mut reverse, Seed(7), |e| e.id);
        for (a, b) in forward.iter().zip(reverse.iter()) {
            assert_eq!(a.tag, b.tag);
        }
    }

    #[test]
    fn empty_and_singletons_are_safe() {
        let mut none: Vec<Entry> = vec![];
        order_cases(&mut none, Seed(3), |e| e.id);
        assert!(none.is_empty());
        let mut one = build_set(&[0]);
        order_cases(&mut one, Seed(3), |e| e.id);
        assert_eq!(one.len(), 1);
    }

    // -- golden reproducibility vectors (R1-CORRECTIVE-1, IMPORTANT-2) --
    //
    // These pin the SplitMix64-v1 stream and the v2 (rejection-based)
    // permutation. If any of them change, that is an ordering break:
    // `SHUFFLE_ALGORITHM_ID` must change explicitly in the same commit.
    // Never silently regenerate these values. Computed at
    // R1-CORRECTIVE-1 (rustc 1.97.1, 2026-09-16).

    const GOLDEN_SPLITMIX8_SEED: u64 = 0x5EED_0000_0000_0001;
    const GOLDEN_SPLITMIX8: [u64; 8] = [
        0x988c_ed4d_9e13_3daa,
        0x659e_27c2_a1c5_ffa8,
        0xd0f1_527c_73b2_efbc,
        0x46fc_33ff_15be_f56a,
        0x00dd_4aab_4072_c7e9,
        0xec50_5698_f15d_1984,
        0x3395_96c5_ac4f_1fb0,
        0x3a48_6ceb_cefa_48b7,
    ];

    #[test]
    fn splitmix64_stream_matches_golden_vectors() {
        let mut rng = SplitMix64V1::new(GOLDEN_SPLITMIX8_SEED);
        for expected in GOLDEN_SPLITMIX8 {
            assert_eq!(rng.next_u64(), expected, "SplitMix64-v1 stream drifted");
        }
    }

    fn golden_case_id(i: u64) -> CaseId {
        let key = CaseKeyV1 {
            payload_id: format!("case-{i:04}"),
            payload_shape: PayloadShape::Plain,
            payload_size_bytes: 64 * 1024,
            old_source_sha256: [i as u8; 32],
            operation: OperationKind::FullParse,
            edit_start_byte: None,
            edit_end_byte: None,
            inserted_text_sha256: None,
            generator_id: Some("r1-golden-test".to_string()),
            generator_seed: Some(i),
        }
        .validated()
        .expect("golden key");
        CaseId::from_key(&key)
    }

    /// GOLDEN: the fixed six-case set (tags 0..=5) under seed
    /// `0x5EED_0000_0000_0002` must land in exactly this tag order,
    /// regardless of the input enumeration order.
    const GOLDEN_PERMUTATION_SEED: u64 = 0x5EED_0000_0000_0002;
    const GOLDEN_PERMUTATION: [u64; 6] = [3, 1, 5, 2, 0, 4];

    #[test]
    fn order_cases_matches_golden_permutation() {
        // Scrambled input order: the sort must make this irrelevant.
        let input_order = [5usize, 2, 4, 0, 3, 1];
        let mut entries: Vec<Entry> = input_order
            .iter()
            .map(|&i| Entry {
                tag: i as u64,
                id: golden_case_id(i as u64),
            })
            .collect();
        order_cases(&mut entries, Seed(GOLDEN_PERMUTATION_SEED), |e| e.id);
        let tags: Vec<u64> = entries.iter().map(|e| e.tag).collect();
        assert_eq!(
            tags,
            GOLDEN_PERMUTATION.to_vec(),
            "case-order permutation drifted: SHUFFLE_ALGORITHM_ID must \
             change explicitly with it"
        );
    }

    #[test]
    fn bounded_draws_are_in_range_and_balanced() {
        // Rejection-sampler sanity for v2: every draw inside 0..n, and a
        // rough balance check that would catch a grossly biased mapping.
        for n in 2usize..=97 {
            let mut rng = SplitMix64V1::new(GOLDEN_SPLITMIX8_SEED);
            let draws = n * 256;
            let mut buckets = vec![0u32; n];
            for _ in 0..draws {
                let draw = rng.below(n);
                buckets[draw] += 1;
            }
            let expected = (draws / n) as i64;
            for (index, &count) in buckets.iter().enumerate() {
                assert!(
                    (count as i64 - expected).abs() <= expected / 4 + 16,
                    "bound {n}: bucket {index} count {count} vs expected ~{expected}"
                );
            }
        }
    }
}
