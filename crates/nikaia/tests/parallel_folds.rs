//! **A fold over a walk on every core, and the cores it runs on**
//! ([ADR-372](../../../docs/specification/adr/adr-372.md) D1, D2, D5-D7, Part
//! II 12.6): `task::cores()`, `Par::fold(init, step, merge; order:, workers:)`
//! and `par_iter(piece:)`, each meaning the same at both settings of
//! `user_parallelism`.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Every fold the issue asks for (#717): a sum at both orders, with fewer and
/// more workers, with the smallest and the largest piece; a fold into a list
/// under `Kept`; over no elements; a type as `init`; and last, `workers: 0`.
const FOLDS: &str = r#"use std::par
use std::task

fn plus(a: i64, b: i64) -> i64 {
    return a + b
}

struct Tally {
    n: i64,
}

impl Tally {
    pub fn() -> Tally sync {
        return Tally { n: 0 }
    }
}

fn tally_add(t: Tally, x: i64) -> Tally {
    return Tally { n: t.n + x }
}

fn tally_merge(a: Tally, b: Tally) -> Tally {
    return Tally { n: a.n + b.n }
}

fn main() {
    println(f"{task::cores()}")
    let mut xs: Vec[i64] = []
    for i in 0..<1_000_000 {
        xs.push(i)
    }
    let mut sequential = 0
    for x in xs.iter() {
        sequential += x
    }
    let kept = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b })
    let any = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, plus; order: par::Order::Any)
    let two = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b }; workers: 2, order: par::Order::Kept)
    let many = xs.par_iter().map(fn(x) { x * 2 }).fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b }; workers: 64, order: par::Order::Any)
    let small = xs.par_iter(piece: 1).fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b })
    let large = xs.par_iter(piece: 1_000_000).fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b })
    println(f"{sequential} {kept} {any} {two} {many} {small} {large}")
    let listed = xs.par_iter().fold(fn {
        let none: Vec[i64] = []
        none
    }, fn(acc, x) {
        let mut l = acc
        l.push(x)
        return l
    }, fn(a, b) {
        let mut l = a
        for x in b {
            l.push(x)
        }
        return l
    })
    let mut out_of_order = 0
    for i in 0..<listed.len() {
        if listed[i] != xs[i] {
            out_of_order += 1
        }
    }
    println(f"{listed.len()} {out_of_order}")
    let empty: Vec[i64] = []
    let seven = empty.par_iter().fold(fn { 7 }, fn(acc, x) { acc + x }, fn(a, b) { a + b }; order: par::Order::Any)
    let eight = empty.par_iter().fold(fn { 8 }, fn(acc, x) { acc + x }, fn(a, b) { a + b })
    println(f"{seven} {eight}")
    let t = xs.par_iter().fold(Tally, fn(acc, x) { tally_add(acc, x) }, fn(a, b) { tally_merge(a, b) }; order: par::Order::Any)
    println(f"{t.n}")
    let none = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b }; workers: 0)
    println(f"not reached {none}")
}
"#;

/// Lower `source` at a setting of `user_parallelism`, compile it, and hand
/// back the binary and the emitted Rust.
fn build(dir: &Path, source: &str, parallel: &str) -> (PathBuf, String) {
    std::fs::create_dir_all(dir).expect("a directory");
    let input = dir.join("main.nika");
    std::fs::write(&input, source).expect("the source");
    let output = dir.join("main.rs");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .args(["lower", input.to_str().unwrap()])
        .args(["--output", output.to_str().unwrap()])
        .args(["--no-cache", "--user-parallelism", parallel])
        .env("NIKAIA_CACHE_DIR", dir.join("cache"))
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "lowering failed:\n{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let rust = std::fs::read_to_string(&output).expect("the emitted Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &output,
        &["--crate-type", "bin", "-o", binary.to_str().unwrap()],
    );
    assert!(
        compiled.status.success(),
        "the emitted Rust did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    (binary, rust)
}

