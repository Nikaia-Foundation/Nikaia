//! **A path of fields is a name of the bounds walk** ([ADR-306] D17, #711):
//! `t.slots`, `p.end`, `self.at` carry the facts a guard gives them while
//! nothing can change them, a precondition may read them ([ADR-269] D5, D9),
//! a counter that only grows keeps its lower bound, and an `Array[T, N]` is
//! `N` long.
//!
//! [ADR-306]: ../../../docs/specification/adr/adr-306.md
//! [ADR-269]: ../../../docs/specification/adr/adr-269.md

use std::process::Command;

use nikaia::bounds::{BoundsChecks, OverflowChecks};
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn report(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    nikaia::bounds_report::report(
        &parsed,
        source,
        "p.nika",
        &own,
        &library,
        BoundsChecks::Removed,
        OverflowChecks::Removed,
        false,
    )
}

/// The report's line for the site at `line` that reads `text`.
fn site(out: &str, line: usize, text: &str) -> String {
    let at = format!("{line:>6}  {text} ");
    out.lines()
        .find(|l| l.starts_with(&at))
        .unwrap_or_else(|| panic!("no line {line} for `{text}`:\n{out}"))
        .to_string()
}

fn proved(out: &str, line: usize, text: &str) {
    assert!(
        site(out, line, text).ends_with("proved"),
        "{line} `{text}`:\n{out}"
    );
}

fn checked(out: &str, line: usize, text: &str) {
    assert!(
        site(out, line, text).contains("checked"),
        "{line} `{text}`:\n{out}"
    );
}

const TABLES: &str = "\
struct Table {
    slots: Vec[i64],
    used: i64,
}

struct Piece {
    start: i64,
    end: i64,
}

fn get(t: Table, i: i64) -> i64 {
    return 0 if i < 0 || i >= t.slots.len()
    return t.slots[i]
}

fn get2(t: Table) -> i64 {
    return 0 if t.used < 0 || t.used >= t.slots.len()
    return t.slots[t.used]
}

fn scan(p: Piece, b: Vec[i64]) -> i64 {
    return 0 if p.start < 0 || p.end > b.len()
    let mut sum = 0
    for k in p.start..<p.end {
        sum += b[k]
    }
    return sum
}

fn scan_locals(p: Piece, b: Vec[i64]) -> i64 {
    let start = p.start
    let end = p.end
    return 0 if start < 0 || end > b.len()
    let mut sum = 0
    for k in start..<end {
        sum += b[k]
    }
    return sum
}
";

/// The issue's probes: a guard on a path proves the index it guards.
#[test]
fn a_guarded_path_proves_its_index() {
    let out = report(TABLES);
    proved(&out, 13, "t.slots[i]");
    proved(&out, 18, "t.slots[t.used]");
    proved(&out, 25, "b[k]");
    proved(&out, 36, "b[k]");
}

const CHANGED: &str = "\
struct Table {
    slots: Vec[i64],
    used: i64,
}

fn kept(t: Table, i: i64) -> i64 {
    let mut u = t
    return 0 if i < 0 || i >= u.slots.len()
    return u.slots[i]
}

fn after_pop(t: Table, i: i64) -> i64 {
    let mut u = t
    return 0 if i < 0 || i >= u.slots.len()
    u.slots.pop()
    return u.slots[i]
}

fn shrink(mut t: Table) {
    t.slots = []
}

fn after_call(t: Table, i: i64) -> i64 {
    let mut u = t
    return 0 if i < 0 || i >= u.slots.len()
    shrink(u)
    return u.slots[i]
}

fn twice(f: fn() -> i64? sync) {
    f()
    f()
}

fn in_lambda(t: Table, i: i64) -> i64 {
    let mut u = t
    return 0 if i < 0 || i >= u.slots.len()
    twice(fn() { u.slots.pop() })
    return u.slots[i]
}

fn written(t: Table) -> i64 {
    let mut u = t
    return 0 if u.used < 0 || u.used >= u.slots.len()
    u.used = 100
    return u.slots[u.used]
}
";

/// A fact about a path goes where the path may change: a method that
/// shrinks it, a call handed it to change, a lambda that changes it, a write.
#[test]
fn a_fact_about_a_path_goes_where_the_path_may_change() {
    let out = report(CHANGED);
    proved(&out, 9, "u.slots[i]");
    checked(&out, 16, "u.slots[i]");
    checked(&out, 27, "u.slots[i]");
    checked(&out, 39, "u.slots[i]");
    checked(&out, 46, "u.slots[u.used]");
}

