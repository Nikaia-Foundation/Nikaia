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

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::ast::Expr;
use crate::build_time::Value;
use crate::contracts::ty::Ty;
use crate::grammar_run::Wall;
use crate::parser::Parsed;

use nikaia_std::tools::comptime_tree as tree;
/// The name the sub-program hands the value back under. **The sub-program's
/// tree work is Nikaia's** (`tools/comptime_tree.nika`, K1 and K2); what
/// stays here holds the `Parsed` files and hands their items over.
pub(crate) use nikaia_std::tools::comptime_tree::VALUE_FN;

/// **The sub-program for one initialiser** (`comptime_sub_program`), or
/// nothing where this stage does not build one: a type with no written form,
/// or an earlier `comptime` whose value is not a literal.
pub(crate) fn sub_program(
    parsed: &Parsed,
    bound: &str,
    value: &Expr,
    ty: &Ty,
    known: &dyn Fn(&str) -> Option<Value>,
    typed: &dyn Fn(&str) -> Option<Ty>,
) -> Option<Parsed> {
    let mut kept = parsed.keeping(|item| tree::comptime_keeps(item, &parsed.interner));
    let interner = &kept.interner;
    let built = tree::comptime_sub_program(
        &mut kept.program.items,
        &parsed.program.items,
        &parsed.interner,
        bound,
        value,
        ty,
        &known,
        &typed,
        &|text: &str| interner.intern_string(text),
    );
    built.then_some(kept)
}

/// **One file of the program as the sub-program carries it**
/// (`comptime_unit_without_its_constants`), or nothing where a constant that
/// was worked out has no literal here.
pub(crate) fn unit_without_its_constants(
    parsed: &Parsed,
    bound: &str,
    known: &dyn Fn(&str) -> Option<Value>,
    typed: &dyn Fn(&str) -> Option<Ty>,
) -> Option<Parsed> {
    let mut kept = parsed.keeping(|item| tree::comptime_keeps(item, &parsed.interner));
    let interner = &kept.interner;
    let carried = tree::comptime_unit_without_its_constants(
        &mut kept.program.items,
        &parsed.program.items,
        &parsed.interner,
        bound,
        &known,
        &typed,
        &|text: &str| interner.intern_string(text),
    );
    carried.then_some(kept)
}

/// **Without what reads a constant the sub-program does not hold** (#528,
/// #529; `comptime_readers_dropped`, followed through over the files). What
/// went is handed back.
pub(crate) fn without_readers(units: &mut [Parsed], absent: BTreeSet<String>) -> BTreeSet<String> {
    // **The count is read before the cell is lent**: a `borrow()` in the
    // argument list lives to the end of the call, and the closure's
    // `borrow_mut()` then panics.
    let count = units.len() as i64;
    let units = RefCell::new(units);
    tree::comptime_readers_dropped(count, absent, &|at: i64, gone: &BTreeSet<String>| {
        drop_readers(&mut units.borrow_mut()[at as usize], gone)
    })
}

/// `unit` without what reads a name of `gone` (`comptime_drop_readers`).
fn drop_readers(unit: &mut Parsed, gone: &BTreeSet<String>) -> Vec<String> {
    tree::comptime_drop_readers(&mut unit.program.items, &unit.interner, gone)
}

/// Every `comptime` the files declare, by name.
pub(crate) fn constants_of<'a>(files: impl IntoIterator<Item = &'a Parsed>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for parsed in files {
        tree::comptime_constants_of(&parsed.program.items, &parsed.interner, &mut out);
    }
    out
}

/// **The units of a build-time program, as `ComptimeLowerHost` answers
/// about them** (`tools/comptime_tree.nika`, K3a): the files, the emitter,
/// the build's trust and the source map stay here; Nikaia names the units by
/// index and assembles the text.
struct LowerHost<'a> {
    units: &'a [&'a Parsed],
    build: crate::emit::Build,
    provenance: crate::contracts::Provenance,
    map: RefCell<crate::emit::SourceMap>,
}

