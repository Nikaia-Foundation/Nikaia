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
        match &self.package {
            Some(package) => format!("{package}::{name}"),
            None => name.to_string(),
        }
    }
}

/// A package this program depends on: the name a `use` writes, and where it is
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D15). Declared in
/// `tools/project_members.nika`, which finds them (P4a).
pub use nikaia_std::tools::project_members::ProjectMembersDependency as Dependency;

/// Every file of the package the entry belongs to, entry first.
///
/// **The directory decides, not the `use` lines** (ADR-286 D1). Every `.nika`
/// beside the entry takes part, whether or not anything names it - which is what
/// makes moving a declaration from one file to another housekeeping rather than a
/// change to the package's surface.
///
/// Entry first because the entry is where `fn main` may be written, and the rest
/// sorted by file name: the order files are emitted in has to be a function of
/// the source tree alone (Part III 13.5's determinism), and a directory listing
/// is not sorted anywhere.
pub fn collect(entry: &Path) -> Result<Vec<Unit>> {
    collect_with(entry, &[])
}

/// The same, with the packages this program depends on
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D15).
///
/// A dependency's files come **after** the program's own, in the order the
/// manifest sorted its names, so the emitted file stays a function of the source
/// tree (Part III 13.5). Each is read from the package's `src/`, because a
/// dependency is a project and that is where a project keeps its files
/// (Part III, 13.1).
pub fn collect_with(entry: &Path, dependencies: &[Dependency]) -> Result<Vec<Unit>> {
    let mut names: BTreeSet<String> = dependencies.iter().map(|d| d.name.clone()).collect();
    // **A Rust crate is a package a `use` may name** (Part III 15.2's `use
    // image`), under the name a call writes it by (ADR-290 D1).
    names.extend(foreign_beside(entry));
    let mut units = package_at(entry, None, &names, &BTreeMap::new())?;

    // **Read, and not emitted** ([ADR-286](../../../docs/specification/adr/adr-286.md)
    // D1). A dependency is its own crate now, so its files are Cargo's to
    // compile - but its *surface* is this program's to check against, and a
    // package's surface is what its files declare. So they arrive here tagged
    // with the name this program reaches them by, the ledger and the checks see
    // them, and `Program::emit` writes none of them.
    //
    // **And its own `use` lines are checked against its own manifest keys**,
    // not against nothing: a package that depends on a package is a package
    // (D3), and the level below it is its business rather than a rule it broke.
    for dependency in dependencies {
        let src = dependency.root.join(SRC);
        if !src.is_dir() {
            return Err(crate::diagnostics::refuse(format!(
                "`{}` is a dependency by path, but {} doesn't exist.\n\
                 A package is a directory with a `{SRC}/` folder in it.",
                dependency.name,
                src.display()
            )));
        }
        units.extend(package_at(
            &src.join(ENTRY),
            Some(&dependency.name),
            &dependency.reachable,
            &dependency.renames,
        )?);
    }
    Ok(units)
}

/// One package: the directory an entry sits in, checked as a namespace of its
/// own.
fn package_at(
    entry: &Path,
    package: Option<&str>,
    reachable: &BTreeSet<String>,
    renames: &BTreeMap<String, String>,
) -> Result<Vec<Unit>> {
    let base = entry
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut beside: Vec<PathBuf> = std::fs::read_dir(&base)
        .with_context(|| format!("cannot read {}", base.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "nika"))
        // The entry by its name: `x.nika` and the `./x.nika` the directory
        // lists are one file (#774).
        .filter(|path| path.file_name() != entry.file_name())
        .collect();
    beside.sort();

    // A library needs no `main.nika`: what a package offers is its public
    // surface, and an entry point is what a *program* has.
    let mut units = match entry.is_file() {
        true => vec![read_unit(
            entry,
            package,
            reachable,
            renames,
            Standing::InProject,
        )?],
        false => Vec::new(),
    };
    for path in beside {
        units.push(read_unit(
            &path,
            package,
            reachable,
            renames,
            Standing::InProject,
        )?);
    }
    one_namespace(&units)?;
    Ok(units)
}

/// Where a project keeps its sources and what its entry point is called
/// (Part III, 13.1).
const SRC: &str = "src";
const ENTRY: &str = "main.nika";

