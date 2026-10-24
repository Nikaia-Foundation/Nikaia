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
const TOOLS_PACKAGE: &str = "lowered.rs";

/// The directory beside [`TOOLS_PACKAGE`] that holds one generated file per
/// `.nika` (#667 step 4): each is a Rust module of its own, so a change to
/// one `.nika` recompiles one module.
const TOOLS_FILES: &str = "lowered";

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
    // Asked once a process: the key is read for each lowering and each cache.
    static ASKED: std::sync::Mutex<BTreeMap<String, String>> =
        std::sync::Mutex::new(BTreeMap::new());
    if let Some(known) = ASKED.lock().ok().and_then(|held| held.get(triple).cloned()) {
        return Ok(known);
    }
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
    let key = format!("native={cpu};{}", features.join(","));
    if let Ok(mut held) = ASKED.lock() {
        held.insert(triple.to_string(), key.clone());
    }
    Ok(key)
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
pub fn lower_tools(dir: &Path) -> Result<Package> {
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
pub fn lower_tools_at(dir: &Path, build: crate::emit::Build) -> Result<Package> {
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
pub fn lower_tools_files(paths: &[PathBuf], build: crate::emit::Build) -> Result<Package> {
    lower_tools_files_on(paths, build, None)
}

/// The same on `threads` threads, or as [`in_parallel`] decides for `None`:
/// the bytes are the same at every count (#667).
pub fn lower_tools_files_on(
    paths: &[PathBuf],
    build: crate::emit::Build,
    threads: Option<usize>,
) -> Result<Package> {
    lower_tools_files_cached(paths, build, threads, &LoweredBefore::none(), &mut 0)
}

/// **The source with every body taken out** (#667): what another file of the
/// package can see of this one - its names, signatures, types and constants.
/// The ranges are the statements' extent, so a comment between two statements
/// goes with them and a comment outside a body stays.
fn without_bodies(source: &str, parsed: &crate::parser::Parsed) -> String {
    fn bodies(item: &crate::ast::Spanned<crate::ast::Item>, cut: &mut Vec<(usize, usize)>) {
        let block = match &item.node {
            crate::ast::Item::Fn { body, .. } => body,
            crate::ast::Item::Test { body, .. } | crate::ast::Item::Bench { body, .. } => body,
            crate::ast::Item::Impl { methods, .. } => {
                methods.iter().for_each(|m| bodies(m, cut));
                return;
            }
            _ => return,
        };
        if let (Some(first), Some(last)) = (block.stmts.first(), block.stmts.last()) {
            cut.push((first.span.start as usize, last.span.end as usize));
        }
    }
    let mut cut: Vec<(usize, usize)> = Vec::new();
    parsed
        .program
        .items
        .iter()
        .for_each(|i| bodies(i, &mut cut));
    cut.sort();
    let mut kept = String::new();
    let mut at = 0;
    for (start, end) in cut {
        if start < at
            || end > source.len()
            || !source.is_char_boundary(start)
            || !source.is_char_boundary(end)
        {
            // Not a range this can trust: the whole text is the interface.
            return source.to_string();
        }
        kept.push_str(&source[at..start]);
        at = end;
    }
    kept.push_str(&source[at..]);
    // A line that is only a comment says nothing to another file (a `///`
    // line is a doc and stays).
    kept.lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("//") || line.starts_with("///")
        })
        .flat_map(|line| [line, "\n"])
        .collect()
}

