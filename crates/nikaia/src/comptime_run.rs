//! **A `comptime` that calls a function is compiled and run**
//! ([ADR-321](../../../docs/specification/adr/adr-321.md) D1).
//!
//! The initialiser and everything it calls are lowered as the program is,
//! compiled for the machine that builds and run there; what the program prints
//! is the value. One implementation of the language computes the build-time
//! and the run-time answer, so an overflow stops the build where it would stop
//! the program, and a function of `std`'s Rust half runs as it does when the
//! program runs.
//!
//! **What is built here** (#468's first stage): one small program per
//! `comptime`, against `std` alone. It is linked against one dynamic library
//! that names `std` (D3, the *bundle*), compiled by `rustc` directly and run
//! once; its answer is kept under a key of the code that produced it (D4), so
//! an unchanged `comptime` is neither compiled nor run again - which is also
//! what keeps a lowering inside Cargo's own build from starting another.
//!
//! The program is the file's items without its `fn main` and without its
//! `comptime`s, then the `comptime`s already worked out, each as the literal it
//! came to, and a function that hands back the initialiser. Its `main` writes
//! the value in the encoding a grammar run already uses
//! ([`crate::grammar_run::decode`]).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::ast::{self, Block, Expr, Item, Span, Spanned, Stmt};
use crate::build_time::Value;
use crate::contracts::ty::Ty;
use crate::grammar_run::Wall;
use crate::parser::Parsed;

/// The name the sub-program hands the value back under.
pub(crate) const VALUE_FN: &str = "__nikaia_comptime_value";

/// **The sub-program for one initialiser**, or nothing where this stage does
/// not build one: a type with no written form, or an earlier `comptime` whose
/// value is not a literal.
///
/// `bound` is the `comptime` being worked out; `known` is what an earlier one
/// came to, by name, and nothing for one not worked out yet - which is left
/// out, as the one being worked out is.
pub(crate) fn sub_program(
    parsed: &Parsed,
    bound: &str,
    value: &Expr,
    ty: &Ty,
    known: &dyn Fn(&str) -> Option<Value>,
) -> Option<Parsed> {
    let mut kept = unit_without_its_constants(parsed, bound, known)?;
    let returns = written_type(ty, &kept)?;
    let main = kept.interner.intern_string("main");
    let value_fn = kept.interner.intern_string(VALUE_FN);
    kept.program.items.push(Spanned::new(
        a_function(main, None, Vec::new()),
        Span::nowhere(),
    ));
    let mut stmts = constants_as_lets(parsed, bound, known, &kept);
    stmts.push(Spanned::new(
        Stmt::Return(Some(value.clone())),
        Span::nowhere(),
    ));
    kept.program.items.push(Spanned::new(
        a_function(value_fn, Some(returns), stmts),
        Span::nowhere(),
    ));
    Some(kept)
}

/// **An earlier `comptime` that holds a `std` type's value**, as a `let` at
/// the head of the value function (ADR-318 D5, #519): `let A:
/// time::Duration = time::Duration::new(90, 0)`. Its constructor is not a
/// program's to write and an item cannot hold it, but this run is the
/// compiler's own, and a later `comptime` reads `A` here.
fn constants_as_lets(
    parsed: &Parsed,
    bound: &str,
    known: &dyn Fn(&str) -> Option<Value>,
    kept: &Parsed,
) -> Vec<Spanned<Stmt>> {
    let name = |text: &str| kept.interner.intern_string(text);
    parsed
        .program
        .items
        .iter()
        .filter_map(|item| {
            let Item::Comptime {
                name: bound_here,
                ty,
                ..
            } = &item.node
            else {
                return None;
            };
            let named = parsed.text(*bound_here);
            if named == bound {
                return None;
            }
            let Some(Value::Constant { constructor, parts }) = known(named) else {
                return None;
            };
            let args = parts
                .iter()
                .map(|part| literal_of(part, kept))
                .collect::<Option<Vec<_>>>()?;
            Some(Spanned::new(
                Stmt::Let {
                    names: vec![name(named)],
                    shape: Vec::new(),
                    mutable: false,
                    ty: ty.clone(),
                    value: Expr::Call {
                        func: Box::new(Expr::Path(
                            constructor.split("::").map(name).collect(),
                            crate::ast::NodeId::fresh(),
                        )),
                        args,
                        config: Vec::new(),
                        id: crate::ast::NodeId::fresh(),
                    },
                },
                Span::nowhere(),
            ))
        })
        .collect()
}

/// **One file of the program as the sub-program carries it**: without its
/// `fn main`, and with each `comptime` that is not `bound` as the literal it
/// came to - or left out, where it has not been worked out. Nothing where one
/// that was worked out has no literal here.
pub(crate) fn unit_without_its_constants(
    parsed: &Parsed,
    bound: &str,
    known: &dyn Fn(&str) -> Option<Value>,
) -> Option<Parsed> {
    let mut kept = parsed.keeping(|item| match item {
        // **The program's `main` is not this program's**, and its constants
        // are carried below as what they came to.
        Item::Fn {
            name: Some(name), ..
        } => parsed.text(*name) != "main",
        Item::Comptime { .. } => false,
        _ => true,
    });
    for item in &parsed.program.items {
        let Item::Comptime {
            name,
            ty: declared,
            public,
            ..
        } = &item.node
        else {
            continue;
        };
        let named = parsed.text(*name);
        if named == bound {
            continue;
        }
        let Some(held) = known(named) else {
            continue;
        };
        // Carried by `constants_as_lets` (#519).
        if matches!(held, Value::Constant { .. }) {
            continue;
        }
        kept.program.items.push(Spanned::new(
            Item::Comptime {
                name: *name,
                ty: declared.clone(),
                value: literal_of(&held, &kept)?,
                public: *public,
                bounds: Vec::new(),
            },
            item.span,
        ));
    }
    Some(kept)
}

/// **Without what reads a constant the sub-program does not hold** (#528,
/// #529): the one being worked out, one not worked out yet, one carried only
/// as a `let` of the value function. A `test` block turned function, or any
/// function beside `main` that names one, named a constant the generated Rust
/// did not declare, and the whole run failed for an item it never calls.
///
/// What goes is followed through: a function that calls a dropped one goes
/// too. Names are matched as words, over-approximately (`tools/names.nika`) -
/// the value cannot reach a reader of a constant it is computing, and a
/// program that had such a reader did not build before, so a word shared by
/// accident drops nothing from a program that built. The value function and
/// `main` are always kept, and what went is handed back: a value function
/// that names it reaches a constant not worked out yet, which is for the
/// interpreter to answer, not this run.
pub(crate) fn without_readers(units: &mut [Parsed], absent: BTreeSet<String>) -> BTreeSet<String> {
    let mut gone = absent;
    loop {
        let before = gone.len();
        for unit in units.iter_mut() {
            let mut dropped = Vec::new();
            let mut items = std::mem::take(&mut unit.program.items);
            items.retain_mut(|item| {
                if let Item::Impl { methods, .. } = &mut item.node {
                    methods.retain(|method| match reads_any(unit, &method.node, &gone) {
                        Some(own) => {
                            dropped.extend(own);
                            false
                        }
                        None => true,
                    });
                    return true;
                }
                match reads_any(unit, &item.node, &gone) {
                    Some(own) => {
                        dropped.extend(own);
                        false
                    }
                    None => true,
                }
            });
            unit.program.items = items;
            gone.extend(dropped);
        }
        if gone.len() == before {
            return gone;
        }
    }
}

