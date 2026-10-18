//! **Every `assert` outside a test is proved while the program is built
//! where it can be, and checked when the program runs where it cannot**
//! ([ADR-269](../../../docs/specification/adr/adr-269.md) D4). A proved claim
//! leaves nothing behind in the emitted Rust.
use nikaia::check::CodeOps;

mod common;

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
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

fn codes(source: &str) -> Vec<&'static str> {
    findings(source).iter().map(|f| f.code_str()).collect()
}

/// How each `assert` of the program is held, in the order they are written.
fn held(source: &str) -> Vec<nikaia::prove::Held> {
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

/// How many calls in the emitted Rust check a precondition where they
/// stand (ADR-269 D20): `(if !… , call).1`.
fn call_checks(rust: &str) -> usize {
    rust.matches("(if !").count() + rust.matches("({ if !").count()
}

fn the_one(source: &str, code: &str) -> nikaia::check::Finding {
    let found = findings(source);
    let mut matching: Vec<_> = found.iter().filter(|f| f.code == code).cloned().collect();
    assert_eq!(matching.len(), 1, "{found:#?}");
    matching.remove(0)
}

/// Lowers, compiles and runs; the program's output, and the Rust it came from.
fn ran(purpose: &str, source: &str) -> (String, String) {
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    let parsed = parse_to_ast(source).expect("the source parses");
    let rust = emit_program(&parsed, Build::default())
        .expect("it lowers")
        .rust;
    let dir = common::scratch_dir(purpose);
    let file = dir.join("main.rs");
    std::fs::write(&file, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let built = common::compile(&file, &["-o", &binary.to_string_lossy()]);
    assert!(
        built.status.success(),
        "{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&built.stderr)
    );
    let out = std::process::Command::new(&binary)
        .output()
        .expect("run it");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_dir_all(&dir).ok();
    (String::from_utf8_lossy(&out.stdout).to_string(), rust)
}

/// **D4, D9: a guard proves what follows it, and the proof costs nothing.**
/// `return 250 if speed > 250` leaves `speed <= 250` (ADR-276).
#[test]
fn a_guard_proves_the_claim_after_it_and_no_check_is_emitted() {
    let (out, rust) = ran(
        "prove-guard",
        "fn limit(speed: i64) -> i64 {\n\
         \x20   return 250 if speed > 250\n\
         \x20   assert(speed <= 250)\n\
         \x20   return speed\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(f\"{limit(300)} {limit(100)}\")\n\
         }\n",
    );
    assert_eq!(out, "250 100\n");
    assert!(!rust.contains("assertion_failed"), "{rust}");
}

/// **D9: the facts the prover reads** - an `if`'s branch, a `let`'s value, a
/// `for` over a range, `!=` ruled out by a guard, and integer tightening
/// (`x >= 5` proves `x > 4`).
#[test]
fn branches_lets_ranges_and_guards_are_facts() {
    let source = "fn f(x: i64, d: i64) -> i64 {\n\
                  \x20   return 0 if d == 0\n\
                  \x20   assert(d != 0)\n\
                  \x20   let y = x + 3\n\
                  \x20   assert(y - x == 3)\n\
                  \x20   if x > 10 {\n\
                  \x20       assert(y > 13)\n\
                  \x20   } else {\n\
                  \x20       assert(x <= 10)\n\
                  \x20   }\n\
                  \x20   for i in 0..<10 {\n\
                  \x20       assert(i >= 0 && i < 10)\n\
                  \x20   }\n\
                  \x20   return 0 if x < 5\n\
                  \x20   assert(x > 4)\n\
                  \x20   return x / d\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{f(12, 3)}\")\n\
                  }\n";
    assert!(codes(source).is_empty(), "{:#?}", findings(source));
}

/// **D4: a claim nothing shows is checked when the program runs**, and says
/// why it was not proved.
#[test]
fn a_claim_nothing_shows_is_checked_when_the_program_runs() {
    // After a loop the claim is not carried back to the entry (ADR-269 D16),
    // so it is no precondition either.
    let source = "fn f(x: i64) -> i64 {\n\
                  \x20   let y = x * 2\n\
                  \x20   for i in 0..<1 {\n\
                  \x20   }\n\
                  \x20   assert(y > 0)\n\
                  \x20   return y\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{f(1)}\")\n\
                  }\n";
    assert_eq!(
        held(source),
        [nikaia::prove::Held::AtRunTime(
            "nothing before it shows it".to_string()
        )]
    );
    let (out, rust) = ran("prove-at-run-time", source);
    assert_eq!(out, "2\n");
    assert!(
        rust.contains("assertion_failed(\"assert(y > 0)\""),
        "{rust}"
    );
}

/// **D9: a claim the prover cannot read is not guessed**: it is checked when
/// the program runs.
#[test]
fn a_claim_outside_what_the_prover_reads_is_checked() {
    let source = "fn main() {\n\
                  \x20   let name = \"ab\"\n\
                  \x20   assert(name == \"ab\")\n\
                  }\n";
    assert_eq!(
        held(source),
        [nikaia::prove::Held::AtRunTime(
            "it is not a comparison of whole numbers the prover reads".to_string()
        )]
    );
    let (_, rust) = ran("prove-unread", source);
    assert!(rust.contains("assertion_failed"), "{rust}");
}

/// **D5: a claim about parameters is the caller's to prove**: a literal
/// argument proves it, a guard proves it, and a call that shows nothing and
/// cannot carry it back - inside a loop (D16) - checks it at the call, with
/// its arguments in place of the parameters.
#[test]
fn a_parameter_claim_is_a_precondition_the_caller_proves() {
    let (out, rust) = ran(
        "prove-precondition",
        "fn percent(part: i64, whole: i64) -> i64 {\n\
         \x20   assert(whole > 0)\n\
         \x20   return part * 100 / whole\n\
         }\n\
         \n\
         fn report(done: i64, total: i64) -> i64 {\n\
         \x20   return 0 if total <= 0\n\
         \x20   return percent(done, total)\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(f\"{percent(1, 4)} {report(3, 4)} {report(3, 0)}\")\n\
         }\n",
    );
    assert_eq!(out, "25 75 0\n");
    assert_eq!(call_checks(&rust), 0, "{rust}");
    assert!(rust.contains("percent__unchecked(1, 4)"), "{rust}");

    let (out, rust) = ran(
        "prove-call-check",
        "fn percent(part: i64, whole: i64) -> i64 {\n\
         \x20   assert(whole > 0)\n\
         \x20   return part * 100 / whole\n\
         }\n\
         \n\
         fn report(done: i64, total: i64) -> i64 {\n\
         \x20   let mut sum = 0\n\
         \x20   for k in 0..<1 {\n\
         \x20       sum = percent(done, total)\n\
         \x20   }\n\
         \x20   return sum\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(f\"{report(3, 4)}\")\n\
         }\n",
    );
    assert_eq!(out, "75\n");
    assert!(
        rust.contains(
            "(if !((total.clone() as i128) > (0i128)) { nikaia_std::abort::assertion_failed("
        ),
        "{rust}"
    );
    assert_eq!(call_checks(&rust), 1, "{rust}");
}

/// **ADR-269 D20: a caller the prover doesn't see reaches the checked
/// entry**: a `pub fn` and a method have a precondition like any function,
/// and a function handed on as a value is its checked entry, while the calls
/// the prover saw prove theirs and take the unchecked one.
#[test]
fn a_caller_the_prover_does_not_see_reaches_the_checked_entry() {
    let source = "struct Counter {\n\
                  \x20   total: i64,\n\
                  }\n\
                  \n\
                  impl Counter {\n\
                  \x20   fn add(self, n: i64) -> i64 {\n\
                  \x20       assert(n > 0)\n\
                  \x20       return self.total + n\n\
                  \x20   }\n\
                  }\n\
                  \n\
                  pub fn half(n: i64) -> i64 {\n\
                  \x20   assert(n >= 0)\n\
                  \x20   return n / 2\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let c = Counter { total: 1 }\n\
                  \x20   let g = half\n\
                  \x20   println(f\"{half(4)} {g(6)} {c.add(2)}\")\n\
                  }\n";
    assert_eq!(
        held(source),
        [
            nikaia::prove::Held::Precondition("Counter::add".to_string(), 0),
            nikaia::prove::Held::Precondition("half".to_string(), 0),
        ]
    );
    let (out, rust) = ran("prove-entries", source);
    assert_eq!(out, "2 3 3\n");
    assert!(rust.contains("pub fn half__unchecked(n: i64)"), "{rust}");
    assert!(rust.contains("half__unchecked(4)"), "{rust}");
    assert!(rust.contains("let g = half;"), "{rust}");
    assert!(rust.contains("c.add(2)"), "{rust}");
    assert!(rust.contains("self.add__unchecked(n)"), "{rust}");
}

/// **D6: a claim about data from outside the program** is refused with where
/// it came from, and proved once a guard has checked it.
#[test]
fn data_from_outside_is_checked_by_a_guard_and_not_asserted() {
    let refusal = the_one(
        "use std::process\n\
         \n\
         fn main() throws {\n\
         \x20   let out = process::run(\"true\", [])\n\
         \x20   let code: i64 = out.code\n\
         \x20   assert(code == 0)\n\
         }\n",
        "NK1202",
    );
    assert!(
        refusal.notes[0].contains("`code` comes from outside the program"),
        "{refusal:#?}"
    );
    assert!(
        refusal
            .help
            .unwrap_or_default()
            .contains("Check it where it arrives")
    );

    let guarded = "use std::process\n\
                   \n\
                   fn main() throws {\n\
                   \x20   let out = process::run(\"true\", [])\n\
                   \x20   let code: i64 = out.code\n\
                   \x20   return if code != 0\n\
                   \x20   assert(code == 0)\n\
                   }\n";
    assert!(
        !codes(guarded).contains(&"NK1202"),
        "{:#?}",
        findings(guarded)
    );
}

/// **D11: a test's `assert` is the test's verdict**, checked when it runs and
/// never refused for want of a proof.
#[test]
fn a_tests_claim_is_not_the_provers() {
    let source = "fn double(n: i64) -> i64 {\n\
                  \x20   return n * 2\n\
                  }\n\
                  \n\
                  test \"doubling\" {\n\
                  \x20   assert(double(2) == 4)\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  }\n";
    assert!(
        !codes(source).contains(&"NK1202"),
        "{:#?}",
        findings(source)
    );
}

/// **A list's length is a number the prover reads** (ADR-269 D9, 0.0.273):
/// `xs.len()` of a binding that does not change, never negative, known for a
/// literal, and carried into a callee's precondition.
#[test]
fn a_length_is_a_variable_of_the_proof() {
    let source = "fn first(xs: Vec[i64]) -> i64 {\n\
                  \x20   assert(xs.len() > 0)\n\
                  \x20   return xs[0]\n\
                  }\n\
                  \n\
                  fn head_or(xs: Vec[i64], fallback: i64) -> i64 {\n\
                  \x20   return fallback if xs.len() == 0\n\
                  \x20   assert(xs.len() >= 1)\n\
                  \x20   return first(xs)\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let xs = [3, 4]\n\
                  \x20   assert(xs.len() == 2)\n\
                  \x20   println(f\"{first(xs)} {head_or(xs, 0)}\")\n\
                  }\n";
    assert!(codes(source).is_empty(), "{:#?}", findings(source));

    // And a call that shows nothing about the length, inside a loop where it
    // cannot be carried back, checks it with the argument's length in place
    // of the parameter's.
    let (out, rust) = ran(
        "prove-length-call",
        "fn first(xs: Vec[i64]) -> i64 {\n\
         \x20   assert(xs.len() > 0)\n\
         \x20   return xs[0]\n\
         }\n\
         \n\
         fn use_it(ys: Vec[i64]) -> i64 {\n\
         \x20   let mut r = 0\n\
         \x20   for k in 0..<1 {\n\
         \x20       r = first(ys)\n\
         \x20   }\n\
         \x20   return r\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(f\"{use_it([1])}\")\n\
         }\n",
    );
    assert_eq!(out, "1\n");
    assert!(
        rust.contains("(if !((ys.len() as i128) > (0i128))"),
        "{rust}"
    );
}

/// **A lambda keeps what holds around it**, and its own parameters are new
/// names: a guard outside proves a claim inside, and a parameter that shadows
/// an outer name knows nothing of it.
#[test]
fn a_lambda_sees_the_facts_around_it_and_not_through_its_parameters() {
    let source = "fn scale(xs: Vec[i64], k: i64) -> Vec[i64] {\n\
                  \x20   return [] if k <= 0\n\
                  \x20   return xs.map fn(x) {\n\
                  \x20       assert(k > 0)\n\
                  \x20       x * 2\n\
                  \x20   }\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{scale([1], 2).len()}\")\n\
                  }\n";
    assert!(
        !codes(source).contains(&"NK1202"),
        "{:#?}",
        findings(source)
    );

    let shadowed = "fn f(k: i64) -> Vec[i64] {\n\
                    \x20   return [] if k <= 0\n\
                    \x20   return [1, 2].map fn(k) {\n\
                    \x20       assert(k > 0)\n\
                    \x20       k\n\
                    \x20   }\n\
                    }\n\
                    \n\
                    fn main() {\n\
                    \x20   println(f\"{f(1).len()}\")\n\
                    }\n";
    assert!(
        matches!(
            held(shadowed).as_slice(),
            [nikaia::prove::Held::AtRunTime(_)]
        ),
        "{:#?}",
        held(shadowed)
    );
}

/// **A binding that reads the name it shadows teaches nothing about the new
/// one** (0.0.353): `let x = x + 1` was taken as the fact `x = x + 1`, which
/// is a contradiction, so every claim after it was "proved" and its check left
/// out - a false `assert` included. The same for a range that reads its own
/// binding.
#[test]
fn a_shadowing_binding_is_not_a_contradiction() {
    let source = "fn main() {\n\
                  \x20   let x = 1\n\
                  \x20   let x = x + 1\n\
                  \x20   assert(x == 0)\n\
                  \x20   let i = 3\n\
                  \x20   for i in 0..<i {\n\
                  \x20       assert(i > 5)\n\
                  \x20   }\n\
                  }\n";
    assert!(
        held(source)
            .iter()
            .all(|h| matches!(h, nikaia::prove::Held::AtRunTime(_))),
        "{:#?}",
        held(source)
    );
}

/// The program's warnings with one code.
/// **The claims the compiler shows false** (ADR-269 D8), each one an error:
/// a proof of a defect does not build.
fn shown_false(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    let found: Vec<_> = nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.code == "NK1207")
        .collect();
    assert!(
        found
            .iter()
            .all(|f| f.severity == nikaia::check::Severity::Error),
        "{found:#?}"
    );
    found
}

/// **ADR-269 D8: a claim the facts rule out is refused, with values**: a
/// proof of a defect does not build. Shown false only where it is false every
/// time it is reached - proved so, certificate and all - with a model of what
/// is known for the values. The bypass builds it as a check.
#[test]
fn a_claim_false_every_time_it_is_reached_is_refused_with_values() {
    let source = "fn twice(n: i64) -> i64 {\n\
                  \x20   return 0 if n < 0\n\
                  \x20   let m = n * 2\n\
                  \x20   assert(m < 0)\n\
                  \x20   return m\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let blank = 2\n\
                  \x20   assert(blank + 1 == 4)\n\
                  \x20   println(f\"{twice(3)}\")\n\
                  }\n";
    let found = shown_false(source);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert_eq!(
        found[0].message,
        "`m < 0` is false every time it is reached."
    );
    assert!(found[0].notes[0].ends_with("`m` is 0."), "{found:#?}");
    assert_eq!(
        found[1].message,
        "`blank + 1 == 4` is false every time it is reached."
    );
    assert!(found[1].notes[0].ends_with("`blank` is 2."), "{found:#?}");
    // Under the bypass, both are checks where they stand.
    assert!(
        held(source)
            .iter()
            .all(|h| matches!(h, nikaia::prove::Held::AtRunTime(why) if why.starts_with("it is false every time"))),
        "{:#?}",
        held(source)
    );
}

/// **A call that breaks a precondition every time** is refused at the call,
/// with the parameter the precondition reads as this call gives it.
#[test]
fn a_call_that_always_breaks_a_precondition_is_refused() {
    let found = shown_false(
        "fn percent(part: i64, whole: i64) -> i64 {\n\
         \x20   assert(whole > 0)\n\
         \x20   return part * 100 / whole\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let none = 0\n\
         \x20   println(f\"{percent(1, none)}\")\n\
         }\n",
    );
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0].message,
        "This call breaks `percent`'s precondition `whole > 0` every time it is reached."
    );
    assert_eq!(found[0].notes[0], "Here `whole` is 0.");
}

