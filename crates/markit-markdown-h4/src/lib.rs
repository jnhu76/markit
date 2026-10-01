//! Markit H4 Markdown provider (product transplant of the H4
//! restart-convergence mechanism). The stable product seam lives in
//! `markit-markdown-api`; everything in this crate — the grammar
//! substrate, the normalized vocabulary, checkpoints, retained blocks,
//! and reuse identity — is provider-private and must not escape.

mod grammar;
mod impls;
mod plugin;
mod session;

pub use plugin::markdown_plugin;
pub use session::{H4_DIALECT, H4Service};

/// Test-side oracle helpers: the grammar's clean authoritative parse,
/// expressed through the product Heading shape. Production code must not
/// call these; the clean parse exists in tests only (campaign B10).
#[doc(hidden)]
pub mod test_support {
    use crate::grammar::parser::parse_full;
    use crate::impls::normalize::NormalizedDocument;
    use crate::session::headings_from;

    /// The clean authoritative parse of a complete document.
    pub fn clean_parse(src: &str) -> NormalizedDocument {
        parse_full(src.as_bytes())
    }

    /// Headings of `src` according to the clean authoritative parse.
    pub fn headings_of(doc: &NormalizedDocument, src: &str) -> Vec<markit_markdown_api::Heading> {
        headings_from(doc, src)
    }
}
