//! **The prover reads through a loop** (ADR-361 D1-D5, D7; issue #640 steps
//! 3-6): a claim past a `while` that changes a whole number it reads is asked
//! once more through the loop's invariant, which closed forms and intervals
//! find and three queries check.

use crate::common;
use std::path::Path;
use std::process::{Command, Output};

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

fn through(h: &Held) -> bool {
    matches!(h, Held::ProvedThrough(_))
}

fn at_run_time(h: &Held) -> bool {
    matches!(h, Held::AtRunTime(_))
}

/// ADR-361 §1, the owner's program: both claims about `b` are proved.
#[test]
fn the_owners_program_is_proved_through_its_loop() {
    let source = "fn main(a: int) {\n\
                  \x20   assert(a > 0)\n\
                  \x20   assert(a <= 10)\n\
                  \x20   let mut i = 0\n\
                  \x20   let mut total = 0\n\
                  \x20   while i < a {\n\
                  \x20       i = i + 1\n\
                  \x20       total = total + 10\n\
                  \x20   }\n\
                  \x20   let b = 4 * example(a) + total\n\
                  \x20   let c = (0 * 4 + 1)\n\
                  \x20   assert(c == 1)\n\
                  \x20   assert(b > 40)\n\
                  \x20   assert(b < 181)\n\
                  \x20   let safety_buffer = (0..<181).collect()\n\
                  \x20   let d = safety_buffer[b]\n\
                  \x20   println(f\"Hello, Nikaia! {b} {c} {d}\")\n\
                  }\n\
                  fn positive(width: int) -> int {\n\
                  \x20   assert(width > 0)\n\
                  \x20   return width\n\
                  }\n\
                  fn example(a: int) -> int {\n\
                  \x20   let whole = 10 + a\n\
                  \x20   return positive(whole)\n\
                  }\n";
    let found = held(source);
    assert!(through(&found[3]) && through(&found[4]), "{found:#?}");
    let Held::ProvedThrough(how) = &found[3] else {
        unreachable!()
    };
    assert!(
        how.contains("10 * i == total") && how.contains("closed forms"),
        "{how}"
    );
}

/// A `break` is a way out of its own: what holds past the loop is what holds
/// at either, so the counter's end value is not claimed.
#[test]
fn a_break_keeps_the_end_value_from_being_claimed() {
    let source = "fn main(a: int) {\n\
                  \x20   assert(a > 0)\n\
                  \x20   let mut i = 0\n\
                  \x20   while i < a {\n\
                  \x20       if i == 3 {\n\
                  \x20           break\n\
                  \x20       }\n\
                  \x20       i = i + 1\n\
                  \x20   }\n\
                  \x20   assert(i <= a)\n\
                  \x20   assert(i == a)\n\
                  }\n";
    let found = held(source);
    assert!(through(&found[1]), "{found:#?}");
    assert!(at_run_time(&found[2]), "{found:#?}");
}

#[test]
fn a_countdown_ends_at_zero() {
    let source = "fn main(n: int) {\n\
                  \x20   assert(n >= 0)\n\
                  \x20   let mut k = n\n\
                  \x20   while k > 0 {\n\
                  \x20       k = k - 1\n\
                  \x20   }\n\
                  \x20   assert(k == 0)\n\
                  }\n";
    let found = held(source);
    assert!(through(&found[1]), "{found:#?}");
}

/// Two counters with different steps are tied by their closed forms.
#[test]
fn two_counters_with_different_steps() {
    let source = "fn main() {\n\
                  \x20   let mut i = 0\n\
                  \x20   let mut j = 100\n\
                  \x20   while i < 10 {\n\
                  \x20       i = i + 1\n\
                  \x20       j = j - 3\n\
                  \x20   }\n\
                  \x20   assert(i == 10)\n\
                  \x20   assert(j == 70)\n\
                  }\n";
    let found = held(source);
    assert!(found.iter().all(through), "{found:#?}");
}

/// Intervals, widened to the condition (D5.2): `x` ends within one step past
/// 100.
#[test]
fn an_interval_bounds_a_counter_past_its_condition() {
    let source = "fn main() {\n\
                  \x20   let mut x = 0\n\
                  \x20   while x < 100 {\n\
                  \x20       x = x + 3\n\
                  \x20   }\n\
                  \x20   assert(x <= 102)\n\
                  }\n";
    let found = held(source);
    assert!(through(&found[0]), "{found:#?}");
    let Held::ProvedThrough(how) = &found[0] else {
        unreachable!()
    };
    assert!(how.contains("x <= 102"), "{how}");
}

/// A loop no strategy reads leaves the claim open, and says which loop and
/// which names (D7); nothing false is proved.
#[test]
fn a_loop_no_strategy_reads_names_its_loop() {
    let source = "fn main(a: int) {\n\
                  \x20   let mut x = 0\n\
                  \x20   while x < a {\n\
                  \x20       x = x * 2 + 1\n\
                  \x20   }\n\
                  \x20   assert(x == 7)\n\
                  }\n";
    let found = held(source);
    let [Held::AtRunTime(why)] = found.as_slice() else {
        panic!("{found:#?}")
    };
    assert!(why.contains("the loop that changes `x`"), "{why}");
}

