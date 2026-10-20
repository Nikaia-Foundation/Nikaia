//! The sysroot: where a generated project finds `nikaia-std`, and where the
//! compiled `std` is kept.
//!
//! [ADR-002](../../../docs/specification/adr/adr-002.md) D4. `nikaia-std` is
//! not published, so a generated project cannot reach it through a registry. It
//! reaches it through a **sysroot** - a directory that travels with the
//! compiler and holds `std`'s *sources*, with the Nikaia half already lowered to
//! Rust. Nothing needs the compiler to build `std`, which is the whole of why
//! the 58 packages that used to exist only to build it are gone.
//!
//! Two things live here:
//!
//! * **Where `std` is.** [`Sysroot::resolve`] - `NIKAIA_SYSROOT`, or the
//!   checkout this compiler was built from, which is what makes a build inside
//!   the repository work with no configuration at all.
//! * **Where the compiled `std` is.** [`Sysroot::rlib_cache`] - a directory in
//!   the user's cache named by [`Key::sysroot`], so a second project on the
//!   same machine links what the first one built instead of compiling it again.
//!
//! The **ledger** is not one of them. `std.contracts` is baked into this binary
//! with `include_str!` ([`crate::contracts::STD`]) and is read from there, never
//! from the sysroot. [ADR-005](../../../docs/specification/adr/adr-005.md) D8
//! says ledger stability across toolchain versions is explicitly *not*
//! required, so a `std` that could be paired with a different compiler would let
//! the ledger describe a compiler that is not there. Keeping the copy the
//! compiler answers from inside the compiler makes that pairing impossible
//! rather than merely discouraged.

use crate::contracts::LedgerOps;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use orchestrator::cache::{self, Key, Layout};

/// The development override, named for what it is.
///
/// It replaced `NIKAIA_STD_PATH`, which named a *crate directory* and could
/// therefore only ever answer one of the questions this module answers.
pub const SYSROOT_VAR: &str = "NIKAIA_SYSROOT";

/// `std`'s directory inside a sysroot. The same name it has in the checkout, so
/// that `crates/` *is* a sysroot and the in-tree flow needs no special case.
pub const STD_DIR: &str = "nikaia-std";

/// The extension `std`'s Nikaia half carries.
const NIKA: &str = "nika";

/// Where the Nikaia that the **toolchain** uses lives, under `std`'s `src`.
///
/// A directory rather than a naming convention, because what separates the two
/// is not what the files are like but what `std` promises about them
/// ([`Sysroot::tool_modules`]).
const TOOLS: &str = "tools";

/// **Where `std`'s own unpublished Nikaia lives** (#723), under `std`'s `src`:
/// what a *program* runs - the HTTP/1 head's grammar - as opposed to what only
/// the compiler runs, in [`TOOLS`]. A package of its own, lowered the same
/// way into its own `package.rs`, because `std` compiles the tools only for
/// the compiler (the `compiler` feature) and this into every program.
const RUNTIME: &str = "runtime";

/// **The toolchain's Nikaia is one package** (ADR-294): every `.nika` in
/// `src/tools` shares one namespace, as the files of a package do (Part I 9.1,
/// ADR-286 D1), and is lowered into this one file beside them. A name declared
/// in any of them is written as it is in any other, and a helper exists once.
const TOOLS_PACKAGE: &str = "package.rs";

/// **What `lib.rs` writes by hand beside the package, by the file it serves**:
/// the names a Rust caller reaches through that file's path, which are Rust
/// because Nikaia cannot say them - a `Span`'s `usize` doors, and the one call
/// that turns a generated parser into a result. The per-file modules
/// [`lower_tools`] writes list them with the file's own names.
const HAND_WRITTEN: &[(&str, &[&str])] = &[
    ("ast.nika", &["LONGEST_SOURCE", "offset"]),
    ("http1.nika", &["written"]),
    ("rust.nika", &["file"]),
];

/// What the Rust a tool module reads says it does (ADR-294 D9.3).
const TOOLS_DESCRIBED: &str = include_str!("../../nikaia-std/src/tools/described.contracts");

/// A sysroot: a directory with `nikaia-std/` in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sysroot {
    root: PathBuf,
}

impl Sysroot {
    /// `NIKAIA_SYSROOT`, or the checkout this compiler was built from.
    ///
    /// The default is `crates/` - the directory `nikaia-std` sits in - which is
    /// what makes `cargo test --workspace` and a build inside the repository
    /// work with nothing set. An installed compiler points the variable at its
    /// own `nikaia-std`'s parent.
    pub fn resolve() -> Sysroot {
        let root = match std::env::var_os(SYSROOT_VAR) {
            Some(dir) => PathBuf::from(dir),
            None => {
                let here = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
                here.canonicalize().unwrap_or(here)
            }
        };
        Sysroot { root }
    }

