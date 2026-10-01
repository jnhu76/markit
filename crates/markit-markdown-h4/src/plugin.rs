//! The stable `markdown` composition role, initially satisfied by the H4
//! backend. Composition identity is the role, never the brand
//! (`docs/product/foundation-rules.md`).

use std::rc::Rc;

use markit_composition::ComponentSpec;
use markit_markdown_api::MarkdownCapability;

use crate::session::H4Service;

/// The Markdown semantic provider plugin. Consumers resolve
/// [`MarkdownCapability`]; swapping the backend must not change this
/// composition identity.
pub fn markdown_plugin() -> ComponentSpec {
    ComponentSpec::new("markdown")
        .provides::<MarkdownCapability>()
        .on_activate(|ctx| {
            ctx.provide::<MarkdownCapability>(Rc::new(H4Service::new()))
                .expect("provides declared");
            Ok(())
        })
}
