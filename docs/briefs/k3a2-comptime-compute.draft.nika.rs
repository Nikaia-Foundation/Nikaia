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

/// What a call's left-out option stands for where its default is not known in
/// a run (#468), and a default's type as its run builds it: Nikaia's
/// (`tools/comptime_tree.nika`).
pub(crate) use nikaia_std::tools::comptime_tree::{
    NOT_COMPUTED_YET, comptime_as_built as as_built,
};

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
    typed: &dyn Fn(&str) -> Option<Ty>,
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
                files, here, bound, value, ty, known, typed, library, workshop, bounds,
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
    typed: &dyn Fn(&str) -> Option<Ty>,
    library: &crate::contracts::Ledger,
    workshop: &crate::grammar_run::Workshop,
    bounds: Bounds,
) -> Computed {
    use crate::contracts::LedgerOps;
    let not_here = |why: &str| Computed::NotHere(why.to_string());
    let Some(sub) = sub_program(files[here], bound, value, ty, known, typed) else {
        // **A callee nothing declares is named** (0.0.113): its result has no
        // type, so neither has the value, and the callee is why.
        if let Some(callee) = undeclared_callee(files, here, value, library) {
            return Computed::NotHere(format!(
                "It calls `{callee}`, which no file of your program declares and `std` doesn't have"
            ));
        }
        // The earlier constants, or else the type, had no written form.
        return match unit_without_its_constants(files[here], bound, known, typed) {
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
            false => unit_without_its_constants(file, bound, known, typed),
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
        let ahead = worked_out_ahead(
            files, bound, &absent, known, typed, library, workshop, bounds,
        );
        // **Only a constant worked out anew is a reason to run again**: one
        // that stays out stays out, and asking again went round without end.
        if ahead.keys().all(|name| known(name).is_some()) {
            return not_here(
                "It reads a `comptime` that can't be computed before it: one that reads it in turn",
            );
        }
        let known = |name: &str| known(name).or_else(|| ahead.get(name).cloned());
        return computed(
            files, here, bound, value, ty, &known, typed, library, workshop, bounds,
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
        return Computed::Forbidden {
            callee,
            because: because.to_string(),
        };
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
    let dump = match crate::grammar_run::dumper(&units[here], ty) {
        Ok(dump) => dump,
        Err(Wall::NoCrossedForm { ty, because }) => {
            return Computed::NotHere(format!(
                "Its type `{ty}` can't be handed back from the program that computes it: {because}"
            ));
        }
        Err(_) => {
            return not_here("Its type can't be handed back from the program that computes it");
        }
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
#[allow(clippy::too_many_arguments)]
fn worked_out_ahead(
    files: &[&Parsed],
    bound: &str,
    absent: &BTreeSet<String>,
    known: &dyn Fn(&str) -> Option<Value>,
    typed: &dyn Fn(&str) -> Option<Ty>,
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
                        files, at, named, value, &ty, known, typed, library, workshop, bounds,
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

/// Why a value is not computed, where this compiler cannot build its program.
const NOT_LOWERED: &str =
    "This compiler couldn't build the program that computes it, which is this compiler's fault";

/// Why a value is not computed, where its run's answer could not be read.
const NOT_READ: &str = "What the program that computed it handed back couldn't be read";

/// Why a value is not computed, where an earlier `comptime` has no literal.
const NOT_CARRIED: &str = "A `comptime` declared beside it came to a value this compiler can't write into the program that computes it";

/// **ADR-321 D19's note**: a check with no place to compile build-time code.
pub(crate) const NOWHERE: &str = "This check has no place to compile build-time code in, and a value that runs code is computed only by compiling it and running it";