/// Run `binary` with a runtime configuration file, or with none.
fn run(binary: &Path, at: &Path, config: Option<&str>) -> Output {
    std::fs::create_dir_all(at).expect("a directory to run in");
    let mut command = Command::new(binary);
    command.current_dir(at);
    match config {
        Some(text) => {
            let path = at.join("nikaia-runtime.toml");
            std::fs::write(&path, text).expect("the runtime configuration");
            command.env("NIKAIA_RUNTIME_CONFIG", &path);
        }
        None => {
            command.env_remove("NIKAIA_RUNTIME_CONFIG");
        }
    }
    command.output().expect("run the compiled program")
}

/// What every run prints after its first line, the cores: the sequential sum
/// six times and twice it once, a list in input order, `init`'s values over
/// no elements, and the sum through a type made by `init`.
fn the_folds(out: &Output) -> Vec<String> {
    let sum: i64 = (0..1_000_000).sum();
    let lines: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        lines[1..],
        [
            format!("{sum} {sum} {sum} {sum} {} {sum} {sum}", 2 * sum),
            "1000000 0".to_owned(),
            "7 8".to_owned(),
            format!("{sum}"),
        ],
        "{lines:?}"
    );
    // **`workers: 0` panics, naming the option** (D6), after everything else.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{lines:?}");
    assert!(stderr.contains("`workers:` is 0"), "{stderr}");
    lines
}

/// **D1, D2, D5-D7**: the same answers at both settings and both orders, and
/// `task::cores()` is 1 at `no`, `user-pool` at `yes`, and the threads the
/// process may run on where `user-pool` is `0`.
#[test]
fn a_fold_is_the_sequential_fold_at_both_settings_and_both_orders() {
    let dir = common::scratch_dir("parallel-folds");
    let (at_no, rust) = build(&dir.join("no"), FOLDS, "no");
    assert!(rust.contains(".fold_merged("), "{rust}");
    assert!(rust.contains("nikaia_std::par::pieced("), "{rust}");
    assert!(!rust.contains("nikaia_std::par::iter("), "{rust}");
    let lines = the_folds(&run(&at_no, &dir.join("run-no"), Some("user-pool = 3\n")));
    assert_eq!(lines[0], "1");

    let (at_yes, rust) = build(&dir.join("yes"), FOLDS, "yes");
    assert!(
        rust.contains("nikaia_std::par::iter_in_pieces(&xs, 1)"),
        "{rust}"
    );
    assert!(rust.contains(".fold_merged("), "{rust}");
    let lines = the_folds(&run(
        &at_yes,
        &dir.join("run-three"),
        Some("user-pool = 3\n"),
    ));
    assert_eq!(lines[0], "3");
    let lines = the_folds(&run(&at_yes, &dir.join("run-default"), None));
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    assert_eq!(lines[0], threads.to_string());
}

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    use nikaia::contracts::LedgerOps;
    let parsed = nikaia::parser::parse_to_ast(source).expect("the source parses");
    let own = nikaia::contracts::Ledger::infer(&parsed);
    let library =
        nikaia::contracts::Ledger::parse(nikaia::contracts::STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

/// **D2: `init`, `step` and `merge` are a walk's lambdas**: they may not
/// pause (`NK2209`) and may not change a name outside them (`NK2107`).
#[test]
fn a_folds_lambdas_are_asked_what_a_walks_are() {
    let pausing = "use std::time\n\
         \n\
         fn slow(x: i64) -> i64 {\n\
         \x20   time::sleep(1.millis())\n\
         \x20   return x\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let xs: Vec[i64] = [1, 2]\n\
         \x20   let s = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + slow(x) }, fn(a, b) { a + b })\n\
         \x20   println(f\"{s}\")\n\
         }\n";
    let found = findings(pausing);
    assert!(found.iter().any(|f| f.code == "NK2209"), "{found:#?}");
    let sharing = "fn main() {\n\
         \x20   let xs: Vec[i64] = [1, 2]\n\
         \x20   let mut merges = 0\n\
         \x20   let s = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) {\n\
         \x20       merges += 1\n\
         \x20       return a + b\n\
         \x20   })\n\
         \x20   println(f\"{s} {merges}\")\n\
         }\n";
    let found = findings(sharing);
    assert!(found.iter().any(|f| f.code == "NK2107"), "{found:#?}");
}
