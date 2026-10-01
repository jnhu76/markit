//! Semantic view identity and the pinned read view (architecture §3,
//! §5.2, ARCH-04): a view is bound to document + revision + semantic
//! configuration + provider generation, and never silently upgrades to
//! latest.

use markit_document::{DocumentId, SourceRange, SourceRevision};

use crate::config::MarkdownSemanticConfig;

/// Identity of one semantic result: which document, which committed
/// source state, under which interpretation configuration, produced by
/// which provider session generation. All four participate in equality —
/// comparing only the revision is insufficient.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SemanticViewId {
    pub document: DocumentId,
    pub revision: SourceRevision,
    pub config: MarkdownSemanticConfig,
    /// The provider session generation that produced (or attempted) the
    /// view; a retired generation's late results are rejected by identity,
    /// never by cancellation.
    pub generation: u64,
}

/// A post-fact notification that something already happened (architecture
/// §5.1). Listeners read the view afterwards; the notification is never
/// the operation itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticEvent {
    /// The semantic view for this identity is available.
    Available(SemanticViewId),
    /// The attempt for this identity failed; the previous view stays
    /// whatever it was. Provider failure never mutates document source.
    Failed {
        attempted: SemanticViewId,
        message: String,
    },
}

/// The latest observable session state: what a `status()` pull answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticStatus {
    /// No semantic view has been computed (or attempted) yet.
    Empty,
    /// The latest computed view.
    Available(SemanticViewId),
    /// The latest attempt failed; nothing newer is available.
    Failed {
        attempted: SemanticViewId,
        message: String,
    },
}

impl From<SemanticEvent> for SemanticStatus {
    fn from(event: SemanticEvent) -> Self {
        match event {
            SemanticEvent::Available(id) => Self::Available(id),
            SemanticEvent::Failed { attempted, message } => Self::Failed { attempted, message },
        }
    }
}

/// One heading of one source revision: outline-grade level, display text,
/// and revision-bound byte location.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Heading {
    /// ATX level, 1..=6.
    pub level: u8,
    /// Display text of the heading content (marker stripped).
    pub text: String,
    /// The heading's byte span in the view's revision.
    pub range: SourceRange,
}

/// A pinned semantic read view: the headings of exactly one revision,
/// under one configuration, produced by one session generation. Immutable
/// by construction — there is no upgrade-to-latest path; clone shares,
/// drop releases.
#[derive(Clone, Debug)]
pub struct PinnedMarkdownRead {
    id: SemanticViewId,
    headings: Vec<Heading>,
}

impl PinnedMarkdownRead {
    /// Providers construct the pinned view from their private semantic
    /// state; this constructor is the only way one comes into existence.
    pub fn new(id: SemanticViewId, headings: Vec<Heading>) -> Self {
        Self { id, headings }
    }

    pub fn id(&self) -> SemanticViewId {
        self.id
    }

    pub fn document_id(&self) -> DocumentId {
        self.id.document
    }

    pub fn revision(&self) -> SourceRevision {
        self.id.revision
    }

    pub fn config(&self) -> MarkdownSemanticConfig {
        self.id.config
    }

    pub fn generation(&self) -> u64 {
        self.id.generation
    }

    /// The headings query, answered from this view only.
    pub fn headings(&self) -> &[Heading] {
        &self.headings
    }
}
