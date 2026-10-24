//! Defects a move of the toolchain into Nikaia found (ADR-294 D3, #125).
//!
//! Each program here was accepted by the checker and lowered to Rust that
//! `rustc` refused, so the test is the whole path: lower, compile, run, and
//! compare what it printed.

use crate::common;
use std::process::Command;

/// Lowers `source` as a one-file program, compiles the Rust it came to and
/// returns what the program printed.
fn printed(purpose: &str, source: &str) -> String {
    printed_and_lowered(purpose, source).0
}

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

/// #565: the lowered tools package allows the two rustc lints that valid
/// Nikaia trips (`let mut s = init` then branches that always assign `s`, and
/// a `while true { … return … }` then a `return`). Measured once by adding both
/// shapes to a tools file and running `cargo check -p nikaia-std`: two
/// warnings without the attribute, none with it.
#[test]
fn the_tools_package_allows_what_valid_nikaia_trips() {
    let lib = include_str!("../../../nikaia-std/src/lib.rs");
    let before_the_module = lib
        .split("pub mod tools {")
        .next()
        .expect("the tools module is declared");
    let attribute = before_the_module
        .rsplit("#[allow(")
        .next()
        .expect("an allow stands over it");
    for lint in ["unused_assignments", "unreachable_code"] {
        assert!(attribute.contains(lint), "{lint} is allowed over `tools`");
    }
}

/// #563: a `String` local reassigned from a view of itself, in a loop, is a
/// mixed text; the view is copied before it replaces the buffer it points
/// into. The tools package also needs `IntoEither` in scope for it, which
/// `nikaia-std`'s `tools` module brings (measured with a probe in a tools
/// file: E0599 without the import, none with it).
#[test]
fn a_string_reassigned_from_a_view_of_itself_in_a_loop() {
    let said = printed(
        "string-from-its-own-view",
        r#"
fn bare(lines: ref Vec[String]) -> Vec[String] {
    let mut out: Vec[String] = []
    for line in lines {
        let mut rest: String = line.clone()
        let mut again = true
        while again {
            again = false
            if rest.starts_with(">") {
                rest = rest.strip_prefix(">") ?? ""
                again = true
            }
        }
        out.push(rest.clone())
    }
    return out
}
fn main() {
    let v: Vec[String] = [">>a", "b"]
    println(f"{bare(v).join(",")}")
}
"#,
    );
    assert_eq!(said.trim(), "a,b");
    let lib = include_str!("../../../nikaia-std/src/lib.rs");
    assert!(
        lib.contains("use crate::either_text::{IntoEither, IntoEitherMaybe};"),
        "the tools module has the conversions in scope"
    );
}

/// #576, the open shape of `a_mut_parameter_is_assigned_and_read_through`: a
/// `match` over a `mut` parameter whose arm takes the parts and assigns the
/// whole (ADR-094 D7). The arm works on the value taken out of the place and
/// writes the whole back; an arm that takes nothing reads a number it binds
/// as the number, and the place can be a field of the parameter.
#[test]
fn an_arm_that_takes_parts_of_a_mut_parameter_writes_the_whole_back() {
    let said = printed(
        "mut-parameter-parts",
        r#"
enum E {
    Num(i64),
    Add(E, E),
    Wrap(E),
}

fn wrap_nums(mut e: E) {
    match e {
        E::Num(n) => { e = E::Wrap(E::Num(n)) }
        E::Add(a, b) => {
            let mut l = a
            let mut r = b
            wrap_nums(l)
            wrap_nums(r)
            e = E::Add(l, r)
        }
        E::Wrap(x) => {}
    }
}

fn swap_all(mut e: E) {
    match e {
        E::Num(n) => {}
        E::Add(a, b) => {
            let mut l = a
            let mut r = b
            swap_all(l)
            swap_all(r)
            e = E::Add(r, l)
        }
        E::Wrap(x) => { e = x }
    }
}

fn bump(mut e: E) {
    match e {
        E::Num(n) => { e = E::Num(n + 1) }
        E::Add(a, b) => {}
        E::Wrap(x) => {}
    }
}

struct Holder {
    root: E,
    count: i64,
}

fn rewrap(mut h: Holder) {
    match h.root {
        E::Num(n) => {}
        E::Add(a, b) => { h.root = E::Wrap(a) }
        E::Wrap(x) => { h.root = x }
    }
    h.count += 1
}

fn show(e: ref E) -> String {
    match e {
        E::Num(n) => f"{n}",
        E::Add(a, b) => f"({show(a)}+{show(b)})",
        E::Wrap(x) => f"[{show(x)}]",
    }
}

fn main() {
    let mut e = E::Add(E::Num(1), E::Add(E::Num(2), E::Num(3)))
    wrap_nums(e)
    println(show(e))
    swap_all(e)
    println(show(e))
    bump(e)
    println(show(e))
    let mut one = E::Num(1)
    bump(one)
    println(show(one))
    let mut h = Holder { root: E::Add(E::Num(7), E::Num(8)), count: 0 }
    rewrap(h)
    rewrap(h)
    println(f"{show(h.root)} {h.count}")
}
"#,
    );
    assert_eq!(said.trim(), "([1]+([2]+[3]))\n((3+2)+1)\n((3+2)+1)\n2\n7 2");
}