/// `Some` - with the item's own name, where it has one - when `item` is a
/// body that names something in `gone` and may be left out.
fn reads_any(parsed: &Parsed, item: &Item, gone: &BTreeSet<String>) -> Option<Option<String>> {
    use nikaia_std::tools::names::names_in_block;
    let mut named = BTreeSet::new();
    let own = match item {
        // An option's default is not read here: the call is handed the
        // value the ledger recorded for it.
        Item::Fn { name, body, .. } => {
            let own = name.map(|n| parsed.text(n).to_string());
            if matches!(own.as_deref(), Some("main" | VALUE_FN)) {
                return None;
            }
            names_in_block(body, &parsed.interner, &mut named);
            own
        }
        Item::Test { body, .. } | Item::Bench { body, .. } => {
            names_in_block(body, &parsed.interner, &mut named);
            None
        }
        _ => return None,
    };
    named.iter().any(|name| gone.contains(name)).then_some(own)
}

/// Every `comptime` the files declare, by name.
pub(crate) fn constants_of<'a>(files: impl IntoIterator<Item = &'a Parsed>) -> BTreeSet<String> {
    files
        .into_iter()
        .flat_map(|parsed| {
            parsed
                .program
                .items
                .iter()
                .filter_map(|item| match &item.node {
                    Item::Comptime { name, .. } => Some(parsed.text(*name).to_string()),
                    _ => None,
                })
        })
        .collect()
}

/// **A program of several files, lowered as one** (Part I 9.1): each file
/// against all of them, in order, as `modules::Program` lowers a package -
/// `entry` is the file that holds the value function and so the `fn main`.
pub(crate) fn lowered_units(
    units: &[Parsed],
    entry: usize,
    build: crate::emit::Build,
    contracts: &crate::contracts::Ledger,
    packages: &BTreeMap<String, crate::contracts::Ledger>,
    library: &crate::contracts::Ledger,
) -> Option<crate::emit::Lowered> {
    use crate::emit::{Lowered, Needs, SourceMap};
    let home = &units[entry].package;
    let beside: Vec<&Parsed> = units.iter().filter(|u| &u.package == home).collect();
    let provenance = crate::contracts::trust::analyse_package(&beside, library).provenance;
    let needs = units
        .iter()
        .fold(Needs::default(), |acc, unit| acc.join(Needs::of(unit)));
    let mut rust = String::from("// Generated: a `comptime`'s program (ADR-321).\n\n");
    rust.push_str(&needs.preamble());
    rust.push('\n');
    let mut map = SourceMap::default();
    for (at, unit) in units.iter().enumerate() {
        if &unit.package != home {
            continue;
        }
        let unit = &without_foreign(unit, contracts);
        let body = crate::emit::emit_module_body_at(
            unit,
            &beside,
            build,
            provenance,
            contracts,
            &crate::contracts::Ledger::blank(),
            at == entry,
            &crate::assets::Reads::none(),
            None,
        )
        .ok()?;
        map.extend(body.map.placed(rust.len(), at));
        rust.push_str(&body.rust);
        rust.push('\n');
    }
    let refs: Vec<&Parsed> = units.iter().collect();
    rust.push_str(&dependency_modules(&refs, packages, build, provenance)?);
    rust.push_str("\nconst __NIKAIA_SITES: &[nikaia_std::abort::Site] = &[];\n");
    Some(Lowered {
        rust,
        map,
        published: Default::default(),
    })
}

/// **A file without its foreign parts** (ADR-321 D14): no `extern` block, and
/// no function or method whose entry in `ledger` may not run while the
/// program is built - which is every one that reaches C or a Rust crate, and
/// what D9 refuses a build-time call to anyway. So the build-time program
/// neither links a foreign library nor names a Rust crate.
pub(crate) fn without_foreign(parsed: &Parsed, ledger: &crate::contracts::Ledger) -> Parsed {
    let runs = |key: String| {
        ledger
            .functions
            .get(&key)
            .is_none_or(|contract| crate::build_time::may_not_run(contract).is_none())
    };
    let mut kept = parsed.keeping(|item| match item {
        Item::Extern { .. } => false,
        Item::Fn {
            name: Some(name), ..
        } => runs(parsed.text(*name).to_string()),
        _ => true,
    });
    for item in &mut kept.program.items {
        if let Item::Impl {
            target, methods, ..
        } = &mut item.node
        {
            let target = parsed.text(target.name).to_string();
            methods.retain(|method| match &method.node {
                Item::Fn {
                    name: Some(name), ..
                } => runs(format!("{target}::{}", parsed.text(*name))),
                _ => true,
            });
        }
    }
    kept
}

/// **Each dependency, as the module the program names it by** (ADR-321 D13):
/// `lib::tripled` is `crate::lib::tripled`, lowered against the package's own
/// entries and with the prelude its files are written for. Empty where the
/// build reads no dependency.
pub(crate) fn dependency_modules(
    units: &[&Parsed],
    packages: &BTreeMap<String, crate::contracts::Ledger>,
    build: crate::emit::Build,
    provenance: crate::contracts::Provenance,
) -> Option<String> {
    let mut rust = String::new();
    for (name, own) in packages {
        let theirs: Vec<&Parsed> = units
            .iter()
            .copied()
            .filter(|u| u.package.as_deref() == Some(name.as_str()))
            .collect();
        rust.push_str(&format!(
            "pub mod {name} {{\n#[allow(unused_imports)]\nuse nikaia_std::prelude::*;\n"
        ));
        for unit in &theirs {
            let unit = &without_foreign(unit, own);
            let body = crate::emit::emit_module_body_at(
                unit,
                &theirs,
                build,
                provenance,
                own,
                &crate::contracts::Ledger::blank(),
                false,
                &crate::assets::Reads::none(),
                None,
            )
            .ok()?;
            rust.push_str(&body.rust);
            rust.push('\n');
        }
        rust.push_str("}\n");
    }
    Some(rust)
}

