// crates/nikaia/src/modules.rs
//
// A program is more than one file, and **a package is a directory**
// (Part I, 9.1; ADR-286 D1).
//
// The files of a package see one another with no `use` at all: they share one
// namespace, so a name declared in any of them can be written in any other. What
// the package offers outward is whatever says `pub`, in whichever file it is
// declared - which is what keeps a library's internal file layout from being its
// public surface.
//
// What this does is **resolution**, and resolution is the one thing ADR-296 D17
// said Stage 0 does not do: the lowering is name for name, and nothing looks a
// name up. That stays true of the *emitter* - what changes is that the compiler
// knows which files take part, and hands the emitter each of them in turn.
//
// The shape it produces is the plainest one the language below has: **one crate
// root**, with every file's items in it. No `mod`, no `use super::*`, nothing to
// resolve - because one namespace in this language is one namespace in that one.
// Two files declaring the same name is refused here rather than left to `rustc`,
// which would report it about a file nobody wrote (Part III, C.1).
//
// **Which files** is the directory and not the `use` lines. A `use` names another
// package (ADR-286 D10), and depending on one is not built (ADR-286), so a
// `use` that is not `std`'s is refused with what to do instead.

use crate::contracts::LedgerOps;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::ast::Item;
use crate::parser::{self, Parsed};
use nikaia_std::error::Thrown;
use nikaia_std::rt::exec::block_on;
use nikaia_std::tools::modules::{
    self as collecting, ModulesHost, ModulesLedgerHost, ModulesRefusal, ModulesUnit,
};
use std::cell::RefCell;

/// One file of a package.
pub struct Unit {
    /// The package this file belongs to, as a **consumer** would write it -
    /// and `None` for every file of the program being built, whose names are
    /// the program's own.
    ///
    /// It is not the file stem any more. A file is not a unit of naming
    /// (ADR-286 D1), so nothing about a name says which file it came from; the
    /// field stays because a package that arrives by path will fill it in
    /// (ADR-286 D15), and that is the same qualification one level up.
    pub package: Option<String>,
    /// The package names **inside this file** that are another word for a
    /// package this build already has a word for - see
    /// [`Dependency::renames`]. Empty for every file of the program itself,
    /// whose words are this build's by definition.
    pub renames: BTreeMap<String, String>,
    pub path: PathBuf,
    pub source: String,
    pub parsed: Parsed,
}

impl Unit {
    /// How a consumer writes a name from this file: `http::serve` for another
    /// package, and plain `serve` inside the program's own.
    pub fn qualify(&self, name: &str) -> String {
        // In Nikaia (`tools/modules.nika`, M3a).
        collecting::modules_qualify(self.package.clone(), name)
    }
}

/// A package this program depends on: the name a `use` writes, and where it is
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D15). Declared in
/// `tools/project_members.nika`, which finds them (P4a).
pub use nikaia_std::tools::project_members::ProjectMembersDependency as Dependency;

/// Every file of the package the entry belongs to, entry first
/// (`modules_collect_with`, `tools/modules.nika`, M3a).
pub fn collect(entry: &Path) -> Result<Vec<Unit>> {
    collect_with(entry, &[])
}

/// The same, with the packages this program depends on
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D15), their files
/// after the program's own (`modules_collect_with`).
pub fn collect_with(entry: &Path, dependencies: &[Dependency]) -> Result<Vec<Unit>> {
    let host = Parsing::default();
    host.units(block_on(collecting::modules_collect_with(
        entry,
        dependencies,
        &host,
    )))
}

/// The entry and nothing beside it - a `.nika` file compiled on its own
/// (`modules_collect_one`): outside a project a directory is not a package.
pub fn collect_one(entry: &Path) -> Result<Vec<Unit>> {
    let host = Parsing::default();
    host.units(block_on(collecting::modules_collect_one(entry, &host)))
}

/// **What collecting a program's files asks of the compiler** (M3a): the
/// files parsed and kept here, by number, until the walk in
/// `tools/modules.nika` has found them all; and the refusal of the one that
/// did not parse, which the walk throws as `ModulesRefusal::Parse`.
#[derive(Default)]
struct Parsing {
    parsed: RefCell<Vec<Option<Parsed>>>,
    failed: RefCell<Option<anyhow::Error>>,
}

