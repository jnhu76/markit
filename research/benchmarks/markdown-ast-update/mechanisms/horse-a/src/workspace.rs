//! The pre-staged bounded explicit commit workspace (#59 §9.1 resource
//! table; ACCOUNTING-CORRECTION-1 §9 resolution A): the post-frontier
//! structural operators — `split`, `remove_max`, `join_with_pivot` — run
//! on explicit descent/unwind stacks whose capacity is fully reserved
//! BEFORE the [`crate::prepared::PreparedCommit`] frontier, instead of
//! the by-value Rust recursion that was the recorded realization debt.
//!
//! Resource properties (the normative part; the private representation is
//! implementation freedom):
//!
//! ```text
//! bounded        — every stack's depth is bounded by the operand tree
//!                  height + 1, and the capacity is derived from that
//!                  bound before the frontier;
//! pre-staged     — all reservation happens in prepare()/staging; the
//!                  workspace is owned by PreparedCommit;
//! allocation-free post-frontier — a push within the reserved capacity
//!                  never allocates (std guarantees Vec::push reallocates
//!                  only at len == capacity), and exceeding the capacity
//!                  is an invariant abort, never a hidden allocation;
//! non-fallible   — no Result, no reserve, no fallback after the frontier.
//! ```
//!
//! Capacity derivation (checked, pre-frontier; `prepare` returns `None`
//! on overflow so the caller fails before the frontier with the existing
//! error authority): the replace_range splice operates on the old root
//! (height `h_old`), its subtrees, and the fresh replacement tree (height
//! `h_fresh`); the intermediate `join(A, middle)` result can reach
//! `max(h_old, h_fresh) + 1` (spec §6.3 witness instance bounds). With
//! `Hmax = max(h_old, h_fresh) + 1`, each operator's stack depth is ≤
//! its operand's height ≤ Hmax, and the frozen #59 §9.1 rows allow
//! `Hmax + 1` — capacity `max(h_old, h_fresh) + 2` covers every stack
//! with one spare frame. Two splits, two remove_max and two
//! join_with_pivot calls run sequentially (each drains its own stack
//! completely before the next begins), so one stack per operator shape
//! suffices; `split`'s unwind calls `join_with_pivot` while split frames
//! are still live, which is why the join and split stacks are distinct.
//!
//! The explicit stacks are a resource mechanism, not a structural-work
//! metric: pushes/pops are never charged to any counter (the schema has
//! no workspace-operation unit and `retirement_workspace_ops` was
//! withdrawn); the iterative realizations charge exactly the same logical
//! work units as the frozen recursive algorithms.

use crate::state::AvlNode;

/// One dissected split-spine frame awaiting reconstruction. `node` has
/// both child slots already taken; `side` records which child the
/// descent continued into and keeps the other original subtree for the
/// reconstruction join.
#[derive(Debug)]
pub(crate) struct SplitFrame {
    pub(crate) node: Box<AvlNode>,
    pub(crate) side: SplitSide,
}

/// Which child the split descent continued into, and the other original
/// subtree held for the reconstruction join.
#[derive(Debug)]
pub(crate) enum SplitSide {
    /// Descent continued into the LEFT subtree (k < left_records): the
    /// original right subtree waits here for
    /// `join_with_pivot(b, node, right)`.
    Left { right: Option<Box<AvlNode>> },
    /// Descent continued into the RIGHT subtree (k > left_records + 1):
    /// the original left subtree waits here for
    /// `join_with_pivot(left, node, c)`.
    Right { left: Option<Box<AvlNode>> },
}

/// One dissected join-spine frame: an inner-spine node whose inner child
/// slot is already taken; the unwind reinstalls the joined result into
/// that same slot (the §15.2 same-frame pair).
#[derive(Debug)]
pub(crate) struct JoinFrame {
    pub(crate) node: Box<AvlNode>,
}

/// One dissected right-spine frame of `remove_max`: a node whose right
/// slot is already taken; the unwind reinstalls the remaining subtree
/// into that same slot.
#[derive(Debug)]
pub(crate) struct ExtractFrame {
    pub(crate) node: Box<AvlNode>,
}

/// The bounded explicit workspace owned by [`crate::prepared::PreparedCommit`].
#[derive(Debug, Default)]
pub(crate) struct CommitWorkspace {
    /// The split spine's dissected frames (split descent ≤ operand
    /// height; one frame per spine level).
    pub(crate) split: Vec<SplitFrame>,
    /// The join inner spine's dissected frames (depth t ≤ δ − 1 ≤ operand
    /// height).
    pub(crate) join: Vec<JoinFrame>,
    /// The remove_max right spine's dissected frames (depth ≤ operand
    /// height).
    pub(crate) extract: Vec<ExtractFrame>,
}

impl CommitWorkspace {
    /// Reserve the bounded stacks for a local-route splice over an old
    /// sequence of height `h_old` and a fresh replacement tree of height
    /// `h_fresh` (checked arithmetic; `None` = capacity derivation
    /// overflowed, the caller must fail BEFORE the frontier).
    pub(crate) fn prepare(h_old: u32, h_fresh: u32) -> Option<Self> {
        let hmax = h_old.max(h_fresh);
        // Hmax = hmax + 1 (intermediate join height); +1 spare frame per
        // the frozen "depth <= Hmax + 1" rows.
        let cap = usize::try_from(hmax).ok()?.checked_add(2)?;
        Some(Self {
            split: Vec::with_capacity(cap),
            join: Vec::with_capacity(cap),
            extract: Vec::with_capacity(cap),
        })
    }

    /// A zero-capacity workspace: the full route performs no post-frontier
    /// structural-operator work (no splice — only retirement and state
    /// installation), so nothing is reserved for it. `Vec::new()` does not
    /// allocate; the stacks are never pushed on this route.
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    /// Re-arm the workspace at the splice entry: every operator drains its
    /// own stack completely, so the stacks are empty here in normal
    /// operation; the clear is O(1) and allocation-free defensive hygiene
    /// for reuse across the two splits and two joins of one replace_range.
    pub(crate) fn clear(&mut self) {
        self.split.clear();
        self.join.clear();
        self.extract.clear();
    }
}

/// The mechanically proven capacity bound, applied at every push: std
/// guarantees `Vec::push` reallocates only when `len == capacity`, so a
/// push that passes this assert cannot allocate; a push that would
/// exceed the pre-frontier reservation aborts on a violated invariant
/// instead of silently allocating past the frontier.
#[inline]
pub(crate) fn push_bounded<T>(stack: &mut Vec<T>, frame: T) {
    assert!(
        stack.len() < stack.capacity(),
        "commit workspace stack exceeded its pre-frontier capacity — \
         an invariant violation, never a post-frontier allocation"
    );
    stack.push(frame);
}

#[cfg(test)]
impl CommitWorkspace {
    /// Unit-test convenience: reserve with a generous height bound for
    /// hand-built fixtures (the capacity assert makes an under-sized
    /// bound fail loudly instead of allocating).
    pub(crate) fn for_tests(height_bound: u32) -> Self {
        Self::prepare(height_bound, height_bound).expect("test workspace reservation")
    }
}