/// **Each dependency's units, derived as the package they are** (ADR-321
/// D13), by the name the program reaches it by.
pub(crate) fn by_package(units: &[&Parsed]) -> BTreeMap<String, crate::contracts::Ledger> {
    use crate::contracts::LedgerOps;
    let mut names: Vec<&str> = units.iter().filter_map(|u| u.package.as_deref()).collect();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .map(|name| {
            let theirs: Vec<&Parsed> = units
                .iter()
                .copied()
                .filter(|u| u.package.as_deref() == Some(name))
                .collect();
            (
                name.to_string(),
                crate::contracts::Ledger::infer_package(&theirs, crate::contracts::std_ledger()),
            )
        })
        .collect()
}

/// **What a compiled build-time run came to.**
pub(crate) enum Computed {
    /// The value.
    Value(Value),
    /// A callee the rule forbids (D2), named as the call resolves.
    Forbidden {
        callee: String,
        because: &'static str,
    },
    /// The run stopped, and what it said.
    Stopped(String),
    /// This compiler builds no program for it, and why, as a sentence: the
    /// value is refused (`NK1127`), never interpreted (ADR-321 D1).
    NotHere(String),
}

/// What a call's left-out option stands for where its default is not known in
/// a run (#468): the emitter's placeholder, which only a refused default
/// reaches.
pub(crate) const NOT_COMPUTED_YET: &str = "a default not computed yet";

/// **A default's type as its run builds it** (#468): the value a view
/// views - `ref String`'s `String` - which the run hands back, and which is
/// written into the ledger the same way.
pub(crate) fn as_built(ty: Ty) -> Ty {
    match ty {
        // `ref String` is read as `str`.
        Ty::Named { name, args, .. } if name == "str" => Ty::Named {
            name: "String".to_string(),
            args,
            view: false,
        },
        Ty::Named { name, args, .. } => Ty::Named {
            name,
            args,
            view: false,
        },
        other => other,
    }
}

/// **Compile and run an initialiser** (ADR-321 D1): `value`, standing in
/// `files[here]`, of type `ty`, with every file of the program around it and
/// `known` for what earlier `comptime`s came to. The rule is asked first
/// (D2), of the ledger entry derived for the function that hands the value
/// back: every call it reaches, by name or on a value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute(
    files: &[&Parsed],
    here: usize,
    bound: &str,
    value: &Expr,
    ty: &Ty,
    known: &dyn Fn(&str) -> Option<Value>,
    library: &crate::contracts::Ledger,
    workshop: &crate::grammar_run::Workshop,
    bounds: Bounds,
    outer: Option<&crate::contracts::Ledger>,
) -> Computed {
    // **The sub-program's own inference and lowering compute no default
    // again**: they read the same functions, and a default compiled while a
    // default is compiled is the same question asked without end. Where the
    // program's ledger is at hand, they read what it recorded (#468).
    defaults_known_from(outer, || {
        defaults_compiled_in(None, || {
            computed(
                files, here, bound, value, ty, known, library, workshop, bounds,
            )
        })
    })
}

#[allow(clippy::too_many_arguments)]
fn computed(
    files: &[&Parsed],
    here: usize,
    bound: &str,
    value: &Expr,
    ty: &Ty,
    known: &dyn Fn(&str) -> Option<Value>,
    library: &crate::contracts::Ledger,
    workshop: &crate::grammar_run::Workshop,
    bounds: Bounds,
) -> Computed {
    use crate::contracts::LedgerOps;
    let not_here = |why: &str| Computed::NotHere(why.to_string());
    let Some(sub) = sub_program(files[here], bound, value, ty, known) else {
        // **A callee nothing declares is named** (0.0.113): its result has no
        // type, so neither has the value, and the callee is why.
        if let Some(callee) = undeclared_callee(files, here, value, library) {
            return Computed::NotHere(format!(
                "It calls `{callee}`, which no file of your program declares and `std` doesn't have"
            ));
        }
        // The earlier constants, or else the type, had no written form.
        return match unit_without_its_constants(files[here], bound, known) {
            None => not_here(NOT_CARRIED),
            Some(_) => not_here(
                "Its type has no form this compiler writes into the program that computes it",
            ),
        };
    };
    let mut sub = Some(sub);
    let mut units: Vec<Parsed> = Vec::with_capacity(files.len());
    for (at, file) in files.iter().enumerate() {
        let unit = match at == here {
            true => sub.take(),
            false => unit_without_its_constants(file, bound, known),
        };
        match unit {
            Some(unit) => units.push(unit),
            None => return not_here(NOT_CARRIED),
        }
    }
    let carried = constants_of(&units);
    let absent: BTreeSet<String> = constants_of(files.iter().copied())
        .into_iter()
        .filter(|name| !carried.contains(name))
        .collect();
    let left_out: BTreeSet<String> = without_readers(&mut units, absent.clone())
        .difference(&absent)
        .cloned()
        .collect();
    // **A value that reads what went** - a constant not worked out yet (one
    // that is, it holds as a `let`), or a function that reads one - has it
    // worked out first, here, as an item
    // read before its declaration is (Part II 10.2), and runs again with it.
    if units[here]
        .program
        .items
        .iter()
        .any(|item| match &item.node {
            Item::Fn {
                name: Some(name),
                body,
                ..
            } if units[here].text(*name) == VALUE_FN => {
                let mut named = BTreeSet::new();
                nikaia_std::tools::names::names_in_block(body, &units[here].interner, &mut named);
                named.iter().any(|name| {
                    left_out.contains(name) || (absent.contains(name) && known(name).is_none())
                })
            }
            _ => false,
        })
    {
        let ahead = worked_out_ahead(files, bound, &absent, known, library, workshop, bounds);
        // **Only a constant worked out anew is a reason to run again**: one
        // that stays out stays out, and asking again went round without end.
        if ahead.keys().all(|name| known(name).is_some()) {
            return not_here(
                "It reads a `comptime` that can't be computed before it: one that reads it in turn",
            );
        }
        let known = |name: &str| known(name).or_else(|| ahead.get(name).cloned());
        return computed(
            files, here, bound, value, ty, &known, library, workshop, bounds,
        );
    }
    // **A dependency's functions are the program's to call** (ADR-321 D13):
    // each package's units are derived on their own, and the program's
    // against `std` and those entries, under the package's name - as the
    // program's own ledger has them.
    // **The run's home is the package of the file it stands in**: a default
    // of a dependency's function is computed in that dependency, where the
    // program's own files are not visible and the other packages are.
    let home = units[here].package.clone();
    let others: Vec<&Parsed> = units
        .iter()
        .filter(|u| u.package.is_some() && u.package != home)
        .collect();
    let packages = by_package(&others);
    let mut with_packages = library.clone();
    for (name, ledger) in &packages {
        with_packages.absorb(Some(name), ledger.clone());
    }
    let unit_refs: Vec<&Parsed> = units.iter().filter(|u| u.package == home).collect();
    let derived = crate::contracts::Ledger::infer_package(&unit_refs, &with_packages);
    if let Some(because) = derived
        .functions
        .get(VALUE_FN)
        .and_then(crate::build_time::may_not_run)
    {
        let callee =
            culprit(files[here], value, &derived, library).unwrap_or_else(|| bound.to_string());
        return Computed::Forbidden { callee, because };
    }
    let counted = crate::emit::Build {
        counts_steps: true,
        ..Default::default()
    };
    let lowered = match units.len() {
        1 => crate::emit::emit_program(&units[here], counted).ok(),
        _ => lowered_units(&units, here, counted, &derived, &packages, library),
    };
    let Some(lowered) = lowered else {
        return not_here(NOT_LOWERED);
    };
    let Ok(dump) = crate::grammar_run::dumper(&units[here], ty) else {
        return not_here("Its type can't be handed back from the program that computes it");
    };
    let Some(program) = with_driver(&lowered, &dump, bounds) else {
        return not_here(NOT_LOWERED);
    };
    match workshop.run_comptime(&program, bound) {
        Ok(text) => match crate::grammar_run::decode(&text) {
            Ok(computed) => Computed::Value(computed),
            Err(_) => not_here(NOT_READ),
        },
        // A default this run did not have is the default's own refusal,
        // said where it is written.
        Err(Wall::Refused { detail }) if detail.contains(NOT_COMPUTED_YET) => {
            not_here("It leaves out an option whose default isn't computed")
        }
        Err(Wall::Refused { detail }) => Computed::Stopped(detail),
        Err(Wall::NowhereToBuild) => not_here(NOWHERE),
        Err(Wall::InsideAnother { .. }) => not_here(
            "It is met while another build-time program is built, and one isn't built inside another",
        ),
        Err(Wall::NoCrossedForm { ty, because }) => Computed::NotHere(format!(
            "`{ty}` has no form that crosses from build time: {because}"
        )),
        Err(Wall::DidNotBuild { .. }) => not_here(NOT_LOWERED),
        Err(Wall::Unreadable { .. }) => not_here(NOT_READ),
    }
}

