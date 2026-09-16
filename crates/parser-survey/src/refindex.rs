//! RUN-4 — research-only ReferenceIndex (issue #19 plan §23–27;
//! CORRECTIVE-1 rework per adversarial review MAJOR-3).
//!
//! Question (H3/Q8): does a semantic dependency index let a GLOBAL
//! meaning change keep a LOCAL syntax parse — i.e. is semantic fanout a
//! separate axis (D-axis) that neither inflates the syntax reparse
//! radius nor requires full-document work?
//!
//! IDENTITY vs ORDER (CORRECTIVE-1): the first RUN-4 prototype stored
//! candidate definitions as `(doc_order, block_id, url)` but wrote the
//! block id into BOTH slots and resolved with `min_by_key(order)` —
//! stable identity and document position were conflated. That resolves
//! the review's counterexample wrongly (a NEW definition block with a
//! LARGER stable id inserted BEFORE the current winner must win; the old
//! index kept the smaller id). This version separates them:
//!
//! - `BlockId` — stable semantic identity, assigned once at block
//!   creation, never reused, NOT compared for order anywhere.
//! - `DocOrder` — the current document order, supplied by the syntax
//!   layer (the block sequence already knows it); the index never owns
//!   a second order authority.
//!
//! EXPERIMENTAL_SUBSET (plan §26 allows this if stated): link reference
//! definitions are recognized only as whole-paragraph def lines
//! (`[label]: url "title"?`), because markit L1 has no ref-def block
//! construct (ORACLE-B finding). Label normalization follows CommonMark:
//! Unicode-lowercase + collapse internal whitespace. First definition in
//! document order wins; later duplicates are candidates only.
//! Definitions inside containers are UNSUPPORTED here.
//!
//! Index invariants (plan §24–25):
//! - per-block `BlockDeps { defs, uses }` extracted from block text;
//! - `update_block` / `remove_block` diff and return a `SemanticDelta`
//!   (affected labels → invalidated user blocks), where only CHANGED
//!   resolutions count — editing a losing duplicate is a zero-fanout
//!   event;
//! - ORACLE (self-equivalence for the index): after every incremental
//!   update, the index must equal a clean rebuild over the same ordered
//!   blocks (resolution of every label + user sets).

use std::collections::BTreeMap;

/// Stable block identity. NEVER used for ordering; ordering comes from
/// [`DocOrder`].
pub type BlockId = u64;

/// Current document order, as provided by the syntax layer (its block
/// sequence). `rank` must return the block's 0-based document position.
/// The semantic index must not keep its own order authority — it asks.
pub trait DocOrder {
    fn rank(&self, block: BlockId) -> u64;
}

/// Rank provider over an explicit ordered id list — what a
/// stable-identity block index hands the semantic layer after each
/// structural update. Unknown ids rank last (defensive; callers are
/// expected to pass a fresh map).
pub struct RankMap(BTreeMap<BlockId, u64>);

impl RankMap {
    pub fn from_order(order: &[BlockId]) -> RankMap {
        RankMap(order.iter().enumerate().map(|(i, &b)| (b, i as u64)).collect())
    }
}

impl DocOrder for RankMap {
    fn rank(&self, block: BlockId) -> u64 {
        self.0.get(&block).copied().unwrap_or(u64::MAX)
    }
}

/// CommonMark label normalization: lowercase + collapse whitespace.
fn normalize_label(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_ws = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            in_ws = !out.is_empty();
            continue;
        }
        if in_ws {
            out.push(' ');
            in_ws = false;
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// One parsed block's dependencies. `defs` are candidate definitions in
/// source order; `uses` are shorthand reference labels used.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockDeps {
    pub defs: Vec<String>, // normalized labels
    pub uses: Vec<String>, // normalized labels
}

/// Extract defs/uses from a paragraph's raw text (research extraction).
pub fn extract_deps(text: &str) -> BlockDeps {
    let mut deps = BlockDeps::default();
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix('[') {
            if let Some(close) = rest.find("]:") {
                let label = &rest[..close];
                // def line: nothing but the URL/title after `]:`
                let after = rest[close + 2..].trim();
                let looks_like_url = after.is_empty()
                    || after.starts_with('/')
                    || after.starts_with('#')
                    || after.starts_with('<')
                    || (after.chars().next().is_some_and(|c| !c.is_whitespace())
                        && !after.starts_with('['));
                if !label.is_empty() && looks_like_url {
                    deps.defs.push(normalize_label(label));
                    continue;
                }
            }
        }
        // shorthand uses: [label], [label][], ![label]
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'[' {
                if let Some(close_rel) = line[i + 1..].find(']') {
                    let close = i + 1 + close_rel;
                    let label = &line[i + 1..close];
                    let inline_link = line[close + 1..].trim_start().starts_with('(');
                    if !label.is_empty() && !inline_link {
                        deps.uses.push(normalize_label(label));
                    }
                    i = close + 1;
                    continue;
                }
            }
            i += 1;
        }
    }
    deps
}

