//! A field in a struct pattern takes a pattern
//! ([ADR-291](../../../docs/specification/adr/adr-291.md) D25, #573): `x: 0`
//! tests the field, `x: px` binds it to `px`, `center: Point { x, .. }` reaches
//! inside, and `x` alone is `x: x`.
//!
//! The program below is checked, lowered, compiled and run, and what it prints
//! is compared.

use crate::common;
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

const PROGRAM: &str = "\
struct Point { x: i64, y: i64 }
struct Pair { name: i64, other: i64 }
enum Shape { Circle { r: i64 }, Square { side: i64 } }
enum Op { Add, Sub }
enum Expr { Num(i64), Bin { op: Op, l: i64, r: i64 } }
enum Event { Click { at: Point }, Quit }

fn where_is(p: Point) -> i64 {
    return match p {
        Point { x: 0, y } => y,
        Point { x: px, .. } => px * 100,
        else => 0,
    }
}

fn size(s: Shape) -> i64 {
    return match s {
        Shape::Circle { r: size } | Shape::Square { side: size } => size,
    }
}

fn eval(e: Expr) -> i64 {
    return match e {
        Expr::Bin { op: Op::Add, l, r } => l + r,
        Expr::Bin { op: Op::Sub, l, r } => l - r,
        Expr::Num(n) => n,
    }
}

fn both(a: Pair, b: Pair) -> i64 {
    return match (a, b) {
        (Pair { name: x, .. }, Pair { name: y, .. }) => x * 10 + y,
        else => 0,
    }
}

fn click_x(e: Event) -> i64 {
    return match e {
        Event::Click { at: Point { x, .. } } => x,
        Event::Quit => -1,
    }
}

fn main() {
    println(f\"{where_is(Point { x: 0, y: 7 })} {where_is(Point { x: 3, y: 7 })}\")
    println(f\"{size(Shape::Circle { r: 4 })} {size(Shape::Square { side: 5 })}\")
    println(f\"{eval(Expr::Bin { op: Op::Add, l: 2, r: 3 })} {eval(Expr::Bin { op: Op::Sub, l: 9, r: 4 })} {eval(Expr::Num(8))}\")
    println(f\"{both(Pair { name: 1, other: 0 }, Pair { name: 2, other: 0 })}\")
    println(f\"{click_x(Event::Click { at: Point { x: 6, y: 0 } })} {click_x(Event::Quit)}\")
}
";

/// Every form of the rule, through the checker, the lowering and `rustc`.
#[test]
fn a_field_takes_a_pattern() {
    let found = findings(PROGRAM);
    assert!(found.is_empty(), "{found:#?}");
    let rust = lowered(PROGRAM);
    for written in [
        "Point { x: 0, y }",
        "Point { x: px, .. }",
        "Shape::Circle { r: size } | Shape::Square { side: size }",
        "Expr::Bin { op: Op::Add, l, r }",
        "Event::Click { at: Point { x, .. } }",
    ] {
        assert!(rust.contains(written), "`{written}` is not in\n{rust}");
    }
    let dir = common::scratch_dir("struct-patterns");
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary).output().expect("run it");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "7 300\n4 5\n5 5 8\n12\n6 -1\n"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A name bound twice by differently-named fields of two alternatives is the
/// same rule as before (`NK1155`): the alternatives have to bind the same names.
#[test]
fn alternatives_that_bind_different_names_are_still_refused() {
    let source = "\
enum Shape { Circle { r: i64 }, Square { side: i64 } }
fn size(s: Shape) -> i64 {
    return match s {
        Shape::Circle { r: radius } | Shape::Square { side: size } => 1,
    }
}
fn main() { println(f\"{size(Shape::Circle { r: 1 })}\") }
";
    let codes: Vec<String> = findings(source).iter().map(|f| f.code.clone()).collect();
    assert!(codes.contains(&"NK1155".to_string()), "{codes:?}");
}

/// **A struct pattern that names some of a struct's fields and not `..` is
/// `NK1234`** (Part I 3.4, #732), as a literal that leaves a field out is,
/// and so is one inside another pattern. With `..`, or naming every field, it
/// is accepted.
#[test]
fn a_pattern_that_leaves_fields_out_without_the_rest_is_refused() {
    let refused = [
        "struct Point { x: i64, y: i64 }\nfn f(p: Point) -> i64 {\n    return match p {\n        Point { x: px } => px,\n    }\n}\nfn main() { println(f\"{f(Point { x: 1, y: 2 })}\") }\n",
        "struct Point { x: i64, y: i64 }\nenum Event { Click(Point), Key }\nfn f(e: Event) -> i64 {\n    return match e {\n        Event::Click(Point { y }) => y,\n        else => 0,\n    }\n}\nfn main() { println(f\"{f(Event::Key)}\") }\n",
        "enum Shape { Rect { w: i64, h: i64 }, Dot }\nfn f(s: Shape) -> i64 {\n    return match s {\n        Shape::Rect { w } => w,\n        else => 0,\n    }\n}\nfn main() { println(f\"{f(Shape::Dot)}\") }\n",
    ];
    for source in refused {
        let found = findings(source);
        assert!(
            found.iter().any(|f| f.code == "NK1234"),
            "{source}\n{:#?}",
            found.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }
    let accepted = [
        "struct Point { x: i64, y: i64 }\nfn f(p: Point) -> i64 {\n    return match p {\n        Point { x: px, .. } => px,\n    }\n}\nfn main() { println(f\"{f(Point { x: 1, y: 2 })}\") }\n",
        "struct Point { x: i64, y: i64 }\nfn f(p: Point) -> i64 {\n    return match p {\n        Point { x, y } => x + y,\n    }\n}\nfn main() { println(f\"{f(Point { x: 1, y: 2 })}\") }\n",
    ];
    for source in accepted {
        let found = findings(source);
        assert!(
            !found.iter().any(|f| f.code == "NK1234"),
            "{source}\n{found:#?}"
        );
    }
}