    pub fn new(root: impl Into<PathBuf>) -> Sysroot {
        Sysroot { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The `nikaia-std` a generated project depends on by path.
    pub fn std_dir(&self) -> PathBuf {
        self.root.join(STD_DIR)
    }

    /// `std`'s Nikaia half, in a fixed order.
    ///
    /// Sorted, because [ADR-005](../../../docs/specification/adr/adr-005.md) D8
    /// bans consuming a directory listing in filesystem order anywhere output
    /// depends on it, and the release step below writes files from this list.
    ///
    /// **The top level only, and that is the rule that says what `std` is.**
    /// A `.nika` beside `lib.rs` is a module of `std` and
    /// `crates/nikaia/tests/contracts.rs` requires `std.contracts` to carry
    /// every `pub` thing it declares. What is in [`Self::tool_modules`] is not.
    pub fn std_modules(&self) -> Result<Vec<PathBuf>> {
        self.nika_in(&self.std_dir().join("src"), "std's Nikaia modules")
    }

    /// **Nikaia the toolchain uses and `std` does not publish**, in `src/tools`.
    ///
    /// `tools/rust.nika` is the reading half of `nikaia describe`
    /// ([ADR-290](../../../docs/specification/adr/adr-290.md) D13), lowered the
    /// same way `std`'s own Nikaia half is and reached by the compiler as an
    /// ordinary Rust module ([ADR-290](../../../docs/specification/adr/adr-290.md)
    /// D1). It lives in this crate because this is where the release step
    /// already looks, and in a directory of its own because **a `.nika` beside
    /// `lib.rs` means something**: that `std` offers it, and that
    /// `std.contracts` has to say so. This one is not offered and must not be
    /// in that file.
    /// The directory of the toolchain's Nikaia package, which `std` compiles
    /// only under its `compiler` feature (#723).
    pub fn tools_dir(&self) -> PathBuf {
        self.std_dir().join("src").join(TOOLS)
    }

    /// The directory of `std`'s run-time Nikaia package (#723): unpublished
    /// like the tools, but compiled into every program.
    pub fn runtime_dir(&self) -> PathBuf {
        self.std_dir().join("src").join(RUNTIME)
    }

    pub fn tool_modules(&self) -> Result<Vec<PathBuf>> {
        let dir = self.std_dir().join("src").join(TOOLS);
        match dir.is_dir() {
            false => Ok(Vec::new()),
            true => self.nika_in(&dir, "the toolchain's Nikaia modules"),
        }
    }

    /// Every `.nika` directly in one directory, sorted.
    fn nika_in(&self, src: &Path, what: &str) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        let entries = std::fs::read_dir(src)
            .with_context(|| format!("reading {} for {what}", src.display()))?;
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some(NIKA) {
                out.push(path);
            }
        }
        out.sort();
        Ok(out)
    }

    /// Where the compiled `std` for this build goes (ADR-002 D4).
    ///
    /// A directory in the user's cache named by [`Key::sysroot`]. It is a Cargo
    /// target directory and Cargo's own fingerprinting owns what is stale
    /// *inside* it ([ADR-021](../../../docs/specification/adr/adr-021.md) D1);
    /// the key decides only **which** directory, which is what lets two builds
    /// that differ in a way Cargo would answer by rebuilding - a different
    /// codegen table, a different toolchain - coexist instead of evicting one
    /// another (D7, §4's "dimensions coexist; they do not share").
    /// **Which tree, and the pruning that keeps the rest from piling up.** A new
    /// toolchain, target or codegen table starts a new tree, and nothing ever
    /// took an old one away - measured, 1.7 GB in a user cache and 13 GB in the
    /// one the project tests share, with a build then failing for want of disk.
    /// Coexisting is right (D7); coexisting forever is the leak. See
    /// [`cache::prune`].
    pub fn rlib_cache(
        &self,
        target: &str,
        codegen: &Codegen,
        features: &str,
        cpu: &str,
    ) -> PathBuf {
        let key = Key::sysroot(
            env!("NIKAIA_RUSTC_VERSION"),
            target,
            &codegen.render(),
            features,
            cpu,
        );
        let root = Layout::user_cache_dir().join("rlib");
        let tree = root.join(key.as_str());
        // Marked in use, and held, *before* anything prunes, so this build's
        // own tree can never be the one that goes.
        cache::use_entry(&tree);
        tree
    }
}

/// **The CPU levels `--cpu` takes for `triple`**, the baseline first
/// ([ADR-002](../../docs/specification/adr/adr-002.md) D6). x86-64 has its four
/// named levels; a target without named levels takes only its baseline, the
/// CPU `rustc` builds for when told none. Every target takes `native`.
pub fn cpu_levels(triple: &str) -> Result<Vec<String>> {
    let mut levels: Vec<String> = match triple.starts_with("x86_64") {
        true => ["x86-64", "x86-64-v2", "x86-64-v3", "x86-64-v4"]
            .map(str::to_string)
            .to_vec(),
        false => {
            let cpus = rustc_cpus(triple, &[])?;
            let baseline = cpus
                .lines()
                .find(|line| line.contains("default target CPU"))
                .and_then(|line| line.split_whitespace().next())
                .unwrap_or("generic");
            vec![baseline.to_string()]
        }
    };
    levels.push("native".to_string());
    Ok(levels)
}

