//! **An element or a field read off a place is a view where a `let` declares
//! `ref T`** ([ADR-353](../../../docs/specification/adr/adr-354.md), #613).

use crate::common;
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

/// D2 at every position: `ref` on a word is `NK1257` at a parameter, a field,
/// a result and a trait method's parameter alike (#628).
#[test]
fn a_word_declared_ref_is_refused_at_every_position() {
    for (program, position) in [
        (
            "fn f(x: ref i32) -> i32 { return x + 1 }\n\nfn main() { println(f\"{f(2)}\") }\n",
            "parameter",
        ),
        (
            "struct S { n: ref i64 }\n\nfn main() { println(\"x\") }\n",
            "field",
        ),
        (
            "fn f(xs: ref Vec[bool]) -> ref bool { return xs[0] }\n\nfn main() { println(\"x\") }\n",
            "result",
        ),
        (
            "trait T { fn f(ref self, x: ref f64) -> f64 }\n\nfn main() { println(\"x\") }\n",
            "parameter",
        ),
        (
            "trait T { fn f(ref self) -> ref u8 }\n\nfn main() { println(\"x\") }\n",
            "result",
        ),
        (
            "enum Colour { Red, Green }\n\nfn f(c: ref Colour) -> i64 { return 1 }\n\nfn main() { println(\"x\") }\n",
            "parameter",
        ),
    ] {
        let found = findings(program);
        let f = found
            .iter()
            .find(|f| f.code == "NK1257")
            .unwrap_or_else(|| panic!("{program}\n{found:#?}"));
        assert!(f.message.contains(position), "{}", f.message);
        assert!(
            f.help.as_deref().unwrap_or("").contains("without `ref`"),
            "{f:#?}"
        );
    }
}

/// D2 names a word by its kind, so `int` - the integer most programs write -
/// is one too, at a parameter, a field and a result (#654).
#[test]
fn ref_int_is_refused_as_a_word() {
    for (program, position) in [
        (
            "fn f(x: ref int) -> int { return x + 1 }\n\nfn main() { println(f\"{f(2)}\") }\n",
            "parameter",
        ),
        (
            "struct S { n: ref int }\n\nfn main() { println(\"x\") }\n",
            "field",
        ),
        (
            "fn f(xs: ref Vec[int]) -> ref int { return xs[0] }\n\nfn main() { println(\"x\") }\n",
            "result",
        ),
    ] {
        let found = findings(program);
        let f = found
            .iter()
            .find(|f| f.code == "NK1257")
            .unwrap_or_else(|| panic!("{program}\n{found:#?}"));
        assert!(f.message.contains(position), "{}", f.message);
        assert!(f.message.contains("`int`"), "{}", f.message);
    }
}

/// D2 inside a type (#703): a type argument, a tuple's part and a code type's
/// parameter or result are positions of the declaration, at a `let` too. A walk
/// over numbers yields the numbers (ADR-231 D1), so `Vec[ref i64]` collected
/// from one was a list of views `rustc` refused.
#[test]
fn a_word_declared_ref_inside_a_type_is_refused() {
    for program in [
        "fn main() {\n\
         \x20   let xs = [1, 2, 3]\n\
         \x20   let back: Vec[ref i64] = xs.iter().filter(fn(x) { x > 1 }).collect()\n\
         \x20   println(f\"{back.len()}\")\n\
         }\n",
        "fn f(v: Vec[ref i64]) -> i64 { return 1 }\n\nfn main() { println(\"x\") }\n",
        "struct S { xs: Vec[ref bool] }\n\nfn main() { println(\"x\") }\n",
        "fn f() -> (String, ref u8)? { return null }\n\nfn main() { println(\"x\") }\n",
        "fn f(g: fn(ref i64) -> i64 sync) -> i64 { return 1 }\n\nfn main() { println(\"x\") }\n",
        "fn f(g: fn(i64) -> ref i64 sync) -> i64 { return 1 }\n\nfn main() { println(\"x\") }\n",
    ] {
        let found = findings(program);
        let f = found
            .iter()
            .find(|f| f.code == "NK1257")
            .unwrap_or_else(|| panic!("{program}\n{found:#?}"));
        assert!(
            f.help.as_deref().unwrap_or("").contains("without `ref`"),
            "{f:#?}"
        );
    }
    // Without `ref` the same `let` is the program.
    let found = findings(
        "fn main() {\n\
         \x20   let xs = [1, 2, 3]\n\
         \x20   let back: Vec[i64] = xs.iter().filter(fn(x) { x > 1 }).collect()\n\
         \x20   println(f\"{back.len()}\")\n\
         }\n",
    );
    assert!(found.is_empty(), "{found:#?}");
}

/// D2's exception: at the C boundary `ref i32` and `ref mut i32` are the
/// pointers C expects, and stay.
#[test]
fn a_word_declared_ref_at_the_c_boundary_stays() {
    let found = findings(
        "extern \"C\" {\n\
         \x20   fn takes_one(n: ref i32) -> i32\n\
         \x20   fn fills_one(n: ref mut i32) -> i32\n\
         }\n\n\
         fn main() { println(\"x\") }\n",
    );
    assert!(!found.iter().any(|f| f.code == "NK1257"), "{found:#?}");
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