/// #567: a `match` over a `let` over a place, over a field and through a nested `match` binds a number as the number.
#[test]
fn a_number_bound_under_a_reference_is_a_number() {
    let said = printed(
        "number-under-a-reference",
        r#"
enum V {
    Case(i64, i64),
    Pair(V, i64),
    Nothing,
}

struct H {
    kind: V,
    list: Vec[i64],
}

impl H {
    fn one(ref self) -> i64 {
        match self.kind {
            V::Case(a, b) => self.list[a] + b * 2
            else => 0
        }
    }

    fn two(ref self, vs: ref Vec[V]) -> i64 {
        let mut t = 0
        for v in vs {
            match v {
                V::Case(a, b) => { t += self.list[a] + b }
                V::Pair(inner, k) => {
                    match inner {
                        V::Case(c, d) => { t += self.list[c] + d + k }
                        else => {}
                    }
                }
                V::Nothing => {}
            }
        }
        return t
    }
}

fn three(vs: ref Vec[V], list: ref Vec[i64]) -> i64 {
    let v = vs[0]
    match v {
        V::Case(a, b) => list[a] + b
        else => 0
    }
}


fn five(h: ref H) -> i64 {
    match h.kind {
        V::Pair(inner, k) => {
            match inner {
                V::Case(c, d) => h.list[c] + d + k
                else => 0
            }
        }
        else => 0
    }
}

fn main() {
    let h = H { kind: V::Case(1, 5), list: [10, 20, 30] }
    let vs = [V::Case(1, 5), V::Pair(V::Case(2, 1), 4)]
    println(f"{h.one()} {h.two(vs)} {three(vs, h.list)} {five(h)}")
}
"#,
    );
    assert_eq!(said.trim(), "30 60 25 0");
}

/// #570: `v[v.len() - 1] = x` and `+=`, on a list, a list of lists and a field.
#[test]
fn an_index_that_reads_the_target_is_worked_out_before_the_write() {
    let said = printed(
        "index-reads-the-target",
        r#"
struct Holder {
    items: Vec[i64],
}

fn last_set(mut v: Vec[i64], x: i64) {
    v[v.len() - 1] = x
}

fn last_add(mut v: Vec[i64]) {
    v[v.len() - 1] += 1
}

fn nested(mut vv: Vec[Vec[i64]]) {
    vv[vv.len() - 1][0] = 9
    vv[0][vv[0].len() - 1] += 2
}

fn field(mut h: Holder) {
    h.items[h.items.len() - 1] = 7
}

fn main() {
    let mut v: Vec[i64] = [1, 2, 3]
    last_set(v, 8)
    last_add(v)
    let mut vv: Vec[Vec[i64]] = [[1, 2], [3, 4]]
    nested(vv)
    let mut h = Holder { items: [5, 6] }
    field(h)
    let mut w: Vec[i64] = [1, 2, 3]
    w[w.len() - 1] = 5
    w[w.len() - 1] += 1
    println(f"{v[2]} {vv[1][0]} {vv[0][1]} {h.items[1]} {w[2]}")
}
"#,
    );
    assert_eq!(said.trim(), "9 9 4 7 6");
}

/// #574: `x ?? BinaryOp::Add` for an `x: ref BinaryOp?`, with a struct variant as the fallback.
#[test]
fn a_value_beside_a_view_of_an_option_is_lent_with_it() {
    let said = printed(
        "fallback-beside-a-view",
        r#"
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Pow(i64),
}

enum Shape {
    Square(i64),
    Rect { w: i64, h: i64 },
}


fn name(op: ref BinaryOp) -> String {
    match op {
        BinaryOp::Add => "add".clone()
        BinaryOp::Sub => "sub".clone()
        BinaryOp::Mul => "mul".clone()
        BinaryOp::Pow(_) => "pow".clone()
    }
}

fn pick(x: ref BinaryOp?) -> String {
    let op = x ?? BinaryOp::Add
    return name(op)
}

fn pick_const(x: ref BinaryOp?) -> String {
    let op = x ?? BinaryOp::Mul
    return name(op)
}

fn area(s: ref Shape?) -> i64 {
    let shape = s ?? Shape::Rect { w: 2, h: 3 }
    match shape {
        Shape::Square(n) => n * n
        Shape::Rect { w, h } => w * h
    }
}

fn main() {
    let some: BinaryOp? = BinaryOp::Sub
    let none: BinaryOp? = null
    let shape: Shape? = null
    println(f"{pick(some)} {pick(none)} {pick_const(none)} {area(shape)}")
}
"#,
    );
    assert_eq!(said.trim(), "sub add mul 6");
}

/// #566: `return ("text", n)` for a `(String, i64)`, a tail, a struct field, a list element and a nested tuple (no longer reproduces; kept as the test).
#[test]
fn a_literal_in_a_tuple_is_text_where_the_result_says_text() {
    let said = printed(
        "literal-in-a-tuple",
        r#"
struct Row {
    name: String,
    n: i64,
}

fn pair(n: i64) -> (String, i64) {
    return ("text", n)
}

fn tail(n: i64) -> (String, i64) {
    ("tail", n)
}

fn built(n: i64) -> Row {
    return Row { name: "row", n: n }
}

fn rows(n: i64) -> Vec[(String, i64)] {
    return [("a", n), ("b", n + 1)]
}

fn nested(n: i64) -> ((String, i64), String) {
    return (("deep", n), "outer")
}

fn main() {
    let (s, n) = pair(1)
    let (t, m) = tail(2)
    let r = built(3)
    let xs = rows(4)
    let q = nested(5)
    println(f"{s}{n} {t}{m} {r.name}{r.n} {xs.len()} {q.0.0}{q.0.1}{q.1}")
}
"#,
    );
    assert_eq!(said.trim(), "text1 tail2 row3 2 deep5outer");
}

/// #568: mutually recursive `throws` functions, one throwing `make_error(x)` (no longer reproduces; kept as the test).
#[test]
fn a_group_that_throws_a_called_error_keeps_its_channel() {
    let said = printed(
        "throw-a-call",
        r#"
enum Fault {
    Odd { at: i64 },
    Deep(i64),
}

impl Error for Fault {
    fn message(ref self) -> String {
        match self {
            Fault::Odd { at } => f"odd {at}"
            Fault::Deep(n) => f"deep {n}"
        }
    }
}

fn make_error(x: i64) -> Fault {
    return Fault::Odd { at: x }
}

fn even(x: i64) -> i64 throws {
    if x == 0 {
        return 0
    }
    if x == 7 {
        throw make_error(x)
    }
    return odd(x - 1)
}

fn odd(x: i64) -> i64 throws {
    if x == 0 {
        throw Fault::Deep(x)
    }
    return even(x - 1)
}

fn main() {
    let r = even(4) catch { -1 }
    let q = odd(8) catch { -2 }
    let z = even(7) catch { -3 }
    println(f"{r} {q} {z}")
}
"#,
    );
    assert_eq!(said.trim(), "0 -2 -3");
}

