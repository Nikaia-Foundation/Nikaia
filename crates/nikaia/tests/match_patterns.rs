//! The six pattern shapes
//! ([ADR-291](../../../docs/specification/adr/adr-291.md) D10-D12).
//!
//! `match` matched a literal, a path with bindings and the catch-all, and
//! nothing else. The absence was visible in the corpus: `examples/calc/src/main.nika`
//! matched `step.0` and then read `step.1` in every arm, because it could not
//! match `step`.
//!
//! Five of the six are Rust's and lower verbatim. The sixth — the **range** —
//! needed the spelling decided first, because a pattern's range includes both
//! ends and `..` meant the other thing until D4 was built.

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
        .filter(|f| f.code != "NK1261")
        .collect()
}

fn lowered(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Build::default())
        .expect("it lowers")
        .rust
}

const OP: &str = "enum Op { Plus, Minus, Times }\n";

/// **A tuple pattern**, which is the shape `calc.nika` wanted.
#[test]
fn a_tuple_pattern_reads_the_pair() {
    let source = "fn describe(point: (i64, i64)) -> ref String {\n\
                  \x20   return match point {\n\
                  \x20       (0, 0) => \"origin\",\n\
                  \x20       else => \"elsewhere\",\n\
                  \x20   }\n\
                  }\n\
                  fn main() { println(f\"{describe((0, 0))}\") }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    assert!(
        lowered(source).contains("(0, 0) => \"origin\""),
        "{}",
        lowered(source)
    );
}

/// **An or-pattern**, and every alternative binds the same names.
#[test]
fn an_or_pattern_is_one_arm() {
    let source = "fn describe(point: (i64, i64)) -> ref String {\n\
                  \x20   return match point {\n\
                  \x20       (0, y) | (y, 0) => \"on an axis\",\n\
                  \x20       else => \"elsewhere\",\n\
                  \x20   }\n\
                  }\n\
                  fn main() { println(f\"{describe((0, 1))}\") }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    assert!(
        lowered(source).contains("(0, y) | (y, 0) =>"),
        "{}",
        lowered(source)
    );
}

/// **`NK1155`: the alternatives bind different names.** The rule is what keeps
/// the arm's body answerable — a name the body reads has to be bound whichever
/// alternative matched.
#[test]
fn alternatives_that_bind_different_names_are_refused() {
    let found: Vec<_> = findings(
        "fn f(point: (i64, i64)) -> i64 {\n\
         \x20   return match point {\n\
         \x20       (0, y) | (x, 0) => y,\n\
         \x20       else => 0,\n\
         \x20   }\n\
         }\n",
    )
    .into_iter()
    .filter(|f| f.code == "NK1155")
    .collect();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains('x') && found[0].message.contains('y'),
        "{found:#?}"
    );
}

/// **`NK1253`: a positional variant's pattern names every part**
/// `E::A(_)` against `A(i64, i64)` reached rustc.
#[test]
fn a_variant_pattern_with_too_few_parts_is_refused() {
    let found: Vec<_> = findings(
        "enum E { A(i64, i64), B }\n\
         fn f(e: E) -> i64 { return match e { E::A(_) => 1, else => 0 } }\n",
    )
    .into_iter()
    .filter(|f| f.code == "NK1253")
    .collect();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains("1 part") && found[0].message.contains("carries 2"),
        "{found:#?}"
    );
    let fine = findings(
        "enum E { A(i64, i64), B }\n\
         fn f(e: E) -> i64 { return match e { E::A(x, _) => x, else => 0 } }\n",
    );
    assert!(fine.iter().all(|f| f.code != "NK1253"), "{fine:#?}");
}

/// **An arm of a statement `match` is a statement**:
/// an arm that is a call handing back a value drops it with `;`, so it agrees
/// with `else => {}`.
#[test]
fn a_statement_arm_that_hands_back_a_value_drops_it() {
    let source = "use std::collections\n\
                  fn f(k: i64, mut seen: collections::BTreeMap[i64, i64]) {\n\
                  \x20   match k {\n\
                  \x20       1 => seen.insert(k, 2),\n\
                  \x20       else => {},\n\
                  \x20   }\n\
                  }\n\
                  fn main() { let mut m: collections::BTreeMap[i64, i64] = collections::BTreeMap()\n f(1, m)\n println(f\"{m.len()}\") }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    assert!(
        lowered(source).contains("1 => { seen.insert(k, 2); },"),
        "{}",
        lowered(source)
    );
}

