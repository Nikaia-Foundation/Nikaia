//! **A program `nikaia check` accepts is lowered, compiled by `rustc`, and runs
//! with the expected output.** A `rustc` error after the checker accepted a
//! program is always a compiler bug.
//!
//! `tests/accepted/` holds one small program per case, each with its expected
//! standard output beside it (`<name>.out`): the probes of fixed issues and
//! their variants, which must not slide back to `rustc`. A directory there
//! holding `app/nikaia.toml` is a project of several packages, built and run
//! by `nikaia run`, its output in `<directory>.out`.
//!
//! **One `rustc` for all the programs**: each lowered program becomes a module
//! of one crate, and its binary runs each by name. Only when that crate does
//! not compile is each program compiled alone, so that every failing program
//! is named with its first error - one error may stop `rustc` before it checks
//! the others.

use crate::common;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use nikaia::emit::Build;

fn accepted_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/accepted")
}

/// A program of the suite: its name, the module it becomes, its source.
struct Case {
    name: String,
    module: String,
    path: PathBuf,
}

fn cases() -> Vec<Case> {
    let mut found: Vec<Case> = std::fs::read_dir(accepted_dir())
        .expect("tests/accepted")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "nika"))
        .map(|path| {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("utf-8 name")
                .to_string();
            let module = format!("case_{}", name.replace(|c: char| !c.is_alphanumeric(), "_"));
            Case { name, module, path }
        })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn projects() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(accepted_dir())
        .expect("tests/accepted")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("app/nikaia.toml").is_file())
        .collect();
    found.sort();
    found
}

