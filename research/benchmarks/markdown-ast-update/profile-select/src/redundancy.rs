//! Redundancy analysis (CORRECTIVE-B §21): exact byte duplicates and an
//! inspectable near-duplicate estimate.
//!
//! Frozen near-duplicate method (diagnostic only — near-duplicates are
//! never automatically removed):
//!
//! ```text
//! normalization   lowercase; strip Markdown punctuation characters
//!                 (`*_`[]()#>+-|`~<>!.) ; collapse all whitespace runs
//!                 to single spaces
//! shingles        word 5-grams of the normalized token stream
//! minhash         64 fixed permutation functions; each is
//!                 h_i(x) = sha256("CORRECTIVE-B-MINHASH-v1:" + i + ":" + x)
//!                 interpreted as a little-endian u64 over the first
//!                 8 bytes of the digest; signature = element-wise min
//!                 over the file's shingles; no random seed
//! similarity      estimated Jaccard = matching signature components /
//!                 64, reported for pairs >= THRESHOLD
//! threshold       0.75 (frozen)
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{CandidateRow, REDUNDANCY_SCHEMA};

pub const NEARDUP_VERSION: &str = "SHINGLE-W5-MINHASH64-JACCARD-v1";
pub const NEARDUP_THRESHOLD: f64 = 0.75;
pub const SHINGLE_WIDTH: usize = 5;
pub const PERMUTATIONS: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub struct ExactDuplicateGroup {
    pub sha256: String,
    pub members: Vec<String>,
    pub projects: Vec<String>,
    /// Lexically smallest member (frozen canonical choice).
    pub canonical_member: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NearDuplicateGroup {
    pub members: Vec<String>,
    pub projects: Vec<String>,
    pub estimated_jaccard: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RedundancyArtifact {
    pub schema: String,
    pub method: String,
    pub threshold: f64,
    pub shingle_width: usize,
    pub permutations: usize,
    pub exact_groups: Vec<ExactDuplicateGroup>,
    pub near_groups: Vec<NearDuplicateGroup>,
    pub notes: Vec<String>,
}

/// Exact duplicate groups by source SHA-256 (§21.1). The acquisition
/// `exact-duplicates-v1.json` registry is cross-checked, not replaced.
pub fn exact_groups(rows: &[CandidateRow]) -> Vec<ExactDuplicateGroup> {
    let mut by_hash: BTreeMap<&str, Vec<&CandidateRow>> = BTreeMap::new();
    for row in rows {
        by_hash
            .entry(row.source_sha256.as_str())
            .or_default()
            .push(row);
    }
    let mut groups = Vec::new();
    for (hash, members) in by_hash {
        if members.len() < 2 {
            continue;
        }
        let mut keys: Vec<String> = members
            .iter()
            .map(|row| format!("{}/{}", row.identity.source_id, row.identity.snapshot_path))
            .collect();
        keys.sort();
        let mut projects: Vec<String> = members
            .iter()
            .map(|row| row.identity.source_id.clone())
            .collect();
        projects.sort();
        projects.dedup();
        groups.push(ExactDuplicateGroup {
            sha256: hash.to_string(),
            canonical_member: keys[0].clone(),
            members: keys,
            projects,
        });
    }
    groups
}

fn normalize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else if "*_`[]()#>+-|~<!.".contains(lower) {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(lower);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn shingles(tokens: &[String]) -> Vec<String> {
    if tokens.len() < SHINGLE_WIDTH {
        return vec![tokens.join(" ")];
    }
    (0..=tokens.len() - SHINGLE_WIDTH)
        .map(|index| tokens[index..index + SHINGLE_WIDTH].join(" "))
        .collect()
}

fn permute(i: usize, shingle: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(format!("CORRECTIVE-B-MINHASH-v1:{i}:{shingle}"));
    let digest = hasher.finalize();
    u64::from_le_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

/// Deterministic 64-component MinHash signature.
pub fn signature(text: &str) -> [u64; PERMUTATIONS] {
    let tokens = normalize(text);
    let shingles = shingles(&tokens);
    let mut signature = [u64::MAX; PERMUTATIONS];
    for shingle in &shingles {
        for (i, slot) in signature.iter_mut().enumerate() {
            let hash = permute(i, shingle);
            if hash < *slot {
                *slot = hash;
            }
        }
    }
    signature
}

fn estimated_jaccard(a: &[u64; PERMUTATIONS], b: &[u64; PERMUTATIONS]) -> f64 {
    let equal = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
    equal as f64 / PERMUTATIONS as f64
}

/// Near-duplicate candidate groups (§21.2). Reads the materialized bytes
/// itself so the analysis stays independent of the profile pipeline.
pub fn near_groups(
    workloads_root: &Path,
    rows: &[CandidateRow],
) -> Result<Vec<NearDuplicateGroup>, String> {
    // Files whose normalized token stream is a single token (or empty)
    // cannot be meaningfully compared; they are skipped and reported.
    let mut signatures: Vec<(usize, [u64; PERMUTATIONS])> = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        if row.profile_failure.is_some() {
            continue;
        }
        let path = crate::universe::materialized_path(workloads_root, &row.identity);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let tokens = normalize(&text);
        if tokens.len() < 2 {
            continue;
        }
        signatures.push((index, signature(&text)));
    }

    // Deterministic grouping: union-find over qualifying pairs, processed
    // in frozen lexical pair order.
    let mut parent: Vec<usize> = (0..rows.len()).collect();
    fn find(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }
    for i in 0..signatures.len() {
        for j in (i + 1)..signatures.len() {
            let (index_a, signature_a) = &signatures[i];
            let (index_b, signature_b) = &signatures[j];
            if estimated_jaccard(signature_a, signature_b) >= NEARDUP_THRESHOLD {
                let root_a = find(&mut parent, *index_a);
                let root_b = find(&mut parent, *index_b);
                if root_a != root_b {
                    // Union to the lexically smaller root for determinism.
                    if rows[root_a].lexical_key() < rows[root_b].lexical_key() {
                        parent[root_b] = root_a;
                    } else {
                        parent[root_a] = root_b;
                    }
                }
            }
        }
    }

    let mut clusters: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, _) in &signatures {
        let root = find(&mut parent, *index);
        clusters.entry(root).or_default().push(*index);
    }

    let mut groups = Vec::new();
    for (_, members) in clusters {
        if members.len() < 2 {
            continue;
        }
        // Only groups whose minimum pairwise similarity qualifies are
        // reported; per-member similarity detail is retained via the
        // minimum over the cluster.
        let mut min_similarity: f64 = 1.0;
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let signature_a = &signatures
                    .iter()
                    .find(|(index, _)| *index == members[i])
                    .map(|(_, signature)| signature)
                    .unwrap();
                let signature_b = &signatures
                    .iter()
                    .find(|(index, _)| *index == members[j])
                    .map(|(_, signature)| signature)
                    .unwrap();
                min_similarity = min_similarity.min(estimated_jaccard(signature_a, signature_b));
            }
        }
        let mut keys: Vec<String> = members
            .iter()
            .map(|index| {
                format!(
                    "{}/{}",
                    rows[*index].identity.source_id, rows[*index].identity.snapshot_path
                )
            })
            .collect();
        keys.sort();
        let mut projects: Vec<String> = members
            .iter()
            .map(|index| rows[*index].identity.source_id.clone())
            .collect();
        projects.sort();
        projects.dedup();
        groups.push(NearDuplicateGroup {
            members: keys,
            projects,
            estimated_jaccard: min_similarity,
        });
    }
    // Frozen group order: by first member, then size descending.
    groups.sort_by(|a, b| a.members[0].cmp(&b.members[0]));
    Ok(groups)
}

