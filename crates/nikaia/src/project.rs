//! A project build: `nikaia.toml` becomes a `Cargo.toml`, and `cargo` runs it.
//!
//! [ADR-002](../../../docs/specification/adr/adr-002.md) D1. Dependency
//! resolution and linking are solved problems, and a language that cannot reach
//! crates.io starts with no ecosystem - so the toolchain does not resolve
//! versions, does not link, and does not own a build graph. It translates a
//! manifest, hands the result to Cargo, and intercepts the one step Cargo
//! cannot do: turning a `.nika` file into something `rustc` accepts.
//!
//! The interception is `RUSTC_WORKSPACE_WRAPPER`, and the choice of *that*
//! variable rather than `RUSTC_WRAPPER` is the load-bearing part. It is applied
//! to **workspace members only**, so a crate from crates.io is compiled by the
//! real `rustc` with nothing in between - which is what makes "native Rust
//! dependencies pass through unchanged" true of the compilation and not only of
//! the manifest.
//!
//! What is generic about all this lives in
//! [`orchestrator::project`] ([ADR-003](../../../docs/specification/adr/adr-003.md)
//! D2): rendering a manifest, running `cargo`, splitting a `rustc` command
//! line. What is here is Nikaia's half - which file is the entry, which
//! switches were chosen, and what the lowering does.

use crate::contracts::LedgerOps;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use nikaia_std::rt::exec::block_on;
use nikaia_std::tools::project_check;
use nikaia_std::tools::project_lower;
use nikaia_std::tools::project_members::{
    self, ProjectMembersForeignHost, ProjectMembersHost, ProjectMembersManifest,
    ProjectMembersMember, ProjectMembersRefusal, project_members_canonical as canonical,
};
use nikaia_std::tools::project_settings::{self, ProjectSettingsRefusal, ProjectWords};
use nikaia_std::tools::project_tests::{self, ProjectTestsOutcome as Outcome};
pub use nikaia_std::tools::project_tests::{OutputTest, ProgramTest};

use anyhow::{Context, Result, anyhow};
use orchestrator::cache::{Artifacts, Cache, Choices, Layout, Lockfile};
use orchestrator::project::{
    Cargo, CargoProject, CrateKind, Invocation, Package, Profile, Workspace,
    record_extra_dependencies, resolved_versions, toolchain_can_build_for, toolchain_is_new_enough,
    write_if_changed,
};

use crate::contracts::{Ledger, sync};
use crate::emit::Build;
use crate::manifest::{Dependency, Manifest};
use crate::sysroot::{Codegen, Sysroot};
use crate::{assets, check, diagnostics, modules, refuse, refused};

/// The extension a Nikaia source carries, and therefore the argument the
/// wrapper is looking for in a `rustc` command line.
pub const EXTENSION: &str = "nika";

/// The entry point of a project, relative to its root (Part III 13.1).
pub const ENTRY: &str = "src/main.nika";

/// Set on the wrapper's own process, and the only thing that tells it apart
/// from an ordinary run: Cargo owns the wrapper's argument list, so there is no
/// flag or subcommand available to say which mode this is.
pub const WRAPPER_MARKER: &str = "NIKAIA_RUSTC_WRAPPER";
// The switches' variables (`TARGET_VAR` .. `CPU_VAR`) are
// `project_settings.nika`'s, which writes and reads them.
const GEN_DIR_VAR: &str = "NIKAIA_GEN_DIR";
/// What the program's crate name carries beyond its package's in the shared
/// target directory (#495), for the wrapper to take off again.
const PROGRAM_TAG_VAR: &str = "NIKAIA_PROGRAM_TAG";
const NO_CACHE_VAR: &str = "NIKAIA_NO_CACHE";
/// **The file a loose `nikaia run` was given**, for the wrapper to name the
/// entry by (#510): it lowers the copy in the cache, and a panic's location is
/// written then.
const SHOWN_AS_VAR: &str = "NIKAIA_SHOWN_AS";

/// How `--allow-read-from-list` reaches the `rustc` wrapper
/// ([ADR-310](../../docs/specification/adr/adr-310.md) D5, D9).
///
/// The wrapper is a **second process**, started by Cargo, and it lowers the
/// same file a second time — so it has to be told the same list or the two
/// halves would disagree about which files this build may read. The value is
/// an absolute path, because the wrapper runs in a directory Cargo chose.
///
/// **And it is how D6 holds.** Every member of the workspace is lowered under
/// one environment, so a dependency's `asset("…")` is checked against the list
/// of the build that is running rather than against anything the dependency
/// ships.
const READS_VAR: &str = "NIKAIA_ALLOW_READ_FROM_LIST";
/// **The system libraries' linker flags**, as the build formed them from
/// `pkg-config` (Part III 13.4, ADR-324 D3, D4), one per `\u{1f}`-separated
/// field: handed to `rustc` for the program and nothing else.
const LINK_VAR: &str = "NIKAIA_LINK_FLAGS";

/// A file the wrapper appends one line to per `rustc` it is handed: the crate
/// Cargo named, and whether this compiler lowered it or passed it through.
///
/// It exists because D1's central claim is otherwise invisible. A crate from
/// crates.io compiled by the real `rustc` and one passed *through* this wrapper
/// to the real `rustc` produce identical output, so "dependencies are untouched"
/// cannot be checked by looking at what was built - only by asking what was
/// asked for. Off unless the variable is set.
pub const TRACE_VAR: &str = "NIKAIA_WRAPPER_TRACE";

/// The build switches, resolved: flag over manifest over built-in default.
///
/// One value threaded through rather than three fields re-read at each use.
/// Every setting is kept both as the word that named it and as the parsed
/// thing, because the cache key and the diagnostics want the word (ADR-021 D5
/// hashes what was asked for) and the emitter wants the value.
#[derive(Debug, Clone)]
pub struct Settings {
    pub build: Build,
    /// The words (`project_settings.nika`, P1).
    pub words: ProjectWords,
}

/// A build setting's word refused, as the error it was before the move (P1).
fn refusal(thrown: nikaia_std::error::Thrown<ProjectSettingsRefusal>) -> anyhow::Error {
    let ProjectSettingsRefusal::Refused(why) = thrown.split().0;
    anyhow!(why)
}

/// A build setting's word refused as a statement about the build
/// (`refuse!`, as `--cpu` was before the move).
fn refused(thrown: nikaia_std::error::Thrown<ProjectSettingsRefusal>) -> anyhow::Error {
    let ProjectSettingsRefusal::Refused(why) = thrown.split().0;
    crate::diagnostics::refuse(why)
}

/// What `project_settings.nika` asks of the compiler: the environment,
/// what `native` is on this machine, a hash and a hard link.
struct SettingsHost;

impl project_settings::ProjectSettingsHost for SettingsHost {
    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn native_cpu(&self, triple: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
        crate::sysroot::native_cpu(triple).map_err(|e| format!("{e:#}").into())
    }

    fn hard_link(&self, from: &Path, to: &Path) -> bool {
        std::fs::hard_link(from, to).is_ok()
    }
}

/// `--optimization`'s words to what they decide
/// ([ADR-306](../../docs/specification/adr/adr-306.md) D14):
/// `project_settings::optimizations`, its refusal as an error.
pub fn optimizations(
    words: &str,
) -> Result<(crate::bounds::BoundsChecks, crate::bounds::OverflowChecks)> {
    project_settings::optimizations(words).map_err(refusal)
}

pub use project_settings::{prove_through_loops, skip_free_at_exit};

/// `[build] symbol-prefix` (ADR-284 D13): a C identifier, or the build refuses
/// it, naming the character.
fn symbol_prefix(word: &str) -> Result<String> {
    let mut chars = word.chars();
    let first_is_fine = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if let Some(bad) = word
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '_'))
        .or_else(|| (!first_is_fine).then(|| word.chars().next().unwrap_or(' ')))
    {
        return Err(anyhow!(
            "`symbol-prefix` `{word}` is not a C identifier: `{bad}` cannot stand there. Write \
             letters, digits and `_`, not beginning with a digit."
        ));
    }
    Ok(word.to_string())
}

impl Settings {
    /// Whether this build makes a library C calls (ADR-284 D4).
    pub fn library(&self) -> bool {
        self.words.artifact == "c-library"
    }

    /// Resolve every switch before anything is read, so a mistyped one fails on
    /// its own account rather than after a compile (ADR-037 D5).
    pub fn resolve(
        manifest: &Manifest,
        target: Option<&str>,
        user_parallelism: Option<&str>,
    ) -> Result<Settings> {
        Settings::of(
            project_settings::project_settings_resolve(
                &|key, default| manifest.setting(key, None, default).to_string(),
                manifest.package_name().map(str::to_string),
                target.map(str::to_string),
                user_parallelism.map(str::to_string),
                crate::emit::default_target().name().to_string(),
            )
            .map_err(|e| anyhow!("{e}"))?,
        )
    }

    /// The parsed values beside the checked `words`: the symbol prefix, the
    /// target's words and the optimizations, in that order, after the words
    /// `project_settings` checked.
    fn of(words: ProjectWords) -> Result<Settings> {
        symbol_prefix(&words.symbol_prefix)?;
        let mut build = Build::parse(
            &words.target,
            &words.user_parallelism,
            &words.reentrancy_check,
        )?;
        (build.bounds, build.overflow) = optimizations(&words.optimization)?;
        build.skip_free_at_exit = skip_free_at_exit(&words.optimization);
        build.prove_through_loops = prove_through_loops(&words.optimization);
        build.called_from_c = words.artifact == "c-library";
        Ok(Settings { build, words })
    }

    /// **The command line's `--optimization` over the manifest's**
    /// ([ADR-306](../../docs/specification/adr/adr-306.md) D1). Nothing a
    /// program means depends on it, so a single build may say it, as it may
    /// say `--target`.
    /// **The command line's `--refuted-claims` over the manifest's**
    /// ([ADR-269](../../docs/specification/adr/adr-269.md) D8): a single build
    /// may let through a claim a newer prover refutes, as it may name another
    /// target.
    /// **The command line's `--unproved-claims` over the manifest's**
    /// ([ADR-380](../../docs/specification/adr/adr-380.md) D2), the same way.
    pub fn refutations(&mut self, refuted: Option<&str>, unproved: Option<&str>) -> Result<()> {
        (self.words.refuted_claims, self.words.unproved_claims) =
            project_settings::project_settings_refutations(
                &self.words,
                &refuted.map(str::to_string),
                &unproved.map(str::to_string),
            )
            .map_err(refusal)?;
        Ok(())
    }

    /// **`--cpu=LEVEL`** ([ADR-002](../../docs/specification/adr/adr-002.md)
    /// D6): one of the target's levels, or the build fails naming them. The
    /// baseline is the same build as no level at all.
    pub fn choose_cpu(&mut self, level: Option<&str>) -> Result<()> {
        let Some(level) = level else {
            return Ok(());
        };
        let target = self.build.target;
        let levels = crate::sysroot::cpu_levels(target.triple())?;
        self.words.cpu = project_settings::project_settings_chosen_cpu(
            level.to_string(),
            &levels,
            target.name(),
        )
        .map_err(refused)?;
        Ok(())
    }

    /// The CPU level as a dimension of the compiled `std` (ADR-002 D6): empty
    /// for the baseline, the level's name, and for `native` the CPU `rustc`
    /// finds on this machine, never the word.
    pub fn cpu_key(&self) -> Result<String> {
        project_settings::project_settings_cpu_key(
            &self.words,
            self.build.target.triple(),
            &SettingsHost,
        )
        .map_err(|e| anyhow!("{e}"))
    }

    /// What Cargo is told for the CPU level (ADR-002 D6, ADR-325).
    pub fn cpu_config(&self) -> Vec<String> {
        project_settings::cpu_config(&self.words, self.build.target)
    }

    /// **`--cpu` against the user's `RUSTFLAGS`** (#753, ADR-002 D6, Part III
    /// 13.3). Cargo ignores the `rustflags` of `cpu_config` when `RUSTFLAGS` or
    /// `CARGO_ENCODED_RUSTFLAGS` is set, so then the child gets
    /// `CARGO_ENCODED_RUSTFLAGS`: the user's flags, then `-C target-cpu=L` last
    /// (the level wins, the flags still apply). `None` where nothing is needed.
    pub fn cpu_rustflags(&self) -> Result<Option<String>> {
        project_settings::project_settings_cpu_rustflags(
            &self.words,
            self.build.target.triple(),
            &SettingsHost,
        )
        .map_err(|e| anyhow!("{e}"))
    }

    pub fn optimize(&mut self, flags: &[String]) -> Result<()> {
        if flags.is_empty() {
            return Ok(());
        }
        let words = flags.join(",");
        (self.build.bounds, self.build.overflow) = optimizations(&words)?;
        self.build.skip_free_at_exit = skip_free_at_exit(&words);
        self.build.prove_through_loops = prove_through_loops(&words);
        self.words.optimization = words;
        Ok(())
    }

    /// The words, for the wrapper. Cargo owns the wrapper's arguments, so the
    /// environment is the only channel the switches can travel down - and they
    /// travel already resolved, because a second resolution is a second chance
    /// to disagree.
    fn as_env(&self) -> Vec<(String, OsString)> {
        project_settings::project_settings_as_env(&self.words)
            .into_iter()
            .map(|(name, word)| (name, OsString::from(word)))
            .collect()
    }

    /// What the driver put in the environment. Absent means the wrapper was run
    /// by hand, and the built-in defaults are the honest answer.
    fn from_env() -> Result<Settings> {
        Settings::of(
            project_settings::project_settings_from_env(
                crate::emit::default_target().name(),
                &SettingsHost,
            )
            .map_err(refusal)?,
        )
    }

    /// The cache's view of this build (ADR-021 D5): the key is
    /// `project_settings::choices_key`'s, and the backend is `"rust"` because it
    /// names **the lowering that produced the artifact**, not the flag the user
    /// typed, and a bare invocation *is* `--backend rust`
    /// ([ADR-004](../../docs/specification/adr/adr-004.md) D1), so the two
    /// spellings share one entry - which is correct, because it is one lowering.
    /// The dimension itself stays in [`orchestrator::cache::Key::build`] (D5, D7)
    /// so that the day a second backend caches, the literal here becomes that
    /// backend's name and the two cannot collide.
    pub fn choices(&self) -> Choices {
        let cpu_key = match self.words.cpu {
            Some(_) => self.cpu_key().unwrap_or_default(),
            None => String::new(),
        };
        let build = &self.build;
        Choices::new(
            project_settings::choices_key(
                &self.words,
                build.bounds,
                build.overflow,
                build.skip_free_at_exit,
                build.prove_through_loops,
                &cpu_key,
            ),
            "rust",
        )
    }

    /// What a panic does on this machine (ADR-037 D1), in Cargo's vocabulary.
    fn panic_strategy(&self) -> String {
        project_settings::panic_strategy(self.build.target)
    }
}

/// Everything one lowering produced.
pub struct Lowered {
    pub rust: String,
    pub ledger: String,
    /// What the ledger was derived from, for `nikaia.derived` beside it
    /// ([ADR-251](../../docs/specification/adr/adr-251.md) D1).
    pub derived: String,
    /// Every `.nika` file that took part, entry first. The wrapper puts these
    /// back into Cargo's dependency info, or a changed module would not
    /// rebuild.
    pub sources: Vec<PathBuf>,
    /// Whether the build cache answered instead of the emitter.
    pub reused: bool,
    /// **What the author could promise and has not**
    /// ([ADR-288](../../docs/specification/adr/adr-288.md) D29): the note
    /// [`modules::Program::could_promise`] writes, empty where there is none.
    /// Handed back rather than printed, because the `rustc` wrapper lowers too
    /// and only the command a person ran should say it - once.
    pub notes: String,
    /// **The prover's answers as this lowering left them**
    /// ([ADR-270](../../docs/specification/adr/adr-270.md) D21, D22), for
    /// [`write_proofs`]: `None` on a cache hit, where nothing was asked, and
    /// outside a project, where there is no file.
    pub proofs: Option<crate::proofs::Book>,
    /// **`<package>.h`**, where this build makes a library and the package
    /// exports entry points (ADR-284 D12).
    pub header: Option<String>,
    /// Its Python binding, which `nikaia bind python` writes (ADR-284 D26).
    pub python: Option<String>,
    /// Its N-API module in C, which `nikaia bind node` writes (ADR-284 D28).
    pub node: Option<String>,
}

/// **The packages a project depends on by path**
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D15): the walk is
/// `project_members_packages_of` in `tools/project_members.nika` (P4a); this
/// hands it the manifest and turns a refusal back into its error.
pub fn packages_of(
    manifest: &Manifest,
    root: &Path,
    announce: bool,
) -> Result<Vec<modules::Dependency>> {
    let host = Manifests::default();
    project_members::project_members_packages_of(&manifest_view(manifest), root, announce, &host)
        .map_err(|thrown| host.error(thrown))
}

