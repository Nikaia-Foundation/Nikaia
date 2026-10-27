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
//! what the ledger reads: the mark's name and text.

use super::*;

/// The mark itself, which `std` declares and nothing else may be.
pub(crate) const MARK: &str = "meta::Attribute";

/// **A mark as the ledger writes it**: `field, struct; repeatable` (ADR-331
/// D8), read off its arguments (`tools/check_attributes.nika`).
pub(crate) fn mark_text(unit: &Parsed, mark: &ast::Attribute) -> String {
    nikaia_std::tools::check_attributes::check_attributes_mark_text(&parsed_view_of(unit), mark)
}
