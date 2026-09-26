//! Deterministic mixed operation sequences (I3 task contract §45; §25):
//! split / join / replace_range over varied sizes and certificate
//! patterns with a small deterministic PRNG — no new test dependency.
//! After every operation: the full §36 invariant matrix, the Vec model,
//! and allocation-identity conservation across the whole live pool.

use super::*;
use crate::sequence::{join, split};

fn sorted(mut v: Vec<usize>) -> Vec<usize> {
    v.sort_unstable();
    v
}

/// Deterministic LCG (numerical-recipes constants).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn pool_addresses(pool: &[(OwnerSeq, Model)]) -> Vec<usize> {
    let mut out = Vec::new();
    for (seq, _) in pool {
        out.extend(addresses(seq));
    }
    out
}

#[test]
fn deterministic_mixed_sequences_preserve_every_invariant() {
    for seed in [1u64, 7, 42, 20260926] {
        for n0 in [1usize, 2, 3, 5, 8, 13, 21] {
            let weights: Vec<usize> = (0..n0)
                .map(|i| if i % 5 == 0 { 31 } else { i % 7 + 1 })
                .collect();
            let certs: Vec<bool> = (0..n0).map(|i| i % 3 == 0).collect();
            let model: Model = weights.iter().copied().zip(certs.iter().copied()).collect();
            let mut pool: Vec<(OwnerSeq, Model)> = vec![(make(&weights, &certs), model)];
            // Conservation baseline: every allocation ever created must
            // stay alive in exactly one pool entry (no clone, no drop of
            // retained state — retirement is I5's business, not I3's).
            let mut expected_addrs = pool_addresses(&pool);
            let mut rng = Lcg(seed ^ (n0 as u64).wrapping_mul(0x9E3779B97F4A7C15));

            for _step in 0..64 {
                // With fewer than two live sequences, join is not
                // selectable.
                let op = if pool.len() < 2 {
                    [0usize, 2][rng.below(2)]
                } else {
                    rng.below(3)
                };
                match op {
                    // split a random entry at a random rank
                    0 => {
                        let i = rng.below(pool.len());
                        let records = pool[i].0.records();
                        let k = rng.below(records + 1);
                        let (root, model) = pool.swap_remove(i);
                        let (a, b) = split(root.root, k);
                        pool.push((OwnerSeq { root: a }, model[..k].to_vec()));
                        pool.push((OwnerSeq { root: b }, model[k..].to_vec()));
                    }
                    // join two distinct random entries
                    1 => {
                        let len = pool.len();
                        let i = rng.below(len);
                        let j = (i + 1 + rng.below(len - 1)) % len;
                        let (root_i, model_i) = pool.swap_remove(i);
                        // swap_remove moved the last element into slot i.
                        let j = if j == len - 1 { i } else { j };
                        let (root_j, model_j) = pool.swap_remove(j);
                        let mut model = model_i;
                        model.extend(model_j);
                        pool.push((
                            OwnerSeq {
                                root: join(root_i.root, root_j.root),
                            },
                            model,
                        ));
                    }
                    // replace a random range with a fresh middle
                    _ => {
                        let i = rng.below(pool.len());
                        let records = pool[i].0.records();
                        let lo = rng.below(records + 1);
                        let hi = lo + rng.below(records - lo + 1);
                        let mid_n = rng.below(4);
                        let mid_w: Vec<usize> = (0..mid_n).map(|_| rng.below(50) + 1).collect();
                        let mid_c: Vec<bool> = (0..mid_n).map(|_| rng.below(2) == 0).collect();
                        let middle = make(&mid_w, &mid_c);
                        expected_addrs.extend(addresses(&middle));
                        let mid_model: Model =
                            mid_w.iter().copied().zip(mid_c.iter().copied()).collect();
                        let (mut seq, mut model) = pool.swap_remove(i);
                        let removed = seq.replace_range(lo, hi, middle);
                        let removed_model = model_of(&removed);
                        model.splice(lo..hi, mid_model);
                        pool.push((seq, model));
                        pool.push((removed, removed_model));
                    }
                }

                for (seq, model) in &pool {
                    assert_seq(seq, model);
                }
                assert_eq!(
                    sorted(pool_addresses(&pool)),
                    sorted(expected_addrs.clone()),
                    "allocation conservation (seed {seed}, n0 {n0})"
                );
            }
        }
    }
}

#[test]
fn retained_node_identity_survives_split_join_and_replace() {
    // I3 task contract §25: explicit address-identity check across the
    // three structural operations.
    let weights = [1usize, 2, 3, 7, 31, 2, 5, 8];
    let certs = [true, false, true, false, true, false, false, true];
    let model: Model = weights.iter().copied().zip(certs).collect();

    // split at 3: P/S address sequences are exact slices of the original.
    let seq = make(&weights, &certs);
    let addrs = addresses(&seq);
    let (a, b) = split(seq.root, 3);
    let seq_a = OwnerSeq { root: a };
    let seq_b = OwnerSeq { root: b };
    assert_eq!(addresses(&seq_a), addrs[..3]);
    assert_eq!(addresses(&seq_b), addrs[3..]);

    // join back: identical in-order node sequence.
    let rejoined = OwnerSeq {
        root: join(seq_a.root, seq_b.root),
    };
    assert_eq!(
        addresses(&rejoined),
        addrs,
        "join restores the exact sequence"
    );
    assert_seq(&rejoined, &model);

    // replace [2..5) with a fresh middle: P/S keep their allocations,
    // the removed range keeps its own, in order.
    let mut seq2 = make(&weights, &certs);
    let original = addresses(&seq2);
    let removed = seq2.replace_range(2, 5, make(&[90, 91], &[true, false]));
    let after = addresses(&seq2);
    assert_eq!(&after[..2], &original[..2], "retained prefix identity");
    assert_eq!(&after[4..], &original[5..], "retained suffix identity");
    assert_eq!(
        addresses(&removed),
        original[2..5].to_vec(),
        "removed range identity, in order"
    );
}