/// What `tools/project_members.nika` reads of a manifest (P4a): `Manifest`
/// and `toml` stay Rust.
fn manifest_view(manifest: &Manifest) -> ProjectMembersManifest {
    ProjectMembersManifest {
        name: manifest.package_name().map(str::to_string),
        keys: manifest.dependencies().keys().cloned().collect(),
        paths: manifest
            .dependencies()
            .iter()
            .filter_map(|(name, value)| match value {
                Dependency::Path(path) => Some((name.clone(), path.clone())),
                _ => None,
            })
            .collect(),
        foreign: manifest.foreign_crates().into_iter().collect(),
        has_build: manifest.has_build_section(),
    }
}

/// The manifests the walk read, by the path it read them at, kept for the
/// members it returns; and the error of the one that did not read, which
/// the walk throws as `ProjectMembersRefusal::Manifest`.
#[derive(Default)]
struct Manifests {
    read: std::cell::RefCell<BTreeMap<PathBuf, Manifest>>,
    failed: std::cell::RefCell<Option<anyhow::Error>>,
}

impl ProjectMembersHost for Manifests {
    fn manifest_at(&self, path: &Path) -> Option<ProjectMembersManifest> {
        match Manifest::read(path) {
            Ok(manifest) => {
                let view = manifest_view(&manifest);
                self.read.borrow_mut().insert(path.to_path_buf(), manifest);
                Some(view)
            }
            Err(error) => {
                *self.failed.borrow_mut() = Some(error);
                None
            }
        }
    }
}

impl Manifests {
    /// A refusal of the walk, as the error it was before the move.
    fn error(&self, thrown: nikaia_std::error::Thrown<ProjectMembersRefusal>) -> anyhow::Error {
        match thrown.split().0 {
            ProjectMembersRefusal::Manifest => self
                .failed
                .borrow_mut()
                .take()
                .unwrap_or_else(|| anyhow!("a manifest did not read")),
            ProjectMembersRefusal::Refused(why) => refused!("{why}"),
            ProjectMembersRefusal::Unreadable { what, why } => anyhow!(why).context(what),
        }
    }
}

/// One package of a build, and everything needed to generate a crate for it
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D19): the fields of
/// `ProjectMembersMember` (`tools/project_members.nika`, where they are
/// described), and the manifest the generated workspace reads.
#[derive(Debug, Clone)]
pub struct Member {
    pub name: String,
    pub root: PathBuf,
    pub manifest: Manifest,
    pub entry: PathBuf,
    pub dependencies: Vec<modules::Dependency>,
}

/// The members in an order where every package comes after the packages it
/// depends on: `project_members_dependencies_first`.
fn dependencies_first(members: &[Member]) -> Vec<usize> {
    let parts: Vec<ProjectMembersMember> = members
        .iter()
        .map(|m| ProjectMembersMember {
            name: m.name.clone(),
            root: m.root.clone(),
            entry: m.entry.clone(),
            dependencies: m.dependencies.clone(),
        })
        .collect();
    project_members::project_members_dependencies_first(&parts)
        .into_iter()
        .map(|at| at as usize)
        .collect()
}

/// Every package of this build, the entry first
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D19): the walk is
/// `project_members_members_of`; each member gets the manifest it read.
pub fn members_of(entry_manifest: &Manifest, entry_root: &Path) -> Result<Vec<Member>> {
    let host = Manifests::default();
    let members = project_members::project_members_members_of(
        &manifest_view(entry_manifest),
        entry_root,
        &host,
    )
    .map_err(|thrown| host.error(thrown))?;
    let read = host.read.into_inner();
    members
        .into_iter()
        .enumerate()
        .map(|(at, member)| {
            let path = member.root.join("nikaia.toml");
            let manifest = match (at, read.get(&path)) {
                (0, _) => entry_manifest.clone(),
                (_, Some(manifest)) => manifest.clone(),
                (_, None) => Manifest::read(&path)?,
            };
            Ok(Member {
                name: member.name,
                root: member.root,
                manifest,
                entry: member.entry,
                dependencies: member.dependencies,
            })
        })
        .collect()
}

/// A Rust dependency's table with a relative `path` resolved against the
/// project root ([ADR-286](../../docs/specification/adr/adr-286.md) D17).
///
/// **Absolute, because the file this is written into is not where the author
/// is.** The generated member manifest lives at
/// `target/nikaia/build/<package>/Cargo.toml`
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D19), and a path relative
/// to *that* is a number a person derives from a build layout. The same
/// manifest already writes `nikaia-std` absolute for the same reason.
///
/// A table with no `path` is handed back unchanged, which is every `version`
/// and `git` dependency there is.
fn rooted(value: &toml::Value, root: &Path) -> toml::Value {
    let Some(table) = value.as_table() else {
        return value.clone();
    };
    let Some(declared) = table.get("path").and_then(toml::Value::as_str) else {
        return value.clone();
    };
    let at = Path::new(declared);
    if at.is_absolute() {
        return value.clone();
    }
    let mut table = table.clone();
    table.insert(
        "path".to_string(),
        toml::Value::String(canonical(&root.join(at)).display().to_string()),
    );
    toml::Value::Table(table)
}

pub fn lower(
    input: &Path,
    settings: &Settings,
    no_cache: bool,
    packages: &[modules::Dependency],
) -> Result<Lowered> {
    lower_reading(input, settings, no_cache, packages, None)
}

/// **What a build may read, and where it compiles a parser**
/// ([ADR-310](../../docs/specification/adr/adr-310.md) D4 and D5,
/// [ADR-177](../../docs/specification/adr/adr-177.md) D2).
///
/// **One function rather than two constructions**, because both of this
/// build's lowerings have to answer the same question. The second one is
/// `Project::report`'s, which rebuilds the source map when the backend had
/// something to say — and when it was handed `Reads::none()` instead, every
/// `comptime` that read a file or ran a grammar evaluated to nothing on that
/// pass, so the emitter refused an item the build had already lowered. What
/// reached the user was this compiler's own internal sentence in place of the
/// backend's message, which is the opposite of what `report` exists for.
///
/// A list that cannot be read fails the build on its own account: a flag that
/// names a file and is quietly treated as *no list* would turn a build that
/// meant to read into one that says a path is not named, which is the wrong
/// sentence about the wrong thing.
fn reads_for(layout: &Layout, allowlist: Option<&Path>) -> Result<assets::Reads> {
    let reads = match allowlist {
        Some(list) => assets::Reads::with(&layout.root, assets::Allowlist::read(list)?),
        None => assets::Reads::at(&layout.root),
    };
    // **Where a grammar's parser is compiled** (issue #178). Beside
    // the cache rather than under the root, which is the rule that file already
    // keeps: a loose `.nika` outside a project has nothing written next to it.
    Ok(reads.building_in(layout.store.with_file_name("build-time")))
}

/// The same, told which allowlist is in effect
/// ([ADR-310](../../docs/specification/adr/adr-310.md) D5).
///
/// `None` is D1, and D1 is what every caller that does not pass the flag gets:
/// a build given no list reads nothing while it builds.
/// **A file and the project it belongs to, read as `nikaia build` reads it**
/// ([ADR-260](../../docs/specification/adr/adr-260.md) D1, D2).
///
/// Inside a project the file is a file of that project: its package is the
/// directory it sits in and its dependencies are the ones the manifest
/// declares, resolved by [`packages_of`] exactly as a build resolves them.
/// Outside one it is standalone and has none - which is what its `use` lines
/// are then told (`modules::collect_one`).
/// **What a build of one file may read and where it compiles build-time
/// code**: the same [`reads_for`] its lowering uses, for a report that has to
/// answer as the lowering did.
pub fn reads_for_file(input: &Path, allowlist: Option<&Path>) -> Result<assets::Reads> {
    reads_for(&Layout::resolve(input), allowlist)
}

pub fn read_file(input: &Path) -> Result<(modules::Program, Vec<modules::Dependency>)> {
    let packages = packages_of_file(input)?;
    match Layout::resolve(input).in_project {
        false => Ok((modules::Program::read_one(input)?, packages)),
        true => Ok((modules::Program::read_with(input, &packages)?, packages)),
    }
}

/// [`read_file`]'s dependencies without the program: for a caller that hands
/// them to [`lower_reading`], which reads the program itself, so the files
/// are not parsed and their ledger inferred twice.
pub fn packages_of_file(input: &Path) -> Result<Vec<modules::Dependency>> {
    let layout = Layout::resolve(input);
    if !layout.in_project {
        return Ok(Vec::new());
    }
    let manifest = Manifest::read(&layout.root.join("nikaia.toml"))?;
    // Not announced: `Manifest::find` already read this manifest for the
    // settings, and a note said twice is noise.
    packages_of(&manifest, &layout.root, false)
}

/// **The project `nikaia run <file>` builds** ([ADR-260](../../docs/specification/adr/adr-260.md) D4).
///
/// With a `nikaia.toml` above the file it is that project, and the file has to
/// be the binary's entry: a project has one, `src/main.nika`, and running a
/// different file of it would be running something the project does not build.
///
/// With none it is a project kept for the file alone, in the user's cache
/// directory and keyed on the file's path, built through the same Cargo
/// workspace `nikaia build` makes - so a single file behaves as a program and
/// not as a lowering. The interpreter is not used: it handles a part of the
/// language, and a program means one thing (Part I 1.2).
///
/// **The file is checked where it is first.** The kept project holds a copy,
/// and a refusal about the copy would name a path the user never wrote; so the
/// file is lowered under its own name, standalone (D2), and only a file that
/// lowers is copied and built. The copy is rewritten only when its bytes
/// changed, so Cargo's freshness keeps working.
pub fn project_for_file(
    file: &Path,
    target: Option<&str>,
    user_parallelism: Option<&str>,
    no_cache: bool,
    allowlist: Option<&Path>,
) -> Result<Project> {
    if !file.is_file() {
        refuse!("{} is not there.", file.display());
    }
    let layout = Layout::resolve(file);
    if layout.in_project {
        let project = Project::open(&layout.root, target, user_parallelism)?;
        if canonical(file) != canonical(&project.entry()) {
            refuse!(
                "{} is not the entry point of the project at {}.\n\
                 `nikaia run` with a file runs the program whose entry it is, and this \
                 project's program starts at `{ENTRY}`. Run `nikaia run --project {}`, or \
                 `nikaia lower {}` to lower the file to Rust.",
                file.display(),
                project.root.display(),
                project.root.display(),
                file.display()
            );
        }
        return Ok(project);
    }

    let settings = Settings::resolve(&Manifest::find(file)?, target, user_parallelism)?;
    lower_reading(file, &settings, no_cache, &[], allowlist)?;
    let warned = true;

    let canonical_file = canonical(file);
    let key = orchestrator::cache::sha256_hex(canonical_file.to_string_lossy().as_bytes());
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    // Cargo's package names are letters, digits, `-` and `_`, starting with a
    // letter; the binary is called what the package is.
    let name = match stem.chars().next() {
        Some(first)
            if first.is_ascii_alphabetic()
                && stem
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
        {
            stem
        }
        _ => "program".to_string(),
    };
    let root = Layout::user_cache_dir()
        .join("run")
        .join(format!("{name}-{}", &key[..16]));
    // An entry of the user cache like any other: pruned when it was used
    // longest ago (#724).
    orchestrator::cache::use_entry(&root);
    write_if_changed(
        &root.join("nikaia.toml"),
        &format!(
            "# Kept by `nikaia run` for {} (ADR-260 D4): a file outside any project\n\
             # is built as one of its own. Rewritten from that file on every run.\n\
             [package]\nname = \"{name}\"\nversion = \"0.0.0\"\n",
            canonical_file.display()
        ),
    )?;
    write_if_changed(
        &root.join(ENTRY),
        &std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?,
    )?;
    let mut project = Project::open(&root, target, user_parallelism)?;
    project.warned = warned;
    project.written = Some(file.to_path_buf());
    Ok(project)
}

pub fn lower_reading(
    input: &Path,
    settings: &Settings,
    no_cache: bool,
    packages: &[modules::Dependency],
    allowlist: Option<&Path>,
) -> Result<Lowered> {
    // `Layout` decides where the lock and the store go, and guarantees that
    // outside a `nikaia.toml` project nothing is written into the source tree.
    // It also names the unit relative to its root: an absolute path is D7's
    // first failure direction, a key that moves with the checkout.
    let layout = Layout::resolve(input);
    let unit = layout.unit_name(input);
    // **What this build may read, resolved before anything is read**
    // ([ADR-310](../../docs/specification/adr/adr-310.md) D4, D5).
    let reads = reads_for(&layout, allowlist)?;
    // **The level `target::has` answers for in a `comptime`** (ADR-371 D6).
    crate::comptime_run::build_for_cpu(settings.cpu_key().unwrap_or_default());
    let choices = match reads.list_digest() {
        Some(digest) => settings.choices().reading(digest),
        None => settings.choices(),
    };
    // **And what this build's boundaries say**
    // ([ADR-290](../../docs/specification/adr/adr-290.md) D5). A description is
    // a file a person edits, so an edit has to reach the next build - which it
    // did not, and failed open while it did not.
    let choices = choices.describing(crate::describe::descriptions_digest(&layout.root));
    // **The committed proofs are an input** (#448): a recorded answer is what
    // the lowering uses, so the file belongs in the key once it exists.
    let choices = match layout.in_project {
        true => choices.proving(block_on(project_members::project_members_proofs_digest(
            &layout.root,
            &ForeignHost,
        ))),
        false => choices,
    };

    // A cache that cannot be opened is a slower build, never a failed one
    // (D12). The steps from here on are Nikaia (`project_lower_reading`,
    // ADR-294, #125, P5b); the program, the cache, the check and the emitter
    // stay Rust behind [`LowerHost`].
    let host = LowerHost {
        input,
        settings,
        no_cache,
        packages,
        unit,
        layout: &layout,
        reads: &reads,
        choices: RefCell::new(Some(choices)),
        program: RefCell::new(None),
        key_source: RefCell::new(String::new()),
        cache: RefCell::new(None),
        exported: RefCell::new(None),
        foreign: RefCell::new(None),
        book: RefCell::new(None),
        checked_facts: RefCell::new(BTreeMap::new()),
        published: RefCell::new(BTreeMap::new()),
        proofs: RefCell::new(None),
        failed: RefCell::new(None),
    };
    let out = match project_lower::project_lower_reading(&host, layout.in_project) {
        Ok(out) => out,
        Err(error) => {
            return Err(host
                .failed
                .into_inner()
                .unwrap_or_else(|| anyhow!("{error}")));
        }
    };
    let proofs = host.proofs.into_inner();
    let exported = host.exported.into_inner();
    let (rust, ledger) = (out.rust, out.ledger);

    // **A library's entry points get their wrappers and their header**
    // (ADR-284 D4, D7, D12), whether the cache answered or not; the header
    // carries the digest of the ledger this lowering wrote.
    let digest = crate::assets::digest(ledger.as_bytes());
    let (rust, header, python, node) = match exported {
        Some(exported) => (
            rust + &exported.rust,
            Some(
                exported
                    .header
                    .replace(crate::cexport::LEDGER_DIGEST, &digest),
            ),
            Some(
                exported
                    .python
                    .replace(crate::cexport::LEDGER_DIGEST, &digest),
            ),
            Some(
                exported
                    .node
                    .replace(crate::cexport::LEDGER_DIGEST, &digest),
            ),
        ),
        None => (rust, None, None, None),
    };

    Ok(Lowered {
        rust,
        ledger,
        derived: out.derived,
        sources: out.sources,
        reused: out.reused,
        notes: out.notes,
        proofs,
        header,
        python,
        node,
    })
}

/// What `project_lower_reading` asks of the compiler (`tools/project_lower.nika`,
/// P5b): the program, the cache and the build's choices it holds, the check
/// and the emitter over them, and what they leave for [`Lowered`]. A refusal
/// of a `throws` method is kept in `failed` and handed back unchanged.
struct LowerHost<'a> {
    input: &'a Path,
    settings: &'a Settings,
    no_cache: bool,
    packages: &'a [modules::Dependency],
    unit: String,
    layout: &'a Layout,
    reads: &'a assets::Reads,
    /// The key's choices; named with the program's paths once it is read.
    choices: RefCell<Option<Choices>>,
    program: RefCell<Option<modules::Program>>,
    key_source: RefCell<String>,
    cache: RefCell<Option<Cache>>,
    exported: RefCell<Option<crate::cexport::Exported>>,
    /// The boundary the check read, for the emitter.
    foreign: RefCell<Option<Foreign>>,
    /// The prover's book between the check and the emitter.
    book: RefCell<Option<crate::proofs::Book>>,
    checked_facts: RefCell<BTreeMap<usize, check::CheckFacts>>,
    published: RefCell<BTreeMap<String, crate::prove::Published>>,
    /// The book as the lowering left it, in a project.
    proofs: RefCell<Option<crate::proofs::Book>>,
    failed: RefCell<Option<anyhow::Error>>,
}

type HostResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl LowerHost<'_> {
    /// `result` for the Nikaia side, its error kept to be handed back.
    fn held<T>(&self, result: Result<T>) -> HostResult<T> {
        result.map_err(|error| {
            let said = format!("{error:#}");
            *self.failed.borrow_mut() = Some(error);
            said.into()
        })
    }

    fn program(&self) -> std::cell::Ref<'_, modules::Program> {
        std::cell::Ref::map(self.program.borrow(), |program| {
            program.as_ref().expect("the program is read first")
        })
    }

    fn choices(&self) -> std::cell::Ref<'_, Choices> {
        std::cell::Ref::map(self.choices.borrow(), |choices| {
            choices.as_ref().expect("the choices are set")
        })
    }
}

impl project_lower::ProjectLowerHost for LowerHost<'_> {
    fn read_program(&self) -> HostResult<()> {
        // Part I 9.1: a package is a directory, so a build is every `.nika`
        // beside the entry - and the cache cannot be asked about a program
        // until the program is known.
        //
        // **Outside a project it is the one file.** A package is a directory *of a
        // project*, which is what a `nikaia.toml` declares; a directory of loose
        // examples is a directory of programs, and compiling one of them must not
        // pull in the other ten (ADR-286 D1, `modules::collect_one`).
        // **An option's computed default is compiled where this build compiles
        // its `comptime`s** (ADR-321 D1, ADR-318 D1): the ledger inferred while
        // the program is read records the value.
        let (input, packages) = (self.input, self.packages);
        let program =
            crate::comptime_run::defaults_compiled_in(self.reads.workshop().place(), || {
                match (self.settings.words.tests, self.layout.in_project) {
                    (true, in_project) => {
                        modules::Program::read_for_tests(input, packages, in_project)
                    }
                    (false, true) => modules::Program::read_with(input, packages),
                    (false, false) => modules::Program::read_one(input),
                }
            });
        let mut program = self.held(program)?;
        program.entry_shown_as = SHOWN_AS.with(|cell| cell.borrow().clone());
        *self.key_source.borrow_mut() = program.sources().join("\n// --- unit ---\n");
        // **What the output names is part of the key** (0.0.239): the abort table
        // names each file as it was handed to the compiler (ADR-300 D9), so a
        // lowering cached for `/abs/one.nika` and reused for `one.nika` named the
        // first path in the second program's aborts. A choice, and not part of
        // the source, because the source's hash is the lockfile's.
        let choices = self
            .choices
            .borrow_mut()
            .take()
            .expect("the choices are set");
        *self.choices.borrow_mut() = Some(choices.naming(program.shown_paths().join("\n")));
        *self.program.borrow_mut() = Some(program);
        Ok(())
    }

    fn open_cache(&self) {
        if self.no_cache {
            return;
        }
        match Cache::open(
            &self.layout.lock,
            &self.layout.store,
            env!("NIKAIA_RUSTC_VERSION"),
            env!("NIKAIA_COMPILER"),
        ) {
            Ok(cache) => *self.cache.borrow_mut() = Some(cache),
            Err(error) => eprintln!("warning: the build cache is unavailable: {error:#}"),
        }
    }

    fn source_paths(&self) -> Vec<PathBuf> {
        self.program()
            .units
            .iter()
            .map(|unit| unit.path.clone())
            .collect()
    }

    fn could_promise(&self) -> String {
        self.program().could_promise()
    }

    fn committed_ledger(&self) -> Option<Ledger> {
        std::fs::read_to_string(self.layout.root.join("nikaia.contracts"))
            .ok()
            .and_then(|text| Ledger::parse(&text).ok())
    }

    fn lost_sync(&self, committed: &Ledger) -> String {
        self.program().lost_sync(committed)
    }

    fn export(&self) -> HostResult<()> {
        let settings = self.settings;
        if !settings.library() {
            return Ok(());
        }
        let package = Manifest::read(&self.layout.root.join("nikaia.toml"))
            .ok()
            .and_then(|m| m.package_name().map(str::to_string))
            .unwrap_or_else(|| settings.words.symbol_prefix.clone());
        let exported = crate::cexport::export(
            &self.program(),
            &settings.words.symbol_prefix,
            &package,
            settings.words.user_parallelism == "yes",
            &settings.words.target,
        );
        *self.exported.borrow_mut() = self.held(exported)?;
        Ok(())
    }

    fn cached(&self) -> Option<project_lower::ProjectLowerArtifacts> {
        let cache = self.cache.borrow();
        let artifacts = cache
            .as_ref()?
            .lookup(
                &self.unit,
                &self.key_source.borrow(),
                &self.choices(),
                &self.layout.root,
            )
            .filter(|artifacts| artifacts.has_all(&[RUST, CONTRACTS, DERIVED, WARNINGS]))?;
        let get = |name| artifacts.get(name).expect("checked above").to_string();
        Some(project_lower::ProjectLowerArtifacts {
            rust: get(RUST),
            ledger: get(CONTRACTS),
            derived: get(DERIVED),
            warnings: get(WARNINGS),
        })
    }

    fn record(&self, made: &project_lower::ProjectLowerArtifacts) {
        let mut cache = self.cache.borrow_mut();
        let Some(cache) = cache.as_mut() else {
            return;
        };
        // An empty map is correct and usual: with no list in effect nothing
        // can be read at all.
        let artifacts = Artifacts::new()
            .with(RUST, &made.rust)
            .with(CONTRACTS, &made.ledger)
            .with(DERIVED, &made.derived)
            .with(WARNINGS, &made.warnings);
        let stored = cache
            .record(
                &self.unit,
                &self.key_source.borrow(),
                self.reads.taken(),
                &self.choices(),
                &artifacts,
            )
            .and_then(|()| cache.save());
        if let Err(error) = stored {
            // The outputs are in hand; only the next build is slower.
            eprintln!("warning: the build cache could not be updated: {error:#}");
        }
    }

    fn in_the_wrapper(&self) -> bool {
        std::env::var_os(WRAPPER_MARKER).is_some() || WARNED.with(|warned| warned.get())
    }

    fn eprint(&self, text: &str) {
        eprint!("{text}");
    }

    fn check_units(&self) -> HostResult<String> {
        let (layout, settings) = (self.layout, self.settings);
        let program = self.program();
        // Every unit is checked against the *program's* contracts, not its
        // own: `utils::double` is a name `main.nika` may write, and the
        // type checker resolves it in the one place any name is resolved.
        let modules = program.package_names();
        // **Read once per lowering and not per unit** (ADR-290 D1): the
        // boundary is the build's, and every unit of it stands at the same
        // one. `layout.root` is the project this input belongs to, and
        // outside a project there is no manifest and therefore nothing
        // declared.
        let foreign = match layout.in_project {
            true => foreign_of(&layout.root),
            false => project_members::project_members_no_foreign(),
        };
        // **The committed ledger against the one this build inferred**
        // ([ADR-308](../../docs/specification/adr/adr-308.md) D11), read
        // once for the package rather than once per unit: the question is
        // whether a *contract* moved, and a contract belongs to the
        // package.
        let mut newly = newly(&layout.root, &program.contracts);
        newly.keepers = program.keepers.clone();
        // Collected once for the whole package, because every unit is
        // checked against the same program (`ADR-286`) and a `comptime` in
        // any of them may call into any other.
        let beside: Vec<&crate::parser::Parsed> =
            program.units.iter().map(|unit| &unit.parsed).collect();
        let around = Around {
            foreign: &foreign,
            newly: &newly,
            beside: &beside,
            reads: self.reads,
        };
        // **A dependency's unit is checked as its own package writes it**
        // (issue #159). The program's
        // contracts are the right ledger for the program's own files and the
        // wrong one for a package's: absorbing qualified its keys *and the
        // types inside them*, and the package's file writes the bare word.
        // `Program::as_its_own` is the ledger that package was inferred
        // with, kept rather than reconstructed.
        //
        // **And the program's own files against what each dependency
        // promised** (ADR-288 D28), not what its body happens to be: an
        // `"inferred"` `sync` reads as *may pause* across the boundary.
        let promised = program.contracts_as_promised();
        // **The prover's recorded answers** ([ADR-270](../../docs/specification/adr/adr-270.md)
        // D4, D21, #448): open while the program is checked and lowered,
        // which is where every question is asked, and written back beside
        // `nikaia.contracts` after. Outside a project there is no file.
        let book = match layout.in_project {
            true => crate::proofs::Book::read(&layout.root),
            false => crate::proofs::Book::default(),
        };
        let mut warnings = String::new();
        // **The check's facts are the emitter's** where the check asked
        // exactly what `check::propagation_against` asks: the unit's own
        // ledger the emitter's, no module and no contract that moved
        // (#727).
        let facts_reusable = foreign.packages(&modules).is_empty()
            && newly.throws.is_empty()
            && newly.keeps.is_empty()
            && promised == program.contracts;
        let mut checked_facts = BTreeMap::new();
        let (checked, book) = crate::proofs::with_book(book, || -> Result<()> {
            for (at, unit) in program.units.iter().enumerate() {
                let contracts = match &unit.package {
                    Some(package) => program
                        .as_its_own
                        .get(package)
                        .unwrap_or(&program.contracts),
                    None => &promised,
                };
                let mut facts = None;
                let keep = facts_reusable && unit.package.is_none();
                warnings += &crate::proofs::for_unit(
                    &unit.path,
                    &unit.source,
                    unit.package.is_some(),
                    || {
                        check(
                            &unit.parsed,
                            contracts,
                            &modules,
                            around,
                            &unit.path,
                            &unit.source,
                            &settings.words.user_parallelism,
                            &settings.words.refuted_claims,
                            &settings.words.unproved_claims,
                            settings.library(),
                            keep.then_some(&mut facts),
                        )
                    },
                )?;
                if let Some(facts) = facts {
                    checked_facts.insert(at, facts);
                }
            }
            Ok(())
        });
        self.held(checked)?;
        *self.book.borrow_mut() = Some(book);
        *self.checked_facts.borrow_mut() = checked_facts;
        *self.foreign.borrow_mut() = Some(foreign);
        Ok(warnings)
    }

    fn emit(&self) -> HostResult<String> {
        let mut program = self
            .program
            .borrow_mut()
            .take()
            .expect("the program is read first");
        // **The described crates' boundary reaches the emitter**
        // (ADR-290 D20), which the checker above already read it for.
        program.described = self
            .foreign
            .borrow()
            .as_ref()
            .expect("the program is checked first")
            .descriptions
            .clone();
        program.checked_facts = std::mem::take(&mut *self.checked_facts.borrow_mut());
        let book = self
            .book
            .borrow_mut()
            .take()
            .expect("the program is checked first");
        let (lowered, book) = crate::proofs::with_book(book, || {
            program.emit_reading(self.settings.build, self.reads)
        });
        *self.program.borrow_mut() = Some(program);
        let lowered = self.held(lowered)?;
        if self.layout.in_project {
            *self.proofs.borrow_mut() = Some(book);
        }
        *self.published.borrow_mut() = lowered.published;
        Ok(lowered.rust)
    }

    fn contracts(&self) -> Ledger {
        self.program().contracts.clone()
    }

    fn published(&self) -> BTreeMap<String, crate::prove::Published> {
        std::mem::take(&mut *self.published.borrow_mut())
    }

    fn render(&self, ledger: &Ledger) -> (String, String) {
        (ledger.render(), ledger.render_derived())
    }

    fn unused_reads(&self) -> Vec<String> {
        self.reads.unused()
    }

    fn changed_contracts(&self, now: &Ledger, committed: &Ledger) -> String {
        // The packages this program depends on: their entries in the ledger are
        // theirs, and a contract they changed is said in their own build.
        let program = self.program();
        crate::prove::changed_contracts(now, committed, &program.keepers, |key| {
            !key.split_once("::")
                .is_some_and(|(package, _)| program.as_its_own.contains_key(package))
        })
    }

    fn parse_ledger(&self, text: &str) -> Option<Ledger> {
        Ledger::parse(text).ok()
    }
}

/// The names a lowering stores its outputs under.
pub const RUST: &str = "rust";
pub const CONTRACTS: &str = "contracts";
/// What the contract was derived from, `nikaia.derived`
/// ([ADR-251](../../docs/specification/adr/adr-251.md) D1).
pub const DERIVED: &str = "derived";
/// The warnings the check said while it made the lowering, said again when it
/// is reused.
pub const WARNINGS: &str = "warnings";

/// **What this build links against, and what has been described**
/// ([ADR-290](../../docs/specification/adr/adr-290.md) D1): the struct, how a
/// build reads it and what it gives a program's packages are
/// `ProjectMembersForeign` in `tools/project_members.nika` (P4b).
pub use nikaia_std::tools::project_members::ProjectMembersForeign as Foreign;

/// What a build of `root` is at its boundary
/// (`project_members_foreign_of`, P4b).
pub fn foreign_of(root: &Path) -> Foreign {
    block_on(project_members::project_members_foreign_of(
        root,
        &Manifests::default(),
        &ForeignHost,
    ))
}

/// **What `tools/project_members.nika` asks of the compiler at a build's
/// foreign boundary** (P4b): SHA-256, and the ledger's reading and merging.
struct ForeignHost;

impl ProjectMembersForeignHost for ForeignHost {
    fn read_beside(&self, path: &Path) -> Option<Ledger> {
        Ledger::read_beside(path)
    }

    fn absorb(&self, into: Ledger, other: Ledger) -> Ledger {
        let mut into = into;
        into.absorb(None, other);
        into
    }

    fn empty_ledger(&self) -> Ledger {
        Ledger::empty()
    }

    fn crate_root(&self, root: &Path, name: String) -> Option<PathBuf> {
        crate::describe::crate_root(root, &name)
    }
}

/// **`std`'s ledger and every described crate's, in one**
/// ([ADR-290](../../docs/specification/adr/adr-290.md) D1). A described
/// boundary is a library boundary: the analyses ask a ledger what a callee
/// takes, hands back, pauses on and keeps, and where the callee is foreign
/// the answer comes from the file the reader was told to write.
///
/// **`std` wins a collision**, and the reason it can be decided in one line
/// is that there is nothing to decide yet: a description's names carry the
/// crate word in front of them, `std`'s carry a module's, and no manifest in
/// this repository declares a crate whose word is one of `std`'s modules.
/// The day one does, that is a refusal to write and not a silence to keep —
/// issue #100 carries it.
pub fn foreign_library(foreign: &Foreign) -> Result<Ledger> {
    let mut library = crate::contracts::std_library();
    for (name, contract) in &foreign.descriptions.functions {
        library
            .functions
            .entry(name.clone())
            .or_insert_with(|| contract.clone());
    }
    for (name, contract) in &foreign.descriptions.types {
        library
            .types
            .entry(name.clone())
            .or_insert_with(|| contract.clone());
    }
    Ok(library)
}

/// `user_parallelism` reaches this and reaches **nothing inside the analyses**.
/// The verdict of every rule is a property of the program, so no switch may
/// change it (ADR-005 §1 Group B); what the switch does change is whether a
/// refusal is about *this* build, and that is a question about severity. See
/// `project_check_lint_where_nothing_crosses` (`tools/project_check.nika`).
/// **What a build knows that one unit does not**, handed to the check together.
///
/// Two facts, and both are about the *build* rather than about the file being
/// checked — what the manifest links against and what was described beside it
/// ([ADR-290](../../docs/specification/adr/adr-290.md) D1), and what a callee
/// has newly gained since the ledger committed beside it
/// ([ADR-308](../../docs/specification/adr/adr-308.md) D11). Neither can be read
/// from a `Parsed`, and neither belongs to it.
///
/// **One argument and not two**, because they arrive together and for the same
/// reason: the day a third fact about the build is needed, it goes here rather
/// than onto a signature that is already long enough to be read wrong.
#[derive(Debug, Clone, Copy)]
pub struct Around<'a> {
    pub foreign: &'a Foreign,
    pub newly: &'a check::Newly,
    /// **What this build may read while it builds**
    /// ([ADR-310](../../docs/specification/adr/adr-310.md)).
    ///
    /// The fourth fact, and it belongs beside the other three for the reason
    /// the doc above gives: it is about the *build* rather than about a file,
    /// no ledger can carry it, and it binds the whole build — a dependency's
    /// `asset("…")` is checked against the list of the build that is running,
    /// never against anything the dependency ships (D6).
    pub reads: &'a crate::assets::Reads,
    /// **Every file of the program being built.**
    ///
    /// The third fact this carries, and the one that had to be a `Parsed`
    /// rather than a column: a `comptime` calling across a file boundary needs
    /// the callee's **body**, and a ledger records what a caller has to know
    /// about a function it cannot see the body of. Each file owns the interner
    /// its symbols resolve in, so the AST travels rather than the items.
    pub beside: &'a [&'a crate::parser::Parsed],
}