/// The entry and nothing beside it - a `.nika` file compiled on its own.
///
/// `nikaia lower` outside a project is not a package: a directory of loose examples is
/// a directory of programs, and compiling one of them must not pull in the other
/// ten. A package is a directory **of a project**, which is what a `nikaia.toml`
/// declares.
pub fn collect_one(entry: &Path) -> Result<Vec<Unit>> {
    Ok(vec![read_unit(
        entry,
        None,
        &BTreeSet::new(),
        &BTreeMap::new(),
        Standing::Alone,
    )?])
}

/// **Whether a file has a project to declare its dependencies in**
/// ([ADR-260](../../../docs/specification/adr/adr-260.md) D2): what a `use` of
/// a package nobody declares is told depends on it. A file of a project is
/// missing a line in its `nikaia.toml`; a file outside one has no `nikaia.toml`
/// for the line to be missing from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Standing {
    InProject,
    Alone,
}

fn read_unit(
    path: &Path,
    package: Option<&str>,
    reachable: &BTreeSet<String>,
    renames: &BTreeMap<String, String>,
    standing: Standing,
) -> Result<Unit> {
    let source =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    // `with_context` and not `anyhow!("{e}")`: formatting the error into a string
    // loses its type, and the type is what says this is a refusal of the program
    // rather than a failure of this compiler (`diagnostics::Refused`).
    // A parse error is a finding, rendered here where the path is known; any
    // other failure keeps the path as its context.
    let parsed =
        parser::parse_to_ast(&source).map_err(
            |error| match crate::diagnostics::refused_finding(&error) {
                Some(finding) => crate::diagnostics::refuse(
                    crate::diagnostics::render_finding(
                        finding,
                        &path.display().to_string(),
                        &source,
                    )
                    .trim_end()
                    .to_string(),
                ),
                None => error.context(path.display().to_string()),
            },
        )?;
    check_imports(&parsed, path, reachable, standing)?;
    let mut parsed = parsed;
    parsed.package = package.map(str::to_string);
    Ok(Unit {
        package: package.map(str::to_string),
        renames: renames.clone(),
        path: path.to_path_buf(),
        source,
        parsed,
    })
}

/// **Two files of one package may not declare the same name** (ADR-286 D1).
///
/// One namespace, so two `Row`s in it is an error rather than a rule about which
/// of them a line means. Refused here and not left to `rustc`: it would report a
/// duplicate definition against the generated file, which is exactly what
/// Part III C.1 forbids - and it would name a line the user never wrote.
///
/// **Two files and not one**
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D6). A name declared
/// twice in *one* file is `NK1148` in the checker, which every build runs -
/// where this runs only for a build that has a manifest. This kept the one-file
/// case for as long as it was the only rule there was, and served it badly:
/// a sentence about two files, naming one of them twice.
fn one_namespace(units: &[Unit]) -> Result<()> {
    let files: Vec<(&Path, &Parsed)> = units
        .iter()
        .map(|unit| (unit.path.as_path(), &unit.parsed))
        .collect();
    declared_once(&files)
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

/// The names a file declares: what `one_namespace` counts.
///
/// A method is not among them - it belongs to its type and two types may each
/// have a `len`. An `impl` block declares nothing of its own, and neither does a
/// **rule** of a grammar: it belongs to its grammar and is reached as
/// `Json::value` ([ADR-140](../../../docs/specification/adr/adr-140.md) D3).
///
/// **A `trait` and a `grammar` are among them**
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D5), and were not:
/// `trait Foo` beside `struct Foo` was accepted through every path, including
/// this one, and `rustc` answered about the generated file.
fn declared_names(parsed: &Parsed) -> Vec<String> {
    nikaia_std::tools::modules::declared_names(&parsed.interner, &parsed.program.items)
}