/// **A claim about an option is a precondition** (ADR-269 D5, #651), as one
/// about a subject parameter is: the call proves it with the value it names,
/// or with the default where it names none, and takes the unchecked entry; a
/// call that breaks it is refused, and so is one whose default does.
#[test]
fn an_options_claim_is_a_precondition_the_caller_proves() {
    let callee = "fn tipped(a: i64; t: i64 = 0) -> i64 {\n\
                  \x20   assert(t <= 30)\n\
                  \x20   return a + t\n\
                  }\n\
                  \n";
    let (out, rust) = ran(
        "prove-option-precondition",
        &format!("{callee}fn main() {{\n    println(f\"{{tipped(1; t: 5)}} {{tipped(1)}}\")\n}}\n"),
    );
    assert_eq!(out, "6 1\n");
    assert_eq!(call_checks(&rust), 0, "{rust}");
    let found = shown_false(&format!(
        "{callee}fn main() {{\n    println(f\"{{tipped(1; t: 50)}}\")\n}}\n"
    ));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0].message,
        "This call breaks `tipped`'s precondition `t <= 30` every time it is reached."
    );
    assert_eq!(found[0].notes[0], "Here `t` is 50.");
    let found = shown_false(
        "fn tipped(a: i64; t: i64 = 50) -> i64 {\n\
         \x20   assert(t <= 30)\n\
         \x20   return a + t\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(f\"{tipped(1)}\")\n\
         }\n",
    );
    assert_eq!(found.len(), 1, "{found:#?}");
}

