//! **`assert`, a claim** ([ADR-269](../../../docs/specification/adr/adr-269.md)
//! D2, D3): a prelude function the compiler knows, whose condition changes
//! nothing. Outside a test it is proved while the program is built where it
//! can be, and checked when it runs where it cannot
//! ([ADR-269](../../../docs/specification/adr/adr-269.md) D4); a program runs
//! here at both settings of `user_parallelism`.

mod common;

use std::process::{Command, Output};

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

/// Through the command line, as a user runs it: the file has a name, so an
/// abort names the `.nika` line (ADR-300) - which is half of what D2 says a
/// false claim reports.
fn run_with(purpose: &str, source: &str, parallel: &str, flags: &[&str]) -> Output {
    let dir = common::scratch_dir(&format!("assert-{purpose}"));
    let input = dir.join("claims.nika");
    std::fs::write(&input, source).expect("write the source");
    let path = dir.join("claims.rs");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "claims.nika", "--output", "claims.rs"])
        .args(["--user-parallelism", parallel])
        .args(flags)
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "{purpose} did not lower:\n{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let binary = dir.join("claims");
    let compiled = common::compile(
        &path,
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("run it");
    std::fs::remove_dir_all(&dir).ok();
    out
}

/// Runs at both settings and hands back what the program printed to each
/// stream, having checked it is the same at both. Built at
/// `unproved-claims = "warn"` (ADR-380 D2), where a claim not proved is
/// checked when the program runs, which is what these tests watch.
fn outcome(purpose: &str, source: &str) -> (bool, String, String) {
    let found: Vec<_> = findings(source)
        .into_iter()
        .filter(|f| f.code != "NK1300")
        .collect();
    assert!(found.is_empty(), "{purpose}: {found:#?}");
    outcome_with(purpose, source, &["--unproved-claims", "warn"])
}

/// **A program with a claim the compiler shows false** (ADR-269 D8): refused
/// as it is, and built with `--refuted-claims warn`, the bypass, so that the
/// check where the claim is reached can be seen to fail.
fn outcome_refuted(purpose: &str, source: &str) -> (bool, String, String) {
    let found = findings(source);
    assert!(
        !found.is_empty() && found.iter().all(|f| f.code == "NK1207"),
        "{purpose}: {found:#?}"
    );
    let dir = common::scratch_dir(&format!("assert-{purpose}-refused"));
    std::fs::write(dir.join("claims.nika"), source).expect("write the source");
    let refused = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "claims.nika", "--output", "claims.rs"])
        .output()
        .expect("the nikaia binary runs");
    let said = String::from_utf8_lossy(&refused.stderr);
    assert!(!refused.status.success(), "{purpose}: {said}");
    assert!(said.contains("error[NK1207]"), "{purpose}: {said}");
    std::fs::remove_dir_all(&dir).ok();
    outcome_with(purpose, source, &["--refuted-claims", "warn"])
}

fn outcome_with(purpose: &str, source: &str, flags: &[&str]) -> (bool, String, String) {
    let mut seen = Vec::new();
    for parallel in ["no", "yes"] {
        let out = run_with(purpose, source, parallel, flags);
        seen.push((
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        ));
    }
    assert_eq!(seen[0], seen[1], "{purpose}: the two settings disagree");
    seen.remove(0)
}

fn refused(source: &str, code: &str) -> Vec<String> {
    let found: Vec<_> = findings(source)
        .into_iter()
        .filter(|f| f.code == code)
        .collect();
    assert!(!found.is_empty(), "no {code} for:\n{source}");
    found
        .into_iter()
        .map(|f| format!("{} | {}", f.message, f.notes.join(" | ")))
        .collect()
}

/// **Claims the prover holds change nothing**
/// ([ADR-269](../../../docs/specification/adr/adr-269.md) D4): the program
/// runs to its end, at both settings, and no check was written for any of
/// them.
#[test]
fn claims_the_compiler_proves_let_the_program_run() {
    let source = "fn clamp(n: i64) -> i64 sync {\n\
                  \x20   return 0 if n < 0\n\
                  \x20   assert(n >= 0)\n\
                  \x20   return n\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let blank = 2\n\
                  \x20   assert(blank + 1 == 3)\n\
                  \x20   for i in 0..<3 {\n\
                  \x20       assert(i < 3 && !(i < 0))\n\
                  \x20   }\n\
                  \x20   println(f\"all held {clamp(0 - 4)}\")\n\
                  }\n";
    let (ok, stdout, stderr) = outcome("hold", source);
    assert!(ok, "{stderr}");
    assert_eq!(stdout, "all held 0\n");
}