/// A name a turn changes is unknown inside the loop and past it, unless the
/// invariant says otherwise: a fact from before the loop does not survive it.
#[test]
fn a_fact_from_before_the_loop_does_not_survive_it() {
    let source = "fn main(a: int) {\n\
                  \x20   let mut x = 0\n\
                  \x20   while x < a {\n\
                  \x20       assert(x == 0)\n\
                  \x20       x = x + 1\n\
                  \x20   }\n\
                  \x20   assert(x == 0)\n\
                  }\n";
    let found = held(source);
    assert!(found.iter().all(at_run_time), "{found:#?}");
}

/// A mutable whole number changed in a branch is either side's value after
/// it (D9): the old value alone is not proved.
#[test]
fn a_branch_that_assigns_leaves_either_value() {
    let source = "fn main(a: int) {\n\
                  \x20   let mut x = 0\n\
                  \x20   if a > 3 {\n\
                  \x20       x = 5\n\
                  \x20   }\n\
                  \x20   assert(x == 0)\n\
                  }\n";
    let found = held(source);
    assert!(found.iter().all(at_run_time), "{found:#?}");
}

/// A `for` over a range is the `while` it means (ADR-361 D1): its binding
/// starts at the range's start and goes up by one each turn.
#[test]
fn a_sum_over_an_exclusive_range_is_proved_past_the_loop() {
    let source = "fn main(n: int) {\n\
                  \x20   assert(n >= 0)\n\
                  \x20   let mut total = 0\n\
                  \x20   for i in 0..<n {\n\
                  \x20       total = total + 3\n\
                  \x20   }\n\
                  \x20   assert(total == 3 * n)\n\
                  }\n";
    let found = held(source);
    assert!(through(&found[1]), "{found:#?}");
}

/// An inclusive range turns once more: its end is the last value.
#[test]
fn an_inclusive_range_counts_its_end() {
    let source = "fn main() {\n\
                  \x20   let mut total = 0\n\
                  \x20   for i in 1..10 {\n\
                  \x20       total = total + 2\n\
                  \x20   }\n\
                  \x20   assert(total == 20)\n\
                  }\n";
    let found = held(source);
    assert!(through(&found[0]), "{found:#?}");
}

/// A `break` in a `for` is a way out of its own: the bound is proved, the
/// count reaching the end is not.
#[test]
fn a_break_in_a_for_keeps_the_end_value_from_being_claimed() {
    let source = "fn main(n: int) {\n\
                  \x20   assert(n >= 0)\n\
                  \x20   let mut count = 0\n\
                  \x20   for i in 0..<n {\n\
                  \x20       if i == 3 {\n\
                  \x20           break\n\
                  \x20       }\n\
                  \x20       count = count + 1\n\
                  \x20   }\n\
                  \x20   assert(count <= n)\n\
                  \x20   assert(count == n)\n\
                  }\n";
    let found = held(source);
    assert!(through(&found[1]), "{found:#?}");
    assert!(at_run_time(&found[2]), "{found:#?}");
}

/// A `for` over a list is no counter the prover follows: the claim stays
/// open.
#[test]
fn a_for_over_a_list_stays_open() {
    let source = "fn main() {\n\
                  \x20   let xs = [1, 2, 3]\n\
                  \x20   let mut total = 0\n\
                  \x20   for x in xs {\n\
                  \x20       total = total + 1\n\
                  \x20   }\n\
                  \x20   assert(total == 3)\n\
                  }\n";
    let found = held(source);
    assert!(found.iter().all(at_run_time), "{found:#?}");
}

/// A claim the loop makes false is never proved through it.
#[test]
fn a_claim_a_for_makes_false_is_not_proved() {
    let source = "fn main(n: int) {\n\
                  \x20   assert(n >= 0)\n\
                  \x20   let mut total = 0\n\
                  \x20   for i in 0..<n {\n\
                  \x20       total = total + 3\n\
                  \x20   }\n\
                  \x20   assert(total == 3 * n + 1)\n\
                  }\n";
    let found = held(source);
    assert!(
        !found
            .iter()
            .any(|h| through(h) || matches!(h, Held::Proved)),
        "{found:#?}"
    );
}

/// A package whose one claim is proved through its loop.
const LOOPED: &str = "fn main() {\n\
                      \x20   let mut total = 0\n\
                      \x20   for i in 0..<10 {\n\
                      \x20       total = total + 2\n\
                      \x20   }\n\
                      \x20   assert(total == 20)\n\
                      \x20   println(f\"{total}\")\n\
                      }\n";

fn looped_package(purpose: &str) -> std::path::PathBuf {
    let dir = common::scratch_dir(purpose);
    std::fs::create_dir_all(dir.join("src")).expect("the package");
    std::fs::write(
        dir.join("nikaia.toml"),
        "[package]\nname = \"lp\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");
    std::fs::write(dir.join("src/main.nika"), LOOPED).expect("a source");
    dir
}

