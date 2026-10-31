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

/// A default's type as its run builds it: Nikaia's
/// (`tools/comptime_tree.nika`).
pub(crate) use nikaia_std::tools::comptime_tree::comptime_as_built as as_built;

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

/// **What a bundle is built for.** Build-time code counts what it allocates
/// against its bound (D10); a program `nikaia run` compiles against the
/// bundle ([`crate::run_fast`]) means what its Cargo build means, so its
/// bundle has no counting allocator and the profile the generated workspace
/// gives a dependency: no overflow checks, no debug assertions, no debug
/// information, unwinding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flavour {
    BuildTime,
    Run,
}

/// The run bundle's one file: the crates, named, and nothing else.
const RUN_SOURCE: &str = "// GENERATED: the library `nikaia run` links a program against.
pub extern crate nikaia_std;
pub extern crate winnow;
pub extern crate winnow_grammar;
";

/// The run bundle's profile: what `[profile.dev]` of a generated workspace
/// says about a crate that is not the program's.
const RUN_PROFILE: &str = "
[profile.dev]
overflow-checks = false
debug-assertions = false
debug = 0
panic = \"unwind\"
";

/// The crates the bundle names: `std`, and what a `grammar` lowers to.
const NAMED: [&str; 3] = ["nikaia_std", "winnow", "winnow_grammar"];

/// Build the bundle, or say why it could not be built.
///
/// **Once per machine, not per project**: what it is depends on its manifest
/// (where `std` comes from), its one file and the toolchain, never on the
/// program - so it lives under the user cache, keyed by the first two, as the
/// compiled `std` of ADR-002 D4 does. Under each project's own workshop it was
/// built again for every new project, some forty crates (half a minute) before
/// the first `comptime` could run. Cargo's lock on the target directory is what
/// makes two builds at once wait for one another rather than collide.
pub(crate) fn bundle(_workshop: &Path) -> Result<Bundle, String> {
    bundle_of(Flavour::BuildTime)
}

/// **The bundle `nikaia run` links a program against**, built once and then
/// found from its record without starting Cargo: Cargo's start and freshness
/// check is most of what a fast run saves. The record is trusted while this
/// compiler, the toolchain and `std`'s sources are what they were when it was
/// written, and every file it names is still there.
pub(crate) fn run_bundle() -> Result<Bundle, String> {
    let (manifest, source) = bundle_text(Flavour::Run);
    let at = bundle_dir(Flavour::Run, &manifest, &source);
    let record = at.join("built");
    let fingerprint = run_fingerprint();
    if let Some(bundle) = std::fs::read_to_string(&record)
        .ok()
        .and_then(|text| read_record(&text, &fingerprint))
    {
        return Ok(bundle);
    }
    let bundle = bundle_of(Flavour::Run)?;
    let mut text = format!(
        "{fingerprint}\n{}\n{}\n{}\n",
        bundle.dylib.display(),
        bundle.deps.display(),
        bundle.libdir.display()
    );
    for (name, rlib) in &bundle.rlibs {
        text.push_str(&format!("{name}={}\n", rlib.display()));
    }
    write_if_changed(&record, &text)?;
    Ok(bundle)
}

/// What the run bundle's record is valid for: this compiler, the toolchain
/// and the newest of `std`'s sources (a `std` edited under a development
/// checkout is built again, as Cargo would).
fn run_fingerprint() -> String {
    let mut newest = 0u128;
    let mut count = 0usize;
    let mut pending = vec![crate::sysroot::Sysroot::resolve().std_dir()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == "tests")
            {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                pending.push(path);
                continue;
            }
            count += 1;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            newest = newest.max(modified);
        }
    }
    crate::assets::digest(
        format!(
            "{}\n{}\n{}\n{newest}:{count}",
            env!("NIKAIA_COMPILER"),
            orchestrator::project::rustc_version(),
            rustc()
        )
        .as_bytes(),
    )
}

/// The bundle a record names, where it is still the one it names.
fn read_record(text: &str, fingerprint: &str) -> Option<Bundle> {
    let mut lines = text.lines();
    if lines.next()? != fingerprint {
        return None;
    }
    let dylib = PathBuf::from(lines.next()?);
    let deps = PathBuf::from(lines.next()?);
    let libdir = PathBuf::from(lines.next()?);
    let rlibs: Vec<(String, PathBuf)> = lines
        .filter_map(|line| line.split_once('='))
        .map(|(name, rlib)| (name.to_string(), PathBuf::from(rlib)))
        .collect();
    if rlibs.len() != NAMED.len() {
        return None;
    }
    let stamp = stamp_of(&dylib, &rlibs).ok()?;
    Some(Bundle {
        dylib,
        rlibs,
        deps,
        stamp,
        libdir,
    })
}

