// crates/nikaia-std/src/text_normal.rs
//
// `s.normalized(form)` over `text::Form`
// ([ADR-320](../../../docs/specification/adr/adr-320.md) D8, D10), written in
// Rust: Unicode's normalisation forms (UAX #15) as `unicode-normalization`
// 0.1.25 implements them, which is **Unicode 17.0.0**
// ([`NORMALIZATION_UNICODE_VERSION`]); `Form` itself is declared in Nikaia
// (`runtime/text_normal.nika`). The compiler reads a `'…'` of several
// scalars through [`normalized`] too, so a program and its compiler agree on
// what NFC makes of a text.

/// The Unicode version the normalisation follows, `(major, minor, update)`.
pub use unicode_normalization::UNICODE_VERSION as NORMALIZATION_UNICODE_VERSION;

/// One of Unicode's four normalisation forms (UAX #15), declared in Nikaia
/// (`runtime/text_normal.nika`).
pub use crate::runtime::Form;

/// `text` in the normalisation form `form`, as new text.
pub fn normalized(text: &str, form: Form) -> crate::block::Text {
    use unicode_normalization::UnicodeNormalization;
    match form {
        Form::Nfc => text.nfc().collect(),
        Form::Nfd => text.nfd().collect(),
        Form::Nfkc => text.nfkc().collect(),
        Form::Nfkd => text.nfkd().collect(),
    }
}

/// `s.normalized(form)`: what `normalized` makes of the text, as a method.
pub trait TextNormalized {
    /// The text in the normalisation form `form`, as new text.
    fn normalized(&self, form: Form) -> crate::block::Text;
}

impl TextNormalized for str {
    fn normalized(&self, form: Form) -> crate::block::Text {
        normalized(self, form)
    }
}

/// `s.quoted()`: the text as Rust's `{:?}` writes it (#804).
pub trait TextQuoted {
    /// The text between double quotes, with `"`, `\`, newline, tab, carriage
    /// return and the other control characters escaped, as new text.
    fn quoted(&self) -> crate::block::Text;
}

impl TextQuoted for str {
    fn quoted(&self) -> crate::block::Text {
        crate::block::format(crate::block::Place::Outside, format_args!("{self:?}"))
    }
}