fn build_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(dir)
        .args(args)
        .env("NIKAIA_PROVER_COUNTS", "1")
        .output()
        .expect("the nikaia binary runs")
}

/// `(searched, read)`: the invariants a build searched for and read from
/// `nikaia.proofs`, as `NIKAIA_PROVER_COUNTS` says them.
fn invariants(out: &Output) -> (usize, usize) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    let line = stderr
        .lines()
        .find(|l| l.starts_with("prover: "))
        .unwrap_or_else(|| panic!("the counts: {stderr}"));
    let number = |before: &str| -> usize {
        let head = line.split(before).next().expect("the counts");
        head.rsplit(' ')
            .next()
            .expect("a number")
            .parse()
            .expect("a number")
    };
    (
        number(" invariants searched"),
        number(" read from nikaia.proofs"),
    )
}

/// The `invariant` lines of the package's `nikaia.proofs`.
fn invariant_lines(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("nikaia.proofs"))
        .expect("the file is written")
        .lines()
        .filter(|l| l.split(' ').nth(1) == Some("invariant"))
        .map(str::to_string)
        .collect()
}

/// **ADR-361 D4**: the invariant used is kept in `nikaia.proofs` with its
/// loop's place and the strategy that found it; a second build reads it,
/// checks it with the three queries and searches nothing, and leaves the
/// file as it was - which `--locked` then accepts.
#[test]
fn an_invariant_is_kept_and_read_back_without_a_search() {
    let dir = looped_package("proofs-invariant");
    let first = build_in(&dir, &["build", "--no-cache"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let (searched, _) = invariants(&first);
    assert!(searched >= 1, "{}", String::from_utf8_lossy(&first.stderr));
    let written = std::fs::read_to_string(dir.join("nikaia.proofs")).expect("written");
    let lines = invariant_lines(&dir);
    assert_eq!(lines.len(), 1, "{written}");
    let words: Vec<&str> = lines[0].split(' ').collect();
    assert_eq!(words.len(), 4, "{written}");
    assert_eq!(words[0].len(), 64, "a SHA-256: {written}");
    assert_eq!(words[2], "src/main.nika:3:5", "the loop's place: {written}");
    assert!(
        words[3].contains("closed-forms=") || words[3].contains("intervals="),
        "{written}"
    );
    assert!(words[3].contains("total"), "{written}");

    let second = build_in(&dir, &["build", "--no-cache"]);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let (searched, read) = invariants(&second);
    assert!(
        searched == 0 && read >= 1,
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let again = std::fs::read_to_string(dir.join("nikaia.proofs")).expect("still there");
    assert_eq!(again, written);

    let locked = build_in(&dir, &["build", "--no-cache", "--locked"]);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        locked.status.success(),
        "{}",
        String::from_utf8_lossy(&locked.stderr)
    );
    assert_eq!(invariants(&locked).0, 0);
}

/// **ADR-361 D4**: an invariant broken by hand fails the three queries and
/// proves nothing: the build searches the loop again and writes the
/// invariant it finds, and `--locked` refuses the file as it refuses any
/// entry that does not check.
#[test]
fn a_broken_invariant_in_the_file_is_searched_again_or_refused() {
    let dir = looped_package("proofs-invariant-broken");
    let first = build_in(&dir, &["build", "--no-cache"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let written = std::fs::read_to_string(dir.join("nikaia.proofs")).expect("written");
    let line = invariant_lines(&dir).pop().expect("an invariant line");
    let words: Vec<&str> = line.split(' ').collect();
    // Holds at entry, and a turn does not keep it.
    let wrong = format!(
        "{} {} {} closed-forms=eq(total,0)",
        words[0], words[1], words[2]
    );
    let broken = written.replace(&line, &wrong);
    std::fs::write(dir.join("nikaia.proofs"), &broken).expect("rewritten");

    let locked = build_in(&dir, &["build", "--no-cache", "--locked"]);
    let stderr = String::from_utf8_lossy(&locked.stderr).to_string();
    assert!(!locked.status.success(), "{stderr}");
    assert!(stderr.contains("commit `nikaia.proofs`"), "{stderr}");
    assert!(invariants(&locked).0 >= 1, "searched again: {stderr}");
    assert_eq!(
        std::fs::read_to_string(dir.join("nikaia.proofs")).expect("left alone"),
        broken,
        "--locked writes nothing"
    );

    let again = build_in(&dir, &["build", "--no-cache"]);
    assert!(
        again.status.success(),
        "{}",
        String::from_utf8_lossy(&again.stderr)
    );
    let (searched, _) = invariants(&again);
    assert!(searched >= 1, "{}", String::from_utf8_lossy(&again.stderr));
    let repaired = std::fs::read_to_string(dir.join("nikaia.proofs")).expect("written");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(repaired, written);
}