/// A bundle's manifest and its one file.
fn bundle_text(flavour: Flavour) -> (String, String) {
    let mut manifest = nikaia_std::tools::comptime_driver::cr_bundle_manifest(
        &crate::project::runtime_dependencies_for(NAMED.join(" ").as_str()),
    );
    let source = match flavour {
        Flavour::BuildTime => nikaia_std::tools::comptime_driver::cr_bundle_source(),
        Flavour::Run => {
            manifest.push_str(RUN_PROFILE);
            RUN_SOURCE.to_string()
        }
    };
    (manifest, source)
}

/// Where a bundle lives: the user cache, keyed by its manifest and its file.
fn bundle_dir(flavour: Flavour, manifest: &str, source: &str) -> PathBuf {
    let key = orchestrator::cache::sha256_hex(format!("{manifest}\n{source}").as_bytes());
    orchestrator::cache::Layout::user_cache_dir()
        .join(match flavour {
            Flavour::BuildTime => "build-time-bundle",
            Flavour::Run => "run-bundle",
        })
        .join(&key[..16])
}

/// The library and the crates it names, and when each was last built.
fn stamp_of(dylib: &PathBuf, rlibs: &[(String, PathBuf)]) -> Result<String, String> {
    Ok(std::iter::once(dylib)
        .chain(rlibs.iter().map(|(_, rlib)| rlib))
        .map(|file| {
            let meta = std::fs::metadata(file).map_err(|e| format!("{}: {e}", file.display()))?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            Ok(format!("{}:{}:{modified}", file.display(), meta.len()))
        })
        .collect::<Result<Vec<String>, String>>()?
        .join("\n"))
}

