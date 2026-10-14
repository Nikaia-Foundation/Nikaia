//! A function value keeps the error set inferred for it
//! ([ADR-351](../../../docs/specification/adr/adr-351.md), #620): storing one
//! calls nothing (D2), and a call through the place it was stored in fails with
//! the union of what reaches that place (D1, D3).

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
}

fn codes(source: &str) -> Vec<String> {
    findings(source)
        .into_iter()
        .map(|f| f.code.to_string())
        .collect()
}

const JOB: &str = "enum Bad { Oops }\n\
                   impl Error for Bad {\n\
                   \x20   fn message(ref self) -> String { \"oops\" }\n\
                   }\n\
                   struct Job { run: fn() throws }\n\
                   fn fail() throws {\n\
                   \x20   throw Bad::Oops\n\
                   }\n\
                   fn quiet() {\n\
                   \x20   print(\"fine\")\n\
                   }\n";

/// **The program of #620**: the kept arm is exhaustive with no `else`, and
/// storing `fail` is not a call (`NK2605` is not reported on the `let`).
#[test]
fn a_call_through_a_field_fails_with_what_was_stored() {
    let source = format!(
        "{JOB}fn main() {{\n\
         \x20   let job = Job {{ run: fail }}\n\
         \x20   job.run() catch {{\n\
         \x20       match error {{\n\
         \x20           Bad::Oops => print(\"oops\")\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    );
    let found = codes(&source);
    assert!(found.is_empty(), "{:?}", findings(&source));
}

/// **Storing a failing function in a function that cannot fail is fine**
/// (D2): only the call fails.
#[test]
fn storing_a_failing_function_is_not_a_call() {
    let source = format!("{JOB}fn main() {{\n    let job = Job {{ run: fail }}\n}}\n");
    let found = codes(&source);
    assert!(!found.contains(&"NK2605".to_string()), "{found:?}");
}

/// **A call through the field without a `catch` makes the function fail**.
#[test]
fn an_uncaught_call_through_the_field_needs_throws() {
    let source =
        format!("{JOB}fn main() {{\n    let job = Job {{ run: fail }}\n    job.run()\n}}\n");
    let found = codes(&source);
    assert_eq!(found, vec!["NK2605".to_string()], "{:?}", findings(&source));
}

/// **The handler opens the box by the type its arm names** (D2), and needs
/// no catch-all where the arms cover the set.
#[test]
fn the_kept_arm_lowers_without_an_else() {
    let source = format!(
        "{JOB}fn main() {{\n\
         \x20   let job = Job {{ run: fail }}\n\
         \x20   job.run() catch {{\n\
         \x20       match error {{\n\
         \x20           Bad::Oops => print(\"oops\")\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    );
    let parsed = parse_to_ast(&source).expect("the source parses");
    let rust = nikaia::emit::emit_program(&parsed, nikaia::emit::Build::default())
        .expect("the source lowers")
        .rust;
    assert!(rust.contains("nikaia_std::error::holds::<Bad>"), "{rust}");
    assert!(rust.contains("unreachable!("), "{rust}");
}

/// **A place no failing value reaches carries the empty set** (D3): a `catch`
/// over a call through it is `NK1134`.
#[test]
fn a_catch_over_a_field_nothing_failing_reaches_is_nk1134() {
    let source = format!(
        "{JOB}fn main() {{\n\
         \x20   let job = Job {{ run: quiet }}\n\
         \x20   job.run() catch {{ print(\"never\") }}\n\
         }}\n"
    );
    let found = codes(&source);
    assert_eq!(found, vec!["NK1134".to_string()], "{:?}", findings(&source));
}

/// **A name bound to a function value is a place too** (D1): a call through
/// it is caught by the type that was stored there, with no `else`.
#[test]
fn a_call_through_a_variable_fails_with_what_it_was_given() {
    let source = format!(
        "{JOB}fn main() {{\n\
         \x20   let run = fail\n\
         \x20   run() catch {{\n\
         \x20       match error {{\n\
         \x20           Bad::Oops => print(\"oops\")\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    );
    let found = codes(&source);
    assert!(found.is_empty(), "{:?}", findings(&source));
}

/// **The elements of a list of code meet** (D4): a catch over a call of an
/// element takes what each of them fails with, by name.
#[test]
fn a_list_of_functions_fails_with_the_union() {
    let source = format!(
        "{JOB}enum Worse {{ Boom }}\n\
         impl Error for Worse {{\n\
         \x20   fn message(ref self) -> String {{ \"boom\" }}\n\
         }}\n\
         fn worse() throws {{\n\
         \x20   throw Worse::Boom\n\
         }}\n\
         fn main() {{\n\
         \x20   let all: Vec[fn() throws] = [fail, worse]\n\
         \x20   for run in all {{\n\
         \x20       run() catch {{\n\
         \x20           match error {{\n\
         \x20               Bad::Oops => print(\"oops\")\n\
         \x20               Worse::Boom => print(\"boom\")\n\
         \x20           }}\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    );
    let found = codes(&source);
    assert!(found.is_empty(), "{:?}", findings(&source));
}

/// **A parameter of code is given what every caller hands it** (D4).
#[test]
fn a_parameter_fails_with_what_its_callers_hand_it() {
    let source = format!(
        "{JOB}fn guard(run: fn() throws) {{\n\
         \x20   run() catch {{\n\
         \x20       match error {{\n\
         \x20           Bad::Oops => print(\"oops\")\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n\
         fn main() {{\n\
         \x20   guard(fail)\n\
         \x20   guard(quiet)\n\
         }}\n"
    );
    let found = codes(&source);
    assert!(found.is_empty(), "{:?}", findings(&source));
}

/// **A list of functions that cannot fail carries the empty set**: `NK1134`.
#[test]
fn a_list_of_quiet_functions_is_nk1134() {
    let source = format!(
        "{JOB}fn main() {{\n\
         \x20   let all: Vec[fn() throws] = [quiet, quiet]\n\
         \x20   for run in all {{\n\
         \x20       run() catch {{ print(\"never\") }}\n\
         \x20   }}\n\
         }}\n"
    );
    let found = codes(&source);
    assert_eq!(found, vec!["NK1134".to_string()], "{:?}", findings(&source));
}

/// **A variable given a function that cannot fail carries the empty set**.
#[test]
fn a_catch_over_a_variable_nothing_failing_reaches_is_nk1134() {
    let source = format!(
        "{JOB}fn main() {{\n\
         \x20   let run: fn() throws = quiet\n\
         \x20   run() catch {{ print(\"never\") }}\n\
         }}\n"
    );
    let found = codes(&source);
    assert_eq!(found, vec!["NK1134".to_string()], "{:?}", findings(&source));
}

const WORSE: &str = "enum Boom { Bang }\n\
                     impl Error for Boom {\n\
                     \x20   fn message(ref self) -> String { \"boom\" }\n\
                     }\n\
                     fn worse() throws {\n\
                     \x20   throw Boom::Bang\n\
                     }\n";

const ONLY_BAD: &str = "catch {\n\
                        \x20       match error {\n\
                        \x20           Bad::Oops => print(\"oops\")\n\
                        \x20       }\n\
                        \x20   }";

/// **Every way more code reaches a place must reach its set, or the set is
/// `?`** (ADR-351 D4): an arm list naming only `Bad` would otherwise lower to
/// an `unreachable!` that a `Boom` hits at run time.
fn refused_without_else(body: &str, extra: &str) {
    let source = format!("{JOB}{WORSE}{extra}fn main() {{\n{body}\n}}\n");
    let found = codes(&source);
    assert!(!found.is_empty(), "accepted, would panic:\n{source}");
}

#[test]
fn a_function_returned_as_a_value_has_callers_nobody_sees() {
    refused_without_else(
        "    guard(fail)\n    let g = pick()\n    g(worse)",
        &format!(
            "fn guard(run: fn() throws) {{\n    run() {ONLY_BAD}\n}}\n\
             fn pick() -> fn(fn() throws) {{\n    return guard\n}}\n"
        ),
    );
}

#[test]
fn a_mut_name_given_other_code_later() {
    refused_without_else(
        &format!("    let mut run: fn() throws = fail\n    run = worse\n    run() {ONLY_BAD}"),
        "",
    );
}

#[test]
fn a_mut_list_pushed_other_code() {
    refused_without_else(
        &format!(
            "    let mut all: Vec[fn() throws] = [fail]\n    all.push(worse)\n    for run in all {{\n        run() {ONLY_BAD}\n    }}"
        ),
        "",
    );
}

#[test]
fn a_field_assigned_other_code() {
    refused_without_else(
        &format!(
            "    let mut job = Job {{ run: fail }}\n    job.run = worse\n    job.run() {ONLY_BAD}"
        ),
        "",
    );
}

#[test]
fn a_field_given_other_code_by_with() {
    refused_without_else(
        &format!(
            "    let job = Job {{ run: fail }}\n    let other = job with {{ run: worse }}\n    other.run() {ONLY_BAD}"
        ),
        "",
    );
}

#[test]
fn a_list_field_pushed_other_code() {
    refused_without_else(
        &format!(
            "    let mut jobs = Jobs {{ all: [fail] }}\n    jobs.all.push(worse)\n    for run in jobs.all {{\n        run() {ONLY_BAD}\n    }}"
        ),
        "struct Jobs { all: Vec[fn() throws] }\n",
    );
}

#[test]
fn a_published_field_is_stored_into_by_other_packages() {
    refused_without_else(
        &format!("    let job = Task {{ run: fail }}\n    job.run() {ONLY_BAD}"),
        "pub struct Task { pub run: fn() throws }\n",
    );
}

/// **A caller replaces `from(p)` with what it passed** (ADR-351 D4): `now` is
/// `pub`, so its parameter's code is `?` for `now` itself, but the call
/// `now(fail)` fails with what `fail` fails with, and the `catch` over it
/// needs no `else`. Handing it `worse` makes the one arm too few.
#[test]
fn a_caller_replaces_from_with_what_it_passed() {
    let now = "pub fn now(f: fn() throws) throws {\n    f()\n}\n";
    let source = format!("{JOB}{WORSE}{now}fn main() {{\n    now(fail) {ONLY_BAD}\n}}\n");
    assert!(codes(&source).is_empty(), "{:?}", findings(&source));
    refused_without_else(&format!("    now(worse) {ONLY_BAD}"), now);
}

/// **The ledger writes `from(p)` and reads it back** (ADR-351 D4).
#[test]
fn the_ledger_says_from_the_parameter() {
    let parsed = parse_to_ast("pub fn now(f: fn() throws) throws {\n    f()\n}\nfn main() {\n}\n")
        .expect("the source parses");
    let text = Ledger::infer(&parsed).render();
    assert!(text.contains("throws = [\"from(f)\"]"), "{text}");
    let back = Ledger::parse(&text).expect("it reads back");
    let now = &back.functions["now"];
    assert_eq!(now.throws_from, vec!["f".to_string()]);
    assert_eq!(now.fails_with, vec!["?".to_string()]);
}