/// **No refusal where a claim is only not proved**: `y > 0` after
/// `let y = x * 2` can hold, and a value of `x` the facts allow is not a value
/// the program is shown to reach.
#[test]
fn a_claim_that_can_hold_is_not_refused() {
    let source = "fn f(x: i64) -> i64 {\n\
                  \x20   let y = x * 2\n\
                  \x20   assert(y > 0)\n\
                  \x20   return y\n\
                  }\n\
                  \n\
                  fn g(n: i64) -> i64 {\n\
                  \x20   return f(n)\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{f(1)} {g(2)}\")\n\
                  }\n";
    assert!(shown_false(source).is_empty());
}

/// **ADR-269 D15 through a call: a callee's precondition this function
/// cannot show is its callers'**, carried back to its entry as an `assert`
/// of it at the call would be - across two functions here - so the caller
/// that breaks it is the one told, with its values and the `assert` it
/// came from, and the calls in between need no check of their own.
#[test]
fn a_callee_precondition_is_carried_back_through_the_call() {
    let source = "fn percent(part: i64, whole: i64) -> i64 {\n\
                  \x20   assert(whole > 0)\n\
                  \x20   return part * 100 / whole\n\
                  }\n\
                  \n\
                  fn half(x: i64) -> i64 {\n\
                  \x20   return percent(1, x - 2)\n\
                  }\n\
                  \n\
                  fn twice(y: i64) -> i64 {\n\
                  \x20   return half(y + 1)\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{twice(1)} {half(5)}\")\n\
                  }\n";
    let found = shown_false(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0].message,
        "This call breaks `twice`'s precondition `y + 1 - 2 > 0` every time it is reached."
    );
    assert_eq!(
        found[0].notes[..2],
        [
            "Here `y` is 1.".to_string(),
            "The precondition is `assert(whole > 0)` in `percent`, carried back to `twice`'s \
             entry."
                .to_string()
        ]
    );

    // Where every caller proves it, nothing is checked but the entries.
    let (out, rust) = ran(
        "prove-carried-through-calls",
        "fn percent(part: i64, whole: i64) -> i64 {\n\
         \x20   assert(whole > 0)\n\
         \x20   return part * 100 / whole\n\
         }\n\
         \n\
         fn half(x: i64) -> i64 {\n\
         \x20   return percent(1, x - 2)\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   println(f\"{half(4)} {half(102)}\")\n\
         }\n",
    );
    assert_eq!(out, "50 1\n");
    assert_eq!(call_checks(&rust), 0, "{rust}");
    assert!(rust.contains("percent__unchecked(1, x - 2)"), "{rust}");
}