/// **Every `use` in a file, against the packages it may name**
/// ([ADR-286](../../../docs/specification/adr/adr-286.md) D11, D13, D14).
///
/// `use` makes a package reachable and brings **no name** in, so there are four
/// things to say no to and one thing to accept. Each message names the way out,
/// because a rule a reader cannot act on is an obstacle (Part III, C.2):
///
/// * a **path** other than `std`'s — `use net::http` — which is either a nested
///   package (there is no such thing) or an attempt to import a name;
/// * a name that is a **file beside this one**, the shape that worked before
///   ADR-286 D1 made a package a directory;
/// * a name **no dependency declares**, which is D4 read from the other side: a
///   prefix must be introduced, and this is the introduction that names nothing;
/// * the **same name twice**, which is D5 — two packages under one name is an
///   error rather than a rule about which of them wins.
///
/// The braced and glob forms (`use pool::{Conn}`, `use pool::*`) are parse errors
/// at the brace and the star, so D2's sentence about them is not reachable from
/// here; that is [ADR-286](../../../docs/specification/adr/adr-286.md)'s
/// remaining piece and it belongs in the grammar.
fn check_imports(
    parsed: &Parsed,
    at: &Path,
    reachable: &BTreeSet<String>,
    standing: Standing,
) -> Result<()> {
    let refused = nikaia_std::tools::modules::a_use_refused(
        &parsed.interner,
        &parsed.program.items,
        &at.display().to_string(),
        reachable,
        matches!(standing, Standing::Alone),
        &|package: &str| {
            at.parent()
                .map(|dir| dir.join(format!("{package}.nika")))
                .is_some_and(|path| path.is_file())
        },
    );
    match refused {
        Some(message) => Err(crate::diagnostics::refuse(message)),
        None => Ok(()),
    }
}

/// The units of one package, by file name and the SHA-256 of their bytes
/// ([ADR-100](../../../docs/specification/adr/adr-100.md) D3).
///
/// **The same digest the cache key uses** ([ADR-021](../../../docs/specification/adr/adr-021.md)),
/// because the record says so and because a build that hashed the same file
/// twice with two digests would be paying twice to disagree with itself.
fn sources_of<'a>(units: impl IntoIterator<Item = &'a Unit>) -> BTreeMap<String, String> {
    units
        .into_iter()
        .map(|unit| {
            let name = unit
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                // A unit always comes from a file; a path that ends in nothing
                // is not one this compiler read, and naming it by its whole
                // spelling is better than dropping it out of the table.
                .unwrap_or_else(|| unit.path.to_string_lossy().to_string());
            (
                name,
                orchestrator::cache::sha256_hex(unit.source.as_bytes()),
            )
        })
        .collect()
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

/// One package's contracts: the ledger it ships where that may be believed, and
/// the inference over its own units where it may not
/// ([ADR-100](../../../docs/specification/adr/adr-100.md) D1 and D3).
///
/// **Believed is the fast path and the *correct* one**, not merely the quick
/// one. A package's own build had its own dependencies in view and this one does
/// not (ADR-286 D21), so an answer derived here can only be the same or worse -
/// and two builds that derive the same function differently is the divergence
/// D1 exists to make impossible.
///
/// **And it is never believed against its own sources.** The hashes in the
/// header say which files the entries are an answer about; where one does not
/// match, that package is derived again here. A ledger with no `[sources]` at
/// all is treated as not matching, which is the fail-closed direction
/// ([ADR-010](../../../docs/specification/adr/adr-010.md) D1): *nothing was
/// recorded* must not read as *nothing changed*.
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
    let sources = sources_of(units);

    if let Some(dependency) = dependency
        && let Some(shipped) = shipped_ledger(&dependency.root)
        && shipped.stale_against(&sources).is_empty()
    {
        return (
            shipped.published(&dependency.reachable),
            None,
            Default::default(),
        );
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

/// A package's committed `nikaia.contracts`, where it has one this compiler can
/// read.
///
/// **A ledger that does not parse is not an error here.** It is a generated file
/// a person may have edited or an older compiler may have written, and the
/// answer to both is the same as to a stale one: derive this package again. A
/// build that failed because a *cache* was unreadable would be a worse build
/// than one that is slower.
fn shipped_ledger(root: &Path) -> Option<crate::contracts::Ledger> {
    crate::contracts::Ledger::read_beside(&root.join("nikaia.contracts"))
}

/// The descriptions of the crates the project around `entry` declares
/// (`contracts/<crate>.contracts`), or none outside a project.
/// The Rust crates the manifest of the project an entry belongs to declares,
/// under the name a program writes them by; none outside a project.
fn foreign_beside(entry: &Path) -> BTreeSet<String> {
    entry
        .ancestors()
        .skip(1)
        .find(|dir| dir.join("nikaia.toml").is_file())
        .and_then(|root| crate::manifest::Manifest::read(&root.join("nikaia.toml")).ok())
        .map(|manifest| manifest.foreign_crates())
        .unwrap_or_default()
}

fn described_beside(entry: &Path) -> crate::contracts::Ledger {
    entry
        .ancestors()
        .skip(1)
        .find(|dir| dir.join("nikaia.toml").is_file())
        .map(|root| crate::project::foreign_of(root).descriptions)
        .unwrap_or_else(crate::contracts::Ledger::blank)
}