/// The semantic resolution of one label: resolved url or unresolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Resolved(String),
    Unresolved,
}

#[derive(Clone, Default, Debug)]
pub struct ReferenceIndex {
    /// candidate defs per label: (stable BlockId, url). Order is NOT
    /// stored; resolution consults a `DocOrder` provider (CORRECTIVE-1).
    candidates: BTreeMap<String, Vec<(BlockId, String)>>,
    /// users per label: stable block ids
    users: BTreeMap<String, std::collections::BTreeSet<BlockId>>,
    /// per-block extracted deps (for incremental diffing)
    deps: BTreeMap<BlockId, BlockDeps>,
}

#[derive(Debug, Default)]
pub struct SemanticDelta {
    /// (user block id, label, new resolution) for every user of every
    /// touched label. The harness diffs against pre-update resolutions
    /// to count genuinely changed ones.
    pub invalidated: Vec<(BlockId, String, Resolution)>,
    pub labels_touched: usize,
}

impl ReferenceIndex {
    /// Build from blocks ALREADY IN DOCUMENT ORDER (the syntax layer's
    /// sequence). Ids are stable identities supplied by the caller.
    pub fn build(ordered_blocks: &[(BlockId, String)]) -> ReferenceIndex {
        let mut idx = ReferenceIndex::default();
        for (id, text) in ordered_blocks {
            let deps = extract_deps(text);
            for (order, label) in deps.defs.iter().enumerate() {
                let url = def_url(text, order);
                idx.candidates
                    .entry(label.clone())
                    .or_default()
                    .push((*id, url));
            }
            for label in &deps.uses {
                idx.users.entry(label.clone()).or_default().insert(*id);
            }
            idx.deps.insert(*id, deps);
        }
        idx
    }

    /// Resolution of `label`: the first candidate in CURRENT DOCUMENT
    /// ORDER (via the provider), never by id comparison.
    pub fn resolution(&self, label: &str, order: &dyn DocOrder) -> Resolution {
        match self.candidates.get(label) {
            Some(cands) if !cands.is_empty() => {
                let best = cands
                    .iter()
                    .min_by_key(|(id, _)| order.rank(*id))
                    .unwrap();
                Resolution::Resolved(best.1.clone())
            }
            _ => Resolution::Unresolved,
        }
    }

    /// All labels known to the index (candidates ∪ users).
    pub fn labels(&self) -> impl Iterator<Item = &String> {
        self.candidates.keys().chain(self.users.keys())
    }

    /// User set for one label (for index-equality oracles).
    pub fn users_set(&self, label: &str) -> std::collections::BTreeSet<BlockId> {
        self.users.get(label).cloned().unwrap_or_default()
    }

    /// Plan §25: replace block `id`'s deps (insert == update with no
    /// prior entry). Returns the touched labels' user sets with the NEW
    /// resolution per label. Exact changed-vs-unchanged diffing is the
    /// harness's job: it captures `old_index.resolution(label, order)`
    /// before the update.
    pub fn update_block(
        &mut self,
        id: BlockId,
        new_deps: BlockDeps,
        new_text: &str,
        order: &dyn DocOrder,
    ) -> SemanticDelta {
        let old = self.deps.remove(&id).unwrap_or_default();

        let mut affected: Vec<String> = old.defs.clone();
        affected.extend(new_deps.defs.clone());
        // labels whose USER set can change: old uses and new uses of id
        let mut user_labels: Vec<String> = old.uses.clone();
        user_labels.extend(new_deps.uses.clone());

        // remove old contributions
        for label in &old.defs {
            if let Some(cands) = self.candidates.get_mut(label) {
                cands.retain(|&(block, _)| block != id);
                if cands.is_empty() {
                    self.candidates.remove(label);
                }
            }
        }
        for label in &old.uses {
            if let Some(us) = self.users.get_mut(label) {
                us.remove(&id);
                if us.is_empty() {
                    self.users.remove(label);
                }
            }
        }

        // add new contributions
        let mut def_i = 0usize;
        for label in &new_deps.defs {
            let url = def_url(new_text, def_i);
            self.candidates.entry(label.clone()).or_default().push((id, url));
            def_i += 1;
        }
        for label in &new_deps.uses {
            self.users.entry(label.clone()).or_default().insert(id);
        }
        self.deps.insert(id, new_deps);

        // collect (users × new resolution) over every touched label
        let mut delta = SemanticDelta::default();
        let mut seen = std::collections::BTreeSet::new();
        let mut all_labels = affected;
        all_labels.extend(user_labels);
        for label in all_labels {
            if !seen.insert(label.clone()) {
                continue;
            }
            delta.labels_touched += 1;
            let new_res = self.resolution(&label, order);
            if let Some(us) = self.users.get(&label) {
                for &user in us {
                    delta.invalidated.push((user, label.clone(), new_res.clone()));
                }
            }
        }
        delta
    }