/// **What `native` means on this machine**, as the compiled `std`'s key
/// holds it (ADR-002 D6): the CPU `rustc` finds and the features it turns on,
/// never the word, so a cache directory moved to another machine is not
/// taken for that machine's.
pub fn native_cpu(triple: &str) -> Result<String> {
    let said = rustc_cpus(triple, &["--print", "cfg", "-C", "target-cpu=native"])?;
    let cpu = said
        .lines()
        .find(|line| line.trim_start().starts_with("native "))
        .and_then(|line| line.split("currently ").nth(1))
        .map(|rest| rest.trim_end_matches([')', '.']))
        .unwrap_or("");
    let features: Vec<&str> = said
        .lines()
        .filter(|line| line.starts_with("target_feature="))
        .collect();
    Ok(format!("native={cpu};{}", features.join(",")))
}

/// `rustc --print target-cpus` for `triple`, with `more` arguments, started
/// where every tool is (ADR-325 D1).
fn rustc_cpus(triple: &str, more: &[&str]) -> Result<String> {
    let out = std::process::Command::new("rustc")
        .current_dir(Layout::where_tools_start())
        .args(["--print", "target-cpus", "--target", triple])
        .args(more)
        .output()
        .with_context(|| format!("asking `rustc` for the CPUs of `{triple}`"))?;
    if !out.status.success() {
        anyhow::bail!(
            "`rustc` names no CPUs for `{triple}`: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// What the machine's codegen table asked for, as a key dimension.
///
/// A rendered string rather than a struct of known keys on purpose: whatever
/// `[build.<target>]` grows - `codegen-units` being the latest - becomes
/// a dimension of the compiled `std` in the same commit that adds it, with
/// nothing here to remember to update. That is [ADR-021](../../../docs/specification/adr/adr-021.md)
/// D13's obligation kept by construction rather than by discipline.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Codegen {
    table: BTreeMap<String, String>,
}

impl Codegen {
    /// The `[build.<target>]` table of the machine this build chose, plus the
    /// panic strategy - which is not a choice (ADR-037 D1) but does change the
    /// code, so it belongs in the key beside the choices.
    pub fn new(table: &BTreeMap<String, toml::Value>, panic: &str) -> Codegen {
        let mut rendered: BTreeMap<String, String> = table
            .iter()
            .map(|(key, value)| (key.clone(), value.to_string()))
            .collect();
        rendered.insert("panic".to_string(), panic.to_string());
        Codegen { table: rendered }
    }

    /// One line per key, in key order, so the same table renders the same way
    /// however it was read.
    fn render(&self) -> String {
        self.table
            .iter()
            .map(|(key, value)| format!("{key}={value}\n"))
            .collect()
    }
}

/// Lower one of `std`'s Nikaia modules to the Rust that is committed beside it.
///
/// **One setting, and that is now a promise rather than an accident.** The
/// build switches reach this as `target = x86_64-linux` (named, not defaulted,
/// since the default is the host) and `user_parallelism = no`, whatever the
/// consuming program is built at, because there is one compiled `std` per
/// machine and the Nikaia half of it is lowered
/// once, at release time. Nobody decided that while it was a build script; it is
/// decided now (ADR-002 D4), and it is sound for exactly as long as **nothing
/// switch-sensitive appears in `std`'s `.nika` files**.
///
/// **`Shared` is what would break it**, and it has been on both sides of that
/// sentence. [ADR-037](../../../docs/specification/adr/adr-037.md) D3 made it
/// `Rc` at `user_parallelism = no` and `Arc` at `yes`; D6 took the
/// representation off the switch and it was safe here for as long as that held;
/// [ADR-312](../../../docs/specification/adr/adr-312.md) D10 put the **emission**
/// back on it - at one user thread every count is the cheap one, because nothing
/// can cross there and D1 closed the last way out of the program. So a `Shared`
/// in a `.nika` file here would be lowered once with the cheap count and handed
/// to a program built at `yes`: a `std` that cannot cross a thread inside a
/// program that may.
///
/// What is *not* switch-sensitive is the **verdict** - whether a value may cross
/// at all - which `contracts::send` answers from a type and a destination and
/// never from the switch ([ADR-312](../../../docs/specification/adr/adr-312.md)
/// D1). That is what keeps `std`'s *ledger* sound across builds; it is the
/// lowered bytes this constraint is about. The constraint is checked rather than
/// only written down:
/// `tests/sysroot.rs::stds_nikaia_half_lowers_the_same_at_both_switches` lowers
/// every module at both settings and requires the bytes to agree, so the day
/// something switch-sensitive arrives it is a red build and not a miscompile.
pub fn lower_std_module(path: &Path) -> Result<String> {
    let source =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let parsed = crate::parser::parse_to_ast(&source)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    // **Checked before it is lowered**, which it was not, and
    // [Part III C.1](../../../docs/specification/30-nikaia-tooling.md) is what
    // that cost: a module this compiler would refuse a *program* for lowered in
    // silence, and `rustc` was left to complain about the generated file. It
    // happened — `tools/rust.nika` had a grammar action calling a function that
    // could pause, which `NK2209` exists to refuse, and what a reader saw was
    // *`await` is only allowed inside `async` functions*.
    //
    // The library is `std`'s own shipped ledger, which is what a module of it
    // is compiled against anyway, and the checks that need a project — a
    // manifest's boundary, an allowlist, a package's other units — have nothing
    // to say about a file that is one unit and depends on nothing.
    let library = crate::contracts::std_library();
    let beside: Vec<&crate::parser::Parsed> = Vec::new();
    let units: Vec<&crate::parser::Parsed> = vec![&parsed];
    let own = crate::contracts::Ledger::infer_package(&units, &library);
    let mut checked = crate::check::check_against(
        &parsed,
        &beside,
        &own,
        &library,
        &std::collections::BTreeSet::new(),
        &crate::check::Newly::default(),
        &crate::assets::Reads::none(),
    );
    crate::check::refuse_assumes_in_std(&mut checked);
    let refusals: Vec<&crate::check::Finding> = checked
        .findings
        .iter()
        .filter(|finding| matches!(finding.severity, crate::check::Severity::Error))
        .collect();
    if !refusals.is_empty() {
        anyhow::bail!("{}", every_refusal(path, &source, &refusals));
    }
    let lowered =
        crate::emit::emit_std_against(&parsed, &beside, &own, &crate::contracts::Ledger::blank())
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    Ok(lowered.rust)
}

/// **Every refusal of a file, as `nikaia check` says them** (#575): in file
/// order, each with its place, its notes and its help. A file with ten
/// findings cost ten cycles when only the first was reported, and each cycle
/// can be a rebuild.
fn every_refusal(path: &Path, source: &str, refusals: &[&crate::check::Finding]) -> String {
    let mut ordered: Vec<&&crate::check::Finding> = refusals.iter().collect();
    ordered.sort_by_key(|finding| finding.span.at());
    let shown: Vec<String> = ordered
        .into_iter()
        .map(|finding| {
            crate::diagnostics::render_finding(finding, &path.display().to_string(), source)
        })
        .collect();
    format!(
        "{}\n{} refusal{} in {}",
        shown.join("\n"),
        refusals.len(),
        if refusals.len() == 1 { "" } else { "s" },
        path.display()
    )
}

/// **The toolchain's Nikaia, lowered as the one package it is** (ADR-294).
///
/// Every `.nika` in `dir` is checked with the others beside it and against
/// `std`'s ledger and the tools' described one (ADR-294 D9.3), as the files of
/// a program's package are, and lowered into one Rust file: one preamble, then
/// each file's items, then one module per file that names what that file
/// offers - `tools::ty::Ty`, `tools::ledger::read` - so a Rust caller still
/// says which file a name is in, and the name is one name underneath.
///
/// **At `user_parallelism = yes`, whatever this binary's default**
/// ([ADR-363](../../../docs/specification/adr/adr-363.md) D1): only the
/// compiler runs this code, and the compiler may be concurrent (Part I 1.2).
/// Code lowered at `yes` is correct where one thread runs it; code lowered at
/// `no` is not correct where several do.
pub fn lower_tools(dir: &Path) -> Result<String> {
    lower_tools_at(dir, tools_build())
}

/// The build the toolchain's package is lowered at (ADR-363 D1).
pub fn tools_build() -> crate::emit::Build {
    crate::emit::Build {
        target: crate::emit::Target::X86_64Linux,
        user_parallelism: crate::emit::UserParallelism::Yes,
        ..crate::emit::Build::default()
    }
}

/// The same at a given build, for a test that lowers the package at another
/// setting than [`tools_build`].
pub fn lower_tools_at(dir: &Path, build: crate::emit::Build) -> Result<String> {
    lower_tools_files(&tools_sources(dir)?, build)
}

/// The package's files: every `.nika` in `dir`, sorted.
fn tools_sources(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|e| e == NIKA))
        .collect();
    paths.sort();
    Ok(paths)
}

