//! Defects a move of the toolchain into Nikaia found (ADR-294 D3, #125).
//!
//! Each program here was accepted by the checker and lowered to Rust that
//! `rustc` refused, so the test is the whole path: lower, compile, run, and
//! compare what it printed.
//!
//! Only the tests that also assert on the lowered Rust stay here; the programs
//! whose output or refusal is all they check are `tests/compiler/tests/*.nika`
//! (#821, `scripts/move_tests.nika`).

use crate::common;
use std::process::Command;

/// What the program prints, and the Rust it was lowered to.
fn printed_and_lowered(purpose: &str, source: &str) -> (String, String) {
    let dir = common::scratch_dir(&format!("selfhosting-{purpose}"));
    std::fs::write(dir.join("p.nika"), source).expect("write the source");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "p.nika", "--output", "p.rs"])
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "{purpose} did not lower:\n{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let binary = dir.join("p");
    let compiled = common::compile(
        &dir.join("p.rs"),
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary).output().expect("run it");
    assert!(out.status.success(), "{purpose} did not run");
    let rust = std::fs::read_to_string(dir.join("p.rs")).expect("read the lowering");
    std::fs::remove_dir_all(&dir).ok();
    (String::from_utf8_lossy(&out.stdout).into_owned(), rust)
}

/// #560: `Wrapped(x, at)` is the constructor of a struct another file of the
/// package declares, as it is of one this file declares.
#[test]
fn a_constructor_call_of_a_struct_in_another_file_is_the_new_it_has() {
    let dir = common::scratch_dir("selfhosting-constructor-beside");
    std::fs::write(
        dir.join("a.nika"),
        "pub struct Wrapped[T] {\n    pub node: T,\n    pub at: i64,\n}\n\n\
         impl Wrapped[T] {\n    pub fn new(node: T, at: i64) -> Wrapped[T] {\n        \
         return Wrapped { node: node, at: at }\n    }\n}\n",
    )
    .expect("write a");
    std::fs::write(
        dir.join("b.nika"),
        "pub fn wrap(name: String, at: i64) -> Wrapped[String] {\n    return Wrapped(name, at)\n}\n\n\
         pub fn wrap_written(name: String, at: i64) -> Wrapped[String] {\n    \
         return Wrapped { node: name, at: at }\n}\n",
    )
    .expect("write b");
    let lowered = nikaia::sysroot::lower_tools(&dir).expect("the package lowers");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        lowered.contains("Wrapped::new(name, at)"),
        "the call spelling reaches the `new`:\n{lowered}"
    );
    assert!(
        lowered.contains("Wrapped { node: name, at }"),
        "and the struct spelling stays a struct:\n{lowered}"
    );
}

/// #558: `kept_calls` is keyed by the expression (ADR-340 D5), so two in one
/// statement get their own answers: `f(5) + g(5)` where `f` may pause and `g` is `sync`: only `f(5)` is awaited.
#[test]
fn two_kept_calls_in_a_statement_pause_each_on_its_own() {
    let (said, rust) = printed_and_lowered(
        "kept-calls-by-callee",
        r#"
use std::time

fn slow(n: i64) -> i64 {
    time::sleep(1.millis())
    return n + 1
}

fn double(n: i64) -> i64 sync { return n * 2 }

fn main() {
    let f: fn(i64) -> i64 = slow
    let g: fn(i64) -> i64 sync = double
    println(f"{f(5) + g(5)}")
}
"#,
    );
    assert_eq!(said.trim(), "16");
    assert!(rust.contains("f(5).await + g(5)"), "{rust}");
}