#[allow(clippy::too_many_arguments)]
pub fn check(
    parsed: &crate::parser::Parsed,
    own: &Ledger,
    modules: &BTreeSet<String>,
    around: Around<'_>,
    path: &Path,
    source: &str,
    user_parallelism: &str,
    refuted_claims: &str,
    unproved_claims: &str,
    building_a_library: bool,
    // Where the walk's facts go, for an emitter that would ask the same.
    facts_out: Option<&mut Option<check::CheckFacts>>,
) -> Result<String> {
    let Around {
        foreign,
        newly,
        beside,
        reads,
    } = around;
    let host = CheckHost {
        parsed,
        beside,
        own,
        library: foreign_library(foreign)?,
        // A described crate is a package by the spelling rule, which is what
        // `modules` is: a set of words that appear in front of a `::` and are
        // not `std`'s ([`Foreign::packages`]).
        modules: foreign.packages(modules),
        newly,
        reads,
        foreign,
        facts: std::cell::RefCell::new(None),
        violations: std::cell::RefCell::new(Vec::new()),
    };
    let said = project_check::project_check_check(
        &host,
        &path.display().to_string(),
        source,
        user_parallelism,
        refuted_claims,
        unproved_claims,
        building_a_library,
    );
    if let (Some(out), Some(facts)) = (facts_out, host.facts.into_inner()) {
        *out = Some(facts);
    }
    match said.refused {
        // A refusal and not a failure of this compiler, so it leaves without a
        // backtrace: the diagnostics have already said everything, and this is
        // the tally (`diagnostics::Refused`).
        Some(tally) => Err(diagnostics::refuse(tally)),
        None => Ok(said.warnings),
    }
}

/// What `project_check_check` asks of the compiler (`tools/project_check.nika`):
/// the walks over one unit, which stay Rust, and the renderer.
struct CheckHost<'a> {
    parsed: &'a crate::parser::Parsed,
    beside: &'a [&'a crate::parser::Parsed],
    own: &'a Ledger,
    library: Ledger,
    modules: BTreeSet<String>,
    newly: &'a check::Newly,
    reads: &'a assets::Reads,
    foreign: &'a Foreign,
    /// The walk's facts, for the caller's `facts_out`.
    facts: std::cell::RefCell<Option<check::CheckFacts>>,
    /// What `sync::check` found, named by index on the Nikaia side.
    violations: std::cell::RefCell<Vec<sync::Violation>>,
}

impl project_check::ProjectCheckHost for CheckHost<'_> {
    fn check_against(&self) -> Vec<check::Finding> {
        let checked = check::check_against(
            self.parsed,
            self.beside,
            self.own,
            &self.library,
            &self.modules,
            self.newly,
            self.reads,
        );
        *self.facts.borrow_mut() = Some(checked.facts);
        checked.findings
    }

    fn foreign_check(&self) -> Vec<check::Finding> {
        crate::foreign::check(
            self.parsed,
            &self.foreign.declared,
            &self.foreign.described,
            &self.foreign.moved,
        )
    }

    fn sync_violations(&self) -> i64 {
        let violations = sync::check(self.parsed, self.own, &self.library);
        let count = violations.len() as i64;
        *self.violations.borrow_mut() = violations;
        count
    }

    fn render_sync_violation(&self, at: i64, path: &str, source: &str) -> String {
        diagnostics::render_sync_violation(&self.violations.borrow()[at as usize], path, source)
    }

    fn render_finding_in(&self, finding: &check::Finding, path: &str, lines: &[String]) -> String {
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        diagnostics::render_finding_in(finding, path, &lines)
    }

    fn in_the_wrapper(&self) -> bool {
        std::env::var_os(WRAPPER_MARKER).is_some() || WARNED.with(|warned| warned.get())
    }

    fn eprint(&self, text: &str) {
        eprint!("{text}");
    }
}

/// **What each function has newly gained in its `throws` set** since the
/// ledger committed beside it ([ADR-308](../../docs/specification/adr/adr-308.md)
/// D1).
///
/// The ledger records `throws` as a **set** ([ADR-023](../../docs/specification/adr/adr-023.md)
/// D1), so a callee that gains a failure is a set that gained a member — and
/// every `catch` over that callee now receives something it did not when it was
/// written. D1 asks for a line naming each of them; this is the half that
/// answers *which errors*.
///
/// **Three things answer *nothing gained*, and each is right rather than
/// cheap.** A build with no committed ledger beside it has nothing to compare,
/// and a first build is not a change. A ledger that does not parse is not an
/// answer (`ADR-100` D3's rule one file over). And a function the committed
/// ledger does not name is **new**, so nothing was ever written against its
/// set — a new function's whole set is not a set that grew.
fn newly(root: &Path, inferred: &Ledger) -> check::Newly {
    let Some(committed) = std::fs::read_to_string(root.join("nikaia.contracts"))
        .ok()
        .and_then(|text| Ledger::parse(&text).ok())
    else {
        return check::Newly::new();
    };
    check::Newly {
        throws: project_check::project_check_gained(&committed, inferred),
        keeps: keeps_that_moved(&committed, inferred),
        keepers: Default::default(),
    }
}

/// **The parameters whose `keeps` changed**
/// ([ADR-297](../../docs/specification/adr/adr-297.md) D12): kept now and not
/// before, or kept before and only read now. A parameter the old signature
/// does not have is **new** rather than moved, as a new function's set is not
/// a set that grew. Whether the type's teardown does anything is the checker's
/// question, which has both ledgers.
fn keeps_that_moved(committed: &Ledger, inferred: &Ledger) -> BTreeMap<String, Vec<check::Kept>> {
    project_check::project_check_keeps_that_moved(committed, inferred)
        .into_iter()
        .map(|(name, moved)| {
            let moved = moved
                .into_iter()
                .map(|kept| check::Kept {
                    param: kept.param,
                    now: kept.now,
                    ty: kept.ty,
                })
                .collect();
            (name, moved)
        })
        .collect()
}

/// **Write `nikaia.proofs`, or - under `--locked` - check that it did not
/// need to change** ([ADR-270](../../docs/specification/adr/adr-270.md) D21,
/// D22): every question the build asked has an entry that checks, and every
/// entry is one a question asked, so the file cannot rot. Nothing to do where
/// the lowering asked nothing - a cache hit, a file outside a project.
pub fn write_proofs(root: &Path, book: Option<&crate::proofs::Book>, locked: bool) -> Result<()> {
    let Some(book) = book else {
        return Ok(());
    };
    if !locked {
        return book
            .write(root)
            .with_context(|| format!("writing {}", root.join(crate::proofs::FILE).display()));
    }
    if book.searched > 0 || !book.as_recorded(root) {
        // **Where the questions were asked** (D22), the first few: the file
        // and the function, so the change that asked them can be found.
        const SHOWN: usize = 5;
        let mut places: String = book
            .searched_at
            .iter()
            .take(SHOWN)
            .map(|place| format!("\n  asked at {}", place.shown(root)))
            .collect();
        if book.searched_at.len() > SHOWN {
            places.push_str(&format!(
                "\n  and at {} other places",
                book.searched_at.len() - SHOWN
            ));
        }
        let why = match book.searched {
            0 => "it holds entries no question asked".to_string(),
            1 => "1 question was searched".to_string(),
            n => format!("{n} questions were searched"),
        };
        return Err(anyhow!(project_check::project_check_locked_proofs(
            &root.join(crate::proofs::FILE).display().to_string(),
            &why,
            &places,
            crate::proofs::FILE
        )));
    }
    Ok(())
}

/// Write the ledger, or - under `--locked` - check that it did not need
/// writing.
///
/// The determinism guarantee (13.5) is what lets this compare bytes rather than
/// meanings: the same sources and the same compiler produce the same file, so a
/// difference is a change in a contract and never in the formatting.
pub fn write_ledger(path: &Path, ledger: &str, locked: bool) -> Result<()> {
    if !locked {
        // **Written whole, and only where it changed**: builds side by side
        // read a package's ledger (the programs `nikaia test` builds on its
        // workers all depend on the one package), and a build that read it
        // while another was rewriting it read half a ledger and refused
        // sound code.
        return crate::comptime_run::write_if_changed(path, ledger).map_err(|e| anyhow!(e));
    }

    let committed = std::fs::read_to_string(path).with_context(|| {
        project_check::project_check_locked_ledger_missing(&path.display().to_string())
    })?;

    if committed != *ledger {
        return Err(diagnostics::refuse(
            project_check::project_check_locked_ledger_changed(
                &path.display().to_string(),
                &changed_lines(ledger, &committed).join("\n"),
            ),
        ));
    }

    Ok(())
}

/// The lines the build produced that the committed ledger does not have, each
/// under the entry it belongs to.
///
/// Naming the entry is the whole point. A contract line is short and repeats -
/// `sync = true` says nothing on its own, and since ADR-288 it is the commonest
/// line in the file - so a bare list of them tells you that *something* changed
/// and leaves you to find out what. 13.5 asks this diff to narrate a cause; it
/// cannot do that without saying whose contract moved.
pub fn changed_lines(ledger: &str, committed: &str) -> Vec<String> {
    nikaia_std::tools::diffs::changed_lines(ledger, committed)
}

/// Which of the three explanations a run was asked for
/// ([ADR-292](../../docs/specification/adr/adr-292.md) D2,
/// [ADR-037](../../docs/specification/adr/adr-037.md) D8,
/// [ADR-010](../../docs/specification/adr/adr-010.md) D7).
///
/// They exist because these analyses **cannot be asked for** - there is no way to
/// request the cheaper reference count, the overlap or the faster hash; every
/// fallback is enumerated instead. `--sharing`'s own help says why that is only
/// fair if the fallbacks can be asked about, *"and this is the asking"*.
///
/// So they have to reach a real program, and a person with a real program builds
/// it with `nikaia build`. They were on the single-file path only, which is
/// exactly where the asking is not done.
#[derive(Debug, Clone, Copy, Default)]
pub struct Explain {
    pub overlaps: bool,
    pub sharing: bool,
    /// [ADR-283](../../docs/specification/adr/adr-283.md) D4's inverse tool:
    /// inspection rather than assertion.
    pub tethers: bool,
    pub trust: bool,
    /// [ADR-304](../../docs/specification/adr/adr-304.md) D9's report: what a
    /// `T::fields` loop was unrolled to, for the types actually used.
    pub comptime: bool,
    /// [ADR-269](../../docs/specification/adr/adr-269.md) D7's report: every
    /// `assert`, and whether it was proved, refuted or left to run time.
    pub asserts: bool,
    /// #389's report: every index and operation whose check the optimizations
    /// may drop, and whether it was.
    pub bounds: bool,
}

impl Explain {
    pub fn asked(&self) -> bool {
        self.overlaps
            || self.sharing
            || self.tethers
            || self.trust
            || self.comptime
            || self.asserts
            || self.bounds
    }
}

/// Print the explanations a run asked for, over every file of the package.
///
/// **Against the package's own ledger and not each file's**, which is the
/// difference from the single-file path doing this three times: `sync`, the touch
/// sets and the sharing classes are whole-program facts (ADR-288, ADR-292), and a
/// report built from one file's inferences would answer a different question from
/// the one the build answers.
///
/// The file name is printed where there is more than one, because a slot key
/// (`zaehle::counts`) does not say which file it is in - one namespace or not.
pub fn explain(
    program: &modules::Program,
    settings: &Settings,
    want: Explain,
    reads: &assets::Reads,
) -> Result<()> {
    if !want.asked() {
        return Ok(());
    }
    // The kinds in the order they are printed in, per unit.
    let kinds: Vec<String> = [
        ("overlaps", want.overlaps),
        ("sharing", want.sharing),
        ("tethers", want.tethers),
        ("asserts", want.asserts),
        ("bounds", want.bounds),
        ("trust", want.trust),
    ]
    .into_iter()
    .filter(|(_, asked)| *asked)
    .map(|(kind, _)| kind.to_string())
    .collect();
    let host = ExplainHost {
        program,
        settings,
        reads,
        library: crate::contracts::std_ledger(),
    };
    project_check::project_check_explain(&host, want.comptime, &kinds);
    Ok(())
}

/// What `project_check_explain` asks of the compiler: the reports, which stay
/// Rust, over the program's units by index.
struct ExplainHost<'a> {
    program: &'a modules::Program,
    settings: &'a Settings,
    reads: &'a assets::Reads,
    library: &'static Ledger,
}

impl project_check::ProjectCheckExplainHost for ExplainHost<'_> {
    fn units(&self) -> i64 {
        self.program.units.len() as i64
    }

    fn unit_path(&self, at: i64) -> String {
        self.program.units[at as usize].path.display().to_string()
    }

    fn unrolling_report(&self) -> String {
        let beside: Vec<&crate::parser::Parsed> =
            self.program.units.iter().map(|unit| &unit.parsed).collect();
        // **`Reads::none()`**, because what is reported is which types a
        // function was **used with**, and no file a build reads changes that.
        // Handing it a real allowlist would run a grammar to answer a question
        // about call sites ([ADR-177](../../docs/specification/adr/adr-177.md)).
        crate::check::unrolling_report(
            &beside,
            &self.program.contracts,
            &crate::assets::Reads::none(),
        )
    }

    fn explain_report(&self, kind: &str, at: i64) -> String {
        let (program, settings, library) = (self.program, self.settings, self.library);
        let unit = &program.units[at as usize];
        match kind {
            "overlaps" => crate::contracts::order::overlap_report(
                &unit.parsed,
                &program.contracts,
                library,
                &crate::emit::branch_starts_first(&unit.parsed, settings.build, &program.contracts),
            ),
            "sharing" => crate::contracts::sharing::report(
                &unit.parsed,
                &program.contracts,
                library,
                settings.build.user_parallelism == crate::emit::UserParallelism::Yes,
            ),
            "tethers" => crate::contracts::tether::report(&unit.parsed, &program.contracts),
            "asserts" => crate::check::claims_report(
                &unit.parsed,
                &unit.source,
                &unit.path.display().to_string(),
                &program
                    .units
                    .iter()
                    .map(|unit| &unit.parsed)
                    .collect::<Vec<_>>(),
                &program.contracts,
                library,
                self.reads,
                settings.words.unproved_claims != "warn",
                // **Red and green only on a terminal**, and never where
                // `NO_COLOR` is set (Part III, `--asserts`).
                std::env::var_os("NO_COLOR").is_none()
                    && std::io::IsTerminal::is_terminal(&std::io::stdout()),
            ),
            "bounds" => crate::bounds_report::report(
                &unit.parsed,
                &unit.source,
                &unit.path.display().to_string(),
                &program.contracts,
                library,
                settings.build.bounds,
                settings.build.overflow,
                settings.build.prove_through_loops,
            ),
            "trust" => crate::contracts::trust::render(
                &crate::contracts::trust::analyse_in(
                    &unit.parsed,
                    &program
                        .units
                        .iter()
                        .filter(|u| u.package == unit.package)
                        .map(|u| &u.parsed)
                        .collect::<Vec<_>>(),
                    library,
                ),
                &unit.path.display().to_string(),
                &unit.source,
            ),
            other => unreachable!("no `--explain` report `{other}`"),
        }
    }
}

/// A project: its root, its manifest, and the switches this build resolved.
#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub settings: Settings,
    /// **The warnings were said already** (#481): `nikaia run` with a file
    /// lowers it where it stands first, so its findings name the file the
    /// author wrote, and the build of the copy then keeps them to itself.
    pub warned: bool,
    /// **The file a loose `nikaia run` was given** (#510): the build is of a
    /// copy in the cache, and a message the language below raised about it is
    /// said against the file the author wrote, not the copy's path.
    pub written: Option<PathBuf>,
    /// **Which of `nikaia test`'s builds this is** (ADR-269 D12): the test
    /// build and the program, each at each setting asked for. They differ in
    /// what is lowered and in the switches, so in one place each would make
    /// the others stale and Cargo would build all of them again every run;
    /// each keeps a directory and a name of its own instead. `None` for a
    /// plain build, which keeps `target/nikaia/` as it is.
    pub variant: Option<String>,
}

