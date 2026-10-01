//! The stable `markdown` capability: the composition identity consumers
//! bind to. The implementing backend (initially H4) is invisible here.

use markit_composition::Capability;

use crate::session::MarkdownSemanticService;

/// The one Markdown semantic capability of the product
/// (`docs/product/architecture.md` §2.2, §7). Service is the
/// backend-neutral provider seam.
pub struct MarkdownCapability;

impl Capability for MarkdownCapability {
    const NAME: &'static str = "MarkdownSemantic";
    type Service = dyn MarkdownSemanticService;
}
