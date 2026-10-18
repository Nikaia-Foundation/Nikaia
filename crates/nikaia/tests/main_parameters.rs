//! **`main`'s parameters are the command line** (Part I 5.5,
//! [ADR-360](../../../docs/specification/adr/adr-360.md)): subjects are
//! positional, options named; `main`'s preconditions, carried back as any
//! function's are, are checked where the words arrive, and a wrong word ends
//! the program with status 2 before the body runs.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn codes(source: &str) -> Vec<String> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .map(|f| format!("{} {}", f.code, f.message))
        .collect()
}

/// Lowers `source` through the command line with `flags` and hands back what
/// the compiler printed.
fn lower(dir: &Path, source: &str, flags: &[&str]) -> Output {
    std::fs::write(dir.join("prog.nika"), source).expect("write the source");
    Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(dir)
        .args(["lower", "prog.nika", "--output", "prog.rs"])
        .args(flags)
        .output()
        .expect("the nikaia binary runs")
}

/// The program, built: its binary.
fn built(purpose: &str, source: &str) -> (PathBuf, PathBuf) {
    let dir = common::scratch_dir(&format!("main-parameters-{purpose}"));
    let lowered = lower(&dir, source, &[]);
    assert!(
        lowered.status.success(),
        "{purpose} did not lower:\n{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let binary = dir.join("prog");
    let compiled = common::compile(
        &dir.join("prog.rs"),
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    (dir, binary)
}

/// Status, standard output, standard error.
fn run(binary: &Path, words: &[&str]) -> (i32, String, String) {
    let out = Command::new(binary)
        .args(words)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("run it");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const MULTIPLY: &str = "/// Multiplies a positive number.\n\
fn main(a: i64; times: i64 = 4, be_loud: bool = false) {\n\
\x20   assert(a > 0)\n\
\x20   let b = times * a\n\
\x20   if be_loud {\n\
\x20       println(f\"{times} * {a} = {b}\")\n\
\x20   } else {\n\
\x20       println(f\"{b}\")\n\
\x20   }\n\
}\n";

/// D1, D4, D5: positional and named words, `--`, `-3`, a wrong word with
/// status 2, and the help with status 0.
#[test]
fn the_words_are_the_parameters() {
    let (dir, prog) = built("multiply", MULTIPLY);
    assert_eq!(run(&prog, &["5"]), (0, "20\n".to_string(), String::new()));
    assert_eq!(run(&prog, &["--times=3", "5"]).1, "15\n");
    assert_eq!(run(&prog, &["5", "--times", "3"]).1, "15\n");
    assert_eq!(run(&prog, &["--be-loud", "5"]).1, "4 * 5 = 20\n");
    assert_eq!(run(&prog, &["--be-loud=false", "5"]).1, "20\n");
    assert_eq!(run(&prog, &["--", "5"]).1, "20\n");
    for (words, line) in [
        (&[][..], "error: `a` is missing"),
        (&["x"][..], "error: `a` is an `i64`, and `x` is not one."),
        (&["5", "6"][..], "error: `6` is one word too many."),
        (
            &["-3"][..],
            "error: `a` is -3, which breaks `main`'s precondition `a > 0`.",
        ),
        (
            &["--times=x", "5"][..],
            "error: `--times` is an `i64`, and `x` is not one.",
        ),
        (&["--nope", "5"][..], "error: `--nope` names no option."),
    ] {
        let (status, out, err) = run(&prog, words);
        assert_eq!(status, 2, "{words:?}: {err}");
        assert!(out.is_empty(), "{words:?}: the body ran: {out}");
        assert!(err.starts_with(line), "{words:?}: {err}");
        assert!(
            err.contains("usage: prog [--times <i64>] [--be-loud] <a>"),
            "{words:?}: {err}"
        );
    }
    let (status, help, _) = run(&prog, &["--help"]);
    assert_eq!(status, 0);
    assert!(
        help.starts_with("prog\nMultiplies a positive number.\n"),
        "{help}"
    );
    assert!(
        help.contains("<a>                  i64; requires `a > 0`"),
        "{help}"
    );
    assert!(
        help.contains("--times <i64>        i64, default 4"),
        "{help}"
    );
    assert!(
        help.contains("--be-loud            bool, default false"),
        "{help}"
    );
    assert_eq!(run(&prog, &["-h"]).1, help);
    // After `--`, `--help` is a word like any other.
    assert_eq!(run(&prog, &["--", "--help"]).0, 2);
    std::fs::remove_dir_all(&dir).ok();
}

/// D2: an optional subject, a last `Vec`, `String` and `f64`.
#[test]
fn every_kind_of_word() {
    let (dir, prog) = built(
        "kinds",
        "fn main(name: String, o: i64?, xs: Vec[u8]; scale: f64 = 1.5, label: String? = null) {\n\
         \x20   println(f\"{name} {o ?? 0} {xs.len()} {scale} {label ?? \"-\"}\")\n\
         }\n",
    );
    assert_eq!(run(&prog, &["ab", "3", "1", "2"]).1, "ab 3 2 1.5 -\n");
    assert_eq!(
        run(&prog, &["ab", "3", "--scale", "2", "--label=x"]).1,
        "ab 3 0 2 x\n"
    );
    assert_eq!(run(&prog, &["ab", "3", "256"]).0, 2);
    assert_eq!(run(&prog, &["ab"]).1, "ab 0 0 1.5 -\n");
    std::fs::remove_dir_all(&dir).ok();
}

/// D1: a subject `T?` may be left out at the end and is then `null`; usage
/// and help show it as optional.
#[test]
fn an_optional_subject_may_be_left_out() {
    let (dir, prog) = built(
        "maybe",
        "fn main(a: i64, name: String?) {\n\
         \x20   println(f\"{a} {name ?? \"null\"}\")\n\
         }\n",
    );
    assert_eq!(run(&prog, &["5"]).1, "5 null\n");
    assert_eq!(run(&prog, &["5", "x"]).1, "5 x\n");
    let (status, _, err) = run(&prog, &["5", "x", "y"]);
    assert_eq!(status, 2, "{err}");
    assert!(err.contains("`y` is one word too many"), "{err}");
    assert!(err.contains("usage: prog <a> [<name>]"), "{err}");
    assert_eq!(run(&prog, &[]).0, 2);
    let help = run(&prog, &["--help"]).1;
    assert!(help.contains("[<name>]"), "{help}");
    std::fs::remove_dir_all(&dir).ok();
}

/// D1: a subject `T?` before a last `Vec[T]`: the words fill the `T?` first.
#[test]
fn an_optional_subject_before_the_rest() {
    let (dir, prog) = built(
        "maybe-rest",
        "fn main(a: i64, n: i64?, xs: Vec[String]) {\n\
         \x20   println(f\"{a} {n ?? -1} {xs.len()}\")\n\
         }\n",
    );
    assert_eq!(run(&prog, &["1"]).1, "1 -1 0\n");
    assert_eq!(run(&prog, &["1", "2"]).1, "1 2 0\n");
    assert_eq!(run(&prog, &["1", "2", "p", "q"]).1, "1 2 2\n");
    assert_eq!(run(&prog, &["1", "p"]).0, 2);
    assert!(
        run(&prog, &["--help"])
            .1
            .contains("usage: prog <a> [<n>] [<xs>...]")
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// D3, D4: a precondition carried back from a callee is `main`'s, checked
/// where the words arrive, and its line names the `assert` it came from.
#[test]
fn a_carried_back_precondition_is_checked_where_the_words_arrive() {
    let source = "fn main(a: i64) {\n\
                  \x20   let b = 4 * example(a)\n\
                  \x20   println(f\"{b}\")\n\
                  }\n\
                  fn positive(whole: i64) -> i64 { assert(whole > 0); return whole }\n\
                  fn example(a: i64) -> i64 { let whole = 10 + a; return positive(whole) }\n";
    let (dir, prog) = built("carried", source);
    assert_eq!(run(&prog, &["-9"]).1, "4\n");
    let (status, out, err) = run(&prog, &["-10"]);
    assert_eq!((status, out.as_str()), (2, ""), "{err}");
    assert!(
        err.starts_with(
            "error: `a` is -10, which breaks `main`'s precondition `10 + a > 0`, from \
             `assert(whole > 0)` in `positive`."
        ),
        "{err}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// D3: `--asserts` reports an `assert` about `main`'s parameters as its
/// precondition, checked where the words arrive, and it is a fact after.
#[test]
fn asserts_reports_mains_precondition() {
    let dir = common::scratch_dir("main-parameters-asserts");
    let source = "fn main(a: i64) {\n\
                  \x20   assert(a > 0)\n\
                  \x20   let b = 4 * (10 + a)\n\
                  \x20   assert(b > 40)\n\
                  \x20   println(f\"{b}\")\n\
                  }\n";
    let out = lower(&dir, source, &["--asserts"]);
    let report = String::from_utf8_lossy(&out.stdout);
    assert!(
        report.contains(
            "prog.nika:2  assert(a > 0)  precondition of `main`, checked where the words arrive"
        ),
        "{report}"
    );
    assert!(
        report.contains("prog.nika:4  assert(b > 40)  proved"),
        "{report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// D3, D4 (#651): an `assert` about an option is a precondition of `main` as
/// one about a subject is - checked where the words arrive, in `--asserts`
/// and in the help - and the default the prover proves it for.
#[test]
fn an_options_precondition_is_checked_where_the_words_arrive() {
    let source = "fn main(total: i64; tip: i64 = 0) {\n\
                  \x20   assert(tip <= 30)\n\
                  \x20   println(f\"{total + total * tip / 100}\")\n\
                  }\n";
    let (dir, prog) = built("option-precondition", source);
    assert_eq!(run(&prog, &["100"]).1, "100\n");
    assert_eq!(run(&prog, &["100", "--tip", "10"]).1, "110\n");
    let (status, out, err) = run(&prog, &["100", "--tip", "50"]);
    assert_eq!((status, out.as_str()), (2, ""), "{err}");
    assert!(
        err.starts_with("error: `tip` is 50, which breaks `main`'s precondition `tip <= 30`."),
        "{err}"
    );
    assert!(err.contains("usage: prog [--tip <i64>] <total>"), "{err}");
    let (_, help, _) = run(&prog, &["--help"]);
    assert!(
        help.contains("--tip <i64>          i64, default 0; requires `tip <= 30`"),
        "{help}"
    );
    let out = lower(&dir, source, &["--asserts"]);
    let report = String::from_utf8_lossy(&out.stdout);
    assert!(
        report.contains(
            "prog.nika:2  assert(tip <= 30)  precondition of `main`, checked where the words arrive"
        ),
        "{report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// D2 (#651): a usage error says the type with the article its sound takes.
#[test]
fn a_usage_error_says_a_u8_and_an_i64() {
    let source = "fn main(a: u8; n: i64 = 0) {\n\
                  \x20   println(f\"{a} {n}\")\n\
                  }\n";
    let (dir, prog) = built("article", source);
    assert!(
        run(&prog, &["x"])
            .2
            .starts_with("error: `a` is a `u8`, and `x` is not one.")
    );
    assert!(
        run(&prog, &["1", "--n", "x"])
            .2
            .starts_with("error: `--n` is an `i64`, and `x` is not one.")
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// D2: what no word can be is `NK1273`; an option `help` is `NK1274`.
#[test]
fn a_parameter_no_word_can_be_is_refused() {
    for (source, code, said) in [
        (
            "struct P { x: i64 }\nfn main(p: P) { }",
            "NK1273",
            "it is a `P`",
        ),
        ("fn main(s: ref String) { }", "NK1273", "a `ref` parameter"),
        (
            "fn main(xs: Vec[i64], n: i64) { }",
            "NK1273",
            "not the last subject",
        ),
        (
            "fn main(xs: Vec[Vec[i64]]) { }",
            "NK1273",
            "a `Vec[Vec[i64]]`",
        ),
        ("fn main(m: HashMap[String, i64]) { }", "NK1273", "HashMap"),
        (
            "fn main(a: i64; limit: Vec[i64] = [1]) { }",
            "NK1273",
            "not the last subject",
        ),
        (
            "fn main(a: i64; help: bool = false) { }",
            "NK1274",
            "named `help`",
        ),
        (
            "fn main(a: i64?, b: i64) { }",
            "NK1273",
            "a required word after `a`",
        ),
        // Part I 2.2 (#651): Rust's other widths are no type, on `main` too.
        ("fn main(a: i8) { }", "NK1135", "`i8`"),
        ("fn main(a: i64; n: i16 = 0) { }", "NK1135", "`i16`"),
        ("fn main(a: i64; n: u16 = 0) { }", "NK1135", "`u16`"),
    ] {
        let found = codes(source);
        assert!(
            found
                .iter()
                .any(|f| f.starts_with(code) && f.contains(said)),
            "{source}: {found:#?}"
        );
    }
    for source in [
        "fn main(a: i64, b: u8, c: f64, d: bool, e: String, f: i32?, rest: Vec[String]; max_depth: i64 = 3) { }",
        "fn other(p: ref String) { }",
        "fn main(a: i64?, b: String?, rest: Vec[String]) { }",
        "fn main(a: i64; max_depth: i64 = 3, max_depth_x: i64 = 4) { }",
    ] {
        let found = codes(source);
        assert!(
            !found.iter().any(|f| f.starts_with("NK127")),
            "{source}: {found:#?}"
        );
    }
}
