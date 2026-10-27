//! **A map at a package's boundary is a type parameter each use fills**
//! ([ADR-010](../../../../docs/specification/adr/adr-010.md) D9, #581 step 5).
//!
//! A map position in a `pub` struct's fields or in a `pub` function's
//! parameters and result is a hidden type parameter `__S0`, `__S1`, ... of that
//! item, in the order the declaration writes them, bounded by
//! `nikaia_std::hash::Seed`. A `pub` struct's parameters default to
//! `nikaia_std::hash::Keyed`. A mention of such a struct is a position that
//! carries its parameters too: in a `pub` function's signature they become
//! that function's, anywhere else the package's own choice fills them
//! (`Fast` where the package's input is trusted, `Keyed` otherwise).
//!
//! The types are written by the type emitter as it always did; this reads the
//! text of one written type and puts the parameters in. A type's text is one
//! well-formed run of paths and angle brackets, which is all it has to be.

use std::collections::BTreeMap;

use crate::ast::Type;
use crate::contracts::maps::Shape;
use crate::parser::Parsed;
use nikaia_std::tools::emit_seeds::{
    EmitSeedsHost, EmitSeedsPlace, EmitSeedsTables, emit_seeds_fill_calls, emit_seeds_key,
    emit_seeds_read, emit_seeds_tables, emit_seeds_write,
};

/// What the packages read declare: which `pub` structs hold maps (and how
/// many positions, nested structs counted), and which `pub` functions have
/// positions no argument can fix. The tables and their queries are Nikaia's
/// (`tools/emit_seeds.nika`).
#[derive(Debug, Clone)]
pub struct Seeds {
    tables: EmitSeedsTables,
    /// The hidden parameters of each `pub` function and method, by the key
    /// the ledger gives it: one per group of positions its body joins
    /// (`contracts::maps`, Part III 17.1).
    shapes: BTreeMap<String, Shape>,
}

impl Default for Seeds {
    fn default() -> Seeds {
        Seeds {
            tables: emit_seeds_tables(),
            shapes: BTreeMap::new(),
        }
    }
}

impl Seeds {
    /// Reads every file of every package in the build.
    pub fn read(units: &[&Parsed]) -> Seeds {
        Seeds::read_with(units, &[])
    }

    /// As [`Seeds::read`], with the names of the functions of described Rust
    /// crates that hold a map position (barriers too).
    pub fn read_with(units: &[&Parsed], described: &[String]) -> Seeds {
        let views: Vec<_> = units
            .iter()
            .map(|p| crate::check::parsed_view_of(p))
            .collect();
        let mut seeds = Seeds {
            tables: emit_seeds_read(&SeedsHost { units }, &views, described),
            shapes: BTreeMap::new(),
        };
        let found = crate::contracts::maps::analyse_with(units, &seeds);
        let analysed = analysed(&found);
        for found in found {
            seeds.shapes.insert(found.key, found.shape);
        }
        emit_seeds_fill_calls(&mut seeds.tables, &views, &analysed);
        seeds
    }

    /// Whether a `pub` function's positions are the keyed map (a barrier).
    pub fn is_fixed(&self, name: &str) -> bool {
        self.tables.is_fixed(name)
    }

    /// The names a call must not hand a fast map to: calling one makes the
    /// program no more trusted than the keyed hash.
    pub fn barrier_names(&self) -> std::collections::BTreeSet<String> {
        self.tables.barrier_names()
    }

    /// Whether the units declare a trait method, or implement one, with a map
    /// in its signature.
    pub fn trait_maps(&self) -> bool {
        self.tables.trait_maps
    }

    pub(crate) fn key(&self, parsed: &Parsed, ty: &Type, top: bool) -> String {
        emit_seeds_key(&parsed.interner, ty, top)
    }

    /// Whether a type mentions a `pub` struct that declares parameters.
    pub fn mentions_a_struct(&self, parsed: &Parsed, ty: &Type) -> bool {
        self.tables.mentions_a_struct(&parsed.interner, ty)
    }

    /// The positions of a type, for the item that declares them.
    pub fn positions(&self, parsed: &Parsed, ty: &Type) -> usize {
        self.tables.count(&parsed.interner, ty) as usize
    }

    /// How the `pub` function (or method, `Type::name`) declares its hidden
    /// parameters.
    pub fn shape(&self, key: &str) -> Option<&Shape> {
        self.shapes.get(key)
    }

    /// The tables, for the emitter's state (`EmitState::seeds`).
    pub fn tables(&self) -> &EmitSeedsTables {
        &self.tables
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// The `pub` functions a call has to fill, and how many positions each has.
    pub fn calls(&self) -> &BTreeMap<String, (i64, i64)> {
        &self.tables.calls
    }

    /// Whether a `pub` struct of this name declares parameters.
    pub fn arity(&self, name: &str) -> usize {
        self.tables.arity(name) as usize
    }

    /// One written type, with the parameters in. `fast` says what the package
    /// fills an ordinary mention with.
    pub fn write(&self, text: &str, place: &mut EmitSeedsPlace, fast: bool) -> String {
        if self.is_empty() && !place.declaring {
            return text.to_string();
        }
        emit_seeds_write(text, place, fast, &self.tables)
    }
}

/// Per `pub` function of the analysis its key, whether its `impl`'s own type
/// is joined, and how many hidden parameters it declares and its parameters
/// fix (`EmitSeedsHost::analysed`).
fn analysed(found: &[crate::contracts::maps::FnMaps]) -> Vec<(String, bool, i64, i64)> {
    found
        .iter()
        .map(|f| {
            (
                f.key.clone(),
                f.shape.conflict,
                f.shape.own() as i64,
                f.shape.fixed() as i64,
            )
        })
        .collect()
}

/// The maps' analysis for the reader (`tools/emit_seeds.nika`), against the
/// tables it has read so far.
struct SeedsHost<'a> {
    units: &'a [&'a Parsed],
}

impl EmitSeedsHost for SeedsHost<'_> {
    fn analysed(&self, tables: &EmitSeedsTables) -> Vec<(String, bool, i64, i64)> {
        let seeds = Seeds {
            tables: tables.clone(),
            shapes: BTreeMap::new(),
        };
        analysed(&crate::contracts::maps::analyse_with(self.units, &seeds))
    }
}