impl ModulesHost for Parsing {
    fn parse_unit(&self, path: &Path, source: &str, package: Option<String>) -> Option<i64> {
        // A parse error is a finding, rendered here where the path is known;
        // any other failure keeps the path as its context. Either way it
        // keeps its type, which is what says this is a refusal of the program
        // rather than a failure of this compiler (`diagnostics::Refused`).
        match parser::parse_to_ast(source) {
            Ok(mut parsed) => {
                parsed.package = package;
                let mut kept = self.parsed.borrow_mut();
                kept.push(Some(parsed));
                Some(kept.len() as i64 - 1)
            }
            Err(error) => {
                let refused = match crate::diagnostics::refused_finding(&error) {
                    Some(finding) => crate::diagnostics::refuse(
                        crate::diagnostics::render_finding(
                            finding,
                            &path.display().to_string(),
                            source,
                        )
                        .trim_end()
                        .to_string(),
                    ),
                    None => error.context(path.display().to_string()),
                };
                *self.failed.borrow_mut() = Some(refused);
                None
            }
        }
    }

    fn declared(&self, unit: i64) -> Vec<String> {
        declared_names(self.kept(unit).as_ref().expect(KEPT))
    }

    fn use_refused(
        &self,
        unit: i64,
        at: &Path,
        reachable: &BTreeSet<String>,
        alone: bool,
    ) -> Option<String> {
        let parsed = self.kept(unit);
        let parsed = parsed.as_ref().expect(KEPT);
        nikaia_std::tools::modules::a_use_refused(
            &parsed.interner,
            &parsed.program.items,
            &at.display().to_string(),
            reachable,
            alone,
            &|package: &str| {
                at.parent()
                    .map(|dir| dir.join(format!("{package}.nika")))
                    .is_some_and(|path| path.is_file())
            },
        )
    }

    fn foreign_crates(&self, path: &Path) -> Vec<String> {
        crate::manifest::Manifest::read(path)
            .map(|manifest| manifest.foreign_crates().into_iter().collect())
            .unwrap_or_default()
    }
}

/// A file the walk asks about is kept until `Parsing::units` takes it.
const KEPT: &str = "a parsed file is kept until the units are built";

impl Parsing {
    fn kept(&self, unit: i64) -> std::cell::Ref<'_, Option<Parsed>> {
        std::cell::Ref::map(self.parsed.borrow(), |kept| &kept[unit as usize])
    }

    /// The walk's files as `Unit`s, each with the file parsed for it; or its
    /// refusal, as the error it was before the move.
    fn units(
        &self,
        found: std::result::Result<Vec<ModulesUnit>, Thrown<ModulesRefusal>>,
    ) -> Result<Vec<Unit>> {
        let found = found.map_err(|thrown| match thrown.split().0 {
            ModulesRefusal::Parse => self
                .failed
                .borrow_mut()
                .take()
                .unwrap_or_else(|| anyhow::anyhow!("a file did not parse")),
            ModulesRefusal::Refused(why) => crate::diagnostics::refuse(why),
            ModulesRefusal::Unreadable { what, why } => anyhow::anyhow!(why).context(what),
        })?;
        let mut kept = self.parsed.borrow_mut();
        Ok(found
            .into_iter()
            .map(|unit| Unit {
                package: unit.package,
                renames: unit.renames,
                path: unit.path,
                source: unit.source,
                parsed: kept[unit.parsed as usize].take().expect(KEPT),
            })
            .collect())
    }
}

/// The same rule for any set of files that is one package: a project's, and
/// `std`'s toolchain package (`sysroot::lower_tools`), whose files were lowered
/// into one namespace without it - two private `a_call`s in two files were
/// read as one, and a call of the one met the other's parameters (found
/// moving `trust`'s walk into Nikaia, #125).
pub(crate) fn declared_once(files: &[(&Path, &Parsed)]) -> Result<()> {
    // In Nikaia (`tools/modules.nika`, ADR-294, #125).
    let paths: Vec<String> = files
        .iter()
        .map(|(path, _)| path.display().to_string())
        .collect();
    let declared: Vec<Vec<String>> = files
        .iter()
        .map(|(_, parsed)| declared_names(parsed))
        .collect();
    match nikaia_std::tools::modules::declared_once(&paths, &declared) {
        Some(message) => Err(crate::diagnostics::refuse(message)),
        None => Ok(()),
    }
}

/// The names a file declares: what one package's namespace counts
/// (`declared_names`, `tools/modules.nika`).
fn declared_names(parsed: &Parsed) -> Vec<String> {
    nikaia_std::tools::modules::declared_names(&parsed.interner, &parsed.program.items)
}

