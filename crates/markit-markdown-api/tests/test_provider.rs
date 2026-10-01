//! Contract-level acceptance tests over a TEST PROVIDER — a tiny
//! line-scanner that is not H4. Its only purpose: prove Document/Outline
//! consumers compile and behave against the product contract alone, and
//! that failure/stale-completion semantics are contract properties, not
//! H4 properties.

use std::cell::RefCell;
use std::rc::Rc;

use markit_document::{Document, SourceEdit, SourceOffset, SourceRange, SourceSnapshot};
use markit_markdown_api::{
    Heading, ListenerSet, MarkdownSemanticConfig, MarkdownSemanticService, MarkdownSession,
    OutlineProjection, PinnedMarkdownRead, SemanticError, SemanticEvent, SemanticStatus,
    SemanticViewId, Subscription,
};

/// A deliberately naive provider: headings are lines starting with `#`;
/// every update recomputes from the snapshot. A source line containing
/// `FAIL:` makes the attempt fail — failure injection for the contract.
#[derive(Default)]
pub struct TestService {
    next_generation: std::cell::Cell<u64>,
}

pub const TEST_DIALECT: &str = "test-line-headings-v1";

struct TestSession {
    document: markit_document::DocumentId,
    generation: u64,
    state: RefCell<TestState>,
    listeners: ListenerSet,
}

enum TestState {
    Empty,
    Good(Rc<PinnedMarkdownRead>),
    Failed {
        attempted: SemanticViewId,
        message: String,
    },
}

impl TestService {
    fn open(&self, document: markit_document::DocumentId) -> Rc<dyn MarkdownSession> {
        let generation = self.next_generation.get();
        self.next_generation.set(generation + 1);
        Rc::new(TestSession {
            document,
            generation,
            state: RefCell::new(TestState::Empty),
            listeners: ListenerSet::default(),
        })
    }
}

impl MarkdownSemanticService for TestService {
    fn open_session(
        &self,
        document: markit_document::DocumentId,
        _config: MarkdownSemanticConfig,
    ) -> Rc<dyn MarkdownSession> {
        self.open(document)
    }
}

fn scan(source: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut at = 0usize;
    for line in source.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix('#') {
            let level = rest.chars().take_while(|&c| c == '#').count() as u8;
            let text = rest
                .trim_start_matches('#')
                .trim_start()
                .trim_end()
                .to_owned();
            out.push(Heading {
                level,
                text,
                range: SourceRange::new(SourceOffset::new(at), SourceOffset::new(at + line.len())),
            });
        }
        at += line.len();
    }
    out
}

impl TestSession {
    fn view_id(&self, revision: markit_document::SourceRevision) -> SemanticViewId {
        SemanticViewId {
            document: self.document,
            revision,
            config: MarkdownSemanticConfig::new(TEST_DIALECT),
            generation: self.generation,
        }
    }

    fn recompute(&self, source: &SourceSnapshot) {
        let id = self.view_id(source.revision());
        if source.source().contains("FAIL:") {
            let event = SemanticEvent::Failed {
                attempted: id,
                message: "injected failure".to_owned(),
            };
            *self.state.borrow_mut() = match event.clone() {
                SemanticEvent::Failed { attempted, message } => {
                    TestState::Failed { attempted, message }
                }
                _ => unreachable!(),
            };
            self.listeners.broadcast(&event);
            return;
        }
        let view = Rc::new(PinnedMarkdownRead::new(id, scan(source.source())));
        *self.state.borrow_mut() = TestState::Good(view.clone());
        self.listeners
            .broadcast(&SemanticEvent::Available(view.id()));
    }
}

impl MarkdownSession for TestSession {
    fn document(&self) -> markit_document::DocumentId {
        self.document
    }

    fn config(&self) -> MarkdownSemanticConfig {
        MarkdownSemanticConfig::new(TEST_DIALECT)
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn status(&self) -> SemanticStatus {
        match &*self.state.borrow() {
            TestState::Empty => SemanticStatus::Empty,
            TestState::Good(view) => SemanticStatus::Available(view.id()),
            TestState::Failed { attempted, message } => SemanticStatus::Failed {
                attempted: *attempted,
                message: message.clone(),
            },
        }
    }

    fn reset(&self, source: &SourceSnapshot) {
        self.recompute(source);
    }

    fn apply_edit(
        &self,
        _before: &SourceSnapshot,
        after: &SourceSnapshot,
        _edit: &SourceEdit,
    ) -> Result<(), SemanticError> {
        self.recompute(after);
        Ok(())
    }

    fn pin(&self, revision: markit_document::SourceRevision) -> Option<Rc<PinnedMarkdownRead>> {
        let current = match &*self.state.borrow() {
            TestState::Good(view) => Some(view.clone()),
            _ => None,
        };
        current.filter(|view| view.revision() == revision)
    }

    fn subscribe(&self, listener: Rc<dyn Fn(&SemanticEvent)>) -> Subscription {
        self.listeners.subscribe(listener)
    }
}

fn commit(
    doc: &mut Document,
    range: SourceRange,
    replacement: &str,
) -> markit_document::SourceRevision {
    let base = doc.revision();
    doc.commit(SourceEdit::replace(
        doc.id(),
        base,
        range,
        replacement.to_owned(),
    ))
    .expect("valid edit")
}

fn whole_range(doc: &Document) -> SourceRange {
    SourceRange::new(SourceOffset::new(0), SourceOffset::new(doc.source().len()))
}

/// A Document + Outline consumer drives the TEST provider end to end and
/// never names H4 (this test binary does not depend on the H4 crate).
#[test]
fn document_and_outline_run_over_a_non_h4_provider() {
    let service = TestService::default();
    let mut doc = Document::new("# Hello\n\ntext\n");
    let session = service.open_session(doc.id(), MarkdownSemanticConfig::new(TEST_DIALECT));

    let before = doc.snapshot();
    session.reset(&before);
    let outline = OutlineProjection::derive(&session.pin(before.revision()).expect("view"));
    assert_eq!(outline.entries.len(), 1);
    assert_eq!(outline.entries[0].text, "Hello");

    // edit: replace the heading line
    let before = doc.snapshot();
    let base = doc.revision();
    let edit = SourceEdit::replace(
        doc.id(),
        base,
        whole_range(&doc),
        "# Hello\n\n## 世界 🌍\n".to_owned(),
    );
    let rev = doc.commit(edit.clone()).expect("commit");
    let after = doc.snapshot();
    session
        .apply_edit(&before, &after, &edit)
        .expect("sequence");
    let pinned = session.pin(rev).expect("view for the new revision");
    let outline = OutlineProjection::derive(&pinned);
    assert_eq!(outline.revision, rev);
    let texts: Vec<_> = outline.entries.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, ["Hello", "世界 🌍"]);
    assert_eq!(
        doc.source(),
        "# Hello\n\n## 世界 🌍\n",
        "provider never mutates source"
    );
}

