//! **An `assert` the compiler does not prove refuses the program, unless the
//! build says `warn`** ([ADR-380](../../../docs/specification/adr/adr-380.md)).
//!
//! Through the command line, as a user meets it: `NK1300` and what it says,
//! `--unproved-claims` and `[build] unproved-claims`, the claims it does not
//! ask about, and `nikaia check --asserts`.

mod common;

use std::path::Path;
use std::process::{Command, Output};

fn nikaia(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env_remove("RUST_BACKTRACE")
        .args(args)
        .output()
        .expect("the nikaia binary runs")
}

fn said(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// `nikaia check` of one file, with `args` after it.
fn checked(purpose: &str, source: &str, args: &[&str]) -> Output {
    let dir = common::scratch_dir(&format!("unproved-{purpose}"));
    std::fs::write(dir.join("main.nika"), source).expect("write the source");
    let mut all = vec!["check", "main.nika"];
    all.extend_from_slice(args);
    let out = nikaia(&dir, &all);
    std::fs::remove_dir_all(&dir).ok();
    out
}

/// A claim nothing before it shows: `y` comes out of a loop the claim is not
/// carried through.
const UNPROVED: &str = "fn f(x: i64) -> i64 {\n\
                        \x20   let y = x * 2\n\
                        \x20   for i in 0..<1 {\n\
                        \x20   }\n\
                        \x20   assert(y > 0; message: \"positive\")\n\
                        \x20   return y\n\
                        }\n\
                        \n\
                        fn main() {\n\
                        \x20   println(f\"{f(1)}\")\n\
                        \x20   println(f\"{f(-1)}\")\n\
                        }\n";

/// **D1: refused, and the message says not proved is not false.**
#[test]
fn an_assert_not_proved_is_refused() {
    let out = checked("refused", UNPROVED, &[]);
    let text = said(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("error[NK1300]"), "{text}");
    assert!(
        text.contains(
            "`assert(y > 0; message: \"positive\")` is not proved: nothing before it shows it."
        ),
        "{text}"
    );
    assert!(text.contains("Not proved is not false"), "{text}");
    assert!(text.contains("unproved-claims = \"warn\""), "{text}");
}

/// **D1: a precondition a call does not prove** is told at the claim, with
/// how many calls leave it open.
#[test]
fn a_precondition_a_call_does_not_prove_is_refused() {
    let source = "fn percent(part: i64, whole: i64) -> i64 {\n\
                  \x20   assert(whole > 0)\n\
                  \x20   return part * 100 / whole\n\
                  }\n\
                  \n\
                  fn report(done: i64, total: i64) -> i64 {\n\
                  \x20   let mut sum = 0\n\
                  \x20   for k in 0..<1 {\n\
                  \x20       sum = sum + percent(done, total)\n\
                  \x20   }\n\
                  \x20   return sum\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{report(3, 4)}\")\n\
                  }\n";
    let out = checked("precondition", source, &[]);
    let text = said(&out);
    assert!(!out.status.success(), "{text}");
    assert!(
        text.contains(
            "`assert(whole > 0)`, the precondition of `percent`, is not proved: one call does not prove it."
        ),
        "{text}"
    );
}

/// **D1: what is not asked**: a proved claim, a precondition every call
/// proves, `main`'s precondition, a test's claim and an `assume`.
#[test]
fn proved_claims_main_tests_and_assumes_are_not_asked() {
    let source = "fn half(x: i64) -> i64 {\n\
                  \x20   assert(x >= 0)\n\
                  \x20   return x / 2\n\
                  }\n\
                  \n\
                  fn main(n: i64) {\n\
                  \x20   assert(n > 0)\n\
                  \x20   let v = 7\n\
                  \x20   assert(v > 3)\n\
                  \x20   let w = n * n\n\
                  \x20   unsafe { assume(w >= 0) }\n\
                  \x20   println(f\"{half(v)} {w}\")\n\
                  }\n\
                  \n\
                  test \"a test's claim is its verdict\" {\n\
                  \x20   let k = half(4) * half(4)\n\
                  \x20   assert(k == 4)\n\
                  }\n";
    let out = checked("not-asked", source, &[]);
    let text = said(&out);
    assert!(out.status.success(), "{text}");
    assert!(!text.contains("NK1300"), "{text}");
}

/// **D1: a claim shown false is `NK1207`, not also `NK1300`.**
#[test]
fn a_claim_shown_false_is_not_told_twice() {
    let source = "fn main() {\n\
                  \x20   let v = 2\n\
                  \x20   assert(v > 3)\n\
                  }\n";
    let out = checked("refuted", source, &[]);
    let text = said(&out);
    assert!(text.contains("error[NK1207]"), "{text}");
    assert!(!text.contains("NK1300"), "{text}");
}

/// **D2: `--unproved-claims warn` builds it**, the claim a warning, and the
/// program checks it when it runs: a false one stops it.
#[test]
fn at_warn_the_claim_is_checked_when_the_program_runs() {
    let dir = common::scratch_dir("unproved-warn-run");
    std::fs::write(dir.join("main.nika"), UNPROVED).expect("write the source");
    let lowered = nikaia(
        &dir,
        &[
            "lower",
            "main.nika",
            "--output",
            "main.rs",
            "--unproved-claims",
            "warn",
        ],
    );
    let text = said(&lowered);
    assert!(lowered.status.success(), "{text}");
    assert!(text.contains("warning[NK1300]"), "{text}");
    let binary = dir.join("main");
    let compiled = common::compile(
        &dir.join("main.rs"),
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let ran = Command::new(&binary)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("run it");
    std::fs::remove_dir_all(&dir).ok();
    assert!(!ran.status.success());
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "2\n");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("positive"),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
}

/// **D2: `[build] unproved-claims` in the manifest**, and a word that is
/// neither `error` nor `warn` is refused naming both.
#[test]
fn the_manifest_says_it_and_a_wrong_word_is_refused() {
    let dir = common::scratch_dir("unproved-manifest");
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(dir.join("src/main.nika"), UNPROVED).expect("write the source");
    let manifest = |word: &str| {
        format!(
            "[package]\nname = \"claims\"\nversion = \"0.1.0\"\n\n[build]\nunproved-claims = \"{word}\"\n"
        )
    };
    std::fs::write(dir.join("nikaia.toml"), manifest("warn")).expect("manifest");
    let warned = nikaia(&dir, &["check", "src/main.nika"]);
    assert!(warned.status.success(), "{}", said(&warned));
    assert!(
        said(&warned).contains("warning[NK1300]"),
        "{}",
        said(&warned)
    );

    std::fs::write(dir.join("nikaia.toml"), manifest("maybe")).expect("manifest");
    let wrong = nikaia(&dir, &["check", "src/main.nika"]);
    std::fs::remove_dir_all(&dir).ok();
    assert!(!wrong.status.success());
    assert!(
        said(&wrong).contains("`unproved-claims` is `error` or `warn`, not `maybe`"),
        "{}",
        said(&wrong)
    );
}

/// **`nikaia check --asserts` prints the report**, as `lower --asserts` does.
#[test]
fn check_prints_the_report() {
    let out = checked(
        "report",
        UNPROVED,
        &["--asserts", "--unproved-claims", "warn"],
    );
    let text = said(&out);
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("main.nika:5  assert(y > 0; message: \"positive\")  checked at run time: nothing before it shows it"),
        "{text}"
    );
}