/// **The constants a run reads before their turn**, worked out ahead
/// (Part II 10.2: an item-level `comptime` may read one declared below it): a
/// constant of literals folded, one that runs code and writes its type
/// compiled and run as any `comptime` is. One already being worked out
/// further up is left alone, which is how a ring ends: its run stays without
/// it and is refused.
fn worked_out_ahead(
    files: &[&Parsed],
    bound: &str,
    absent: &BTreeSet<String>,
    known: &dyn Fn(&str) -> Option<Value>,
    library: &crate::contracts::Ledger,
    workshop: &crate::grammar_run::Workshop,
    bounds: Bounds,
) -> std::collections::BTreeMap<String, Value> {
    let mut ahead = std::collections::BTreeMap::new();
    let entered = AHEAD.with(|held| held.borrow_mut().insert(bound.to_string()));
    if !entered {
        return ahead;
    }
    let reads = crate::assets::Reads::none();
    for (at, file) in files.iter().enumerate() {
        for item in &file.program.items {
            let Item::Comptime {
                name, ty, value, ..
            } = &item.node
            else {
                continue;
            };
            let named = file.text(*name);
            if !absent.contains(named) || AHEAD.with(|held| held.borrow().contains(named)) {
                continue;
            }
            let so_far = |name: &str| known(name);
            let folded = crate::build_time::BuildTime::new(file, files, &reads, &so_far)
                .evaluate(value)
                .ok();
            let worked = match (folded, ty) {
                (Some(value), _) => Some(value),
                (None, Some(written)) => {
                    let ty = <Ty as crate::contracts::ty::TyOps>::from_ast(file, written);
                    match computed(
                        files, at, named, value, &ty, known, library, workshop, bounds,
                    ) {
                        Computed::Value(value) => Some(value),
                        _ => None,
                    }
                }
                (None, None) => None,
            };
            if let Some(value) = worked {
                ahead.insert(named.to_string(), value);
            }
        }
    }
    AHEAD.with(|held| held.borrow_mut().remove(bound));
    ahead
}

thread_local! {
    /// The constants being worked out ahead, so that a ring of them ends.
    static AHEAD: std::cell::RefCell<BTreeSet<String>> =
        const { std::cell::RefCell::new(BTreeSet::new()) };
}

/// Why a value is not computed, where this compiler cannot build its program.
const NOT_LOWERED: &str =
    "This compiler couldn't build the program that computes it, which is this compiler's fault";

/// Why a value is not computed, where its run's answer could not be read.
const NOT_READ: &str = "What the program that computed it handed back couldn't be read";

/// Why a value is not computed, where an earlier `comptime` has no literal.
const NOT_CARRIED: &str = "A `comptime` declared beside it came to a value this compiler can't write into the program that computes it";

/// **ADR-321 D19's note**: a check with no place to compile build-time code.
pub(crate) const NOWHERE: &str = "This check has no place to compile build-time code in, and a value that runs code is computed only by compiling it and running it";

/// **The callee the rule forbids, named as the call resolves**: a function by
/// its name or its path, a method by its key (`Reader::read`), read off the
/// program's entries and then `std`'s.
fn undeclared_callee(
    files: &[&Parsed],
    here: usize,
    value: &Expr,
    library: &crate::contracts::Ledger,
) -> Option<String> {
    let declared = |name: &str| {
        library.functions.contains_key(name)
            || files.iter().any(|file| {
                file.program.items.iter().any(|item| match &item.node {
                    Item::Fn {
                        name: Some(named), ..
                    } => file.text(*named) == name,
                    Item::Struct { name: named, .. } | Item::Enum { name: named, .. } => {
                        file.text(*named) == name
                    }
                    _ => false,
                })
            })
    };
    let mut found = None;
    crate::emit::visit_expr(value, &mut |expr: &Expr| {
        if found.is_some() {
            return;
        }
        if let Expr::Call { func, .. } = expr
            && let Expr::Variable(name, _) = &**func
        {
            let name = files[here].text(*name);
            if !declared(name) {
                found = Some(name.to_string());
            }
        }
    });
    found
}

/// **The callee the rule forbids**, see [`culprit`]'s callers; the callee
/// nothing declares is [`undeclared_callee`]'s.
fn culprit(
    parsed: &Parsed,
    value: &Expr,
    derived: &crate::contracts::Ledger,
    library: &crate::contracts::Ledger,
) -> Option<String> {
    let forbidden = |key: &str| {
        derived
            .functions
            .get(key)
            .or_else(|| library.functions.get(key))
            .is_some_and(|contract| crate::build_time::may_not_run(contract).is_some())
    };
    let mut found = None;
    crate::emit::visit_expr(value, &mut |expr: &Expr| {
        if found.is_some() {
            return;
        }
        let key = match expr {
            Expr::Call { func, .. } => match &**func {
                Expr::Variable(name, _) => Some(parsed.text(*name).to_string()),
                Expr::Path(path, _) => Some(
                    path.iter()
                        .map(|segment| parsed.text(*segment))
                        .collect::<Vec<_>>()
                        .join("::"),
                ),
                _ => None,
            },
            Expr::MethodCall { method, .. } | Expr::SafeMethod { method, .. } => {
                let method = format!("::{}", parsed.text(*method));
                derived
                    .functions
                    .keys()
                    .find(|key| key.ends_with(&method) && forbidden(key))
                    .cloned()
            }
            _ => None,
        };
        if let Some(key) = key.filter(|key| forbidden(key)) {
            found = Some(key);
        }
    });
    found
}

