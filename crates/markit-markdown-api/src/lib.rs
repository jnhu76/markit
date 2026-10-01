//! Markit Markdown semantic contract: the stable, backend-neutral
//! product vocabulary for Markdown interpretation
//! (`docs/product/architecture.md` §2.2, §5). One interpretation
//! contract; providers implement it; consumers never see provider
//! internals.
//!
//! Publication model (frozen): push = post-fact availability/failure
//! notifications through per-session typed callbacks; pull = pinned,
//! revision-bound read views queried through typed methods. There is no
//! general event bus here — subscriptions live and die with one session.

mod capability;
mod config;
mod outline;
mod session;
mod view;

pub use capability::MarkdownCapability;
pub use config::MarkdownSemanticConfig;
pub use outline::{OutlineEntry, OutlineProjection};
pub use session::{
    ListenerSet, MarkdownSemanticService, MarkdownSession, SemanticError, Subscription,
};
pub use view::{Heading, PinnedMarkdownRead, SemanticEvent, SemanticStatus, SemanticViewId};