/// The same with the per-file cache of [`LoweredBefore`]: `reused` counts the
/// files taken from it. A file's key is its own text and the package's
/// interface - the others' signatures, the ledger and the trust it is checked
/// against - and not the others' bodies, so a body change lowers one file and
/// a signature change (or a change the ledger shows) lowers all.
fn lower_tools_files_cached(
    paths: &[PathBuf],
    build: crate::emit::Build,
    threads: Option<usize>,
    cache: &LoweredBefore,
    reused: &mut usize,
) -> Result<Package> {
    let mut timer = Phases::from_env();
    let mut parsed: Vec<crate::parser::Parsed> = Vec::new();
    let mut sizes: Vec<usize> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    for path in paths {
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        sizes.push(source.len());
        let one = crate::parser::parse_to_ast(&source)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        texts.push(source);
        parsed.push(one);
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
    // **The package's interface**, the same for every file of it.
    let interface: Option<String> = cache.compiler.as_ref().map(|compiler| {
        let mut read: Vec<u8> = Vec::new();
        let mut part = |bytes: &[u8]| {
            read.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            read.extend_from_slice(bytes);
        };
        part(b"nikaia-lower-file-v1");
        part(compiler.as_bytes());
        part(
            std::env::var("NIKAIA_LOWER_CACHE_SALT")
                .unwrap_or_default()
                .as_bytes(),
        );
        part(format!("{build:?}{provenance:?}{facts_reusable}").as_bytes());
        part(format!("{own:?}").as_bytes());
        for (path, (text, one)) in paths.iter().zip(texts.iter().zip(&parsed)) {
            part(path.file_name().unwrap_or_default().as_encoded_bytes());
            part(without_bodies(text, one).as_bytes());
        }
        cache::sha256_hex(&read)
    });
    let file_key = |at: usize| -> Option<String> {
        let mut read = interface.clone()?.into_bytes();
        let name = paths[at].file_name()?.as_encoded_bytes();
        read.extend_from_slice(&(name.len() as u64).to_le_bytes());
        read.extend_from_slice(name);
        read.extend_from_slice(texts[at].as_bytes());
        Some(cache::sha256_hex(&read))
    };

    // **Each file checked and lowered on its own, on every core** (#667): a
    // file's lowering reads the package (`beside`, `own`, `provenance`) and
    // nothing another file's lowering writes, so the files are independent;
    // the sections still follow `paths`, and the first refusal reported is
    // the first in file order, as it was one file after the other.
    let lower_one = |at: usize| -> (
        Result<(String, bool)>,
        std::time::Duration,
        std::time::Duration,
    ) {
        let started = std::time::Instant::now();
        let (path, unit) = (&paths[at], &parsed[at]);
        let key = file_key(at);
        if let Some(rust) = cache.get(key.as_deref()) {
            return (
                Ok((rust, true)),
                started.elapsed(),
                std::time::Duration::ZERO,
            );
        }
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
        .map(|lowered| {
            cache.put(key.as_deref(), &lowered.rust);
            (lowered.rust, false)
        })
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()));
        (lowered, checking, started.elapsed() - checking)
    };
    // The largest first, so no core is left with a long file at the end.
    let mut order: Vec<usize> = (0..paths.len()).collect();
    order.sort_by_key(|at| std::cmp::Reverse(sizes[*at]));
    let results = in_parallel(&order, threads, lower_one);
    timer.lap("files");

    let mut imports: Vec<String> = Vec::new();
    let mut modules: Vec<(String, String)> = Vec::new();
    let mut facades: Vec<(String, String)> = Vec::new();
    let mut files: Vec<(String, String)> = Vec::new();
    let mut grammar = false;
    for ((path, unit), (lowered, checking, emitting)) in paths.iter().zip(&parsed).zip(results) {
        timer.file(path, checking, emitting);
        let (lowered, was_cached) = lowered?;
        *reused += usize::from(was_cached);
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
        let stem = name.trim_end_matches(".nika");
        // **One module per file** (#667 step 4): the file's items see the
        // package's names through `use super::*`, and the package root
        // re-exports them; an item another file may call is `pub(crate)`.
        modules.push((stem.to_string(), format!(
            "mod {stem}_lowered {{\n    #[allow(unused_imports)]\n    use super::*;\n    include!(\"{TOOLS_FILES}/{stem}.rs\");\n}}\n#[allow(unused_imports)]\npub use {stem}_lowered::*;\n\n"
        )));
        let text = format!(
            "// Generated by the Nikaia bootstrap compiler (Stage 0).\n// Edit {name} in the directory above, not this file.\n\n{}",
            shared(body.trim_start_matches('\n'))
        );
        files.push((format!("{TOOLS_FILES}/{stem}.rs"), text));
        facades.push((stem.to_string(), facade(name, unit)));
    }
    // **Sorted by file**, whatever order the files were read in.
    modules.sort();
    facades.sort();
    files.sort();

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
    modules.iter().for_each(|(_, one)| rust.push_str(one));
    facades.iter().for_each(|(_, one)| rust.push_str(one));
    files.insert(0, (TOOLS_PACKAGE.to_string(), rust));
    timer.lap("assemble");
    timer.report();
    Ok(Package { files })
}

