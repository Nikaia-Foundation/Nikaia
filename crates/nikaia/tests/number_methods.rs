//! **A number's methods are `std`'s** (#438, Part III C.1): `i64::min`,
//! `i64::max` and `i64::div_euclid` are in its ledger, and so is the `abs` of
//! both signed types that Part I 2.2 names, and a method of a number
//! the ledger does not describe is refused here (`NK1210`) rather than taken as
//! *nothing is known* - which made the function around it `async` and handed
//! the call to `rustc` as written.
//!
//! What the methods compute is `tests/language/src/number_methods.nika`.

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn codes(source: &str) -> Vec<String> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .map(|f| f.code.to_string())
        .collect()
}

fn lowered(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Build::default())
        .expect("the source lowers")
        .rust
}

const DESCRIBED: &str = "fn lo(a: i64, b: i64) -> i64 {\n\
     \x20   return a.min(b)\n\
     }\n\
     \n\
     fn hi(a: i64, b: i64) -> i64 {\n\
     \x20   return a.max(b)\n\
     }\n\
     \n\
     fn floor_half(a: i64) -> i64 {\n\
     \x20   return a.div_euclid(2)\n\
     }\n\
     \n\
     fn size(a: i64) -> i64 {\n\
     \x20   return a.abs()\n\
     }\n\
     \n\
     fn main() {\n\
     \x20   println(f\"{lo(7, 3)} {hi(7, 3)} {floor_half(-7)} {size(-5)}\")\n\
     }\n";

/// The issue's three methods are described, so the functions that call them
/// are accepted and stay plain functions.
#[test]
fn min_max_and_div_euclid_are_described_and_do_not_pause() {
    assert!(codes(DESCRIBED).is_empty(), "{:?}", codes(DESCRIBED));
    let rust = lowered(DESCRIBED);
    for name in ["lo", "hi", "floor_half", "size"] {
        assert!(
            rust.contains(&format!("\nfn {name}(")),
            "`{name}` calls only described methods, so it is not `async`:\n{rust}"
        );
    }
}

/// A method no ledger describes on a number is refused in this compiler's words.
#[test]
fn a_method_of_a_number_nothing_describes_is_refused() {
    let source = "fn f(a: i64) -> i64 {\n    return a.frobnicate(2)\n}\n\nfn main() {}\n";
    assert_eq!(codes(source), vec!["NK1210".to_string()]);
    let float = "fn g(a: f64) -> f64 {\n    return a.frobnicate()\n}\n\nfn main() {}\n";
    assert_eq!(codes(float), vec!["NK1210".to_string()]);
}

/// **The methods a program reaches for first are described** (#443): since
/// `NK1210`, a method missing from the ledger is a program refused, so text,
/// powers, the smaller of two and a float's floor have entries - and the
/// function that calls them stays a plain one. What they compute is
/// `tests/language/src/number_methods.nika`.
#[test]
fn the_common_methods_of_a_number_are_described() {
    let source = "fn calc(a: i64, c: i32, d: i32, u: u64, f: f64, g: f64) -> String {\n\
         \x20   let parts = [a.to_string(), a.pow(2).to_string(), (-a).rem_euclid(3).to_string(), (-a).signum().to_string(), c.min(d).to_string(), u.count_ones().to_string(), f.floor().to_string(), f.round().to_string(), f.max(g).to_string(), f.powi(2).to_string(), f.is_nan().to_string()]\n\
         \x20   return parts.join(\" \")\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(calc(7, 4, 2, 5, 1.5, 2.5))\n\
         }\n";
    assert!(codes(source).is_empty(), "{:?}", codes(source));
    let rust = lowered(source);
    assert!(
        rust.contains("\nfn calc("),
        "`calc` does not pause:\n{rust}"
    );
}

/// The findings of a program, code and help.
fn found(source: &str) -> Vec<(String, Option<String>)> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .map(|f| (f.code.to_string(), f.help.clone()))
        .collect()
}