/// The runs of units that belong to one package, in the order they were read.
///
/// The units arrive grouped (`collect_with`), so this is a walk and not a sort.
fn groups(units: &[Unit]) -> Vec<std::ops::Range<usize>> {
    // In Nikaia (`tools/modules.nika`).
    nikaia_std::tools::modules::groups(&packages_of(units))
        .into_iter()
        .map(|(from, to)| from as usize..to as usize)
        .collect()
}

/// Each unit's package, `None` for the program's own: what the Nikaia half
/// of the assembly reads of a unit.
fn packages_of(units: &[Unit]) -> Vec<Option<String>> {
    units.iter().map(|unit| unit.package.clone()).collect()
}

/// One package's contracts: the ledger it ships where that may be believed
/// (`modules_believed`), and the inference over its own units where it may
/// not ([ADR-100](../../../docs/specification/adr/adr-100.md) D1 and D3).
fn package_ledger(
    units: &[Unit],
    dependency: Option<&Dependency>,
    library: &crate::contracts::Ledger,
) -> crate::contracts::Ledger {
    package_ledger_noted(units, dependency, library).0
}

/// [`package_ledger`], and what the `sync` inference learned for the build to
/// say (ADR-288 D29, D32) - nothing for a shipped ledger, which was inferred by
/// the package's own build and is read rather than derived.
fn package_ledger_noted(
    units: &[Unit],
    dependency: Option<&Dependency>,
    library: &crate::contracts::Ledger,
) -> (crate::contracts::Ledger, crate::contracts::sync::Noted) {
    let (own, _, noted) = package_ledger_checked(units, dependency, library);
    (own, noted)
}

/// [`package_ledger_noted`], with the ledger pass's checked units where it ran
/// one (`None` for a shipped ledger).
fn package_ledger_checked(
    units: &[Unit],
    dependency: Option<&Dependency>,
    library: &crate::contracts::Ledger,
) -> (
    crate::contracts::Ledger,
    Option<Vec<crate::check::Checked>>,
    crate::contracts::sync::Noted,
) {
    let paths: Vec<PathBuf> = units.iter().map(|u| u.path.clone()).collect();
    let texts: Vec<String> = units.iter().map(|u| u.source.clone()).collect();
    let sources = collecting::modules_sources_of(&paths, &texts, &LedgerHost);

    if let Some(dependency) = dependency
        && let Some(believed) = collecting::modules_believed(dependency, &sources, &LedgerHost)
    {
        return (believed, None, Default::default());
    }

    let parsed: Vec<&Parsed> = units.iter().map(|u| &u.parsed).collect();
    let (mut own, checked, noted) = crate::contracts::Ledger::infer_package_noted(&parsed, library);
    // **What these entries are an answer about** (D3), recorded here because
    // this is the only place that has both the answer and the files it came
    // from - and recorded for every package, so that the ledger a dependency's
    // own build writes says what it was derived from.
    own.sources = sources;
    (own, Some(checked), noted)
}

/// **What a package's ledger asks of the compiler** (M3a): SHA-256 and
/// reading a ledger.
struct LedgerHost;

impl ModulesLedgerHost for LedgerHost {
    fn sha256_hex(&self, text: &str) -> String {
        orchestrator::cache::sha256_hex(text.as_bytes())
    }

    fn read_beside(&self, path: &Path) -> Option<crate::contracts::Ledger> {
        crate::contracts::Ledger::read_beside(path)
    }
}

