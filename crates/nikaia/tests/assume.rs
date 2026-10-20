//! **`unsafe { assume(claim) }`, a claim the program vouches for**
//! ([ADR-376](../../../docs/specification/adr/adr-376.md)): written only inside
//! `unsafe` (`NK1294`) and never in `std` (`NK1295`), a fact from where it
//! stands (D1), checked as an `assert` is in a build that removes no checks and
//! believed in one that does (D2), and listed by `--asserts` (D3).

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use nikaia::bounds::BoundsChecks;
use nikaia::check::CodeOps;
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

/// An index the prover cannot show in range, vouched for by the program.
const PICK: &str = "use std::cli\n\
                    \n\
                    fn pick(xs: ref Vec[i64], i: i64) -> i64 {\n\
                    \x20   unsafe { assume(i >= 0 && i < xs.len()) }\n\
                    \x20   return xs[i]\n\
                    }\n\
                    \n\
                    fn main() {\n\
                    \x20   let xs: Vec[i64] = [10, 20, 30]\n\
                    \x20   let i = cli::args().len() + AT\n\
                    \x20   println(f\"{pick(xs, i)}\")\n\
                    }\n";

fn codes(source: &str) -> Vec<&'static str> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .findings
        .iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .map(|f| f.code_str())
        .collect()
}

/// The lowered body of the function `name`, up to the next `fn`.
fn body<'a>(rust: &'a str, name: &str) -> &'a str {
    let from = rust
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("no `{name}` in:\n{rust}"));
    let rest = &rust[from + 3..];
    let to = rest.find("fn ").unwrap_or(rest.len());
    &rest[..to]
}

/// The program lowered with `optimization`, compiled and run with `input` on
/// its standard input: whether it ran to the end, and what it printed to each
/// stream.
fn run(purpose: &str, source: &str, optimization: &str, input: &str) -> (bool, String, String) {
    let dir = common::scratch_dir(&format!("assume-{purpose}"));
    std::fs::write(dir.join("assume.nika"), source).expect("write the source");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "assume.nika", "--output", "assume.rs"])
        .args(["--optimization", optimization])
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "{purpose} did not lower:\n{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let binary = dir.join("assume");
    let compiled = common::compile(
        &dir.join("assume.rs"),
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let mut child = Command::new(&binary)
        .env_remove("RUST_BACKTRACE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run it");
    child
        .stdin
        .take()
        .expect("its input")
        .write_all(input.as_bytes())
        .expect("write its input");
    let out = child.wait_with_output().expect("it ends");
    std::fs::remove_dir_all(&dir).ok();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// D1, D2: in a build that removes bounds checks, the index the `assume`
/// vouches for is read without a check, and the claim lowers to
/// `assert_unchecked`.
#[test]
fn an_assumed_index_carries_no_check_where_checks_are_removed() {
    let parsed = parse_to_ast(&PICK.replace("AT", "1")).expect("the source parses");
    let how = Build {
        bounds: BoundsChecks::Removed,
        ..Build::default()
    };
    let rust = emit_program(&parsed, how).expect("it lowers").rust;
    let pick = body(&rust, "pick");
    assert!(
        pick.contains("core::hint::assert_unchecked("),
        "the claim is believed:\n{pick}"
    );
    assert!(!pick.contains("assumption_failed"), "{pick}");
    assert!(
        !pick.contains("index::at("),
        "the index is unchecked:\n{pick}"
    );
    let (ok, out, err) = run(
        "on",
        &PICK.replace("AT", "1"),
        "remove-bounds-checks:on",
        "",
    );
    assert!(ok, "{err}");
    assert_eq!(out.trim(), "30");
}

/// D2: in a build that removes no checks, a false `assume` stops the program
/// as a false `assert` does, saying it was an assumption.
#[test]
fn a_false_assumption_stops_where_no_checks_are_removed() {
    let (ok, out, err) = run(
        "off",
        &PICK.replace("AT", "5"),
        "remove-bounds-checks:off",
        "",
    );
    assert!(!ok, "the index 6 is past the end: {out}");
    assert!(err.contains("assumption failed"), "{err}");
    assert!(err.contains("assume(i >= 0 && i < xs.len())"), "{err}");
    assert!(!err.contains("assertion failed"), "{err}");
    let (ok, out, err) = run(
        "off-holds",
        &PICK.replace("AT", "0"),
        "remove-bounds-checks:off",
        "",
    );
    assert!(ok, "{err}");
    assert_eq!(out.trim(), "20");
}

/// D1: `NK1294` outside `unsafe`, and nothing inside it.
#[test]
fn an_assume_outside_unsafe_is_refused() {
    let outside = "fn f(i: i64) -> i64 {\n    assume(i > 0)\n    return i\n}\n\
                   fn main() {\n    println(f\"{f(1)}\")\n}\n";
    assert!(codes(outside).contains(&"NK1294"), "{:?}", codes(outside));
    let inside = outside.replace("assume(i > 0)", "unsafe { assume(i > 0) }");
    assert!(codes(&inside).is_empty(), "{:?}", codes(&inside));
}

/// D3: `std`'s Nikaia assumes nothing (`NK1295`).
#[test]
fn an_assume_in_std_is_refused() {
    let dir = common::scratch_dir("assume-std");
    let path = dir.join("assumes.nika");
    std::fs::write(
        &path,
        "pub fn f(i: i64) -> i64 {\n    unsafe { assume(i > 0) }\n    return i\n}\n",
    )
    .expect("write the module");
    let refused = nikaia::sysroot::lower_std_module(&path);
    std::fs::remove_dir_all(&dir).ok();
    let error = format!(
        "{:#}",
        refused.expect_err("an `assume` in `std` is refused")
    );
    assert!(error.contains("NK1295"), "{error}");
}

/// D3: `--asserts` lists every `assume`, with its line and its claim.
#[test]
fn asserts_lists_every_assume() {
    let dir = common::scratch_dir("assume-report");
    std::fs::write(dir.join("claims.nika"), PICK.replace("AT", "1")).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .args(["lower", "claims.nika", "--output", "claims.rs", "--asserts"])
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("; assumed 1\n"), "{stdout}");
    assert!(
        stdout.contains("  claims.nika:4  assume(i >= 0 && i < xs.len())  assumed: "),
        "{stdout}"
    );
}

/// D1: data from outside the program may be an `assume`'s subject, as it may
/// not be an `assert`'s (`NK1202`).
#[test]
fn an_assume_over_standard_input_is_accepted() {
    let source = "use std::io\n\
                  \n\
                  fn main() throws {\n\
                  \x20   let data = io::read()\n\
                  \x20   let n: i64 = data.len()\n\
                  \x20   unsafe { assume(n > 0) }\n\
                  \x20   println(f\"{data[0]}\")\n\
                  }\n";
    assert!(codes(source).is_empty(), "{:?}", codes(source));
    let (ok, out, err) = run("stdin", source, "remove-bounds-checks:on", "A");
    assert!(ok, "{err}");
    assert_eq!(out.trim(), "65");
}
