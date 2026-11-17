#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
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
/// D1): `nikaia-std/src/compiler/ty.nika`, with the words below, the text a type
/// is written as, and what the checker asks of one.
pub use nikaia_std::compiler::ty::{
    ARRAY, ENDS, PAR, PAUSES, REPLAYS, SEEN, SEQ, SIZED, Shape, TEXT, TEXT_VIEW, Ty, bind, qualify,
    renamed, split_args, substitute,
};

/// **What stays Rust of a type**: the doors the Rust host calls a type's
/// constructors through, each a call into `compiler/ty.nika` (`a_named_ty`,
/// `a_view_ty`, `a_seen_ty`, `parse`, #1913) or `compiler/declared.nika`
/// (`written_ty`). What the checker asks of a type (`fits`, `erase`,
/// `is_a_view`, …) and the walks over one (`bind`, `substitute`, `qualify`,
/// `renamed`) are `ty.nika`'s since ADR-294 step (d) (0.0.295). An extension
/// trait, because the type is `nikaia-std`'s and an inherent `impl` belongs to
/// the crate that declares it; it goes when its Rust callers move.
pub trait TyOps {
    fn named(name: impl Into<String>) -> Ty;
    fn view(name: impl Into<String>) -> Ty;
    fn seen(inner: Ty) -> Ty;
    fn parse(text: &str) -> Ty;
    fn from_ast(parsed: &Parsed, ty: &ast::Type) -> Ty;
}

impl TyOps for Ty {
    fn named(name: impl Into<String>) -> Ty {
        nikaia_std::compiler::ty::a_named_ty(name.into())
    }

    fn view(name: impl Into<String>) -> Ty {
        nikaia_std::compiler::ty::a_view_ty(name.into())
    }

    /// `Seen[T]`, what a lock handed out (ADR-281 D22): `ty.nika`'s `a_seen_ty`.
    fn seen(inner: Ty) -> Ty {
        nikaia_std::compiler::ty::a_seen_ty(inner)
    }

    /// Read one back: `ty.nika`'s `parse` (ADR-294 step (c)), which reads
    /// what [`Ty::text`] writes and the older spellings a ledger may still hold.
    fn parse(text: &str) -> Ty {
        nikaia_std::compiler::ty::parse(text)
    }

    /// The type a `.nika` declaration names: `declared.nika`'s `written_ty`.
    fn from_ast(parsed: &Parsed, ty: &ast::Type) -> Ty {
        nikaia_std::compiler::declared::written_ty(&parsed.interner, &parsed.aliases, ty)
    }
}