thread_local! {
    /// Set while a build lowers what [`Project::warned`] said already.
    static WARNED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Set while a build lowers the copy [`Project::written`] names: the entry
    /// it lowers is said under that file's name (#510).
    static SHOWN_AS: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

impl Project {
    /// Find the project governing `start` and read its manifest.
    ///
    /// The walk to the root is `Layout::resolve`'s - the same one the build
    /// cache uses - so a build and the cache entries it writes can never
    /// disagree about which project a file belongs to.
    pub fn open(
        start: &Path,
        target: Option<&str>,
        user_parallelism: Option<&str>,
    ) -> Result<Project> {
        // `Layout::resolve` searches from an input file's *parent*, so it is
        // handed the manifest's own path: the search then starts at `start`
        // itself, which is what "build the project I am standing in" means.
        let layout = Layout::resolve(&start.join("nikaia.toml"));
        if !layout.in_project {
            refuse!("{}", project_settings::project_settings_no_project(start));
        }

        let manifest = Manifest::read(&layout.root.join("nikaia.toml"))?;
        // ADR-303 D5 moved `cleanup-deadline` out of the manifest. The key is
        // still accepted, and the note is what keeps the move from being
        // silent - a manifest whose setting stopped being read without saying
        // so is the mistake `[build]`'s unknown-key check already exists for.
        for note in manifest.notes() {
            eprintln!("note: {note}");
        }
        let settings = Settings::resolve(&manifest, target, user_parallelism)?;
        if let Some(missing) = settings.build.target.unbuildable() {
            refuse!(
                "{}",
                project_settings::project_settings_unbuildable(
                    settings.build.target.triple(),
                    &missing
                )
            );
        }

        Ok(Project {
            root: layout.root,
            manifest,
            settings,
            warned: false,
            written: None,
            variant: None,
        })
    }

    /// `src/main.nika` (Part III 13.1).
    pub fn entry(&self) -> PathBuf {
        self.root.join(ENTRY)
    }

    /// Where the generated `Cargo.toml` goes. Under `target/`, because it is
    /// build output: it is regenerated from `nikaia.toml` on every build and
    /// editing it would be editing something the next build overwrites.
    pub fn build_dir(&self) -> PathBuf {
        project_settings::project_settings_build_dir(&self.root, &self.variant)
    }

    /// `target/nikaia/`, or a directory under it of the [`Self::variant`]'s.
    fn own_dir(&self) -> PathBuf {
        project_settings::project_settings_own_dir(&self.root, &self.variant)
    }

    /// Where the emitted Rust goes - **outside** [`Self::build_dir`], and that
    /// is not tidiness. Cargo lists the files of a path package to decide
    /// whether it is fresh, so a generated source inside the package would make
    /// the package dirty the moment it was written, and every build would
    /// recompile.
    pub fn gen_dir(&self) -> PathBuf {
        project_settings::project_settings_gen_dir(&self.root, &self.variant)
    }

    /// **The switches, as a file Cargo can see** (ADR-021 D5).
    ///
    /// A build switch is not a source, so nothing Cargo watches changes when
    /// one does — and the lowering happens inside the `rustc` wrapper, which
    /// Cargo runs only for a package it thinks is stale. So flipping
    /// `user-parallelism` in `nikaia.toml` left the package fresh and the
    /// binary at the **old** setting, silently: the cache had the switch as a
    /// dimension (D5) and nothing ever got as far as asking it.
    ///
    /// This is the third member of the same family as the two freshness traps
    /// the project build already carries, and it takes the same answer: give
    /// Cargo a file. The driver writes the resolved switches here before
    /// `cargo` runs, and the wrapper puts it into the dependency file beside
    /// the `.nika` sources — so a switch that changed is an input that changed.
    ///
    /// Under `target/` and outside [`Self::build_dir`], for [`Self::gen_dir`]'s
    /// reason: a file *inside* the path package would make it dirty on every
    /// build rather than on a changed one.
    pub fn switches_path(&self) -> PathBuf {
        project_settings::project_settings_switches_path(&self.root, &self.variant)
    }

    /// The ledger, in the project root (Part III 13.5).
    pub fn ledger_path(&self) -> PathBuf {
        project_settings::project_settings_ledger_path(&self.root)
    }

    /// The packages this project depends on by path - see [`packages_of`].
    pub fn packages(&self) -> Result<Vec<modules::Dependency>> {
        packages_of(&self.manifest, &self.root, true)
    }

    /// Every package of this build, the entry first - see [`members_of`].
    pub fn members(&self) -> Result<Vec<Member>> {
        members_of(&self.manifest, &self.root)
    }

    /// One member's `nikaia.toml` translated (ADR-002 D1).
    ///
    /// `rust` is that package as this build lowered it, and it decides which of
    /// the compiler's own runtime crates the generated crate declares - a
    /// package that names none of them depends on nothing and builds in a
    /// second.
    ///
    /// `kind` is what puts the entry package's `fn main` in a binary and every
    /// package under it in a library beside it
    /// ([ADR-286](../../docs/specification/adr/adr-286.md) D19).
    fn cargo_member(&self, member: &Member, rust: &str, kind: CrateKind) -> Result<CargoProject> {
        let mut dependencies = runtime_dependencies(rust, self.settings.build.reentrancy_check)?;
        for (dependency, value) in member.manifest.dependencies() {
            match value {
                // The whole point of D1: whatever the author wrote reaches
                // Cargo, and Cargo resolves and links it as it would for any
                // Rust project - **narrowed by one key**
                // ([ADR-286](../../docs/specification/adr/adr-286.md) D17). A
                // relative `path` is resolved against `nikaia.toml` and written
                // absolute; `version`, `git`, `tag`, `features` and the rest
                // still travel untouched.
                Dependency::Rust(value) => {
                    dependencies.insert(dependency.clone(), rooted(value, &member.root));
                }
                // **Cargo's renaming form** (ADR-286 D20): the package is named
                // by its identity, the key is the name *this* crate uses for
                // it. So two packages that both want the key `c` are in two
                // different crates and never meet, and one package reached
                // through two parents is one crate - which is ADR-286 D15 rule 3
                // holding because Cargo already works that way.
                Dependency::Path(path) => {
                    let root = canonical(&member.root.join(path));
                    let named = self.crate_named(&root)?;
                    let mut table = toml::value::Table::new();
                    table.insert("package".to_string(), toml::Value::String(named.clone()));
                    // Beside this member, under the generated build directory:
                    // every crate of the build is written there, one directory
                    // each, named by the same name.
                    table.insert(
                        "path".to_string(),
                        toml::Value::String(format!("../{named}")),
                    );
                    dependencies.insert(dependency.clone(), toml::Value::Table(table));
                }
                // A registry, a name space and a distribution format are all
                // undecided, and guessing at one here would be inventing the
                // answer in the place it is hardest to review
                // ([ADR-002](../../docs/specification/adr/adr-002.md) D1 §5).
                Dependency::Nikaia(_) => refuse!(
                    "`{dependency}` is a Nikaia package given by version, but there's no \
                     package registry yet.\n\
                     Depend on it by path: `{dependency} = {{ path = \"../{dependency}\" }}`. \
                     (A crate from crates.io is written \
                     `{dependency} = {{ type = \"rust\", version = \"…\" }}`.)"
                ),
            }
        }

        Ok(CargoProject {
            package: Package {
                name: member.name.clone(),
                version: member.manifest.package_version().to_string(),
                // What the emitter writes, and what the tests compile it as.
                edition: "2024".to_string(),
                // ADR-288 D27: the floor the lowering needs, from one constant
                // in the emitter.
                rust_version: Some(crate::emit::RUST_FLOOR.to_string()),
            },
            kind,
            // A `[lib] name` has to be an identifier; a package name does not.
            // Cargo does this same replacement for a lib it names itself.
            //
            // **A program in the shared directory is named for its project as
            // well** (#495): Cargo puts it at `<target>/debug/<name>`, and two
            // projects of one name built there at once would replace each
            // other's. [`Self::own_copy`] gives it its own name back.
            bin_name: match kind {
                CrateKind::Bin => self.program_name(&member.name),
                CrateKind::Lib | CrateKind::CLibrary => member.name.replace('-', "_"),
            },
            bin_path: member.entry.clone(),
            features: runtime_features(&dependencies),
            dependencies,
        })
    }

    /// **Whether `nikaia run` can compile this program without Cargo**
    /// (`crate::run_fast`): one package, depending on no crate, linking no
    /// library, built for this machine with the `dev` profile untouched and
    /// `std` as its default features build it - which is what the bundle it
    /// links against is. On Linux and macOS, where a path written into the
    /// program finds the library at run time. And where the user's Cargo
    /// configuration or the environment names no linker and no flags.
    fn runs_without_cargo(&self, members: &[Member], link_flags: &[String]) -> bool {
        cfg!(any(target_os = "linux", target_os = "macos"))
            && members.len() == 1
            && members[0].manifest.dependencies().is_empty()
            && link_flags.is_empty()
            && !self.settings.library()
            && Some(self.settings.build.target) == crate::emit::host_target()
            && self.settings.panic_strategy() == "unwind"
            && self.settings.build.reentrancy_check.is_on()
            // **A CPU level is `std`'s too** (ADR-002 D6): the library this
            // path links was built at the baseline, so `--cpu` takes Cargo's.
            && self.settings.words.cpu.is_none()
            && self.manifest.codegen_for(&self.settings.words.target).is_empty()
            // A trace is asked about the wrapper Cargo runs.
            && std::env::var_os(TRACE_VAR).is_none()
            // The user's linker and flags are applied by Cargo (#786).
            && !crate::run_fast::cargo_sets_linker_or_flags(self.settings.build.target.triple())
    }

    /// The generated workspace: one crate per Nikaia package
    /// ([ADR-286](../../docs/specification/adr/adr-286.md) D19).
    ///
    /// `rust` is each member as this build lowered it, in `members`' order.
    pub fn cargo_workspace(&self, members: &[Member], rust: &[String]) -> Result<Workspace> {
        let mut out = Vec::new();
        for (at, member) in members.iter().enumerate() {
            // The entry package is the program; everything it reaches is a
            // library beside it.
            let kind = match (at, self.settings.library()) {
                (0, true) => CrateKind::CLibrary,
                (0, false) => CrateKind::Bin,
                _ => CrateKind::Lib,
            };
            let rust = rust.get(at).map(String::as_str).unwrap_or("");
            out.push((member.name.clone(), self.cargo_member(member, rust, kind)?));
        }

        let codegen = self.manifest.codegen_for(&self.settings.words.target);
        Ok(Workspace {
            members: out,
            // One profile, because a Nikaia build has no dev/release division
            // to map onto: `[build.<target>]` is the project's single statement
            // about output size and speed, and `dev` is the profile Cargo uses
            // when nothing asks otherwise.
            profile_name: "dev".to_string(),
            profile: Profile {
                opt_level: codegen.get("opt-level").cloned(),
                lto: codegen.get("lto").cloned(),
                codegen_units: codegen.get("codegen-units").cloned(),
                incremental: codegen.get("incremental").cloned(),
                debug: codegen.get("debug").cloned(),
                panic: Some(self.settings.panic_strategy().to_string()),
            },
        })
    }

    /// What the package at `root` is called as a crate - its own `[package]
    /// name`, which is what every other crate of the build names it by (D2).
    fn crate_named(&self, root: &Path) -> Result<String> {
        let manifest = Manifest::read(&root.join("nikaia.toml"))?;
        Ok(manifest
            .package_name()
            .ok_or_else(|| {
                refused!(
                    "{}/nikaia.toml has no `[package] name`. Add one: every package needs \
                     a name.",
                    root.display()
                )
            })?
            .to_string())
    }

    /// What this build asked the code generator for, as the compiled-`std` cache
    /// keys it (ADR-002 D4).
    ///
    /// The same two sources the Cargo profile above is written from - the
    /// machine's `[build.<target>]` table and the panic strategy that follows
    /// from the machine - because it is exactly those that decide what `std`'s
    /// machine code looks like.
    fn codegen(&self) -> Codegen {
        Codegen::new(
            &self.manifest.codegen_for(&self.settings.words.target),
            &self.settings.panic_strategy(),
        )
    }

    /// Build the project, or build and run it.
    ///
    /// The lowering happens twice and that is deliberate: **here**, so the
    /// checks and the ledger are the compiler's own output rather than
    /// something buried in a `cargo` subprocess, and again inside the wrapper,
    /// where it is what `rustc` is actually handed. Both go through [`lower`]
    /// and the second is a cache hit, so the work is done once even though the
    /// decision is made in two places.
    #[allow(clippy::too_many_arguments)]
    pub fn drive(
        &self,
        subcommand: &str,
        program_args: &[String],
        no_cache: bool,
        locked: bool,
        want: Explain,
        // **The list this build reads under** (ADR-310 D5), or `None` for D1.
        allowlist: Option<&Path>,
    ) -> Result<i32> {
        self.drive_to(subcommand, program_args, no_cache, locked, want, allowlist)
            .map(|(code, _)| code)
    }

    /// [`Project::drive`], handing back the binary the build made as well -
    /// which is what `nikaia test` runs, once per test.
    pub fn drive_to(
        &self,
        subcommand: &str,
        program_args: &[String],
        no_cache: bool,
        locked: bool,
        want: Explain,
        allowlist: Option<&Path>,
    ) -> Result<(i32, Option<PathBuf>)> {
        let entry = self.entry();
        if !entry.is_file() {
            refuse!(
                "{} is not there. A project's entry point is `{ENTRY}`.",
                entry.display()
            );
        }

        // Before the build, because an explanation is about the program and a
        // build that reuses a cached lowering still answers the question. The
        // package is read a second time here rather than threaded out of
        // `lower`: reading it is parsing, and paying for it only when somebody
        // asked is cheaper than reshaping the build for a flag nobody usually
        // passes.
        if want.asked() {
            let program = modules::Program::read_with(&entry, &self.packages()?)?;
            let reads = reads_for(&Layout::resolve(&self.entry()), allowlist)?;
            explain(&program, &self.settings, want, &reads)?;
        }

        // Every package of this build, resolved before anything is read: a
        // dependency that is not there, or two names for one path, is a
        // statement about the project and should not wait for a lowering.
        let members = self.members()?;

        // **Every member is lowered here**, for the reason the doc comment
        // above gives for the entry: the checks and the diagnostics are the
        // compiler's own output rather than something buried in a `cargo`
        // subprocess. Each is lowered again inside the wrapper and each of
        // those is a cache hit, so a package is compiled once however many
        // crates now ask about it.
        //
        // **Dependencies first** ([ADR-100](../../docs/specification/adr/adr-100.md)
        // D5). A package's ledger is a build input of everything that depends
        // on it: `http`'s own build has `deeper` in view and a consumer's does
        // not (ADR-286 D21), so the consumer has to read the answer `http`
        // computed rather than derive a worse one. `members_of` is
        // breadth-first from the entry, which is the wrong end and is not a
        // topological order once a package is reached two ways — so the order
        // is computed here, and this loop is where the two builds of one
        // package stop being able to disagree.
        let order = dependencies_first(&members);
        let mut rust: Vec<Option<String>> = vec![None; members.len()];
        // **`<package>.h`**, the entry's, where this build makes a library
        // (ADR-284 D12).
        let mut header: Option<String> = None;
        let mut python: Option<String> = None;
        let mut node: Option<String> = None;
        let mut link_flags: Vec<String> = Vec::new();
        for at in order {
            let member = &members[at];
            let outer = WARNED.with(|warned| warned.replace(self.warned));
            let shown = match &self.written {
                Some(written) if canonical(&member.entry) == canonical(&self.entry()) => {
                    Some(written.display().to_string())
                }
                _ => None,
            };
            let shown_outer = SHOWN_AS.with(|cell| cell.replace(shown));
            let lowered = lower_reading(
                &member.entry,
                &self.settings,
                no_cache,
                &member.dependencies,
                allowlist,
            );
            SHOWN_AS.with(|cell| cell.replace(shown_outer));
            WARNED.with(|warned| warned.set(outer));
            let lowered = lowered?;
            // **The libraries this package names, and how it finds them**
            // (ADR-324 D2-D4): both ways before anything is linked, and the
            // flags from `pkg-config` before `cargo` runs.
            let own: Vec<PathBuf> = lowered
                .sources
                .iter()
                .filter(|s| {
                    s.starts_with(&member.root)
                        && !members.iter().any(|other| {
                            other.root != member.root
                                && other.root.starts_with(&member.root)
                                && s.starts_with(&other.root)
                        })
                })
                .cloned()
                .collect();
            let declared = crate::libraries::declared(&own)?;
            crate::libraries::named_both_ways(
                &member.name,
                &member.root.join("nikaia.toml"),
                member.manifest.libraries(),
                &declared,
            )?;
            for flag in crate::libraries::link_flags(&member.name, member.manifest.libraries())? {
                if !link_flags.contains(&flag) {
                    link_flags.push(flag);
                }
            }
            // **Each package's ledger in that package's own root**
            // (Part III 13.5, ADR-100 D1): written here so the consumers
            // lowered after it read it, and committed with the package the way
            // a lockfile is. Under `--locked` this is D4 — each is compared
            // byte for byte rather than rewritten, which is the check CI wants
            // and the one a development build must not pay for.
            //
            // **Not the entry's, in `nikaia test`'s test build** (#534): it
            // lowers the `test` blocks too, and they are no contract of the
            // package's. A consumer reads the ledger the program's build
            // writes, and `nikaia test` in a package must leave its committed
            // files as they were.
            if !(at == 0 && self.settings.words.tests && self.variant.is_some()) {
                write_ledger(
                    &member.root.join("nikaia.contracts"),
                    &lowered.ledger,
                    locked,
                )?;
                write_ledger(
                    &member.root.join("nikaia.derived"),
                    &lowered.derived,
                    locked,
                )?;
                write_proofs(&member.root, lowered.proofs.as_ref(), locked)?;
            }
            // **The note is the entry's** (ADR-288 D29): what a dependency's
            // author could promise is theirs to hear, in their own build.
            if at == 0 {
                eprint!("{}", lowered.notes);
                header = lowered.header.clone();
                python = lowered.python.clone();
                node = lowered.node.clone();
            }
            rust[at] = Some(lowered.rust);
        }
        // Back into `members_of`'s own order, which is what the generated
        // workspace is written in.
        let rust: Vec<String> = rust
            .into_iter()
            .map(|r| r.expect("every member is lowered"))
            .collect();

        // **The generated Rust, before Cargo starts** (#280). The wrapper
        // writes the same file while `rustc` runs, and Cargo dates the
        // dependency file to when the invocation *started*: a source written
        // after that looks edited during the build, so a new project was
        // compiled again on its second build with nothing changed. Written
        // here, the wrapper's write finds the bytes already there and touches
        // nothing. The path is the wrapper's: the generated directory, then
        // the crate's name as Cargo passes it, `-` made `_`.
        for (member, rust) in members.iter().zip(&rust) {
            let crate_name = member.name.replace('-', "_");
            write_if_changed(
                &self
                    .gen_dir()
                    .join(&crate_name)
                    .join(format!("{crate_name}.rs")),
                rust,
            )?;
        }

        // **Before `cargo`**, and written only when it differs, so an
        // unchanged switch does not dirty the package every build. What is in
        // it is what the cache keys on, so the two cannot disagree about which
        // switches decide an artifact.
        let switches = self.switches_path();
        if let Some(dir) = switches.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let choices = self.settings.choices();
        // **And the libraries' flags**, which change what the program is
        // linked against and nothing Cargo watches.
        //
        // **And this compiler's identity** (ADR-002 D4): the wrapper that
        // lowers the program is this compiler, and Cargo fingerprints the
        // wrapper by its path, not by what it writes. The directory the program
        // is built in is shared and outlives an edit to the compiler, so this
        // line is what makes the program's crates stale after one - and only
        // them, not `std` and what is under it, which the compiler never wrote.
        write_if_changed(
            &switches,
            &format!(
                "{}\n{}\n{}\n{}\n",
                choices.build,
                choices.backend,
                link_flags.join(" "),
                env!("NIKAIA_COMPILER"),
            ),
        )?;

        // **Before Cargo is handed anything** (ADR-288 D27): its own answer
        // about the floor names a package the author never wrote, which is
        // Part III C.1's class.
        toolchain_is_new_enough(crate::emit::RUST_FLOOR)?;
        // **The machine named is the machine built for** (ADR-037 D1): the
        // triple goes to Cargo as `--target`, at the host too, so there is one
        // path and not a second one that only a cross build takes. Where it is
        // not the host, the toolchain may not have its `std`, and Cargo's own
        // answer is `can't find crate for core` - so that is asked first.
        let target = self.settings.build.target;
        if Some(target) != crate::emit::host_target() {
            toolchain_can_build_for(target.triple())?;
        }
        let target_args = ["--target".to_string(), target.triple().to_string()];

        let manifest = self
            .cargo_workspace(&members, &rust)?
            .write_to(&self.build_dir())?;

        // **A `run` of a program that needs nothing but `std` skips Cargo**
        // (`crate::run_fast`): `rustc` compiles it against `std` as a dynamic
        // library. The workspace above is still written, for whoever reads it;
        // where that library cannot be built, Cargo's path follows.
        if subcommand == "run"
            && self.runs_without_cargo(&members, &link_flags)
            && let Some(built) = crate::run_fast::compile(
                &self.own_dir().join("fast"),
                &members[0].name.replace('-', "_"),
                &rust[0],
                &self
                    .gen_dir()
                    .join(members[0].name.replace('-', "_"))
                    .join(format!("{}.rs", members[0].name.replace('-', "_"))),
            )?
        {
            self.report(&built.messages, allowlist)?;
            return match built.binary {
                // Cargo's own code for a crate that did not compile.
                None => Ok((101, None)),
                Some(binary) => Ok((run_directly(&binary, program_args)?, Some(binary))),
            };
        }

        let mut env = self.settings.as_env();
        env.push((WRAPPER_MARKER.to_string(), OsString::from("1")));
        env.push((GEN_DIR_VAR.to_string(), self.gen_dir().into_os_string()));
        // The program's crate is named for its project in the shared directory
        // (#495), and its generated file keeps its package's name.
        if std::env::var_os("CARGO_TARGET_DIR").is_none() {
            env.push((
                PROGRAM_TAG_VAR.to_string(),
                OsString::from(format!("_{}", self.root_tag())),
            ));
        }
        env.push((
            LINK_VAR.to_string(),
            OsString::from(link_flags.join("\u{1f}")),
        ));
        if no_cache {
            env.push((NO_CACHE_VAR.to_string(), OsString::from("1")));
        }
        // The entry it names and the name, `\u{1f}` between: only that file is
        // said under another name, not a package the program depends on.
        if let Some(written) = &self.written {
            env.push((
                SHOWN_AS_VAR.to_string(),
                OsString::from(format!(
                    "{}\u{1f}{}",
                    canonical(&self.entry()).display(),
                    written.display()
                )),
            ));
        }
        // **Absolute**, because the wrapper runs in a directory Cargo chose.
        if let Some(list) = allowlist {
            let list = list
                .canonicalize()
                .with_context(|| format!("reading the allowlist `{}`", list.display()))?;
            env.push((READS_VAR.to_string(), list.into_os_string()));
        }

        let rustflags = match self.settings.cpu_rustflags()? {
            Some(flags) => {
                env.push(("CARGO_ENCODED_RUSTFLAGS".to_string(), flags.into()));
                true
            }
            None => false,
        };
        let cargo = Cargo {
            manifest,
            // **The compiled-`std` cache** (ADR-002 D4). Cargo builds into a
            // directory in the user's cache named by `Key::sysroot`, so the
            // second project on this machine links the `std` the first one built
            // instead of compiling it and everything under it again. It cannot
            // live under this project's `target/`, because "shared between
            // projects" is the whole of what it is for.
            //
            // `CARGO_TARGET_DIR` still wins where it is set. It is Cargo's own
            // documented override and a build that asked for a directory has to
            // get it.
            target_dir: match std::env::var_os("CARGO_TARGET_DIR") {
                Some(_) => None,
                None => {
                    let tree = Sysroot::resolve().rlib_cache(
                        &self.settings.words.target,
                        &self.codegen(),
                        &self.settings.words.reentrancy_check,
                        &self.settings.cpu_key()?,
                    );
                    // What projects that are gone left there goes (ADR-021 D12.5).
                    orchestrator::cache::sweep_programs(&tree, &self.root_tag(), &self.root);
                    Some(tree)
                }
            },
            wrapper: std::env::current_exe().context("finding this compiler's own path")?,
            env,
            config: self.settings.cpu_config(),
            unset: if rustflags {
                vec!["RUSTFLAGS".to_string()]
            } else {
                Vec::new()
            },
        };

        // **`build` first, always, and its diagnostics read rather than
        // relayed** (ADR-005 D7, Part III C.1). Cargo's stderr is Rust about a
        // file the author never opened; `--message-format=json` is the channel
        // the translator can read, and it is only usable for `build` - a `run`
        // writes the *program's* output to the same stdout.
        //
        // So a `nikaia run` is a build and then a run, and the run is a no-op
        // rebuild. That is the same shape the lowering already has (above): the
        // work happens once, and the decision is made where it can be reported.
        //
        if self.settings.library() && subcommand == "run" {
            refuse!(
                "This package is built as a library (`artifact = \"c-library\"`), and a library \
                 has nothing to run: `nikaia build` makes it, and a C program calls it."
            );
        }
        if self.settings.library() && header.is_none() {
            return Err(crate::cexport::nothing_exported());
        }
        // **A `run` shows the program's output and nothing of Cargo's.** Its
        // progress lines ("Compiling", "Finished") are not about the program,
        // and on a terminal they mix into what the program prints. `--quiet`
        // keeps what Cargo cannot do (a manifest it cannot read) on stderr;
        // the diagnostics arrive on the JSON channel either way.
        let mut build_args = target_args.to_vec();
        if subcommand == "run" {
            build_args.push("--quiet".to_string());
        }
        let (mut code, messages) = cargo.messages("build", &build_args)?;
        // Cargo is done with what this build added to the user cache, so its
        // size is known: hold the cache to its limit before a `run` starts the
        // program (#724).
        orchestrator::cache::settle();
        self.report(&messages, allowlist)?;
        // **The library and its header, together where a C build looks**
        // (ADR-284 D4, D12): `target/nikaia/c-library/`.
        if code == 0
            && let Some(header) = &header
        {
            let made = self.own_dir().join("c-library");
            std::fs::create_dir_all(&made).with_context(|| format!("making {}", made.display()))?;
            let package = self
                .manifest
                .package_name()
                .unwrap_or("program")
                .to_string();
            write_if_changed(&made.join(format!("{package}.h")), header)?;
            // **The binding asked for, over it** (D26): `<package>/__init__.py`
            // beside the library it loads.
            if self.settings.words.bind.as_deref() == Some("python")
                && let Some(python) = &python
            {
                let module = made.join(package.replace('-', "_"));
                std::fs::create_dir_all(&module)
                    .with_context(|| format!("making {}", module.display()))?;
                write_if_changed(&module.join("__init__.py"), python)?;
                println!("binding: {}", module.join("__init__.py").display());
            }
            // `node/<package>.c` and its `binding.gyp` (D28), which the
            // host's toolchain compiles against the library.
            if self.settings.words.bind.as_deref() == Some("node")
                && let Some(node) = &node
            {
                let module = made.join("node");
                std::fs::create_dir_all(&module)
                    .with_context(|| format!("making {}", module.display()))?;
                write_if_changed(&module.join(format!("{package}.c")), node)?;
                write_if_changed(
                    &module.join("binding.gyp"),
                    &crate::cexport::node_gyp(&package),
                )?;
                println!("binding: {}", module.join(format!("{package}.c")).display());
            }
            for built in libraries_in(&messages) {
                if let Some(name) = built.file_name() {
                    std::fs::copy(&built, made.join(name))
                        .with_context(|| format!("copying {}", built.display()))?;
                }
            }
            println!("library: {}", made.display());
        }
        let binary = match (executable_in(&messages), &cargo.target_dir) {
            (Some(built), Some(_)) => Some(self.own_copy(&built)?),
            (built, _) => built,
        };

        // Cargo has resolved by now, and only now: the versions do not exist
        // before it ran. A build that failed resolved nothing worth recording.
        //
        // **Before the program starts**, because `run` hands this process over
        // to it (#281) and nothing after that point runs. What is recorded is
        // what the *build* resolved, so the program's own exit status was never
        // the right condition for it.
        if code == 0
            && let Err(error) = self.record_resolved_dependencies(no_cache)
        {
            // D12: the record costs the *next* reader some information. It
            // never costs this build, which has already succeeded.
            eprintln!(
                "warning: the resolved dependency versions could not be recorded \
                     in nikaia.lock: {error:#}"
            );
        }

        if code == 0 && subcommand == "run" {
            // **The built program is run directly, and that is Part III C.1
            // rather than a shortcut.** `cargo run` is a second invocation that
            // cannot use the JSON channel - the program's own output is on that
            // stdout - so Cargo renders its **cached** diagnostics to stderr
            // while it checks freshness, and a program with a warning saw it
            // twice: once in this language's words from `report` above, and
            // once as `rustc` about `target/nikaia/gen/….rs`. Measured on
            // Part I 2.3's own example.
            //
            // **The path is read and not computed.** Cargo decides where a
            // binary lands from the target directory, the profile and
            // `CARGO_TARGET_DIR`, and replicating those rules here would be a
            // second place that has to agree with Cargo forever. The build
            // above already captured the answer: a `compiler-artifact` line
            // whose `executable` is not null.
            //
            // Where there is no such line the old path is taken, so this can
            // only remove a duplicate and never lose a run.
            code = match &binary {
                Some(binary) => run_directly(binary, program_args)?,
                None => cargo.run("run", &build_args, program_args)?,
            };
        }
        Ok((code, binary))
    }

    /// Say what the backend said, against the `.nika` line it was about.
    ///
    /// Part III C.1's Iron Rule: an untranslated backend error reaching the user
    /// is a bug in this compiler. Until this existed the project build had no
    /// interception at all - `cargo`'s stderr *was* the user interface - so every
    /// class ADR-005 D7 enumerates, `E0277` among them since the structural
    /// `Send` check landed, arrived as Rust about `target/nikaia/gen/….rs`.
    ///
    /// **The position is translated and the text is not**, which is ADR-300's
    /// own choice and is honest for most of what arrives: the lowering is name
    /// for name (ADR-296 D17), so "`xs` does not live long enough" is already a
    /// sentence about the Nikaia source and only its place was wrong. It is
    /// *not* honest for a trait-bound error, whose every noun is a Rust type the
    /// author did not write - and rewriting one into Nikaia vocabulary is a
    /// second question, recorded in ADR-005 D7 rather than answered here. What
    /// the frontend can decide about a crossing it now refuses itself, with
    /// `NK2501`/`NK2502` and Nikaia words.
    ///
    /// The map is rebuilt rather than carried: the lowering is deterministic, so
    /// a second one gives the same map (ADR-300), and this way the cost is paid
    /// only by a build the backend had something to say about.
    fn report(&self, messages: &str, allowlist: Option<&Path>) -> Result<()> {
        if messages
            .lines()
            .all(|line| !line.contains("\"compiler-message\""))
        {
            return Ok(());
        }

        // **With the packages**, or a `use` line would be read against an empty
        // set and the translation of somebody else's error would be a refusal of
        // a program that is fine.
        // **The program the build lowered**, test build included (ADR-269
        // D1): a test build's entry has its dispatcher, and a map rebuilt
        // from the other program places a warning in the wrong file.
        let mut program = match self.settings.words.tests {
            true => modules::Program::read_for_tests(&self.entry(), &self.packages()?, true)?,
            false => modules::Program::read_with(&self.entry(), &self.packages()?)?,
        };
        // **The same reads the build lowered under** (ADR-310, ADR-177). This
        // lowering has to be the lowering, or an item the build wrote a `const`
        // for is refused here and the backend's message never arrives.
        let layout = Layout::resolve(&self.entry());
        let reads = reads_for(&layout, allowlist)?;
        // …and the same descriptions (ADR-290 D20), for the same reason.
        let foreign = match layout.in_project {
            true => foreign_of(&layout.root),
            false => project_members::project_members_no_foreign(),
        };
        program.described = foreign.descriptions.clone();
        let lowered = program.emit_reading(self.settings.build, &reads)?;
        let sources: Vec<&str> = program.sources();
        let entry = canonical(&self.entry());
        let paths: Vec<String> = program
            .units
            .iter()
            .map(|unit| match &self.written {
                Some(written) if canonical(&unit.path) == entry => written.display().to_string(),
                _ => unit.path.display().to_string(),
            })
            .collect();
        let generated = self.gen_dir().display().to_string();

        let mut translated: Vec<_> = diagnostics::translate_units(messages, &lowered.map, &sources)
            .into_iter()
            .filter(diagnostics::is_about_the_program)
            .collect();
        // **A boundary mismatch is said as a stale ledger** (ADR-100 D6,
        // ADR-290 D21): the packages this program depends on and the crates it
        // describes are where a ledger the lowering believed can disagree with
        // the code below it.
        let boundaries: Vec<diagnostics::Boundary> = self
            .packages()?
            .into_iter()
            .map(|package| diagnostics::Boundary {
                word: package.name,
                described: false,
                moved: Vec::new(),
            })
            .chain(foreign.described.iter().map(|word| diagnostics::Boundary {
                word: word.clone(),
                described: true,
                moved: foreign.moved.get(word).cloned().unwrap_or_default(),
            }))
            .collect();
        for diagnostic in &mut translated {
            let at = diagnostic.location.as_ref().map_or(0, |l| l.unit);
            diagnostics::a_stale_boundary(diagnostic, sources[at], &boundaries);
        }

        for diagnostic in &translated {
            let at = diagnostic.location.as_ref().map_or(0, |l| l.unit);
            eprint!(
                "{}",
                diagnostics::render(diagnostic, &paths[at], sources[at], &generated)
            );
        }

        // **The backend's own words, kept** (ADR-056 D2). Shown above because
        // the reader is being told this is not their mistake, and written here
        // because a bug report is made later and from a file.
        if let Some(log) = diagnostics::log_internal(&translated, &self.gen_dir()) {
            eprintln!("note: the backend's own messages are in {}", log.display());
        }
        Ok(())
    }

    /// Copy the versions Cargo chose into `nikaia.lock` (ADR-021 D2).
    ///
    /// The toolchain does not resolve versions and must not (ADR-002 D1), so
    /// this reads Cargo's `Cargo.lock` for the generated package rather than
    /// computing anything - D4's distinction exactly: `nikaia.toml` declares
    /// `regex = { type = "rust", version = "1.5" }`, and this records the
    /// `1.13.1` that turned into.
    ///
    /// Two conditions, and neither is tidiness. **`--no-cache` writes nothing**,
    /// because `nikaia.lock` is the cache's own record and a run told not to
    /// keep one must not leave a file behind. **A lockfile that is not there is
    /// not created**, because a file holding resolved versions and no unit
    /// hashes would answer D2's question with half an answer while looking like
    /// a whole one.
    ///
    /// Nothing is written when the versions are unchanged, which is the common
    /// case: a committed file must not turn up as modified after a build that
    /// changed nothing about it.
    fn record_resolved_dependencies(&self, no_cache: bool) -> Result<()> {
        if no_cache {
            return Ok(());
        }
        // The same walk the cache and `Project::open` use, so the three can
        // never disagree about where the record lives.
        let lock_path = Layout::resolve(&self.root.join("nikaia.toml")).lock;
        if !lock_path.is_file() {
            return Ok(());
        }

        let name = self.manifest.package_name().ok_or_else(|| {
            refused!("The project has no `[package] name`. Add one to `nikaia.toml`.")
        })?;
        let resolved = resolved_versions(&self.build_dir().join("Cargo.lock"), name)?;

        let mut lock = Lockfile::load(
            &lock_path,
            env!("NIKAIA_RUSTC_VERSION"),
            env!("NIKAIA_COMPILER"),
        )?;
        if lock.dependencies == resolved {
            return Ok(());
        }
        lock.dependencies = resolved;
        lock.save(&lock_path)
    }
}

impl Project {
    /// The program's name for Cargo: `project_settings_program_name`.
    fn program_name(&self, package: &str) -> String {
        project_settings::project_settings_program_name(
            &self.root,
            &self.variant,
            package.to_string(),
            &SettingsHost,
        )
    }

    /// What sets this project's program apart in the shared directory:
    /// `project_settings_root_tag`.
    fn root_tag(&self) -> String {
        project_settings::project_settings_root_tag(&self.root, &self.variant, &SettingsHost)
    }

    /// The program a build in the shared directory produced, under the
    /// project's own name: `project_settings_own_copy`, its refusal as an
    /// error.
    fn own_copy(&self, built: &Path) -> Result<PathBuf> {
        project_settings::project_settings_own_copy(&self.root, &self.variant, built, &SettingsHost)
            .map_err(refusal)
    }
}

/// What this particular program links against, out of the three crates the
/// emitter can name.
///
/// Declared because the *emitted Rust* names them, not because a program might:
/// a program with no grammar in it depends on nothing at all and builds without
/// fetching anything, and a manifest that declared three crates regardless would
/// make every project pay for the ones it does not use.
///
/// `std` is reached **by path into the sysroot** (ADR-002 D4). It is not
/// published, so there is no registry to reach it through; what changed is that
/// the path is a sysroot's rather than a checkout's, and that the sources there
/// need nothing but `rustc` to build.
///
/// The other two versions are baked into this binary by `build.rs` rather than
/// read from a workspace manifest at run time. A sysroot is not a Cargo
/// workspace and has no `[workspace.dependencies]` above it - and these are the
/// *compiler's* versions anyway, since it is the emitter that writes
/// `winnow::Parser` into a program. Read at the compiler's build time rather
/// than written down twice: a program that linked a different `winnow` from the
/// one `winnow-grammar` was built against is a type error at every `Stream`
/// bound, so the two must not be able to drift apart.
/// The program Cargo built, as its own build said where it was.
///
/// **Read rather than computed.** Cargo decides where a binary lands from the
/// target directory, the profile and `CARGO_TARGET_DIR`; a second place that
/// worked those rules out would have to keep agreeing with Cargo forever. A
/// `--message-format=json` build says it outright, on the `compiler-artifact`
/// line whose `executable` is not null - and this compiler already captures that
/// output to read the diagnostics.
///
/// `None` where nothing says, which is not an error: the caller falls back to
/// `cargo run` and the build behaves as it did before.
/// The shared and the static library a `c-library` build made (ADR-284 D4).
fn libraries_in(messages: &str) -> Vec<std::path::PathBuf> {
    messages
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["reason"] == "compiler-artifact")
        .filter(|value| {
            value["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|k| k == "cdylib" || k == "staticlib"))
        })
        .flat_map(|value| {
            value["filenames"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|f| f.as_str().map(std::path::PathBuf::from))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn executable_in(messages: &str) -> Option<std::path::PathBuf> {
    messages
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["reason"] == "compiler-artifact")
        .filter_map(|value| value["executable"].as_str().map(std::path::PathBuf::from))
        .next_back()
}

/// The package's output tests, by name (`tools/project_tests.nika`).
pub fn output_tests(root: &Path) -> Result<Vec<OutputTest>> {
    block_on(project_tests::output_tests(root)).map_err(|e| anyhow!("{e}"))
}

/// **A directory of a test's own, removed when it goes**
/// ([ADR-247](../../docs/specification/adr/adr-247.md) D1).
struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Result<Sandbox> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static MADE: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "nikaia-test-{}-{}",
            std::process::id(),
            MADE.fetch_add(1, Ordering::Relaxed)
        ));
        // A directory left by a run that was killed has this name only if the
        // process id came round again; what is in it is that run's, not ours.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("making a directory for a test at {}", dir.display()))?;
        Ok(Sandbox(dir))
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Run a built program with its arguments and its standard input, in `dir`,
/// with `own` as the directory `fs::scratch` makes its directories in
/// ([ADR-247](../../docs/specification/adr/adr-247.md) D5).
fn outcome_of(
    binary: &Path,
    args: &[String],
    stdin: &[u8],
    dir: &Path,
    own: &Path,
) -> Result<std::process::Output> {
    use std::io::Write;
    let mut child = std::process::Command::new(binary)
        .current_dir(dir)
        .env(nikaia_std::fs::TEST_DIR_VAR, own)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("running {}", binary.display()))?;
    if let Some(mut input) = child.stdin.take() {
        // A program that stops reading early closes the pipe, which is its
        // business and not a failure of the run.
        let _ = input.write_all(stdin);
    }
    child
        .wait_with_output()
        .with_context(|| format!("running {}", binary.display()))
}

