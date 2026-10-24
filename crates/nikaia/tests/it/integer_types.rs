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

use crate::common;
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

/// **#629 D-3: Rust's other widths are `NK1135`** outside an `extern "C"`
/// declaration (Part I 2.2), in a parameter, a result, a field and a `let`.
#[test]
fn widths_the_language_does_not_offer_are_refused() {
    for source in [
        "fn g(n: i16) -> i64 {\n    return 1\n}\n",
        "fn g(n: i64) -> usize {\n    return 1\n}\n",
        "struct S {\n    a: i8,\n}\n",
        "fn main() {\n    let x: usize = 3\n}\n",
        "fn g(n: u128) -> i64 {\n    return 1\n}\n",
        "fn g(xs: Vec[u16]) -> i64 {\n    return 1\n}\n",
    ] {
        let found = findings(source);
        assert!(
            found
                .iter()
                .any(|f| f.code == "NK1135" && f.message.starts_with("There's no type called")),
            "{source}\n{found:#?}"
        );
    }
}

#[test]
fn an_extern_c_signature_still_takes_rusts_widths() {
    let found = findings(
        "extern fn width(n: usize) -> i16 {\n\
         \x20   return 1\n\
         }\n\
         extern struct Laid {\n\
         \x20   n: u16,\n\
         }\n",
    );
    assert!(!found.iter().any(|f| f.code == "NK1135"), "{found:#?}");
}

/// **#630: `int` holds an `i64`'s values** and takes an `i64` without `as` in
/// both directions (ADR-285 D36).
const WITH_INT: &str = "fn f(a: int) -> int {
    return a + 5
}

fn main() {
    let x: i64 = f(1)
    let y: int = x
    println(f\"{x} {f(y)}\")
}
";

#[test]
fn an_int_checks_and_runs() {
    assert!(findings(WITH_INT).is_empty(), "{:#?}", findings(WITH_INT));
    let (clean, out, err) = run("int-runs", WITH_INT);
    assert!(clean, "{err}");
    assert_eq!(out.trim(), "6 11");
}

/// **`int` is lowered as an `i64`** (D37: no narrowing is built yet): the same
/// Rust as the program written with `i64`.
#[test]
fn an_int_lowers_as_an_i64() {
    let written_wide = WITH_INT
        .replace(": int", ": i64")
        .replace("-> int", "-> i64");
    assert_eq!(lowered(WITH_INT), lowered(&written_wide));
}

/// **`int` overflows where an `i64` does**, read at run time.
#[test]
fn an_int_aborts_at_the_i64_bound() {
    let source = "use std::text

fn next(a: int) -> int {
    return a + 1
}

fn main() {
    let read: int = text::parse_i64(\"9223372036854775807\") ?? 0
    println(f\"{next(read)}\")
}
";
    let (clean, out, err) = run("int-overflows", source);
    assert!(!clean, "it must stop, and printed {out:?}");
    assert!(err.contains("overflow"), "{err}");
}

/// **`NK1262`: `int` where C reads the bits** (D38), in an `extern "C"`
/// function and in an `extern "C"` block.
#[test]
fn an_int_at_the_c_boundary_is_refused() {
    for source in [
        "extern fn g(n: int) -> i64 {\n    return 1\n}\n",
        "extern fn g(n: i64) -> int {\n    return 1\n}\n",
        "extern struct Laid {\n    n: int,\n}\n",
        "extern {\n    fn abs(n: int) -> i32\n}\n",
    ] {
        let found = findings(source);
        assert!(
            found.iter().any(|f| f.code == "NK1262"),
            "{source}\n{found:#?}"
        );
    }
}

/// **The owner's target** (#630): `int` through a `let` and a call. The
/// `whole > 0` is the prover's (#632), so only the types are asked here.
#[test]
fn the_owners_example_checks_with_int() {
    let found = findings(
        "fn positive(whole: int) -> int {\n\
         \x20   assert(whole > 0)\n\
         \x20   return whole\n\
         }\n\
         fn example(a: int) -> int {\n\
         \x20   let whole = 10 + a\n\
         \x20   return positive(whole)\n\
         }\n",
    );
    assert!(
        !found
            .iter()
            .any(|f| f.code.starts_with("NK11") || f.code.starts_with("NK12")),
        "{found:#?}"
    );
}

/// **The prover reads an `int` as it reads an `i64`** (D36): a claim about an
/// `int` parameter is a precondition, carried back from a callee, and a fact
/// after it. Before, every claim over an `int` was *not a whole number the
/// prover reads* and stayed a check.
#[test]
fn the_prover_reads_an_int() {
    let source = "fn positive(whole: int) -> int {\n\
                  \x20   assert(whole > 0)\n\
                  \x20   return whole\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let a: int = 3\n\
                  \x20   let b = 4 * (10 + positive(a))\n\
                  \x20   assert(a > 0)\n\
                  \x20   println(f\"{b}\")\n\
                  }\n";
    let dir = common::scratch_dir("int-prover");
    std::fs::write(dir.join("claims.nika"), source).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "claims.nika", "--output", "claims.rs", "--asserts"])
        .output()
        .expect("the nikaia binary runs");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("claims.nika:2  assert(whole > 0)  precondition of `positive`"),
        "{stdout}"
    );
    assert!(
        stdout.contains("claims.nika:9  assert(a > 0)  proved"),
        "{stdout}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
