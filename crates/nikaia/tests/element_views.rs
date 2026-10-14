//! **An element or a field read off a place is a view where a `let` declares
//! `ref T`** ([ADR-353](../../../docs/specification/adr/adr-354.md), #613).

mod common;

use nikaia::contracts::LedgerOps;
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = nikaia::contracts::Ledger::infer(&parsed);
    let library =
        nikaia::contracts::Ledger::parse(nikaia::contracts::STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

fn run(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let ran = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    String::from_utf8_lossy(&ran.stdout).into_owned()
}

/// D1: the `let` and the arm, both `NK1103` before, compile and run.
#[test]
fn an_element_under_ref_t_and_ref_t_nullable_runs() {
    let out = run(
        "element-view",
        "struct Expr { n: i64, s: String }\n\n\
         struct Pair { a: Expr, b: Expr }\n\n\
         fn pick(given: ref Vec[Expr], tail: ref Expr, c: bool) -> i64 {\n\
         \x20   let y: ref Expr = given[0]\n\
         \x20   let x: ref Expr? = if c { tail } else { given[0] }\n\
         \x20   return y.n + (x?.n ?? 0)\n\
         }\n\n\
         fn field(p: ref Pair) -> i64 {\n\
         \x20   let b: ref Expr = p.b\n\
         \x20   return b.n\n\
         }\n\n\
         fn main() {\n\
         \x20   let v = [Expr { n: 1, s: \"a\" }]\n\
         \x20   let t = Expr { n: 2, s: \"b\" }\n\
         \x20   let p = Pair { a: Expr { n: 5, s: \"c\" }, b: Expr { n: 7, s: \"d\" } }\n\
         \x20   println(f\"{pick(v, t, true)} {pick(v, t, false)} {field(p)}\")\n\
         }\n",
    );
    assert_eq!(out, "3 2 7\n");
}

/// D2: a word written as `ref i64` is `NK1257`, with the `let` to write.
#[test]
fn a_word_declared_ref_is_refused() {
    let found = findings(
        "fn first(given: ref Vec[i64]) -> i64 {\n\
         \x20   let y: ref i64 = given[0]\n\
         \x20   return y\n\
         }\n\n\
         fn main() { println(f\"{first([1])}\") }\n",
    );
    let f = found
        .iter()
        .find(|f| f.code == "NK1257")
        .unwrap_or_else(|| panic!("{found:#?}"));
    assert_eq!(
        f.help.as_deref(),
        Some("an `i64` is copied at a read; write `let y = given[0]`.")
    );
    assert!(!found.iter().any(|f| f.code == "NK1103"), "{found:#?}");
}

/// D3: a generic body's `ref T` is a view, and instantiated at `i64` runs.
#[test]
fn a_generic_ref_t_at_a_word_runs() {
    let out = run(
        "element-view-generic",
        "fn first[T](given: ref Vec[T]) -> bool {\n\
         \x20   let y: ref T = given[0]\n\
         \x20   return given.len() > 0\n\
         }\n\n\
         fn main() {\n\
         \x20   let v: Vec[i64] = [4, 5]\n\
         \x20   println(f\"{first(v)}\")\n\
         }\n",
    );
    assert_eq!(out, "true\n");
}

/// ADR-279 D11: the view kept past the loan is refused as any view is.
#[test]
fn an_element_view_returned_past_its_list_is_refused() {
    let dir = common::scratch_dir("element-view-kept");
    std::fs::write(
        dir.join("main.nika"),
        "struct Expr { n: i64 }\n\n\
         fn main() {\n\
         \x20   let mut out: ref Expr? = null\n\
         \x20   {\n\
         \x20       let given = [Expr { n: 1 }]\n\
         \x20       let y: ref Expr = given[0]\n\
         \x20       out = y\n\
         \x20   }\n\
         \x20   println(f\"{out?.n ?? 0}\")\n\
         }\n",
    )
    .expect("the source");
    let ran = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!ran.status.success());
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("`given` does not live long enough"),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
}
