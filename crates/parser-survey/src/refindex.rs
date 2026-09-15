//! RUN-4 — research-only ReferenceIndex (issue #19 plan §23–27).
//!
//! Question (H3/Q8): does a semantic dependency index let a GLOBAL
//! meaning change keep a LOCAL syntax parse — i.e. is semantic fanout a
//! separate axis (D-axis) that neither inflates the syntax reparse
//! radius nor requires full-document work?
//!
//! EXPERIMENTAL_SUBSET (plan §26 allows this if stated): link reference
//! definitions are recognized only as whole-paragraph def lines
//! (`[label]: url "title"?`), because markit L1 has no ref-def block
//! construct (ORACLE-B finding). Label normalization follows CommonMark:
//! Unicode-lowercase + collapse internal whitespace. First definition
//! in document order wins; later duplicates are candidates only.
//! Definitions inside containers are UNSUPPORTED here.
//!
//! Index invariants (plan §24–25):
//! - per-block `BlockDeps { defs, uses }` extracted from block text;
//! - `update(k, old_deps, new_deps)` diffs and returns a
//!   `SemanticDelta` (affected labels → invalidated user blocks), where
//!   only CHANGED resolutions count — editing a losing duplicate is a
//!   zero-fanout event;
//! - ORACLE (self-equivalence for the index): after every incremental
//!   update, the index must equal a clean rebuild.

use std::collections::BTreeMap;

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
    /// candidate defs per label: (doc order, block id, url)
    candidates: BTreeMap<String, Vec<(usize, usize, String)>>,
    /// users per label: block ids
    users: BTreeMap<String, std::collections::BTreeSet<usize>>,
    /// per-block extracted deps (for incremental diffing)
    deps: BTreeMap<usize, BlockDeps>,
}

#[derive(Debug, Default)]
pub struct SemanticDelta {
    /// (user block id, label, new resolution) for every user of every
    /// touched label. The harness diffs against pre-update resolutions
    /// to count genuinely changed ones.
    pub invalidated: Vec<(usize, String, Resolution)>,
    pub labels_touched: usize,
}

impl ReferenceIndex {
    pub fn build(blocks: &[(usize, String)]) -> ReferenceIndex {
        let mut idx = ReferenceIndex::default();
        let mut ordered: Vec<&(usize, String)> = blocks.iter().collect();
        ordered.sort_by_key(|b| b.0);
        for &&(id, ref text) in &ordered {
            let deps = extract_deps(text);
            for (order, label) in deps.defs.iter().enumerate() {
                let url = def_url(text, order);
                // doc order = block id in this prototype (ids ascend)
                idx.candidates
                    .entry(label.clone())
                    .or_default()
                    .push((id, id, url));
            }
            for label in &deps.uses {
                idx.users.entry(label.clone()).or_default().insert(id);
            }
            idx.deps.insert(id, deps);
        }
        idx
    }

    pub fn resolution(&self, label: &str) -> Resolution {
        match self.candidates.get(label) {
            Some(cands) if !cands.is_empty() => {
                let best = cands.iter().min_by_key(|(order, _, _)| *order).unwrap();
                Resolution::Resolved(best.2.clone())
            }
            _ => Resolution::Unresolved,
        }
    }

    /// All labels known to the index (candidates ∪ users).
    pub fn labels(&self) -> impl Iterator<Item = &String> {
        self.candidates.keys().chain(self.users.keys())
    }

    /// Total user-set size across labels (cheap index equality proxy
    /// paired with per-label resolution comparison).
    pub fn users_of_lens(&self) -> usize {
        self.users.values().map(|s| s.len()).sum()
    }

    /// Plan §25: replace block `k`'s deps. Returns the touched labels'
    /// user sets with the NEW resolution per label. Exact
    /// changed-vs-unchanged diffing is the harness's job: it captures
    /// `old_index.resolution(label)` before the update (the real system
    /// diffs resolutions inside the index; the split only keeps the
    /// timed region honest about what it measures).
    pub fn update_block(
        &mut self,
        k: usize,
        new_deps: BlockDeps,
        new_text: &str,
    ) -> SemanticDelta {
        let old = self.deps.remove(&k).unwrap_or_default();

        let mut affected: Vec<String> = old.defs.clone();
        affected.extend(new_deps.defs.clone());
        // labels whose USER set can change: old uses and new uses of k
        let mut user_labels: Vec<String> = old.uses.clone();
        user_labels.extend(new_deps.uses.clone());

        // remove old contributions
        for label in &old.defs {
            if let Some(cands) = self.candidates.get_mut(label) {
                cands.retain(|&(_, block, _)| block != k);
                if cands.is_empty() {
                    self.candidates.remove(label);
                }
            }
        }
        for label in &old.uses {
            if let Some(us) = self.users.get_mut(label) {
                us.remove(&k);
                if us.is_empty() {
                    self.users.remove(label);
                }
            }
        }

        // add new contributions
        let mut order = 0usize;
        for label in &new_deps.defs {
            let url = def_url(new_text, order);
            self.candidates
                .entry(label.clone())
                .or_default()
                .push((k, k, url));
            order += 1;
        }
        for label in &new_deps.uses {
            self.users.entry(label.clone()).or_default().insert(k);
        }
        self.deps.insert(k, new_deps);

        // collect (users × new resolution) over every touched label
        let mut delta = SemanticDelta::default();
        let mut seen = std::collections::BTreeSet::new();
        let mut all_labels: Vec<String> = affected;
        all_labels.extend(user_labels);
        for label in all_labels {
            if !seen.insert(label.clone()) {
                continue;
            }
            delta.labels_touched += 1;
            let new_res = self.resolution(&label);
            if let Some(us) = self.users.get(&label) {
                for &user in us {
                    delta.invalidated.push((user, label.clone(), new_res.clone()));
                }
            }
        }
        delta
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

// --- Correctness fixtures (plan §26) --------------------------------------

pub fn fixtures() -> Result<(), String> {
    fn expect_resolution(doc: &[&str], use_label: &str, want: &str) -> Result<(), String> {
        let blocks: Vec<(usize, String)> =
            doc.iter().enumerate().map(|(i, s)| (i, s.to_string())).collect();
        let idx = ReferenceIndex::build(&blocks);
        let want_s = if want == "<unresolved>" {
            Resolution::Unresolved
        } else {
            Resolution::Resolved(want.into())
        };
        if idx.resolution(use_label) != want_s {
            return Err(format!(
                "fixture failed: [{use_label}] = {:?}, want {want_s:?} in {doc:?}",
                idx.resolution(use_label)
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
    // a def losing line does not match: `[x]: some text` (not a URL-less
    // title form) is still a def per CommonMark only for specific forms;
    // this subset treats any non-bracket remainder as a def URL.
    Ok(())
}