const SCANNER: &str = "\
struct Scanner {
    c: Vec[scalar],
    at: i64,
}

impl Scanner {
    fn pair(ref self) -> bool {
        if self.at >= 0 && self.at + 1 < self.c.len() {
            return self.c[self.at] == self.c[self.at + 1]
        }
        return false
    }

    fn step(ref mut self) {
        self.at += 1
    }

    fn after_step(ref mut self) -> bool {
        if self.at >= 0 && self.at < self.c.len() {
            self.step()
            return self.c[self.at] == 'x'
        }
        return false
    }
}

fn main() {
    let mut s = Scanner { c: ['a', 'a', 'x'], at: 0 }
    println(f\"{s.pair()} {s.after_step()}\")
}
";

/// `self.at + 1 < self.c.len()` proves both reads; a method that changes
/// its receiver drops what was known of its fields.
#[test]
fn a_scanner_reads_inside_what_its_cursor_says() {
    let out = report(SCANNER);
    proved(&out, 9, "self.c[self.at]");
    proved(&out, 9, "self.c[self.at + 1]");
    checked(&out, 21, "self.c[self.at]");
}

const COUNTED: &str = "\
fn counter(b: Vec[i64], start: i64) -> i64 {
    return 0 if start < 0
    let mut i = start
    let mut sum = 0
    while i < b.len() {
        sum += b[i]
        i += 1
    }
    return sum
}

fn fixed(a: Array[i64, 8], i: i64) -> i64 {
    return a[i & 7]
}

fn fixed_loop(a: Array[i64, 8]) -> i64 {
    let mut s = 0
    for k in 0..<a.len() {
        s += a[k]
    }
    return s
}

fn falls(b: Vec[i64], start: i64) -> i64 {
    return 0 if start < 0
    let mut i = start
    let mut sum = 0
    while i < b.len() {
        sum += b[i]
        i -= 1
    }
    return sum
}
";

/// A counter that only grows keeps `i >= start` beside a sum that does not
/// grow; an `Array[T, N]` is `N` long and `x & 7` is in `0..=7`.
#[test]
fn a_growing_counter_and_a_fixed_array_prove_their_reads() {
    let out = report(COUNTED);
    proved(&out, 6, "b[i]");
    proved(&out, 13, "a[i & 7]");
    proved(&out, 19, "a[k]");
    checked(&out, 29, "b[i]");
}

const PRECONDITION: &str = "\
struct Piece {
    start: i64,
    end: i64,
}

fn total(p: Piece, b: Vec[i64]) -> i64 {
    assert(p.start >= 0 && p.end <= b.len())
    let mut sum = 0
    for k in p.start..<p.end {
        sum += b[k]
    }
    return sum
}

fn main() {
    let b = [1, 2, 3, 4]
    let p = Piece { start: 1, end: 3 }
    println(f\"{total(p, b)}\")
    let mut n = 0
    for k in 0..<b.len() {
        n += b[k]
    }
    let q = Piece { start: 0, end: n - 6 }
    println(f\"{total(q, b)}\")
}
";

/// `nikaia --asserts lower`, then the program run: the report and what it
/// printed.
fn asserted(purpose: &str, source: &str) -> (String, String) {
    let dir = common::scratch_dir(&format!("field-paths-{purpose}"));
    std::fs::write(dir.join("p.nika"), source).expect("write the source");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["--asserts", "lower", "p.nika", "--output", "p.rs"])
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let binary = dir.join("p");
    let compiled = common::compile(
        &dir.join("p.rs"),
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary).output().expect("run it");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        String::from_utf8_lossy(&lowered.stdout).to_string(),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

/// A precondition over fields: the body's reads are proved by it, a caller
/// that builds the value from a literal proves it, and one whose field
/// nothing bounds checks it at the call.
#[test]
fn a_precondition_over_fields_is_proved_at_a_literal_caller() {
    let out = report(PRECONDITION);
    proved(&out, 10, "b[k]");
    let (asserts, printed) = asserted("precondition", PRECONDITION);
    assert!(
        asserts.contains("precondition of `total`: one call checks it when it runs"),
        "{asserts}"
    );
    assert_eq!(printed, "5\n10\n");
}

mod common;