/// **The tools package from these files, in this order** (#581 step 9): what
/// a file lowers to depends on the package, never on where the file stands in
/// the list; only the order of the sections follows it.
pub fn lower_tools_files(paths: &[PathBuf], build: crate::emit::Build) -> Result<String> {
    lower_tools_files_on(paths, build, None)
}

/// The same on `threads` threads, or as [`in_parallel`] decides for `None`:
/// the bytes are the same at every count (#667).
pub fn lower_tools_files_on(
    paths: &[PathBuf],
    build: crate::emit::Build,
    threads: Option<usize>,
) -> Result<String> {
    let mut timer = Phases::from_env();
    let mut parsed: Vec<crate::parser::Parsed> = Vec::new();
    let mut sizes: Vec<usize> = Vec::new();
    for path in paths {
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        sizes.push(source.len());
        parsed.push(
            crate::parser::parse_to_ast(&source)
                .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?,
        );
    }
    timer.lap("parse");
    // **One package, one namespace** (ADR-286 D1), as a project's files are.
    let files: Vec<(&Path, &crate::parser::Parsed)> = paths
        .iter()
        .map(PathBuf::as_path)
        .zip(parsed.iter())
        .collect();
    crate::modules::declared_once(&files)?;
    let described =
        crate::contracts::Ledger::parse(TOOLS_DESCRIBED).context("the tools' described ledger")?;
    let mut library = crate::contracts::std_library();
    library.types.extend(described.types.clone());
    library.functions.extend(described.functions.clone());
    // **The file check's facts serve the emitter** when its library is the
    // one `check::propagation_against` builds: there `std` wins a collision
    // with a described name, here the described one does, so the two are the
    // same ledger exactly when no name is in both.
    let std = crate::contracts::std_ledger();
    let facts_reusable = !described
        .functions
        .keys()
        .any(|k| std.functions.contains_key(k))
        && !described.types.keys().any(|k| std.types.contains_key(k));
    let units: Vec<&crate::parser::Parsed> = parsed.iter().collect();
    let (own, checked) = crate::contracts::Ledger::infer_package_checked(&units, &library);
    timer.lap("ledger");
    // **The package's trust, asked once** (ADR-010 D9, #581): the join over all
    // its files is one answer, and asking it per file reads the package again
    // for each of them.
    // The ledger's pass above is the one the join would run again.
    let provenance =
        crate::contracts::trust::analyse_package_checked(&units, &library, &checked).provenance;
    drop(checked);
    timer.lap("trust");

    // **Each file checked and lowered on its own, on every core** (#667): a
    // file's lowering reads the package (`beside`, `own`, `provenance`) and
    // nothing another file's lowering writes, so the files are independent;
    // the sections still follow `paths`, and the first refusal reported is
    // the first in file order, as it was one file after the other.
    let lower_one = |at: usize| -> (Result<String>, std::time::Duration, std::time::Duration) {
        let started = std::time::Instant::now();
        let (path, unit) = (&paths[at], &parsed[at]);
        let beside: Vec<&crate::parser::Parsed> = parsed
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != at)
            .map(|(_, other)| other)
            .collect();
        // **Checked before it is lowered** (Part III C.1), with the package
        // beside it.
        let mut checked = crate::check::check_against(
            unit,
            &beside,
            &own,
            &library,
            &std::collections::BTreeSet::new(),
            &crate::check::Newly::default(),
            &crate::assets::Reads::none(),
        );
        crate::check::refuse_assumes_in_std(&mut checked);
        let refusals: Vec<&crate::check::Finding> = checked
            .findings
            .iter()
            .filter(|finding| matches!(finding.severity, crate::check::Severity::Error))
            .collect();
        if !refusals.is_empty() {
            let source = std::fs::read_to_string(path).unwrap_or_default();
            let refused = Err(anyhow::anyhow!(
                "{}",
                every_refusal(path, &source, &refusals)
            ));
            return (refused, started.elapsed(), std::time::Duration::ZERO);
        }
        let checking = started.elapsed();
        let facts = facts_reusable.then_some(checked.facts);
        let lowered = crate::emit::emit_std_items_against_at(
            unit, &beside, &own, &described, build, provenance, facts,
        )
        .map(|lowered| lowered.rust)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()));
        (lowered, checking, started.elapsed() - checking)
    };
    // The largest first, so no core is left with a long file at the end.
    let mut order: Vec<usize> = (0..paths.len()).collect();
    order.sort_by_key(|at| std::cmp::Reverse(sizes[*at]));
    let results = in_parallel(&order, threads, lower_one);
    timer.lap("files");

    let mut imports: Vec<String> = Vec::new();
    let mut bodies = String::new();
    let mut facades = String::new();
    let mut grammar = false;
    for ((path, unit), (lowered, checking, emitting)) in paths.iter().zip(&parsed).zip(results) {
        timer.file(path, checking, emitting);
        let lowered = lowered?;
        // **One preamble for the package**: a `use std::…` is the file's
        // import of a `std` module, and one namespace imports it once.
        let mut body = String::new();
        let mut lines = lowered.lines().peekable();
        while let Some(line) = lines.next() {
            if line == "#[allow(unused_imports)]"
                && let Some(next) = lines.peek()
                && next.starts_with("use nikaia_std::")
            {
                let import = next.to_string();
                lines.next();
                if !imports.contains(&import) {
                    imports.push(import);
                }
                continue;
            }
            body.push_str(line);
            body.push('\n');
        }
        grammar |= unit
            .program
            .items
            .iter()
            .any(|item| matches!(item.node, crate::ast::Item::Grammar(_)));
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        bodies.push_str(&format!("// --- {name} ---\n\n"));
        bodies.push_str(body.trim_start_matches('\n'));
        bodies.push('\n');
        facades.push_str(&facade(name, unit));
    }

    let mut rust = String::new();
    rust.push_str("// Generated by the Nikaia bootstrap compiler (Stage 0).\n");
    rust.push_str("// Edit the .nika sources in this directory, not this file.\n\n");
    if grammar {
        rust.push_str("use winnow_grammar::grammar;\n");
    }
    rust.push_str("#[allow(unused_imports)]\npub use nikaia_std::error::Full;\n");
    imports.sort();
    for import in imports {
        rust.push_str("#[allow(unused_imports)]\n");
        rust.push_str(&import);
        rust.push('\n');
    }
    rust.push('\n');
    rust.push_str(&bodies);
    rust.push_str(&facades);
    timer.lap("assemble");
    timer.report();
    Ok(rust)
}