impl<'a> LowerHost<'a> {
    fn new(
        units: &'a [&'a Parsed],
        build: crate::emit::Build,
        provenance: crate::contracts::Provenance,
    ) -> Self {
        LowerHost {
            units,
            build,
            provenance,
            map: RefCell::new(crate::emit::SourceMap::default()),
        }
    }

    fn those(&self, at: &[i64]) -> Vec<&'a Parsed> {
        at.iter().map(|&at| self.units[at as usize]).collect()
    }
}

impl tree::ComptimeLowerHost for LowerHost<'_> {
    fn unit_count(&self) -> i64 {
        self.units.len() as i64
    }

    fn package_of(&self, at: i64) -> Option<String> {
        self.units[at as usize].package.clone()
    }

    fn preamble(&self) -> String {
        let needs = self
            .units
            .iter()
            .fold(crate::emit::Needs::none(), |acc, unit| {
                acc.join(&crate::emit::needs_of(unit))
            });
        needs.preamble()
    }

    fn emit_unit(
        &self,
        at: i64,
        beside: &[i64],
        contracts: &crate::contracts::Ledger,
        entry: bool,
        placed_at: Option<i64>,
    ) -> Option<String> {
        let unit = &without_foreign(self.units[at as usize], contracts);
        let body = crate::emit::emit_module_body_at(
            unit,
            &self.those(beside),
            self.build,
            self.provenance,
            contracts,
            &crate::contracts::Ledger::blank(),
            entry,
            &crate::assets::Reads::none(),
            None,
        )
        .ok()?;
        if let Some(placed_at) = placed_at {
            self.map
                .borrow_mut()
                .extend(body.map.placed(placed_at as usize, at as usize));
        }
        Some(body.rust)
    }

    fn infer_package(&self, at: &[i64]) -> crate::contracts::Ledger {
        use crate::contracts::LedgerOps;
        crate::contracts::Ledger::infer_package(&self.those(at), crate::contracts::std_ledger())
    }
}

/// **A program of several files, lowered as one** (Part I 9.1,
/// `comptime_lowered_units`): `entry` is the file that holds the value
/// function and so the `fn main`.
pub(crate) fn lowered_units(
    units: &[&Parsed],
    entry: usize,
    build: crate::emit::Build,
    contracts: &crate::contracts::Ledger,
    packages: &BTreeMap<String, crate::contracts::Ledger>,
    library: &crate::contracts::Ledger,
) -> Option<crate::emit::Lowered> {
    let home = &units[entry].package;
    let beside: Vec<&Parsed> = units
        .iter()
        .copied()
        .filter(|u| &u.package == home)
        .collect();
    let provenance = crate::contracts::trust::analyse_package(&beside, library).provenance;
    let host = LowerHost::new(units, build, provenance);
    let rust = tree::comptime_lowered_units(&host, entry as i64, contracts, packages)?;
    Some(crate::emit::Lowered {
        rust,
        map: host.map.into_inner(),
        published: Default::default(),
    })
}

/// **A file without its foreign parts** (ADR-321 D14,
/// `comptime_without_foreign`): what `ledger` says may not run while the
/// program is built (`build_time::may_not_run`) goes.
pub(crate) fn without_foreign(parsed: &Parsed, ledger: &crate::contracts::Ledger) -> Parsed {
    let runs = |key: &str| {
        ledger
            .functions
            .get(key)
            .is_none_or(|contract| crate::build_time::may_not_run(contract).is_none())
    };
    let mut kept = parsed.keeping(|_| true);
    tree::comptime_without_foreign(&mut kept.program.items, &parsed.interner, &runs);
    kept
}

/// **Each dependency, as the module the program names it by** (ADR-321 D13,
/// `comptime_dependency_modules`).
pub(crate) fn dependency_modules(
    units: &[&Parsed],
    packages: &BTreeMap<String, crate::contracts::Ledger>,
    build: crate::emit::Build,
    provenance: crate::contracts::Provenance,
) -> Option<String> {
    tree::comptime_dependency_modules(&LowerHost::new(units, build, provenance), packages)
}