// --- the defaults a ledger records -------------------------------------------

thread_local! {
    /// **Where an option's default is compiled** while a ledger is inferred
    /// ([ADR-318](../../../docs/specification/adr/adr-318.md) D1, ADR-321 D1): the
    /// workshop of the build that reads the program, for as long as it reads it.
    /// The inference has no `Reads` of its own; a build sets this around the read,
    /// and a caller that sets nothing gets the interpreter, as before.
    ///
    /// **One per thread**, because the inference runs on the thread that reads:
    /// two builds side by side, as the tests are, each keep their own.
    static DEFAULTS: std::cell::RefCell<Option<std::sync::Arc<crate::grammar_run::Workshop>>> =
        const { std::cell::RefCell::new(None) };
}

/// Read a program with `at` as the workshop its defaults are compiled in.
pub fn defaults_compiled_in<R>(at: Option<&Path>, read: impl FnOnce() -> R) -> R {
    let workshop = at.map(|at| std::sync::Arc::new(crate::grammar_run::Workshop::at(at)));
    let before = DEFAULTS.with(|held| std::mem::replace(&mut *held.borrow_mut(), workshop));
    let out = read();
    DEFAULTS.with(|held| *held.borrow_mut() = before);
    out
}

thread_local! {
    /// **The defaults the program's ledger recorded**, while a run's
    /// sub-program is inferred (#468): the same functions, so the same
    /// values, read rather than computed a second time.
    static KNOWN: std::cell::RefCell<Option<crate::contracts::Ledger>> =
        const { std::cell::RefCell::new(None) };
}

/// Read with `outer`'s recorded defaults as the ones the inference takes.
fn defaults_known_from<R>(outer: Option<&crate::contracts::Ledger>, read: impl FnOnce() -> R) -> R {
    let before = KNOWN.with(|held| std::mem::replace(&mut *held.borrow_mut(), outer.cloned()));
    let out = read();
    KNOWN.with(|held| *held.borrow_mut() = before);
    out
}

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

/// The workshop defaults are compiled in now, where a build set one.
pub(crate) fn defaults_workshop() -> Option<std::sync::Arc<crate::grammar_run::Workshop>> {
    DEFAULTS.with(|held| held.borrow().clone())
}

fn a_function(
    name: winnow_grammar::Symbol,
    ret_type: Option<ast::Type>,
    stmts: Vec<Spanned<Stmt>>,
) -> Item {
    Item::Fn {
        name: Some(name),
        generics: Vec::new(),
        receiver: None,
        args: Vec::new(),
        config: Vec::new(),
        spread: None,
        ret_type,
        body: Block { stmts },
        is_sync: false,
        sync_by: Vec::new(),
        is_public: false,
        can_throw: false,
        is_extern: false,
    }
}

/// **An earlier `comptime`'s value as the literal that writes it**: a number,
/// a truth value, text, and a list, a tuple, a struct or a variant of those.
/// The names a struct or a variant needs are interned in `parsed`, the file
/// the literal goes into.
fn literal_of(value: &Value, parsed: &Parsed) -> Option<Expr> {
    let name = |text: &str| parsed.interner.intern_string(text);
    let all = |items: &[Value]| -> Option<Vec<Expr>> {
        items.iter().map(|item| literal_of(item, parsed)).collect()
    };
    match value {
        Value::Int(n) => Some(Expr::LitInt {
            value: n.magnitude,
            negative: n.negative,
            id: crate::ast::NodeId::fresh(),
        }),
        Value::Bool(b) => Some(Expr::LitBool(*b, crate::ast::NodeId::fresh())),
        Value::Float(f) => Some(Expr::LitFloat(
            format!("{f:?}"),
            crate::ast::NodeId::fresh(),
        )),
        Value::Text(text) => Some(Expr::LitStr {
            text: crate::build_time::written(text),
            id: crate::ast::NodeId::fresh(),
        }),
        Value::List(items) => Some(Expr::ListLit {
            items: all(items)?,
            id: crate::ast::NodeId::fresh(),
        }),
        Value::Tuple(parts) => Some(Expr::Tuple(all(parts)?, crate::ast::NodeId::fresh())),
        // **A `std` type's constructor is not a program's to write**
        // (ADR-318 D5): an item cannot hold one, and the value function
        // gets it as a `let` instead (`constants_as_lets`, #519).
        Value::Constant { .. } => None,
        Value::Struct { name: ty, fields } => Some(Expr::StructLit {
            name: name(ty),
            fields: fields
                .iter()
                .map(|(field, held)| {
                    Some(ast::FieldInit {
                        name: name(field),
                        value: Some(literal_of(held, parsed)?),
                    })
                })
                .collect::<Option<_>>()?,
            id: crate::ast::NodeId::fresh(),
        }),
        Value::Variant {
            ty,
            variant,
            payload,
        } => {
            let path = Expr::Path(vec![name(ty), name(variant)], crate::ast::NodeId::fresh());
            match payload.is_empty() {
                true => Some(path),
                false => Some(Expr::Call {
                    func: Box::new(path),
                    args: all(payload)?,
                    config: Vec::new(),
                    id: crate::ast::NodeId::fresh(),
                }),
            }
        }
    }
}

/// `ty` as the sub-program writes it in the value function's signature.
fn written_type(ty: &Ty, parsed: &Parsed) -> Option<ast::Type> {
    let plain = |name: &str| ast::Type {
        name: parsed.interner.intern_string(name),
        generics: Vec::new(),
        is_view: false,
        is_tuple: false,
        is_nullable: false,
        code: Box::new(None),
        count: None,
        is_mut: false,
        is_slice: false,
        either: false,
        lasting: false,
    };
    match ty {
        Ty::Named { name, args, view } => {
            let mut out = plain(name);
            out.is_view = *view;
            out.generics = args
                .iter()
                .map(|arg| written_type(arg, parsed))
                .collect::<Option<_>>()?;
            Some(out)
        }
        Ty::Count(n) => {
            let mut out = plain(&n.to_string());
            out.count = Some(*n);
            Some(out)
        }
        Ty::Tuple(parts) => {
            let mut out = plain("");
            out.is_tuple = true;
            out.generics = parts
                .iter()
                .map(|part| written_type(part, parsed))
                .collect::<Option<_>>()?;
            Some(out)
        }
        Ty::Nullable(inner) => {
            let mut out = written_type(inner, parsed)?;
            out.is_nullable = true;
            Some(out)
        }
        _ => None,
    }
}

