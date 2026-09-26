//! READY-invariant validators (task #45): pure, panic-free checks over
//! an existing state, for tests, debug assertions, and focused review.
//! These are construction/diagnostic surfaces — no post-update
//! attribution is built here, and no benchmark treatment runs them in
//! its timed path.
//!
//! `full_build` does not run them on its production path (independent
//! I2 review: the frozen mechanism requires no redundant post-build
//! whole-state audit walk); debug builds keep the check, and tests
//! invoke the validators explicitly.

use markit_mdbench_oracle::normalized::{Node, NodeKind};

use crate::state::{AvlNode, OwnerPayload, ReadyDocument};

/// Every READY invariant in one gate (spec §2.1).
pub fn validate_ready(doc: &ReadyDocument) -> Result<(), String> {
    validate_coverage(doc)?;
    validate_full_build_tree(&doc.owners)?;
    validate_certificates(doc)?;
    validate_owner_relative_payload(doc)?;
    validate_ref_table_projection(doc)?;
    Ok(())
}

/// Owner coverage exactly partitions `[0, source_len)` — no gap, no
/// overlap, no zero-length record (spec §3.2).
pub fn validate_coverage(doc: &ReadyDocument) -> Result<(), String> {
    let mut base = 0usize;
    for owner in doc.owners.owners_in_order() {
        if owner.coverage_len == 0 {
            return Err("a zero-length Owner exists".to_string());
        }
        if base + owner.coverage_len > doc.source_len {
            return Err(format!(
                "Owner coverage [{base}, {}) exceeds the source length {}",
                base + owner.coverage_len,
                doc.source_len
            ));
        }
        base += owner.coverage_len;
    }
    if base != doc.source_len {
        return Err(format!(
            "coverage sums to {base} but the source length is {}",
            doc.source_len
        ));
    }
    Ok(())
}

/// Construction-only sequence invariants (task #33): in-order order
/// preserved, AVL balance holds, height and the three aggregates are
/// correct for every node.
pub fn validate_full_build_tree(seq: &crate::state::OwnerSeq) -> Result<(), String> {
    fn walk(node: &AvlNode) -> Result<(u32, usize, usize, bool), String> {
        let left = match &node.left {
            Some(l) => walk(l)?,
            None => (0, 0, 0, false),
        };
        let right = match &node.right {
            Some(r) => walk(r)?,
            None => (0, 0, 0, false),
        };
        let (lh, lb, lr, ls) = left;
        let (rh, rb, rr, rs) = right;
        if lh.abs_diff(rh) > 1 {
            return Err(format!(
                "AVL balance violated: left height {lh}, right height {rh}"
            ));
        }
        if node.owner.coverage_len == 0 {
            return Err("a zero-length Owner exists in the tree".to_string());
        }
        let height = 1 + lh.max(rh);
        let agg = crate::state::Aggregate {
            subtree_bytes: lb + node.owner.coverage_len + rb,
            subtree_records: lr + 1 + rr,
            subtree_has_safe: ls || node.owner.outgoing_restart.is_some() || rs,
        };
        if node.height != height {
            return Err(format!(
                "node height {} != recomputed {height}",
                node.height
            ));
        }
        if node.agg != agg {
            return Err(format!(
                "node aggregate {:?} != recomputed {agg:?}",
                node.agg
            ));
        }
        Ok((
            height,
            agg.subtree_bytes,
            agg.subtree_records,
            agg.subtree_has_safe,
        ))
    }
    match &seq.root {
        Some(root) => {
            walk(root)?;
        }
        None => {
            if seq.records() != 0 || seq.total_bytes() != 0 || seq.has_safe() {
                return Err("empty sequence reports a non-empty aggregate".to_string());
            }
        }
    }
    Ok(())
}

