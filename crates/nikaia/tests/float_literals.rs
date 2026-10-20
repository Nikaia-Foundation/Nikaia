//! **A float literal is an `f64`** (#503): `f64` is the only float type
//! (Part I 2.2), so a `let` without an annotation whose value is `0.0` is one,
//! and so is everything computed from it. It used to be unknown, as an integer
//! literal is until a use decides it, and a wrong use of it reached `rustc`.

mod common;

use nikaia::check::Finding;
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = common::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's shipped ledger parses");
    common::checked(&parsed, &own, &library).findings
}

/// `let mut px = 0.0`, added to, checks; and it is an `f64` where a `let`
/// says something else.
#[test]
fn a_float_let_is_an_f64() {
    let fine = r#"
fn main() {
    let mut px = 0.0
    let mass = 2.0
    px = px + 1.5 * mass
    println(f"{px}")
}
"#;
    assert!(findings(fine).is_empty(), "{:#?}", findings(fine));
    let wrong = r#"
fn main() {
    let px = 0.0
    let n: i64 = px
    println(f"{n}")
}
"#;
    let found = findings(wrong);
    assert!(
        found
            .iter()
            .any(|f| f.code == "NK1103" && f.message.contains("`f64`")),
        "{found:#?}"
    );
}

/// `px + n` with an `i64` is the mixed-type refusal.
#[test]
fn a_float_let_and_an_integer_do_not_mix() {
    let source = r#"
fn main() {
    let px = 0.0
    let n: i64 = 3
    let sum = px + n
    println(f"{sum}")
}
"#;
    let found = findings(source);
    assert!(found.iter().any(|f| f.code == "NK1199"), "{found:#?}");
}

/// A list of float literals is a `Vec[f64]`, and its elements compare.
#[test]
fn a_list_of_floats_holds_f64s() {
    let fine = r#"
fn main() {
    let readings = [1.0, 2.5, 0.5]
    let mut peak = 0
    for day in 0..<readings.len() {
        if readings[day] > readings[peak] {
            peak = day
        }
    }
    println(f"{peak}")
}
"#;
    assert!(findings(fine).is_empty(), "{:#?}", findings(fine));
    let wrong = r#"
fn main() {
    let readings = [1.0, 2.5, 0.5]
    let first: bool = readings[0]
    println(f"{first}")
}
"#;
    let found = findings(wrong);
    assert!(
        found
            .iter()
            .any(|f| f.code == "NK1103" && f.message.contains("`f64`")),
        "{found:#?}"
    );
}

/// **An integer literal is no float** (Part I 2.4, #703): a number takes its
/// type among the integer types, so `2` where an `f64` is declared, handed or
/// met is refused in the position's own words, as the language below would
/// refuse the generated file.
#[test]
fn an_integer_literal_is_no_float() {
    let cases: [(&str, &str); 11] = [
        (
            "NK1103",
            "fn main() {\n    let x: f64 = 2\n    println(f\"{x}\")\n}\n",
        ),
        (
            "NK1102",
            "fn half(x: f64) -> f64 { return x / 2.0 }\nfn main() { println(f\"{half(3)}\") }\n",
        ),
        (
            "NK1103",
            "fn pick(c: bool) -> f64 {\n    let y: f64 = if c { 2 } else { 3.5 }\n    return y\n}\nfn main() { println(f\"{pick(true)}\") }\n",
        ),
        (
            "NK1104",
            "fn t() -> f64 { 3 }\nfn main() { println(f\"{t()}\") }\n",
        ),
        (
            "NK1104",
            "fn t() -> f64 { return -3 }\nfn main() { println(f\"{t()}\") }\n",
        ),
        (
            "NK1106",
            "struct P { a: f64 }\nfn main() {\n    let p = P { a: 3 }\n    println(f\"{p.a}\")\n}\n",
        ),
        (
            "NK1105",
            "fn main() {\n    let mut x: f64 = 1.5\n    x = 3\n    println(f\"{x}\")\n}\n",
        ),
        (
            "NK1199",
            "fn main() {\n    let x: f64 = 1.5\n    println(f\"{x * 2} {x > 0}\")\n}\n",
        ),
        (
            "NK1199",
            "fn main() {\n    let mut x: f64 = 1.5\n    x += 1\n    println(f\"{x}\")\n}\n",
        ),
        (
            "NK1154",
            "fn main() {\n    let v = [1.5, 2]\n    println(f\"{v.len()}\")\n}\n",
        ),
        (
            "NK1102",
            "fn half(x: f64) -> f64 { return x / 2.0 }\nfn main() {\n    let n = 2\n    println(f\"{half(n)}\")\n}\n",
        ),
    ];
    for (code, source) in cases {
        let found = findings(source);
        assert!(
            found.iter().any(|f| f.code == code),
            "{code} for\n{source}\n{found:#?}"
        );
    }
    let fine = "fn half(x: f64) -> f64 { return x / 2.0 }\nfn main() {\n    let x: f64 = 2.0\n    let n = 3\n    println(f\"{half(x * 2.0 + 1.5)} {n * 2} {-x}\")\n}\n";
    assert!(findings(fine).is_empty(), "{:#?}", findings(fine));
}

/// **A float written out takes `f32` where its use asks for one** (#728,
/// Part I 2.2): a `let`, an argument, a `return`, a field, a list and an
/// assignment. A name bound to one is an `f64` and stays one.
#[test]
fn a_float_literal_is_an_f32_where_one_is_asked() {
    let fine = "struct S { a: f32 }\n\
                fn half(x: f32) -> f32 { return x * 0.5 }\n\
                fn one() -> f32 { return -1.5 }\n\
                fn main() {\n    let y: f32 = 0.5\n    let s = S { a: 2.5 }\n    let v: Vec[f32] = [1.0, 2.25]\n    let o: f32? = 0.5\n    let mut m: f32 = 1.0\n    m = 2.0 * 0.5\n    println(f\"{y} {s.a} {v[0]} {o ?? 0.0} {half(3.0)} {one()} {m}\")\n}\n";
    assert!(findings(fine).is_empty(), "{:#?}", findings(fine));
    let wrong = "fn main() {\n    let x = 0.5\n    let y: f32 = x\n    println(f\"{y}\")\n}\n";
    assert!(
        findings(wrong).iter().any(|f| f.code == "NK1103"),
        "{:#?}",
        findings(wrong)
    );
}

/// **A list that mixes a float and an integer is told once** (#728): `NK1154`
/// about the list, and not `NK1103` about the `let` it goes into as well.
#[test]
fn a_mixed_list_is_only_nk1154() {
    for want in ["Vec[f64]", "Vec[i64]"] {
        let source = format!(
            "fn main() {{\n    let v: {want} = [1.0, 2]\n    println(f\"{{v.len()}}\")\n}}\n"
        );
        let codes: Vec<String> = findings(&source).into_iter().map(|f| f.code).collect();
        assert_eq!(codes, ["NK1154"], "{source}");
    }
}
