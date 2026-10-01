//! The Markdown semantic configuration identity: part of every semantic
//! view identity, so a view is never mistaken across configurations
//! (architecture §3, §5.2).

/// The Markdown interpretation configuration a session was opened with.
/// The dialect identity is provider-reported vocabulary; comparing
/// configurations compares these identities. Extension flags join here
/// when a real consumer earns them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MarkdownSemanticConfig {
    pub dialect: &'static str,
}

impl MarkdownSemanticConfig {
    /// The configuration identity carried by every view of one session.
    pub fn new(dialect: &'static str) -> Self {
        Self { dialect }
    }
}
