//! The H4-backed implementation of the product Markdown session contract.
//! This is the only module where H4 mechanism state meets the product
//! seam; consumers on the other side see `markit-markdown-api` types
//! only.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use markit_document::{
    DocumentId, SourceEdit, SourceOffset, SourceRange, SourceRevision, SourceSnapshot,
};
use markit_markdown_api::{
    Heading, ListenerSet, MarkdownSemanticConfig, MarkdownSemanticService, MarkdownSession,
    PinnedMarkdownRead, SemanticError, SemanticEvent, SemanticStatus, SemanticViewId, Subscription,
};

use crate::impls::normalize::{Node, NodeKind, NormalizedDocument};
use crate::impls::restart::H4Document;

/// The dialect identity this provider reports and requires in session
/// configuration. Configuration vocabulary, not an internal type.
pub const H4_DIALECT: &str = "bench-grammar-v1";

/// Session generations are process-unique: a value never names two
/// sessions, so a late result from a retired provider binding can never
/// pass a consumer's identity check against a new session's views.
static NEXT_SESSION_GENERATION: AtomicU64 = AtomicU64::new(0);

/// The provider: opens one H4 session per document. The only H4-branded
/// object consumers may hold is the capability service, and even that is
/// reached through the neutral [`MarkdownSemanticService`] face.
#[derive(Default)]
pub struct H4Service {}

impl H4Service {
    pub fn new() -> Self {
        Self {}
    }
}

impl MarkdownSemanticService for H4Service {
    fn open_session(
        &self,
        document: DocumentId,
        config: MarkdownSemanticConfig,
    ) -> Rc<dyn MarkdownSession> {
        assert!(
            config.dialect == H4_DIALECT,
            "the H4 provider interprets dialect {:?} only, got {:?}",
            H4_DIALECT,
            config.dialect
        );
        let generation = NEXT_SESSION_GENERATION.fetch_add(1, Ordering::Relaxed);
        Rc::new(H4Session {
            document,
            config,
            generation,
            inner: RefCell::new(SessionState {
                h4: None,
                last_applied: None,
                latest: SemanticStatus::Empty,
                views: HashMap::new(),
            }),
            listeners: ListenerSet::default(),
        })
    }
}

struct SessionState {
    h4: Option<H4Document>,
    /// The source revision the retained mechanism state reflects.
    last_applied: Option<SourceRevision>,
    latest: SemanticStatus,
    /// Computed views available to `pin`. Cache-only entries (no external
    /// holder) are evicted on the next publication: retention is
    /// "latest + explicitly held", never unbounded.
    views: HashMap<SourceRevision, Rc<PinnedMarkdownRead>>,
}

struct H4Session {
    document: DocumentId,
    config: MarkdownSemanticConfig,
    generation: u64,
    inner: RefCell<SessionState>,
    listeners: ListenerSet,
}

impl H4Session {
    fn view_id(&self, revision: SourceRevision) -> SemanticViewId {
        SemanticViewId {
            document: self.document,
            revision,
            config: self.config,
            generation: self.generation,
        }
    }
}

impl MarkdownSession for H4Session {
    fn document(&self) -> DocumentId {
        self.document
    }

    fn config(&self) -> MarkdownSemanticConfig {
        self.config
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn status(&self) -> SemanticStatus {
        self.inner.borrow().latest.clone()
    }

    fn reset(&self, source: &SourceSnapshot) {
        assert!(
            source.document_id() == self.document,
            "reset with a snapshot of another document"
        );
        let view = {
            let mut state = self.inner.borrow_mut();
            let h4 = H4Document::parse(source.source().as_bytes());
            let projected = h4.project();
            let headings = headings_from(&projected, source.source());
            let id = self.view_id(source.revision());
            state.h4 = Some(h4);
            state.last_applied = Some(source.revision());
            let view = Rc::new(PinnedMarkdownRead::new(id, headings));
            publish(&mut state, view.clone());
            view
        };
        self.listeners
            .broadcast(&SemanticEvent::Available(view.id()));
    }

    fn apply_edit(
        &self,
        before: &SourceSnapshot,
        after: &SourceSnapshot,
        edit: &SourceEdit,
    ) -> Result<(), SemanticError> {
        assert!(
            before.document_id() == self.document && after.document_id() == self.document,
            "apply_edit with snapshots of another document"
        );
        let expected_base = self.inner.borrow().last_applied;
        let expected_base = expected_base.ok_or(SemanticError::MismatchedSequence {
            expected_base: SourceRevision::INITIAL,
            edit_base: edit.base(),
        })?;
        let range = edit.range();
        let (es, ee) = (range.start.get(), range.end.get());
        // The edit must be exactly the next commit of the session's state:
        // same document, right base, and `after` is that one commit's
        // result with byte length following from the replacement. A gapped
        // or incoherent sequence is refused; `reset` is the recovery path.
        if edit.document() != self.document
            || edit.base() != expected_base
            || edit.base() != before.revision()
            || after.revision().get() != before.revision().get() + 1
            || ee > before.source().len()
            || es > ee
            || after.source().len() != before.source().len() - (ee - es) + edit.replacement().len()
        {
            return Err(SemanticError::MismatchedSequence {
                expected_base,
                edit_base: edit.base(),
            });
        }

        let view = {
            let mut state = self.inner.borrow_mut();
            let h4 = state.h4.as_mut().expect("sequence check passed");
            h4.update(
                before.source().as_bytes(),
                after.source().as_bytes(),
                es,
                ee,
                edit.replacement().len(),
            );
            let projected = h4.project();
            let headings = headings_from(&projected, after.source());
            let id = self.view_id(after.revision());
            state.last_applied = Some(after.revision());
            let view = Rc::new(PinnedMarkdownRead::new(id, headings));
            publish(&mut state, view.clone());
            view
        };
        self.listeners
            .broadcast(&SemanticEvent::Available(view.id()));
        Ok(())
    }

    fn pin(&self, revision: SourceRevision) -> Option<Rc<PinnedMarkdownRead>> {
        self.inner.borrow().views.get(&revision).cloned()
    }

    fn subscribe(&self, listener: Rc<dyn Fn(&SemanticEvent)>) -> Subscription {
        self.listeners.subscribe(listener)
    }
}

/// Publish a newly computed view: cache it (evicting cache-only older
/// entries), make it the latest status.
fn publish(state: &mut SessionState, view: Rc<PinnedMarkdownRead>) {
    state.views.retain(|_, held| Rc::strong_count(held) > 1);
    state.views.insert(view.revision(), view.clone());
    state.latest = SemanticStatus::Available(view.id());
}

/// Walk the projected semantic tree in document order and collect every
/// heading: ATX level, display text, and the revision-bound byte range.
/// The display text is the heading's raw content bytes (the interval the
/// grammar scanned inlines over, recovered from the first/last inline
/// child) — inline syntax stays visible, exactly as the source has it.
pub(crate) fn headings_from(doc: &NormalizedDocument, src: &str) -> Vec<Heading> {
    fn walk(node: &Node, src: &str, out: &mut Vec<Heading>) {
        if node.kind == NodeKind::Heading {
            let text = match (node.children.first(), node.children.last()) {
                (Some(first), Some(last)) => src[first.start..last.end].to_owned(),
                _ => String::new(),
            };
            out.push(Heading {
                level: node.level.unwrap_or(0),
                text,
                range: SourceRange::new(SourceOffset::new(node.start), SourceOffset::new(node.end)),
            });
        }
        for child in &node.children {
            walk(child, src, out);
        }
    }
    let mut out = Vec::new();
    walk(&doc.root, src, &mut out);
    out
}