/// **A call that comes back to its own function is not carried back**:
/// recursion would grow the precondition by one claim a turn. The call is
/// proved or checked where it stands, and the walk ends.
#[test]
fn a_recursive_call_is_not_carried_back() {
    let source = "fn down(n: i64) -> i64 {\n\
                  \x20   assert(n >= 0)\n\
                  \x20   return 0 if n == 0\n\
                  \x20   return down(n - 1)\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{down(3)}\")\n\
                  }\n";
    assert!(shown_false(source).is_empty());
    assert_eq!(
        held(source),
        [nikaia::prove::Held::Precondition("down".to_string(), 0)]
    );
}

/// **ADR-269 D8 for a claim false for some of the values that reach it**:
/// where what is known pins every name the claim depends on - a range's
/// counter, what a `let` computed from it - a value that breaks it is one the
/// program reaches, and the refusal shows it.
#[test]
fn a_claim_false_for_a_value_that_reaches_it_is_refused_with_values() {
    let source = "fn percent(part: i64, whole: i64) -> i64 {\n\
                  \x20   assert(whole > 0)\n\
                  \x20   return part * 100 / whole\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   for i in 0..<10 {\n\
                  \x20       println(f\"{percent(1, i)}\")\n\
                  \x20   }\n\
                  \x20   let n = 8\n\
                  \x20   for k in 0..<n {\n\
                  \x20       let j = k * 2\n\
                  \x20       assert(j < 14)\n\
                  \x20       println(f\"{j}\")\n\
                  \x20   }\n\
                  }\n";
    let found = shown_false(source);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert_eq!(
        found[0].message,
        "This call breaks `percent`'s precondition `whole > 0` when `i` is 0."
    );
    assert_eq!(found[0].notes[0], "Then `whole` is 0.");
    assert_eq!(
        found[1].message,
        "`j < 14` is false when `j` is 14, `k` is 7, `n` is 8."
    );
    // Under the bypass, a check where it stands.
    assert!(
        held(source).iter().any(|h| matches!(
            h,
            nikaia::prove::Held::AtRunTime(why) if why.starts_with("it is false when")
        )),
        "{:#?}",
        held(source)
    );
}