/// **The lowered sub-program with a `main` of its own**: the generated one
/// starts the runtime and runs the program's body, and this one reports a stop,
/// computes the value and writes it.
///
/// **A stop names the byte it came from**, where the program's own table names
/// a `.nika` line (ADR-300 D9): the sub-program has no file of its own, and the
/// checker turns the byte back into a place in the file it is checking. A row
/// is the generated line, [`STOPPED_AT`], and the byte the line's outermost
/// node starts at.
pub(crate) fn with_driver(
    lowered: &crate::emit::Lowered,
    dump: &str,
    bounds: Bounds,
) -> Option<String> {
    let rows: Vec<(i64, i64, i64)> = lowered
        .map
        .rows()
        .map(|(generated, byte, unit)| (generated as i64, byte as i64, unit as i64))
        .collect();
    nikaia_std::tools::comptime_driver::cr_with_driver(
        &lowered.rust,
        &rows,
        VALUE_FN,
        dump,
        &nikaia_std::tools::grammar_decode::gr_dump_helpers(),
        STOPPED_AT,
        bounds.steps,
        bounds.bytes,
    )
}

/// **What a build-time run may spend** ([ADR-321](../../../docs/specification/adr/adr-321.md)
/// D7, D10): counted steps and bytes live at once. How a `comptime` raises
/// either is not decided (D11); a workshop carries them, so that this
/// compiler's own tests can ask for less.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub steps: u64,
    pub bytes: u64,
}

impl Default for Bounds {
    /// Ten billion steps (D7) and 4 GiB (D10).
    fn default() -> Bounds {
        Bounds {
            steps: 10_000_000_000,
            bytes: 4 << 30,
        }
    }
}

/// What a stop's row names instead of a file: the file's place among the
/// program's follows it, and then the byte.
pub(crate) const STOPPED_AT: &str = "@";

// --- compiling and running -----------------------------------------------------

/// **The dynamic library build-time code links against** (D3): one crate that
/// names `std` and the grammar runtime a program's `grammar` lowers to, built
/// by Cargo from the same compiled dependencies as anything else in the
/// workshop.
#[derive(Debug)]
pub(crate) struct Bundle {
    /// The library itself.
    pub(crate) dylib: PathBuf,
    /// Each crate it names, as Cargo compiled it, which `rustc` is pointed at
    /// so that a path `nikaia_std::…` resolves in every module; the code is
    /// the library's.
    pub(crate) rlibs: Vec<(String, PathBuf)>,
    /// Where the libraries it was built from are.
    pub(crate) deps: PathBuf,
    /// What changes when the library is rebuilt: part of every key.
    pub(crate) stamp: String,
    /// Where the toolchain's own `std` is, which a program linked with
    /// `prefer-dynamic` loads - and beside which the toolchain's `rustc` is.
    pub(crate) libdir: PathBuf,
}

/// **What a bundle is built for.** Build-time code counts what it allocates
/// against its bound (D10); a program `nikaia run` compiles against the
/// bundle ([`crate::run_fast`]) means what its Cargo build means, so its
/// bundle has no counting allocator and the profile the generated workspace
/// gives a dependency: no overflow checks, no debug assertions, no debug
/// information, unwinding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flavour {
    BuildTime,
    Run,
}

/// The run bundle's one file: the crates, named, and nothing else.
const RUN_SOURCE: &str = "// GENERATED: the library `nikaia run` links a program against.
pub extern crate nikaia_std;
pub extern crate winnow;
pub extern crate winnow_grammar;
";

/// The run bundle's profile: what `[profile.dev]` of a generated workspace
/// says about a crate that is not the program's.
const RUN_PROFILE: &str = "
[profile.dev]
overflow-checks = false
debug-assertions = false
debug = 0
panic = \"unwind\"
";

/// The crates the bundle names: `std`, and what a `grammar` lowers to.
const NAMED: [&str; 3] = ["nikaia_std", "winnow", "winnow_grammar"];

/// Build the bundle, or say why it could not be built.
///
/// **Once per machine, not per project**: what it is depends on its manifest
/// (where `std` comes from), its one file and the toolchain, never on the
/// program - so it lives under the user cache, keyed by the first two, as the
/// compiled `std` of ADR-002 D4 does. Under each project's own workshop it was
/// built again for every new project, some forty crates (half a minute) before
/// the first `comptime` could run. Cargo's lock on the target directory is what
/// makes two builds at once wait for one another rather than collide.
pub(crate) fn bundle(_workshop: &Path) -> Result<Bundle, String> {
    bundle_of(Flavour::BuildTime)
}

/// **The bundle `nikaia run` links a program against**, built once and then
/// found from its record without starting Cargo: Cargo's start and freshness
/// check is most of what a fast run saves. The record is trusted while this
/// compiler, the toolchain and `std`'s sources are what they were when it was
/// written, and every file it names is still there.
pub(crate) fn run_bundle() -> Result<Bundle, String> {
    let (manifest, source) = bundle_text(Flavour::Run);
    let at = bundle_dir(Flavour::Run, &manifest, &source);
    let record = at.join("built");
    let fingerprint = run_fingerprint();
    if let Some(bundle) = std::fs::read_to_string(&record)
        .ok()
        .and_then(|text| read_record(&text, &fingerprint))
    {
        return Ok(bundle);
    }
    let bundle = bundle_of(Flavour::Run)?;
    let mut text = format!(
        "{fingerprint}\n{}\n{}\n{}\n",
        bundle.dylib.display(),
        bundle.deps.display(),
        bundle.libdir.display()
    );
    for (name, rlib) in &bundle.rlibs {
        text.push_str(&format!("{name}={}\n", rlib.display()));
    }
    write_if_changed(&record, &text)?;
    Ok(bundle)
}

/// What the run bundle's record is valid for: this compiler, the toolchain
/// and the newest of `std`'s sources (a `std` edited under a development
/// checkout is built again, as Cargo would).
fn run_fingerprint() -> String {
    let mut newest = 0u128;
    let mut count = 0usize;
    let mut pending = vec![crate::sysroot::Sysroot::resolve().std_dir()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == "tests")
            {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                pending.push(path);
                continue;
            }
            count += 1;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            newest = newest.max(modified);
        }
    }
    crate::assets::digest(
        format!(
            "{}\n{}\n{}\n{newest}:{count}",
            env!("NIKAIA_COMPILER"),
            orchestrator::project::rustc_version(),
            rustc()
        )
        .as_bytes(),
    )
}

