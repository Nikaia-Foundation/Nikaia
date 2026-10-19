//! **A lambda takes the arguments it names** (Part I 5.3, #703): `fn { … }`
//! takes none, so handing it to a place that hands it one is `NK1101`, said
//! here and not by the language below about the generated closure.

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

#[test]
fn a_lambda_that_names_none_is_handed_one() {
    for (walk, count) in [
        ("xs.map fn { 7 }", "len"),
        ("xs.iter().map fn { 7 }", "count"),
    ] {
        let source = format!(
            "fn main() {{\n    let xs = [1, 2, 3]\n    let ys = {walk}\n    println(f\"{{ys.{count}()}}\")\n}}\n"
        );
        let found = findings(&source);
        assert!(
            found
                .iter()
                .any(|f| f.code == "NK1101" && f.message.contains("takes no arguments")),
            "{source}\n{found:#?}"
        );
    }
    let fine = "fn main() {\n    let xs = [1, 2, 3]\n    let ys = xs.map fn(_) { 7 }\n    println(f\"{ys.len()}\")\n}\n";
    assert!(findings(fine).is_empty(), "{:#?}", findings(fine));
}