/// **A guard is `if`** (D2), and it stands on the arm.
#[test]
fn a_guard_is_written_if() {
    let source = "fn describe(point: (i64, i64)) -> ref String {\n\
                  \x20   return match point {\n\
                  \x20       (x, y) if x == y => \"diagonal\",\n\
                  \x20       else => \"elsewhere\",\n\
                  \x20   }\n\
                  }\n\
                  fn main() { println(f\"{describe((1, 1))}\") }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    assert!(
        lowered(source).contains("(x, y) if x == y =>"),
        "{}",
        lowered(source)
    );
}

/// **A `match` as a guard is written in brackets, and the refusal says so**
/// (ADR-301 D4).
#[test]
fn a_match_in_a_guard_without_brackets_names_them() {
    let source = "fn small(n: i64) -> bool { return n < 3 }\n\
                  fn f(a: i64, b: i64) -> bool {\n\
                  \x20   return match a {\n\
                  \x20       n if match b { 1 => true, else => false } => small(n),\n\
                  \x20       else => false,\n\
                  \x20   }\n\
                  }\n";
    let error = parse_to_ast(source).expect_err("a bare `match` in a guard");
    let finding =
        nikaia::diagnostics::refused_finding(&error).expect("a parse error carries its finding");
    let shown = nikaia::diagnostics::render_finding(finding, "app.nika", source);
    assert!(shown.contains("written in parentheses"), "{shown}");
    let bracketed = source.replace(
        "if match b { 1 => true, else => false }",
        "if (match b { 1 => true, else => false })",
    );
    assert!(
        findings(&bracketed).is_empty(),
        "{:#?}",
        findings(&bracketed)
    );
}

/// **A guarded arm covers nothing**, which is Rust's reading and has to be this
/// one too: the pattern says which values reach the arm and the guard says
/// which of those it takes, so the rest reach the arms below.
#[test]
fn a_guarded_arm_does_not_cover() {
    let found: Vec<_> = findings(&format!(
        "{OP}fn word(o: Op) -> ref String {{\n\
         \x20   return match o {{\n\
         \x20       rest if true => \"anything\",\n\
         \x20   }}\n\
         }}\n"
    ))
    .into_iter()
    .filter(|f| f.code == "NK1151")
    .collect();
    assert_eq!(found.len(), 1, "{found:#?}");
}

/// **A range, inclusive at both ends** (D3), and `..=` below because that is
/// how the language below spells the same set.
#[test]
fn a_range_pattern_includes_both_ends() {
    let source = "fn band(code: i64) -> ref String {\n\
                  \x20   return match code {\n\
                  \x20       200..299 => \"ok\",\n\
                  \x20       else => \"other\",\n\
                  \x20   }\n\
                  }\n\
                  fn main() { println(f\"{band(204)}\") }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    assert!(
        lowered(source).contains("200..=299 =>"),
        "{}",
        lowered(source)
    );
}

/// **`..<` is never written in a pattern** (D4): a pattern is one shape, and an
/// exclusive range is written by moving the end.
#[test]
fn an_exclusive_range_is_refused_in_a_pattern() {
    let source = "fn band(code: i64) -> ref String {\n\
                  \x20   return match code {\n\
                  \x20       200..<300 => \"ok\",\n\
                  \x20       else => \"other\",\n\
                  \x20   }\n\
                  }\n";
    let refused = parse_to_ast(source).expect_err("`..<` is not a pattern");
    let finding =
        nikaia::diagnostics::refused_finding(&refused).expect("a parse error carries its finding");
    let message = nikaia::diagnostics::render_finding(finding, "band.nika", source);
    assert!(message.contains("A pattern can't use `..<`."), "{message}");
    assert!(message.contains("200..299"), "{message}");
}