/// The bundle a record names, where it is still the one it names.
fn read_record(text: &str, fingerprint: &str) -> Option<Bundle> {
    let mut lines = text.lines();
    if lines.next()? != fingerprint {
        return None;
    }
    let dylib = PathBuf::from(lines.next()?);
    let deps = PathBuf::from(lines.next()?);
    let libdir = PathBuf::from(lines.next()?);
    let rlibs: Vec<(String, PathBuf)> = lines
        .filter_map(|line| line.split_once('='))
        .map(|(name, rlib)| (name.to_string(), PathBuf::from(rlib)))
        .collect();
    if rlibs.len() != NAMED.len() {
        return None;
    }
    let stamp = stamp_of(&dylib, &rlibs).ok()?;
    Some(Bundle {
        dylib,
        rlibs,
        deps,
        stamp,
        libdir,
    })
}

/// A bundle's manifest and its one file.
fn bundle_text(flavour: Flavour) -> (String, String) {
    let mut manifest = nikaia_std::tools::comptime_driver::cr_bundle_manifest(
        &crate::project::runtime_dependencies_for(NAMED.join(" ").as_str()),
    );
    let source = match flavour {
        Flavour::BuildTime => nikaia_std::tools::comptime_driver::cr_bundle_source(),
        Flavour::Run => {
            manifest.push_str(RUN_PROFILE);
            RUN_SOURCE.to_string()
        }
    };
    (manifest, source)
}

/// Where a bundle lives: the user cache, keyed by its manifest and its file.
fn bundle_dir(flavour: Flavour, manifest: &str, source: &str) -> PathBuf {
    let key = orchestrator::cache::sha256_hex(format!("{manifest}\n{source}").as_bytes());
    orchestrator::cache::Layout::user_cache_dir()
        .join(match flavour {
            Flavour::BuildTime => "build-time-bundle",
            Flavour::Run => "run-bundle",
        })
        .join(&key[..16])
}

/// The library and the crates it names, and when each was last built.
fn stamp_of(dylib: &PathBuf, rlibs: &[(String, PathBuf)]) -> Result<String, String> {
    Ok(std::iter::once(dylib)
        .chain(rlibs.iter().map(|(_, rlib)| rlib))
        .map(|file| {
            let meta = std::fs::metadata(file).map_err(|e| format!("{}: {e}", file.display()))?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            Ok(format!("{}:{}:{modified}", file.display(), meta.len()))
        })
        .collect::<Result<Vec<String>, String>>()?
        .join("\n"))
}

fn bundle_of(flavour: Flavour) -> Result<Bundle, String> {
    let (manifest, bundle_source) = bundle_text(flavour);
    let at = bundle_dir(flavour, &manifest, &bundle_source);
    orchestrator::cache::use_entry(&at);
    let dir = at.join("bundle");
    std::fs::create_dir_all(dir.join("src")).map_err(|e| format!("{}: {e}", dir.display()))?;
    write_if_changed(&dir.join("Cargo.toml"), &manifest)?;
    write_if_changed(&dir.join("src").join("lib.rs"), &bundle_source)?;
    let built = Command::new(cargo())
        .current_dir(orchestrator::cache::Layout::where_tools_start())
        .args(["rustc", "--quiet", "--lib", "--crate-type", "dylib"])
        .arg("--message-format=json")
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(at.join("target"))
        .args(["--", "-C", "prefer-dynamic"])
        .output()
        .map_err(|e| format!("running cargo: {e}"))?;
    if !built.status.success() {
        return Err(String::from_utf8_lossy(&built.stderr).to_string());
    }
    let mut dylib = None;
    let mut rlibs: Vec<(String, PathBuf)> = Vec::new();
    for line in String::from_utf8_lossy(&built.stdout).lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if message["reason"] != "compiler-artifact" {
            continue;
        }
        let files = message["filenames"].as_array().cloned().unwrap_or_default();
        let files = files.iter().filter_map(|f| f.as_str()).map(PathBuf::from);
        match message["target"]["name"].as_str() {
            Some(name) if NAMED.contains(&name) => {
                if let Some(rlib) = files
                    .clone()
                    .find(|f| f.extension().is_some_and(|e| e == "rlib"))
                {
                    rlibs.push((name.to_string(), rlib));
                }
            }
            Some("nikaia_bundle") => {
                dylib = files.clone().find(|f| {
                    f.extension()
                        .is_some_and(|e| e == std::env::consts::DLL_EXTENSION)
                })
            }
            _ => {}
        }
    }
    let (Some(dylib), true) = (dylib, rlibs.len() == NAMED.len()) else {
        return Err("cargo built the bundle and named no library".to_string());
    };
    let deps = rlibs[0]
        .1
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "no directory for std".to_string())?;
    let stamp = stamp_of(&dylib, &rlibs)?;
    Ok(Bundle {
        dylib,
        rlibs,
        deps,
        stamp,
        libdir: PathBuf::from(target_libdir()),
    })
}

/// **The CPU level this process builds for** (ADR-371 D6), as
/// `Settings::cpu_key` resolves it: empty for the baseline, `native=...` for
/// this machine. Set by the lowering before it checks a program.
static CPU_LEVEL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// The lowering that follows builds for the CPU level `key`.
pub(crate) fn build_for_cpu(key: String) {
    if let Ok(mut held) = CPU_LEVEL.lock() {
        *held = key;
    }
}

