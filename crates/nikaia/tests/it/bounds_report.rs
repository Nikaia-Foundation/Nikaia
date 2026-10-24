//! **`--bounds`: every check the optimizations may drop, and what became of
//! it** (#389). Proved sites say so; a checked one names the shape that
//! stopped the walk, with the issue that would supply the fact.

use nikaia::bounds::{BoundsChecks, OverflowChecks};
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn report(source: &str, bounds: BoundsChecks, overflow: OverflowChecks) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    nikaia::bounds_report::report(
        &parsed, source, "p.nika", &own, &library, bounds, overflow, false,
    )
}

const SOURCE: &str = "\
struct Grid {
    width: i64,
}

fn scaled(p: i64, q: i64) -> i64 {
    return p * q
}

fn main() {
    let xs: Vec[i64] = [1, 2, 3]
    let mut total: i64 = 0
    for i in 0..<xs.len() {
        total = total + xs[i]
    }
    let g = Grid { width: 4 }
    let a = xs[0] * g.width
    println(f\"{total} {a} {scaled(2, 3)}\")
}
";

#[test]
fn every_site_is_listed_with_what_became_of_it() {
    let out = report(SOURCE, BoundsChecks::Removed, OverflowChecks::Removed);
    let line = |text: &str| {
        out.lines()
            .find(|l| l.contains(text))
            .unwrap_or_else(|| panic!("no line for `{text}`:\n{out}"))
            .to_string()
    };
    assert!(out.starts_with("bounds in p.nika: "), "{out}");
    assert!(
        out.lines()
            .any(|l| l.split_whitespace().nth(1) == Some("xs[i]") && l.ends_with("proved")),
        "{out}"
    );
    assert!(line("total + xs[i]").contains("checked"), "{out}");
    // A field of a name is a name of the walk (ADR-306 D17): what is left
    // is the product.
    assert!(line("xs[0] * g.width").contains("#386"), "{out}");
    assert!(line("p * q").contains("#386"), "{out}");
    assert!(
        out.lines()
            .next()
            .is_some_and(|l| l.ends_with("indexes on, operations on")),
        "{out}"
    );
}

/// **What is proved does not depend on the build** (ADR-306 D14): a build
/// that removes no check is reported with the same proofs, and the first line
/// says it writes every check.
#[test]
fn a_build_that_removes_nothing_proves_the_same_and_says_so() {
    let kept = report(SOURCE, BoundsChecks::Kept, OverflowChecks::Kept);
    let removed = report(SOURCE, BoundsChecks::Removed, OverflowChecks::Removed);
    assert!(
        kept.lines()
            .next()
            .is_some_and(|l| l.ends_with("indexes off, operations off")),
        "{kept}"
    );
    assert_eq!(
        kept.lines().skip(1).collect::<Vec<_>>(),
        removed.lines().skip(1).collect::<Vec<_>>()
    );
}

/// #709: a word read of a `Bytes` is a site of its own, proved under the
/// guard its cut needs (ADR-320 D12).
#[test]
fn a_bytes_word_is_a_site() {
    let source = "\
fn word(t: String, i: i64) -> u64 {
    let b = t.bytes
    if i >= 0 && i + 8 <= b.len() { return b.u64_le(i) }
    return b.u32_le(i) as u64
}

fn main() {
    println(f\"{word(\"abcdefghij\", 2)}\")
}
";
    let out = report(source, BoundsChecks::Removed, OverflowChecks::Removed);
    let line = |text: &str| {
        out.lines()
            .find(|l| l.contains(text))
            .unwrap_or_else(|| panic!("no line for `{text}`:\n{out}"))
            .to_string()
    };
    assert!(line("b.u64_le(i)").ends_with("proved"), "{out}");
    assert!(line("b.u32_le(i)").contains("checked"), "{out}");
}

/// **A mask by a `comptime` is a mask by its value** (#700 d03): `h & (WIDTH -
/// 1)` is proved inside an array of `WIDTH` as `h & 1023` is, typed or open.
#[test]
fn a_mask_by_a_constant_is_proved_as_the_literal_is() {
    let source = "comptime WIDTH: i64 = 1024\n\
         comptime OPEN = 1024\n\
         fn main() {\n\
         \x20   let a: Array[i64, 1024] = [0; 1024]\n\
         \x20   let h: i64 = 77 * 31\n\
         \x20   println(f\"{a[h & (WIDTH - 1)]} {a[h & 1023]} {a[h & (OPEN - 1)]}\")\n\
         }\n";
    let out = report(source, BoundsChecks::Removed, OverflowChecks::Removed);
    for site in ["a[h & WIDTH - 1]", "a[h & 1023]", "a[h & OPEN - 1]"] {
        let line = out
            .lines()
            .find(|l| l.contains(site))
            .unwrap_or_else(|| panic!("no line for `{site}`:\n{out}"));
        assert!(line.trim_end().ends_with("proved"), "{out}");
    }
}

/// The line `--bounds` writes for `site`.
fn site_line<'a>(out: &'a str, site: &str) -> &'a str {
    out.lines()
        .find(|l| l.contains(site))
        .unwrap_or_else(|| panic!("no line for `{site}`:\n{out}"))
        .trim_end()
}

