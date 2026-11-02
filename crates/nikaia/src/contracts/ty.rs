// crates/nikaia/src/contracts/ty.rs
//
// The type language the checker reasons in, and the one the ledger records.
//
// It is `ast::Type` with one addition and one subtraction. The addition is
// `Unknown`, which is not a type but the absence of a claim - see below. The
// subtraction is the interner: a ledger is a file, so a name here is text, and
// two types are the same when they are written the same. That is name-for-name
// (ADR-296 D17) applied to types: nothing resolves a module or an alias, so
// `postgres::Connection` and `Connection` are different types, which is correct
// for a compiler that does not know they are not.
//
// **`Unknown` is the whole design.** Stage 0 has no signatures for the Rust
// half of `std` beyond what `std.contracts` writes down, and a Nikaia program
// calls `push_str`, `entry`, `map_or` and `chars` freely. A checker that had to
// answer for those would either need a second frontend for Rust or would have
// to guess - and a type checker that guesses reports errors that are not there,
// which is worse than one that says less. So the rule is:
//
//   * anything involving `Unknown` is compatible with anything;
//   * an error is reported only where **both** sides are known and disagree.
//
// The checker is therefore sound in the direction that matters for a tool
// people run: it never rejects a correct program. It does not catch every
// wrong one, and what it does not catch is a function of how much of `std` is
// written down - which is a number that goes up as ADR-014 proceeds, without
// this file changing.

use crate::ast;
use crate::parser::Parsed;

/// `Ty` and `Shape` are declared in Nikaia ([ADR-294](../../../docs/specification/adr/adr-294.md)
/// D1): `nikaia-std/src/tools/ty.nika`, with the words below, the text a type
/// is written as, and what the checker asks of one.
pub use nikaia_std::tools::ty::{
    ARRAY, ENDS, PAR, PAUSES, REPLAYS, SEEN, SEQ, SIZED, Shape, TEXT, TEXT_VIEW, Ty, bind, qualify,
    renamed, split_args, substitute,
};

/// **What `T::fields` walks**, one element of it
/// ([ADR-304](../../../docs/specification/adr/adr-304.md) D2,
/// [ADR-304](../../../docs/specification/adr/adr-304.md)).
///
/// A name **no program can write**: `$` is not in an identifier, so a reader
/// never meets it and a `.nika` file cannot declare one. It is the type a
/// `for field in T::fields` binding has, and the two members it answers —
/// `.name` and `.of(value)` — are the whole of what Part II 10.3 gives a
/// reflected field.
///
/// **A type and not a `struct` in `std`**, because it never reaches the
/// language below: the loop is **unrolled**, so what is emitted is the field
/// reads a program would have written by hand and there is nothing left for a
/// descriptor to be at run time.
pub const FIELD: &str = "$Field";

/// **One of an `enum`'s variants, as `T::variants` hands it out**: `.name`,
/// the variant's own name as text, and `.is(value)`, whether a value is that
/// variant. Unrolled as a field is, so it never reaches the language below.
pub const VARIANT: &str = "$Variant";

/// A type's own name, with the module it lives in taken off.
///
/// **Since [ADR-313](../../../docs/specification/adr/adr-313.md) D3 a `std`
/// type carries its module**: `collections::HashMap`, `time::Duration`,
/// `foreign::CStr`. The module says where the name is *reached from* and the
/// last segment is the type, so every rule here that names a type by hand — the
/// copy list, the crossing analysis's containers, the hash a map gets — asks
/// this rather than the written name.
///
/// Asked for every key of every ledger in some walks, so the last `:` is found
/// by its byte rather than by a searcher for `"::"` built per call; a `:` that
/// is not half of a `::` - which no key writes - takes the general search.
pub fn base(name: &str) -> &str {
    match name.rfind(':') {
        None => name,
        Some(at) if at > 0 && name.as_bytes()[at - 1] == b':' => &name[at + 1..],
        Some(_) => name.rsplit_once("::").map_or(name, |(_, last)| last),
    }
}

/// **What stays Rust of a type**: its constructors from a name, which take
/// anything that becomes text, reading one back (`tools/ty.nika`'s `parse`),
/// and the type a `.nika` declaration names, which reads this compiler's tree.
/// What the checker asks of a type (`fits`, `erase`, `is_a_view`, …) and the
/// walks over one (`bind`, `substitute`, `qualify`, `renamed`) are
/// `tools/ty.nika`'s since ADR-294 step (d) (0.0.295). An extension trait,
/// because the type is `nikaia-std`'s and an inherent `impl` belongs to the
/// crate that declares it.
pub trait TyOps {
    fn named(name: impl Into<String>) -> Ty;
    fn view(name: impl Into<String>) -> Ty;
    fn seen(inner: Ty) -> Ty;
    fn parse(text: &str) -> Ty;
    fn from_ast(parsed: &Parsed, ty: &ast::Type) -> Ty;
}

impl TyOps for Ty {
    fn named(name: impl Into<String>) -> Ty {
        Ty::Named {
            name: name.into(),
            args: Vec::new(),
            view: false,
        }
    }

    fn view(name: impl Into<String>) -> Ty {
        Ty::Named {
            name: name.into(),
            args: Vec::new(),
            view: true,
        }
    }

    /// **What a lock handed out**
    /// ([ADR-281](../../../../docs/specification/adr/adr-281.md) D22).
    ///
    /// `Seen[T]` is a type here and in the ledger's type language, and it is
    /// **not** a type in the language below: the emitter erases it, so a
    /// `Seen[i64]` is an `i64`, a field declared `Seen[i64]` is an `i64` field,
    /// and a signature with `Seen` in it is one without. No counter, no marker,
    /// no check at run time, no bytes.
    ///
    /// What it buys is that the **shape** of a read-modify-write through two
    /// doors is visible: the value a `set` is given carries where it came from.
    fn seen(inner: Ty) -> Ty {
        Ty::Named {
            name: SEEN.to_string(),
            args: vec![inner],
            view: false,
        }
    }

    /// Read one back: `tools/ty.nika`'s `parse` (ADR-294 step (c)), which reads
    /// what [`Ty::text`] writes and the older spellings a ledger may still hold.
    fn parse(text: &str) -> Ty {
        nikaia_std::tools::ty::parse(text)
    }

    /// The type a `.nika` declaration names.
    fn from_ast(parsed: &Parsed, ty: &ast::Type) -> Ty {
        // **In Nikaia** (`tools/declared.nika`, ADR-294, #125).
        nikaia_std::tools::declared::written_ty(&parsed.interner, &parsed.aliases, ty)
    }
}