/// What `nikaia test` runs: the package's `test` blocks, against the test
/// build, and its output tests, against the program - each built once per
/// setting asked for.
pub struct Suite<'a> {
    pub tests: &'a [modules::TestCase],
    /// One test build per setting, where there are blocks.
    pub test_builds: &'a [(String, PathBuf)],
    pub outputs: &'a [OutputTest],
    /// One build of the program per setting, where there are output tests.
    pub programs: &'a [(String, PathBuf)],
    pub root: &'a Path,
    /// The test programs, `tests/NAME.nika` (ADR-269 D22).
    pub programs_of_their_own: &'a [ProgramTest],
    /// The `user_parallelism` settings each runs at.
    pub settings: &'a [String],
    /// `--bless` ([ADR-247](../../docs/specification/adr/adr-247.md) D3).
    pub bless: bool,
}

/// **Run each test in a process and a directory of its own, and say how each
/// went** ([ADR-269](../../docs/specification/adr/adr-269.md) D1, D12, D13;
/// [ADR-247](../../docs/specification/adr/adr-247.md)).
///
/// A `test` block fails on an error that leaves it, a false `assert` or a
/// `panic` - each of which ends its process unsuccessfully. An output test
/// fails where the program does not end successfully, or prints anything but
/// its `.stdout`, or leaves any file its `.out/` names other than byte for
/// byte. At two settings either fails as well where the two outcomes differ,
/// because the meaning of a program is the same at both (Part I 1.2) and a
/// difference is a fault of this compiler. A failing test does not stop the
/// others.
pub fn run_tests(suite: &Suite<'_>) -> Result<i32> {
    let total = suite.tests.len() + suite.outputs.len() + suite.programs_of_their_own.len();
    project_tests::project_tests_running(total as i64);
    let mut failed: Vec<Failed<'_>> = Vec::new();
    for (k, test) in suite.tests.iter().enumerate() {
        let mut outcomes = Vec::new();
        for (setting, binary) in suite.test_builds {
            // In the package's directory, as Go's tests run in theirs, so a
            // test reads what the package holds; what it writes goes under
            // `fs::scratch()`, inside `own`.
            let own = Sandbox::new()?;
            let out = outcome_of(binary, &[k.to_string()], &[], suite.root, &own.0)?;
            let ended_well = out.status.success();
            outcomes.push((
                setting.clone(),
                Outcome {
                    ended_well,
                    passed: ended_well,
                    stdout: out.stdout.into(),
                    stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                    files: Vec::new(),
                },
            ));
        }
        let label = format!(
            "test \"{}\" ({}:{})",
            test.title,
            project_tests::shown_path(&test.path, suite.root),
            test.line
        );
        if !project_tests::project_tests_verdict(&label, &outcomes) {
            failed.push((label, outcomes, None));
        }
    }
    for output in suite.outputs {
        let mut outcomes = Vec::new();
        for (setting, binary) in suite.programs {
            let own = Sandbox::new()?;
            if let Some(inputs) = &output.inputs {
                block_on(project_tests::copy_into(inputs, &own.0)).map_err(|e| anyhow!("{e}"))?;
            }
            let out = outcome_of(binary, &output.args, &output.stdin, &own.0, &own.0)?;
            outcomes.push((
                setting.clone(),
                block_on(project_tests::project_tests_output_outcome(
                    output,
                    &own.0,
                    out.status.success(),
                    out.stdout.into(),
                    String::from_utf8_lossy(&out.stderr).into_owned(),
                )),
            ));
        }
        let label = project_tests::project_tests_output_label(output);
        if suite.bless {
            let blessed = block_on(project_tests::project_tests_bless(output, &outcomes))
                .map_err(|e| anyhow!("{e}"))?;
            if !project_tests::project_tests_blessed_line(&label, &blessed) {
                failed.push((label, outcomes, Some(output)));
            }
            continue;
        }
        if !project_tests::project_tests_verdict(&label, &outcomes) {
            failed.push((label, outcomes, Some(output)));
        }
    }
    run_program_tests(suite, |program, outcomes| {
        let label = project_tests::project_tests_program_label(program);
        if suite.bless {
            let blessed = block_on(project_tests::project_tests_bless_program(
                program, &outcomes,
            ))
            .map_err(|e| anyhow!("{e}"))?;
            if !project_tests::project_tests_blessed_line(&label, &blessed) {
                failed.push((label, outcomes, None));
            }
            return Ok(());
        }
        if !project_tests::project_tests_verdict(&label, &outcomes) {
            failed.push((label, outcomes, None));
        }
        Ok(())
    })?;
    if !failed.is_empty() {
        println!("\nfailures:");
        for (label, outcomes, output) in &failed {
            println!("\n--- {label}");
            let agree = project_tests::project_tests_agree(outcomes);
            let shown = match agree {
                true => &outcomes[..1],
                false => &outcomes[..],
            };
            for (setting, outcome) in shown {
                if !agree {
                    println!("at user-parallelism = {setting}:");
                }
                match output {
                    Some(output) => project_tests::project_tests_explain_output(output, outcome),
                    None => print!("{}", String::from_utf8_lossy(&outcome.stdout)),
                }
                eprint!("{}", outcome.stderr);
            }
        }
    }
    Ok(project_tests::project_tests_result(
        total as i64,
        failed.len() as i64,
    ))
}

