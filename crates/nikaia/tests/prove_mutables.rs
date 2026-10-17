//! **The prover follows a `let mut` through each assignment** (ADR-361 D9;
//! issue #642 steps 1-2): an assignment gives the name a new value over the
//! old one, and after an `if` that changes it the name is either side's
//! value. A binding handed to a call or changed from a lambda stays out.

mod common;

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;
use nikaia::prove::Held;

/// How each `assert` is held, in the order they stand.
fn held(source: &str) -> Vec<Held> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .facts
        .claims
        .into_values()
        .filter_map(|c| c.held)
        .collect()
}

fn proved(h: &Held) -> bool {
    matches!(h, Held::Proved | Held::ProvedThrough(_))
}

fn at_run_time(h: &Held) -> bool {
    matches!(h, Held::AtRunTime(_))
}

/// `x = x + 1`: the new `x` is the old one plus 1.
#[test]
fn an_assignment_is_the_old_value_plus_one() {
    let source = "fn main() {\n\
                  \x20   let mut x = 0\n\
                  \x20   x = x + 1\n\
                  \x20   assert(x == 1)\n\
                  }\n";
    let found = held(source);
    assert!(found.iter().all(proved), "{found:#?}");
}

/// Two assignments in a row, the second a `*` by a constant.
#[test]
fn assignments_in_a_row_are_followed() {
    let source = "fn main() {\n\
                  \x20   let mut x = 0\n\
                  \x20   x = 1\n\
                  \x20   x = x * 3\n\
                  \x20   assert(x == 3)\n\
                  }\n";
    let found = held(source);
    assert!(found.iter().all(proved), "{found:#?}");
}

/// `+=`, `-=` and `*=` by a constant are assignments over the old value,
/// and a binding read from the old value keeps it.
#[test]
fn compound_assignments_are_followed() {
    let source = "fn main(a: int) {\n\
                  \x20   assert(a > 0 && a < 100)\n\
                  \x20   let mut x = a\n\
                  \x20   let y = x\n\
                  \x20   x += 2\n\
                  \x20   x *= 3\n\
                  \x20   x -= 1\n\
                  \x20   assert(x == 3 * y + 5)\n\
                  }\n";
    let found = held(source);
    assert!(found[1..].iter().all(proved), "{found:#?}");
}

/// After an `if`/`else` that changes `x` on both sides, `x` is either
/// side's value: within both, but not the one side's.
#[test]
fn a_branch_leaves_either_value() {
    let source = "fn main(c: bool) {\n\
                  \x20   let mut x = 0\n\
                  \x20   if c {\n\
                  \x20       x = 1\n\
                  \x20   } else {\n\
                  \x20       x = 2\n\
                  \x20   }\n\
                  \x20   assert(x >= 1 && x <= 2)\n\
                  \x20   assert(x == 1)\n\
                  }\n";
    let found = held(source);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(proved(&found[0]), "{found:#?}");
    assert!(at_run_time(&found[1]), "{found:#?}");
}

/// An `if` without `else` keeps the old value on the other side, and the
/// side's value is told by its condition.
#[test]
fn an_if_without_else_keeps_the_old_value() {
    let source = "fn main(a: int) {\n\
                  \x20   let mut x = 0\n\
                  \x20   if a > 3 {\n\
                  \x20       x = x + 5\n\
                  \x20   }\n\
                  \x20   assert(x == 0 || x == 5)\n\
                  \x20   assert(a > 3 || x == 0)\n\
                  }\n";
    let found = held(source);
    assert!(found.iter().all(proved), "{found:#?}");
}

/// A name the branch binds itself does not speak for the one outside.
#[test]
fn a_name_a_branch_binds_stays_inside() {
    let source = "fn main(a: int) {\n\
                  \x20   let t = a\n\
                  \x20   let mut x = 0\n\
                  \x20   if a > 3 {\n\
                  \x20       let t = 7\n\
                  \x20       x = t\n\
                  \x20   }\n\
                  \x20   assert(t == 7)\n\
                  }\n";
    let found = held(source);
    assert!(found.iter().all(|h| !proved(h)), "{found:#?}");
}

/// After assignments, a loop is still read through its invariant.
#[test]
fn a_loop_after_assignments_is_read_through_its_invariant() {
    let source = "fn main(a: int) {\n\
                  \x20   assert(a > 0 && a <= 10)\n\
                  \x20   let mut i = 0\n\
                  \x20   i = i + 1\n\
                  \x20   i = i - 1\n\
                  \x20   let mut total = 0\n\
                  \x20   while i < a {\n\
                  \x20       i = i + 1\n\
                  \x20       total = total + 10\n\
                  \x20   }\n\
                  \x20   assert(total == 10 * a)\n\
                  }\n";
    let found = held(source);
    assert!(proved(&found[1]), "{found:#?}");
}

/// A binding handed to a call, or changed from a lambda, stays out: another
/// piece of code could change it while the prover believes a value.
#[test]
fn a_binding_handed_on_stays_out() {
    let source = "fn bump(mut n: int) { n = n + 1 }\n\
                  fn main() {\n\
                  \x20   let mut x = 0\n\
                  \x20   x = x + 1\n\
                  \x20   bump(x)\n\
                  \x20   assert(x == 1)\n\
                  \x20   let mut y = 0\n\
                  \x20   y = y + 1\n\
                  \x20   let f = fn() { y = 5 }\n\
                  \x20   f()\n\
                  \x20   assert(y == 1)\n\
                  }\n";
    let found = held(source);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found.iter().all(|h| !proved(h)), "{found:#?}");
}
