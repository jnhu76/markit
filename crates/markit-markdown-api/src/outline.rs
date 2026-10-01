//! The Outline projection (architecture §9): a coarse, deterministic,
//! revision-bound derivation over a pinned semantic read view. It owns no
//! semantic authority — the same pinned view always derives the same
//! outline, a failed provider never produces a "current" outline, and a
//! new revision earns a fresh projection.

use markit_document::{SourceRange, SourceRevision};

use crate::view::PinnedMarkdownRead;

/// One outline row: the view's heading, unchanged.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OutlineEntry {
    pub level: u8,
    pub text: String,
    pub range: SourceRange,
}

/// The outline of exactly one source revision. Constructing one requires
/// a pinned view; there is no path to derive an outline "from latest".
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OutlineProjection {
    pub revision: SourceRevision,
    pub entries: Vec<OutlineEntry>,
}

impl OutlineProjection {
    /// Coarse full refresh over the whole pinned view (architecture
    /// §5.3): legal and correct; incremental refinement is earned later.
    pub fn derive(view: &PinnedMarkdownRead) -> Self {
        Self {
            revision: view.revision(),
            entries: view
                .headings()
                .iter()
                .map(|h| OutlineEntry {
                    level: h.level,
                    text: h.text.clone(),
                    range: h.range,
                })
                .collect(),
        }
    }
}