/// The variable that sets how many test programs build and run at once; `1`
/// runs them one after another.
const TEST_JOBS_VAR: &str = "NIKAIA_TEST_JOBS";

/// **The test programs, on workers of their own** (as many as
/// [`TEST_JOBS_VAR`] says, else as the machine has cores), each worker with a
/// place per setting that it keeps from one program to the next
/// ([`run_program_test`]). `done` is called on this thread with each
/// program's outcomes at every setting, **in the order of the programs**,
/// whichever finished first: what is printed is the same as one after
/// another. An error stops the workers from taking more and is returned once
/// the programs before it are done.
fn run_program_tests<'a>(
    suite: &Suite<'a>,
    mut done: impl FnMut(&'a ProgramTest, Vec<(String, Outcome)>) -> Result<()>,
) -> Result<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let (programs, root, settings) = (suite.programs_of_their_own, suite.root, suite.settings);
    let jobs = std::env::var(TEST_JOBS_VAR)
        .ok()
        .and_then(|jobs| jobs.trim().parse::<usize>().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
        .clamp(1, programs.len().max(1));
    let places = (0..jobs)
        .map(|_| settings.iter().map(|_| Sandbox::new()).collect())
        .collect::<Result<Vec<Vec<_>>>>()?;
    let next = AtomicUsize::new(0);
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for place in &places {
            let (send, next) = (send.clone(), &next);
            scope.spawn(move || {
                loop {
                    let k = next.fetch_add(1, Ordering::SeqCst);
                    let Some(program) = programs.get(k) else {
                        break;
                    };
                    let outcomes = settings
                        .iter()
                        .zip(place)
                        .map(|(setting, place)| {
                            run_program_test(program, root, setting, &place.0)
                                .map(|outcome| (setting.clone(), outcome))
                        })
                        .collect::<Result<Vec<_>>>();
                    if send.send((k, outcomes)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(send);
        let mut finished = BTreeMap::new();
        let mut shown = 0;
        for (k, outcomes) in receive {
            finished.insert(k, outcomes);
            while let Some(outcomes) = finished.remove(&shown) {
                let reported = outcomes.and_then(|outcomes| done(&programs[shown], outcomes));
                if reported.is_err() {
                    next.store(programs.len(), Ordering::SeqCst);
                    return reported;
                }
                shown += 1;
            }
        }
        Ok(())
    })
}

/// The package's test programs, by name (`tools/project_tests.nika`).
pub fn program_tests(root: &Path) -> Result<Vec<ProgramTest>> {
    block_on(project_tests::program_tests(root)).map_err(|e| anyhow!("{e}"))
}

/// **Build and run one test program** in a package of its own, in a directory
/// of its own: the package it tests is its dependency, so `use` reaches it. A
/// refusal is the outcome (`ended_well` false, `stderr` the message with the
/// path written `tests/NAME.nika`); a program that builds is run there.
///
/// **The directory, `own`, is kept for the next program at this setting**,
/// with only the package's `target/` left in it: the package then stays at
/// one path, so Cargo finds the tested package it depends on built and
/// compiles only the program, where a new path each time made it compile the
/// tested package again for every program. What the program sees is what a
/// new directory held: `project/` with the manifest and `src/main.nika`, and
/// no ledger, lock or file a program before it left.
///
/// **The built program is run directly**, not through a second `nikaia run`:
/// that one would build again only to find nothing changed, and then do what
/// is done here - start the binary in the same directory, with the same
/// variables and no standard input, as a child of this process (its own
/// process replaced by the program, #281). What the run wrote before the
/// program started, the build's warnings, is the build's standard error with
/// Cargo kept quiet as `run` keeps it (`CARGO_TERM_QUIET`).
fn run_program_test(test: &ProgramTest, root: &Path, setting: &str, own: &Path) -> Result<Outcome> {
    let project = own.join("project");
    empty_but(own, "project")?;
    empty_but(&project, "target")?;
    std::fs::create_dir_all(project.join("src"))?;
    let source = test.dir.join(format!("{}.nika", test.name));
    std::fs::copy(&source, project.join(ENTRY))
        .with_context(|| format!("copying {}", source.display()))?;
    let dependency = match Manifest::read(&root.join("nikaia.toml"))?.package_name() {
        Some(name) => format!(
            "\n[dependencies]\n{name} = {{ path = {:?} }}\n",
            canonical(root)
        ),
        None => String::new(),
    };
    std::fs::write(
        project.join("nikaia.toml"),
        format!("[package]\nname = \"program_test\"\nversion = \"0.1.0\"\n{dependency}"),
    )?;
    let exe = std::env::current_exe().context("finding this compiler")?;
    let run = |verb: &str, scratch: &Path| -> Result<std::process::Output> {
        let mut command = std::process::Command::new(&exe);
        command
            .args([verb, "--user-parallelism", setting, "--project"])
            .arg(&project)
            .current_dir(own)
            .env(nikaia_std::fs::TEST_DIR_VAR, scratch)
            .env_remove("CARGO_TARGET_DIR")
            .stdin(std::process::Stdio::null());
        if verb == "build" {
            command.env("CARGO_TERM_QUIET", "true");
        }
        command
            .output()
            .with_context(|| format!("running {}", exe.display()))
    };
    let entry = project.join(ENTRY);
    let written = format!("tests/{}.nika", test.name);
    let relocate = |bytes: &[u8]| -> String {
        String::from_utf8_lossy(bytes)
            .replace(&entry.display().to_string(), &written)
            .replace(&canonical(&entry).display().to_string(), &written)
    };
    let built = run("build", own)?;
    if !built.status.success() {
        return Ok(project_tests::project_tests_refused_outcome(
            test,
            &relocate(&built.stderr),
        ));
    }
    // `project_settings_own_copy`'s place for the program, with no variant.
    let binary = project
        .join("target")
        .join("nikaia")
        .join("bin")
        .join(format!("program_test{}", std::env::consts::EXE_SUFFIX));
    let (out, said) = match binary.is_file() {
        true => {
            let out = std::process::Command::new(&binary)
                .current_dir(own)
                .env(nikaia_std::fs::TEST_DIR_VAR, own)
                .env_remove("CARGO_TARGET_DIR")
                .stdin(std::process::Stdio::null())
                .output()
                .with_context(|| format!("running {}", binary.display()))?;
            let mut said = built.stderr;
            said.extend_from_slice(&out.stderr);
            (out, said)
        }
        // Where the program is not where it is expected, `nikaia run` finds it.
        false => {
            let out = run("run", own)?;
            let said = out.stderr.clone();
            (out, said)
        }
    };
    Ok(project_tests::project_tests_ran_outcome(
        test,
        out.status.success(),
        out.stdout.into(),
        relocate(&said),
    ))
}

/// Remove everything in `dir` but the entry named `keep`.
fn empty_but(dir: &Path, keep: &str) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_name() == keep {
            continue;
        }
        let path = entry.path();
        match entry.file_type()?.is_dir() {
            true => std::fs::remove_dir_all(&path),
            false => std::fs::remove_file(&path),
        }
        .with_context(|| format!("emptying {}", path.display()))?;
    }
    Ok(())
}

/// A test that failed: what it is called, what each setting did, and the
/// output test it is, where it is one.
type Failed<'a> = (String, Vec<(String, Outcome)>, Option<&'a OutputTest>);

