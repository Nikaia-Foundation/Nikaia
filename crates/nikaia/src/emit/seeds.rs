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

use crate::ast::{Item, Type};
use crate::contracts::maps::Shape;
use crate::parser::Parsed;
use nikaia_std::tools::emit_seeds::{
    EmitSeedsPlace, EmitSeedsTables, emit_seeds_bare, emit_seeds_key, emit_seeds_last,
    emit_seeds_tables, emit_seeds_write,
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

/// The variable a value is a field chain of: `m`, `t.counts`.
fn root_name(parsed: &Parsed, e: &crate::ast::Expr) -> Option<String> {
    match e {
        crate::ast::Expr::Variable(n, _) => Some(parsed.text(*n).to_string()),
        crate::ast::Expr::Field { base, .. } => root_name(parsed, base),
        _ => None,
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
        let mut seeds = Seeds::default();
        seeds.tables.barriers.extend(described.iter().cloned());
        // Fields of every public struct, then the count of each, until the
        // counts stop growing (a struct holds another one that holds a map).
        let mut declared: Vec<(String, Vec<(&Parsed, &Type)>)> = Vec::new();
        for parsed in units {
            for item in &parsed.program.items {
                if let Item::Struct {
                    name,
                    fields,
                    is_public: true,
                    is_extern: false,
                    ..
                } = &item.node
                {
                    declared.push((
                        parsed.text(*name).to_string(),
                        fields.iter().map(|f| (*parsed, &f.ty)).collect(),
                    ));
                }
            }
        }
        seeds.count_types(&declared);
        seeds.read_barriers(units);
        // **A method that hands a map of its type to a barrier**: the `impl`'s
        // parameters are the type's and one method cannot fix them to std's
        // hasher, so the type declares none - its maps are the keyed one - and
        // a program that names it is no more trusted than the keyed hash.
        for _ in 0..4 {
            let mut bad = seeds.self_handers(units);
            // And a method whose body makes two maps of its own type one: the
            // type declares a parameter for each, no signature can say that
            // two of the `impl`'s parameters are the same.
            for found in crate::contracts::maps::analyse_with(units, &seeds) {
                if found.shape.conflict
                    && let Some((target, _)) = found.key.split_once("::")
                    && seeds.arity(target) > 0
                    && !bad.iter().any(|b| b == target)
                {
                    bad.push(target.to_string());
                }
            }
            if bad.is_empty() {
                break;
            }
            for name in bad {
                seeds.tables.excluded.insert(name.clone());
                seeds.tables.barriers.insert(name);
            }
            seeds.count_types(&declared);
            seeds.read_barriers(units);
        }
        for found in crate::contracts::maps::analyse_with(units, &seeds) {
            seeds.shapes.insert(found.key, found.shape);
        }
        for parsed in units {
            for item in &parsed.program.items {
                if let Item::Fn {
                    name: Some(name),
                    generics,
                    args,
                    ret_type,
                    is_public,
                    receiver: None,
                    ..
                } = &item.node
                {
                    if !*is_public
                        && !args
                            .iter()
                            .map(|a| &a.ty)
                            .chain(ret_type.iter())
                            .any(|t| seeds.mentions_a_struct(parsed, t))
                    {
                        continue;
                    }
                    let params: Vec<&Type> = args.iter().map(|a| &a.ty).collect();
                    let mut seen = std::collections::BTreeSet::new();
                    let fixed: usize = params
                        .iter()
                        .map(|t| seeds.distinct(parsed, t, &mut seen))
                        .sum();
                    let total: usize = fixed
                        + ret_type
                            .iter()
                            .map(|t| seeds.distinct(parsed, t, &mut seen))
                            .sum::<usize>();
                    if total > 0 {
                        seeds.tables.functions = true;
                    }
                    if seeds.tables.fixed.contains(parsed.text(*name)) {
                        continue;
                    }
                    // A `pub` function declares one parameter for each group
                    // of positions its body joins.
                    let (total, fixed) = match seeds.shapes.get(parsed.text(*name)) {
                        Some(shape) if *is_public => (shape.own(), shape.fixed()),
                        _ => (total, fixed),
                    };
                    if total > fixed {
                        // A type parameter of its own comes first, and nothing
                        // fixes it from here: the call leaves it to `_`.
                        seeds.tables.calls.insert(
                            parsed.text(*name).to_string(),
                            (
                                (total + generics.len()) as i64,
                                (fixed + generics.len()) as i64,
                            ),
                        );
                    }
                }
            }
        }
        seeds
    }

    /// The hidden parameters of each `pub` struct, until the counts stop
    /// growing (a struct holds another one that holds a map).
    fn count_types(&mut self, declared: &[(String, Vec<(&Parsed, &Type)>)]) {
        self.tables.types.clear();
        for (name, _) in declared {
            if !self.tables.excluded.contains(name) {
                self.tables.types.insert(name.clone(), 0);
            }
        }
        for _ in 0..8 {
            let mut changed = false;
            for (name, fields) in declared {
                if self.tables.excluded.contains(name) {
                    continue;
                }
                let n: i64 = fields
                    .iter()
                    .map(|(p, t)| self.tables.count(&p.interner, t))
                    .sum();
                if self.tables.types.get(name) != Some(&n) {
                    self.tables.types.insert(name.clone(), n);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        self.tables.types.retain(|_, n| *n > 0);
    }

    /// Whether a body hands one of `held` (a name, or a field of one) to a
    /// barrier, to a function that is one, or to a private function with a map
    /// of its own.
    fn hands(&self, parsed: &Parsed, body: &crate::ast::Block, held: &[String]) -> bool {
        let mut hands = false;
        crate::emit::visit_block(body, &mut |e| {
            let (callee, given): (String, &[crate::ast::Expr]) = match e {
                crate::ast::Expr::MethodCall { method, args, .. } => {
                    (parsed.text(*method).to_string(), args)
                }
                crate::ast::Expr::Call { func, args, .. } => match &**func {
                    crate::ast::Expr::Path(segments, _) => match segments.last() {
                        Some(last) => (parsed.text(*last).to_string(), args),
                        None => return,
                    },
                    crate::ast::Expr::Variable(one, _) => (parsed.text(*one).to_string(), args),
                    _ => return,
                },
                _ => return,
            };
            if (self.tables.barriers.contains(&callee)
                || self.tables.fixed.contains(&callee)
                || self.tables.private.contains(&callee))
                && given
                    .iter()
                    .any(|a| root_name(parsed, a).is_some_and(|n| held.contains(&n)))
            {
                hands = true;
            }
        });
        hands
    }

    /// The types with a method (not a trait's) that hands a map of `self` on.
    fn self_handers(&self, units: &[&Parsed]) -> Vec<String> {
        let mut out = Vec::new();
        for parsed in units {
            for item in &parsed.program.items {
                let Item::Impl {
                    trait_name: None,
                    target,
                    methods,
                    ..
                } = &item.node
                else {
                    continue;
                };
                let name = emit_seeds_last(parsed.text(target.name));
                if self.arity(&name) == 0 || out.contains(&name) {
                    continue;
                }
                let held = vec!["self".to_string()];
                for m in methods {
                    if let Item::Fn {
                        receiver: Some(_),
                        body,
                        ..
                    } = &m.node
                        && self.hands(parsed, body, &held)
                    {
                        out.push(name.clone());
                        break;
                    }
                }
            }
        }
        out
    }

    /// The traits' methods with a map position, and then the `pub` functions
    /// that hand a position they were given to one of them (or to another such
    /// function), until none is added.
    fn read_barriers(&mut self, units: &[&Parsed]) {
        for parsed in units {
            for item in &parsed.program.items {
                match &item.node {
                    Item::Trait { methods, .. } => {
                        for m in methods {
                            let held = m
                                .node
                                .args
                                .iter()
                                .map(|a| &a.ty)
                                .chain(m.node.ret_type.iter())
                                .any(|t| self.count(parsed, t) > 0);
                            if held {
                                self.tables.trait_maps = true;
                                self.tables
                                    .barriers
                                    .insert(parsed.text(m.node.name).to_string());
                            }
                        }
                    }
                    Item::Impl {
                        trait_name: Some(_),
                        methods,
                        ..
                    } => {
                        for m in methods {
                            if let Item::Fn {
                                name: Some(name),
                                args,
                                ret_type,
                                ..
                            } = &m.node
                                && args
                                    .iter()
                                    .map(|a| &a.ty)
                                    .chain(ret_type.iter())
                                    .any(|t| self.count(parsed, t) > 0)
                            {
                                self.tables.trait_maps = true;
                                self.tables.barriers.insert(parsed.text(*name).to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // **A method whose result alone holds a map position**: a method call
        // takes no turbofish here, so nothing could fill it. Its positions are
        // the keyed map, like a barrier's.
        for parsed in units {
            for item in &parsed.program.items {
                let Item::Impl {
                    trait_name: None,
                    methods,
                    ..
                } = &item.node
                else {
                    continue;
                };
                for m in methods {
                    let Item::Fn {
                        name: Some(name),
                        args,
                        ret_type: Some(ret),
                        is_public: true,
                        ..
                    } = &m.node
                    else {
                        continue;
                    };
                    let mut seen = std::collections::BTreeSet::new();
                    let given: usize = args
                        .iter()
                        .map(|a| self.distinct(parsed, &a.ty, &mut seen))
                        .sum();
                    if self.distinct(parsed, ret, &mut seen) > 0 && given == 0 {
                        self.tables.fixed.insert(parsed.text(*name).to_string());
                        self.tables.functions = true;
                    }
                }
            }
        }
        // **A private function with a map of its own in its signature** has
        // the package's class for it, not a hidden parameter: a `pub` function
        // that hands it a map it was given cannot say that the two are one
        // type, so its positions are the keyed map, as a trait method's are
        // (the package writes the keyed map in its own bodies, `seeded()`).
        self.tables.private.clear();
        for parsed in units {
            for item in &parsed.program.items {
                if let Item::Fn {
                    name: Some(name),
                    args,
                    ret_type,
                    is_public: false,
                    receiver: None,
                    ..
                } = &item.node
                    && args
                        .iter()
                        .map(|a| &a.ty)
                        .chain(ret_type.iter())
                        .any(|t| emit_seeds_bare(&parsed.interner, t) > 0)
                {
                    self.tables.private.insert(parsed.text(*name).to_string());
                }
            }
        }
        for _ in 0..4 {
            let mut changed = false;
            for parsed in units {
                for item in &parsed.program.items {
                    let Item::Fn {
                        name: Some(name),
                        args,
                        body,
                        is_public: true,
                        receiver: None,
                        ..
                    } = &item.node
                    else {
                        continue;
                    };
                    let fname = parsed.text(*name).to_string();
                    if self.tables.fixed.contains(&fname) {
                        continue;
                    }
                    let held: Vec<String> = args
                        .iter()
                        .filter(|a| self.count(parsed, &a.ty) > 0)
                        .map(|a| parsed.text(a.name).to_string())
                        .collect();
                    if held.is_empty() {
                        continue;
                    }
                    let hands = self.hands(parsed, body, &held);
                    if hands {
                        self.tables.fixed.insert(fname);
                        self.tables.functions = true;
                        changed = true;
                    }
                }
            }
            // A method that hands a map it was given (not `self`'s) on.
            for parsed in units {
                for item in &parsed.program.items {
                    let Item::Impl {
                        trait_name: None,
                        methods,
                        ..
                    } = &item.node
                    else {
                        continue;
                    };
                    for m in methods {
                        let Item::Fn {
                            name: Some(name),
                            args,
                            body,
                            is_public: true,
                            receiver: Some(_),
                            ..
                        } = &m.node
                        else {
                            continue;
                        };
                        let fname = parsed.text(*name).to_string();
                        if self.tables.fixed.contains(&fname) {
                            continue;
                        }
                        let held: Vec<String> = args
                            .iter()
                            .filter(|a| self.count(parsed, &a.ty) > 0)
                            .map(|a| parsed.text(a.name).to_string())
                            .collect();
                        if !held.is_empty() && self.hands(parsed, body, &held) {
                            self.tables.fixed.insert(fname);
                            self.tables.functions = true;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
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

    fn count(&self, parsed: &Parsed, ty: &Type) -> usize {
        self.tables.count(&parsed.interner, ty) as usize
    }

    fn distinct(
        &self,
        parsed: &Parsed,
        ty: &Type,
        seen: &mut std::collections::BTreeSet<String>,
    ) -> usize {
        self.tables.distinct(&parsed.interner, ty, seen) as usize
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
        self.count(parsed, ty)
    }

    /// How the `pub` function (or method, `Type::name`) declares its hidden
    /// parameters.
    pub fn shape(&self, key: &str) -> Option<&Shape> {
        self.shapes.get(key)
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
