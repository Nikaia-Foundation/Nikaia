//! **The ledger key `maps`** ([ADR-010](../../../../docs/specification/adr/adr-010.md)
//! D9, Part III 13.5, #581 step 6): for a `pub` function with a map in its
//! signature, which of those map positions are one map, and what its body
//! puts into each - `from(p, ...)` (keys as trusted as the arguments named,
//! `from()` for none) or `keyed`.
//!
//! ## A permission key, so every doubt is `keyed`
//!
//! A caller reads the line and never the body. A wrong `from(..)` lets a caller
//! give the fixed seed to a map the body fills with keys an adversary chose, so
//! the body is read in a few shapes only and everything else is `keyed`:
//!
//!   * a map is read (`len`, `get`, `keys`, ...), filled by `m[k] = v` or
//!     `m.insert(k, ..)`, returned, or put in the struct the function returns;
//!   * a key is a literal, a parameter, or built from those with a method that
//!     only reshapes it (`clone`, `trim`, ...); a `let` or a `for` binding
//!     carries what it is made of.
//!
//! Anything else - a map handed to a call, kept in a `let`, compared with
//! something unseen, a key from a call or from a name nothing explains, a
//! `match`, a closure - makes **every** map of the function `keyed`, and its
//! positions are then grouped by the type written (the safe grouping for the
//! types, [`Shape::exact`]).
//!
//! ## Positions, groups, hidden parameters
//!
//! A position is one place a map type is written in the signature: a
//! parameter, a map field of a `pub` struct a parameter has (`t.counts`), or
//! the result likewise. A `pub` type declares one hidden parameter per
//! position. A `pub` function declares one per **group of positions its body
//! joins** (a map assigned to another, compared, returned from a parameter):
//! the `|` in a line. Where the body is not read exactly, the positions of one
//! written type are one group.

use crate::emit::seeds::Seeds;
use crate::parser::Parsed;

use super::Ledger;

use nikaia_std::tools::maps::{MpFnMaps, MpHeld, mp_analyse};

/// What a `pub` function says of its map positions.
#[derive(Debug, Clone)]
pub struct FnMaps {
    /// The ledger's key: `name`, or `Type::name` for a method.
    pub key: String,
    /// The lines of the `maps` key, sorted.
    pub lines: Vec<String>,
    pub shape: Shape,
}

/// How the lowering declares the hidden parameters of a `pub` function: one
/// per group of positions (ADR-010 D9, Part III 17.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    /// For each position, the first position of its group. Positions are
    /// numbered `self`'s first (a method), then the parameters in order, then
    /// the result.
    pub groups: Vec<usize>,
    /// For each position, the ordinal among its `impl` type's own parameters
    /// where it is a position of that type.
    pub within: Vec<Option<usize>>,
    /// How many positions are `self`'s: the written types start after them.
    pub start: usize,
    /// How many positions the parameters have (the result's follow).
    pub of_params: usize,
    /// The body was read to its end, so the groups are the joins it makes.
    /// Otherwise they are the positions of one written type.
    pub exact: bool,
    /// Two positions of the `impl`'s own type are joined: the type declares
    /// one parameter for each, so no signature can say it.
    pub conflict: bool,
}

impl Shape {
    /// The number of hidden parameters the function declares of its own.
    pub fn own(&self) -> usize {
        self.count(self.start, self.groups.len())
    }

    /// How many of them the parameters alone fix.
    pub fn fixed(&self) -> usize {
        self.count(self.start, self.start + self.of_params)
    }

    fn count(&self, from: usize, to: usize) -> usize {
        (from..to)
            .filter(|&at| self.within[at].is_none() && self.groups[at] == at)
            .count()
    }
}

fn to_usize(n: i64) -> usize {
    usize::try_from(n).unwrap_or(0)
}

impl From<MpFnMaps> for FnMaps {
    fn from(found: MpFnMaps) -> FnMaps {
        let shape = found.shape;
        FnMaps {
            key: found.key,
            lines: found.lines,
            shape: Shape {
                groups: shape.groups.into_iter().map(to_usize).collect(),
                within: shape.within.into_iter().map(|w| w.map(to_usize)).collect(),
                start: to_usize(shape.start),
                of_params: to_usize(shape.of_params),
                exact: shape.exact,
                conflict: shape.conflict,
            },
        }
    }
}

/// Every `pub` function of the units that has a map position, with the key
/// the ledger files it under, its lines and its shape.
pub fn analyse(units: &[&Parsed]) -> Vec<FnMaps> {
    analyse_with(units, &Seeds::read(units))
}

/// The same, against the counts `seeds` has already read.
pub fn analyse_with(units: &[&Parsed], seeds: &Seeds) -> Vec<FnMaps> {
    let programs: Vec<&crate::ast::Program> = units.iter().map(|p| &p.program).collect();
    let words: Vec<&winnow_grammar::InternerContext> = units.iter().map(|p| &p.interner).collect();
    let held = MpHeld {
        units: &programs,
        words: &words,
    };
    let key = |unit: i64, ty: &crate::ast::Type| seeds.key(units[to_usize(unit)], ty, true);
    let arity = |name: &str| seeds.arity(name) as i64;
    mp_analyse(&held, &key, &arity)
        .into_iter()
        .map(FnMaps::from)
        .collect()
}

/// Writes the lines into the ledger's `pub` functions.
pub fn infer(ledger: &mut Ledger, units: &[&Parsed]) {
    for f in analyse(units) {
        if let Some(contract) = ledger.functions.get_mut(&f.key) {
            contract.maps = f.lines;
        }
    }
}

/// The keys of the `pub` functions of a package that have a map position and
/// whose ledger line does not let a caller keep the fast hash: a line that
/// says `keyed`, or none.
pub fn keyed_functions(units: &[&Parsed], ledger: &Ledger, package: &str) -> Vec<String> {
    analyse(units)
        .into_iter()
        .map(|f| f.key)
        .filter(|key| {
            ledger
                .functions
                .get(&format!("{package}::{key}"))
                .is_none_or(|c| c.maps.is_empty() || c.maps.iter().any(|l| l.ends_with(": keyed")))
        })
        .collect()
}