/// **A false claim the prover cannot hold stops the program** (ADR-269 D4,
/// D2): at both settings, naming its line, its message, the claim as written
/// and its operand's value. After a loop the claim is not carried back to the
/// entry (ADR-269 D16), so it is checked where it stands.
#[test]
fn a_false_claim_stops_the_program_when_it_runs() {
    let (ok, stdout, stderr) = outcome(
        "false",
        "fn twice(n: i64) -> i64 sync {\n\
         \x20   let blank = n * 2\n\
         \x20   for i in 0..<1 {\n\
         \x20   }\n\
         \x20   assert(blank == 1; message: \"an empty line is blank\")\n\
         \x20   return blank\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(\"before\")\n\
         \x20   println(f\"{twice(1)}\")\n\
         }\n",
    );
    assert!(!ok, "{stdout}");
    assert_eq!(stdout, "before\n");
    assert!(
        stderr.contains("claims.nika:5")
            && stderr.contains("assertion failed: an empty line is blank")
            && stderr.contains("assert(blank == 1; message: \"an empty line is blank\")")
            && stderr.contains("blank is 2"),
        "{stderr}"
    );
}

/// **A claim carried back to the entry fails at the call that breaks it**
/// (ADR-269 D15, D20, D21), with the `assert`'s message, the condition at the
/// entry, the `assert` it came from and the parameter's value. The compiler
/// shows the call false, so the program is refused (D8) and built here with
/// the bypass.
#[test]
fn a_computed_precondition_fails_at_the_call() {
    let (ok, stdout, stderr) = outcome_refuted(
        "computed",
        "fn twice(n: i64) -> i64 sync {\n\
         \x20   let blank = n * 2\n\
         \x20   assert(blank == 1; message: \"an empty line is blank\")\n\
         \x20   return blank\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(\"before\")\n\
         \x20   println(f\"{twice(1)}\")\n\
         }\n",
    );
    assert!(!ok, "{stdout}");
    assert_eq!(stdout, "before\n");
    assert!(
        stderr.contains("claims.nika:9")
            && stderr.contains("assertion failed: an empty line is blank")
            && stderr.contains("precondition of `twice`: `n * 2 == 1`, from `assert(blank == 1)`")
            && stderr.contains("n is 1"),
        "{stderr}"
    );
}

/// **A call that does not prove a precondition checks it, and its failure
/// names the call** (ADR-269 D5): the caller broke the contract. `report`
/// cannot show `total > 0` either, so the claim is carried back to its entry
/// (D15) and the call that fails is `main`'s, which passed the `0` - shown
/// false while it is built (D8), so built here with the bypass.
#[test]
fn a_broken_precondition_names_the_caller() {
    let (ok, stdout, stderr) = outcome_refuted(
        "precondition",
        "fn percent(part: i64, whole: i64) -> i64 sync {\n\
         \x20   assert(whole > 0)\n\
         \x20   return part * 100 / whole\n\
         }\n\
         \n\
         fn report(done: i64, total: i64) -> i64 sync {\n\
         \x20   return percent(done, total)\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(f\"{report(1, 4)}\")\n\
         \x20   println(f\"{report(1, 0)}\")\n\
         }\n",
    );
    assert!(!ok, "{stdout}");
    assert_eq!(stdout, "25\n");
    assert!(
        stderr.contains("claims.nika:12")
            && stderr.contains(
                "precondition of `report`: `total > 0`, from `assert(whole > 0)` in `percent`"
            )
            && stderr.contains("total is 0"),
        "{stderr}"
    );
}

/// **The condition changes nothing** (D3): a call that touches output, one
/// that pauses, one that changes its receiver and one nothing describes are
/// each `NK1194`, named.
#[test]
fn a_condition_that_could_change_something_is_refused() {
    let noisy = refused(
        "fn noisy() -> bool {\n\
         \x20   println(\"said\")\n\
         \x20   return true\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   assert(noisy())\n\
         }\n",
        "NK1194",
    );
    assert!(noisy[0].contains("`noisy` touches"), "{noisy:?}");
    let pauses = refused(
        "use std::fs\n\
         \n\
         fn main() throws {\n\
         \x20   assert(fs::read_to_string(\"x\"; root: fs::Root::Anywhere) == \"\")\n\
         }\n",
        "NK1194",
    );
    assert!(pauses[0].contains("can pause"), "{pauses:?}");
    let changes = refused(
        "fn main() {\n\
         \x20   let mut xs: Vec[i64] = Vec()\n\
         \x20   assert(xs.push(1) == xs.push(2))\n\
         }\n",
        "NK1194",
    );
    assert!(
        changes[0].contains("changes what it is called on"),
        "{changes:?}"
    );
    let unknown = refused(
        "fn main() {\n\
         \x20   assert(frobnicate(1))\n\
         }\n",
        "NK1194",
    );
    assert!(
        unknown[0].contains("knows nothing about `frobnicate`"),
        "{unknown:?}"
    );
}