/// #569: a `Copy` and a non-`Copy` enum handed on from a `ref` binding and an owned one (no longer reproduces; kept as the test).
#[test]
fn an_enum_is_passed_by_value_from_a_view_and_an_owner() {
    let said = printed(
        "enum-by-value",
        r#"
enum Op {
    Add,
    Sub,
}

enum Tree {
    Leaf(i64),
    Node(Op, Tree, Tree),
}

fn apply(op: Op, a: i64, b: i64) -> i64 {
    match op {
        Op::Add => a + b
        Op::Sub => a - b
    }
}

fn describe(op: Op) -> String {
    match op {
        Op::Add => "add".clone()
        Op::Sub => "sub".clone()
    }
}

fn eval(t: ref Tree) -> i64 {
    match t {
        Tree::Leaf(n) => n
        Tree::Node(op, l, r) => apply(op, eval(l), eval(r))
    }
}

fn names(ops: ref Vec[Op]) -> Vec[String] {
    let mut out: Vec[String] = []
    for op in ops {
        out.push(describe(op))
    }
    return out
}

fn first(ops: ref Vec[Op]) -> String {
    let op = ops[0]
    return describe(op)
}

fn main() {
    let t = Tree::Node(Op::Add, Tree::Leaf(2), Tree::Node(Op::Sub, Tree::Leaf(9), Tree::Leaf(4)))
    let ops = [Op::Add, Op::Sub]
    println(f"{eval(t)} {names(ops).len()} {first(ops)}")
}
"#,
    );
    assert_eq!(said.trim(), "7 2 add");
}

/// #571: `let n = opt ?? return` then `m.get(n)`, `contains_key(n)` and `n.len()` for `ref String?` and `String?` (no longer reproduces; kept as the test).
#[test]
fn an_option_opened_past_a_jump_is_not_borrowed_again() {
    let said = printed(
        "option-past-a-jump",
        r#"
use std::collections

fn lookup(opt: ref String?, m: ref collections::HashMap[String, i64]) -> i64 {
    let n = opt ?? return 0
    let a = m.get(n) ?? 0
    let b = if m.contains_key(n) { 1 } else { 0 }
    return a + b + n.len()
}

fn lookup_owned(opt: String?, m: ref collections::HashMap[String, i64]) -> i64 {
    let n = opt ?? return 0
    let a = m.get(n) ?? 0
    let b = if m.contains_key(n) { 1 } else { 0 }
    return a + b + n.len()
}

fn main() {
    let mut m: collections::HashMap[String, i64] = collections::HashMap()
    m["ab"] = 5
    let some: String? = "ab"
    let none: String? = null
    let other: String? = "ab"
    println(f"{lookup(some, m)} {lookup(none, m)} {lookup_owned(other, m)}")
}
"#,
    );
    assert_eq!(said.trim(), "8 0 8");
}

/// #572: a struct variant holding an enum, matched twice through a `ref` parameter (no longer reproduces; kept as the test).
#[test]
fn a_second_match_under_a_view_writes_no_ref() {
    let said = printed(
        "match-twice",
        r#"
enum Kind {
    Num(i64),
    Neg,
}

enum Term {
    Atom { kind: Kind, name: String },
    Group { inner: Kind, count: i64 },
}

fn show(t: ref Term) -> String {
    match t {
        Term::Atom { kind, name } => {
            match kind {
                Kind::Num(n) => f"{name}={n}"
                Kind::Neg => f"-{name}"
            }
        }
        Term::Group { inner, count } => {
            match inner {
                Kind::Num(n) => f"g{count}:{n}"
                Kind::Neg => f"g{count}"
            }
        }
    }
}

fn main() {
    let a = Term::Atom { kind: Kind::Num(3), name: "x" }
    let b = Term::Group { inner: Kind::Neg, count: 2 }
    println(f"{show(a)} {show(b)}")
}
"#,
    );
    assert_eq!(said.trim(), "x=3 g2");
}

/// Lowers `source` and returns what the compiler said, for a program that is
/// refused.
fn refused(purpose: &str, source: &str) -> String {
    let dir = common::scratch_dir(&format!("selfhosting-{purpose}"));
    std::fs::write(dir.join("p.nika"), source).expect("write the source");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "p.nika", "--output", "p.rs"])
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    assert!(!lowered.status.success(), "{purpose} was not refused");
    String::from_utf8_lossy(&lowered.stderr).into_owned()
}

/// #562: a mixed text position fed by a `match` wraps each arm on its own.
#[test]
fn a_match_into_a_mixed_result_wraps_each_arm() {
    let said = printed(
        "mixed-match",
        r#"
fn int_text(n: i64) -> String {
    return f"{n}"
}

fn plain(s: ref String, n: i64) -> String {
    return match n {
        0 => s,
        else => int_text(n),
    }
}

fn tail_plain(s: ref String, n: i64) -> String {
    match n {
        0 => { s },
        else => { int_text(n) },
    }
}

fn nested(s: ref String, n: i64) -> String {
    return match n {
        0 => s,
        else => if n > 5 { int_text(n) } else { s },
    }
}

fn main() {
    println(plain("hi", 0))
    println(plain("hi", 5))
    println(tail_plain("hi", 0))
    println(tail_plain("hi", 6))
    println(nested("hi", 0))
    println(nested("hi", 9))
    println(nested("hi", 3))
}
"#,
    );
    assert_eq!(said.trim(), "hi\n5\nhi\n6\nhi\n9\nhi");
}

