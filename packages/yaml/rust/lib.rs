//! serde_yaml 0.9's reading surface over the native YAML parser (ADR-yaml-1
//! D10; ADR-393 D7, option A): `from_str` into any `Deserialize` type, and
//! `Value`, `Mapping`, `Number`, `Error`. The parser is `native`, the lowered
//! package; this facade is hand-written Rust over it.

mod de;
mod error;
pub mod mapping;
mod number;
mod path;
pub mod value;

pub use crate::de::{from_slice, from_str};
pub use crate::error::{Error, Location, Result};
pub use crate::mapping::Mapping;
pub use crate::number::Number;
pub use crate::value::{Sequence, Tag, TaggedValue, Value};
