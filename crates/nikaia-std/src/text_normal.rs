// crates/nikaia-std/src/text_normal.rs
//
// `text::Form` and `s.normalized(form)`
// ([ADR-320](../../../docs/specification/adr/adr-320.md) D8, D10), written in
// Rust: Unicode's normalisation forms (UAX #15) as `unicode-normalization`
// 0.1.25 implements them, which is **Unicode 17.0.0**
// ([`NORMALIZATION_UNICODE_VERSION`]). The compiler reads a `'…'` of several
// scalars through [`normalized`] too, so a program and its compiler agree on
// what NFC makes of a text.

/// The Unicode version the normalisation follows, `(major, minor, update)`.
pub use unicode_normalization::UNICODE_VERSION as NORMALIZATION_UNICODE_VERSION;

/// One of Unicode's four normalisation forms (UAX #15).
///
/// `Nfc` composes and `Nfd` decomposes, by canonical equivalence; `Nfkc` and
/// `Nfkd` do the same by compatibility as well, so `ﬁ` becomes `fi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Canonical decomposition, then canonical composition.
    Nfc,
    /// Canonical decomposition.
    Nfd,
    /// Compatibility decomposition, then canonical composition.
    Nfkc,
    /// Compatibility decomposition.
    Nfkd,
}

/// `text` in the normalisation form `form`, as new text.
pub fn normalized(text: &str, form: Form) -> String {
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
    fn normalized(&self, form: Form) -> String;
}

impl TextNormalized for str {
    fn normalized(&self, form: Form) -> String {
        normalized(self, form)
    }
}

/// `s.quoted()`: the text as Rust's `{:?}` writes it (#804).
pub trait TextQuoted {
    /// The text between double quotes, with `"`, `\`, newline, tab, carriage
    /// return and the other control characters escaped, as new text.
    fn quoted(&self) -> String;
}

impl TextQuoted for str {
    fn quoted(&self) -> String {
        format!("{self:?}")
    }
}