    /// Block `id` left the document (delete/absorb): drop its
    /// contributions; touched labels are reported exactly like
    /// `update_block` (old deps are diffed from the map, nothing is
    /// added back).
    pub fn remove_block(&mut self, id: BlockId, order: &dyn DocOrder) -> SemanticDelta {
        self.update_block(id, BlockDeps::default(), "", order)
    }
}

/// Extract the `order`-th def URL from a def paragraph's text.
fn def_url(text: &str, order: usize) -> String {
    let mut n = 0;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix('[') {
            if let Some(close) = rest.find("]:") {
                if n == order {
                    return rest[close + 2..].trim().trim_matches('"').to_string();
                }
                n += 1;
            }
        }
    }
    String::new()
}

// --- Correctness fixtures (plan §26 + CORRECTIVE-1 winner cases) ----------

pub fn fixtures() -> Result<(), String> {
    fn expect_resolution(doc: &[&str], use_label: &str, want: &str) -> Result<(), String> {
        let blocks: Vec<(BlockId, String)> =
            doc.iter().enumerate().map(|(i, s)| (i as u64, s.to_string())).collect();
        let idx = ReferenceIndex::build(&blocks);
        let rank = RankMap::from_order(
            &(0..blocks.len() as u64).collect::<Vec<_>>(),
        );
        let want_s = if want == "<unresolved>" {
            Resolution::Unresolved
        } else {
            Resolution::Resolved(want.into())
        };
        if idx.resolution(use_label, &rank) != want_s {
            return Err(format!(
                "fixture failed: [{use_label}] = {:?}, want {want_s:?} in {doc:?}",
                idx.resolution(use_label, &rank)
            ));
        }
        Ok(())
    }

    expect_resolution(&["[x]: /a", "use [x] here"], "x", "/a")?;
    // definition AFTER use
    expect_resolution(&["use [x] here", "[x]: /a"], "x", "/a")?;
    // duplicate defs: first wins
    expect_resolution(&["[x]: /first", "use [x]", "[x]: /second"], "x", "/first")?;
    // case normalization
    expect_resolution(&["[Foo]: /a", "use [foo]"], "foo", "/a")?;
    expect_resolution(&["[Foo]: /a", "use [f o o]"], "foo", "/a")?;
    // whitespace normalization
    expect_resolution(&["[F  o]: /a", "use [f o]"], "f o", "/a")?;
    // undefined reference
    expect_resolution(&["use [nope]"], "nope", "<unresolved>")?;
    // images and collapsed refs count as uses; inline links do not
    let deps = extract_deps("see ![pic] and [ref][] but not [inline](/url)");
    let mut u = deps.uses.clone();
    u.sort();
    if u != vec!["pic", "ref"] {
        return Err(format!("use extraction wrong: {u:?}"));
    }

    // CORRECTIVE-1: identity ≠ order. The review's counterexample: the
    // winning def has a SMALL id; a new def block inserted BEFORE it
    // gets a LARGER id — CommonMark says the new (earlier) def wins.
    {
        let blocks: Vec<(BlockId, String)> = vec![
            (10, "[x]: /old".into()),
            (11, "use [x]".into()),
        ];
        let idx = ReferenceIndex::build(&blocks);
        // insert id 999 BEFORE id 10
        let mut new_blocks: Vec<(BlockId, String)> =
            vec![(999, "[x]: /new".into())];
        new_blocks.extend(blocks.iter().cloned());
        let mut idx2 = idx.clone();
        let deps = extract_deps("[x]: /new");
        let rank_after = RankMap::from_order(
            &new_blocks.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
        );
        idx2.update_block(999, deps, "[x]: /new", &rank_after);
        if idx2.resolution("x", &rank_after) != Resolution::Resolved("/new".into()) {
            return Err("identity≠order: inserted id 999 before winner id 10 must win \
                        by document position (got the old winner)"
                .into());
        }
    }
    // Moving an EXISTING def (same stable id) before the winner flips
    // the winner — pure rank change, no id change.
    {
        let blocks: Vec<(BlockId, String)> = vec![
            (117, "[x]: /old".into()),
            (224, "[x]: /moved".into()),
        ];
        let idx = ReferenceIndex::build(&blocks);
        let rank0 = RankMap::from_order(&[117, 224]);
        if idx.resolution("x", &rank0) != Resolution::Resolved("/old".into()) {
            return Err("move fixture precondition failed".into());
        }
        // move id 224 to the front: same ids, new order
        let rank1 = RankMap::from_order(&[224, 117]);
        if idx.resolution("x", &rank1) != Resolution::Resolved("/moved".into()) {
            return Err("identity≠order: moving id 224 before id 117 must flip the \
                        winner without any id change"
                .into());
        }
    }
    Ok(())
}