/// **Each dependency's units, derived as the package they are** (ADR-321
/// D13, `comptime_by_package`), by the name the program reaches it by.
pub(crate) fn by_package(units: &[&Parsed]) -> BTreeMap<String, crate::contracts::Ledger> {
    let host = LowerHost::new(
        units,
        crate::emit::Build::default(),
        crate::contracts::Provenance::Trusted,
    );
    tree::comptime_by_package(&host)
}

/// **What a compiled build-time run came to**: the checker's own
/// `CheckComputed` (`tools/check_host.nika`).
pub(crate) use nikaia_std::tools::check_host::CheckComputed as Computed;

/// A default's type as its run builds it: Nikaia's
/// (`tools/comptime_tree.nika`).
pub(crate) use nikaia_std::tools::comptime_tree::comptime_as_built as as_built;

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
            let host = ComputeHost {
                files,
                units: RefCell::new(Vec::new()),
                library,
                workshop,
                bounds,
            };
            tree::comptime_computed(&host, here as i64, bound, value, ty, known, typed)
        })
    })
}

/// **What computing a build-time constant asks of the compiler**
/// (`ComptimeComputeHost`, K3a): the program's files, the units made from
/// them, the inference, the emitter and the workshop stay here; Nikaia names
/// files by their place and units by their index in `units`.
struct ComputeHost<'a> {
    files: &'a [&'a Parsed],
    units: RefCell<Vec<Parsed>>,
    library: &'a crate::contracts::Ledger,
    workshop: &'a crate::grammar_run::Workshop,
    bounds: Bounds,
}

impl ComputeHost<'_> {
    /// `unit` kept, and its index.
    fn made(&self, unit: Option<Parsed>) -> Option<i64> {
        let mut units = self.units.borrow_mut();
        units.push(unit?);
        Some(units.len() as i64 - 1)
    }
}

