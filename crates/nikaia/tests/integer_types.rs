//! **The integers a program writes** (Part I 2.2): `int`, and the five fixed
//! widths `i64`, `i32`, `u8`, `u64` and `u32`
//! ([ADR-285](../../../docs/specification/adr/adr-285.md) D36-D39).
//!
//! Two halves. The open numbers (#629): an `as` out of a number no annotation
//! typed is checked as out of a typed one (D-1), a method of one does not make
//! the function `async` (D-2), and Rust's other widths are `NK1135` outside an
//! `extern "C"` declaration (D-3). And `int` (#630): an `i64`'s values and
//! aborts, taking an `i64` without `as`, lowered as an `i64`, and `NK1262` where
//! C reads the bits.

mod common;

use std::path::Path;
use std::process::Command;

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

fn lowered(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Build::default())
        .expect("it lowers")
        .rust
}

/// Lowers `source` with the binary, compiles it with the overflow check on, and
/// runs it: the program's stdout and stderr, and whether it exited cleanly.
fn run(purpose: &str, source: &str) -> (bool, String, String) {
    let dir = common::scratch_dir(purpose);
    let input = dir.join("main.nika");
    std::fs::write(&input, source).expect("the source");
    let output = dir.join("main.rs");
    let lowering = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .args(["lower", input.to_str().expect("utf-8 path")])
        .args(["--output", output.to_str().expect("utf-8 path")])
        .args(["--no-cache"])
        .env("NIKAIA_CACHE_DIR", dir.join("cache"))
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowering.status.success(),
        "lowering failed:\n{}{}",
        String::from_utf8_lossy(&lowering.stdout),
        String::from_utf8_lossy(&lowering.stderr)
    );
    let binary = dir.join("program");
    let compiled = compile(&output, &binary);
    assert!(
        compiled.status.success(),
        "the emitted Rust did not compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let ran = Command::new(&binary).output().expect("run the program");
    let _ = std::fs::remove_dir_all(&dir);
    (
        ran.status.success(),
        String::from_utf8_lossy(&ran.stdout).into_owned(),
        String::from_utf8_lossy(&ran.stderr).into_owned(),
    )
}

fn compile(source: &Path, binary: &Path) -> std::process::Output {
    common::compile(
        source,
        &[
            "--crate-type",
            "bin",
            "-C",
            "overflow-checks=on",
            "-o",
            binary.to_str().expect("utf-8 path"),
        ],
    )
}

/// **#629 D-1: an `as` out of an open number is checked** as out of a typed one
/// (ADR-285 D12). `v` is an `i64` because `parse_i64` gives it one; the cast was
/// Rust's truncating `as` and printed 44.
const OPEN_CAST: &str = "use std::text

fn conv(s: ref String) -> u8 {
    let mut v = -1
    v = text::parse_i64(s) ?? 0
    return v as u8
}

fn main() {
    println(f\"{conv(\"300\")}\")
}
";

#[test]
fn a_cast_out_of_an_open_number_is_checked() {
    let rust = lowered(OPEN_CAST);
    assert!(rust.contains("u8::try_from(v)"), "{rust}");
    let (clean, out, err) = run("open-number-cast", OPEN_CAST);
    assert!(!clean, "it must stop, and printed {out:?}");
    assert!(!out.contains("44"), "{out}");
    assert!(err.contains("does not fit in an `u8`"), "{err}");
}

/// A cast out of an open number that nothing else types still has its type
/// written, so the language below can read the checked conversion.
#[test]
fn a_cast_out_of_an_unasked_open_number_writes_its_type() {
    let rust = lowered(
        "fn small() -> u8 {\n\
         \x20   let n = 7\n\
         \x20   return n as u8\n\
         }\n\
         fn main() {\n\
         \x20   println(f\"{small()}\")\n\
         }\n",
    );
    assert!(rust.contains("let n: i32 = 7;"), "{rust}");
    assert!(rust.contains("u8::try_from(n)"), "{rust}");
}

/// **#629 D-2: a method of an open number does not pause.** `digits` was an
/// `async fn` and every caller `.await`ed it; and the method is a use whose
/// result is the number's own type, so the `return` types `n`.
#[test]
fn a_method_of_an_open_number_keeps_the_function_plain() {
    let source = "fn digits() -> i64 {\n\
                  \x20   let n = 5\n\
                  \x20   return n.abs()\n\
                  }\n\
                  fn main() {\n\
                  \x20   println(f\"{digits()}\")\n\
                  }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    let rust = lowered(source);
    assert!(rust.contains("\nfn digits() -> i64"), "{rust}");
    assert!(!rust.contains("async fn digits"), "{rust}");
    assert!(rust.contains("let n: i64 = 5;"), "{rust}");
    let (clean, out, err) = run("open-number-method", source);
    assert!(clean, "{err}");
    assert_eq!(out.trim(), "5");
}