/// A whole program: one ledger, one Rust file, one source map.
pub struct Program {
    pub units: Vec<Unit>,
    /// **The name the entry file is said under, where it is not its path**
    /// (#510): `nikaia run f.nika` builds a copy in the cache, and a panic's
    /// location names `f.nika`, the file the author has open. `None` names
    /// every unit by its path.
    pub entry_shown_as: Option<String>,
    /// **What the manifest's described crates say about their boundary**
    /// ([ADR-290](../../../docs/specification/adr/adr-290.md) D1,
    /// [ADR-290](../../../docs/specification/adr/adr-290.md) D20): read by the
    /// emitter for one question, whether a call into one can fail. Empty
    /// outside a project and for a project that declares no crate.
    pub described: crate::contracts::Ledger,
    /// One ledger for the project (Part III, 13.5), with every module's entries
    /// under the name a caller writes.
    pub contracts: crate::contracts::Ledger,
    /// **Each dependency's ledger as that package itself wrote it**, before
    /// `absorb` qualified its keys and the types inside them.
    ///
    /// `contracts` above is the right ledger for the *program's* files and the
    /// wrong one for a package's own: absorbing turns `Request` into
    /// `http::Request`, in the key and in every signature that names it, and the
    /// package's own file writes the bare word as its author must
    /// ([ADR-286](../../../docs/specification/adr/adr-286.md) D11 gives no import
    /// to write). Checked against the absorbed one, a package's `[H: Handler]`
    /// found no trait and `answer(connection, handler)` was told its own
    /// `fn(Request) -> Response` is not a `fn(http::Request) -> http::Response` —
    /// a correct program refused, twice over
    /// ([Part III C.4](../../../docs/specification/30-nikaia-tooling.md)).
    ///
    /// **Kept rather than reconstructed.** Un-qualifying the absorbed ledger
    /// would be `absorb` run backwards, and an inverse that is nearly right is
    /// worse than none: this is the very ledger the package was inferred with, so
    /// there is nothing to get wrong.
    pub as_its_own: std::collections::BTreeMap<String, crate::contracts::Ledger>,
    /// **The program's `test` blocks**, in the order `nikaia test` numbers
    /// them ([ADR-269](../../../docs/specification/adr/adr-269.md) D1). Empty
    /// in every build but a test build, which is the only one that keeps them.
    pub tests: Vec<TestCase>,
    /// **The program's own functions that pause only where their lambdas do**
    /// ([ADR-288](../../../docs/specification/adr/adr-288.md) D29), by key, with
    /// the code parameters that decide: each could promise `sync(f)`. Read by
    /// [`Program::could_promise`].
    pub paused_by_code: BTreeMap<String, Vec<String>>,
    /// **Why each of the program's own functions that can pause does**
    /// ([ADR-288](../../../docs/specification/adr/adr-288.md) D32): the calls
    /// that lead to a statement that pauses, its unit an index into
    /// [`Program::units`].
    pub pauses: BTreeMap<String, crate::contracts::sync::Pause>,
    /// **The ledger pass over the program's own files, against `std` alone**
    /// (`Ledger::infer_package_checked`), kept where that is what it ran
    /// against - no described crate, no dependency - so the trust join of
    /// [`Program::emit_reading`] reads it rather than inferring the ledger a
    /// second time (#727). `None` where the library held more.
    pub own_checked: Option<Vec<crate::check::Checked>>,
    /// **What the check of a unit found, by its index in [`Program::units`]**,
    /// kept where that check asked exactly what the emitter's own would
    /// (`check::propagation_against`), so the emitter reads it rather than
    /// walking the unit again (#727).
    pub checked_facts: BTreeMap<usize, crate::check::CheckFacts>,
}

/// One `test "…" { … }` of the program, as `nikaia test` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    /// The title as written, escapes and all.
    pub title: String,
    pub path: PathBuf,
    /// The line the block starts on.
    pub line: usize,
}

/// The function a test build's entry dispatches from, before it takes `main`'s
/// place.
const DISPATCH: &str = "__nikaia_tests";

impl Program {
    /// Parse every file, and infer the contracts of all of them together.
    ///
    /// Together, and not one at a time: `sync` is a fixpoint over the call
    /// graph (ADR-288), and a call graph that stops at a file boundary would
    /// give a different answer depending on which file was looked at first.
    /// 13.5 makes the ledger a pure function of the source tree, which is a
    /// promise about the tree and not about each file in it.
    pub fn read(entry: &Path) -> Result<Program> {
        Self::of(collect(entry)?, &[], &crate::contracts::Ledger::blank())
    }

    /// The same, with the packages this program depends on
    /// ([ADR-286](../../../docs/specification/adr/adr-286.md) D15).
    pub fn read_with(entry: &Path, dependencies: &[Dependency]) -> Result<Program> {
        Self::of(
            collect_with(entry, dependencies)?,
            dependencies,
            &described_beside(entry),
        )
    }

    /// The entry compiled on its own - what `nikaia lower` outside a project is
    /// (see [`collect_one`]).
    pub fn read_one(entry: &Path) -> Result<Program> {
        Self::of(collect_one(entry)?, &[], &crate::contracts::Ledger::blank())
    }

    /// **The program as `nikaia test` builds it**
    /// ([ADR-269](../../../docs/specification/adr/adr-269.md) D1): every
    /// `test` block of the program's own files is a function, and the entry's
    /// `main` is replaced by one that runs the test whose number it is given
    /// as its first argument - so the program is compiled once and each test
    /// runs in a process of its own. A package that is a dependency keeps no
    /// tests; nor does an entry with no test, which is then the program as
    /// every other build sees it.
    pub fn read_for_tests(
        entry: &Path,
        dependencies: &[Dependency],
        in_project: bool,
    ) -> Result<Program> {
        let (units, described) = match in_project {
            true => (collect_with(entry, dependencies)?, described_beside(entry)),
            false => (collect_one(entry)?, crate::contracts::Ledger::blank()),
        };
        let (units, tests) = with_tests_as_functions(units)?;
        let mut program = Self::of(units, dependencies, &described)?;
        program.tests = tests;
        Ok(program)
    }