/// A failed attempt is published as failure, the document stays intact and
/// editable, and no view is offered for the failed revision.
#[test]
fn provider_failure_leaves_source_editable_and_reports_failure() {
    let service = TestService::default();
    let mut doc = Document::new("# Fine\n");
    let session = service.open_session(doc.id(), MarkdownSemanticConfig::new(TEST_DIALECT));
    session.reset(&doc.snapshot());

    let events: Rc<RefCell<Vec<SemanticEvent>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let _sub = session.subscribe(Rc::new(move |e| sink.borrow_mut().push(e.clone())));

    let before = doc.snapshot();
    let base = doc.revision();
    let edit = SourceEdit::replace(doc.id(), base, whole_range(&doc), "FAIL: boom\n".to_owned());
    let rev = doc.commit(edit.clone()).expect("commit");
    let after = doc.snapshot();
    session
        .apply_edit(&before, &after, &edit)
        .expect("sequence accepted");

    match session.status() {
        SemanticStatus::Failed { attempted, message } => {
            assert_eq!(attempted.revision, rev);
            assert_eq!(attempted.generation, session.generation());
            assert!(message.contains("injected"));
        }
        other => panic!("expected failure status, got {other:?}"),
    }
    assert_eq!(doc.source(), "FAIL: boom\n", "failure never mutates source");

    // still editable after failure
    let _ = commit(
        &mut doc,
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(5)),
        "# Ok",
    );
    assert!(
        doc.source().starts_with("# Ok"),
        "document remains editable"
    );
    assert!(
        matches!(events.borrow().last(), Some(SemanticEvent::Failed { .. })),
        "the failure was announced"
    );
}

/// A late event from a retired session generation cannot pass the
/// consumer's target check — freshness is identity-based, never
/// cancellation-based.
#[test]
fn stale_generation_view_cannot_be_accepted_as_current() {
    let service = TestService::default();
    let doc = Document::new("# A\n");
    let old_session = service.open_session(doc.id(), MarkdownSemanticConfig::new(TEST_DIALECT));
    old_session.reset(&doc.snapshot());

    let doc2 = Document::new("# A\n");
    let new_session = service.open_session(doc2.id(), MarkdownSemanticConfig::new(TEST_DIALECT));
    new_session.reset(&doc2.snapshot());

    // A captured "late" event from the retired session...
    let late: Rc<RefCell<Option<SemanticEvent>>> = Rc::new(RefCell::new(None));
    let sink = late.clone();
    let _sub = old_session.subscribe(Rc::new(move |e| *sink.borrow_mut() = Some(e.clone())));
    old_session.reset(&doc.snapshot());
    let late = late.borrow().clone().expect("captured"); // delivered late, after `new_session` took over

    // ...is checked against the interactive target before acceptance.
    let target_is_current = |event: &SemanticEvent, session: &dyn MarkdownSession| match event {
        SemanticEvent::Available(id) => {
            id.generation == session.generation() && id.document == session.document()
        }
        SemanticEvent::Failed { .. } => false,
    };
    assert!(matches!(
        late,
        SemanticEvent::Available(ref id) if id.generation == old_session.generation()
    ));
    assert!(
        !target_is_current(&late, new_session.as_ref()),
        "a retired generation's view must not become current"
    );
}

/// Subscriptions end when the guard drops: no further events arrive.
#[test]
fn subscription_ends_when_the_guard_drops() {
    let service = TestService::default();
    let doc = Document::new("# A\n");
    let session = service.open_session(doc.id(), MarkdownSemanticConfig::new(TEST_DIALECT));

    let count = Rc::new(std::cell::Cell::new(0u32));
    let counter = count.clone();
    let sub = session.subscribe(Rc::new(move |_| counter.set(counter.get() + 1)));
    session.reset(&doc.snapshot());
    assert_eq!(count.get(), 1);
    drop(sub);
    session.reset(&doc.snapshot());
    assert_eq!(count.get(), 1, "no events after unsubscribe");
}