/// Every persistent certificate corresponds to one real interior Owner
/// boundary and real parser evidence, holds Owner-relative support that
/// does not cross the Owner's coverage start, and ends exactly at the
/// outgoing boundary (tasks #19/#30). The EOF boundary is never
/// certified: a certified Owner must have a successor record.
pub fn validate_certificates(doc: &ReadyDocument) -> Result<(), String> {
    let mut base = 0usize;
    for owner in doc.owners.owners_in_order() {
        if let Some(cert) = &owner.outgoing_restart {
            if base + owner.coverage_len >= doc.source_len {
                return Err(format!(
                    "a certificate sits on the EOF boundary (Owner coverage ends at {})",
                    base + owner.coverage_len
                ));
            }
            let support = &cert.support;
            if support.blank_line.is_empty() {
                return Err("certificate support has an empty blank line".to_string());
            }
            if support.blank_line.end != owner.coverage_len {
                return Err(format!(
                    "certificate support ends at {} but the Owner's outgoing boundary is {}",
                    support.blank_line.end, owner.coverage_len
                ));
            }
            match support.preceding_lf {
                // `None` is legal only when the blank physical line truly
                // begins at the certificate Owner's base — for a full
                // build that can only be the document BOF itself.
                None => {
                    if support.blank_line.start != 0 {
                        return Err(format!(
                            "BOF support (no preceding LF) but the blank line starts at {}",
                            support.blank_line.start
                        ));
                    }
                }
                Some(p) => {
                    if p + 1 != support.blank_line.start {
                        return Err(format!(
                            "preceding LF {p} is not the byte immediately before the blank line start {}",
                            support.blank_line.start
                        ));
                    }
                }
            }
            // The contiguous support span never crosses the Owner start:
            // it lies inside [0, coverage_len) by the boundary check.
            let _ = support.support_span();
        }
        base += owner.coverage_len;
    }
    Ok(())
}

/// Total relative-coordinate check (task #40): every retained semantic
/// coordinate of every Syntax Owner lies within that Owner's
/// coverage-relative range, parents contain their children, sibling
/// starts never decrease, and every `FencedCode.content` interval stays
/// inside its fence span. This catches an accidental absolute field that
/// export equality might hide.
pub fn validate_owner_relative_payload(doc: &ReadyDocument) -> Result<(), String> {
    for (base, owner) in owner_bases(doc) {
        if let OwnerPayload::Syntax(payload) = &owner.payload {
            check_relative(&payload.root, base, owner.coverage_len)?;
        }
    }
    Ok(())
}

fn check_relative(node: &Node, base: usize, coverage_len: usize) -> Result<(), String> {
    if node.start > node.end {
        return Err(format!(
            "inverted span [{}, {}) in a {:?} node at base {base}",
            node.start, node.end, node.kind
        ));
    }
    if node.end > coverage_len {
        return Err(format!(
            "{:?} span end {} exceeds the Owner-relative range [0, {coverage_len}) — \
             an absolute coordinate leaked into retained state (base {base})",
            node.kind, node.end
        ));
    }
    if let Some((a, b)) = node.content {
        if !(node.start <= a && a <= b && b <= node.end) {
            return Err(format!(
                "{:?} content interval ({a}, {b}) escapes its fence span [{}, {})",
                node.kind, node.start, node.end
            ));
        }
    }
    let mut prev_start: Option<usize> = None;
    for child in &node.children {
        if child.start < node.start || child.end > node.end {
            return Err(format!(
                "{:?} child span [{}, {}) escapes its {:?} parent [{}, {})",
                child.kind, child.start, child.end, node.kind, node.start, node.end
            ));
        }
        if prev_start.is_some_and(|ps| child.start < ps) {
            return Err(format!(
                "sibling starts decrease before a {:?} child at {}",
                child.kind, child.start
            ));
        }
        prev_start = Some(child.start);
        check_relative(child, base, coverage_len)?;
    }
    Ok(())
}

/// The document RefTable is exactly the source-order projection of all
/// retained ReferenceDefinition facts (spec §2.4) — duplicates included,
/// in dispatch order (which is pre-order over the retained payloads).
pub fn validate_ref_table_projection(doc: &ReadyDocument) -> Result<(), String> {
    let mut facts: Vec<(String, String)> = Vec::new();
    for owner in doc.owners.owners_in_order() {
        if let OwnerPayload::Syntax(payload) = &owner.payload {
            collect_definition_facts(&payload.root, &mut facts);
        }
    }
    if facts.as_slice() != doc.refs.entries() {
        return Err(format!(
            "RefTable is not the source-order projection of the retained definition facts: \
             payload projects {facts:?}, table holds {:?}",
            doc.refs.entries()
        ));
    }
    Ok(())
}

fn collect_definition_facts(node: &Node, out: &mut Vec<(String, String)>) {
    if node.kind == NodeKind::ReferenceDefinition {
        if let (Some(label), Some(destination)) = (&node.label, &node.destination) {
            out.push((label.clone(), destination.clone()));
        }
    }
    for child in &node.children {
        collect_definition_facts(child, out);
    }
}

/// (base, owner) pairs in source order — the byte-weight prefix sums.
fn owner_bases(doc: &ReadyDocument) -> Vec<(usize, &crate::state::Owner)> {
    let mut out = Vec::with_capacity(doc.owners.records());
    doc.owners
        .for_each_in_order(|base, owner| out.push((base, owner)));
    out
}
