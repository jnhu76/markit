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
//! bounded        — every stack's depth is bounded by its pre-derived
//!                  LOGICAL limit (a general AVL height bound), not by
//!                  the allocator's capacity, and the guard checks that
//!                  logical limit at every push;
//! pre-staged     — all reservation happens in staging, while the old
//!                  READY document is still owned by the caller; the
//!                  workspace is owned by UpdateStaging and then
//!                  PreparedCommit;
//! fallibly reserved — the reservation uses try_reserve_exact, so an
//!                  ordinary allocator refusal is a pre-frontier
//!                  UpdateError (the old READY state survives it);
//! allocation-free post-frontier — a push within the logical limit
//!                  never allocates (std guarantees Vec::push reallocates
//!                  only at len == capacity, and capacity >= limit), and
//!                  exceeding the logical limit is an invariant abort,
//!                  never a hidden allocation;
//! non-fallible   — no Result, no reserve, no fallback after the frontier.
//! ```
//!
//! Capacity derivation (checked, pre-frontier; `prepare` returns
//! [`WorkspaceError`] so the caller fails before the frontier with the
//! existing error authority): the replace_range splice operates on the
//! old root (height `h_old`), its subtrees, and the fresh replacement
//! tree (height `h_fresh`); the intermediate `join(A, middle)` result
//! can reach `max(h_old, h_fresh) + 1` (a general AVL join property:
//! the join of two AVL trees of heights a and b has height in
//! `[max(a, b), max(a, b) + 1]`). With `Hmax = max(h_old, h_fresh) + 1`,
//! each operator's stack depth is ≤ its operand's height ≤ Hmax, and
//! the frozen #59 §9.1 rows allow `Hmax + 1` — logical limit
//! `max(h_old, h_fresh) + 2` covers every stack with one spare frame.
//! Two splits, two remove_max and two join_with_pivot calls run
//! sequentially (each drains its own stack completely before the next
//! begins), so one stack per operator shape suffices; `split`'s unwind
//! calls `join_with_pivot` while split frames are still live, which is
//! why the join and split stacks are distinct.
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

/// An explicit stack whose depth is guarded by a pre-derived LOGICAL
/// limit, not by the underlying allocator capacity: `Vec::capacity()` is
/// an allocator/runtime fact and is never a proof boundary. The limit is
/// the general AVL height bound derived in [`CommitWorkspace::prepare`];
/// a push that would exceed it aborts on a violated invariant (the bound
/// was mis-derived), never silently allocates past the frontier.
pub(crate) struct BoundedStack<T> {
    frames: Vec<T>,
    limit: usize,
}

// Manual impls: the derived ones would impose `T: Default` bounds the
// stacks do not need (Vec<T> defaults without T).
impl<T> Default for BoundedStack<T> {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            limit: 0,
        }
    }
}

impl<T> std::fmt::Debug for BoundedStack<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundedStack")
            .field("frames", &self.frames.len())
            .field("limit", &self.limit)
            .finish_non_exhaustive()
    }
}

impl<T> BoundedStack<T> {
    /// Push one frame, guarded by the logical limit. The assert is the
    /// mechanically proven capacity bound applied at every push: std
    /// guarantees `Vec::push` reallocates only when `len == capacity`,
    /// so a push that passes this assert cannot allocate; a push that
    /// would exceed the pre-frontier logical bound aborts on a violated
    /// invariant instead of silently allocating past the frontier —
    /// even when the allocator happens to hold spare capacity.
    #[inline]
    pub(crate) fn push_bounded(&mut self, frame: T) {
        assert!(
            self.frames.len() < self.limit,
            "Horse-A workspace logical bound exceeded: push at depth {} would \
             pass the pre-derived limit {} — a capacity-derivation \
             invariant violation, never a post-frontier allocation",
            self.frames.len(),
            self.limit
        );
        self.frames.push(frame);
    }

    /// Pop the most recent frame (the unwind direction).
    #[inline]
    pub(crate) fn pop(&mut self) -> Option<T> {
        self.frames.pop()
    }

    /// Whether the stack is drained (every operator drains its own stack
    /// completely before the next begins).
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// Why the pre-frontier commit-workspace formation refused. Both variants
/// are ordinary pre-frontier resource failures: the caller's old READY
/// state was never consumed and remains usable (#59 §9.1 resource table —
/// resource/allocation failure before `PreparedCommit` is a pre-frontier
/// error).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceError {
    /// The checked logical-limit derivation over `h_old`/`h_fresh`
    /// overflowed.
    CapacityOverflow,
    /// The allocator refused the fallible reservation
    /// (`try_reserve_exact`).
    ReservationRefused,
}

/// The bounded explicit workspace formed during staging (while the old
/// READY document is still owned by the caller) and owned by
/// [`crate::prepared::PreparedCommit`] after formation.
#[derive(Debug, Default)]
pub(crate) struct CommitWorkspace {
    /// The split spine's dissected frames (split descent ≤ operand
    /// height; one frame per spine level).
    pub(crate) split: BoundedStack<SplitFrame>,
    /// The join inner spine's dissected frames (depth t ≤ δ − 1 ≤ operand
    /// height).
    pub(crate) join: BoundedStack<JoinFrame>,
    /// The remove_max right spine's dissected frames (depth ≤ operand
    /// height).
    pub(crate) extract: BoundedStack<ExtractFrame>,
}