/// **One condition, one option** (D2): `NK1195` for anything else.
#[test]
fn an_assert_of_another_shape_is_refused() {
    let shapes = [
        ("assert(5)", "a literal of another type"),
        ("assert(true; msg: \"x\")", "no option called `msg`"),
        ("assert(true, false)", "hands it 2"),
        ("assert(true; message: 3)", "`message:` has to be text"),
    ];
    for (call, says) in shapes {
        let found = refused(&format!("fn main() {{\n    {call}\n}}\n"), "NK1195");
        assert!(found[0].contains(says), "{call}: {found:?}");
    }
}

/// **A function of the program's own called `assert` is that function**: the
/// prelude's is the one nothing else declared.
#[test]
fn a_program_may_declare_its_own_assert() {
    let source = "fn assert(ok: bool) {\n\
                  \x20   println(f\"mine: {ok}\")\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   assert(1 == 2)\n\
                  }\n";
    let (ok, stdout, stderr) = outcome("own", source);
    assert!(ok, "{stderr}");
    assert_eq!(stdout, "mine: false\n");
}

/// **`--asserts` names every claim and how it is held** (ADR-269 D7): proved,
/// a precondition and the calls that check it (D5), or checked at run time
/// with the reason - and how many of each. `twice > 2` reads `half(4)`, which
/// the build runs for it (ADR-359 D11): 2.
#[test]
fn asserts_reports_every_claim() {
    let source = "fn half(n: i64) -> i64 sync {\n\
                  \x20   assert(n >= 0; message: \"not negative\")\n\
                  \x20   return n / 2\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let k = half(4)\n\
                  \x20   let four = 4\n\
                  \x20   assert(four == 4)\n\
                  \x20   let twice: i64 = k * 2\n\
                  \x20   assert(twice > 2)\n\
                  \x20   println(f\"{k}\")\n\
                  }\n";
    let dir = common::scratch_dir("assert-report");
    std::fs::write(dir.join("claims.nika"), source).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "claims.nika", "--output", "claims.rs", "--asserts"])
        .output()
        .expect("the nikaia binary runs");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains(
            "asserts in claims.nika: 3 - not proved 0 (refused 0, checked at run time 0, \
             checked by a test 0); checked where the words arrive 0; proved 3 (proved 2, \
             preconditions proved at every call 1)\n\
             \x20 claims.nika:9  assert(four == 4)  proved\n\
             \x20 claims.nika:11  assert(twice > 2)  proved\n\
             \x20 claims.nika:2  assert(n >= 0; message: \"not negative\")  precondition of \
             `half`: proved at every call the compiler sees; any other reaches the entry \
             that checks it\n"
        ),
        "{stdout}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// **No `Ok(())` after a statement that never comes back** (0.0.240): a
/// `throws` function ending in `panic(…)` or `return` was lowered with an
/// `Ok(())` after it, and `rustc` warned *unreachable expression* about a file
/// nobody wrote. `nikaia test`'s own entry ends that way, which is where it
/// was found.
#[test]
fn a_body_that_leaves_is_not_followed_by_ok() {
    let source = "fn check(n: i64) throws {\n\
                  \x20   if n > 0 {\n\
                  \x20       return\n\
                  \x20   }\n\
                  \x20   panic(\"negative\")\n\
                  }\n\
                  \n\
                  fn done() throws {\n\
                  \x20   println(\"done\")\n\
                  \x20   return\n\
                  }\n\
                  \n\
                  fn main() throws {\n\
                  \x20   check(1)\n\
                  \x20   done()\n\
                  }\n";
    let dir = common::scratch_dir("assert-leaves");
    std::fs::write(dir.join("leaves.nika"), source).expect("write");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "leaves.nika", "--output", "leaves.rs"])
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let compiled = common::compile(
        &dir.join("leaves.rs"),
        &[
            "--crate-type",
            "bin",
            "-o",
            &dir.join("leaves").to_string_lossy(),
        ],
    );
    let said = String::from_utf8_lossy(&compiled.stderr);
    assert!(compiled.status.success(), "{said}");
    assert!(!said.contains("unreachable"), "{said}");
    std::fs::remove_dir_all(&dir).ok();
}

