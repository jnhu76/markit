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
pub const SHUFFLE_ALGORITHM_ID: &str = "splitmix64-v1+fisher-yates-mulshift-v1";

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

    /// Unbiased draw in `0..=n-1` via 64-bit widening multiplication
    /// (Lemire's "fastrange"). Deterministic for a fixed stream.
    fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        (((self.next_u64() as u128) * (n as u128)) >> 64) as usize
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
}