/// #558: `kept_functions` is keyed by the expression (ADR-340 D5), so two in one
/// statement get their own answers: `Pair { f: slow, g: double }`: `slow` is kept as a boxed future, `double` as a plain closure.
#[test]
fn two_kept_functions_in_a_statement_are_wrapped_each_on_its_own() {
    let (said, rust) = printed_and_lowered(
        "kept-functions-by-name",
        r#"
use std::time

struct Pair {
    f: fn(i64) -> i64,
    g: fn(i64) -> i64 sync,
}

fn slow(n: i64) -> i64 {
    time::sleep(1.millis())
    return n + 1
}

fn double(n: i64) -> i64 sync { return n * 2 }

fn main() {
    let p = Pair { f: slow, g: double }
    let f = p.f
    let g = p.g
    println(f"{f(5)} {g(5)}")
}
"#,
    );
    assert_eq!(said.trim(), "6 10");
    assert!(
        rust.contains("Box::pin(async move { slow(") && rust.contains("{ double(__nikaia_arg0) }"),
        "{rust}"
    );
}

/// #558: `future_lambdas` is keyed by the expression (ADR-340 D5), so two in one
/// statement get their own answers: `both(fn .., fn ..)`: the lambda for a parameter that may pause is `async`, the one for a `sync` parameter is not.
#[test]
fn two_lambdas_in_a_statement_are_futures_each_on_its_own() {
    let (said, rust) = printed_and_lowered(
        "future-lambdas-by-node",
        r#"
use std::time

fn both(f: fn(i64) -> i64, g: fn(i64) -> i64 sync) -> i64 {
    return f(1) + g(2)
}

fn main() {
    let r = both(fn(x) {
        time::sleep(1.millis())
        return x + 10
    }, fn(x) { return x * 3 })
    println(f"{r}")
}
"#,
    );
    assert_eq!(said.trim(), "17");
    assert!(
        rust.contains("both(&async |x|") && rust.contains("&|x| { x * 3 }"),
        "{rust}"
    );
}

/// #558: `run_lambdas` is keyed by the expression (ADR-340 D5), so two in one
/// statement get their own answers: `both(fn .., fn ..)` where `both` runs `f` and keeps `g`: `f` is an `async` closure, `g` a kept boxed future.
#[test]
fn a_run_and_a_kept_lambda_in_a_statement_differ() {
    let (said, rust) = printed_and_lowered(
        "run-lambdas-by-node",
        r#"
use std::time

struct Holder {
    g: fn(i64) -> i64,
}

fn both(f: fn(i64) -> i64, g: fn(i64) -> i64) -> Holder {
    println(f"{f(1)}")
    return Holder { g: g }
}

fn main() {
    let h = both(fn(x) {
        time::sleep(1.millis())
        return x + 10
    }, fn(x) {
        time::sleep(1.millis())
        return x * 3
    })
    let k = h.g
    println(f"{k(2)}")
}
"#,
    );
    assert_eq!(said.trim(), "11\n6");
    assert!(
        rust.contains("both(&async |x|")
            && rust.contains("Kept(std::sync::Arc::new(move |x| Box::pin"),
        "{rust}"
    );
}

/// #558: `pausing_methods` is keyed by the expression (ADR-340 D5), so two in one
/// statement get their own answers: `a.map(slow..)[2] * 100 + b.map(plain..)[2]`: only `a.map` is the pausing `map_list`.
#[test]
fn two_maps_in_a_statement_pause_each_on_its_own() {
    let (said, rust) = printed_and_lowered(
        "pausing-methods-by-receiver",
        r#"
use std::time

fn slow(n: i64) -> i64 {
    time::sleep(1.millis())
    return n + 1
}

fn main() {
    let a = [1, 2, 3]
    let b = [4, 5, 6]
    let total = a.map(fn(x) { return slow(x) })[2] * 100 + b.map(fn(x) { return x * 2 })[2]
    println(f"{total}")
}
"#,
    );
    assert_eq!(said.trim(), "412");
    assert!(
        rust.contains("map_list(a,") && rust.contains("b.map(|x|"),
        "{rust}"
    );
}