    fn of(
        mut units: Vec<Unit>,
        dependencies: &[Dependency],
        described: &crate::contracts::Ledger,
    ) -> Result<Program> {
        // **A `test` block is compiled only by `nikaia test`** (ADR-269 D1),
        // which has turned every one it runs into a function by now: what is
        // left is left out.
        for unit in &mut units {
            unit.parsed
                .program
                .items
                .retain(|item| !matches!(item.node, Item::Test { .. }));
        }
        // **A type's rule is a fact only where no file of its package, and
        // no package using it, can break it** (ADR-369 D3).
        for range in groups(&units) {
            let mut files: Vec<(&mut crate::parser::Parsed, &str)> = units[range]
                .iter_mut()
                .map(|u| (&mut u.parsed, u.source.as_str()))
                .collect();
            crate::type_rules::across_files(&mut files);
        }
        // **The dependencies first, and the program's own package last**
        // (ADR-100 D5): in Nikaia (`modules_program_of`, ADR-294, #125).
        let mut reaches = BTreeMap::new();
        let mut their_renames = BTreeMap::new();
        for dependency in dependencies.iter().rev() {
            reaches.insert(dependency.name.clone(), dependency.reachable.clone());
            their_renames.insert(dependency.name.clone(), dependency.renames.clone());
        }
        let host = ProgramHost {
            units: &units,
            dependencies,
            checked: RefCell::new(None),
            paused_by_code: RefCell::new(BTreeMap::new()),
            pauses: RefCell::new(BTreeMap::new()),
        };
        let parts = collecting::modules_program_of(
            &host,
            described,
            &packages_of(&units),
            &units.iter().map(|u| u.renames.clone()).collect::<Vec<_>>(),
            &reaches,
            &their_renames,
        );
        let own_checked = match (parts.own_checked_from, host.checked.into_inner()) {
            (Some(from), Some((at, checked))) if at == from => Some(checked),
            _ => None,
        };
        let paused_by_code = host.paused_by_code.into_inner();
        let pauses = host.pauses.into_inner();
        Ok(Program {
            units,
            entry_shown_as: None,
            described: crate::contracts::Ledger::blank(),
            contracts: parts.contracts,
            as_its_own: parts.as_its_own,
            tests: Vec::new(),
            paused_by_code,
            pauses,
            own_checked,
            checked_facts: BTreeMap::new(),
        })
    }

    /// Each unit's path as a message says it: the entry under
    /// [`Program::entry_shown_as`] where that is set.
    pub fn shown_paths(&self) -> Vec<String> {
        // In Nikaia (`tools/modules.nika`, ADR-294, #125).
        let paths: Vec<String> = self
            .units
            .iter()
            .map(|u| u.path.display().to_string())
            .collect();
        nikaia_std::tools::modules::shown_paths(&paths, self.entry_shown_as.as_deref())
    }

    /// **[ADR-288](../../../docs/specification/adr/adr-288.md) D29's note**:
    /// the program's own `pub` functions that could promise more about pausing
    /// than they do (`tools/modules.nika`).
    pub fn could_promise(&self) -> String {
        nikaia_std::tools::modules::could_promise(
            &self.contracts,
            &self.as_its_own,
            &self.paused_by_code,
        )
    }

    /// **[ADR-288](../../../docs/specification/adr/adr-288.md) D32's warning**
    /// for every `pub` function of the program's own that could not pause in
    /// the committed ledger and now can (`tools/modules.nika`).
    pub fn lost_sync(&self, committed: &crate::contracts::Ledger) -> String {
        nikaia_std::tools::modules::lost_sync(
            &self.contracts,
            &self.as_its_own,
            &self.pauses,
            committed,
            &|finding: &crate::check::Finding, unit: i64| {
                let unit = &self.units[unit as usize];
                crate::diagnostics::render_finding(
                    finding,
                    &unit.path.display().to_string(),
                    &unit.source,
                )
            },
        )
    }

    /// **The program's contracts as its own files are checked against them**
    /// ([ADR-288](../../../docs/specification/adr/adr-288.md) D28), in
    /// `tools/modules.nika`. [`Program::contracts`] stays the ledger the
    /// lowering reads and the one written beside the program.
    pub fn contracts_as_promised(&self) -> crate::contracts::Ledger {
        nikaia_std::tools::modules::contracts_as_promised(&self.contracts, &self.as_its_own)
    }

