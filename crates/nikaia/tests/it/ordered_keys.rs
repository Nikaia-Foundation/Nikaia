//! An ordered map whose key has no order is refused here, not by `rustc`
//! ([ADR-010](../../../docs/specification/adr/adr-010.md) D10, `NK1247`, #589).
//!
//! `BTreeMap[P, V]` over a declared `struct` passed the checker and reached
//! `rustc` as *the trait bound `P: Ord` is not satisfied*: a message about a
//! file nobody wrote (Part III C.1). Both directions are the point: a key that
//! has an order must pass, or a correct program is refused (C.4).

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn refusals(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.code == "NK1247")
        .collect()
}

fn with_key(key: &str, map: &str) -> String {
    format!(
        "use std::collections\n\
         struct P {{ x: i64 }}\n\
         enum E {{ A, B }}\n\
         fn main() {{\n\
         \x20   let m: collections::{map}[{key}, i64] = collections::{map}()\n\
         \x20   println(\"x\")\n\
         }}\n"
    )
}

#[test]
fn a_key_with_an_order_passes() {
    for key in [
        "i64",
        "String",
        "bool",
        "char",
        "(i64, String)",
        "Vec[String]",
        "String?",
        "winnow_grammar::Symbol",
    ] {
        let source = with_key(key, "BTreeMap");
        assert!(
            refusals(&source).is_empty(),
            "{key}: {:#?}",
            refusals(&source)
        );
    }
    let set = "use std::collections\nfn main() {\n    let s: collections::BTreeSet[i64] = collections::BTreeSet()\n    println(\"x\")\n}\n";
    assert!(refusals(set).is_empty());
}

#[test]
fn a_key_with_none_is_refused_in_this_languages_words() {
    for key in [
        "P",
        "E",
        "f64",
        "(i64, f64)",
        "collections::HashMap[String, i64]",
    ] {
        let source = with_key(key, "BTreeMap");
        let found = refusals(&source);
        assert_eq!(found.len(), 1, "{key}: {found:#?}");
        assert!(
            found[0].message.contains("keeps its keys in order"),
            "{}",
            found[0].message
        );
        assert!(found[0].help.as_deref().unwrap_or("").contains("HashMap"));
    }
    let set = with_key("P", "BTreeSet");
    assert_eq!(refusals(&set).len(), 1);
}

/// The hash maps ask nothing of their key's order.
#[test]
fn an_unordered_map_asks_for_no_order() {
    for key in ["P", "f64"] {
        let source = with_key(key, "HashMap");
        assert!(refusals(&source).is_empty(), "{key}");
    }
}

/// A key inside a field and a signature is asked too.
#[test]
fn a_key_in_a_field_or_a_signature_is_refused() {
    let source = "use std::collections\n\
                  struct P { x: i64 }\n\
                  struct Holder { by: collections::BTreeMap[P, i64] }\n\
                  fn ids(m: ref collections::BTreeMap[P, i64]) -> i64 { return 0 }\n\
                  fn main() { println(\"x\") }\n";
    assert_eq!(refusals(source).len(), 2, "{:#?}", refusals(source));
}