/// #558: `unrolled_calls` is keyed by the expression (ADR-340 D5), so two in one
/// statement get their own answers: `size(User ..) * 10 + size(Point ..)`: each call names its own copy.
#[test]
fn two_shape_walks_in_a_statement_unroll_each_on_its_own() {
    let (said, rust) = printed_and_lowered(
        "unrolled-calls-by-call",
        r#"
struct User { name: String, age: i64 }
struct Point { x: i64, y: i64, z: i64 }

fn size[T: Struct](value: T) -> i64 {
    let mut n = 0
    for field in T::fields { n = n + 1 }
    return n
}

fn main() {
    let r = size(User { name: "ada".clone(), age: 36 }) * 10 + size(Point { x: 1, y: 2, z: 3 })
    println(f"{r}")
}
"#,
    );
    assert_eq!(said.trim(), "23");
    assert!(
        rust.contains("size__User(User") && rust.contains("size__Point(Point"),
        "{rust}"
    );
}

/// #558: `task_handles` is keyed by the expression (ADR-340 D5), so two in one
/// statement get their own answers: two `spawn`s in one list: each task duplicates only the handle it reads.
#[test]
fn two_tasks_in_a_statement_take_each_its_own_handles() {
    let (said, rust) = printed_and_lowered(
        "task-handles-by-task",
        r#"
fn main() {
    let db = Shared(1)
    let other = Shared(2)
    let _ts = [spawn fn { println(f"{db}") }, spawn fn { println(f"{other}") }]
    println(f"{db} {other}")
}
"#,
    );
    let mut lines: Vec<&str> = said.lines().collect();
    lines.sort();
    assert_eq!(lines, ["1", "1 2", "2"]);
    assert!(rust.contains("{ let db = db.clone(); nikaia_std::task::TaskHandle::start(async move { println!(\"{}\", db) }) }"), "{rust}");
}

/// **`throw made()` throws the type `made` returns**, as `throw Bad::No` does:
/// the error set names `Bad`, so the function lowers to `Thrown<Bad>` and not
/// to `Box<dyn Error>`, which a caller that `split()`s it could not take
/// (rustc E0599).
#[test]
fn a_throw_of_a_call_names_the_type_the_callee_returns() {
    let (said, rust) = printed_and_lowered(
        "throw-of-a-call",
        r#"
enum Bad { No }
impl Error for Bad {
    fn message(ref self) -> String {
        return "bad"
    }
}
fn made() -> Bad {
    return Bad::No
}
fn f(fail: bool) -> i64 throws {
    if fail {
        throw made()
    }
    return 1
}
fn main() {
    let a = f(false) catch { 7 }
    let b = f(true) catch { 7 }
    println(f"{a} {b}")
}
"#,
    );
    assert_eq!(said.trim(), "1 7");
    assert!(rust.contains("nikaia_std::error::Thrown<Bad>"), "{rust}");
}

/// #809: `v[i]?.clone()` on a list of options matched on `*index::get(..)`,
/// which moves the element out of the list (E0507). The match is on the
/// reference.
#[test]
fn a_reach_off_a_list_element_matches_on_the_reference() {
    let (said, lowered) = printed_and_lowered(
        "reach-off-a-list-element",
        r#"
fn first(v: ref Vec[String?]) -> String? {
    return v[0]?.clone()
}
fn main() {
    let v: Vec[String?] = ["a", null]
    println(first(v) ?? "none")
    let w = v[1]?.clone()
    println(w ?? "none")
    println(f"{v[0]?.len() ?? 0}")
}
"#,
    );
    assert_eq!(said, "a\nnone\n1\n");
    assert!(
        !lowered.contains("match *nikaia_std::index::get"),
        "{lowered}"
    );
}

/// #814: an element of a `mut` list handed to a `mut` parameter is lent
/// mutably (`&mut args[i]`), not read through `index::get`.
#[test]
fn an_element_handed_to_a_mut_parameter_is_changed_in_place() {
    let (said, lowered) = printed_and_lowered(
        "element-to-a-mut-parameter",
        r#"
struct Expr {
    n: i64,
}
struct Block {
    args: Vec[Expr],
}
fn bump(mut e: Expr) {
    e.n += 1
}
fn main() {
    let mut args: Vec[Expr] = [Expr { n: 1 }]
    let i = 0
    bump(args[i])
    print(args[0].n)
    let mut b = Block { args: [Expr { n: 5 }] }
    bump(b.args[i])
    println(f" {b.args[0].n}")
}
"#,
    );
    assert_eq!(said, "2 6\n");
    assert!(
        !lowered.contains("&mut *nikaia_std::index::get"),
        "{lowered}"
    );
}