/// Run it, with the program's own arguments and every stream its own.
///
/// Nothing is captured and nothing is translated: what a running program writes
/// is the program's, and the last thing a compiler should do is stand between
/// the two.
///
/// **On Unix this process becomes the program** (#281). Started as a child, it
/// outlived a `nikaia run` that was killed - by a supervisor, by `timeout`, by
/// this repository's own tests - and kept its port. Replaced, it has this
/// process's id, its signals and its exit status, and there is nothing left
/// behind to orphan. What this compiler wrote is flushed first, since nothing
/// of this process runs afterwards to do it.
#[cfg(unix)]
fn run_directly(binary: &std::path::Path, args: &[String]) -> Result<i32> {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();
    // `exec` returns only when it failed to replace this process.
    let error = std::process::Command::new(binary).args(args).exec();
    Err(error).with_context(|| format!("running {}", binary.display()))
}

#[cfg(not(unix))]
fn run_directly(binary: &std::path::Path, args: &[String]) -> Result<i32> {
    let status = std::process::Command::new(binary)
        .args(args)
        .status()
        .with_context(|| format!("running {}", binary.display()))?;
    // A program killed by a signal has no exit code. `1` is what a shell would
    // report for one, and the alternative - claiming success - is the one answer
    // that must not be given.
    Ok(status.code().unwrap_or(1))
}

/// The same, for a program this compiler wrote for its own use — the parser a
/// grammar run compiles ([`crate::grammar_run`]).
///
/// **The same table a program gets**, rendered: a parser compiled against a
/// different `winnow` from the one the program links is a type error at every
/// `Stream`, and the point of compiling the generated parser at all is that it
/// *is* the program's parser.
pub fn runtime_dependencies_for(rust: &str) -> Vec<(String, String)> {
    runtime_dependencies(rust, crate::emit::ReentrancyCheck::Yes)
        .map(|table| {
            table
                .into_iter()
                .map(|(name, value)| (format!("{name:?}"), value.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// **The features the generated code tests in its own crate.** A `grammar!`
/// expansion writes `#[cfg(feature = "trace")]` into the crate that invokes it,
/// so that crate is the one that has to declare `trace`, or every build of a
/// program with a `grammar` carries `rustc`'s *unexpected `cfg` condition
/// value* (0.0.230, issue #163 before it). It forwards to `winnow-grammar`'s
/// own `trace`, as `nikaia` and `nikaia-std` do, so turning it on traces the
/// parser end to end.
fn runtime_features(dependencies: &BTreeMap<String, toml::Value>) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    if dependencies.contains_key("winnow-grammar") {
        out.insert(
            "trace".to_string(),
            vec!["winnow-grammar/trace".to_string()],
        );
    }
    out
}

fn runtime_dependencies(
    rust: &str,
    reentrancy_check: crate::emit::ReentrancyCheck,
) -> Result<BTreeMap<String, toml::Value>> {
    let mut out = BTreeMap::new();

    if rust.contains("nikaia_std") {
        let mut table = toml::Table::new();
        table.insert(
            "path".to_string(),
            toml::Value::String(Sysroot::resolve().std_dir().to_string_lossy().into_owned()),
        );
        // **A declined guarantee reaches the crate that carries it**
        // ([ADR-281](../../docs/specification/adr/adr-281.md) D36). The check
        // lives in `std`, so the switch is a Cargo feature: on by default, and
        // a program that declined it builds `std` without it. The compiled-`std`
        // cache keys on the word too (`Sysroot::rlib_cache`), or the second
        // project on this machine would link the first one's answer.
        if !reentrancy_check.is_on() {
            table.insert("default-features".to_string(), toml::Value::Boolean(false));
        }
        out.insert("nikaia-std".to_string(), toml::Value::Table(table));
    }

    // `winnow_grammar` contains `winnow`, so a grammar pulls in both - which is
    // right: the expansion writes `winnow::Parser` as well.
    for (crate_name, declaration) in [
        ("winnow-grammar", env!("NIKAIA_RUNTIME_WINNOW_GRAMMAR")),
        ("winnow", env!("NIKAIA_RUNTIME_WINNOW")),
    ] {
        if !rust.contains(&crate_name.replace('-', "_")) {
            continue;
        }
        let value: toml::Value = toml::from_str(&format!("value = {declaration}"))
            .map(|table: toml::Table| table["value"].clone())
            .with_context(|| {
                format!("the `{crate_name}` declaration this compiler was built with")
            })?;
        out.insert(crate_name.to_string(), value);
    }

    Ok(out)
}

/// The wrapper: what Cargo runs in `rustc`'s place, for workspace members.
///
/// Cargo calls this as `nikaia <rustc> <args…>`, for its own probes as well as
/// for real compiles, so the first thing it does is decide whether this
/// invocation has anything to do with Nikaia at all. Everything else - every
/// crate from crates.io - is passed to the real `rustc` untouched, which is
/// exactly what `RUSTC_WORKSPACE_WRAPPER` already guarantees and what this
/// keeps true for a workspace member that happens to be Rust.
pub fn wrapper_main() -> Result<i32> {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mut invocation = Invocation::parse(&argv, EXTENSION)
        .ok_or_else(|| anyhow!("{WRAPPER_MARKER} is set but no `rustc` was named"))?;

    let Some(source) = invocation.source_path() else {
        trace(&invocation, "passed through");
        return invocation.run();
    };
    trace(&invocation, "lowered");

    let settings = Settings::from_env()?;
    // **The packages, found the same way the project build finds them.** Cargo
    // hands this a `.nika` path, so the manifest is the one above it
    // (`Manifest::find` walks to the same root the cache does) - and it has to be
    // read here rather than passed down, because the only channel from the build
    // into the wrapper is the environment and a dependency list is not a switch.
    // `packages_of` is the one resolution rule, so the two cannot disagree about
    // what the program is.
    let manifest = Manifest::find(&source)?;
    let packages = match manifest.root() {
        Some(root) => packages_of(&manifest, root, false)?,
        None => Vec::new(),
    };
    let allowlist = std::env::var_os(READS_VAR).map(PathBuf::from);
    let shown = std::env::var_os(SHOWN_AS_VAR).and_then(|value| {
        let value = value.to_string_lossy().into_owned();
        let (entry, written) = value.split_once('\u{1f}')?;
        (canonical(&source).display().to_string() == entry).then(|| written.to_string())
    });
    let shown_outer = SHOWN_AS.with(|cell| cell.replace(shown));
    let lowered = lower_reading(
        &source,
        &settings,
        std::env::var_os(NO_CACHE_VAR).is_some(),
        &packages,
        allowlist.as_deref(),
    );
    SHOWN_AS.with(|cell| cell.replace(shown_outer));
    let lowered = lowered?;

    let name = invocation.crate_name.clone().unwrap_or_else(|| {
        source
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "main".to_string())
    });
    let name = match std::env::var_os(PROGRAM_TAG_VAR) {
        Some(tag) => name
            .strip_suffix(tag.to_string_lossy().as_ref())
            .map(str::to_string)
            .unwrap_or(name),
        None => name,
    };
    let gen_dir = match std::env::var_os(GEN_DIR_VAR) {
        Some(dir) => PathBuf::from(dir),
        None => std::env::temp_dir().join("nikaia-gen"),
    };
    // **One directory per crate**, and that is not tidiness. Every generated
    // crate root used to sit beside every other, so `rustc` refusing a name from
    // a package this crate does not depend on offered to fix it with
    // `mod deeper` - a file it had found next door, and advice that means
    // nothing here. A crate root with no siblings cannot be offered any.
    let generated = gen_dir.join(&name).join(format!("{name}.rs"));
    write_if_changed(&generated, &lowered.rust)?;

    invocation.replace_source(&generated);
    // **The program links the libraries its packages name** (ADR-324 D2, D3),
    // and only the program: a library crate is linked into it, and the
    // build-time library is built elsewhere and never sees these (ADR-321 D14).
    let a_program = invocation
        .args
        .windows(2)
        .any(|pair| pair[0] == "--crate-type" && pair[1] == "bin");
    if a_program && let Some(flags) = std::env::var_os(LINK_VAR) {
        let flags: Vec<String> = flags
            .to_string_lossy()
            .split('\u{1f}')
            .filter(|f| !f.is_empty())
            .map(str::to_string)
            .collect();
        for arg in crate::libraries::as_rustc_args(&flags) {
            invocation.args.push(OsString::from(arg));
        }
    }
    let code = invocation.run()?;

    // Cargo believes the dependency file `rustc` wrote, and `rustc` only saw
    // the generated Rust. Without this an edit to a `.nika` source leaves the
    // package fresh and the binary stale.
    //
    // **And the switches, for the same reason one step further out**
    // (`Project::switches_path`): a build switch is not a source, so nothing
    // Cargo watches changes when one does — and this wrapper is only run for a
    // package Cargo already thinks is stale. Without the file in here, flipping
    // `user-parallelism` left the program running at the setting it was built
    // at, and said nothing.
    if code == 0
        && let Some(dep_info) = invocation.dep_info()
    {
        let mut inputs = lowered.sources.clone();
        if let Some(switches) = switches_beside(&gen_dir) {
            inputs.push(switches);
        }
        record_extra_dependencies(&dep_info, &inputs)?;
    }
    Ok(code)
}

/// The switches file the driver wrote, where there is one.
///
/// The wrapper is told the generated-source directory and nothing else
/// (`GEN_DIR_VAR` is the one channel), and the file sits in it — so this is a
/// lookup rather than a second answer to where it lives. Absent means the
/// wrapper was run by hand rather than by a project build, and then there is no
/// switch anybody could flip between two runs.
fn switches_beside(gen_dir: &Path) -> Option<PathBuf> {
    project_settings::project_settings_switches_beside(gen_dir)
}

/// Record one invocation, if anybody asked: `project_settings_trace`.
fn trace(invocation: &Invocation, verdict: &str) {
    block_on(project_settings::project_settings_trace(
        TRACE_VAR,
        invocation.crate_name.clone(),
        invocation.is_a_probe(),
        verdict,
        &SettingsHost,
    ));
}

#[cfg(test)]
mod tests {
    #[test]
    fn cpu_comes_after_the_users_rustflags() {
        let e = |encoded: Option<&str>, plain: Option<&str>, cpu: &str| {
            nikaia_std::tools::project_settings::encoded_with_cpu(
                &encoded.map(String::from),
                &plain.map(String::from),
                cpu,
            )
        };
        assert_eq!(e(None, None, "x86-64-v3"), None);
        assert_eq!(
            e(None, Some(" -C  overflow-checks=off "), "x86-64-v3").unwrap(),
            "-C\x1foverflow-checks=off\x1f-C\x1ftarget-cpu=x86-64-v3"
        );
        assert_eq!(
            e(Some("-C\x1ftarget-cpu=x86-64"), Some("-g"), "x86-64-v3").unwrap(),
            "-C\x1ftarget-cpu=x86-64\x1f-C\x1ftarget-cpu=x86-64-v3"
        );
        assert_eq!(
            e(Some(""), None, "znver4").unwrap(),
            "-C\x1ftarget-cpu=znver4"
        );
    }

    use super::*;

    /// #823: only the Nikaia compiler's `error[NKxxxx]` is a refusal.
    #[test]
    fn a_rustc_failure_is_no_refusal() {
        assert!(project_tests::is_refusal("error[NK1103]: mismatched\n"));
        assert!(project_tests::is_refusal("error: expected `)`\n"));
        assert!(!project_tests::is_refusal(
            "error[E0308]: mismatched\nerror: could not compile `p`\n"
        ));
        assert!(!project_tests::is_refusal(
            "error[NK1103]: x\nerror: could not compile `p`\n"
        ));
        assert!(!project_tests::is_refusal("warning: unused\n"));
    }

    /// `--locked` names the entry a changed line belongs to.
    ///
    /// A contract line is short and repeats; since ADR-288 `sync` is the
    /// commonest line in the file. Reporting three bare `sync = true` lines
    /// tells a reader that something moved and leaves them to find out what,
    /// which is not the narrated diff 13.5 asks for.
    #[test]
    fn a_locked_diff_says_whose_contract_moved() {
        let committed = "version = 1\n\n[fn.\"a\"]\nsync = true\n\n[fn.\"b\"]\n";
        let built = "version = 1\n\n[fn.\"a\"]\nsync = true\n\n[fn.\"b\"]\nsync = \"inferred\"\n";

        assert_eq!(
            changed_lines(built, committed),
            ["  [fn.\"b\"]", "      sync = \"inferred\""]
        );
    }

    /// An entry is named once, however many of its lines changed.
    #[test]
    fn an_entry_is_named_once_for_all_of_its_changes() {
        let committed = "version = 1\n\n[fn.\"a\"]\n";
        let built = "version = 1\n\n[fn.\"a\"]\nsync = \"inferred\"\nthrows = true\n";

        assert_eq!(
            changed_lines(built, committed),
            [
                "  [fn.\"a\"]",
                "      sync = \"inferred\"",
                "      throws = true"
            ]
        );
    }

    /// A wholly new entry names itself, and is not named twice.
    #[test]
    fn a_new_entry_names_itself_once() {
        let committed = "version = 1\n";
        let built = "version = 1\n\n[fn.\"z\"]\nsync = \"inferred\"\nthrows = true\n";

        assert_eq!(
            changed_lines(built, committed),
            [
                "  [fn.\"z\"]",
                "      sync = \"inferred\"",
                "      throws = true"
            ]
        );
    }

    /// A header line belongs to no entry and is reported on its own - which is
    /// what a toolchain upgrade changing the inference looks like.
    #[test]
    fn a_changed_header_is_reported_without_an_entry() {
        let committed = "version = 1\ninference = \"stage0-signatures\"\n";
        let built = "version = 1\ninference = \"stage0-signatures+sync-bodies\"\n";

        assert_eq!(
            changed_lines(built, committed),
            ["  inference = \"stage0-signatures+sync-bodies\""]
        );
    }

    /// The runtime the emitted Rust names: `std` by path into the sysroot, the
    /// other two from the declarations `build.rs` baked in. Written down a second
    /// time, the copies would drift and a program would link two `winnow`s.
    #[test]
    fn the_runtime_travels_with_the_compiler() {
        let runtime = runtime_dependencies(
            "use winnow_grammar::grammar; use nikaia_std::prelude;",
            crate::emit::ReentrancyCheck::Yes,
        )
        .expect("the declarations this compiler was built with parse");
        assert!(runtime["nikaia-std"].get("path").is_some());
        assert!(runtime.contains_key("winnow-grammar"));
        assert!(
            runtime.contains_key("winnow"),
            "a grammar expansion writes `winnow::Parser` too"
        );
    }

    /// A `grammar!` expansion tests `feature = "trace"` in the crate that
    /// invokes it, so that crate declares it (0.0.230, issue #163 before it), forwarding
    /// to `winnow-grammar`'s. A program without a grammar declares no feature.
    #[test]
    fn a_program_with_a_grammar_declares_trace() {
        let with = runtime_dependencies(
            "use winnow_grammar::grammar;",
            crate::emit::ReentrancyCheck::Yes,
        )
        .expect("resolves");
        assert_eq!(
            runtime_features(&with)["trace"],
            ["winnow-grammar/trace".to_string()]
        );
        let without = runtime_dependencies(
            "use nikaia_std::prelude;",
            crate::emit::ReentrancyCheck::Yes,
        )
        .expect("resolves");
        assert!(runtime_features(&without).is_empty());
    }

    /// A program that names none of them declares none of them, and fetches
    /// nothing at all.
    #[test]
    fn a_program_that_names_no_runtime_depends_on_nothing() {
        let runtime = runtime_dependencies(
            "fn main() { println!(\"hi\"); }",
            crate::emit::ReentrancyCheck::Yes,
        )
        .expect("resolves");
        assert!(runtime.is_empty(), "{runtime:?}");
    }

    /// ADR-037 D1: the machine decides what a panic does, and that reaches the
    /// Cargo profile (ADR-002 D1).
    #[test]
    fn the_machine_decides_the_panic_strategy() {
        let native = Settings::resolve(&Manifest::default(), None, None).expect("resolves");
        assert_eq!(native.panic_strategy(), "unwind");

        let wasm = Settings::resolve(&Manifest::default(), Some("wasm32-unknown"), None)
            .expect("resolves");
        assert_eq!(
            wasm.panic_strategy(),
            "abort",
            "WebAssembly traps and has no stack to unwind"
        );
    }
}
