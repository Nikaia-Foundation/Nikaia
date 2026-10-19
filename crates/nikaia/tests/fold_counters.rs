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

// --- D3 -------------------------------------------------------------------

/// 1BRC: `count` grows by one a step and the merge sums it, so both of its
/// checks are proved; `sum` grows by a temperature, which is not a literal,
/// and keeps its checks (that is D4's).
#[test]
fn the_1brc_count_is_proved_and_its_sum_checked() {
    let out = report(&brc());
    assert!(proved(&out, "self.count += 1"), "{out}");
    assert!(proved(&out, "self.count += other.count"), "{out}");
    assert!(!proved(&out, "self.sum += …"), "{out}");
    assert!(!proved(&out, "self.sum += other.sum"), "{out}");
}

/// A method the step reaches that is also called outside the fold keeps its
/// checks: there it runs where no step count holds.
#[test]
fn a_method_also_called_outside_the_fold_keeps_its_checks() {
    let source = changed(
        &brc(),
        "    let totals = Measurements::file(data)\n",
        "    let totals = Measurements::file(data)\n    let mut extra = Stats(1)\n    extra.add(2)\n    println(f\"{extra.count}\")\n",
    );
    let out = report(&source);
    assert!(!proved(&out, "self.count += 1"), "{out}");
    assert!(!proved(&out, "self.count += other.count"), "{out}");
}

/// A write inside a loop has no fixed amount a step.
#[test]
fn an_increment_inside_a_loop_keeps_its_checks() {
    let source = changed(
        &brc(),
        "        self.count += 1\n    }\n\n    fn merge",
        "        for i in 0..<2 {\n            self.count += 1\n        }\n    }\n\n    fn merge",
    );
    let out = report(&source);
    assert!(!proved(&out, "self.count += 1"), "{out}");
    assert!(!proved(&out, "self.count += other.count"), "{out}");
}

/// A merge that keeps the larger counter is neither a sum nor one of the two
/// as D3 reads a merge.
#[test]
fn a_merge_that_takes_the_larger_counter_keeps_the_checks() {
    let source = changed(
        &brc(),
        "        self.count += other.count\n",
        "        if other.count > self.count { self.count = other.count }\n",
    );
    let out = report(&source);
    assert!(!proved(&out, "self.count += 1"), "{out}");
}

/// The program prints the same with the proved checks dropped, and the
/// counter is written without its check.
#[test]
fn the_1brc_output_is_unchanged_without_the_counters_checks() {
    let source = brc();
    let parsed = parse_to_ast(&source).expect("the example parses");
    let build = Build {
        overflow: OverflowChecks::Removed,
        ..Build::default()
    };
    let rust = emit_program(&parsed, build).expect("lowers").rust;
    assert!(
        rust.contains("self.count = <i64>::wrapping_add(self.count, 1"),
        "the counter keeps its check:\n{rust}"
    );
    assert!(
        !rust.contains("self.sum = <i64>::wrapping_add"),
        "the sum lost its check"
    );
    let dir = common::scratch_dir("fold-counters");
    let file = dir.join("brc.rs");
    std::fs::write(&file, &rust).expect("write the Rust");
    let binary = dir.join("brc");
    let out = common::compile(
        &file,
        &[
            "--crate-type",
            "bin",
            "-o",
            binary.to_str().expect("utf-8 path"),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let input = dir.join("measurements.txt");
    std::fs::write(
        &input,
        "Hamburg;12.0\nBulawayo;8.9\nHamburg;-3.4\nPalembang;38.8\nBulawayo;22.1\nHamburg;0.0\n",
    )
    .expect("write the input");
    let ran = std::process::Command::new(&binary)
        .arg(&input)
        .output()
        .expect("runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout).trim(),
        "{Bulawayo=8.9/15.5/22.1, Hamburg=-3.4/2.9/12.0, Palembang=38.8/38.8/38.8}"
    );
}

const TALLY: &str = "\
grammar Lines {
    @frame(boundary: \"\\n\")
    rule LINE -> i64 = digit+ frame_end { 1 }
    entry rule file -> Tally = fold(LINE, Tally { lines: 0, total: 0 }, fn(acc, n) { acc.lines += 1
        acc.total -= 2 })
}

struct Tally {
    lines: i64,
    total: i64,
}

fn main() {
    let t = Lines::file(\"1\\n2\\n\") catch {
        return
    }
    println(f\"{t.lines} {t.total}\")
}
";

/// A sequential `fold` whose step writes the accumulator's fields itself:
/// both grow by a fixed amount a step, up or down.
#[test]
fn a_fold_whose_lambda_counts_is_proved() {
    let out = report(TALLY);
    assert!(proved(&out, "acc.lines += 1"), "{out}");
    assert!(proved(&out, "acc.total -= 2"), "{out}");
}
