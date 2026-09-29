//! The offline required-effect ledger (L1.4).
//!
//! For each cell this independently determines what the frozen EAGER
//! semantics actually REQUIRED to change, using the frozen H0
//! normalized oracle (`parse_document` over the pre and post sources) —
//! an authority independent of both Horse-A arms.
//!
//! OFFLINE DIAGNOSTIC ORACLE != FREE ONLINE V2 MECHANISM: everything in
//! this module is derived by two full H0 parses and a tree diff. None
//! of it is available to a mechanism at update time without cost; no
//! future design may treat these numbers as zero-cost candidate
//! authority.

use std::collections::BTreeMap;

use markit_mdbench_full_rebuild::parse_document;
use markit_mdbench_oracle::normalized::{Node, NormalizedDocument};
use markit_mdbench_oracle::NormalizeV1;
use markit_mdbench_horse_a::state::ReadyDocument;

/// Facts-level effect summary (RefTable projections + first-wins map).
#[derive(serde::Serialize)]
pub struct FactsEffects {
    pub facts_pre: usize,
    pub facts_post: usize,
    pub facts_added: usize,
    pub facts_removed: usize,
    /// Facts whose (label, destination) pair changed at the same
    /// source-order position (value change).
    pub facts_value_changed: usize,
    /// Labels whose EFFECTIVE (first-wins) winner destination changed.
    pub effective_winner_changed: usize,
    /// Labels that gained an effective definition (unresolved -> resolved).
    pub labels_gained_definition: usize,
    /// Labels that lost their effective definition (resolved -> unresolved).
    pub labels_lost_definition: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub winner_changes: Vec<(String, String, String)>,
}

/// Output-level effect summary: multiset diff of normalized node
/// SIGNATURES (kind + all value fields; spans deliberately excluded so
/// bulk span shifts do not masquerade as semantic output changes —
/// span-only movement is counted separately as a structural-only
/// change).
#[derive(serde::Serialize, Default)]
pub struct OutputEffects {
    pub nodes_pre: usize,
    pub nodes_post: usize,
    /// Nodes whose (kind, fields) signature exists post but not pre.
    pub nodes_added: usize,
    /// Nodes whose signature exists pre but not post.
    pub nodes_removed: usize,
    /// Span-only changes among signature-paired nodes (topology/value
    /// identical, positions moved).
    pub nodes_span_only_changed: usize,
    /// Per-kind added/removed detail (kind name -> (added, removed)).
    pub by_kind: BTreeMap<String, (i64, i64)>,
    /// ReferenceLink nodes whose resolved destination changed
    /// (derived from the winner map, counted at node level).
    pub ref_links_dest_changed: usize,
    /// Nodes whose signature changed value-side only (counted once).
    pub value_changed_nodes: usize,
    /// Text -> ReferenceLink topology flips.
    pub text_to_ref_link: i64,
    /// ReferenceLink -> Text topology flips.
    pub ref_link_to_text: i64,
}

/// The complete required-effect ledger for one cell.
#[derive(serde::Serialize)]
pub struct EffectLedger {
    pub cell_id: String,
    /// Definition facts changed at all?
    pub definition_facts_changed: bool,
    /// Effective winner identity/value changed?
    pub effective_winner_changed: bool,
    /// Reference existence changed (either direction)?
    pub reference_existence_changed: bool,
    pub facts: FactsEffects,
    pub outputs: OutputEffects,
    /// Minimum number of semantically changed outputs (nodes added +
    /// removed + destination-changed ReferenceLinks) — the eager
    /// obligation floor.
    pub semantically_changed_outputs: usize,
}

/// The frozen first-wins winner map of an ordered fact projection.
fn winner_map(entries: &[(String, String)]) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for (l, d) in entries {
        m.entry(l.clone()).or_insert_with(|| d.clone());
    }
    m
}