/// **Compile and run one build-time program**, or hand back what it said when
/// it ran before: its standard output, or `Wall::Refused` with what it wrote
/// where it stopped. Keyed on the program, the toolchain and the bundle (D4).
pub(crate) fn run(at: &Path, bundle: &Bundle, program: &str, name: &str) -> Result<String, Wall> {
    // **A program that asks `target::has` answers for the build's level**
    // (ADR-371 D6): the level is in its key and handed to it; one that does
    // not keeps one answer for every level.
    let level = match program.contains("target::has") {
        true => CPU_LEVEL
            .lock()
            .map(|held| held.clone())
            .unwrap_or_default(),
        false => String::new(),
    };
    let key = crate::assets::digest(
        format!("{program}\n{}\n{}\n{level}", toolchain(), bundle.stamp).as_bytes(),
    );
    // What the run reads: the level's name, `native` for this machine's.
    let named = match level.starts_with("native=") {
        true => "native".to_string(),
        false => level,
    };
    let dir = at.join("comptime").join(&key);
    let answered = dir.join("answer");
    let stopped = dir.join("stopped");
    if let Ok(answer) = std::fs::read_to_string(&answered) {
        return Ok(answer);
    }
    if let Ok(said) = std::fs::read_to_string(&stopped) {
        return Err(Wall::Refused { detail: said });
    }
    let did_not = |detail: String| Wall::DidNotBuild { detail };
    // **A program that did not compile is not compiled again** in this
    // process: the same text fails the same way, and a check asks for one
    // value many times over (each test, each variant).
    if let Some(detail) = NOT_BUILT
        .lock()
        .ok()
        .and_then(|held| held.get(&key).cloned())
    {
        return Err(did_not(detail));
    }
    let source = dir.join("main.rs");
    write_if_changed(&source, program).map_err(did_not)?;
    // **Each build compiles and runs its own binary, in a directory of its
    // own**, moved into place after: `rustc` names its object files after the
    // crate, so two builds of the same program side by side wrote, linked and
    // deleted each other's.
    let scratch = unique_beside(&dir.join("build"));
    std::fs::create_dir_all(&scratch)
        .map_err(|e| did_not(format!("{}: {e}", scratch.display())))?;
    let binary = scratch.join("run");
    let compiled = Command::new(rustc())
        .current_dir(orchestrator::cache::Layout::where_tools_start())
        .args([
            "--edition=2024",
            "--crate-name",
            "comptime",
            "--crate-type",
            "bin",
        ])
        .args(["-C", "opt-level=0", "-C", "overflow-checks=on"])
        .args([
            "-C",
            "debug-assertions=off",
            "-C",
            "prefer-dynamic",
            "-A",
            "warnings",
        ])
        .arg("--extern")
        .arg(format!("nikaia_bundle={}", bundle.dylib.display()))
        .args(bundle.rlibs.iter().flat_map(|(name, rlib)| {
            ["--extern".to_string(), format!("{name}={}", rlib.display())]
        }))
        .arg("-L")
        .arg(format!("dependency={}", bundle.deps.display()))
        .arg("-o")
        .arg(&binary)
        .arg(&source)
        .output()
        .map_err(|e| did_not(format!("running rustc: {e}")))?;
    if !compiled.status.success() {
        let _ = std::fs::remove_dir_all(&scratch);
        let detail = String::from_utf8_lossy(&compiled.stderr).to_string();
        if let Ok(mut held) = NOT_BUILT.lock() {
            held.insert(key, detail.clone());
        }
        return Err(did_not(detail));
    }
    let mut libraries: Vec<PathBuf> = Vec::new();
    if let Some(dir) = bundle.dylib.parent() {
        libraries.push(dir.to_path_buf());
    }
    libraries.push(bundle.deps.clone());
    libraries.push(PathBuf::from(target_libdir()));
    if let Some(held) = std::env::var_os(LIBRARY_PATH) {
        libraries.extend(std::env::split_paths(&held));
    }
    let child = Command::new(&binary)
        .env("RUST_BACKTRACE", "0")
        .env(nikaia_std::target::LEVEL_VAR, &named)
        .env(
            LIBRARY_PATH,
            std::env::join_paths(libraries).map_err(|e| did_not(e.to_string()))?,
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| did_not(format!("running {}: {e}", binary.display())))?;
    let ran =
        waited(child, name).map_err(|e| did_not(format!("running {}: {e}", binary.display())));
    let _ = std::fs::rename(&binary, dir.join("run"));
    let _ = std::fs::remove_dir_all(&scratch);
    let ran = ran?;
    if ran.status.success() {
        let answer = String::from_utf8_lossy(&ran.stdout).trim().to_string();
        write_if_changed(&answered, &answer).map_err(did_not)?;
        return Ok(answer);
    }
    let mut said = String::from_utf8_lossy(&ran.stderr).trim().to_string();
    // **A run that ended without a word** - killed by the system, most often
    // for its memory - says how it ended instead.
    if said.is_empty() {
        said = format!("it ended without saying why ({})", ran.status);
    }
    // **A stop the program reported is an answer**, and kept as one: the same
    // code stops the same way. A run that died without a word is not, and is
    // tried again next time.
    if said.contains("the program stopped") || said.contains("nikaia-build-time:") {
        write_if_changed(&stopped, &said).map_err(did_not)?;
    }
    Err(Wall::Refused { detail: said })
}

/// The programs this process could not compile, by their key, with what
/// `rustc` said.
static NOT_BUILT: std::sync::Mutex<BTreeMap<String, String>> =
    std::sync::Mutex::new(BTreeMap::new());

/// **The run, waited for, and named while it takes long**
/// ([ADR-321](../../../docs/specification/adr/adr-321.md) D12): after a few
/// seconds the build says which `comptime` is still running and for how long.
/// A message, never a bound.
fn waited(mut child: std::process::Child, name: &str) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out = std::thread::spawn(move || {
        let mut held = Vec::new();
        if let Some(pipe) = stdout.as_mut() {
            let _ = pipe.read_to_end(&mut held);
        }
        held
    });
    let err = std::thread::spawn(move || {
        let mut held = Vec::new();
        if let Some(pipe) = stderr.as_mut() {
            let _ = pipe.read_to_end(&mut held);
        }
        held
    });
    let began = std::time::Instant::now();
    let mut next = std::time::Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if began.elapsed() >= next {
            eprintln!(
                "note: `{name}` is still being computed while the program is built ({} s)",
                began.elapsed().as_secs()
            );
            next += std::time::Duration::from_secs(10);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    Ok(std::process::Output {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}

/// The variable the loader reads for where libraries are.
#[cfg(target_os = "macos")]
const LIBRARY_PATH: &str = "DYLD_LIBRARY_PATH";
#[cfg(windows)]
const LIBRARY_PATH: &str = "PATH";
#[cfg(not(any(target_os = "macos", windows)))]
const LIBRARY_PATH: &str = "LD_LIBRARY_PATH";

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

pub(crate) fn rustc() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

/// `rustc -vV`, once: the toolchain a key is about.
pub(crate) fn toolchain() -> &'static str {
    static HELD: OnceLock<String> = OnceLock::new();
    HELD.get_or_init(|| {
        Command::new(rustc())
            .current_dir(orchestrator::cache::Layout::where_tools_start())
            .arg("-vV")
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
            .unwrap_or_default()
    })
}

/// Where the toolchain's own `std` is, which a program linked with
/// `prefer-dynamic` loads.
pub(crate) fn target_libdir() -> &'static str {
    static HELD: OnceLock<String> = OnceLock::new();
    HELD.get_or_init(|| {
        Command::new(rustc())
            .current_dir(orchestrator::cache::Layout::where_tools_start())
            .args(["--print", "target-libdir"])
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .unwrap_or_default()
    })
}

/// Written only when it changed, so that Cargo's freshness keeps working.
pub(crate) fn write_if_changed(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    if std::fs::read_to_string(path).is_ok_and(|held| held == text) {
        return Ok(());
    }
    // **Written whole or not at all**: builds side by side share the
    // workshop, and one that read a file another was still writing compiled
    // half a program or read half an answer.
    let partial = unique_beside(path);
    std::fs::write(&partial, text).map_err(|e| format!("{}: {e}", partial.display()))?;
    std::fs::rename(&partial, path).map_err(|e| {
        let _ = std::fs::remove_file(&partial);
        format!("{}: {e}", path.display())
    })
}

/// A path beside `path` that no other build writes at the same time.
pub(crate) fn unique_beside(path: &Path) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.{n}.partial", std::process::id()));
    path.with_file_name(name)
}
