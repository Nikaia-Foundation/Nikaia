//! `Eq` and `Ord` are traits of the language
//! ([ADR-341](../../../docs/specification/adr/adr-341.md), #592).
//!
//! A bound on them gives the body the operators; an operator on a type
//! parameter no bound allows is `NK1248`, which used to reach `rustc` as
//! *binary operation `<` cannot be applied to type `T`* (Part III C.1); and a
//! program writes `impl Eq` and `impl Ord` for the types it declares
//! (`NK1249` otherwise). What the programs do when they run is
//! `tests/language/src/std_bounds.nika`; this file keeps the refusals and the
//! lowering.

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library).findings
}

fn codes(source: &str) -> Vec<String> {
    findings(source).into_iter().map(|f| f.code).collect()
}

fn lowered(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Build::default())
        .expect("the source lowers")
        .rust
}

const POINT: &str = "struct P { x: i64 }\n\
    impl Ord for P {\n\
    \x20   fn compare(ref self, other: ref P) -> Ordering {\n\
    \x20       if self.x < other.x { return Ordering::Less }\n\
    \x20       if self.x > other.x { return Ordering::Greater }\n\
    \x20       return Ordering::Equal\n\
    \x20   }\n\
    }\n";

#[test]
fn a_bound_gives_the_operators() {
    let source = "fn biggest[T: Ord](a: T, b: T) -> T {\n\
         \x20   if a > b { return a }\n\
         \x20   return b\n\
         }\n\
         fn same[T: Eq](a: ref T, b: ref T) -> bool { return a == b }\n\
         fn ordered_is_equal[T: Ord](a: ref T, b: ref T) -> bool { return a == b }\n\
         fn main() { println(biggest(1, 2)) }\n";
    assert!(codes(source).is_empty(), "{:#?}", findings(source));
    let rust = lowered(source);
    assert!(rust.contains("fn biggest<T: Ord>"), "{rust}");
    assert!(rust.contains("fn same<T: Eq>"), "{rust}");
}

#[test]
fn an_operator_without_a_bound_is_refused_in_this_languages_words() {
    for (op, needs) in [("<", "Ord"), (">=", "Ord"), ("==", "Eq"), ("!=", "Eq")] {
        let source = format!(
            "fn f[T](a: ref T, b: ref T) -> bool {{ return a {op} b }}\n\
             fn main() {{ println(1) }}\n"
        );
        let found: Vec<_> = findings(&source)
            .into_iter()
            .filter(|f| f.code == "NK1248")
            .collect();
        assert_eq!(found.len(), 1, "{op}: {:#?}", findings(&source));
        assert!(
            found[0].message.contains(&format!("T: {needs}")),
            "{}",
            found[0].message
        );
    }
    // `Eq` does not give an order.
    let source = "fn f[T: Eq](a: ref T, b: ref T) -> bool { return a < b }\n\
                  fn main() { println(1) }\n";
    assert!(codes(source).contains(&"NK1248".to_string()));
}

#[test]
fn an_impl_ord_is_a_bridge_to_the_standard_librarys() {
    let source = format!("{POINT}fn main() {{ println(1) }}\n");
    assert!(codes(&source).is_empty(), "{:#?}", findings(&source));
    let rust = lowered(&source);
    assert!(rust.contains("impl Ord for P"), "{rust}");
    assert!(rust.contains("impl PartialOrd for P"), "{rust}");
    assert!(!rust.contains("async fn compare"), "{rust}");
}

#[test]
fn an_impl_eq_replaces_the_derived_equality() {
    let source = "struct P { x: i64, note: String }\n\
         impl Eq for P {\n\
         \x20   fn eq(ref self, other: ref P) -> bool { return self.x == other.x }\n\
         }\n\
         fn main() { println(1) }\n";
    assert!(codes(source).is_empty(), "{:#?}", findings(source));
    let rust = lowered(source);
    assert!(rust.contains("impl PartialEq for P"), "{rust}");
    assert!(!rust.contains("derive(Debug, Clone, PartialEq"), "{rust}");
}

#[test]
fn an_impl_for_a_type_the_program_does_not_declare_is_nk1249() {
    let source = "use std::time\n\
         impl Ord for time::Duration {\n\
         \x20   fn compare(ref self, other: ref time::Duration) -> Ordering { return Ordering::Equal }\n\
         }\n\
         fn main() { println(1) }\n";
    assert!(
        codes(source).contains(&"NK1249".to_string()),
        "{:#?}",
        findings(source)
    );
}

#[test]
fn an_order_for_a_type_that_is_not_eq_is_nk1249() {
    let source = "struct R { t: f64 }\n\
         impl Ord for R {\n\
         \x20   fn compare(ref self, other: ref R) -> Ordering { return Ordering::Equal }\n\
         }\n\
         fn main() { println(1) }\n";
    let found: Vec<_> = findings(source)
        .into_iter()
        .filter(|f| f.code == "NK1249")
        .collect();
    assert_eq!(found.len(), 1, "{:#?}", findings(source));
    assert!(found[0].message.contains("`Eq`"), "{}", found[0].message);
}

#[test]
fn an_impl_owes_its_one_method_and_no_other() {
    let missing = "struct P { x: i64 }\nimpl Ord for P {}\nfn main() { println(1) }\n";
    assert!(
        codes(missing).contains(&"NK1130".to_string()),
        "{:#?}",
        findings(missing)
    );
    let extra = "struct P { x: i64 }\n\
         impl Eq for P {\n\
         \x20   fn eq(ref self, other: ref P) -> bool { return true }\n\
         \x20   fn other(ref self) -> bool { return true }\n\
         }\n\
         fn main() { println(1) }\n";
    assert!(
        codes(extra).contains(&"NK1130".to_string()),
        "{:#?}",
        findings(extra)
    );
}

#[test]
fn a_declared_type_with_impl_ord_is_a_key() {
    let source = format!(
        "use std::collections\n{POINT}\
         fn main() {{\n\
         \x20   let mut m: collections::BTreeMap[P, i64] = collections::BTreeMap()\n\
         \x20   m.insert(P {{ x: 1 }}, 1)\n\
         \x20   println(m.len())\n\
         }}\n"
    );
    assert!(
        !codes(&source).contains(&"NK1247".to_string()),
        "{:#?}",
        findings(&source)
    );
}

#[test]
fn a_program_that_declares_its_own_ord_keeps_it() {
    let source = "trait Ord { fn rank(ref self) -> i64 }\n\
         struct P { x: i64 }\n\
         impl Ord for P { fn rank(ref self) -> i64 { return self.x } }\n\
         fn first[T: Ord](a: ref T) -> i64 { return a.rank() }\n\
         fn main() { println(first(P { x: 1 })) }\n";
    assert!(codes(source).is_empty(), "{:#?}", findings(source));
}
