//! **A function named as a value has its declaration's type** (#502, #507):
//! `double`, a type's name for its anonymous constructor (`P`), and a method
//! (`P::merge`), whose first parameter is the receiver. A wrong use is
//! refused here; a right one still runs.

mod common;

use nikaia::check::Finding;
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;
use std::process::Command;

fn findings(source: &str) -> Vec<Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = common::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's shipped ledger parses");
    common::checked(&parsed, &own, &library).findings
}

const DECLARED: &str = "struct P { x: i64 }\n\
     impl P {\n\
     \x20   fn() -> P { return P { x: 1 } }\n\
     \x20   fn merge(ref mut self, other: P) { self.x = self.x + other.x }\n\
     }\n\
     fn double(n: i64) -> i64 sync { return n * 2 }\n\
     fn apply(f: fn(i64) -> i64 sync, x: i64) -> i64 sync { return f(x) }\n\
     fn make(g: fn() -> P sync) -> P sync { return g() }\n";

#[test]
fn a_function_named_as_a_value_is_typed() {
    for (value, ty) in [
        ("double", "fn(i64) -> i64 sync"),
        ("P", "fn() -> P sync"),
        ("P::merge", "fn(ref P, P) sync"),
    ] {
        let source = format!(
            "{DECLARED}fn main() {{\n    let wrong: bool = {value}\n    println(\"x\")\n}}\n"
        );
        let found = findings(&source);
        assert!(
            found
                .iter()
                .any(|f| f.code == "NK1103" && f.message.contains(&format!("`{ty}`"))),
            "{value}: {found:#?}"
        );
    }
}

/// Passed straight to a parameter that runs it, a declared function is passed
/// as it is - the `&*` a kept closure gets would not compile (#502).
#[test]
fn a_declared_function_passed_as_an_argument_runs() {
    let dir = common::scratch_dir("function-values");
    std::fs::write(
        dir.join("passed.nika"),
        format!(
            "{DECLARED}fn main() {{\n    println(f\"{{apply(double, 5)}} {{make(P).x}}\")\n}}\n"
        ),
    )
    .expect("write the source");
    let ran = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "--no-cache", "passed.nika"])
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "10 1");
}

/// **A method's function parameter it only runs is run, not kept**: a
/// receiver is no argument, so `ask` is the method's second parameter and the
/// trailing lambda a plain closure, not a `Kept` one (0.0.912 read `self` as
/// the first argument).
#[test]
fn a_recursive_method_runs_the_lambda_it_hands_on() {
    let dir = common::scratch_dir("function-values-method");
    std::fs::write(
        dir.join("descent.nika"),
        "struct D { total: i64 }\n\
         impl D {\n\
         \x20   fn down(ref mut self, n: i64, ask: fn(i64) -> bool sync) {\n\
         \x20       if n == 0 { return }\n\
         \x20       if ask(n) { self.total += n }\n\
         \x20       self.down(n - 1, ask)\n\
         \x20   }\n\
         }\n\
         fn main() {\n\
         \x20   let mut walk = D { total: 0 }\n\
         \x20   walk.down(6) fn(k) { k % 2 == 0 }\n\
         \x20   println(f\"{walk.total}\")\n\
         }\n",
    )
    .expect("write the source");
    let ran = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "--no-cache", "descent.nika"])
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "12");
}

/// **A tuple's position it does not have is `NK1107`** (#515), where it
/// reached `rustc` as *no field `2`*; the help names the ones it has.
#[test]
fn a_tuple_position_it_does_not_have_is_refused() {
    let found = findings(
        "fn main() {\n    let t = (\"a\", 2)\n    let u = t.2\n    println(f\"{u}\")\n}\n",
    );
    let refused = found
        .iter()
        .find(|f| f.code == "NK1107")
        .unwrap_or_else(|| panic!("{found:#?}"));
    assert_eq!(refused.message, "This tuple has no part `.2`.");
    assert!(
        refused
            .help
            .as_deref()
            .unwrap_or("")
            .contains("`.0` to `.1`")
    );
}

/// **`.collect()` after a list's `map` is `NK1210`** (#517): the `map` is a
/// list already, and the call reached `rustc`.
#[test]
fn a_list_is_not_collected() {
    let found = findings(
        "fn double(n: i64) -> i64 sync { return n * 2 }\n\
         fn main() {\n    let ys = [1, 2, 3].map(double).collect()\n    println(f\"{ys.len()}\")\n}\n",
    );
    let refused = found
        .iter()
        .find(|f| f.code == "NK1210")
        .unwrap_or_else(|| panic!("{found:#?}"));
    assert_eq!(refused.help.as_deref(), Some("Leave out `.collect()`."));
}

/// **A parameter that may pause runs a declared function either way** (#516):
/// one that pauses (`slow`) and one that never does (`double`), which the
/// lowering hands over inside an `async` closure.
#[test]
fn a_parameter_that_may_pause_runs_either_kind_of_function() {
    let dir = common::scratch_dir("function-values-pause");
    std::fs::write(
        dir.join("either.nika"),
        "use std::time\n\
         \n\
         fn slow(n: i64) -> i64 {\n    time::sleep(1.millis())\n    return n + 1\n}\n\
         fn double(n: i64) -> i64 { return n * 2 }\n\
         fn apply(f: fn(i64) -> i64, x: i64) -> i64 { return f(x) }\n\
         fn main() {\n    println(f\"{apply(slow, 5)} {apply(double, 5)}\")\n}\n",
    )
    .expect("write the source");
    let ran = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "--no-cache", "either.nika"])
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "6 10");
}