/// **The owner's program** (#632, #633): `example(2)` is 12, so `4 / example(2)`
/// is 0 and `assert(b > 10)` is false every time it is reached. The compiler
/// runs `example(2)` while it builds (ADR-359 D11), as a `comptime` runs, folds
/// the quotient (D5) and refuses the program with what `fixtures/` says.
#[test]
fn the_owners_program_is_refused_while_it_is_built() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let source = std::fs::read_to_string(fixtures.join("four_by_example.nika")).expect("read");
    let expected = std::fs::read_to_string(fixtures.join("four_by_example.stderr")).expect("read");
    let dir = common::scratch_dir("assert-four-by-example");
    std::fs::write(dir.join("four_by_example.nika"), source).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args([
            "lower",
            "four_by_example.nika",
            "--output",
            "four_by_example.rs",
        ])
        .output()
        .expect("the nikaia binary runs");
    assert!(!out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stderr), expected);
    std::fs::remove_dir_all(&dir).ok();
}

/// `--asserts` over a program, with `NO_COLOR` set and its output not a
/// terminal: the report as printed, without colour. At
/// `unproved-claims = "warn"`, so that a program with a claim not proved
/// builds and its report can be read.
fn asserts_report(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(&format!("assert-report-{name}"));
    std::fs::write(dir.join(format!("{name}.nika")), source).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .args([
            "lower",
            &format!("{name}.nika"),
            "--output",
            &format!("{name}.rs"),
            "--asserts",
            "--unproved-claims",
            "warn",
        ])
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// **The owner's program with a parameter** (ADR-359 D1, D2): `b` is
/// `4 * example(a)`, and `example`'s body returns `positive(10 + a)`, whose
/// body returns its argument - so with `a > 0`, `b > 40` is proved while the
/// program is built. `b > 50` is not: `a = 3` makes it true, so it is neither
/// proved nor refused, and stays a check. The report lists what is not proved
/// first, `main`'s preconditions next, and what the build proved last.
#[test]
fn the_owners_program_with_a_parameter_is_proved_through_the_bodies() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let source = std::fs::read_to_string(fixtures.join("owners_main.nika")).expect("read");
    let report = asserts_report("owners_main", &source);
    assert_eq!(
        report
            .lines()
            .take(5)
            .map(|l| format!("{l}\n"))
            .collect::<String>(),
        "asserts in owners_main.nika: 4 - not proved 0 (refused 0, checked at run time 0, checked by a test 0); checked where the words arrive 1; proved 3 (proved 2, preconditions proved at every call 1)\n\
         \x20 owners_main.nika:2  assert(a > 0)  precondition of `main`, checked where the words arrive\n\
         \x20 owners_main.nika:5  assert(c == 1)  proved\n\
         \x20 owners_main.nika:6  assert(b > 40)  proved\n\
         \x20 owners_main.nika:10  assert(whole > 0)  precondition of `positive`: proved at every call the compiler sees; any other reaches the entry that checks it\n"
    );
    let fifty = source.replace("assert(b>40)", "assert(b>50)");
    let report = asserts_report("owners_fifty", &fifty);
    assert!(
        report.contains("owners_fifty.nika:6  assert(b > 50)  checked at run time"),
        "{report}"
    );
    // The order: what is not proved, then `main`'s preconditions, then what
    // the build proved.
    let at_run_time = report.find("assert(b > 50)").expect("listed");
    let arrives = report.find("assert(a > 0)").expect("listed");
    let proved = report.find("assert(c == 1)").expect("listed");
    assert!(at_run_time < arrives && arrives < proved, "{report}");
    assert!(!report.contains('\u{1b}'), "no colour: {report:?}");
}

/// **`main`'s precondition carried back from a callee has a row** (ADR-269
/// D15, ADR-360 D3): without `assert(a > 0)`, `positive`'s claim becomes
/// `10 + a > 0` at `main`'s entry, checked where the words arrive.
#[test]
fn a_precondition_of_main_carried_back_from_a_callee_is_listed() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let source = std::fs::read_to_string(fixtures.join("owners_main.nika"))
        .expect("read")
        .replace("   assert(a>0)\n", "");
    let report = asserts_report("owners_carried", &source);
    assert!(
        report.contains("  owners_carried.nika  10 + a > 0  precondition of `main` carried back from `assert(whole > 0)`, checked where the words arrive\n"),
        "{report}"
    );
    assert!(
        report.contains("assert(b > 40)  checked at run time"),
        "{report}"
    );
}