/// Full redundancy artifact. The acquisition exact-duplicate registry is
/// cross-checked: any disagreement is a hard error (the same authority
/// must produce the same groups).
pub fn analyze(workloads_root: &Path, rows: &[CandidateRow]) -> Result<RedundancyArtifact, String> {
    let exact = exact_groups(rows);
    let near = near_groups(workloads_root, rows)?;

    // Cross-check against the acquisition registry.
    let registry_path = workloads_root.join("manifests/exact-duplicates-v1.json");
    let registry: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&registry_path)
            .map_err(|error| format!("{}: {error}", registry_path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", registry_path.display()))?;
    let mut registry_hashes: Vec<String> = Vec::new();
    for group in registry["duplicate_groups"]
        .as_array()
        .unwrap_or(&Vec::new())
    {
        if let Some(hash) = group["sha256"].as_str() {
            registry_hashes.push(hash.to_string());
        }
    }
    registry_hashes.sort();
    let mut computed_hashes: Vec<String> = exact.iter().map(|group| group.sha256.clone()).collect();
    computed_hashes.sort();
    if registry_hashes != computed_hashes {
        return Err(format!(
            "exact-duplicate cross-check failed: acquisition registry has {} groups, profiling found {}",
            registry_hashes.len(),
            computed_hashes.len()
        ));
    }

    let mut notes = vec![
        "near-duplicate analysis is diagnostic only: groups are reported and used as tie-break information; no candidate is removed".to_string(),
        format!("pair threshold {NEARDUP_THRESHOLD} over {PERMUTATIONS}-component deterministic MinHash (no seed, no embeddings)"),
    ];
    let skipped = rows
        .iter()
        .filter(|row| row.profile_failure.is_some())
        .count();
    if skipped > 0 {
        notes.push(format!(
            "{skipped} rows with profile failures skipped from near-duplicate analysis"
        ));
    }

    Ok(RedundancyArtifact {
        schema: REDUNDANCY_SCHEMA.to_string(),
        method: NEARDUP_VERSION.to_string(),
        threshold: NEARDUP_THRESHOLD,
        shingle_width: SHINGLE_WIDTH,
        permutations: PERMUTATIONS,
        exact_groups: exact,
        near_groups: near,
        notes,
    })
}

/// Number of near-duplicate groups in which both this candidate and at
/// least one already-selected file are members (§25.2 tie-break 4).
pub fn near_conflict_count(
    artifact: &RedundancyArtifact,
    row: &CandidateRow,
    selected: &[&CandidateRow],
) -> u64 {
    let key = format!("{}/{}", row.identity.source_id, row.identity.snapshot_path);
    let selected_keys: std::collections::BTreeSet<String> = selected
        .iter()
        .map(|chosen| {
            format!(
                "{}/{}",
                chosen.identity.source_id, chosen.identity.snapshot_path
            )
        })
        .collect();
    artifact
        .near_groups
        .iter()
        .filter(|group| group.members.iter().any(|member| *member == key))
        .filter(|group| {
            group
                .members
                .iter()
                .any(|member| selected_keys.contains(member))
        })
        .count() as u64
}
