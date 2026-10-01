//! The `document` composition role. Document authority is an independent
//! product capability/lifecycle, so it participates in K0 composition like
//! any other plugin. The store/factory service is composition-owned; the
//! concrete documents it hands out are domain objects, not fibers, and
//! their edit/read path is the already-bound typed face.

use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use std::rc::Rc;

use markit_composition::{ComponentSpec, Capability};

use crate::document::Document;
use crate::identity::DocumentId;

/// The stable `document` role's capability contract: create, reach, and
/// retire open documents. Composition identity is the role, not any
/// storage brand.
pub struct DocumentStoreCapability;

impl Capability for DocumentStoreCapability {
    const NAME: &'static str = "DocumentStore";
    type Service = DocumentStore;
}

/// The document factory/registry service: owns the open documents of one
/// composition. Shared through K0 as an `Rc`, so the map is behind a
/// `RefCell`; keying is by [`DocumentId`], so consumers never guess.
#[derive(Default)]
pub struct DocumentStore {
    documents: RefCell<BTreeMap<DocumentId, Document>>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a document holding `initial` content. The document authority
    /// stays owned here; callers work on it through the id.
    pub fn create(&self, initial: impl Into<String>) -> DocumentId {
        let document = Document::new(initial);
        let id = document.id();
        self.documents.borrow_mut().insert(id, document);
        id
    }

    pub fn get(&self, id: DocumentId) -> Option<Ref<'_, Document>> {
        Ref::filter_map(self.documents.borrow(), |documents| documents.get(&id)).ok()
    }

    pub fn get_mut(&self, id: DocumentId) -> Option<RefMut<'_, Document>> {
        RefMut::filter_map(self.documents.borrow_mut(), |documents| documents.get_mut(&id)).ok()
    }

    /// Close (retire) a document. Explicitly retained snapshots handed out
    /// earlier stay readable — release is dropping the pin, not closing
    /// the document.
    pub fn close(&self, id: DocumentId) -> Option<Document> {
        self.documents.borrow_mut().remove(&id)
    }
}

/// The stable `document` role (foundation-rules constructor convention).
pub fn document_plugin() -> ComponentSpec {
    ComponentSpec::new("document")
        .provides::<DocumentStoreCapability>()
        .on_activate(|ctx| {
            ctx.provide::<DocumentStoreCapability>(Rc::new(DocumentStore::new()))
                .expect("provides declared");
            Ok(())
        })
}
