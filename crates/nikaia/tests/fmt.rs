//! `nikaia fmt` ([ADR-337](../../../docs/specification/adr/adr-337.md),
//! Part III 13.2): the comments the parser hands beside the tree, and the
//! printer that puts them back.

use nikaia::parser::parse_to_ast;

/// The text of every comment the parser recorded, in order.
fn comments(source: &str) -> Vec<String> {
    let parsed = parse_to_ast(source).expect("parses");
    parsed
        .comments
        .iter()
        .map(|span| source[span.start as usize..span.end as usize].to_string())
        .collect()
}

#[test]
fn every_comment_is_recorded_in_order_with_its_place() {
    let source = "// before the item\n\
                  /// the item's documentation\n\
                  fn main() { // at the end of a line\n\
                  \x20   let a = 1 /* inside /* nested */ a line */ + 2\n\
                  \x20   // inside a block\n\
                  \x20   println(f\"{a}\")\n\
                  }\n";
    assert_eq!(
        comments(source),
        vec![
            "// before the item",
            "/// the item's documentation",
            "// at the end of a line",
            "/* inside /* nested */ a line */",
            "// inside a block",
        ]
    );
}

#[test]
fn slashes_in_a_string_or_a_dsl_body_are_no_comment() {
    let source = "fn page(a: String) -> String {\n\
                  \x20   let url = \"https://example.org\"\n\
                  \x20   return dsl html {\n\
                  \x20       <a href=\"https://example.org\">{a}</a> // not one\n\
                  \x20   } eod\n\
                  }\n\
                  fn main() {\n\
                  \x20   println(page(\"x\")) // one\n\
                  }\n";
    assert_eq!(comments(source), vec!["// one"]);
}
