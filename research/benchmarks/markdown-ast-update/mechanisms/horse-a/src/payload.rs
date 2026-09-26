//! Owner payload construction: eager materialization of one top-level
//! block under the FINAL document RefTable, then conversion of every
//! retained coordinate to Owner-relative form.
//!
//! The shared semantic engine is reused, not forked (task #39):
//! `materialize_one_with_sink` produces the NORMALIZED-RESULT-v1 subtree
//! with absolute document spans; the single transformation below shifts
//! every span-bearing coordinate it carries. A `Node`'s span-bearing
//! fields are exactly `start`, `end`, and `FencedCode.content` — audited
//! total over the shared semantic model (task #15/#40): block spans,
//! nested block spans, inline spans, link/image spans, definition spans,
//! list/quote descendants, and `FencedCode.content`.

use markit_mdbench_oracle::normalized::Node;

/// Shift every retained coordinate of the subtree by `delta`. Rebase to
/// Owner-relative form with `-(base as isize)`; restore document-absolute
/// form at export with `+(base as isize)`.
pub(crate) fn shift_spans(node: &mut Node, delta: isize) {
    node.start = (node.start as isize + delta) as usize;
    node.end = (node.end as isize + delta) as usize;
    if let Some((a, b)) = node.content.as_mut() {
        *a = (*a as isize + delta) as usize;
        *b = (*b as isize + delta) as usize;
    }
    for child in &mut node.children {
        shift_spans(child, delta);
    }
}
