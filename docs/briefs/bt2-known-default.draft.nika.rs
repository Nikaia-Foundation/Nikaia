use std::cell::RefCell;

use std::collections::{BTreeMap, BTreeSet};

use std::path::{Path, PathBuf};

use std::process::Command;

use std::sync::OnceLock;

use crate::ast::{Expr, Item};

use crate::build_time::Value;

use crate::contracts::ty::Ty;

use crate::grammar_run::Wall;

use crate::parser::Parsed;

use nikaia_std::tools::comptime_tree as tree;

/// The name the sub-program hands the value back under. **The sub-program's
/// tree work is Nikaia's** (`tools/comptime_tree.nika`, K1 and K2); what
/// stays here holds the `Parsed` files and hands their items over.
pub(crate) use nikaia_std::tools::comptime_tree::VALUE_FN;

/// **What a compiled build-time run came to**: the checker's own
/// `CheckComputed` (`tools/check_host.nika`).
pub(crate) use nikaia_std::tools::check_host::CheckComputed as Computed;

/// A default's type as its run builds it: Nikaia's
/// (`tools/comptime_tree.nika`).
pub(crate) use nikaia_std::tools::comptime_tree::comptime_as_built as as_built;

/// What the program's ledger recorded for option `at` of `key` - qualified
/// by the package for one that is not the program's - where a run's
/// sub-program is being read and the ledger holds a value.
pub(crate) fn known_default(package: Option<&str>, key: &str, at: usize) -> Option<String> {
    KNOWN.with(|held| {
        let held = held.borrow();
        let ledger = held.as_ref()?;
        let key = match package {
            Some(package) => format!("{package}::{key}"),
            None => key.to_string(),
        };
        let text = &ledger
            .functions
            .get(&key)?
            .signature
            .as_ref()?
            .config
            .get(at)?
            .default;
        (!text.is_empty()).then(|| text.clone())
    })
}