/// **`truncating_` only narrows** (ADR-285 D42, #742): between two types of
/// one width no digit is lost, so the same-width name is refused (`NK1210`)
/// and the help names `bits_as_`. `u8` has no signed partner and so no
/// `bits_as_`; it has no `wrapping_neg` either (ADR-285 D41, #729).
#[test]
fn a_same_width_truncating_name_is_refused_naming_bits_as() {
    for (ty, method, named) in [
        ("u64", "truncating_i64", "bits_as_i64"),
        ("i64", "truncating_u64", "bits_as_u64"),
        ("u32", "truncating_i32", "bits_as_i32"),
        ("i32", "truncating_u32", "bits_as_u32"),
    ] {
        let source = format!(
            "fn f(x: {ty}) {{\n    let y = x.{method}()\n    println(f\"{{y}}\")\n}}\n\nfn main() {{}}\n"
        );
        let got = found(&source);
        assert_eq!(got.len(), 1, "{source}: {got:?}");
        assert_eq!(got[0].0, "NK1210", "{source}: {got:?}");
        let help = got[0].1.clone().unwrap_or_default();
        assert!(help.contains(&format!("`{named}()`")), "{source}: {help}");
    }
    for method in ["bits_as_i32()", "bits_as_u32()", "wrapping_neg()"] {
        let source = format!(
            "fn f(x: u8) {{\n    let y = x.{method}\n    println(f\"{{y}}\")\n}}\n\nfn main() {{}}\n"
        );
        let got = found(&source);
        assert_eq!(got.len(), 1, "{source}: {got:?}");
        assert_eq!(got[0].0, "NK1210", "{source}: {got:?}");
        assert!(
            !got[0].1.clone().unwrap_or_default().contains("bits_as"),
            "{source}: {got:?}"
        );
    }
}

/// **The new names are described** (#742, #729): `bits_as_`, the narrowing
/// `truncating_` names, and `u8`'s arithmetic and conversions are accepted, and
/// the conversions lower to Rust's `as` and `try_from`.
#[test]
fn bits_as_and_the_u8_names_are_described_and_lowered() {
    let source = "fn f(a: u32, b: i64, c: u64, d: f64, e: u8, g: i32) -> String {\n    \
         let w = a.bits_as_i32()\n    \
         let k = b.bits_as_u64()\n    \
         let l = c.truncating_i32()\n    \
         let m = d.truncating_u32()\n    \
         let n = d.truncating_u64()\n    \
         let o = e.wrapping_sub(48)\n    \
         let p = e.saturating_add(10)\n    \
         let q = e.checked_rem(3) ?? 0\n    \
         let r = g.truncating_u8()\n    \
         let s = b.checked_u8() ?? 0\n    \
         let t = d.truncating_u8()\n    \
         return f\"{w} {k} {l} {m} {n} {o} {p} {q} {r} {s} {t}\"\n\
         }\n\nfn main() {}\n";
    assert_eq!(codes(source), Vec::<String>::new());
    let rust = lowered(source);
    for expected in [
        "a as i32",
        "b as u64",
        "c as i32",
        "d as u32",
        "d as u64",
        "e.wrapping_sub(48)",
        "g as u8",
        "u8::try_from(b)",
        "d as u8",
    ] {
        assert!(rust.contains(expected), "{expected}:\n{rust}");
    }
    assert!(
        !rust.contains("bits_as") && !rust.contains("truncating"),
        "{rust}"
    );
}

/// **`NK1215` applies to `bits_as_` and to `u8`'s names** as to the other
/// width-dependent ones (ADR-285 D9).
#[test]
fn bits_as_and_u8_names_are_width_dependent() {
    for (decl, line) in [
        ("a: u32", "let t: i64 = a.bits_as_i32()"),
        ("a: u8", "let t: i32 = a.wrapping_sub(48)"),
        ("a: u8", "let t: u32 = a.saturating_add(10)"),
    ] {
        let source =
            format!("fn f({decl}) {{\n    {line}\n    println(f\"{{t}}\")\n}}\n\nfn main() {{}}\n");
        assert!(
            codes(&source).contains(&"NK1215".to_string()),
            "{source}: {:?}",
            codes(&source)
        );
    }
}