/// Ordered-sequence facts diff (no alignment magic: report counts of
/// added/removed/value-changed via position-wise comparison over the
/// common prefix length plus the tail).
fn facts_diff(pre: &[(String, String)], post: &[(String, String)]) -> (usize, usize, usize) {
    let common = pre.len().min(post.len());
    let mut value_changed = 0usize;
    for i in 0..common {
        if pre[i] != post[i] {
            value_changed += 1;
        }
    }
    // Beyond the common prefix, count as added/removed; among those,
    // same-label pairs count as value changes rather than add+remove
    // when exactly one of each side matches by label.
    let added = post.len().saturating_sub(common);
    let removed = pre.len().saturating_sub(common);
    // label-preserving refinement on the tails
    let pre_tail = &pre[common..];
    let post_tail = &post[common..];
    let mut used_post = vec![false; post_tail.len()];
    let mut pure_value = 0usize;
    for (l, _) in pre_tail {
        for (j, (l2, _)) in post_tail.iter().enumerate() {
            if !used_post[j] && l2 == l {
                used_post[j] = true;
                pure_value += 1;
                break;
            }
        }
    }
    let pure_add = added.saturating_sub(pure_value);
    let pure_remove = removed.saturating_sub(pure_value);
    (pure_add + pure_value, pure_remove + pure_value, value_changed + pure_value)
}

/// One node's semantic signature (kind + pinned value fields; spans and
/// content intervals are positions, not values — content is included
/// because it is byte territory, not semantic value).
fn signature(n: &Node) -> String {
    let mut s = String::with_capacity(48);
    s.push_str(n.kind.name());
    if let Some(l) = n.level {
        s.push_str(&format!(" L{l}"));
    }
    if let Some(m) = &n.marker {
        s.push_str(&format!(" M{m}"));
    }
    if let Some(i) = &n.info {
        s.push_str(&format!(" I{i}"));
    }
    if let Some(d) = &n.destination {
        s.push_str(&format!(" D{d}"));
    }
    if let Some(l) = &n.label {
        s.push_str(&format!(" B{l}"));
    }
    if n.content.is_some() {
        s.push_str(" C");
    }
    s
}

fn count_by_signature(doc: &NormalizedDocument) -> (BTreeMap<String, i64>, usize, BTreeMap<String, Vec<(usize, usize)>>) {
    let mut map: BTreeMap<String, i64> = BTreeMap::new();
    let mut spans: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let mut total = 0usize;
    fn walk(n: &Node, map: &mut BTreeMap<String, i64>, spans: &mut BTreeMap<String, Vec<(usize, usize)>>, total: &mut usize) {
        let sig = signature(n);
        *map.entry(sig.clone()).or_insert(0) += 1;
        spans.entry(sig).or_default().push((n.start, n.end));
        *total += 1;
        for c in &n.children {
            walk(c, map, spans, total);
        }
    }
    walk(&doc.root, &mut map, &mut spans, &mut total);
    (map, total, spans)
}

/// Count signature classes whose multiplicity is IDENTICAL pre/post but
/// whose span sets differ — semantically unchanged outputs that merely
/// moved (class-level, conservative).
fn span_only_changes(pre: &NormalizedDocument, post: &NormalizedDocument) -> usize {
    let (pre_map, _, pre_spans) = count_by_signature(pre);
    let (post_map, _, post_spans) = count_by_signature(post);
    let mut moved = 0usize;
    for (sig, pre_count) in &pre_map {
        if let Some(post_count) = post_map.get(sig) {
            if post_count == pre_count {
                let mut a = pre_spans[sig].clone();
                let mut b = post_spans[sig].clone();
                a.sort_unstable();
                b.sort_unstable();
                if a != b {
                    moved += 1;
                }
            }
        }
    }
    moved
}