/// **A struct literal of plain fields proves its type's rule** over the
/// values it is built from, not over a field of a new binding.
#[test]
fn a_rule_is_proved_over_a_literal_of_plain_fields() {
    let source = "struct Pair {\n\
                  \x20   xs: Vec[i64],\n\
                  \x20   used: i64,\n\
                  \x20   assert(xs.len() == 2 && used >= 0; message: \"two\")\n\
                  }\n\
                  \n\
                  fn pair() -> Pair {\n\
                  \x20   let xs = [1, 2]\n\
                  \x20   let used = 0\n\
                  \x20   return Pair { xs: xs, used }\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{pair().used}\")\n\
                  }\n";
    let out = checked("literal-rule", source, &[]);
    let text = said(&out);
    assert!(out.status.success(), "{text}");
    assert!(!text.contains("NK1300"), "{text}");
}

/// **D1: where the build refuses it, the report says refused**, not checked
/// at run time.
#[test]
fn the_report_counts_a_refused_claim_as_refused() {
    let out = checked("report-refused", UNPROVED, &["--asserts"]);
    let text = said(&out);
    assert!(
        text.contains("main.nika:5  assert(y > 0; message: \"positive\")  refused, not proved: nothing before it shows it"),
        "{text}"
    );
    assert!(
        text.contains("(refused 1, checked at run time 0,"),
        "{text}"
    );
}
