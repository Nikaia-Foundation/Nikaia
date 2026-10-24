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
//! what the ledger and the emitter read: the mark, `mark_of`, `@inline`'s word.

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

/// Where an attribute may stand, as the mark's arguments spell them (D2).
const PLACES: [&str; 8] = [
    "field",
    "struct",
    "enum",
    "variant",
    "fn",
    "parameter",
    "trait",
    "impl",
];

/// What a struct's `@meta::Attribute(…)` says: the places, and whether it may
/// stand twice.
#[derive(Clone, Debug, Default)]
pub(crate) struct Mark {
    pub places: BTreeSet<String>,
    pub repeatable: bool,
}

impl Mark {
    /// As the ledger writes it: `field, struct; repeatable` (ADR-331 D8).
    pub(crate) fn text(&self) -> String {
        let places = self.places.iter().cloned().collect::<Vec<_>>().join(", ");
        match self.repeatable {
            true => format!("{places}; repeatable"),
            false => places,
        }
    }
}

/// The places a mark names, read off its arguments; a word that is not one is
/// refused where the mark is checked.
pub(crate) fn mark_of(unit: &Parsed, mark: &ast::Attribute) -> Mark {
    let places = mark
        .args
        .iter()
        .filter_map(|arg| match arg {
            Expr::Variable(word, _) => Some(unit.text(*word).to_string()),
            Expr::Path(segments, _) => segments.last().map(|s| unit.text(*s).to_lowercase()),
            _ => None,
        })
        .filter(|word| PLACES.contains(&word.as_str()))
        .collect();
    let repeatable = mark.config.iter().any(|option| {
        unit.text(option.name) == "repeatable" && matches!(option.value, Expr::LitBool(true, _))
    });
    Mark { places, repeatable }
}