impl CommitWorkspace {
    /// Reserve the bounded stacks for a local-route splice over an old
    /// sequence of height `h_old` and a fresh replacement tree of height
    /// `h_fresh`. The logical limit is derived with checked arithmetic
    /// (`CapacityOverflow`), and the reservation itself is fallible
    /// (`try_reserve_exact` → `ReservationRefused`): both refusals are
    /// ordinary pre-frontier errors and must occur BEFORE the frontier,
    /// while the caller still owns the old READY state.
    pub(crate) fn prepare(h_old: u32, h_fresh: u32) -> Result<Self, WorkspaceError> {
        let hmax = h_old.max(h_fresh);
        // Hmax = hmax + 1 (the general AVL join intermediate height); +1
        // spare frame per the frozen "depth <= Hmax + 1" rows.
        let limit = usize::try_from(hmax)
            .ok()
            .and_then(|h| h.checked_add(2))
            .ok_or(WorkspaceError::CapacityOverflow)?;

        // TEST-ONLY deterministic reservation-refusal seam: consumes the
        // one-shot thread-local flag exactly where the fallible
        // reservation runs. Compiled out of every non-test build — this
        // is not a production fault-injection mechanism.
        #[cfg(test)]
        if take_test_reservation_failure() {
            return Err(WorkspaceError::ReservationRefused);
        }

        let mut split: Vec<SplitFrame> = Vec::new();
        let mut join: Vec<JoinFrame> = Vec::new();
        let mut extract: Vec<ExtractFrame> = Vec::new();
        split
            .try_reserve_exact(limit)
            .map_err(|_| WorkspaceError::ReservationRefused)?;
        join.try_reserve_exact(limit)
            .map_err(|_| WorkspaceError::ReservationRefused)?;
        extract
            .try_reserve_exact(limit)
            .map_err(|_| WorkspaceError::ReservationRefused)?;
        Ok(Self {
            split: BoundedStack {
                frames: split,
                limit,
            },
            join: BoundedStack {
                frames: join,
                limit,
            },
            extract: BoundedStack {
                frames: extract,
                limit,
            },
        })
    }

    /// A zero-limit workspace: the full route performs no post-frontier
    /// structural-operator work (no splice — only retirement and state
    /// installation), so nothing is reserved for it. `Vec::new()` does
    /// not allocate; the stacks are never pushed on this route.
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    /// Re-arm the workspace at the splice entry: every operator drains its
    /// own stack completely, so the stacks are empty here in normal
    /// operation; the clear is O(1) and allocation-free defensive hygiene
    /// for reuse across the two splits and two joins of one replace_range.
    pub(crate) fn clear(&mut self) {
        self.split.frames.clear();
        self.join.frames.clear();
        self.extract.frames.clear();
    }
}

/// Arm the TEST-ONLY one-shot reservation failure (see
/// [`reservation_failure_gate`]); test binaries only.
#[cfg(test)]
pub(crate) fn arm_reservation_failure_for_tests() {
    reservation_failure_gate::arm();
}

/// Consume the armed test failure inside `CommitWorkspace::prepare`.
#[cfg(test)]
fn take_test_reservation_failure() -> bool {
    reservation_failure_gate::take()
}

#[cfg(test)]
impl CommitWorkspace {
    /// Unit-test convenience: reserve with a generous height bound for
    /// hand-built fixtures (the logical-limit assert makes an under-sized
    /// bound fail loudly instead of allocating).
    pub(crate) fn for_tests(height_bound: u32) -> Self {
        Self::prepare(height_bound, height_bound).expect("test workspace reservation")
    }
}

#[cfg(test)]
impl<T> BoundedStack<T> {
    /// Test-only: a stack with an explicit logical limit over a
    /// deliberately LARGER underlying reservation, so tests can observe
    /// that the guard is the logical limit, not the allocator capacity
    /// (which may exceed any request).
    pub(crate) fn with_over_reservation_for_tests(limit: usize) -> Self {
        Self {
            frames: Vec::with_capacity(limit + 8),
            limit,
        }
    }

    /// The current number of live frames (test observation).
    pub(crate) fn len(&self) -> usize {
        self.frames.len()
    }

    /// The underlying allocator capacity (test observation only — never
    /// a proof boundary).
    pub(crate) fn allocator_capacity(&self) -> usize {
        self.frames.capacity()
    }
}

/// TEST-ONLY deterministic reservation-refusal seam (#62 post-merge
/// corrective pass): a thread-local one-shot flag that makes the NEXT
/// `CommitWorkspace::prepare` on this thread refuse exactly where the
/// fallible `try_reserve_exact` runs. Thread-local so parallel test
/// threads cannot arm each other's failures; one-shot so a test that
/// arms it fails exactly one staging attempt. Gated behind `#[cfg(test)]`
/// — no production fault-injection mechanism exists.
#[cfg(test)]
mod reservation_failure_gate {
    use std::cell::Cell;

    thread_local! {
        static FAIL_NEXT: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn arm() {
        FAIL_NEXT.with(|f| f.set(true));
    }

    /// Consume the armed failure (one-shot).
    pub(super) fn take() -> bool {
        FAIL_NEXT.with(Cell::take)
    }
}