impl tree::ComptimeComputeHost for ComputeHost<'_> {
    fn file_count(&self) -> i64 {
        self.files.len() as i64
    }

    fn sub_program(
        &self,
        here: i64,
        bound: &str,
        value: &Expr,
        ty: &Ty,
        known: &impl Fn(&str) -> Option<Value>,
        typed: &impl Fn(&str) -> Option<Ty>,
    ) -> Option<i64> {
        self.made(sub_program(
            self.files[here as usize],
            bound,
            value,
            ty,
            known,
            typed,
        ))
    }

    fn unit_without_its_constants(
        &self,
        at: i64,
        bound: &str,
        known: &impl Fn(&str) -> Option<Value>,
        typed: &impl Fn(&str) -> Option<Ty>,
    ) -> Option<i64> {
        self.made(unit_without_its_constants(
            self.files[at as usize],
            bound,
            known,
            typed,
        ))
    }

    fn file_words(&self, at: i64) -> &winnow_grammar::InternerContext {
        &self.files[at as usize].interner
    }

    fn file_items(&self, at: i64) -> &[crate::ast::Spanned<crate::ast::Item>] {
        &self.files[at as usize].program.items
    }

    fn library(&self) -> &crate::contracts::Ledger {
        self.library
    }

    fn constants_of_files(&self) -> BTreeSet<String> {
        constants_of(self.files.iter().copied())
    }

    fn constants_of_units(&self, units: &[i64]) -> BTreeSet<String> {
        let held = self.units.borrow();
        constants_of(units.iter().map(|&at| &held[at as usize]))
    }

    fn drop_readers(&self, unit: i64, gone: &BTreeSet<String>) -> Vec<String> {
        drop_readers(&mut self.units.borrow_mut()[unit as usize], gone)
    }

    fn value_fn_names(&self, unit: i64) -> BTreeSet<String> {
        let held = self.units.borrow();
        let unit = &held[unit as usize];
        tree::comptime_value_fn_names(&unit.program.items, &unit.interner)
    }

    fn unit_package(&self, unit: i64) -> Option<String> {
        self.units.borrow()[unit as usize].package.clone()
    }

    fn by_package(&self, units: &[i64]) -> BTreeMap<String, crate::contracts::Ledger> {
        let held = self.units.borrow();
        let refs: Vec<&Parsed> = units.iter().map(|&at| &held[at as usize]).collect();
        by_package(&refs)
    }

    fn derived(
        &self,
        units: &[i64],
        packages: &BTreeMap<String, crate::contracts::Ledger>,
    ) -> crate::contracts::Ledger {
        use crate::contracts::LedgerOps;
        let mut with_packages = self.library.clone();
        for (name, ledger) in packages {
            with_packages.absorb(Some(name), ledger.clone());
        }
        let held = self.units.borrow();
        let refs: Vec<&Parsed> = units.iter().map(|&at| &held[at as usize]).collect();
        crate::contracts::Ledger::infer_package(&refs, &with_packages)
    }

    fn lowered_alone(&self, unit: i64) -> Option<tree::ComptimeLowered> {
        let held = self.units.borrow();
        crate::emit::emit_program(&held[unit as usize], counted())
            .ok()
            .map(rows_of)
    }

    fn lowered_units(
        &self,
        units: &[i64],
        here: i64,
        derived: &crate::contracts::Ledger,
        packages: &BTreeMap<String, crate::contracts::Ledger>,
    ) -> Option<tree::ComptimeLowered> {
        let held = self.units.borrow();
        let refs: Vec<&Parsed> = units.iter().map(|&at| &held[at as usize]).collect();
        lowered_units(
            &refs,
            here as usize,
            counted(),
            derived,
            packages,
            self.library,
        )
        .map(rows_of)
    }

    fn dump(&self, unit: i64, ty: &Ty, out: &mut String) -> Option<Wall> {
        let held = self.units.borrow();
        let unit = &held[unit as usize];
        nikaia_std::tools::grammar_run::grammar_run_dump(
            &unit.interner,
            &unit.aliases,
            &unit.program,
            crate::contracts::std_ledger(),
            ty,
            out,
        )
    }

    fn bounds(&self) -> (u64, u64) {
        (self.bounds.steps, self.bounds.bytes)
    }

    fn run_comptime(&self, program: &str, bound: &str, answer: &mut String) -> Option<Wall> {
        match self.workshop.run_comptime(program, bound) {
            Ok(text) => {
                *answer = text;
                None
            }
            Err(wall) => Some(wall),
        }
    }

    fn decode(&self, text: &str) -> Option<Value> {
        crate::grammar_run::decode(text).ok()
    }

    fn enter_ahead(&self, bound: &str) -> bool {
        AHEAD.with(|held| held.borrow_mut().insert(bound.to_string()))
    }

    fn is_ahead(&self, name: &str) -> bool {
        AHEAD.with(|held| held.borrow().contains(name))
    }

    fn leave_ahead(&self, bound: &str) {
        AHEAD.with(|held| held.borrow_mut().remove(bound));
    }

    fn comptimes_of(&self, at: i64) -> Vec<tree::ComptimeDeclared> {
        let file = self.files[at as usize];
        tree::comptime_declared_of(&file.program.items, &file.interner)
    }

    fn folded(
        &self,
        at: i64,
        value: &Expr,
        known: &impl Fn(&str) -> Option<Value>,
    ) -> Option<Value> {
        let reads = crate::assets::Reads::none();
        let so_far = |name: &str| known(name);
        crate::build_time::BuildTime::new(self.files[at as usize], self.files, &reads, &so_far)
            .evaluate(value)
            .ok()
    }

    fn ty_of(&self, at: i64, written: &crate::ast::Type) -> Ty {
        <Ty as crate::contracts::ty::TyOps>::from_ast(self.files[at as usize], written)
    }
}

thread_local! {
    /// The constants being worked out ahead, so that a ring of them ends.
    static AHEAD: std::cell::RefCell<BTreeSet<String>> =
        const { std::cell::RefCell::new(BTreeSet::new()) };
}

