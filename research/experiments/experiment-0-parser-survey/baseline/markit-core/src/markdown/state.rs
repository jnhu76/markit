//! Parser state at block boundaries — the restart/convergence vocabulary.
//!
//! `docs/product/markdown-l1-semantic-contract.md` §9: a block boundary is
//! a **safe restart boundary** exactly when the parser state there is
//! [`BlockParseState::Ground`]; parsing any suffix from a safe boundary is
//! deterministic and independent of everything before it. Because L1 is
//! flat (no container recursion), an open fence is the only state that
//! can span lines, so the state enum has two variants.

use std::fmt;

/// Which character a code fence is built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FenceChar {
    /// A ``` ``` ``` fence.
    Backtick,
    /// A `~~~` fence.
    Tilde,
}

impl FenceChar {
    /// The fence's character.
    pub fn as_char(self) -> char {
        match self {
            Self::Backtick => '`',
            Self::Tilde => '~',
        }
    }
}

/// Parser state at a block boundary.
///
/// Stored per record (`state_before`/`state_after`) so the state at any
/// boundary is queryable without reparsing. In a complete parse of the
/// flat L1 stream every record starts in `Ground`; a document that ends
/// inside an unclosed fence has the fence record's `state_after` set to
/// [`BlockParseState::InFence`] — that is the documented honest
/// propagation to end of document (contract §6.7, issue #12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockParseState {
    /// Between blocks: the next line is classified by its own shape.
    Ground,
    /// Inside an open fenced code block, waiting for a closing fence of
    /// `fence_char` with length >= `fence_len` (or end of document).
    InFence {
        /// The opening fence's character.
        fence_char: FenceChar,
        /// The opening fence's length (minimum closing length).
        fence_len: usize,
    },
}

impl BlockParseState {
    /// Whether this is a safe restart boundary (contract §9.1).
    pub fn is_ground(&self) -> bool {
        matches!(self, Self::Ground)
    }
}

impl fmt::Display for BlockParseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ground => write!(f, "Ground"),
            Self::InFence {
                fence_char,
                fence_len,
            } => {
                write!(f, "InFence({}, {})", fence_char.as_char(), fence_len)
            }
        }
    }
}