/// #562: the same into a `String?`, with a `null` arm, in the three forms a
/// value reaches the result in.
#[test]
fn a_match_into_a_mixed_option_wraps_each_arm() {
    let said = printed(
        "mixed-option-match",
        r#"
fn int_text(n: i64) -> String {
    return f"{n}"
}

fn by_return(s: ref String, n: i64) -> String? {
    return match n {
        0 => s,
        1 => int_text(n),
        else => null,
    }
}

fn by_tail(s: ref String, n: i64) -> String? {
    match n {
        0 => s,
        1 => int_text(n),
        else => null,
    }
}

fn by_let(s: ref String, n: i64) -> String? {
    let r = match n {
        0 => s,
        1 => int_text(n),
        else => null,
    }
    return r
}

fn main() {
    for n in 0..<3 {
        println(f"{by_return("hi", n) ?? "none"} {by_tail("hi", n) ?? "none"} {by_let("hi", n) ?? "none"}")
    }
}
"#,
    );
    assert_eq!(said.trim(), "hi hi hi\n1 1 1\nnone none none");
}

/// #562: `if` / `else if` / `else` feeds the same positions.
#[test]
fn an_if_into_a_mixed_option_wraps_each_branch() {
    let said = printed(
        "mixed-option-if",
        r#"
fn int_text(n: i64) -> String {
    return f"{n}"
}

fn by_return(s: ref String, n: i64) -> String? {
    return if n == 0 {
        s
    } else if n == 1 {
        int_text(n)
    } else {
        null
    }
}

fn by_tail(s: ref String, n: i64) -> String? {
    if n == 0 {
        s
    } else if n == 1 {
        int_text(n)
    } else {
        null
    }
}

fn by_let(s: ref String, n: i64) -> String? {
    let r = if n == 0 {
        s
    } else if n == 1 {
        int_text(n)
    } else {
        null
    }
    return r
}

fn plain(s: ref String, n: i64) -> String {
    return if n == 0 { s } else { int_text(n) }
}

fn main() {
    for n in 0..<3 {
        println(f"{by_return("hi", n) ?? "none"} {by_tail("hi", n) ?? "none"} {by_let("hi", n) ?? "none"}")
    }
    println(plain("hi", 0))
    println(plain("hi", 4))
}
"#,
    );
    assert_eq!(said.trim(), "hi hi hi\n1 1 1\nnone none none\nhi\n4");
}

/// #562: `ref String` and `String` arms meet as text; the position they flow
/// into decides the representation.
#[test]
fn a_view_and_text_of_its_own_meet_as_text_in_a_let() {
    let said = printed(
        "view-meets-text",
        r#"
fn int_text(n: i64) -> String {
    return f"{n}"
}

fn h(s: ref String, c: bool) -> String {
    let y = if c { s } else { int_text(1) }
    return y
}

fn main() {
    println(h("a", true))
    println(h("a", false))
}
"#,
    );
    assert_eq!(said.trim(), "a\n1");
}

/// #562: the same through a `let` in a `pub` function stays refused.
#[test]
fn a_pub_function_refuses_the_view_arm_of_a_let() {
    let said = refused(
        "mixed-pub-let",
        r#"
fn int_text(n: i64) -> String {
    return f"{n}"
}

pub fn h(s: ref String, c: bool) -> String {
    let y = if c { s } else { int_text(1) }
    return y
}

fn main() {
    println(h("a", true))
}
"#,
    );
    assert!(said.contains("NK1104"), "{said}");
    assert!(said.contains(".clone()"), "{said}");
    assert!(!said.contains("NK1245"), "{said}");
}

/// #562: where the name goes decides, not the function's result: the mixed
/// `let` is handed to a parameter both kinds flow into in a function that
/// returns a number.
#[test]
fn a_mixed_let_goes_where_the_position_takes_it() {
    let said = printed(
        "mixed-let-argument",
        r#"
fn keep(x: String) -> Vec[String] {
    let mut v: Vec[String] = Vec()
    v.push(x)
    return v
}

fn r(s: ref String, c: bool) -> String {
    let y = if c { s } else { f"{c}" }
    let h = keep(y)
    return f"{h.len()}"
}

fn n(s: ref String, c: bool) -> i64 {
    let y = if c { s } else { f"{c}" }
    let h = keep(y)
    return h.len()
}

fn main() {
    let a = "hi".clone()
    println(r(a, true))
    println(n(a, false))
}
"#,
    );
    assert_eq!(said.trim(), "1\n1");
}

/// #562: the same into a mixed field and a mixed parameter.
#[test]
fn a_choice_into_a_mixed_field_and_argument_wraps_each_arm() {
    let said = printed(
        "mixed-field-argument",
        r#"
struct Row { text: String }

fn int_text(n: i64) -> String {
    return f"{n}"
}

fn same(x: String) -> String {
    return x
}

fn keep(s: ref String, n: i64) -> Row {
    return Row { text: if n == 0 { s } else { int_text(n) } }
}

fn arg(s: ref String, n: i64) -> String {
    return same(if n == 0 { s } else { int_text(n) })
}

fn main() {
    println(keep("hi", 0).text)
    println(keep("hi", 7).text)
    println(arg("hi", 0))
    println(arg("hi", 12))
}
"#,
    );
    assert_eq!(said.trim(), "hi\n7\nhi\n12");
}

