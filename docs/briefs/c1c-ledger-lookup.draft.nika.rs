use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow};

use crate::ast::{Expr, Item};

use crate::contracts::ty::TyOps;

use crate::emit::{borrowing_structs_beside, holds_view, package_borrowing};

use crate::parser::Parsed;

/// **The ledger's small records are declared in Nikaia**
/// ([ADR-294](../../../docs/specification/adr/adr-294.md) step (b)):
/// `nikaia-std/src/tools/ty.nika`, beside the type language their fields
/// hold. Re-exported here so that every reader keeps its path.
pub use nikaia_std::tools::ty::{
    ConfigContract, Crosses, FieldContract, FnContract, Ledger, Lock, Notes, Provenance, Signature,
    Sync, Threads, TypeContract, VariantContract,
};

impl LedgerOps for Ledger {
    /// A function by the name a caller wrote.
    ///
    /// **Exactly the name, since
    /// [ADR-313](../../../docs/specification/adr/adr-313.md)**: a `std` entry
    /// that lives in a module is reached through the module, `text::digit_value`
    /// and not `digit_value`, and what needs no prefix is the list on Part I's
    /// first page — whose entries are keyed **bare** here, so the exact lookup
    /// is the whole rule.
    ///
    /// It used to match on the last segment, which was name-for-name resolution
    /// (ADR-296 D17) rather than import tracking, and it is what a compiler
    /// without a module graph could honestly do before there was a written
    /// list. What it cost, beyond the prelude being undefined, was answering
    /// about the **wrong function**: a program with its own `fn read` found
    /// `io::read` in a unit that does not carry the ledger of the package
    /// beside it, and the `throws` column then spoke for a callee nobody had
    /// resolved.
    fn lookup(&self, name: &str) -> Option<(String, &FnContract)> {
        self.functions
            .get(name)
            .map(|contract| (name.to_string(), contract))
    }
}
