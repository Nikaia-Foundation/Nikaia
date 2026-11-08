//! The hand-written root of `nikaia_demo` (ADR-393 D7): it re-exports
//! two lowered items, one only with the feature `cube`, and adds a function
//! of its own over a module of its own.

mod words;

pub use native::area;

#[cfg(feature = "cube")]
pub use native::cube;

/// The area of a square, in words.
pub fn described(side: i64) -> String {
    format!("{} {}", words::AREA, native::area(side))
}