fn expected(name: &str) -> String {
    let path = accepted_dir().join(format!("{name}.out"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{name}: no expected output {}: {e}", path.display()))
}

/// Run `work` over `items` on as many threads as the machine has.
fn in_parallel<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = Mutex::new(0usize);
    let results: Mutex<Vec<Option<R>>> = Mutex::new(items.iter().map(|_| None).collect());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..threads.min(items.len().max(1)) {
            scope.spawn(|| {
                loop {
                    let at = {
                        let mut n = next.lock().expect("queue");
                        let at = *n;
                        *n += 1;
                        at
                    };
                    let Some(item) = items.get(at) else { break };
                    let result = work(item);
                    results.lock().expect("results")[at] = Some(result);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("results")
        .into_iter()
        .map(|r| r.expect("every item ran"))
        .collect()
}

/// `nikaia check` on one file: `None` when it accepts, else what it said.
fn refused(path: &Path) -> Option<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .arg("check")
        .arg(path)
        .output()
        .expect("the nikaia binary runs");
    if out.status.success() {
        return None;
    }
    Some(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

/// The first error `rustc` reports, with the lines that explain it.
fn first_error(stderr: &str) -> String {
    let mut out = Vec::new();
    for line in stderr.lines().skip_while(|l| !l.starts_with("error")) {
        if !out.is_empty() && (line.starts_with("error") || line.starts_with("warning")) {
            break;
        }
        out.push(line);
    }
    out.join("\n")
}

/// The lowered program as a module of the one crate: its `main` is called by
/// name.
fn as_module(module: &str, rust: &str) -> String {
    format!(
        "pub mod {module} {{\n{}\n}}\n",
        rust.replacen("\nfn main() {", "\npub fn main() {", 1)
    )
}

/// A binary to run, with the arguments that pick a program, or why there is
/// none.
type Built = Result<(PathBuf, Vec<String>), String>;

fn compile_binary(dir: &Path, rust: &str) -> Result<PathBuf, String> {
    let source = dir.join("accepted.rs");
    std::fs::write(&source, rust).expect("write the emitted Rust");
    let binary = dir.join("accepted");
    let compiled = common::compile(
        &source,
        &[
            "--crate-type",
            "bin",
            "-A",
            "warnings",
            "-o",
            binary.to_str().expect("utf-8 path"),
        ],
    );
    if compiled.status.success() {
        Ok(binary)
    } else {
        Err(first_error(&String::from_utf8_lossy(&compiled.stderr)))
    }
}

fn ran(binary: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new(binary)
        .args(args)
        .output()
        .expect("run the program");
    if !out.status.success() {
        return Err(format!(
            "exits with {}:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn every_accepted_program_compiles_and_prints_what_it_should() {
    let started = std::time::Instant::now();
    let cases = cases();
    assert!(!cases.is_empty(), "tests/accepted holds no program");
    let mut failures: Vec<String> = Vec::new();

    // Every program has its output, and every output its program.
    for entry in std::fs::read_dir(accepted_dir())
        .expect("tests/accepted")
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "out") {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let source = accepted_dir().join(format!("{stem}.nika"));
            let project = accepted_dir().join(stem).join("app/nikaia.toml");
            if !source.is_file() && !project.is_file() {
                failures.push(format!("{stem}.out: no program of that name"));
            }
        }
    }

    // The checker accepts each, and each lowers.
    let lowered: Vec<Result<String, String>> = in_parallel(&cases, |case| {
        if let Some(said) = refused(&case.path) {
            return Err(format!("{}: `nikaia check` refuses it:\n{said}", case.name));
        }
        let program = nikaia::modules::Program::read_one(&case.path)
            .map_err(|e| format!("{}: does not read: {e:#}", case.name))?;
        let emitted = program
            .emit(Build::default())
            .map_err(|e| format!("{}: does not lower: {e:#}", case.name))?;
        Ok(emitted.rust)
    });
    let mut good: Vec<(&Case, String)> = Vec::new();
    for (case, result) in cases.iter().zip(lowered) {
        match result {
            Ok(rust) => good.push((case, rust)),
            Err(e) => failures.push(e),
        }
    }

    let checked_and_lowered = started.elapsed();
    // One crate, one `rustc`.
    let dir = common::scratch_dir("accepted");
    let mut crate_text = String::new();
    let mut dispatch =
        String::from("fn main() {\n    match std::env::args().nth(1).as_deref() {\n");
    for (case, rust) in &good {
        crate_text.push_str(&as_module(&case.module, rust));
        dispatch.push_str(&format!(
            "        Some(\"{}\") => {{\n            let _ = {}::main();\n        }}\n",
            case.name, case.module
        ));
    }
    dispatch.push_str("        _ => std::process::exit(2),\n    }\n}\n");
    crate_text.push_str(&dispatch);
    let built: Vec<(String, Built)> = match compile_binary(&dir, &crate_text) {
        Ok(binary) => good
            .iter()
            .map(|(case, _)| {
                (
                    case.name.clone(),
                    Ok((binary.clone(), vec![case.name.clone()])),
                )
            })
            .collect(),
        // Each alone, to name every program `rustc` refuses.
        Err(_) => in_parallel(&good, |(case, rust)| {
            let own = common::scratch_dir(&format!("accepted-{}", case.name));
            let result = compile_binary(&own, rust)
                .map(|binary| (binary, Vec::new()))
                .map_err(|e| {
                    format!(
                        "{}: `nikaia check` accepts it, `rustc` refuses it:\n{e}",
                        case.name
                    )
                });
            (case.name.clone(), result)
        }),
    };

    let compiled = started.elapsed();
    // Each runs and prints what it should.
    let outcomes: Vec<Option<String>> = in_parallel(&built, |(name, result)| {
        let (binary, args) = match result {
            Ok(found) => found,
            Err(e) => return Some(e.clone()),
        };
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        match ran(binary, &args) {
            Err(e) => Some(format!("{name}: {e}")),
            Ok(out) if out != expected(name) => Some(format!(
                "{name}: prints\n{out}--- where it should print ---\n{}",
                expected(name)
            )),
            Ok(_) => None,
        }
    });
    failures.extend(outcomes.into_iter().flatten());

    // A project of several packages, built and run as a user does.
    for project in projects() {
        let name = project
            .file_name()
            .and_then(|s| s.to_str())
            .expect("utf-8 name")
            .to_string();
        let copy = common::scratch_dir(&format!("accepted-{name}"));
        copy_tree(&project, &copy);
        let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
            .arg("run")
            .arg("--project")
            .arg(copy.join("app"))
            .env_remove("CARGO_TARGET_DIR")
            .output()
            .expect("the nikaia binary runs");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        if !out.status.success() {
            failures.push(format!(
                "{name}: does not build or run:\n{}",
                first_error(&String::from_utf8_lossy(&out.stderr))
            ));
        } else if stdout != expected(&name) {
            failures.push(format!(
                "{name}: prints\n{stdout}--- where it should print ---\n{}",
                expected(&name)
            ));
        }
        std::fs::remove_dir_all(&copy).ok();
    }

    std::fs::remove_dir_all(&dir).ok();
    eprintln!(
        "{} programs and {} projects in {:.1?} (checked and lowered at {:.1?}, compiled at {:.1?})",
        cases.len(),
        projects().len(),
        started.elapsed(),
        checked_and_lowered,
        compiled
    );
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy a project");
    for entry in std::fs::read_dir(from).expect("read a project").flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            if entry.file_name() != "target" {
                copy_tree(&path, &target);
            }
        } else {
            std::fs::copy(&path, &target).expect("copy a file");
        }
    }
}
