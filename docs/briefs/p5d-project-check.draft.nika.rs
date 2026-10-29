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

use crate::contracts::SignatureOps;

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
    let library = foreign_library(foreign)?;

    // A described crate is a package by the spelling rule, which is what
    // `modules` is: a set of words that appear in front of a `::` and are not
    // `std`'s ([`Foreign::packages`]).
    let modules = foreign.packages(modules);
    let checked = check::check_against(parsed, beside, own, &library, &modules, newly, reads);
    let mut all = checked.findings;
    if let Some(out) = facts_out {
        *out = Some(checked.facts);
    }
    // **`NK1238` is the checker's answer for a build it knows nothing of**
    // (ADR-284 D4): a library build exports its entry points.
    if building_a_library {
        all.retain(|finding| finding.code != "NK1238");
    }
    // A separate walk, for the reason the three inside `check_program` are
    // separate: it asks about the **boundary** of the build rather than about a
    // type, and it needs the manifest rather than a ledger.
    all.extend(crate::foreign::check(
        parsed,
        &foreign.declared,
        &foreign.described,
        &foreign.moved,
    ));
    all.sort_by_key(|finding| finding.span.at());
    project_check::project_check_lint_where_nothing_crosses(&mut all, user_parallelism);
    project_check::project_check_let_refuted_claims_through(&mut all, refuted_claims);
    project_check::project_check_let_unproved_claims_through(&mut all, unproved_claims);
    let violations = sync::check(parsed, own, &library);
    if all.is_empty() && violations.is_empty() {
        return Ok(String::new());
    }

    // A warning is printed and does not stop anything: a string that looks
    // like an `f"…"` (ADR-309 D15), or a `??` or `?.` after a value that
    // cannot be absent (ADR-279 D13).
    //
    // **Once** (#481): the build lowers every member before `cargo` runs, and
    // the wrapper lowers it again inside, so the wrapper keeps its warnings to
    // itself. An error is still said - the build has stopped before the
    // wrapper on any it saw.
    let in_the_wrapper =
        std::env::var_os(WRAPPER_MARKER).is_some() || WARNED.with(|warned| warned.get());
    //
    // **The warnings are handed back as they were said**, suppressed or not:
    // the build cache keeps them with the lowering, and a build that reuses
    // it says them again.
    let path = path.display().to_string();
    let mut warnings = String::new();
    let lines: Vec<&str> = source.split('\n').collect();
    for finding in &all {
        let rendered = diagnostics::render_finding_in(finding, &path, &lines);
        if finding.severity == check::Severity::Warning {
            warnings.push_str(&rendered);
            if in_the_wrapper {
                continue;
            }
        }
        eprint!("{rendered}");
    }
    if !all.iter().any(|f| f.severity == check::Severity::Error) && violations.is_empty() {
        return Ok(warnings);
    }
    for violation in &violations {
        eprint!(
            "{}",
            diagnostics::render_sync_violation(violation, &path, source)
        );
    }

    // A refusal and not a failure of this compiler, so it leaves without a
    // backtrace: the diagnostics above have already said everything, and this is
    // the tally (`diagnostics::Refused`).
    Err(diagnostics::refuse(
        nikaia_std::tools::project_check::project_check_summary(&all, violations.len() as i64),
    ))
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
    let mut newly: check::Newly = inferred
        .functions
        .iter()
        .filter_map(|(name, contract)| {
            let before = committed.functions.get(name)?;
            let gained: Vec<String> = contract
                .fails_with
                .iter()
                .filter(|error| !before.fails_with.contains(error))
                .cloned()
                .collect();
            (!gained.is_empty()).then(|| (name.clone(), gained))
        })
        .collect();
    newly.keeps = keeps_that_moved(&committed, inferred);
    newly
}

/// **The parameters whose `keeps` changed**
/// ([ADR-297](../../docs/specification/adr/adr-297.md) D12): kept now and not
/// before, or kept before and only read now. A parameter the old signature
/// does not have is **new** rather than moved, as a new function's set is not
/// a set that grew. Whether the type's teardown does anything is the checker's
/// question, which has both ledgers.
fn keeps_that_moved(committed: &Ledger, inferred: &Ledger) -> BTreeMap<String, Vec<check::Kept>> {
    inferred
        .functions
        .iter()
        .filter_map(|(name, contract)| {
            let before = committed.functions.get(name)?;
            let (now, then) = (contract.signature.as_ref()?, before.signature.as_ref()?);
            let moved: Vec<check::Kept> = now
                .arguments()
                .iter()
                .filter(|(param, _)| then.arguments().iter().any(|(p, _)| p == param))
                .filter_map(|(param, ty)| {
                    let kept = contract.keeps.contains(param);
                    (kept != before.keeps.contains(param)).then(|| check::Kept {
                        param: param.clone(),
                        now: kept,
                        ty: ty.text(),
                    })
                })
                .collect();
            (!moved.is_empty()).then(|| (name.clone(), moved))
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
        std::fs::write(path, ledger)?;
        return Ok(());
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
    let library = crate::contracts::std_ledger();
    let several = program.units.len() > 1;

    // **Once for the program and not once per file**
    // ([ADR-304](../../docs/specification/adr/adr-304.md) D10), which is the
    // difference from the four reports below: an unrolling is a fact about a
    // **call**, and the call may stand in a different file from the function it
    // names. Printing it per unit would print each one as many times as the
    // program has files.
    if want.comptime {
        let beside: Vec<&crate::parser::Parsed> =
            program.units.iter().map(|unit| &unit.parsed).collect();
        // **`Reads::none()`**, because what is reported is which types a
        // function was **used with**, and no file a build reads changes that.
        // Handing it a real allowlist would run a grammar to answer a question
        // about call sites ([ADR-177](../../docs/specification/adr/adr-177.md)).
        print!(
            "{}",
            crate::check::unrolling_report(
                &beside,
                &program.contracts,
                &crate::assets::Reads::none(),
            )
        );
    }

    for unit in &program.units {
        if several {
            println!("--- {}", unit.path.display());
        }
        if want.overlaps {
            print!(
                "{}",
                crate::contracts::order::overlap_report(
                    &unit.parsed,
                    &program.contracts,
                    library,
                    &crate::emit::branch_starts_first(
                        &unit.parsed,
                        settings.build,
                        &program.contracts,
                    ),
                )
            );
        }
        if want.sharing {
            print!(
                "{}",
                crate::contracts::sharing::report(
                    &unit.parsed,
                    &program.contracts,
                    library,
                    settings.build.user_parallelism == crate::emit::UserParallelism::Yes,
                )
            );
        }
        if want.tethers {
            print!(
                "{}",
                crate::contracts::tether::report(&unit.parsed, &program.contracts)
            );
        }
        if want.asserts {
            print!(
                "{}",
                crate::check::claims_report(
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
                    reads,
                    settings.words.unproved_claims != "warn",
                    // **Red and green only on a terminal**, and never where
                    // `NO_COLOR` is set (Part III, `--asserts`).
                    std::env::var_os("NO_COLOR").is_none()
                        && std::io::IsTerminal::is_terminal(&std::io::stdout()),
                )
            );
        }
        if want.bounds {
            print!(
                "{}",
                crate::bounds_report::report(
                    &unit.parsed,
                    &unit.source,
                    &unit.path.display().to_string(),
                    &program.contracts,
                    library,
                    settings.build.bounds,
                    settings.build.overflow,
                    settings.build.prove_through_loops,
                )
            );
        }
        if want.trust {
            print!(
                "{}",
                crate::contracts::trust::render(
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
                )
            );
        }
    }
    Ok(())
}
