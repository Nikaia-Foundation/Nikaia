//! **The id every `Expr` node carries** ([ADR-340](../../../docs/specification/adr/adr-340.md),
//! #558): what the compiler's tables are keyed on where Rust once keyed them on
//! an expression's address.
//!
//! A `u32` the parser gives when it makes the node (D1), below
//! [`FIRST_BUILT`]; a node the compiler builds after parsing takes one from
//! [`NodeId::fresh`], from [`FIRST_BUILT`] up, so no two nodes of any tree
//! share one (D2). A clone keeps its id.
//!
//! **Two ids are always equal** (D3): the id is identity, not value, so two
//! trees that differ only in their ids compare equal and `Expr` keeps its
//! derived `PartialEq`. A table is keyed on [`NodeId::get`], the number, never
//! on the `NodeId`, whose `==` says nothing about which node it is.
//!
//! **A parsed id carries its span until it is numbered**
//! ([ADR-383](../../../docs/specification/adr/adr-383.md) D1, D4): the grammar
//! gives [`NodeId::parsed`], a rule that knows where the node was read puts the
//! span on it with [`NodeId::at`] (the innermost rule first), and the numbering
//! pass reads it with [`NodeId::span`] into the file's span table and gives the
//! number with [`NodeId::numbered`], which carries no span.

use std::sync::atomic::{AtomicU32, Ordering};

/// The first id a node the compiler builds takes. The parser gives the ids
/// below it, file by file.
pub const FIRST_BUILT: u32 = 1 << 31;

/// The number of a parsed id the numbering pass has not numbered yet
/// ([ADR-383](../../../docs/specification/adr/adr-383.md) D1): below
/// [`FIRST_BUILT`], and shared by every such node, so no table is keyed on it
/// before the pass (D2).
pub const PARSED: u32 = FIRST_BUILT - 1;

/// The `start` of an id that carries no span: after any `end`, so no span a
/// rule records has it.
const NO_SPAN: u32 = u32::MAX;

static BUILT: AtomicU32 = AtomicU32::new(FIRST_BUILT);

/// One expression node's id.
///
/// Its span (two `u32`, `start` `NO_SPAN` where it has none) is where the
/// node was read, until the numbering pass moves it into the span table
/// (ADR-383 D4, D5).
#[derive(Clone, Copy)]
pub struct NodeId {
    number: u32,
    start: u32,
    end: u32,
}

impl NodeId {
    /// The id with this number, and no span: what the parser gives, in the
    /// order it makes its nodes.
    pub fn numbered(number: u32) -> NodeId {
        NodeId {
            number,
            start: NO_SPAN,
            end: 0,
        }
    }

    /// The id of a node the grammar makes, not yet numbered and with no span
    /// (ADR-383 D1).
    pub fn parsed() -> NodeId {
        NodeId::numbered(PARSED)
    }

    /// An id no other node has, for a node made after parsing.
    pub fn fresh() -> NodeId {
        NodeId::numbered(BUILT.fetch_add(1, Ordering::Relaxed))
    }

    /// The number a table is keyed on.
    pub fn get(self) -> u32 {
        self.number
    }

    /// Whether the numbering pass has still to number it (ADR-383 D3).
    pub fn is_parsed(self) -> bool {
        self.number == PARSED
    }

    /// This id with the node read from `span`, unless it already carries one:
    /// the innermost rule that finished the node gives the span (ADR-383 D4),
    /// as `Trivia::at` keeps it.
    ///
    /// Two `u32`, not the tree's `Span`: this module is part of every
    /// program's `std`, which has no `tools`.
    pub fn at(self, start: u32, end: u32) -> NodeId {
        if self.start != NO_SPAN {
            return self;
        }
        NodeId {
            number: self.number,
            start,
            end,
        }
    }

    /// Where the node was read, if a rule said so and the pass has not
    /// numbered it yet: its first byte and the byte after it.
    pub fn span(self) -> Option<(u32, u32)> {
        (self.start != NO_SPAN).then_some((self.start, self.end))
    }
}

/// D3: the id is identity, not value.
impl PartialEq for NodeId {
    fn eq(&self, _: &NodeId) -> bool {
        true
    }
}

impl Eq for NodeId {}

/// **Printed as `NodeId`, and with its number only in the pretty form**
/// (`{:#?}`): the compiler tells two arguments apart by their `{:?}` text
/// (`check::argument_shape`), and a number in it would make two copies of one
/// expression differ, which is D3 turned around.
impl std::fmt::Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "NodeId({})", self.number)
        } else {
            f.write_str("NodeId")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_ids_are_equal_and_their_numbers_are_not() {
        assert_eq!(NodeId::numbered(1), NodeId::numbered(2));
        assert_ne!(NodeId::numbered(1).get(), NodeId::numbered(2).get());
    }

    #[test]
    fn a_fresh_id_is_new_every_time_and_above_the_parser() {
        let first = NodeId::fresh().get();
        let second = NodeId::fresh().get();
        assert!(first >= FIRST_BUILT);
        assert_ne!(first, second);
    }
}