/// **No refusal where the facts are weaker than what reaches**: a `break`
/// before the call ends the loop before the value that would break it, a
/// guard rules it out, a loop over a parameter's range runs for whatever the
/// callers pass, and a condition the prover cannot read decides which turns
/// reach the call.
#[test]
fn a_value_the_facts_allow_but_nothing_shows_reaches_is_not_refused() {
    let source = "fn percent(part: i64, whole: i64) -> i64 {\n\
                  \x20   assert(whole > 0)\n\
                  \x20   return part * 100 / whole\n\
                  }\n\
                  \n\
                  fn over(n: i64) -> i64 {\n\
                  \x20   let mut sum = 0\n\
                  \x20   for i in 0..<n {\n\
                  \x20       sum += percent(1, i)\n\
                  \x20   }\n\
                  \x20   return sum\n\
                  }\n\
                  \n\
                  fn odd(x: i64) -> bool {\n\
                  \x20   return x % 2 == 1\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   for i in 0..<10 {\n\
                  \x20       break if i == 0\n\
                  \x20       println(f\"{percent(1, i - 1)}\")\n\
                  \x20   }\n\
                  \x20   for i in 0..<10 {\n\
                  \x20       continue if i == 0\n\
                  \x20       println(f\"{percent(1, i)}\")\n\
                  \x20   }\n\
                  \x20   for i in 0..<10 {\n\
                  \x20       if odd(i) {\n\
                  \x20           println(f\"{percent(1, i - 1)}\")\n\
                  \x20       }\n\
                  \x20   }\n\
                  \x20   println(f\"{over(0)}\")\n\
                  }\n";
    let found = shown_false(source);
    assert!(found.is_empty(), "{found:#?}");
}

/// **ADR-269 D15: a claim is carried back to the entry.** A parameter changed
/// before the claim (`let x = x + 1`), a value computed from one, a branch and
/// a guard: each claim is a precondition over the parameters, its callers
/// prove it, and no check is left in the program.
#[test]
fn a_claim_carried_back_to_the_entry_is_a_precondition() {
    let source = "fn f(x: i64, mode: i64) -> i64 {\n\
                  \x20   let y = x - 1\n\
                  \x20   if mode == 1 {\n\
                  \x20       assert(y > 0)\n\
                  \x20   }\n\
                  \x20   return y\n\
                  }\n\
                  \n\
                  fn g(x: i64) -> i64 {\n\
                  \x20   let x = x + 1\n\
                  \x20   assert(x > 3)\n\
                  \x20   return x\n\
                  }\n\
                  \n\
                  fn h(n: i64) -> i64 {\n\
                  \x20   return 0 if n < 0\n\
                  \x20   let m = n - 5\n\
                  \x20   assert(m >= 0)\n\
                  \x20   return m\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{f(5, 1)} {f(0, 2)} {g(3)} {h(7)} {h(0 - 1)}\")\n\
                  }\n";
    assert!(
        held(source)
            .iter()
            .all(|h| matches!(h, nikaia::prove::Held::Precondition(_, 0))),
        "{:#?}",
        held(source)
    );
    let (out, rust) = ran("prove-carried-back", source);
    assert_eq!(out, "4 -1 4 2 0\n");
    assert_eq!(call_checks(&rust), 0, "{rust}");
}

/// **A call that breaks a computed precondition** is told the condition at
/// the entry and the `assert` it came from (ADR-269 D21), and the call checks
/// the condition at the entry with its arguments in place (D7).
#[test]
fn a_computed_precondition_names_both_ends() {
    let source = "fn f(x: i64, mode: i64) -> i64 {\n\
                  \x20   let y = x - 1\n\
                  \x20   if mode == 1 {\n\
                  \x20       assert(y > 0)\n\
                  \x20   }\n\
                  \x20   return y\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   println(f\"{f(0, 1)}\")\n\
                  }\n";
    let found = shown_false(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        found[0].message,
        "This call breaks `f`'s precondition `mode == 1 → x - 1 > 0` every time it is reached."
    );
    assert_eq!(
        found[0].notes[..2],
        [
            "Here `mode` is 1, `x` is 0.".to_string(),
            "The precondition is `assert(y > 0)` in `f`, carried back to its entry.".to_string()
        ]
    );
    assert_eq!(
        held(source),
        [nikaia::prove::Held::Precondition("f".to_string(), 1)]
    );
}

/// **ADR-269 D17: a claim about the value returned, shown at every exit, is a
/// postcondition**, and a caller knows it of the result: `clamp`'s
/// `result >= 0` proves `a >= 0`, and a chain of them proves `t >= 0`. One
/// exit that does not show it - `wrong`'s `return -1` - and it is none, so
/// `b >= 0` stays a check.
#[test]
fn a_claim_shown_at_every_exit_is_a_postcondition() {
    use nikaia::prove::Held::Proved;
    let source = "fn clamp(n: i64) -> i64 {\n\
                  \x20   return 0 if n < 0\n\
                  \x20   assert(n >= 0)\n\
                  \x20   return n\n\
                  }\n\
                  \n\
                  fn wrong(n: i64) -> i64 {\n\
                  \x20   return 0 - 1 if n < 0\n\
                  \x20   let m = n\n\
                  \x20   assert(m >= 0)\n\
                  \x20   return m\n\
                  }\n\
                  \n\
                  fn twice_clamped(n: i64) -> i64 {\n\
                  \x20   let c = clamp(n)\n\
                  \x20   let d = c * 2\n\
                  \x20   assert(d >= 0)\n\
                  \x20   return d\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let a = clamp(0 - 5)\n\
                  \x20   assert(a >= 0)\n\
                  \x20   let b = wrong(5)\n\
                  \x20   assert(b >= 0)\n\
                  \x20   let t = twice_clamped(3)\n\
                  \x20   assert(t >= 0)\n\
                  \x20   println(f\"{a} {b} {t}\")\n\
                  }\n";
    let held = held(source);
    assert!(
        matches!(
            held.as_slice(),
            [Proved, Proved, Proved, Proved, Proved, Proved]
        ),
        "{held:#?}"
    );
    // `wrong` has no postcondition, but inside the package its body says
    // what `wrong(5)` returns (ADR-359 D1, D2): 5.
    let (out, rust) = ran("prove-postcondition", source);
    assert_eq!(out, "0 5 6\n");
    assert_eq!(rust.matches("assertion_failed(").count(), 0, "{rust}");
}