/// **A pattern inside a pattern**, and `..` for the fields this one does not
/// name.
#[test]
fn a_pattern_nests_and_a_struct_may_leave_the_rest() {
    let source = "struct Point { x: i64, y: i64 }\n\
                  enum Event { Click(Point), Quit }\n\
                  fn at(e: Event) -> i64 {\n\
                  \x20   return match e {\n\
                  \x20       Event::Click(Point { x, .. }) => x,\n\
                  \x20       Event::Quit => 0,\n\
                  \x20   }\n\
                  }\n\
                  fn main() { println(f\"{at(Event::Quit)}\") }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    assert!(
        lowered(source).contains("Event::Click(Point { x, .. }) => x"),
        "{}",
        lowered(source)
    );
}

/// **A struct pattern that names nothing** is every field and none of them.
#[test]
fn a_struct_pattern_may_name_no_field_at_all() {
    let source = "struct Point { x: i64, y: i64 }\n\
                  enum Event { Click(Point), Quit }\n\
                  fn any(e: Event) -> i64 {\n\
                  \x20   return match e {\n\
                  \x20       Event::Click(Point { .. }) => 1,\n\
                  \x20       Event::Quit => 0,\n\
                  \x20   }\n\
                  }\n\
                  fn main() { println(f\"{any(Event::Quit)}\") }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    assert!(
        lowered(source).contains("Event::Click(Point { .. }) => 1"),
        "{}",
        lowered(source)
    );
}

/// **An or-pattern covers every variant in it**, which is
/// [ADR-291](../../../docs/specification/adr/adr-291.md)'s question answered
/// by building the form it waited on: `Op::Plus | Op::Minus` is two cases and
/// one arm.
#[test]
fn an_or_pattern_covers_each_variant_it_names() {
    let source = format!(
        "{OP}fn word(o: Op) -> ref String {{\n\
         \x20   return match o {{\n\
         \x20       Op::Plus | Op::Minus => \"additive\",\n\
         \x20       Op::Times => \"times\",\n\
         \x20   }}\n\
         }}\n\
         fn main() {{ println(f\"{{word(Op::Plus)}}\") }}\n"
    );
    assert!(findings(&source).is_empty(), "{:#?}", findings(&source));

    // And one left out is still a case missing.
    let short = format!(
        "{OP}fn word(o: Op) -> ref String {{\n\
         \x20   return match o {{\n\
         \x20       Op::Plus | Op::Minus => \"additive\",\n\
         \x20   }}\n\
         }}\n"
    );
    let found: Vec<_> = findings(&short)
        .into_iter()
        .filter(|f| f.code == "NK1151")
        .collect();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].message.contains("Op::Times"), "{found:#?}");
}

/// **`calc.nika` matches the pair**, which is the corpus line this record
/// opened with.
#[test]
fn the_calculator_matches_the_pair() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/calc/src/main.nika");
    let source = std::fs::read_to_string(path).expect("calc.nika");
    assert!(source.contains("match step {"), "it still matches `step.0`");
    // The comment beside it *names* the old shape, which is the point of a
    // comment; what must be gone is the code.
    let code: Vec<&str> = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();
    assert!(
        !code.join("\n").contains("step.0"),
        "it still reads `step.0`"
    );
}

/// **`E::V(..)` matches all of a variant's values** (ADR-349 D1), on one value
/// and on three, without `NK1253`'s count and with no case missing, and
/// lowers to Rust's own `E::V(..)`.
#[test]
fn a_rest_matches_a_whole_payload() {
    let source = "enum Held { One(i64), Three(i64, String, bool), Gone }\n\
         fn f(h: Held) -> i64 { return match h { Held::One(..) => 1, Held::Three(..) => 3, Held::Gone => 0 } }\n";
    let found = findings(source);
    assert!(
        found
            .iter()
            .all(|f| f.code != "NK1253" && f.code != "NK1255" && f.code != "NK1151"),
        "{found:#?}"
    );
    let rust = lowered(source);
    assert!(rust.contains("Held::One(..)"), "{rust}");
    assert!(rust.contains("Held::Three(..)"), "{rust}");
}