/// **An `if` expression lies where both its arms do**: a value bound from
/// one is in the join of its arms' ranges, each arm under its condition.
#[test]
fn an_index_bound_from_an_if_expression_is_proved_by_the_join_of_its_arms() {
    let source = "fn main() {\n\
         \x20   let a: Array[i64, 16] = [0; 16]\n\
         \x20   let h: i64 = 77 * 31\n\
         \x20   let i = if h > 3 { h & 7 } else { 15 }\n\
         \x20   let j = if h > 3 { h & 7 } else { 16 }\n\
         \x20   let k = if h < 16 && h >= 0 { h } else { 0 }\n\
         \x20   println(f\"{a[i]} {a[j]} {a[k]}\")\n\
         }\n";
    let out = report(source, BoundsChecks::Removed, OverflowChecks::Removed);
    assert!(site_line(&out, "a[i]").ends_with("proved"), "{out}");
    assert!(site_line(&out, "a[j]").contains("checked"), "{out}");
    assert!(site_line(&out, "a[k]").ends_with("proved"), "{out}");
}

/// **A `comptime` condition takes one arm**: an `if` on a `bool` the build
/// computed has the facts of the arm it takes, as a value and as a
/// statement; the other arm is not in the program.
#[test]
fn an_if_on_a_comptime_bool_has_the_facts_of_the_arm_it_takes() {
    let source = "comptime WIDE: bool = 64 > 32\n\
         comptime NARROW: bool = 1 > 2\n\
         fn main() {\n\
         \x20   let a: Array[i64, 16] = [0; 16]\n\
         \x20   let h: i64 = 77 * 31\n\
         \x20   let i = if WIDE { h & 15 } else { 99 }\n\
         \x20   let j = if NARROW { 99 } else { h & 15 }\n\
         \x20   let k = if !NARROW { h & 15 } else { 99 }\n\
         \x20   let m = if WIDE { 99 } else { h & 15 }\n\
         \x20   let mut n = 0\n\
         \x20   if WIDE {\n\
         \x20       n = h & 15\n\
         \x20   } else {\n\
         \x20       n = 99\n\
         \x20   }\n\
         \x20   println(f\"{a[i]} {a[j]} {a[k]} {a[m]} {a[n]}\")\n\
         }\n";
    let out = report(source, BoundsChecks::Removed, OverflowChecks::Removed);
    for site in ["a[i]", "a[j]", "a[k]", "a[n]"] {
        assert!(site_line(&out, site).ends_with("proved"), "{site}: {out}");
    }
    assert!(site_line(&out, "a[m]").contains("checked"), "{out}");
}