/// **What a postcondition is not shown by**: a `return` inside a lambda is
/// the lambda's, and a body that can end without a `return` has an exit no
/// claim stands before.
#[test]
fn an_exit_that_shows_nothing_makes_no_postcondition() {
    use nikaia::prove::Held::{AtRunTime, Proved};
    let source = "fn open_end(n: i64) -> i64 {\n\
                  \x20   if n > 0 {\n\
                  \x20       assert(n > 0)\n\
                  \x20       return n\n\
                  \x20   }\n\
                  \x20   println(\"none\")\n\
                  \x20   return 0 - 1\n\
                  }\n\
                  \n\
                  fn caller(x: i64) {\n\
                  \x20   let a = open_end(x)\n\
                  \x20   assert(a > 0)\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   caller(2)\n\
                  }\n";
    let held = held(source);
    assert!(
        matches!(held.as_slice(), [Proved, AtRunTime(_)]),
        "{held:#?}"
    );
}

/// **ADR-314 D3: a call inside a `let`'s value is a whole number of its own**:
/// what its callee ensures holds of it, so `10 + example(2)` reads as `10 +
/// r` with `r == 10 + 2`, and an arithmetic around a call is proved - or
/// shown false - as `let r = example(2)` alone already was.
#[test]
fn a_call_inside_a_lets_value_is_read_with_what_its_callee_ensures() {
    use nikaia::prove::Held::*;
    let source = "fn example(a: i64) -> i64 sync {\n\
                  \x20   return 10 + a\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let e = 10 + example(2)\n\
                  \x20   assert(e == 22)\n\
                  \x20   let f = example(example(1)) - 1\n\
                  \x20   assert(f == 20)\n\
                  \x20   println(f\"{e} {f}\")\n\
                  }\n";
    let held = held(source);
    assert!(matches!(held.as_slice(), [Proved, Proved]), "{held:#?}");
    let refuted = "fn example(a: i64) -> i64 sync {\n\
                   \x20   return 10 + a\n\
                   }\n\
                   \n\
                   fn main() {\n\
                   \x20   let b = 2 * example(2)\n\
                   \x20   assert(b > 30)\n\
                   \x20   println(f\"{b}\")\n\
                   }\n";
    let found = shown_false(refuted);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains("`b > 30` is false every time"),
        "{found:#?}"
    );
}

/// **D7: a claim the prover does not read names the part it could not read**:
/// a binding whose value is outside D9 - here a quotient - is named, rather
/// than the claim called something other than a comparison of whole numbers.
#[test]
fn a_claim_over_a_name_the_prover_does_not_read_names_it() {
    let source = "fn quotient(n: i64, m: i64) {\n\
                  \x20   let b = n / m\n\
                  \x20   assert(b > 10)\n\
                  \x20   println(f\"{b}\")\n\
                  }\n";
    assert_eq!(
        held(source),
        [nikaia::prove::Held::AtRunTime(
            "`b` is not a whole number the prover reads".to_string()
        )]
    );
}

/// **ADR-359 D5: a quotient or a remainder of two known numbers is its
/// value**, truncated toward zero as Part I 2.2 folds it (#632 step 1).
#[test]
fn a_quotient_of_two_literals_is_its_value() {
    let refuted = "fn main() {\n\
                   \x20   let b = 4 / 12\n\
                   \x20   assert(b > 10)\n\
                   \x20   println(f\"{b}\")\n\
                   }\n";
    let found = shown_false(refuted);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains("`b > 10` is false every time"),
        "{found:#?}"
    );
    assert!(
        found[0].notes.iter().any(|n| n.contains("`b` is 0")),
        "{found:#?}"
    );
    let proved = "fn main() {\n\
                  \x20   let q = -7 / 2\n\
                  \x20   let r = -7 % 2\n\
                  \x20   let s = 48 / 4\n\
                  \x20   assert(q == -3)\n\
                  \x20   assert(r == -1)\n\
                  \x20   assert(s == 12)\n\
                  \x20   assert(7 % 3 == 1)\n\
                  \x20   println(f\"{q} {r} {s}\")\n\
                  }\n";
    let answers = held(proved);
    assert!(
        answers
            .iter()
            .all(|h| matches!(h, nikaia::prove::Held::Proved))
            && answers.len() == 4,
        "{answers:#?}"
    );
}

/// **ADR-359 D7: an integer `comptime` binding is its value**, in a body and
/// as an item (#632 step 1).
#[test]
fn an_integer_comptime_binding_is_its_value() {
    // A value the checker folds; one a call computes is compiled and run
    // (ADR-321 D1), and is `comptime_compiled.rs`'s.
    let in_a_body = "fn main() {\n\
                     \x20   comptime B: i64 = 4 / 12\n\
                     \x20   assert(B > 10)\n\
                     \x20   println(f\"{B}\")\n\
                     }\n";
    let found = shown_false(in_a_body);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains("`B > 10` is false every time"),
        "{found:#?}"
    );
    let an_item = "comptime LIMIT: i64 = 6 * 7\n\
                   \n\
                   fn main() {\n\
                   \x20   assert(LIMIT == 42)\n\
                   \x20   assert(LIMIT / 5 == 8)\n\
                   \x20   println(f\"{LIMIT}\")\n\
                   }\n";
    let answers = held(an_item);
    assert!(
        answers
            .iter()
            .all(|h| matches!(h, nikaia::prove::Held::Proved))
            && answers.len() == 2,
        "{answers:#?}"
    );
    // A parameter of the same name is not the `comptime`.
    let hidden = "comptime LIMIT: i64 = 42\n\
                  \n\
                  fn check(LIMIT: i64) {\n\
                  \x20   assert(LIMIT == 42)\n\
                  }\n";
    assert!(
        !held(hidden)
            .iter()
            .any(|h| matches!(h, nikaia::prove::Held::Proved)),
        "{:#?}",
        held(hidden)
    );
}

// --- ADR-359 D11-D13: a call with known arguments a claim needs is run -------

/// **A check with a place to compile build-time code**, as every command-line
/// build has (ADR-321 D19): one workshop for the file, under Cargo's scratch
/// directory.
fn checked_building(source: &str) -> nikaia::check::Checked {
    let workshop = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("claims-run");
    let reads = nikaia::assets::Reads::at(&workshop).building_in(&workshop);
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check_against(
        &parsed,
        &[],
        &own,
        &library,
        &std::collections::BTreeSet::new(),
        &nikaia::check::Newly::new(),
        &reads,
    )
}

