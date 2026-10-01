//! The session/service contracts: one session per open document, one
//! service per provider. This is the whole backend-neutral seam —
//! consumer code (Outline, bridge, editor adapters) compiles against
//! these names only.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use markit_document::{DocumentId, SourceEdit, SourceRevision, SourceSnapshot};

use crate::config::MarkdownSemanticConfig;
use crate::view::{PinnedMarkdownRead, SemanticEvent, SemanticStatus};

/// Why a session refused an operation. A refusal never mutates the
/// session's view state, and never reaches back into the document.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SemanticError {
    /// The edit does not continue exactly from the session's last applied
    /// revision (wrong document, wrong base, or a gap). The provider
    /// refuses rather than misapplying coordinates; the caller must
    /// `reset` from a pinned snapshot.
    MismatchedSequence {
        expected_base: SourceRevision,
        edit_base: SourceRevision,
    },
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MismatchedSequence {
                expected_base,
                edit_base,
            } => write!(
                f,
                "edit base {edit_base} does not continue the session's revision \
                 {expected_base}; reset from a snapshot instead"
            ),
        }
    }
}

impl std::error::Error for SemanticError {}

/// One document's Markdown semantic session (architecture §4.2): applies
/// committed source changes or resets to a snapshot, publishes
/// availability/failure notifications, and hands out pinned read views.
/// Shared as an `Rc`; the session is the lifecycle unit a generation
/// names.
pub trait MarkdownSession {
    fn document(&self) -> DocumentId;

    fn config(&self) -> MarkdownSemanticConfig;

    /// The generation this session's views carry (identity/lifecycle
    /// safety: late work from a retired session never matches).
    fn generation(&self) -> u64;

    /// The latest observable availability/failure state.
    fn status(&self) -> SemanticStatus;

    /// (Re)build semantics from a complete pinned source state. The
    /// always-correct path: initial open, or recovery after any
    /// discontinuity.
    fn reset(&self, source: &SourceSnapshot);

    /// Apply one committed edit that continues exactly from the last
    /// applied revision. A stale or gapped sequence is refused explicitly;
    /// there is no silent rebase and no blind application to N+k
    /// (architecture §4.2 rule 6).
    ///
    /// `before`/`after` are the pinned snapshots of the old and new
    /// revision; `edit` is the committed change (its coordinates are in
    /// `before`'s bytes).
    fn apply_edit(
        &self,
        before: &SourceSnapshot,
        after: &SourceSnapshot,
        edit: &SourceEdit,
    ) -> Result<(), SemanticError>;

    /// Acquire the pinned read view for `revision`, if this session
    /// computed one. The returned view stays at that revision forever;
    /// further commits never upgrade it.
    fn pin(&self, revision: SourceRevision) -> Option<Rc<PinnedMarkdownRead>>;

    /// Observe post-fact semantic events (view available / attempt
    /// failed). The subscription ends when the returned guard is dropped
    /// or the session is retired. This is a per-session typed callback,
    /// not a general event bus.
    fn subscribe(&self, listener: Rc<dyn Fn(&SemanticEvent)>) -> Subscription;
}

/// The provider seam: opens sessions for documents. Implementations are
/// the only place a backend brand exists; consumers bind to the
/// capability, never to a backend type.
pub trait MarkdownSemanticService {
    fn open_session(
        &self,
        document: DocumentId,
        config: MarkdownSemanticConfig,
    ) -> Rc<dyn MarkdownSession>;
}

/// A live subscription to one session's events; dropping it unsubscribes.
#[derive(Clone)]
pub struct Subscription {
    remove: Rc<dyn Fn()>,
}

impl Subscription {
    pub(crate) fn new(remove: Rc<dyn Fn()>) -> Self {
        Self { remove }
    }
}

impl std::ops::Drop for Subscription {
    fn drop(&mut self) {
        (self.remove)();
    }
}

/// A small listener set backing [`MarkdownSession::subscribe`]:
/// session-scoped, drop-unsubscribed, iteration order = subscription
/// order. This is notification plumbing for one session, not a general
/// event bus; providers embed one per session.
type ListenerList = Vec<(usize, Rc<dyn Fn(&SemanticEvent)>)>;

#[derive(Default, Clone)]
pub struct ListenerSet {
    listeners: Rc<RefCell<ListenerList>>,
    next_id: Rc<Cell<usize>>,
}

impl ListenerSet {
    pub fn subscribe(&self, listener: Rc<dyn Fn(&SemanticEvent)>) -> Subscription {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.listeners.borrow_mut().push((id, listener));
        let inner = self.listeners.clone();
        Subscription::new(Rc::new(move || {
            inner.borrow_mut().retain(|(i, _)| *i != id);
        }))
    }

    /// Snapshot the listener handles so the set borrow never spans user
    /// code (a listener may subscribe or unsubscribe re-entrantly).
    pub fn broadcast(&self, event: &SemanticEvent) {
        let listeners: Vec<_> = self
            .listeners
            .borrow()
            .iter()
            .map(|(_, l)| l.clone())
            .collect();
        for listener in listeners {
            listener(event);
        }
    }
}
