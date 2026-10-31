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

pub use project_settings::{prove_through_loops, skip_free_at_exit};

/// **What this build links against, and what has been described**
/// ([ADR-290](../../docs/specification/adr/adr-290.md) D1): the struct, how a
/// build reads it and what it gives a program's packages are
/// `ProjectMembersForeign` in `tools/project_members.nika` (P4b).
pub use nikaia_std::tools::project_members::ProjectMembersForeign as Foreign;

impl Project {
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
}