fn held_building(source: &str) -> Vec<nikaia::prove::Held> {
    checked_building(source)
        .facts
        .claims
        .into_values()
        .filter_map(|c| c.held)
        .collect()
}

fn refuted_building(source: &str) -> Vec<nikaia::check::Finding> {
    checked_building(source)
        .findings
        .into_iter()
        .filter(|f| f.code == "NK1207")
        .collect()
}

const FIB: &str = "fn fib(n: i64) -> i64 {\n\
                   \x20   if n < 2 {\n\
                   \x20       return n\n\
                   \x20   }\n\
                   \x20   return fib(n - 1) + fib(n - 2)\n\
                   }\n\
                   \n";

const EXAMPLE: &str = "fn positive(whole: i64) -> i64 {\n\
                       \x20   assert(whole > 0)\n\
                       \x20   return whole\n\
                       }\n\
                       \n\
                       fn example(a: i64) -> i64 {\n\
                       \x20   let whole = 10 + a\n\
                       \x20   return positive(whole)\n\
                       }\n\
                       \n";

/// **D11: a recursive call with a known argument is compiled and run** where
/// a claim needs its value: `fib(10)` is 55, so `== 55` is proved and `== 54`
/// refused, the note naming the value the run gave.
#[test]
fn a_call_with_known_arguments_a_claim_needs_is_run() {
    use nikaia::prove::Held::Proved;
    let proved = format!(
        "{FIB}fn main() {{\n\
         \x20   assert(fib(10) == 55)\n\
         \x20   println(\"ok\")\n\
         }}\n"
    );
    assert!(
        matches!(held_building(&proved).as_slice(), [Proved]),
        "{:#?}",
        held_building(&proved)
    );
    let refused = format!(
        "{FIB}fn main() {{\n\
         \x20   assert(fib(10) == 54)\n\
         \x20   println(\"ok\")\n\
         }}\n"
    );
    let found = refuted_building(&refused);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0]
            .message
            .contains("`fib(10) == 54` is false every time"),
        "{found:#?}"
    );
    assert!(
        found[0]
            .notes
            .iter()
            .any(|n| n.contains("`fib(10)` is 55, run while the program was built")),
        "{found:#?}"
    );
}

/// **D11 over the owner's `example`**: `example(2)` is 12, through
/// `positive`, and `4 / example(2)` is 0 (#632's step 1 folds the quotient).
#[test]
fn the_value_of_example_is_known_by_running_it() {
    use nikaia::prove::Held::Proved;
    let proved = format!(
        "{EXAMPLE}fn main() {{\n\
         \x20   assert(example(2) == 12)\n\
         \x20   let e = example(2) - 1\n\
         \x20   assert(e == 11)\n\
         \x20   println(f\"{{e}}\")\n\
         }}\n"
    );
    let held = held_building(&proved);
    assert!(
        held.iter().filter(|h| matches!(h, Proved)).count() == 2,
        "{held:#?}"
    );
    let refused = format!(
        "{EXAMPLE}fn main() {{\n\
         \x20   let b = 4 / example(2)\n\
         \x20   assert(1 == 1)\n\
         \x20   assert(b > 10)\n\
         \x20   println(f\"Hello, Nikaia! {{b}}\")\n\
         }}\n"
    );
    let found = refuted_building(&refused);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains("`b > 10` is false every time"),
        "{found:#?}"
    );
    assert!(
        found[0].notes.iter().any(|n| n.contains("`b` is 0"))
            && found[0]
                .notes
                .iter()
                .any(|n| n.contains("`example(2)` is 12")),
        "{found:#?}"
    );
}

/// **D11, D12: a callee that reads `cli::args` or touches the world is not
/// run**: at build time the arguments are the builder's, and a write would
/// happen on the building machine. The claim stays a check, and `--asserts`
/// says why.
#[test]
fn a_callee_that_reads_the_arguments_or_does_io_is_not_run() {
    use nikaia::prove::Held::AtRunTime;
    let source = "use std::cli\n\
                  \n\
                  fn noisy(n: i64) -> i64 {\n\
                  \x20   println(\"computing\")\n\
                  \x20   let mut m = n\n\
                  \x20   m = m + 1\n\
                  \x20   return m\n\
                  }\n\
                  \n\
                  fn counted(n: i64) -> i64 {\n\
                  \x20   return cli::args().len() + n\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   let a = noisy(2)\n\
                  \x20   assert(a == 3)\n\
                  \x20   let c = counted(1)\n\
                  \x20   assert(c > 1)\n\
                  \x20   println(f\"{a} {c}\")\n\
                  }\n";
    let held = held_building(source);
    let [AtRunTime(io), AtRunTime(args)] = held.as_slice() else {
        panic!("{held:#?}");
    };
    assert!(
        io.contains("`noisy(2)` was not run while the program was built")
            && io.contains("touches the world outside the program"),
        "{io}"
    );
    assert!(
        args.contains("`counted(1)` was not run while the program was built")
            && args.contains("arguments"),
        "{args}"
    );
}

/// **D12: a run that stops is a check the compiler shows fails**, in the
/// words the program would print.
#[test]
fn a_callee_that_overflows_for_its_argument_is_refused() {
    let source = "fn sq(n: i32) -> i32 {\n\
                  \x20   return n * n\n\
                  }\n\
                  \n\
                  fn main() {\n\
                  \x20   assert(sq(100000) > 0)\n\
                  \x20   println(\"ok\")\n\
                  }\n";
    let found = refuted_building(source);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0]
            .message
            .contains("`sq(100000)` stops the program every time it is reached"),
        "{found:#?}"
    );
    assert!(
        found[0]
            .notes
            .iter()
            .any(|n| n.contains("attempt to multiply with overflow")),
        "{found:#?}"
    );
}

