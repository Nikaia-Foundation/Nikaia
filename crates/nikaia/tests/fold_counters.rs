//! **A fold over a grammar's input takes no more steps than the input has
//! room for; its counters are proved from that**
//! ([ADR-367](../../../docs/specification/adr/adr-367.md), #432): the least
//! width of a rule (D1), the input's extent (D2), and a field that grows by a
//! fixed amount a step, proved where it fits (D3).

mod common;

use nikaia::bounds::{BoundsChecks, OverflowChecks};
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::fold_bound::{Extent, LENGTH_BOUND, least_width, most_steps};
use nikaia::parser::parse_to_ast;

fn brc() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/1brc.nika");
    std::fs::read_to_string(path).expect("examples/1brc.nika")
}

/// `source` with `from` replaced once, which must be there.
fn changed(source: &str, from: &str, to: &str) -> String {
    assert_eq!(source.matches(from).count(), 1, "`{from}` once");
    source.replace(from, to)
}

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
    )
}

fn line(out: &str, text: &str) -> String {
    out.lines()
        .find(|l| l.contains(text))
        .unwrap_or_else(|| panic!("no line for `{text}`:\n{out}"))
        .to_string()
}

fn proved(out: &str, text: &str) -> bool {
    line(out, text).ends_with("proved")
}

// --- D1 -------------------------------------------------------------------

/// 1BRC's line: a name of no least width, `;`, a temperature of at least
/// three bytes (`0.0`), and the line end.
#[test]
fn a_1brc_measurement_takes_at_least_five_bytes() {
    let parsed = parse_to_ast(&brc()).expect("parses");
    assert_eq!(least_width(&parsed, "Measurements", "MEASUREMENT"), Some(5));
    assert_eq!(least_width(&parsed, "Measurements", "TENTHS"), Some(3));
    assert_eq!(least_width(&parsed, "Measurements", "NAME"), Some(0));
}

const WIDTHS: &str = "\
grammar Widths {
    rule EMPTY -> String = s:text(\"a\"*) { s }
    rule NESTED -> i64 = \"ab\" n:NESTED { n } | \"abc\" { 1 }
    rule THREE -> i64 = digit{3} \"\\n\" { 1 }
    entry rule file -> i64 = n:THREE { n }
}

fn main() {
}
";

/// A rule that may match nothing is 0 wide, and a step still takes a byte.
#[test]
fn a_rule_that_may_match_nothing_is_zero_wide() {
    let parsed = parse_to_ast(WIDTHS).expect("parses");
    assert_eq!(least_width(&parsed, "Widths", "EMPTY"), Some(0));
    assert_eq!(
        most_steps(Extent::Known, 0),
        Some(LENGTH_BOUND - 1),
        "max(w, 1)"
    );
    assert_eq!(least_width(&parsed, "Widths", "THREE"), Some(4));
    assert_eq!(most_steps(Extent::Known, 4), Some((LENGTH_BOUND - 1) / 4));
}

/// A rule that reaches itself takes the recursive call at nothing.
#[test]
fn a_recursive_rule_counts_its_own_call_as_nothing() {
    let parsed = parse_to_ast(WIDTHS).expect("parses");
    assert_eq!(least_width(&parsed, "Widths", "NESTED"), Some(2));
}

// --- D2 -------------------------------------------------------------------

/// An input of unbounded extent - a stream - has no step count, so nothing
/// is proved from one.
#[test]
fn an_unbounded_input_gives_no_step_count() {
    assert_eq!(most_steps(Extent::Unbounded, 5), None);
    assert_eq!(most_steps(Extent::Unbounded, 0), None);
}