    /// The **packages** this program reaches, by the name a `use` writes
    /// (`tools/modules.nika`).
    pub fn package_names(&self) -> std::collections::BTreeSet<String> {
        nikaia_std::tools::modules::package_names(&packages_of(&self.units))
    }

    /// Whether this is one file, in which case nothing about the build changes.
    pub fn is_single_file(&self) -> bool {
        nikaia_std::tools::modules::is_single_file(self.units.len() as i64)
    }

    /// Every source that took part, in the order they are emitted - which is
    /// what the cache key has to cover (Part III, 13.1: "the SHA256 of each
    /// `.nika` source that took part").
    pub fn sources(&self) -> Vec<&str> {
        self.units.iter().map(|u| u.source.as_str()).collect()
    }

    /// The whole package as one Rust file.
    ///
    /// **One crate root per package** (ADR-286 D1). One namespace in this
    /// language is one namespace in the language below, so the program's own
    /// files need no `mod`, no `use super::*` and nothing for a reader to
    /// resolve. `pub` becomes `pub`, which is what publishes a name out of the
    /// package - the same move ADR-017 D2 made with `html::Render`: put the rule
    /// where the language below can act on it rather than re-implementing it
    /// here.
    ///
    /// **A dependency is a `mod` at that root** (D2), named by the manifest key,
    /// so `http::serve()` in Nikaia is `http::serve()` in Rust and nothing
    /// resolved it (ADR-296 D17). Each opens with `use super::*` under an
    /// `#[allow]`, which brings in the preamble the root wrote - the allow is
    /// because the import is **ours**, and a module that happens to use nothing
    /// from the preamble would otherwise produce a warning about a line no Nikaia
    /// source maps to (Part III, C.1).
    ///
    /// The program's own files go **first**, the entry first among them, because
    /// the entry is the file that may declare `main` and a reader opens the
    /// generated file at the top.
    pub fn emit(&self, build: crate::emit::Build) -> Result<crate::emit::Lowered> {
        self.emit_reading(build, &crate::assets::Reads::none())
    }

    /// The same, told what this build may read while it builds
    /// ([ADR-310](../../../docs/specification/adr/adr-310.md)).
    ///
    /// **A second entry point and not a field on `Build`**: `Build` is the
    /// machine and the switches, copied freely, and what a build may read is a
    /// fact about the invocation with a lifetime on it. A caller with nothing
    /// to say passes [`crate::assets::Reads::none`], which is D1.
    pub fn emit_reading(
        &self,
        build: crate::emit::Build,
        reads: &crate::assets::Reads,
    ) -> Result<crate::emit::Lowered> {
        // In Nikaia (`modules_emit_reading`, ADR-294, #125): the trust
        // decision, the header and the loop over the program's own units.
        let host = EmitHost {
            program: self,
            build,
            reads,
            beside: self.units.iter().map(|unit| &unit.parsed).collect(),
            map: RefCell::new(crate::emit::SourceMap::default()),
            published: RefCell::new(BTreeMap::new()),
            failed: RefCell::new(None),
        };
        let rust = collecting::modules_emit_reading(
            &host,
            &packages_of(&self.units),
            &self.shown_paths(),
            &self.sources(),
        );
        let rust = match rust {
            Ok(rust) => rust,
            Err(error) => {
                return Err(host
                    .failed
                    .into_inner()
                    .unwrap_or_else(|| anyhow::anyhow!("{error}")));
            }
        };
        Ok(crate::emit::Lowered {
            rust,
            map: host.map.into_inner(),
            published: host.published.into_inner(),
        })
    }
}

/// **What assembling a program's ledgers asks of the compiler** (M3b): the
/// inference over a run of units, and what it learned, kept here for
/// [`Program`].
struct ProgramHost<'a> {
    units: &'a [Unit],
    dependencies: &'a [Dependency],
    /// The checked units of the last own group, by the place of its first unit.
    checked: RefCell<Option<(i64, Vec<crate::check::Checked>)>>,
    paused_by_code: RefCell<BTreeMap<String, Vec<String>>>,
    pauses: RefCell<BTreeMap<String, collecting::ModulesPause>>,
}