/// **A named function handed to a parameter the callee keeps** is the closure
/// that calls it, as a lambda handed there is (#601): one the kept type says
/// may pause (`double`, `slow`), one it says never does (`keep_quick`), and a
/// named function beside a lambda in one call.
#[test]
fn a_named_function_handed_to_a_kept_parameter_runs() {
    let dir = common::scratch_dir("function-values-kept");
    std::fs::write(
        dir.join("kept.nika"),
        "use std::time\n\
         \n\
         struct One {\n    f: fn(i64) -> i64,\n}\n\
         struct Quick {\n    f: fn(i64) -> i64 sync,\n}\n\
         struct Two {\n    f: fn(i64) -> i64,\n    g: fn(i64) -> i64,\n}\n\
         fn keep(f: fn(i64) -> i64) -> One {\n    return One { f: f }\n}\n\
         fn keep_quick(f: fn(i64) -> i64 sync) -> Quick {\n    return Quick { f: f }\n}\n\
         fn keep_two(f: fn(i64) -> i64, g: fn(i64) -> i64) -> Two {\n    return Two { f: f, g: g }\n}\n\
         fn double(x: i64) -> i64 {\n    return x * 2\n}\n\
         fn slow(n: i64) -> i64 {\n    time::sleep(1.millis())\n    return n + 1\n}\n\
         fn main() {\n\
         \x20   let o = keep(double)\n    let g = o.f\n\
         \x20   let s = keep(slow)\n    let h = s.f\n\
         \x20   let q = keep_quick(double)\n    let k = q.f\n\
         \x20   let t = keep_two(double, fn(x) { x + 1 })\n    let a = t.f\n    let b = t.g\n\
         \x20   println(f\"{g(4)} {h(4)} {k(4)} {a(4)} {b(4)}\")\n\
         }\n",
    )
    .expect("write the source");
    let ran = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "--no-cache", "kept.nika"])
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "8 5 8 8 5");
}

/// **A method named by its path where a function value is kept** is the
/// closure that calls it, as a named function is (#601): handed to a kept
/// parameter (`keep(P::add)`) and put in a struct literal's field (`One { f:
/// P::add }`), one that takes its `self` by value and pauses (`P::slowly`),
/// one with no `self` that pauses (`P::scaled`), and a method beside a lambda
/// in one call.
#[test]
fn a_method_named_where_a_function_value_is_kept_runs() {
    let dir = common::scratch_dir("function-values-kept-method");
    std::fs::write(
        dir.join("kept.nika"),
        "use std::time\n\
         \n\
         struct P {\n\
         \x20   base: i64,\n\
         }\n\
         impl P {\n\
         \x20   fn add(ref self, n: i64) -> i64 {\n\
         \x20       return self.base + n\n\
         \x20   }\n\
         \x20   fn slowly(self, n: i64) -> i64 {\n\
         \x20       time::sleep(1.millis())\n\
         \x20       return self.base * n\n\
         \x20   }\n\
         \x20   fn scaled(n: i64) -> i64 {\n\
         \x20       time::sleep(1.millis())\n\
         \x20       return n * 3\n\
         \x20   }\n\
         }\n\
         struct One {\n\
         \x20   f: fn(ref P, i64) -> i64 sync,\n\
         }\n\
         struct Own {\n\
         \x20   f: fn(P, i64) -> i64,\n\
         }\n\
         struct Slow {\n\
         \x20   f: fn(i64) -> i64,\n\
         }\n\
         struct Two {\n\
         \x20   f: fn(ref P, i64) -> i64 sync,\n\
         \x20   g: fn(ref P, i64) -> i64 sync,\n\
         }\n\
         fn keep(f: fn(ref P, i64) -> i64 sync) -> One {\n\
         \x20   return One { f: f }\n\
         }\n\
         fn keep_own(f: fn(P, i64) -> i64) -> Own {\n\
         \x20   return Own { f: f }\n\
         }\n\
         fn keep_slow(f: fn(i64) -> i64) -> Slow {\n\
         \x20   return Slow { f: f }\n\
         }\n\
         fn keep_two(f: fn(ref P, i64) -> i64 sync, g: fn(ref P, i64) -> i64 sync) -> Two {\n\
         \x20   return Two { f: f, g: g }\n\
         }\n\
         fn main() {\n\
         \x20   let p = P { base: 10 }\n\
         \x20   let o = keep(P::add)\n\
         \x20   let a = o.f\n\
         \x20   let d = One { f: P::add }\n\
         \x20   let b = d.f\n\
         \x20   let w = keep_own(P::slowly)\n\
         \x20   let c = w.f\n\
         \x20   let s = keep_slow(P::scaled)\n\
         \x20   let h = s.f\n\
         \x20   let e = Slow { f: P::scaled }\n\
         \x20   let k = e.f\n\
         \x20   let t = keep_two(P::add, fn(q, n) { q.base - n })\n\
         \x20   let x = t.f\n\
         \x20   let y = t.g\n\
         \x20   println(f\"{a(p, 4)} {b(p, 5)} {c(P { base: 10 }, 3)} {h(3)} {k(2)} {x(p, 1)} {y(p, 1)}\")\n\
         }\n",
    )
    .expect("write the source");
    let ran = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "--no-cache", "kept.nika"])
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout).trim(),
        "14 15 30 9 6 11 9"
    );
}