/// **Where `lower-std`'s time goes** (#667): with `NIKAIA_LOWER_TIMES` set,
/// each phase's total and the slowest files of the per-file phases go to
/// stderr. Unset, it reads no clock.
struct Phases {
    on: bool,
    last: Option<std::time::Instant>,
    totals: Vec<(String, std::time::Duration)>,
    files: Vec<(String, String, std::time::Duration)>,
}

impl Phases {
    fn from_env() -> Phases {
        let on = std::env::var_os("NIKAIA_LOWER_TIMES").is_some();
        Phases {
            on,
            last: on.then(std::time::Instant::now),
            totals: Vec::new(),
            files: Vec::new(),
        }
    }

    fn take(&mut self) -> Option<std::time::Duration> {
        let now = std::time::Instant::now();
        let spent = now - self.last?;
        self.last = Some(now);
        Some(spent)
    }

    fn add(&mut self, phase: &str, spent: std::time::Duration) {
        match self.totals.iter_mut().find(|(name, _)| name == phase) {
            Some((_, total)) => *total += spent,
            None => self.totals.push((phase.to_string(), spent)),
        }
    }

    fn lap(&mut self, phase: &str) {
        if let Some(spent) = self.take() {
            self.add(phase, spent);
        }
    }

    fn file(&mut self, path: &Path, checking: std::time::Duration, emitting: std::time::Duration) {
        if !self.on {
            return;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        for (phase, spent) in [("check", checking), ("emit", emitting)] {
            self.add(phase, spent);
            self.files
                .push((phase.to_string(), name.to_string(), spent));
        }
    }

    fn report(&mut self) {
        if !self.on {
            return;
        }
        for (phase, total) in &self.totals {
            eprintln!("lower-std: {phase:<10} {:>9.3}s", total.as_secs_f64());
        }
        if self.totals.iter().any(|(phase, _)| phase == "check") {
            eprintln!(
                "lower-std: (check and emit are summed over the threads; files is the wall clock)"
            );
        }
        self.files.sort_by_key(|file| std::cmp::Reverse(file.2));
        for (phase, file, spent) in self.files.iter().take(25) {
            eprintln!(
                "lower-std:   {phase:<6} {file:<32} {:>8.3}s",
                spent.as_secs_f64()
            );
        }
    }
}

/// **`work` for each of `order`, on every core, the answers in index order**
/// (#667): `order` says which index is started first and nothing else. The
/// threads are `threads`, else `NIKAIA_LOWER_THREADS` where it is set, else
/// one per core; at one, the work runs on the calling thread.
fn in_parallel<T: Send>(
    order: &[usize],
    threads: Option<usize>,
    work: impl Fn(usize) -> T + Sync,
) -> Vec<T> {
    // The checker and the emitter recurse as deep as the code they walk.
    const STACK: usize = 256 << 20;
    let threads = threads
        .or_else(|| {
            std::env::var("NIKAIA_LOWER_THREADS")
                .ok()
                .and_then(|n| n.parse::<usize>().ok())
        })
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
        .clamp(1, order.len().max(1));
    let mut done: Vec<(usize, T)> = match threads {
        1 => order.iter().map(|at| (*at, work(*at))).collect(),
        _ => {
            let next = std::sync::atomic::AtomicUsize::new(0);
            let take = || {
                let mut mine = Vec::new();
                loop {
                    let n = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(at) = order.get(n) else {
                        return mine;
                    };
                    mine.push((*at, work(*at)));
                }
            };
            std::thread::scope(|scope| {
                let workers: Vec<_> = (0..threads)
                    .map(|_| {
                        std::thread::Builder::new()
                            .stack_size(STACK)
                            .spawn_scoped(scope, take)
                            .expect("a thread to lower on")
                    })
                    .collect();
                workers
                    .into_iter()
                    .flat_map(|worker| {
                        worker
                            .join()
                            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
                    })
                    .collect()
            })
        }
    };
    done.sort_by_key(|(at, _)| *at);
    done.into_iter().map(|(_, answer)| answer).collect()
}

/// **One file's names, as a path into the package**: `pub mod ty { pub use
/// super::{Ty, …}; }`. What the file declares `pub`, its grammars, and what
/// `lib.rs` writes for it by hand ([`HAND_WRITTEN`]).
fn facade(file: &str, unit: &crate::parser::Parsed) -> String {
    use crate::ast::Item;
    let mut names: Vec<String> = Vec::new();
    for item in &unit.program.items {
        let name = match &item.node {
            Item::Fn {
                name: Some(name),
                is_public: true,
                receiver: None,
                ..
            }
            | Item::Enum {
                name,
                is_public: true,
                ..
            }
            | Item::Struct {
                name,
                is_public: true,
                ..
            }
            | Item::Trait {
                name,
                is_public: true,
                ..
            }
            | Item::Comptime {
                name, public: true, ..
            } => unit.text(*name),
            Item::Grammar(grammar) => unit.text(grammar.name),
            _ => continue,
        };
        names.push(crate::emit::escaped(name).to_string());
    }
    if let Some((_, extra)) = HAND_WRITTEN.iter().find(|(name, _)| *name == file) {
        names.extend(extra.iter().map(|name| name.to_string()));
    }
    let module = file.trim_end_matches(".nika");
    match names.is_empty() {
        true => format!("pub mod {module} {{}}\n"),
        false => format!(
            "pub mod {module} {{\n    #[allow(unused_imports)]\n    pub use super::{{{}}};\n}}\n",
            names.join(", ")
        ),
    }
}

/// Where `lower_std_module`'s result is committed: beside the source, same stem.
pub fn lowered_path(nika: &Path) -> PathBuf {
    nika.with_extension("rs")
}

/// Where [`lower_tools`]'s result is committed: in the directory, beside the
/// sources it is lowered from.
pub fn tools_lowered_path(dir: &Path) -> PathBuf {
    dir.join(TOOLS_PACKAGE)
}

/// Re-lower every Nikaia module in the sysroot's `std`, writing the `.rs` beside
/// the `.nika`. Returns the files that changed.
///
/// This is the release step, and the *only* thing that runs it is the `nikaia`
/// **binary** (`nikaia lower-std`). A from-source install may use it; a binary
/// install never has to, because the `.rs` is committed. What must not happen
/// again is `nikaia-std` linking the compiler as a library to do this, which is
/// what built the compiler a second time inside every project's `target/`.
pub fn lower_std(sysroot: &Sysroot) -> Result<Vec<PathBuf>> {
    let mut changed = Vec::new();
    let mut lowered: Vec<(PathBuf, String)> = Vec::new();
    let cache = LoweredBefore::open();
    for nika in sysroot.std_modules()? {
        let key = cache.key("module", std::slice::from_ref(&nika));
        let rust = match cache.get(key.as_deref()) {
            Some(rust) => rust,
            None => {
                let rust = lower_std_module(&nika)?;
                cache.put(key.as_deref(), &rust);
                rust
            }
        };
        lowered.push((lowered_path(&nika), rust));
    }
    // **Two packages, each lowered alone** (#723): the compiler's and the
    // one every program runs; neither sees the other's names.
    for package in [sysroot.tools_dir(), sysroot.runtime_dir()] {
        if !package.is_dir() {
            continue;
        }
        // **Keyed on every file of the package**: each file is checked and
        // lowered against all the others (`beside`, the package's ledger, its
        // trust), so a change in any of them may change any section.
        let paths = tools_sources(&package)?;
        let key = cache.key("tools", &paths);
        let rust = match cache.get(key.as_deref()) {
            Some(rust) => rust,
            None => {
                let rust = lower_tools_files(&paths, tools_build())?;
                cache.put(key.as_deref(), &rust);
                rust
            }
        };
        lowered.push((tools_lowered_path(&package), rust));
    }
    for (rs, rust) in lowered {
        let current = std::fs::read_to_string(&rs).ok();
        if current.as_deref() != Some(rust.as_str()) {
            std::fs::write(&rs, &rust).with_context(|| format!("writing {}", rs.display()))?;
            changed.push(rs);
        }
    }
    Ok(changed)
}

/// **What `lower-std` wrote before, by everything it read to write it**
/// (#667): the same compiler lowering the same sources writes the same bytes,
/// so a second run with nothing changed takes the answer from the user cache
/// instead of five minutes.
///
/// The key is the compiler's own bytes - its lowering, `std`'s ledger and the
/// tools' described one are all compiled into it - and the name and text of
/// every file read, in order. Each entry is a directory of the cache's
/// `lower-std/`, so `cache::prune` keeps that from piling up.
/// Failure here is never a failed lowering: no key or an unreadable entry
/// lowers again, and an entry that cannot be written is not written.
struct LoweredBefore {
    root: PathBuf,
    compiler: Option<String>,
}

/// The file in an entry's directory that holds what was lowered.
const LOWERED: &str = "lowered.rs";

impl LoweredBefore {
    fn open() -> LoweredBefore {
        let compiler = std::env::current_exe()
            .and_then(std::fs::read)
            .ok()
            .map(|bytes| cache::sha256_hex(&bytes));
        LoweredBefore {
            root: Layout::user_cache_dir().join("lower-std"),
            compiler,
        }
    }