impl collecting::ModulesProgramHost for ProgramHost<'_> {
    fn empty_ledger(&self) -> crate::contracts::Ledger {
        crate::contracts::Ledger::empty()
    }

    fn std_library(&self) -> crate::contracts::Ledger {
        crate::contracts::std_library()
    }

    fn package_ledger(&self, from: i64, to: i64) -> crate::contracts::Ledger {
        let group = &self.units[from as usize..to as usize];
        package_ledger(
            group,
            self.dependencies
                .iter()
                .find(|d| Some(&d.name) == group[0].package.as_ref()),
            &crate::contracts::std_library(),
        )
    }

    fn own_ledger(
        &self,
        from: i64,
        to: i64,
        library: &crate::contracts::Ledger,
    ) -> crate::contracts::Ledger {
        let (own, checked, noted) =
            package_ledger_checked(&self.units[from as usize..to as usize], None, library);
        *self.checked.borrow_mut() = checked.map(|checked| (from, checked));
        self.paused_by_code.borrow_mut().extend(noted.by_code);
        self.pauses
            .borrow_mut()
            .extend(noted.why.into_iter().map(|(name, mut pause)| {
                pause.unit += from;
                (name, pause)
            }));
        own
    }
}

/// **What emitting a program asks of the compiler** (M3b): the emitter over
/// one unit, and the source map and published names it fills, kept here for
/// [`crate::emit::Lowered`]; the refusal of the unit that did not lower.
struct EmitHost<'a> {
    program: &'a Program,
    build: crate::emit::Build,
    reads: &'a crate::assets::Reads,
    /// **Every file, for every file.** What a `comptime` came to is the
    /// checker's answer and the emitter writes it, so the two have to be
    /// asked the same question — a unit lowered against fewer files than it
    /// was checked against would refuse an item the check accepted.
    beside: Vec<&'a Parsed>,
    map: RefCell<crate::emit::SourceMap>,
    published: RefCell<BTreeMap<String, crate::prove::Published>>,
    failed: RefCell<Option<anyhow::Error>>,
}

impl EmitHost<'_> {
    fn own(&self) -> Vec<&Parsed> {
        self.program
            .units
            .iter()
            .filter(|u| u.package.is_none())
            .map(|u| &u.parsed)
            .collect()
    }
}

impl collecting::ModulesEmitHost for EmitHost<'_> {
    fn trust(&self) -> crate::contracts::Provenance {
        let package = self.own();
        // Each question the package's ledger walk asks, in its file.
        let files = self
            .program
            .units
            .iter()
            .filter(|u| u.package.is_none())
            .map(|u| (u.path.clone(), u.source.clone()))
            .collect();
        crate::proofs::with_units(files, || match &self.program.own_checked {
            Some(checked) => {
                crate::contracts::trust::analyse_package_checked(&package, std_ledger(), checked)
            }
            None => crate::contracts::trust::analyse_package(&package, std_ledger()),
        })
        .provenance
    }

    fn keyed_functions(&self, package: &str) -> Vec<String> {
        let files: Vec<&Parsed> = self
            .program
            .units
            .iter()
            .filter(|u| u.package.as_deref() == Some(package))
            .map(|u| &u.parsed)
            .collect();
        crate::contracts::maps::keyed_functions(&files, &self.program.contracts, package)
    }

    fn own_names(&self) -> BTreeSet<String> {
        names_in(&self.own())
    }

    fn barrier_names(&self, all: bool) -> BTreeSet<String> {
        match all {
            true => crate::emit::barrier_names(&self.beside, &self.program.described),
            false => crate::emit::barrier_names(&self.own(), &self.program.described),
        }
    }

    fn preamble(&self) -> String {
        let needs = self
            .program
            .units
            .iter()
            .fold(crate::emit::Needs::none(), |acc, u| {
                acc.join(&crate::proofs::for_unit(
                    &u.path,
                    &u.source,
                    u.package.is_some(),
                    || crate::emit::needs_of(&u.parsed),
                ))
            });
        needs.preamble()
    }

    fn emit_unit(
        &self,
        at: i64,
        provenance: crate::contracts::Provenance,
        offset: i64,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let at = at as usize;
        let unit = &self.program.units[at];
        let body = crate::proofs::for_unit(&unit.path, &unit.source, false, || {
            crate::emit::emit_module_body_at(
                &unit.parsed,
                &self.beside,
                self.build,
                provenance,
                &self.program.contracts,
                &self.program.described,
                // The entry is the only file ADR-303 D4's generated `fn main`
                // may be written from.
                at == 0,
                self.reads,
                self.program.checked_facts.get(&at).cloned(),
            )
        })
        // **A refusal from the lowering gets its line here**
        // ([ADR-171](../../../docs/specification/adr/adr-171.md) D2), which
        // is the one place that has both the byte and the file: the
        // lowering knows the statement and not the path, and whoever
        // catches the error at the top knows neither.
        .map_err(|error| match crate::diagnostics::refusal_at(&error) {
            Some((byte, message)) => {
                crate::diagnostics::refuse(crate::diagnostics::render_refusal(
                    &message,
                    byte,
                    &unit.path.display().to_string(),
                    &unit.source,
                ))
            }
            None => error,
        });
        match body {
            Ok(body) => {
                self.map
                    .borrow_mut()
                    .extend(body.map.placed(offset as usize, at));
                self.published.borrow_mut().extend(body.published);
                Ok(body.rust)
            }
            Err(error) => {
                let said = format!("{error:#}");
                *self.failed.borrow_mut() = Some(error);
                Err(said.into())
            }
        }
    }

    fn has_main(&self) -> bool {
        let entry = &self.program.units[0].parsed;
        entry.program.items.iter().any(|item| {
            matches!(&item.node, crate::ast::Item::Fn { name: Some(name), .. }
                if entry.text(*name) == "main")
        })
    }

    fn map_rows(&self) -> Vec<(i64, i64, i64)> {
        self.map
            .borrow()
            .rows()
            .map(|(at, from, unit)| (at as i64, from as i64, unit as i64))
            .collect()
    }
}

