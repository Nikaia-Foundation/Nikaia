use crate::contracts::LedgerOps;

use std::collections::{BTreeMap, BTreeSet};

use std::ffi::OsString;

use std::path::{Path, PathBuf};

use nikaia_std::rt::exec::block_on;

use nikaia_std::tools::project_check;

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
    // (D12).
    let mut cache = if no_cache {
        None
    } else {
        match Cache::open(
            &layout.lock,
            &layout.store,
            env!("NIKAIA_RUSTC_VERSION"),
            env!("NIKAIA_COMPILER"),
        ) {
            Ok(cache) => Some(cache),
            Err(error) => {
                eprintln!("warning: the build cache is unavailable: {error:#}");
                None
            }
        }
    };

    // Part I 9.1: a package is a directory, so a build is every `.nika` beside
    // the entry - and the cache cannot be asked about a program until the
    // program is known.
    //
    // **Outside a project it is the one file.** A package is a directory *of a
    // project*, which is what a `nikaia.toml` declares; a directory of loose
    // examples is a directory of programs, and compiling one of them must not
    // pull in the other ten (ADR-286 D1, `modules::collect_one`).
    // **An option's computed default is compiled where this build compiles
    // its `comptime`s** (ADR-321 D1, ADR-318 D1): the ledger inferred while
    // the program is read records the value.
    let program = crate::comptime_run::defaults_compiled_in(reads.workshop().place(), || {
        match (settings.words.tests, layout.in_project) {
            (true, in_project) => modules::Program::read_for_tests(input, packages, in_project),
            (false, true) => modules::Program::read_with(input, packages),
            (false, false) => modules::Program::read_one(input),
        }
    })?;
    let mut program = program;
    program.entry_shown_as = SHOWN_AS.with(|cell| cell.borrow().clone());
    let key_source = program.sources().join("\n// --- unit ---\n");
    // **What the output names is part of the key** (0.0.239): the abort table
    // names each file as it was handed to the compiler (ADR-300 D9), so a
    // lowering cached for `/abs/one.nika` and reused for `one.nika` named the
    // first path in the second program's aborts. A choice, and not part of
    // the source, because the source's hash is the lockfile's.
    let choices = choices.naming(program.shown_paths().join("\n"));
    let sources: Vec<PathBuf> = program.units.iter().map(|unit| unit.path.clone()).collect();
    // Read off the program, which every lowering builds - a cache hit too - so
    // the note is the same whether the emitter ran or not.
    let mut notes = program.could_promise();
    // The packages this program depends on: their entries in the ledger are
    // theirs, and a contract they changed is said in their own build.
    let dependencies: BTreeSet<String> = program.as_its_own.keys().cloned().collect();
    // **What a `pub` function lost since the ledger was committed** (ADR-288
    // D5), against the project's committed file, which this lowering has not
    // written yet. Outside a project there is no committed ledger to have lost
    // anything against.
    if layout.in_project
        && let Some(committed) = std::fs::read_to_string(layout.root.join("nikaia.contracts"))
            .ok()
            .and_then(|text| Ledger::parse(&text).ok())
    {
        notes.insert_str(0, &program.lost_sync(&committed));
    }

    // **Read off the program before the lowering takes it**: what the entry
    // points are is the program's, and the wrappers are the same either way.
    let exported = match settings.library() {
        true => {
            let package = Manifest::read(&layout.root.join("nikaia.toml"))
                .ok()
                .and_then(|m| m.package_name().map(str::to_string))
                .unwrap_or_else(|| settings.words.symbol_prefix.clone());
            crate::cexport::export(
                &program,
                &settings.words.symbol_prefix,
                &package,
                settings.words.user_parallelism == "yes",
                &settings.words.target,
            )?
        }
        false => None,
    };

    // An entry that predates an artifact this build needs is a miss, not a
    // gap: adding an output stays a safe change.
    let cached = cache
        .as_ref()
        .and_then(|cache| cache.lookup(&unit, &key_source, &choices, &layout.root))
        .filter(|artifacts| artifacts.has_all(&[RUST, CONTRACTS, DERIVED, WARNINGS]));
    let reused = cached.is_some();
    let mut proofs = None;

    let (rust, ledger, derived) = match &cached {
        Some(artifacts) => {
            // **A warning is the program's, not the build's** (as Cargo
            // replays a fresh crate's): what the check said when this
            // lowering was made is said again, under the same rule of once.
            let in_the_wrapper =
                std::env::var_os(WRAPPER_MARKER).is_some() || WARNED.with(|warned| warned.get());
            if !in_the_wrapper {
                eprint!("{}", artifacts.get(WARNINGS).expect("checked above"));
            }
            (
                artifacts.get(RUST).expect("checked above").to_string(),
                artifacts.get(CONTRACTS).expect("checked above").to_string(),
                artifacts.get(DERIVED).expect("checked above").to_string(),
            )
        }
        None => {
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
            let newly = newly(&layout.root, &program.contracts);
            // Collected once for the whole package, because every unit is
            // checked against the same program (`ADR-286`) and a `comptime` in
            // any of them may call into any other.
            let beside: Vec<&crate::parser::Parsed> =
                program.units.iter().map(|unit| &unit.parsed).collect();
            let around = Around {
                foreign: &foreign,
                newly: &newly,
                beside: &beside,
                reads: &reads,
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
            checked?;
            let (lowered, book) = crate::proofs::with_book(book, || -> Result<_> {
                let lowered = {
                    // **The described crates' boundary reaches the emitter**
                    // (ADR-290 D20), which the checker above already read it for.
                    let mut program = program;
                    program.described = foreign.descriptions.clone();
                    program.checked_facts = checked_facts;
                    let lowered = program.emit_reading(settings.build, &reads)?;
                    (lowered, program)
                };
                Ok(lowered)
            });
            let (lowered, mut program) = lowered?;
            if layout.in_project {
                proofs = Some(book);
            }
            // **The contract the `assert`s make** (ADR-269 D18): the prover
            // decided it while the program was checked, and the ledger
            // publishes it.
            for (key, published) in &lowered.published {
                if let Some(contract) = program.contracts.functions.get_mut(key) {
                    // **An expression function's `ensures` stays** (ADR-314
                    // D3): its body is one `return`, so it has no `assert` of
                    // its own to publish one, only preconditions carried back.
                    let expression = match published.ensures.is_empty() {
                        true => std::mem::take(&mut contract.ensures)
                            .into_iter()
                            .zip(std::mem::take(&mut contract.from))
                            .collect(),
                        false => Vec::new(),
                    };
                    contract.requires = published.requires.clone();
                    contract.ensures = published.ensures.clone();
                    contract.from = published.from.clone();
                    contract.inferred = published.inferred.clone();
                    for (ensures, from) in expression {
                        contract.ensures.push(ensures);
                        contract.from.push(from);
                    }
                }
            }
            let ledger = program.contracts.render();
            let derived = program.contracts.render_derived();

            // **An entry nothing read** ([ADR-310](../../docs/specification/adr/adr-310.md)
            // D8): a list that may hold names nothing uses decays into
            // *everything we ever needed*, which is how an allowlist stops
            // being read. Said at the end of the build, where the whole set is
            // known, and a warning rather than a refusal — the line is a
            // permission that is no longer used, not a program that is wrong.
            for entry in reads.unused() {
                let line = format!("warning: the allowlist names `{entry}` and nothing read it\n");
                eprint!("{line}");
                warnings.push_str(&line);
            }

            if let Some(cache) = &mut cache {
                // **What the build actually read** (ADR-021 D13, ADR-310 D10).
                // An empty map is correct and usual: with no list in effect
                // nothing can be read at all.
                let artifacts = Artifacts::new()
                    .with(RUST, &lowered.rust)
                    .with(CONTRACTS, &ledger)
                    .with(DERIVED, &derived)
                    .with(WARNINGS, &warnings);
                let stored = cache
                    .record(&unit, &key_source, reads.taken(), &choices, &artifacts)
                    .and_then(|()| cache.save());
                if let Err(error) = stored {
                    // The outputs are in hand; only the next build is slower.
                    eprintln!("warning: the build cache could not be updated: {error:#}");
                }
            }
            (lowered.rust, ledger, derived)
        }
    };

    // **A contract that changed in the breaking direction** (ADR-269 D19),
    // against the committed file this lowering has not written yet: the new
    // ledger is read back, whether it came from the cache or not.
    if layout.in_project
        && let Some(committed) = std::fs::read_to_string(layout.root.join("nikaia.contracts"))
            .ok()
            .and_then(|text| Ledger::parse(&text).ok())
        && let Ok(now) = Ledger::parse(&ledger)
    {
        notes.insert_str(
            0,
            &crate::prove::changed_contracts(&now, &committed, |key| {
                !key.split_once("::")
                    .is_some_and(|(package, _)| dependencies.contains(package))
            }),
        );
    }

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
        derived,
        sources,
        reused,
        notes,
        proofs,
        header,
        python,
        node,
    })
}

/// **What this build links against, and what has been described**
/// ([ADR-290](../../docs/specification/adr/adr-290.md) D1): the struct, how a
/// build reads it and what it gives a program's packages are
/// `ProjectMembersForeign` in `tools/project_members.nika` (P4b).
pub use nikaia_std::tools::project_members::ProjectMembersForeign as Foreign;