/// #562: an annotated `let` is a position; the pass writes `.into_either()` on
/// its initializer, and an `if` or a `match` there is wrapped arm by arm.
#[test]
fn an_annotated_mixed_let_fed_by_a_choice_wraps_each_arm() {
    let said = printed(
        "mixed-annotated-let",
        r#"
fn int_text(n: i64) -> String {
    return f"{n}"
}

fn opt(n: i64) -> String? {
    if n > 0 {
        return f"{n}"
    }
    return null
}

fn annotated(s: ref String, c: bool) -> i64 {
    let y: String = if c { s } else { int_text(7) }
    return y.len()
}

fn by_match(s: ref String, n: i64) -> i64 {
    let y: String = match n {
        0 => s,
        else => int_text(n),
    }
    return y.len()
}

fn maybe(s: ref String, n: i64) -> i64 {
    let y: String? = match n {
        0 => s,
        1 => int_text(100),
        else => null,
    }
    return (y ?? "none").len()
}

fn maybe_if(s: ref String, n: i64) -> i64 {
    let y: String? = if n == 0 { s } else if n == 1 { int_text(100) } else { null }
    return (y ?? "none").len()
}

fn maybe_call(s: ref String, n: i64) -> i64 {
    let y: String? = if n == 0 { s } else { opt(n - 1) }
    return (y ?? "none").len()
}

fn main() {
    let a = "hi".clone()
    println(f"{annotated(a, true)} {annotated(a, false)}")
    println(f"{by_match(a, 0)} {by_match(a, 12)}")
    println(f"{maybe(a, 0)} {maybe(a, 1)} {maybe(a, 2)}")
    println(f"{maybe_if(a, 0)} {maybe_if(a, 1)} {maybe_if(a, 2)}")
    println(f"{maybe_call(a, 0)} {maybe_call(a, 1)} {maybe_call(a, 3)}")
}
"#,
    );
    assert_eq!(said.trim(), "2 1\n2 2\n2 3 4\n2 3 4\n2 4 1");
}

/// #598: a view handed to a `String` parameter the callee only reads goes as
/// it is: the parameter is a `&str` (ADR-282 D5, D11), not wrapped in
/// `either(…)`. The same for one call alone and for a `String?` only read.
#[test]
fn a_view_into_a_text_parameter_only_read_is_not_wrapped() {
    let said = printed(
        "view-into-a-read-parameter",
        r#"
fn size(x: String) -> i64 {
    return x.len()
}

fn main() {
    let owned = "abc".clone()
    let s = owned[..<(owned.find("c") ?? return)]
    println(f"{size(s) + size(owned)}")
}
"#,
    );
    assert_eq!(said.trim(), "5");
    let said = printed(
        "view-into-a-read-parameter-alone",
        r#"
fn size(x: String) -> i64 {
    return x.len()
}

fn main() {
    let owned = "abc".clone()
    let s = owned[..<(owned.find("c") ?? return)]
    println(f"{size(s)}")
}
"#,
    );
    assert_eq!(said.trim(), "2");
    let said = printed(
        "view-into-a-nullable-read-parameter",
        r#"
fn size(x: String?) -> i64 {
    return (x ?? "").len()
}

fn main() {
    let owned = "abc".clone()
    let s = owned[..<(owned.find("c") ?? return)]
    let none: String? = null
    println(f"{size(s) + size(owned) + size(none)}")
}
"#,
    );
    assert_eq!(said.trim(), "5");
}

/// #585's fifth over-refusal: a view stored in a local struct that never
/// leaves the function, read only through its fields, is no store past the
/// call (no `NK2302`); `Holder.name` gets a view and text of its own.
#[test]
fn a_view_in_a_local_struct_that_never_leaves_is_not_kept() {
    let said = printed(
        "view-in-a-local-struct",
        r#"
struct Holder {
    name: String,
}

fn two(s: ref String, c: bool) -> i64 {
    if c {
        let h = Holder { name: s }
        return h.name.len()
    }
    let g = Holder { name: f"{c}" }
    return g.name.len()
}

fn main() {
    let a = "ab".clone()
    println(f"{two(a, true)}")
    println(f"{two(a, false)}")
}
"#,
    );
    assert_eq!(said.trim(), "2\n5");
}