/// **A package as `lower-std` writes it** (#667 step 4): the index
/// (`lowered.rs`: the preamble, one `mod` per file, the façades) and one
/// generated file per `.nika` under `lowered/`, each as a path below the
/// package's directory with its text. Sorted by file, so the layout is the
/// same on every run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    files: Vec<(String, String)>,
}

/// The line that starts a file in a package's cache entry.
const PACKED: &str = "//@@ file ";

impl Package {
    /// The generated files: path below the package's directory, and text.
    pub fn files(&self) -> &[(String, String)] {
        &self.files
    }

    /// What is committed in `dir`: the index and every file under `lowered/`.
    pub fn committed(dir: &Path) -> Package {
        let mut names = vec![TOOLS_PACKAGE.to_string()];
        names.extend(Self::stale(dir, &[]));
        let files = names
            .into_iter()
            .map(|name| {
                let text = std::fs::read_to_string(dir.join(&name)).unwrap_or_default();
                (name, text)
            })
            .collect();
        Package { files }
    }

    /// The package as one text, each `include!` of the index replaced by the
    /// file it names: what a test compiles when it has no directory.
    pub fn inlined(&self) -> String {
        let mut text = self
            .files
            .first()
            .map(|(_, t)| t.clone())
            .unwrap_or_default();
        for (name, body) in self.files.iter().skip(1) {
            text = text.replace(&format!("include!(\"{name}\");"), body);
        }
        text
    }

    /// Whether any file holds `needle`.
    pub fn contains(&self, needle: &str) -> bool {
        self.files.iter().any(|(_, text)| text.contains(needle))
    }

    /// The paths below `dir` that differ from what is committed there, or
    /// are committed and no longer generated.
    pub fn drift(&self, dir: &Path) -> Vec<String> {
        let mut drifted: Vec<String> = self
            .files
            .iter()
            .filter(|(name, text)| {
                std::fs::read_to_string(dir.join(name)).ok().as_deref() != Some(text.as_str())
            })
            .map(|(name, _)| name.clone())
            .collect();
        let kept: Vec<&str> = self.files.iter().map(|(name, _)| name.as_str()).collect();
        for stale in Self::stale(dir, &kept) {
            drifted.push(stale);
        }
        drifted
    }

    /// The generated files under `dir/lowered/` that `kept` does not name.
    fn stale(dir: &Path, kept: &[&str]) -> Vec<String> {
        let mut stale: Vec<String> = std::fs::read_dir(dir.join(TOOLS_FILES))
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .map(|name| format!("{TOOLS_FILES}/{name}"))
            .filter(|name| !kept.contains(&name.as_str()))
            .collect();
        stale.sort();
        stale
    }

    fn pack(&self) -> String {
        self.files
            .iter()
            .map(|(name, text)| format!("{PACKED}{name}\n{text}"))
            .collect()
    }

    fn unpack(packed: &str) -> Option<Package> {
        let mut files: Vec<(String, String)> = Vec::new();
        for line in packed.split_inclusive('\n') {
            match line.strip_prefix(PACKED) {
                Some(name) => files.push((name.trim_end().to_string(), String::new())),
                None => files.last_mut()?.1.push_str(line),
            }
        }
        (!files.is_empty()).then_some(Package { files })
    }
}

impl std::fmt::Display for Package {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.files
            .iter()
            .try_for_each(|(name, text)| write!(out, "// --- {name} ---\n{text}"))
    }
}