/// #827: a list element past a jump is a view of it in a `let` typed
/// `ref T`, in a chain, and as an argument to a parameter that only reads it
/// (ADR-282 D7): `index::get(..).as_ref()`, never `*index::get(..)`, which
/// moved the option out of the list (E0507). A type that copies is copied.
#[test]
fn a_list_element_past_a_jump_is_a_view_in_a_typed_let_and_an_argument() {
    let (said, lowered) = printed_and_lowered(
        "list-element-view-typed-and-argument",
        r#"
struct Entry { n: i64, s: String }
struct Pt { x: i64 }
fn by_ref(e: ref Entry) -> i64 { e.n }
fn by_val(e: Entry) -> i64 { e.n }
fn text_len(s: String) -> i64 { s.len() }
fn typed_ref(v: ref Vec[Entry?]) -> i64 {
    let e: ref Entry = v[0] ?? return 0
    e.n
}
fn chain_ref(a: ref Vec[Entry?], b: ref Vec[Entry?]) -> i64 {
    let e: ref Entry = a[0] ?? b[0] ?? return 0
    e.n
}
fn typed_copy(a: ref Vec[Entry?]) -> i64 {
    let e: Entry = a[0]?.clone() ?? return 0
    e.n
}
fn pt(v: ref Vec[Pt?]) -> i64 {
    let p: Pt = v[0] ?? return 0
    p.x
}
fn arg_ref(v: ref Vec[Entry?]) -> i64 {
    by_ref(v[0] ?? return 0)
}
fn arg_val(v: ref Vec[Entry?]) -> i64 {
    by_val(v[0]?.clone() ?? return 0)
}
fn arg_text(v: ref Vec[String?]) -> i64 {
    text_len(v[0] ?? return 0)
}
fn main() {
    let v: Vec[Entry?] = [Entry { n: 3, s: "a" }, null]
    let w: Vec[Entry?] = [null]
    let ps: Vec[Pt?] = [Pt { x: 5 }]
    let mut ts: Vec[String?] = []
    ts.push("abc")
    println(f"{typed_ref(v)} {chain_ref(w, v)} {typed_copy(v)} {pt(ps)} {arg_ref(v)} {arg_val(v)} {arg_ref(w)} {arg_text(ts)}")
}
"#,
    );
    assert_eq!(said, "3 3 3 5 3 3 0 3\n");
    assert!(!lowered.contains("*nikaia_std::index::get(&a"), "{lowered}");
    assert!(
        lowered.contains("by_ref(match nikaia_std::index::get(&v, 0).as_ref()"),
        "{lowered}"
    );
}

/// **A narrower integer widens into a `T?` of a wider one** (E10-4): `u32`
/// into an `i64?` field is `Some(i64::from(..))`, not a bare `Some(..)`.
#[test]
fn a_narrower_integer_widens_into_a_nullable_slot() {
    let (said, lowered) = printed_and_lowered(
        "widen-some",
        r#"struct R { at: i64? }
struct S { start: u32 }
fn main() {
    let s = S { start: 3 }
    let r = R { at: s.start }
    println(f"{r.at ?? 0}")
}
"#,
    );
    assert_eq!(said, "3\n");
    assert!(lowered.contains("Some(i64::from(s.start))"), "{lowered}");
}

/// **A `T?` of views of values that copy is matched as the values** (E10-4):
/// `map.get(k)` of a `bool` bound by name is a `bool`, not a `&bool`.
#[test]
fn a_nullable_view_of_a_copy_is_matched_as_the_value() {
    let (said, lowered) = printed_and_lowered(
        "copied-match",
        r#"use std::collections
fn main() {
    let mut m: collections::BTreeMap[String, bool] = collections::BTreeMap()
    m.insert("a", true)
    let s = m.get("a")
    match s {
        f => { if f { println("yes") } },
        null => { println("none") },
    }
}
"#,
    );
    assert_eq!(said, "yes\n");
    assert!(lowered.contains("(s).copied()"), "{lowered}");
}
