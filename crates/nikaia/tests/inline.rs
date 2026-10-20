//! **`@inline`, `@inline(always)`, `@inline(never)`**
//! ([ADR-370](../../../docs/specification/adr/adr-370.md), Part I 5.6, #715):
//! before a function or a method, lowered to Rust's attribute of the same
//! name; a written one wins over the hint the compiler adds itself (D2).

mod common;

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn codes(source: &str) -> Vec<String> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = common::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    common::checked(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .map(|f| f.code.to_string())
        .collect()
}

fn lowered(source: &str) -> String {
    assert_eq!(codes(source), Vec::<String>::new(), "{source}");
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Build::default())
        .expect("the source lowers")
        .rust
}

/// The first line after each hint that is no attribute, trimmed: what the
/// hint stands before.
fn hinted<'a>(rust: &'a str, hint: &str) -> Vec<&'a str> {
    let lines: Vec<&str> = rust.lines().map(str::trim).collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| **line == hint)
        .filter_map(|(at, _)| lines[at + 1..].iter().find(|l| !l.starts_with("#[")))
        .copied()
        .collect()
}

const PROGRAM: &str = "@inline\n\
     pub fn digit(b: i64) -> i64 {\n\
     \x20   return b - 48\n\
     }\n\
     \n\
     @inline(always)\n\
     fn twice(b: i64) -> i64 {\n\
     \x20   return b * 2\n\
     }\n\
     \n\
     /// A cold path.\n\
     @inline(never)\n\
     fn fail(n: i64) -> i64 {\n\
     \x20   return 0 - n\n\
     }\n\
     \n\
     fn countdown(n: i64) -> i64 {\n\
     \x20   if n <= 0 {\n\
     \x20       return 0\n\
     \x20   }\n\
     \x20   return countdown(n - 1) + 1\n\
     }\n\
     \n\
     struct Point {\n\
     \x20   x: i64,\n\
     }\n\
     \n\
     impl Point {\n\
     \x20   @inline(always)\n\
     \x20   fn shifted(self, by: i64) -> i64 {\n\
     \x20       return self.x + by\n\
     \x20   }\n\
     }\n\
     \n\
     fn main() {\n\
     \x20   let p = Point { x: 3 }\n\
     \x20   println(f\"{digit(50)} {twice(3)} {fail(4)} {countdown(3)} {p.shifted(1)}\")\n\
     }\n";

/// D1: each form is Rust's attribute of the same name, on the function or
/// the method it stands before, and a function without one has none.
#[test]
fn each_form_lowers_to_rusts_attribute() {
    let rust = lowered(PROGRAM);
    let plain = hinted(&rust, "#[inline]");
    assert_eq!(plain.len(), 1, "{rust}");
    assert!(plain[0].starts_with("pub fn digit("), "{rust}");
    let always = hinted(&rust, "#[inline(always)]");
    assert_eq!(always.len(), 2, "{rust}");
    assert!(always[0].starts_with("fn twice("), "{rust}");
    assert!(always[1].starts_with("fn shifted("), "{rust}");
    let never = hinted(&rust, "#[inline(never)]");
    assert_eq!(never.len(), 1, "{rust}");
    assert!(never[0].starts_with("fn fail("), "{rust}");
}

/// None of them changes what a program means: the lowered program builds
/// and prints what it would without them.
#[test]
fn the_hints_change_nothing_the_program_prints() {
    let rust = lowered(PROGRAM);
    let dir = common::scratch_dir("inline-runs");
    let file = dir.join("main.rs");
    std::fs::write(&file, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let built = common::compile(&file, &["-o", &binary.to_string_lossy()]);
    assert!(
        built.status.success(),
        "{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&built.stderr)
    );
    let out = std::process::Command::new(&binary)
        .output()
        .expect("run it");
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "2 6 -4 3 4\n");
}

/// A function with a precondition has two entries (ADR-269 D20): both carry
/// what was written, so a caller that reaches either is inlined.
#[test]
fn both_entries_of_a_checked_function_carry_the_hint() {
    let rust = lowered(
        "@inline(always)\n\
         pub fn half(n: i64) -> i64 {\n\
         \x20   assert(n >= 0)\n\
         \x20   return n / 2\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let g = half\n\
         \x20   println(f\"{half(4)} {g(6)}\")\n\
         }\n",
    );
    let always = hinted(&rust, "#[inline(always)]");
    assert_eq!(always.len(), 2, "{rust}");
    assert!(
        always
            .iter()
            .any(|l| l.starts_with("pub fn half__unchecked(")),
        "{rust}"
    );
    assert!(
        always.iter().any(|l| l.starts_with("pub fn half(")),
        "{rust}"
    );
}

/// D2: a written `@inline(never)` wins over the hint a fold's step's callee
/// gains (ADR-296 D41).
#[test]
fn a_written_never_wins_over_the_fold_hint() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/1brc.nika"
    ))
    .expect("read examples/1brc.nika");
    let fn_record = source
        .lines()
        .find(|l| l.trim_start().starts_with("fn record("))
        .expect("1brc has a record method");
    let indent = &fn_record[..fn_record.len() - fn_record.trim_start().len()];
    let never = source.replacen(
        fn_record,
        &format!("{indent}@inline(never)\n{fn_record}"),
        1,
    );
    let rust = lowered(&never);
    assert!(hinted(&rust, "#[inline]").is_empty(), "{rust}");
    let kept = hinted(&rust, "#[inline(never)]");
    assert_eq!(kept.len(), 1, "{rust}");
    assert!(kept[0].starts_with("fn record("), "{rust}");
}

/// `NK1235`: before anything but a function or a method with a body - a
/// struct, a field, a trait's method, a test.
#[test]
fn inline_out_of_place_is_refused() {
    let main = "\nfn main() {\n}\n";
    for source in [
        "@inline\nstruct P {\n    x: i64,\n}\n",
        "struct P {\n    @inline\n    x: i64,\n}\n",
        "trait Shape {\n    @inline\n    fn area(self) -> i64\n}\n",
        "@inline\ntest \"nothing\" {\n}\n",
    ] {
        assert_eq!(
            codes(&format!("{source}{main}")),
            vec!["NK1235"],
            "{source}"
        );
    }
}

/// `NK1236` for a second one; `NK1293` for a word that is not `always` or
/// `never`, or more than one.
#[test]
fn a_second_inline_and_an_unknown_word_are_refused() {
    let with = |attributes: &str| {
        format!("{attributes}\nfn f() -> i64 {{\n    return 1\n}}\n\nfn main() {{\n}}\n")
    };
    assert_eq!(codes(&with("@inline\n@inline(never)")), vec!["NK1236"]);
    assert_eq!(codes(&with("@inline(sometimes)")), vec!["NK1293"]);
    assert_eq!(codes(&with("@inline(\"always\")")), vec!["NK1293"]);
    assert_eq!(codes(&with("@inline(always, never)")), vec!["NK1293"]);
    assert_eq!(codes(&with("@inline(; always: true)")), vec!["NK1293"]);
}