/// Derive the complete effect ledger for one cell from its pre/post
/// sources and the Horse-A pre/post states (whose `refs` are the
/// definition-facts authority; the normalized trees come from H0).
pub fn effect_ledger(
    cell_id: &str,
    pre_source: &str,
    post_source: &str,
    pre_state: &ReadyDocument,
    post_state_a: &ReadyDocument,
) -> EffectLedger {
    let pre_refs: Vec<(String, String)> = pre_state.refs.entries().to_vec();
    let post_refs: Vec<(String, String)> = post_state_a.refs.entries().to_vec();

    let (facts_added, facts_removed, facts_value_changed) = facts_diff(&pre_refs, &post_refs);

    let pre_winners = winner_map(&pre_refs);
    let post_winners = winner_map(&post_refs);
    let mut winner_changes = Vec::new();
    let mut gained = 0usize;
    let mut lost = 0usize;
    for (label, new_dest) in &post_winners {
        match pre_winners.get(label) {
            Some(old_dest) if old_dest != new_dest => {
                winner_changes.push((label.clone(), old_dest.clone(), new_dest.clone()));
            }
            None => gained += 1,
            _ => {}
        }
    }
    for label in pre_winners.keys() {
        if !post_winners.contains_key(label) {
            lost += 1;
        }
    }

    // Normalized trees from the INDEPENDENT H0 oracle.
    let pre_doc = parse_document(pre_source.as_bytes());
    let post_doc = parse_document(post_source.as_bytes());

    let (pre_map, nodes_pre, _) = count_by_signature(&pre_doc);
    let (post_map, nodes_post, _) = count_by_signature(&post_doc);

    let mut by_kind: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut nodes_added = 0usize;
    let mut nodes_removed = 0usize;
    let mut keys: Vec<&String> = pre_map.keys().chain(post_map.keys()).collect();
    keys.sort();
    keys.dedup();
    // First aggregate signature deltas per KIND so pure value changes
    // (same kind, same multiplicity, different field) can be counted
    // once as a changed node instead of add+remove.
    let mut kind_deltas: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for k in &keys {
        let p = pre_map.get(*k).copied().unwrap_or(0);
        let q = post_map.get(*k).copied().unwrap_or(0);
        if p != q {
            let kind = k.split(' ').next().unwrap_or(k);
            let e = kind_deltas.entry(kind.to_string()).or_insert((0, 0));
            if q > p {
                e.0 += q - p;
            } else {
                e.1 += p - q;
            }
        }
    }
    let mut value_changed_nodes = 0usize;
    for (kind, (add, rem)) in &kind_deltas {
        let e = by_kind.entry(kind.clone()).or_insert((0, 0));
        e.0 = *add;
        e.1 = *rem;
        if add == rem {
            // balanced: `add` nodes of this kind changed value (their
            // old signatures left, new ones arrived)
            value_changed_nodes += *add as usize;
        } else {
            nodes_added += *add as usize;
            nodes_removed += *rem as usize;
        }
    }

    let text_delta = kind_delta(&by_kind, "Text");
    let ref_link_delta = kind_delta(&by_kind, "ReferenceLink");

    // ReferenceLink destination changes: winner-map labels with changed
    // destinations. Counted at NODE multiplicity: a balanced
    // ReferenceLink add/remove pair count IS the set of re-pointed
    // links; otherwise fall back to the winner-label count.
    let ref_links_dest_changed = match by_kind.get("ReferenceLink") {
        Some((add, rem)) if add == rem && *add > 0 => *add as usize,
        _ => winner_changes.len(),
    };

    let span_moved = span_only_changes(&pre_doc, &post_doc);

    let definition_facts_changed =
        facts_added > 0 || facts_removed > 0 || facts_value_changed > 0;
    let effective_winner_changed = !winner_changes.is_empty();
    let reference_existence_changed = gained > 0 || lost > 0;

    // Outputs that genuinely had to change: true topology changes plus
    // value-changed nodes counted ONCE (a value change is one changed
    // output, not an added plus a removed one).
    let semantically_changed_outputs =
        nodes_added + nodes_removed + value_changed_nodes;

    EffectLedger {
        cell_id: cell_id.to_string(),
        definition_facts_changed,
        effective_winner_changed,
        reference_existence_changed,
        facts: FactsEffects {
            facts_pre: pre_refs.len(),
            facts_post: post_refs.len(),
            facts_added,
            facts_removed,
            facts_value_changed,
            effective_winner_changed: winner_changes.len(),
            labels_gained_definition: gained,
            labels_lost_definition: lost,
            winner_changes,
        },
        outputs: OutputEffects {
            nodes_pre,
            nodes_post,
            nodes_added,
            nodes_removed,
            nodes_span_only_changed: span_moved,
            by_kind,
            value_changed_nodes,
            ref_links_dest_changed,
            text_to_ref_link: text_delta.0,
            ref_link_to_text: text_delta.1,
        },
        semantically_changed_outputs,
    }
}

fn kind_delta(by_kind: &BTreeMap<String, (i64, i64)>, kind: &str) -> (i64, i64) {
    by_kind.get(kind).copied().unwrap_or((0, 0))
}

/// H0-normalized parse export for a Horse-A state (used by parity
/// checks): delegates to the state's own NormalizeV1 (frozen export).
pub fn horse_a_normalized(state: &ReadyDocument) -> NormalizedDocument {
    state.normalize_v1()
}