/// **Makes a file's items visible to the others of its package**: a lowered
/// file is a module of its own, and an item written without `pub` is private
/// to it. Every top-level `fn`, `const`, `static`, `struct`, `enum`, `type`
/// and `trait` that stands at the start of a line outside any string, comment
/// or brace becomes `pub(crate)`.
fn shared(text: &str) -> String {
    const KEYWORDS: [&str; 9] = [
        "fn ",
        "const ",
        "static ",
        "struct ",
        "enum ",
        "type ",
        "trait ",
        "unsafe fn ",
        "async fn ",
    ];
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + 4096);
    let (mut at, mut depth, mut start) = (0usize, 0i64, true);
    let mut inherent = false;
    let mut in_struct = false;
    let mut copied = 0usize;
    while at < bytes.len() {
        if start && depth == 0 {
            // An `impl` without `for` is inherent: its methods take a
            // visibility, a trait impl's do not.
            in_struct = ["struct ", "pub struct "]
                .iter()
                .any(|k| text[at..].starts_with(k));
            inherent = text[at..].starts_with("impl")
                && !text[at..]
                    .split('{')
                    .next()
                    .unwrap_or_default()
                    .contains(" for ");
            if KEYWORDS.iter().any(|k| text[at..].starts_with(k)) {
                out.push_str(&text[copied..at]);
                out.push_str("pub(crate) ");
                copied = at;
            }
        } else if start && depth == 1 && in_struct {
            // A field written without `pub` is private to its module.
            let field = at + 4;
            let rest = text.get(field..).unwrap_or_default();
            let name = rest
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            if text[at..].starts_with("    ")
                && name > 0
                && !rest.starts_with("pub")
                && rest[name..].starts_with(": ")
            {
                out.push_str(&text[copied..field]);
                out.push_str("pub(crate) ");
                copied = field;
            }
        } else if start && depth == 1 && inherent {
            let method = at + 4;
            if text[at..].starts_with("    ")
                && text.get(method..).is_some_and(|rest| {
                    KEYWORDS[..2]
                        .iter()
                        .chain(&KEYWORDS[7..])
                        .any(|k| rest.starts_with(k))
                })
            {
                out.push_str(&text[copied..method]);
                out.push_str("pub(crate) ");
                copied = method;
            }
        }
        start = false;
        match bytes[at] {
            b'\n' => start = true,
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                // A line comment: to the end of the line.
                while at + 1 < bytes.len() && bytes[at + 1] != b'\n' {
                    at += 1;
                }
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                let mut nest = 0;
                while at < bytes.len() {
                    if text[at..].starts_with("/*") {
                        nest += 1;
                        at += 1;
                    } else if text[at..].starts_with("*/") {
                        nest -= 1;
                        at += 1;
                        if nest == 0 {
                            break;
                        }
                    }
                    at += 1;
                }
                at = at.min(bytes.len().saturating_sub(1));
            }
            b'r' | b'b'
                if (at == 0
                    || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_'))
                    && raw_string(text, at).is_some() =>
            {
                at = raw_string(text, at).unwrap_or(at);
            }
            b'"' => {
                at += 1;
                while at < bytes.len() && bytes[at] != b'"' {
                    at += usize::from(bytes[at] == b'\\');
                    at += 1;
                }
            }
            b'\'' => {
                // A character literal, not a lifetime: `'x'` or `'\..'`.
                if bytes.get(at + 1) == Some(&b'\\') {
                    at += 2;
                    while at < bytes.len() && bytes[at] != b'\'' {
                        at += 1;
                    }
                } else if let Some(c) = text[at + 1..].chars().next()
                    && bytes.get(at + 1 + c.len_utf8()) == Some(&b'\'')
                {
                    at += 1 + c.len_utf8();
                }
            }
            _ => {}
        }
        at += 1;
    }
    out.push_str(&text[copied.min(text.len())..]);
    out
}

