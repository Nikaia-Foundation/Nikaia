//! serde_yaml 0.9's reading surface over the native YAML parser (ADR-yaml-1
//! D10; ADR-393 D7, option A): `from_str`, `from_slice`, `from_reader` and
//! `Deserializer` (one document, or each of a stream) into any `Deserialize`
//! type; `Value` with its indexing, `Tag` and `TaggedValue`, `Mapping`,
//! `Number`, `from_value` and `to_value`; `with::singleton_map`; `Error`;
//! and the writing side, `to_string`, `to_writer` and `Serializer` (#1515).
//! The parser and the writer are `native`, the lowered package; this facade
//! is hand-written Rust over it. `value/`, `mapping.rs`, `number.rs` and `with.rs` are
//! serde_yaml 0.9.34's (David Tolnay, MIT OR Apache-2.0); `de.rs` reads the
//! native events where serde_yaml reads libyaml's.

#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::derive_partial_eq_without_eq,
    clippy::derived_hash_with_manual_eq,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::manual_map,
    clippy::match_like_matches_macro,
    clippy::match_same_arms,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::question_mark,
    clippy::should_implement_trait,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::wildcard_in_or_patterns
)]

pub use crate::de::{Deserializer, from_reader, from_slice, from_str};
pub use crate::error::{Error, Location, Result};
pub use crate::ser::{Serializer, to_string, to_writer};
#[doc(inline)]
pub use crate::value::{Index, Number, Sequence, Value, from_value, to_value};

#[doc(inline)]
pub use crate::mapping::Mapping;

mod de;
mod error;
pub mod mapping;
mod number;
mod path;
mod ser;
pub mod value;
pub mod with;

// Prevent downstream code from implementing the Index trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl Sealed for crate::Value {}
    impl<T> Sealed for &T where T: ?Sized + Sealed {}
}