    /// The key for lowering `files` as `what`, or `None` where one of them
    /// cannot be read (the lowering then says why) or the compiler could not.
    fn key(&self, what: &str, files: &[PathBuf]) -> Option<String> {
        let mut read: Vec<u8> = Vec::new();
        for part in ["nikaia-lower-std-v1", self.compiler.as_deref()?, what] {
            read.extend_from_slice(part.as_bytes());
            read.push(0);
        }
        for file in files {
            let source = std::fs::read(file).ok()?;
            let name = file.file_name()?.as_encoded_bytes();
            read.extend_from_slice(&(name.len() as u64).to_le_bytes());
            read.extend_from_slice(name);
            read.extend_from_slice(&(source.len() as u64).to_le_bytes());
            read.extend_from_slice(&source);
        }
        Some(cache::sha256_hex(&read))
    }

    fn get(&self, key: Option<&str>) -> Option<String> {
        let entry = self.root.join(key?);
        let rust = std::fs::read_to_string(entry.join(LOWERED)).ok()?;
        cache::use_entry(&entry);
        Some(rust)
    }

    fn put(&self, key: Option<&str>, rust: &str) {
        let Some(key) = key else {
            return;
        };
        let entry = self.root.join(key);
        if std::fs::create_dir_all(&entry).is_err() {
            return;
        }
        // Whole or not at all: a reader never sees half an entry.
        let partial = crate::comptime_run::unique_beside(&entry.join(LOWERED));
        if std::fs::write(&partial, rust).is_ok()
            && std::fs::rename(&partial, entry.join(LOWERED)).is_ok()
        {
            cache::use_entry(&entry);
        } else {
            let _ = std::fs::remove_file(&partial);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default sysroot is the checkout, and `std` is in it. This is the
    /// in-tree flow's whole configuration.
    #[test]
    fn the_checkout_is_a_sysroot() {
        let sysroot = Sysroot::resolve();
        assert!(
            sysroot.std_dir().join("Cargo.toml").is_file(),
            "{} holds std",
            sysroot.root().display()
        );
    }

    /// D7. Two builds that differ only in their codegen table must not share a
    /// compiled `std`, or switching `opt-level` would evict the other one's
    /// artifacts instead of keeping its own beside them.
    #[test]
    fn the_codegen_table_changes_where_the_compiled_std_goes() {
        let sysroot = Sysroot::new("/nowhere");
        let plain = Codegen::new(&BTreeMap::new(), "unwind");
        let tuned = Codegen::new(
            &BTreeMap::from([("opt-level".to_string(), toml::Value::Integer(3))]),
            "unwind",
        );
        assert_ne!(
            sysroot.rlib_cache("x86_64-linux", &plain, "", ""),
            sysroot.rlib_cache("x86_64-linux", &tuned, "", ""),
        );
    }

    /// ADR-002 D6: the CPU level is a dimension, so two levels keep two
    /// compiled `std`s, and `native` is keyed by the CPU found, which is
    /// never the word or another level's name.
    #[test]
    fn the_cpu_level_changes_where_the_compiled_std_goes() {
        let sysroot = Sysroot::new("/nowhere");
        let codegen = Codegen::new(&BTreeMap::new(), "unwind");
        let at = |cpu: &str| sysroot.rlib_cache("x86_64-linux", &codegen, "", cpu);
        assert_ne!(at(""), at("x86-64-v3"));
        assert_ne!(at("x86-64-v2"), at("x86-64-v3"));
        if cfg!(target_arch = "x86_64") {
            let native = native_cpu("x86_64-unknown-linux-gnu").expect("rustc answers");
            assert!(
                native.starts_with("native=") && native.len() > "native=;".len(),
                "{native}"
            );
            assert_ne!(at(&native), at("x86-64-v3"));
            assert_ne!(at(&native), at("native"));
        }
    }

    /// ADR-002 D6: x86-64's four levels and `native`, the baseline first.
    #[test]
    fn x86_64_has_four_levels_and_native() {
        assert_eq!(
            cpu_levels("x86_64-unknown-linux-gnu").expect("named, no rustc asked"),
            ["x86-64", "x86-64-v2", "x86-64-v3", "x86-64-v4", "native"]
        );
    }

    /// The machine is a dimension; it decides what `std` can offer at all
    /// (ADR-037 D1).
    #[test]
    fn the_machine_changes_where_the_compiled_std_goes() {
        let sysroot = Sysroot::new("/nowhere");
        let codegen = Codegen::new(&BTreeMap::new(), "unwind");
        assert_ne!(
            sysroot.rlib_cache("x86_64-linux", &codegen, "", ""),
            sysroot.rlib_cache("wasm32-unknown", &codegen, "", ""),
        );
    }

    /// The panic strategy is not a choice (ADR-037 D1), and it still changes the
    /// code - so it is in the key beside the choices.
    #[test]
    fn the_panic_strategy_is_part_of_the_codegen_dimension() {
        let table = BTreeMap::new();
        assert_ne!(
            Codegen::new(&table, "unwind").render(),
            Codegen::new(&table, "abort").render(),
        );
    }
}