/// Where a raw string (`r"…"`, `r#"…"#`, `br"…"`) that starts at `at` ends:
/// the index of its last byte.
fn raw_string(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = at;
    if bytes[i] == b'b' {
        i += 1;
    }
    if bytes.get(i) != Some(&b'r') {
        return None;
    }
    i += 1;
    let hashes = bytes[i..].iter().take_while(|b| **b == b'#').count();
    i += hashes;
    if bytes.get(i) != Some(&b'"') {
        return None;
    }
    let close = format!("\"{}", "#".repeat(hashes));
    let end = text[i + 1..].find(&close)?;
    Some(i + 1 + end + close.len() - 1)
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
        // **Keyed per file** (#667): see `lower_tools_files_cached`.
        let paths = tools_sources(&package)?;
        // **Nothing changed**: the whole package is one entry, so a run that
        // changes nothing reads no file twice and checks nothing.
        let whole = cache.key("tools-modules", &paths);
        let rust = match cache
            .get(whole.as_deref())
            .as_deref()
            .and_then(Package::unpack)
        {
            Some(rust) => {
                eprintln!("lower-std: {}: unchanged", package.display());
                rust
            }
            None => {
                let mut reused = 0;
                let rust =
                    lower_tools_files_cached(&paths, tools_build(), None, &cache, &mut reused)?;
                eprintln!(
                    "lower-std: {}: {reused} of {} files from the cache",
                    package.display(),
                    paths.len()
                );
                cache.put(whole.as_deref(), &rust.pack());
                rust
            }
        };
        let kept: Vec<&str> = rust.files.iter().map(|(name, _)| name.as_str()).collect();
        for stale in Package::stale(&package, &kept) {
            let _ = std::fs::remove_file(package.join(&stale));
            changed.push(package.join(stale));
        }
        let _ = std::fs::remove_file(package.join("package.rs"));
        std::fs::create_dir_all(package.join(TOOLS_FILES))
            .with_context(|| format!("creating {}", package.join(TOOLS_FILES).display()))?;
        for (name, text) in rust.files {
            lowered.push((package.join(name), text));
        }
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

/// **Which compiler wrote an entry** (#667): the language version, not the
/// binary's bytes. A rebuild of the same sources, in any checkout, then keeps
/// every entry; a change to the lowering raises the version (CLAUDE.md), which
/// starts the cache afresh. `NIKAIA_LOWER_CACHE_SALT` in the environment
/// separates entries by hand while a lowering is being worked on.
const COMPILER_IDENTITY: &str = concat!("nikaia-", env!("CARGO_PKG_VERSION"));

/// The file in an entry's directory that holds what was lowered.
const LOWERED: &str = "lowered.rs";

impl LoweredBefore {
    /// No cache: nothing is read and nothing is written.
    fn none() -> LoweredBefore {
        LoweredBefore {
            root: PathBuf::new(),
            compiler: None,
        }
    }

    fn open() -> LoweredBefore {
        LoweredBefore::with_compiler(&format!(
            "{COMPILER_IDENTITY}{}",
            std::env::var("NIKAIA_LOWER_CACHE_SALT").unwrap_or_default()
        ))
    }

    /// A cache read and written as the compiler `compiler`.
    fn with_compiler(compiler: &str) -> LoweredBefore {
        let compiler = Some(compiler.to_string());
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
        self.compiler.as_ref()?;
        let rust = std::fs::read_to_string(entry.join(LOWERED)).ok()?;
        cache::use_entry(&entry);
        Some(rust)
    }

    fn put(&self, key: Option<&str>, rust: &str) {
        let (Some(key), Some(_)) = (key, &self.compiler) else {
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

    /// #667: the key is the compiler's identity plus the files' names and
    /// text; it holds no binary bytes and no checkout path.
    #[test]
    fn the_lowering_key_follows_text_and_compiler_not_the_binary() {
        let dir = std::env::temp_dir().join(format!("nika-key-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.nika");
        std::fs::write(&file, "fn f() -> i64 sync { 1 }\n").unwrap();
        let files = [file.clone()];
        let one = LoweredBefore::with_compiler("c1").key("m", &files);
        assert_eq!(one, LoweredBefore::with_compiler("c1").key("m", &files));
        assert_ne!(one, LoweredBefore::with_compiler("c2").key("m", &files));
        std::fs::write(&file, "fn f() -> i64 sync { 2 }\n").unwrap();
        assert_ne!(one, LoweredBefore::with_compiler("c1").key("m", &files));
        let _ = std::fs::remove_dir_all(&dir);
        // Two opens, as before and after a rebuild, name the same compiler.
        assert_eq!(
            LoweredBefore::open().compiler,
            LoweredBefore::open().compiler
        );
        assert!(!COMPILER_IDENTITY.contains('/'));
    }

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