fn bundle_of(flavour: Flavour) -> Result<Bundle, String> {
    let (manifest, bundle_source) = bundle_text(flavour);
    let at = bundle_dir(flavour, &manifest, &bundle_source);
    orchestrator::cache::use_entry(&at);
    let dir = at.join("bundle");
    std::fs::create_dir_all(dir.join("src")).map_err(|e| format!("{}: {e}", dir.display()))?;
    write_if_changed(&dir.join("Cargo.toml"), &manifest)?;
    write_if_changed(&dir.join("src").join("lib.rs"), &bundle_source)?;
    let built = Command::new(cargo())
        .current_dir(orchestrator::cache::Layout::where_tools_start())
        .args(["rustc", "--quiet", "--lib", "--crate-type", "dylib"])
        .arg("--message-format=json")
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(at.join("target"))
        .args(["--", "-C", "prefer-dynamic"])
        .output()
        .map_err(|e| format!("running cargo: {e}"))?;
    if !built.status.success() {
        return Err(String::from_utf8_lossy(&built.stderr).to_string());
    }
    let mut dylib = None;
    let mut rlibs: Vec<(String, PathBuf)> = Vec::new();
    for line in String::from_utf8_lossy(&built.stdout).lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if message["reason"] != "compiler-artifact" {
            continue;
        }
        let files = message["filenames"].as_array().cloned().unwrap_or_default();
        let files = files.iter().filter_map(|f| f.as_str()).map(PathBuf::from);
        match message["target"]["name"].as_str() {
            Some(name) if NAMED.contains(&name) => {
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
    let (Some(dylib), true) = (dylib, rlibs.len() == NAMED.len()) else {
        return Err("cargo built the bundle and named no library".to_string());
    };
    let deps = rlibs[0]
        .1
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "no directory for std".to_string())?;
    let stamp = stamp_of(&dylib, &rlibs)?;
    Ok(Bundle {
        dylib,
        rlibs,
        deps,
        stamp,
        libdir: PathBuf::from(target_libdir()),
    })
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

/// **Compile and run one build-time program**, or hand back what it said when
/// it ran before: its standard output, or `Wall::Refused` with what it wrote
/// where it stopped. Keyed on the program, the toolchain and the bundle (D4).
pub(crate) fn run(at: &Path, bundle: &Bundle, program: &str, name: &str) -> Result<String, Wall> {
    // **A program that asks `target::has` answers for the build's level**
    // (ADR-371 D6): the level is in its key and handed to it; one that does
    // not keeps one answer for every level.
    let level = match program.contains("target::has") {
        true => CPU_LEVEL
            .lock()
            .map(|held| held.clone())
            .unwrap_or_default(),
        false => String::new(),
    };
    let key = crate::assets::digest(
        format!("{program}\n{}\n{}\n{level}", toolchain(), bundle.stamp).as_bytes(),
    );
    // What the run reads: the level's name, `native` for this machine's.
    let named = match level.starts_with("native=") {
        true => "native".to_string(),
        false => level,
    };
    let dir = at.join("comptime").join(&key);
    let answered = dir.join("answer");
    let stopped = dir.join("stopped");
    if let Ok(answer) = std::fs::read_to_string(&answered) {
        return Ok(answer);
    }
    if let Ok(said) = std::fs::read_to_string(&stopped) {
        return Err(Wall::Refused { detail: said });
    }
    let did_not = |detail: String| Wall::DidNotBuild { detail };
    // **A program that did not compile is not compiled again** in this
    // process: the same text fails the same way, and a check asks for one
    // value many times over (each test, each variant).
    if let Some(detail) = NOT_BUILT
        .lock()
        .ok()
        .and_then(|held| held.get(&key).cloned())
    {
        return Err(did_not(detail));
    }
    let source = dir.join("main.rs");
    write_if_changed(&source, program).map_err(did_not)?;
    // **Each build compiles and runs its own binary, in a directory of its
    // own**, moved into place after: `rustc` names its object files after the
    // crate, so two builds of the same program side by side wrote, linked and
    // deleted each other's.
    let scratch = unique_beside(&dir.join("build"));
    std::fs::create_dir_all(&scratch)
        .map_err(|e| did_not(format!("{}: {e}", scratch.display())))?;
    let binary = scratch.join("run");
    let compiled = Command::new(rustc())
        .current_dir(orchestrator::cache::Layout::where_tools_start())
        .args([
            "--edition=2024",
            "--crate-name",
            "comptime",
            "--crate-type",
            "bin",
        ])
        .args(["-C", "opt-level=0", "-C", "overflow-checks=on"])
        .args([
            "-C",
            "debug-assertions=off",
            "-C",
            "prefer-dynamic",
            "-A",
            "warnings",
        ])
        .arg("--extern")
        .arg(format!("nikaia_bundle={}", bundle.dylib.display()))
        .args(bundle.rlibs.iter().flat_map(|(name, rlib)| {
            ["--extern".to_string(), format!("{name}={}", rlib.display())]
        }))
        .arg("-L")
        .arg(format!("dependency={}", bundle.deps.display()))
        .arg("-o")
        .arg(&binary)
        .arg(&source)
        .output()
        .map_err(|e| did_not(format!("running rustc: {e}")))?;
    if !compiled.status.success() {
        let _ = std::fs::remove_dir_all(&scratch);
        let detail = String::from_utf8_lossy(&compiled.stderr).to_string();
        if let Ok(mut held) = NOT_BUILT.lock() {
            held.insert(key, detail.clone());
        }
        return Err(did_not(detail));
    }
    let mut libraries: Vec<PathBuf> = Vec::new();
    if let Some(dir) = bundle.dylib.parent() {
        libraries.push(dir.to_path_buf());
    }
    libraries.push(bundle.deps.clone());
    libraries.push(PathBuf::from(target_libdir()));
    if let Some(held) = std::env::var_os(LIBRARY_PATH) {
        libraries.extend(std::env::split_paths(&held));
    }
    let child = Command::new(&binary)
        .env("RUST_BACKTRACE", "0")
        .env(nikaia_std::target::LEVEL_VAR, &named)
        .env(
            LIBRARY_PATH,
            std::env::join_paths(libraries).map_err(|e| did_not(e.to_string()))?,
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| did_not(format!("running {}: {e}", binary.display())))?;
    let ran =
        waited(child, name).map_err(|e| did_not(format!("running {}: {e}", binary.display())));
    let _ = std::fs::rename(&binary, dir.join("run"));
    let _ = std::fs::remove_dir_all(&scratch);
    let ran = ran?;
    if ran.status.success() {
        let answer = String::from_utf8_lossy(&ran.stdout).trim().to_string();
        write_if_changed(&answered, &answer).map_err(did_not)?;
        return Ok(answer);
    }
    let mut said = String::from_utf8_lossy(&ran.stderr).trim().to_string();
    // **A run that ended without a word** - killed by the system, most often
    // for its memory - says how it ended instead.
    if said.is_empty() {
        said = format!("it ended without saying why ({})", ran.status);
    }
    // **A stop the program reported is an answer**, and kept as one: the same
    // code stops the same way. A run that died without a word is not, and is
    // tried again next time.
    if said.contains("the program stopped") || said.contains("nikaia-build-time:") {
        write_if_changed(&stopped, &said).map_err(did_not)?;
    }
    Err(Wall::Refused { detail: said })
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

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

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

/// Written only when it changed, so that Cargo's freshness keeps working.
pub(crate) fn write_if_changed(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    if std::fs::read_to_string(path).is_ok_and(|held| held == text) {
        return Ok(());
    }
    // **Written whole or not at all**: builds side by side share the
    // workshop, and one that read a file another was still writing compiled
    // half a program or read half an answer.
    let partial = unique_beside(path);
    std::fs::write(&partial, text).map_err(|e| format!("{}: {e}", partial.display()))?;
    std::fs::rename(&partial, path).map_err(|e| {
        let _ = std::fs::remove_file(&partial);
        format!("{}: {e}", path.display())
    })
}

/// A path beside `path` that no other build writes at the same time.
pub(crate) fn unique_beside(path: &Path) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.{n}.partial", std::process::id()));
    path.with_file_name(name)
}