/// **D13: only the claim that needs it pays**: a program whose claims are
/// decided without a run runs nothing, though it calls a function with a
/// known argument.
#[test]
fn a_program_whose_claims_are_decided_runs_nothing_at_build_time() {
    use nikaia::prove::Held::AtRunTime;
    let decided = format!(
        "{EXAMPLE}fn main() {{\n\
         \x20   let x = example(2)\n\
         \x20   assert(1 == 1)\n\
         \x20   let y = x + 1\n\
         \x20   assert(y > x)\n\
         \x20   println(f\"{{y}}\")\n\
         }}\n"
    );
    let before = nikaia::prove::claim_runs();
    let held = held_building(&decided);
    assert!(!held.iter().any(|h| matches!(h, AtRunTime(_))), "{held:#?}");
    assert_eq!(nikaia::prove::claim_runs(), before, "a run nobody needed");
    // The same call, where a claim needs it, is run once.
    // A body with an assignment has no summary (ADR-359 D1), so only the run
    // answers.
    let needed = format!(
        "{EXAMPLE}fn looped(n: i64) -> i64 {{\n\
         \x20   let mut s = n\n\
         \x20   s = s + 10\n\
         \x20   return s\n\
         }}\n\
         fn main() {{\n\
         \x20   let x = looped(2)\n\
         \x20   assert(x == 12)\n\
         \x20   assert(x > 11)\n\
         \x20   println(f\"{{x}}\")\n\
         }}\n"
    );
    let before = nikaia::prove::claim_runs();
    let held = held_building(&needed);
    assert!(!held.iter().any(|h| matches!(h, AtRunTime(_))), "{held:#?}");
    assert_eq!(nikaia::prove::claim_runs(), before + 1);
}

/// **D5: a quotient or a remainder by a positive constant is a term**,
/// truncated toward zero for both signs of the dividend.
#[test]
fn a_quotient_by_a_positive_constant_is_a_term() {
    use nikaia::prove::Held::*;
    let source = "fn half(n: i64) {\n\
                  \x20   assert(n > 0)\n\
                  \x20   assert(n / 2 < n)\n\
                  }\n\
                  fn negative(m: i64) {\n\
                  \x20   assert(m == -7)\n\
                  \x20   assert(m / 2 == -3)\n\
                  \x20   assert(m % 2 == -1)\n\
                  }\n\
                  fn every() {\n\
                  \x20   for n in -10..<10 {\n\
                  \x20       assert(n / 3 * 3 + n % 3 == n)\n\
                  \x20   }\n\
                  }\n\
                  fn main() {\n\
                  \x20   half(3)\n\
                  \x20   negative(-7)\n\
                  \x20   every()\n\
                  }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    let held = held(source);
    assert!(matches!(held[1], Proved), "{held:#?}");
    assert!(matches!(held[3], Proved), "{held:#?}");
    assert!(matches!(held[4], Proved), "{held:#?}");
    assert!(matches!(held[5], Proved), "{held:#?}");
    let refused = the_one(
        "fn main() {\n    let n = 4\n    assert(n / 2 > n)\n}\n",
        "NK1207",
    );
    assert!(
        refused.notes.iter().any(|n| n.contains("`n` is 4")),
        "{refused:#?}"
    );
}

/// **D1, D2: inside its package, a call's result is known by its callee's
/// body**, asked only where what is known leaves a claim open - and a
/// program whose claims are decided without it asks no second query.
#[test]
fn a_callees_body_answers_an_open_claim() {
    use nikaia::prove::Held::*;
    let clamp = "fn clamp(n: i64) -> i64 {\n\
                 \x20   if n < 0 {\n\
                 \x20       return 0\n\
                 \x20   }\n\
                 \x20   return n\n\
                 }\n\
                 fn use_it(x: i64) {\n\
                 \x20   let c = clamp(x)\n\
                 \x20   assert(c >= 0)\n\
                 }\n\
                 fn main() {\n    use_it(2)\n}\n";
    assert!(matches!(held(clamp)[0], Proved), "{:#?}", held(clamp));
    let one = format!("{EXAMPLE}fn main() {{\n    assert(example(2) - 1 == 11)\n}}\n");
    let before = nikaia::prove::second_queries();
    assert!(
        held(&one)
            .iter()
            .all(|h| matches!(h, Proved | Precondition(..))),
        "{:#?}",
        held(&one)
    );
    assert!(nikaia::prove::second_queries() > before);
    // Decided without a summary: no second query.
    let decided = format!(
        "{EXAMPLE}fn main() {{\n    let x = 3\n    assert(x > 2)\n    let y = example(x)\n    println(f\"{{y}}\")\n}}\n"
    );
    let before = nikaia::prove::second_queries();
    assert!(
        held(&decided)
            .iter()
            .all(|h| matches!(h, Proved | Precondition(..))),
        "{:#?}",
        held(&decided)
    );
    assert_eq!(
        nikaia::prove::second_queries(),
        before,
        "a second query nobody needed"
    );
    // A body with a loop gives no summary.
    let looping = "fn count(n: i64) -> i64 {\n\
                   \x20   let mut i = 0\n\
                   \x20   while i < n {\n\
                   \x20       i = i + 1\n\
                   \x20   }\n\
                   \x20   return n\n\
                   }\n\
                   fn main() {\n    let c = count(3)\n    assert(c == 3)\n}\n";
    assert!(
        matches!(held(looping)[0], AtRunTime(_)),
        "{:#?}",
        held(looping)
    );
}

/// **D6, D8: a divisor a summary pins is its value, and the refusal says
/// where the value came from** - without running anything.
#[test]
fn a_divisor_a_summary_pins_is_its_value_and_the_note_says_so() {
    let source =
        format!("{EXAMPLE}fn main() {{\n    let b = 4 / example(2)\n    assert(b > 10)\n}}\n");
    let refused = the_one(&source, "NK1207");
    assert!(
        refused.notes.iter().any(|n| n.contains("`b` is 0")),
        "{refused:#?}"
    );
    assert!(
        refused.notes.iter().any(|n| n
            == "`example(2)` is 12: `example` returns `positive(whole)`, `positive` returns `whole`."),
        "{refused:#?}"
    );
}