/// #558: the failure of a method call is the call's, keyed by its receiver's
/// node (ADR-340 D5): `a.get() + b.get()` where only `Strict::get` throws
/// writes one `?`, where the (statement, name) key gave both one.
#[test]
fn two_calls_of_one_name_in_a_statement_fail_each_on_its_own() {
    let said = printed(
        "fallible-by-receiver",
        r#"
struct Strict {
    n: i64,
}

struct Lenient {
    n: i64,
}

impl Strict {
    fn get(ref self) -> i64 throws {
        return self.n
    }
}

impl Lenient {
    fn get(ref self) -> i64 {
        return self.n
    }
}

fn sum(a: Strict, b: Lenient) -> i64 throws {
    return a.get() + b.get()
}

fn main() {
    let total = sum(Strict { n: 2 }, Lenient { n: 3 }) catch { 0 }
    println(f"{total}")
}
"#,
    );
    assert_eq!(said.trim(), "5");
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

/// Owned text is lent to `strip_prefix` and `strip_suffix` as to
/// `starts_with`, and a literal asked of a list of text is a view of text
/// (both reached `rustc` unlent in `tools/*.nika`).
#[test]
fn owned_text_is_lent_to_strip_and_a_literal_to_contains() {
    let said = printed(
        "strip-owned-contains-literal",
        r#"
fn main() {
    let name: String = "now"
    let prefix = f"from({name}::"
    let error: String = "from(now::f)"
    let rest: String = (error.strip_prefix(prefix) ?? "").clone()
    let tail = f")"
    let param: String = (rest.strip_suffix(tail) ?? "").clone()
    let named: Vec[String] = [name.clone(), "?".clone()]
    let found = named.contains("?")
    println(f"{param} {found}")
}
"#,
    );
    assert_eq!(said.trim(), "f true");
}

/// `get(k)?.clone() ?? d` over a `BTreeMap` of a struct and a `HashMap` of
/// text clones the value, not the reference.
#[test]
fn a_cloned_get_over_either_map_gives_the_value() {
    let said = printed(
        "get-clone-either-map",
        r#"
use std::collections

struct Ty { name: String }

fn lookup(m: ref collections::BTreeMap[String, Ty], k: ref String) -> Ty sync {
    return m.get(k)?.clone() ?? Ty { name: "unknown" }
}

fn lookup_h(m: ref collections::HashMap[String, String], k: ref String) -> String sync {
    return m.get(k)?.clone() ?? "none"
}

fn main() {
    let mut m: collections::BTreeMap[String, Ty] = collections::BTreeMap()
    m["a"] = Ty { name: "A" }
    let mut h: collections::HashMap[String, String] = collections::HashMap()
    h["x"] = "X"
    println(f"{lookup(m, "a").name} {lookup(m, "b").name} {lookup_h(h, "x")} {lookup_h(h, "y")}")
}
"#,
    );
    assert_eq!(said.trim(), "A unknown X none");
}

/// A loop over a `ref Vec[T]` of a type that copies by declaration binds
/// the value, as a loop over numbers does.
#[test]
fn a_loop_over_copies_binds_the_value() {
    let said = printed(
        "loop-over-copies",
        r#"
use std::collections

struct P { x: i64, y: i64 }

fn count(points: ref Vec[P]) -> i64 sync {
    let mut m: collections::BTreeMap[i64, P] = collections::BTreeMap()
    let mut at = 0
    for p in points {
        m[at] = p
        at += 1
    }
    return m.len()
}

fn main() {
    let points: Vec[P] = [P { x: 1, y: 2 }, P { x: 3, y: 4 }]
    println(f"{count(points)} {points[1].y}")
}
"#,
    );
    assert_eq!(said.trim(), "2 4");
}

/// A `T?` parameter, by value and as a view, and a `match` of text literals
/// over a `ref String`, also in a guard
/// (kept as the path that runs).
#[test]
fn an_optional_parameter_and_a_match_of_text_run() {
    let said = printed(
        "optional-param-text-match",
        r#"
struct Ty { name: String }

fn show(t: Ty?) -> String sync {
    let v = t ?? return "none".clone()
    return v.name.clone()
}

fn show_r(t: ref Ty?) -> String sync {
    let v = t ?? return "none".clone()
    return v.name.clone()
}

fn kind(name: ref String) -> String sync {
    let k = match name {
        "i32" | "i64" => "int",
        "f64" => "float",
        else => "other",
    }
    return k.clone()
}

fn guarded(name: ref String, n: i64) -> String sync {
    let r = match n {
        _x if (match name { "i32" | "i64" => true, else => false }) => "int",
        else => "no",
    }
    return r.clone()
}

fn main() {
    let x: Ty? = Ty { name: "b" }
    let s: String = "i64"
    let u: String = "u8"
    println(f"{show(Ty { name: "a" })} {show(null)} {show_r(x)} {kind(s)} {kind(u)} {guarded(s, 1)}")
}
"#,
    );
    assert_eq!(said.trim(), "a none b int other int");
}

/// A text method's pattern is only read, in a loop too: `strip_prefix` and
/// `strip_suffix` of an owned `String` declared outside it (was
/// `NK2105`).
#[test]
fn a_pattern_handed_to_strip_in_a_loop_is_only_read() {
    let said = printed(
        "strip-pattern-in-a-loop",
        r#"
fn published(name: ref String, errors: ref Vec[String]) -> Vec[String] {
    let prefix = f"from({name}::"
    let close = f")"
    let mut out: Vec[String] = []
    for error in errors {
        if error.starts_with(prefix) {
            let rest: String = (error.strip_prefix(prefix) ?? "").clone()
            let param: String = (rest.strip_suffix(close) ?? "").clone()
            out.push(param)
        }
    }
    return out
}

fn main() {
    let errors: Vec[String] = ["from(f::p)".clone(), "from(g::q)".clone(), "from(f::r)".clone()]
    let n: String = "f"
    println(published(n, errors).join(" "))
}
"#,
    );
    assert_eq!(said.trim(), "p r");
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

/// **A map's read of a word past a jump is the word**: `let other = b[word] ??
/// return false` over a map of an enum of bare variants binds a `Tier`, as
/// `let other: Tier = …` does, and hands it to a parameter that takes one
/// (it was `&Tier`, rustc E0308).
#[test]
fn a_word_read_out_of_a_map_past_a_jump_is_a_value() {
    let said = printed(
        "word-past-a-jump",
        r#"
use std::collections

enum Tier { Low, High }
fn tier_word(tier: Tier) -> String {
    return match tier {
        Tier::Low => "low",
        Tier::High => "high",
    }
}
fn same(b: ref collections::BTreeMap[String, Tier], word: ref String) -> String {
    let other = b[word] ?? return "none"
    return tier_word(other)
}
fn main() {
    let mut m: collections::BTreeMap[String, Tier] = collections::BTreeMap()
    m["x"] = Tier::High
    println(f"{same(m, "x")} {same(m, "y")}")
}
"#,
    );
    assert_eq!(said.trim(), "high none");
}

/// **A map's read past a jump, handed to a `ref` parameter, is the view it
/// already is**: `written_of(m[k] ?? continue)` over a map of a struct passes
/// what `index::get` opened, to a function and to a function value alike (it
/// wrote `&` before the `match`, and rustc refused `&&E`).
#[test]
fn a_map_read_past_a_jump_is_handed_to_a_ref_parameter_as_it_is() {
    let said = printed(
        "map-read-to-a-ref-parameter",
        r#"
use std::collections

struct E { n: i64 }
fn written_of(e: ref E) -> String {
    return f"<{e.n}>"
}
fn through(m: ref collections::BTreeMap[String, E], keys: ref Vec[String], shown: fn(ref E) -> String sync) -> String {
    let mut out: String = ""
    for k in keys {
        out.push_str(shown(m[k] ?? continue))
    }
    return out
}
fn main() {
    let mut m: collections::BTreeMap[String, E] = collections::BTreeMap()
    m["a"] = E { n: 1 }
    m["c"] = E { n: 3 }
    let keys: Vec[String] = ["a", "b", "c"]
    let mut out: String = ""
    for k in keys {
        out.push_str(written_of(m[k] ?? continue))
    }
    println(f"{out} {through(m, keys, written_of)}")
}
"#,
    );
    assert_eq!(said.trim(), "<1><3> <1><3>");
}

/// **A variant written beside an option of a borrowed word is the word**:
/// `op ?? BinaryOp::Mul` over the `op` of a `ref Stmt`'s arm is a `BinaryOp`
/// on both sides (it lent the fallback, `|| &BinaryOp::Mul`, where
/// `index::or` copies the word out: rustc E0308).
#[test]
fn a_variant_fallback_beside_a_borrowed_option_of_a_word_is_a_value() {
    let said = printed(
        "variant-beside-a-borrowed-word",
        r#"
enum BinaryOp { Add, Sub, Mul }
enum Stmt {
    Assign { op: BinaryOp?, n: i64 },
    Other,
}
fn shown(s: ref Stmt) -> String {
    return match s {
        Stmt::Assign { op, .. } => match op ?? BinaryOp::Mul {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "sub",
            else => "other",
        },
        Stmt::Other => "none",
    }
}
fn main() {
    let a = Stmt::Assign { op: BinaryOp::Add, n: 1 }
    let b = Stmt::Assign { op: null, n: 2 }
    println(f"{shown(a)} {shown(b)} {shown(Stmt::Other)}")
}
"#,
    );
    assert_eq!(said.trim(), "add other none");
}

/// #578: a `bool` bound by a `match` inside another `match` on a reference is
/// read as the value, as #567 reads a number: used as a condition, negated and
/// returned, through a box, a field, an index, a loop and a `mut` parameter.
#[test]
fn a_bool_bound_in_a_nested_match_is_a_bool() {
    let said = printed(
        "bool-in-a-nested-match",
        r#"
enum F {
    On(bool, i64),
    Wrap(F),
    Hold(S),
    Many(Vec[F]),
    Off,
}

struct S {
    f: F,
    n: i64,
}

fn boxed(f: ref F) -> bool {
    return match f {
        F::Wrap(inner) => match inner {
            F::On(flag, _) => {
                if flag { !flag } else { true }
            }
            else => false,
        },
        else => false,
    }
}

fn negated(f: ref F) -> bool {
    return match f {
        F::Wrap(inner) => match inner {
            F::On(flag, _) => !flag,
            else => false,
        },
        else => false,
    }
}

fn field(f: ref F) -> bool {
    match f {
        F::Hold(h) => {
            match h.f {
                F::On(flag, n) if flag => { return flag && n > 0 }
                F::On(flag, _) => { return !flag }
                else => {}
            }
        }
        else => {}
    }
    return false
}

fn listed(f: ref F) -> i64 {
    let mut seen = 0
    match f {
        F::Many(list) => {
            for one in list {
                match one {
                    F::On(flag, n) => { if !flag { seen += n } }
                    else => {}
                }
            }
            match list[0] {
                F::On(flag, _) => { if flag { seen += 100 } }
                else => {}
            }
        }
        else => {}
    }
    return seen
}

fn both(f: ref F, g: ref F) -> bool {
    return match f {
        F::On(x, _) => match g {
            F::On(flag, _) => x == flag,
            else => !x,
        },
        else => false,
    }
}

fn counted(mut s: S, g: ref F) -> bool {
    return match g {
        F::Wrap(inner) => match inner {
            F::On(flag, _) => {
                if !flag { s.n += 1 }
                flag
            }
            else => false,
        },
        else => false,
    }
}

impl S {
    fn inside(ref self) -> bool {
        match self.f {
            F::Wrap(inner) => {
                match inner {
                    F::On(flag, _) => {
                        if !flag { return false }
                        return flag
                    }
                    else => {}
                }
            }
            else => {}
        }
        return false
    }
}

fn main() {
    let on = F::Wrap(F::On(true, 3))
    let off = F::Wrap(F::On(false, 3))
    let held = F::Hold(S { f: F::On(true, 2), n: 0 })
    let many = F::Many([F::On(true, 1), F::On(false, 2), F::On(false, 4)])
    let mut s = S { f: F::Wrap(F::On(true, 1)), n: 0 }
    let kept = counted(s, off)
    println(f"{boxed(on)} {boxed(off)} {negated(on)} {field(held)} {listed(many)} {both(F::On(true, 0), F::On(true, 0))} {kept} {s.n} {s.inside()}")
}
"#,
    );
    assert_eq!(said.trim(), "false true false true 106 true false 1 true");
}

/// #626: a lambda handed a lent tuple takes it apart with a `let`, as NK1101's
/// help says: a number among the parts is read as the number and text as a
/// view, through `filter`, `map` and the chains between; a tuple the walk made
/// is the lambda's own, and so are its parts.
#[test]
fn a_tuple_let_over_a_lent_lambda_parameter_reads_its_parts() {
    let said = printed(
        "tuple-let-over-a-lambda-parameter",
        r#"
fn main() {
    let pairs: Vec[(String, i64)] = [("a".clone(), 1), ("b".clone(), 2)]
    let n = pairs.iter().filter(fn(pair) {
        let (name, v) = pair
        v > 1
    }).count()
    println(f"{n}")
    let doubled: Vec[i64] = pairs.iter().map(fn(pair) {
        let (name, v) = pair
        if name == "b" { v * 10 } else { v + name.len() }
    }).collect()
    println(f"{doubled[0]} {doubled[1]}")
    let named: Vec[String] = pairs.iter().filter(fn(pair) {
        let (name, v) = pair
        !(v > 1) && name.starts_with("a")
    }).map(fn(pair) {
        let (name, v) = pair
        f"{name}={v + 1}"
    }).collect()
    println(named.join(","))
    let kept: Vec[String] = pairs.iter().map(fn(pair) {
        let (name, v) = pair
        name.clone()
    }).collect()
    println(kept.join("+"))
    let owned: Vec[String] = pairs.iter().map(fn(pair) { (pair.0.clone(), pair.1 * 2) }).map(fn(pair) {
        let (name, v) = pair
        if v > 2 { return name }
        "small".clone()
    }).collect()
    println(owned.join(" "))
    let found: i64 = pairs.iter().filter(fn(pair) {
        let (name, v) = pair
        v > 0
    }).take(5).map(fn(pair) {
        let (name, v) = pair
        v
    }).sum()
    println(f"{found}")
}
"#,
    );
    assert_eq!(said, "1\n2 20\na=2\na+b\nsmall b\n3\n");
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

/// The same off a list of lists, and off a list of numbers.
#[test]
fn a_reach_off_a_nested_element_and_a_number_matches_on_the_reference() {
    let said = printed(
        "reach-off-a-nested-element",
        r#"
fn main() {
    let d: Vec[Vec[String?]] = [["x", null], [null, "yz"]]
    println(d[1][1]?.clone() ?? "none")
    println(f"{d[0][1]?.len() ?? 7}")
    let n: Vec[i64?] = [5, null]
    println(f"{n[0]?.abs() ?? 0} {n[1]?.abs() ?? 0}")
}
"#,
    );
    assert_eq!(said, "yz\n7\n5 0\n");
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

/// The same off a list that is not `mut` is the checker's refusal.
#[test]
fn an_element_of_a_list_that_is_not_mut_is_refused_by_the_checker() {
    let said = refused(
        "element-of-an-immutable-list",
        r#"
struct Expr {
    n: i64,
}
fn bump(mut e: Expr) {
    e.n += 1
}
fn main() {
    let args: Vec[Expr] = [Expr { n: 1 }]
    let i = 0
    bump(args[i])
    print(args[0].n)
}
"#,
    );
    assert!(said.contains("NK1139"), "{said}");
}

/// #817: `m["k"]?.clone()` on a map matched on `&*get(..)`, a `&Option<&T>`,
/// whose arm bound a `&&T` and cloned the `&T` (E0308). A map's read is the
/// option of a reference already and is matched as it is.
#[test]
fn a_reach_off_a_map_read_matches_the_option_it_is() {
    let said = printed(
        "reach-off-a-map-read",
        r#"
use std::collections
fn look(m: ref collections::HashMap[String, String], k: String) -> String? {
    return m[k]?.clone()
}
fn main() {
    let mut m: collections::HashMap[String, String] = collections::HashMap()
    m["k"] = "vv"
    println(look(m, "k") ?? "none")
    println(look(m, "q") ?? "none")
    let a = m["k"]?.clone()
    println(a ?? "none")
    println(f"{m["k"]?.len() ?? 0} {m["z"]?.len() ?? 0}")
}
"#,
    );
    assert_eq!(said, "vv\nnone\nvv\n2 0\n");
}

const MUT_VIEW_ENTRY: &str = r#"
use std::collections

struct Entry {
    names: Vec[String],
}
"#;

/// #816: `let mut x = m[k] ?? continue` on a `mut` map is a mutable view of
/// the entry, so a change through `x` reaches `m`.
#[test]
fn a_let_mut_of_a_mut_maps_entry_is_a_mutable_view() {
    let source = format!(
        "{MUT_VIEW_ENTRY}{}",
        r#"
fn add(mut m: collections::HashMap[String, Entry], keys: ref Vec[String]) {
    for k in keys {
        let mut x = m[k] ?? continue
        x.names.push("n".clone())
    }
}

fn main() {
    let mut m: collections::HashMap[String, Entry] = collections::HashMap()
    m["a"] = Entry { names: [] }
    add(m, ["a".clone(), "missing".clone()])
    let e = m["a"] ?? return
    println(e.names.len())
}
"#
    );
    let said = printed("mut-view-of-a-map-entry", &source);
    assert_eq!(said.trim(), "1");
}

/// #816: the same over a list whose elements are options.
#[test]
fn a_let_mut_of_a_mut_lists_element_is_a_mutable_view() {
    let source = format!(
        "{MUT_VIEW_ENTRY}{}",
        r#"
fn add(mut v: Vec[Entry?], at: i64) {
    let mut x = v[at] ?? return
    x.names.push("n".clone())
}

fn main() {
    let mut v: Vec[Entry?] = [Entry { names: [] }, null]
    add(v, 0)
    add(v, 1)
    let n = v[0]?.names?.len() ?? 0
    println(n)
}
"#
    );
    let said = printed("mut-view-of-a-list-element", &source);
    assert_eq!(said.trim(), "1");
}

/// #816: a place that is not `mut` has no mutable view.
#[test]
fn a_let_mut_of_a_place_that_is_not_mut_is_refused() {
    let dir = common::scratch_dir("selfhosting-mut-view-refused");
    let source = format!(
        "{MUT_VIEW_ENTRY}{}",
        r#"
fn add(m: collections::HashMap[String, Entry], keys: ref Vec[String]) {
    for k in keys {
        let mut x = m[k] ?? continue
        x.names.push("n".clone())
    }
}

fn main() {
    let mut m: collections::HashMap[String, Entry] = collections::HashMap()
    add(m, ["a".clone()])
}
"#
    );
    std::fs::write(dir.join("p.nika"), source).expect("write the source");
    let checked = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["check", "p.nika"])
        .output()
        .expect("the nikaia binary runs");
    let said = String::from_utf8_lossy(&checked.stderr).into_owned()
        + &String::from_utf8_lossy(&checked.stdout);
    std::fs::remove_dir_all(&dir).ok();
    assert!(!checked.status.success(), "{said}");
    assert!(said.contains("NK1138"), "{said}");
}

/// #816: `let mut s = c.field ?? return` on a `mut` view of an option field
/// is a mutable view too (`as_mut`), so a change through `s` reaches the map.
#[test]
fn a_let_mut_of_a_mut_views_option_field_is_a_mutable_view() {
    let said = printed(
        "mut-view-of-an-option-field",
        r#"
use std::collections

struct Sig {
    names: Vec[String],
}

struct C {
    sig: Sig?,
}

fn add(mut m: collections::HashMap[String, C], k: ref String) {
    let mut c = m[k] ?? return
    let mut s = c.sig ?? return
    s.names.push("n".clone())
}

fn main() {
    let mut m: collections::HashMap[String, C] = collections::HashMap()
    m["a"] = C { sig: Sig { names: [] } }
    add(m, "a")
    let c = m["a"] ?? return
    let n = c.sig?.names?.len() ?? 0
    println(n)
}
"#,
    );
    assert_eq!(said.trim(), "1");
}