/// A run's lowering is built with its steps counted (ADR-321 D7).
fn counted() -> crate::emit::Build {
    crate::emit::Build {
        counts_steps: true,
        ..Default::default()
    }
}

/// `lowered`'s text and its source map's rows, as `comptime_driven` reads
/// them.
fn rows_of(lowered: crate::emit::Lowered) -> tree::ComptimeLowered {
    let rows = lowered
        .map
        .rows()
        .map(|(generated, byte, unit)| (generated as i64, byte as i64, unit as i64))
        .collect();
    tree::ComptimeLowered {
        rust: lowered.rust,
        rows,
    }
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
        held.borrow().as_ref().and_then(|ledger| {
            nikaia_std::tools::build_eval::bt_known_default(
                ledger,
                package.map(str::to_string),
                key,
                at as i64,
            )
        })
    })
}

/// The workshop defaults are compiled in now, where a build set one.
pub(crate) fn defaults_workshop() -> Option<std::sync::Arc<crate::grammar_run::Workshop>> {
    DEFAULTS.with(|held| held.borrow().clone())
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

// --- compiling and running -----------------------------------------------------
//
// **The bundle and the run are Nikaia's** (`tools/comptime_bundle.nika`,
// K3b): what stays here is the `Bundle` the other modules read, the state
// this process holds (the CPU level, the programs that did not compile, the
// toolchain's answers), Cargo's JSON, and the child's spawn with its progress
// note; each `pub(crate)` function is a forward.

use nikaia_std::rt::exec::block_on;
use nikaia_std::tools::comptime_bundle::{
    self as moved, ComptimeBundle, ComptimeBundleArtifacts, ComptimeBundleHost, ComptimeBundleRan,
};

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

impl From<ComptimeBundle> for Bundle {
    fn from(held: ComptimeBundle) -> Self {
        Bundle {
            dylib: held.dylib,
            rlibs: held.rlibs,
            deps: held.deps,
            stamp: held.stamp,
            libdir: held.libdir,
        }
    }
}

impl From<&Bundle> for ComptimeBundle {
    fn from(held: &Bundle) -> Self {
        ComptimeBundle {
            dylib: held.dylib.clone(),
            rlibs: held.rlibs.clone(),
            deps: held.deps.clone(),
            stamp: held.stamp.clone(),
            libdir: held.libdir.clone(),
        }
    }
}

/// Build the bundle, or say why it could not be built
/// (`comptime_bundle_bundle`): once per machine, under the user cache.
pub(crate) fn bundle(_workshop: &Path) -> Result<Bundle, String> {
    block_on(moved::comptime_bundle_bundle(&BundleHost))
        .map(Bundle::from)
        .map_err(|e| e.split().0.to_string())
}

/// **The bundle `nikaia run` links a program against**
/// (`comptime_bundle_run_bundle`), found from its record without starting
/// Cargo while what it was built from is unchanged.
pub(crate) fn run_bundle() -> Result<Bundle, String> {
    block_on(moved::comptime_bundle_run_bundle(&BundleHost))
        .map(Bundle::from)
        .map_err(|e| e.split().0.to_string())
}

/// **What `tools/comptime_bundle.nika` asks of the compiler** (K3b).
struct BundleHost;

impl ComptimeBundleHost for BundleHost {
    fn std_dir(&self) -> PathBuf {
        crate::sysroot::Sysroot::resolve().std_dir()
    }

    fn compiler(&self) -> String {
        env!("NIKAIA_COMPILER").to_string()
    }

    fn rustc_version(&self) -> String {
        orchestrator::project::rustc_version().to_string()
    }

    fn runtime_dependencies(&self, named: &str) -> Vec<(String, String)> {
        crate::project::runtime_dependencies_for(named)
    }

    fn user_cache_dir(&self) -> PathBuf {
        orchestrator::cache::Layout::user_cache_dir()
    }

    fn use_entry(&self, at: &Path) {
        orchestrator::cache::use_entry(at);
    }

    fn where_tools_start(&self) -> PathBuf {
        orchestrator::cache::Layout::where_tools_start()
    }

    fn artifacts(&self, messages: &str, named: &[String]) -> ComptimeBundleArtifacts {
        artifacts_in(messages, named)
    }

    fn toolchain(&self) -> String {
        toolchain().to_string()
    }

    fn target_libdir(&self) -> String {
        target_libdir().to_string()
    }

    fn cpu_level(&self) -> String {
        CPU_LEVEL
            .lock()
            .map(|held| held.clone())
            .unwrap_or_default()
    }

    fn not_built(&self, key: &str) -> Option<String> {
        NOT_BUILT
            .lock()
            .ok()
            .and_then(|held| held.get(key).cloned())
    }

    fn remember_not_built(&self, key: &str, detail: &str) {
        if let Ok(mut held) = NOT_BUILT.lock() {
            held.insert(key.to_string(), detail.to_string());
        }
    }

    fn unique_beside(&self, path: &Path) -> PathBuf {
        unique_beside(path)
    }

    fn run_child(
        &self,
        binary: &Path,
        level: &str,
        libraries: &[PathBuf],
        name: &str,
    ) -> Result<ComptimeBundleRan, Box<dyn std::error::Error>> {
        let mut libraries = libraries.to_vec();
        if let Some(held) = std::env::var_os(LIBRARY_PATH) {
            libraries.extend(std::env::split_paths(&held));
        }
        let child = Command::new(binary)
            .env("RUST_BACKTRACE", "0")
            .env(nikaia_std::target::LEVEL_VAR, level)
            .env(LIBRARY_PATH, std::env::join_paths(libraries)?)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let ran = waited(child, name)?;
        Ok(ComptimeBundleRan {
            success: ran.status.success(),
            stdout: String::from_utf8_lossy(&ran.stdout).to_string(),
            stderr: String::from_utf8_lossy(&ran.stderr).to_string(),
            status: ran.status.to_string(),
        })
    }
}

/// **The library and the crates Cargo's JSON messages name**: the
/// `nikaia_bundle` target's dynamic library and each named crate's `rlib`.
fn artifacts_in(messages: &str, named: &[String]) -> ComptimeBundleArtifacts {
    let mut dylib = None;
    let mut rlibs: Vec<(String, PathBuf)> = Vec::new();
    for line in messages.lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if message["reason"] != "compiler-artifact" {
            continue;
        }
        let files = message["filenames"].as_array().cloned().unwrap_or_default();
        let files = files.iter().filter_map(|f| f.as_str()).map(PathBuf::from);
        match message["target"]["name"].as_str() {
            Some(name) if named.iter().any(|n| n == name) => {
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
    ComptimeBundleArtifacts { dylib, rlibs }
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

/// **Compile and run one build-time program** (`comptime_bundle_run`), or
/// hand back what it said when it ran before: its standard output, or
/// `Wall::Refused` with what it wrote where it stopped. Keyed on the program,
/// the toolchain and the bundle (D4).
pub(crate) fn run(at: &Path, bundle: &Bundle, program: &str, name: &str) -> Result<String, Wall> {
    block_on(moved::comptime_bundle_run(
        at,
        &ComptimeBundle::from(bundle),
        program,
        name,
        &BundleHost,
    ))
    .map_err(|e| e.split().0)
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

/// Written only when it changed, so that Cargo's freshness keeps working,
/// and whole or not at all (`comptime_bundle_write_if_changed`).
pub(crate) fn write_if_changed(path: &Path, text: &str) -> Result<(), String> {
    block_on(moved::comptime_bundle_write_if_changed(
        path,
        text,
        &unique_beside(path),
    ))
    .map_err(|e| e.split().0.to_string())
}

/// A path beside `path` that no other build writes at the same time.
pub(crate) fn unique_beside(path: &Path) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.{n}.partial", std::process::id()));
    path.with_file_name(name)
}
