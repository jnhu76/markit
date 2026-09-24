//! Frozen balanced-randomized schedules (Issue #50 §3, §9).
//!
//! Every measured round contains every included label exactly once, so the
//! schedule is balanced by construction; the order inside each round is a
//! Fisher–Yates shuffle from a deterministic SplitMix64 stream seeded with
//! a frozen campaign seed, the lane tag, and the round index. The actual
//! schedule is written out, never reconstructed from memory.
//!
//! Nothing here is allowed to be "all A0 first, then all variants"
//! (Issue #50 §10): the ablation lane interleaves `A0`, the identical
//! `A0-duplicate` control, and every ablation inside each single round.

/// Deterministic SplitMix64.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
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

    /// Uniform in `[0, bound)` via Lemire's multiply-shift (rejection-free
    /// approximation is fine here: the bound is tiny and the seed is
    /// frozen, so the schedule is reproducible either way).
    fn below(&mut self, bound: usize) -> usize {
        assert!(bound > 0, "empty bound");
        ((self.next_u64() as u128 * bound as u128) >> 64) as usize
    }
}

/// The frozen campaign seed for #50.
pub const CAMPAIGN_SEED: u64 = 0x4834_4C41_5247_454E; // "H4LARGEN"

/// Seed for one lane/session.
pub fn lane_seed(lane: &str, session: u32) -> u64 {
    let mut hash = CAMPAIGN_SEED;
    for b in lane.as_bytes() {
        hash = (hash ^ *b as u64).wrapping_mul(0x100_0000_01B3);
    }
    hash ^ (session as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// One scheduled slot: a label (the *variant/control* identity) and the
/// cell it runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub round: u32,
    pub ordinal: u32,
    pub label: String,
    pub cell: String,
}

/// Build a balanced randomized schedule: `rounds` rounds, each a random
/// permutation of the full label × cell grid.
pub fn build(cells: &[String], labels: &[String], rounds: u32, seed: u64) -> Vec<Slot> {
    let mut items: Vec<(String, String)> = Vec::new();
    for cell in cells {
        for label in labels {
            items.push((label.clone(), cell.clone()));
        }
    }
    let n = items.len();
    let mut rng = SplitMix64::new(seed);
    let mut out = Vec::with_capacity(rounds as usize * n);
    for round in 0..rounds {
        let mut order = items.clone();
        // Fisher–Yates.
        for i in (1..n).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        for (ordinal, (label, cell)) in order.into_iter().enumerate() {
            out.push(Slot {
                round,
                ordinal: ordinal as u32,
                label,
                cell,
            });
        }
    }
    out
}

/// Render a schedule as JSONL (one object per slot).
pub fn to_jsonl(slots: &[Slot]) -> String {
    let mut out = String::new();
    for s in slots {
        out.push_str(
            &serde_json::json!({
                "round": s.round,
                "ordinal": s.ordinal,
                "label": s.label,
                "cell": s.cell,
            })
            .to_string(),
        );
        out.push('\n');
    }
    out
}