/// **Every `test` block of the program's own files, as a function**, and the
/// entry's `main` replaced by the one that runs them by number
/// ([ADR-269](../../../docs/specification/adr/adr-269.md) D1).
///
/// The dispatcher is Nikaia, appended to the entry's text and parsed with it,
/// so it goes through every check and the one lowering a program does: a test
/// body may pause and may fail, and `main` is where both are already answered
/// (ADR-303 D4). A test's function `throws`, so a failure that leaves its body
/// leaves `main` - which prints it and ends the process unsuccessfully.
fn with_tests_as_functions(mut units: Vec<Unit>) -> Result<(Vec<Unit>, Vec<TestCase>)> {
    use nikaia_std::tools::modules;
    let own: Vec<usize> = (0..units.len())
        .filter(|&at| units[at].package.is_none())
        .collect();
    let count: i64 = own
        .iter()
        .map(|&at| modules::modules_test_count(&units[at].parsed.program.items))
        .sum();
    if count == 0 || units.is_empty() || units[0].package.is_some() {
        return Ok((units, Vec::new()));
    }

    // The entry first, because it is parsed again with the dispatcher in it.
    let entry = &units[0];
    let cli = modules::modules_cli_word(&entry.parsed.interner, &entry.parsed.program.items);
    let mut text = entry.source.clone();
    text.push('\n');
    // The dispatcher's own text is Nikaia's (`tools/modules.nika`).
    text.push_str(&modules::test_dispatcher(
        cli.as_deref().unwrap_or("cli"),
        cli.is_none(),
        count,
        DISPATCH,
    ));
    let mut parsed = parser::parse_to_ast(&text)
        .with_context(|| format!("{} with its tests", entry.path.display()))?;
    let interner = &parsed.interner;
    modules::modules_dispatcher_as_main(&mut parsed.program.items, interner, DISPATCH, &|text| {
        interner.intern_string(text)
    });
    units[0].parsed = parsed;

    let mut tests = Vec::new();
    for at in own {
        let unit = &mut units[at];
        let interner = &unit.parsed.interner;
        let found = modules::modules_tests_as_functions(
            &mut unit.parsed.program.items,
            &unit.source,
            tests.len() as i64,
            &|text| interner.intern_string(text),
        );
        for (title, line) in found {
            tests.push(TestCase {
                title,
                path: unit.path.clone(),
                line: line as usize,
            });
        }
    }
    Ok((units, tests))
}

fn std_ledger() -> &'static crate::contracts::Ledger {
    crate::contracts::std_ledger()
}

/// Every name an expression of these files writes (`modules_names_in`,
/// `tools/modules.nika`).
fn names_in(files: &[&crate::parser::Parsed]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for parsed in files {
        nikaia_std::tools::modules::modules_names_in(
            &parsed.interner,
            &parsed.program.items,
            &mut names,
        );
    }
    names
}

/// The descriptions of the crates the project around `entry` declares
/// (`contracts/<crate>.contracts`), or none outside a project
/// (`modules_project_root`).
fn described_beside(entry: &Path) -> crate::contracts::Ledger {
    collecting::modules_project_root(entry)
        .map(|root| crate::project::foreign_of(&root).descriptions)
        .unwrap_or_else(crate::contracts::Ledger::blank)
}
