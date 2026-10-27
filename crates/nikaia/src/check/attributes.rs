//! **Attributes** ([ADR-331](../../../docs/specification/adr/adr-331.md)
//! D1-D4, Part II 10.3): `@json::Name("createdAt")` before a declaration is a
//! value of the struct the path names. A struct is an attribute where
//! `@meta::Attribute(…)` marks it, naming the places it may stand; anywhere
//! else it is `NK1235`, and a second one where the mark does not say
//! `repeatable: true` is `NK1236`. The arguments are checked as a call's are,
//! against the struct's fields: those without a default by position, those
//! with one by name after the `;`.
//!
//! The checking is `tools/check_attributes.nika` (U-d2); what stays here is
//! what the ledger and the emitter read: the mark's name and text, `@inline`'s word.

use super::*;

/// The mark itself, which `std` declares and nothing else may be.
pub(crate) const MARK: &str = "meta::Attribute";

/// **`@inline`, the language's own** ([ADR-370](../../../docs/specification/adr/adr-370.md)
/// D1): no struct declares it, and it stands before a function or a method
/// that has a body.
pub(crate) const INLINE: &str = "inline";

/// What `@inline` asks, as Rust's attribute writes it: `@inline`,
/// `@inline(always)`, `@inline(never)`. `None` where none is written, or where
/// the one written is refused (its words are the checker's to say).
pub(crate) fn inline_written(unit: &Parsed, attributes: &[ast::Attribute]) -> Option<&'static str> {
    let attribute = attributes.iter().find(|a| unit.text(a.name) == INLINE)?;
    match (&attribute.args[..], attribute.config.is_empty()) {
        ([], true) => Some("#[inline]"),
        ([Expr::Variable(word, _)], true) => match unit.text(*word) {
            "always" => Some("#[inline(always)]"),
            "never" => Some("#[inline(never)]"),
            _ => None,
        },
        _ => None,
    }
}

/// **A mark as the ledger writes it**: `field, struct; repeatable` (ADR-331
/// D8), read off its arguments (`tools/check_attributes.nika`).
pub(crate) fn mark_text(unit: &Parsed, mark: &ast::Attribute) -> String {
    nikaia_std::tools::check_attributes::check_attributes_mark_text(&parsed_view_of(unit), mark)
}