/// **`NK1255`: no rest on a variant without values** (ADR-349 D2); a
/// positional rest beside other parts is the remaining values (ADR-355 D3).
#[test]
fn a_rest_on_a_unit_variant_is_refused_and_beside_a_part_matches() {
    let nk1255 = |source: &str| -> Vec<nikaia::check::Finding> {
        findings(source)
            .into_iter()
            .filter(|f| f.code == "NK1255")
            .collect()
    };
    let unit = nk1255(
        "enum E { A(i64), B }\n\
         fn f(e: E) -> i64 { return match e { E::B(..) => 1, else => 0 } }\n",
    );
    assert_eq!(unit.len(), 1, "{unit:#?}");
    assert!(unit[0].message.contains("no values"), "{unit:#?}");
    let source = "enum E { A(i64, i64), B }\n\
         fn f(e: E) -> i64 { return match e { E::A(x, ..) => x, else => 0 } }\n\
         fn g(t: (i64, i64, i64)) -> i64 { return match t { (.., z) => z } }\n";
    let found = findings(source);
    assert!(found.is_empty(), "{found:#?}");
    let rust = lowered(source);
    assert!(rust.contains("E::A(x, ..)"), "{rust}");
    assert!(rust.contains("(.., z)"), "{rust}");
}

/// **A list pattern** (ADR-355 D1, D2, D5): a slice pattern over the subject,
/// the named rest Rust's `rest @ ..`.
#[test]
fn a_list_pattern_lowers_to_a_slice_pattern() {
    let source = "fn f(xs: ref Vec[i64]) -> i64 {\n\
         return match xs { [] => 0, [x] => x, [a, .., z] => a + z, [h, ..rest] => h + rest.len(), else => 0 }\n}\n";
    let found = findings(source);
    assert!(found.is_empty(), "{found:#?}");
    let rust = lowered(source);
    assert!(rust.contains("match &(xs)[..]"), "{rust}");
    assert!(rust.contains("[a, .., z]"), "{rust}");
    assert!(rust.contains("[h, rest @ ..]"), "{rust}");
}

/// **An arm that keeps an element takes it** (ADR-355 D5, Part I 3.4, #622):
/// over an owned list the slice lends it and the arm opens with its clone;
/// over a lent list the element is a view, and keeping it is refused. **And
/// a name only the guard reads** is copied out for the guard alone.
#[test]
fn a_kept_element_is_taken_and_a_guard_reads_a_copy() {
    let owned = "fn f(xs: Vec[String]) -> Vec[String] {\n\
         let mut out: Vec[String] = []\n\
         match xs { [first, ..] => out.push(first), else => {} }\n\
         return out\n}\n\
         fn g(xs: ref Vec[i64]) -> i64 { return match xs { [a, ..] if a > 3 => 1, else => 0 } }\n";
    let found = findings(owned);
    assert!(found.is_empty(), "{found:#?}");
    let rust = lowered(owned);
    assert!(
        rust.contains("let first = first.clone(); out.push(first)"),
        "{rust}"
    );
    assert!(
        rust.contains("[a, ..] if { let a = *a; a > 3 } => "),
        "{rust}"
    );
    let lent = findings(
        "fn f(xs: ref Vec[String]) -> Vec[String] {\n\
         let mut out: Vec[String] = []\n\
         match xs { [first, ..] => out.push(first), else => {} }\n\
         return out\n}\n",
    );
    assert!(lent.iter().any(|f| f.code == "NK1102"), "{lent:#?}");
}

/// **`NK1258`: one `..` per pattern** (ADR-355 D4); **a list `match` needs
/// `else`** (D5, `NK1151`).
#[test]
fn two_rests_and_a_list_match_without_else_are_refused() {
    let two = findings(
        "fn f(xs: ref Vec[i64]) -> i64 { return match xs { [a, .., b, ..] => a, else => 0 } }\n",
    );
    assert!(two.iter().any(|f| f.code == "NK1258"), "{two:#?}");
    let open =
        findings("fn f(xs: ref Vec[i64]) -> i64 { return match xs { [] => 0, [h, ..] => h } }\n");
    assert!(open.iter().any(|f| f.code == "NK1151"), "{open:#?}");
}

/// **A named rest is a view** (ADR-355 D2): keeping it as a list is refused.
#[test]
fn a_named_rest_cannot_be_kept_as_a_list() {
    let found = findings(
        "struct Keep { tail: Vec[i64] }\n\
         fn f(xs: Vec[i64]) -> Keep { return match xs { [_, ..rest] => Keep { tail: rest }, else => Keep { tail: [] } } }\n",
    );
    assert!(found.iter().any(|f| f.code == "NK1106"), "{found:#?}");
}
