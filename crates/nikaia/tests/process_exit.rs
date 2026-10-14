// crates/nikaia/tests/process_exit.rs
//
// `process::exit(code)` throws the fatal error `process::Exit`
// ([ADR-345](../../../docs/specification/adr/adr-345.md)): it travels with
// `throws`, a `catch` takes it only in an arm naming it, and at the end of
// `main` the program ends with its code after the cleanups; a constant code
// outside 0..=255 is `NK1116` (ADR-343 D4).

mod common;

use std::process::Command;

use nikaia::check::{self, Finding};
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn ran(purpose: &str, source: &str, how: Build) -> (String, String, Option<i32>) {
    let parsed = parse_to_ast(source).expect("the source parses");
    let rust = emit_program(&parsed, how).expect("the source lowers").rust;
    let dir = common::scratch_dir(&format!("process-exit-{purpose}"));
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &[
            "--crate-type",
            "bin",
            "-o",
            binary.to_str().expect("utf-8 path"),
        ],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary)
        .current_dir(&dir)
        .output()
        .expect("run it");
    std::fs::remove_dir_all(&dir).ok();
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code(),
    )
}

fn codes(source: &str) -> Vec<String> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    let found: Vec<Finding> = check::check(&parsed, &own, &library).findings;
    found.into_iter().map(|finding| finding.code).collect()
}

const IN_MAIN: &str = r#"use std::process

fn main() throws {
    println("before")
    process::exit(3)
    println("after")
}
"#;

/// ADR-345 D3, D4: the program ends with the code, and nothing after the call
/// runs.
#[test]
fn exit_in_main_ends_with_its_code() {
    let (out, err, code) = ran("main", IN_MAIN, Build::default());
    assert_eq!(code, Some(3), "{err}");
    assert_eq!(out, "before\n");
    assert!(codes(IN_MAIN).is_empty(), "{:?}", codes(IN_MAIN));
}

const CLEANED: &str = r#"use std::process

struct Noted {
    name: String,
}

impl Cleanup for Noted {
    fn cleanup(ref mut self) {
        println(f"cleanup {self.name}")
    }
}

fn stop() throws {
    let inner = Noted { name: f"inner" }
    println(f"stopping {inner.name}")
    process::exit(4)
}

fn middle() throws {
    stop()
}

fn main() throws {
    let outer = Noted { name: f"outer" }
    println(f"start {outer.name}")
    middle()
}
"#;

/// D3, D4: two levels deep with `throws`; the values the frames hold are
/// cleaned up, in the order they die, before the process ends with the code.
#[test]
fn exit_two_levels_deep_runs_the_cleanups() {
    for how in [Build::default(), Build::parallel()] {
        let (out, err, code) = ran("cleaned", CLEANED, how);
        assert_eq!(code, Some(4), "{err}");
        assert_eq!(
            out, "start outer\nstopping inner\ncleanup inner\ncleanup outer\n",
            "at {how:?}"
        );
    }
}

/// D3: a function that calls `exit` says `throws`.
#[test]
fn exit_needs_throws() {
    let source = "use std::process\n\nfn main() {\n    process::exit(1)\n}\n";
    assert!(
        codes(source).contains(&"NK2605".to_string()),
        "{:?}",
        codes(source)
    );
}

const PASSES: &str = r#"use std::process

fn f() -> i64 throws {
    process::exit(5)
    return 1
}

fn main() throws {
    let v = f() catch { 0 }
    println(f"{v}")
}
"#;

/// D2: a handler without a `match` does not take `Exit`.
#[test]
fn a_plain_catch_lets_exit_through() {
    let (out, err, code) = ran("passes", PASSES, Build::default());
    assert_eq!(code, Some(5), "{err}");
    assert_eq!(out, "");
    let unthrowing = PASSES.replace("fn main() throws", "fn main()");
    assert!(
        codes(&unthrowing).contains(&"NK2605".to_string()),
        "{:?}",
        codes(&unthrowing)
    );
}

const NAMED: &str = r#"use std::process

fn f() -> i64 throws {
    process::exit(5)
    return 1
}

fn main() {
    let v = f() catch {
        match error {
            process::Exit::Code(c) => c + 10
        }
    }
    println(f"{v}")
}
"#;

/// D2: an arm that names the type takes it.
#[test]
fn a_named_arm_takes_exit() {
    assert!(codes(NAMED).is_empty(), "{:?}", codes(NAMED));
    let (out, err, code) = ran("named", NAMED, Build::default());
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "15\n");
}

const ELSE_ARM: &str = r#"use std::process

enum E { A(i64) }
impl Error for E {
    fn message(ref self) -> String { return "e" }
}

fn f(x: i64) -> i64 throws {
    if x > 3 { throw E::A(x) }
    process::exit(6)
    return x
}

fn main() throws {
    let v = f(1) catch {
        match error {
            E::A(c) => c
            else => 0
        }
    }
    println(f"{v}")
}
"#;

/// D2: an `else` arm does not take it.
#[test]
fn an_else_arm_lets_exit_through() {
    let (out, err, code) = ran("else-arm", ELSE_ARM, Build::default());
    assert_eq!(code, Some(6), "{err}");
    assert_eq!(out, "");
}

/// D5: in a task `Exit` is an ordinary error; `NK2607` is retired.
#[test]
fn exit_in_a_spawn_is_an_ordinary_error() {
    let source = "use std::process\n\nfn stop() throws {\n    process::exit(1)\n}\n\nfn main() throws {\n    spawn fn { stop() }\n}\n";
    let found = codes(source);
    assert!(!found.contains(&"NK2607".to_string()), "{found:?}");
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    assert_eq!(
        own.functions["stop"].fails_with,
        vec!["process::Exit".to_string()]
    );
}

/// ADR-346 D5: the joiner receives the task's `Exit`, and `main` ends with it.
#[test]
fn exit_in_a_spawn_reaches_the_joiner() {
    let source = "use std::process\n\nfn stop() -> i64 throws {\n    process::exit(3)\n    return 1\n}\n\nfn main() throws {\n    let h = spawn fn { stop() }\n    let v = h.join()\n    println(f\"never {v}\")\n}\n";
    let (out, err, code) = ran("spawn-exit", source, Build::default());
    assert_eq!(code, Some(3), "{err}");
    assert_eq!(out, "");
}

/// ADR-346 D5: a task's own error reaches `join`'s `catch`; `detach` gives a
/// handle up.
#[test]
fn a_task_error_reaches_join() {
    let source = "enum Bad { Oops(i64) }\nimpl Error for Bad {\n    fn message(ref self) -> String { return \"bad\" }\n}\n\nfn work(n: i64) -> i64 throws {\n    if n > 1 {\n        throw Bad::Oops(n)\n    }\n    return n\n}\n\nfn main() throws {\n    let a = spawn fn { work(1) }\n    let b = spawn fn { work(7) }\n    let x = a.join()\n    let y = b.join() catch { 5 }\n    let w = spawn fn { work(0) }\n    w.detach()\n    println(f\"{x} {y}\")\n}\n";
    let (out, err, code) = ran("spawn-join-error", source, Build::default());
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out.trim(), "1 5");
}

/// ADR-346 D5: a `catch` at `join` names the body's error type; at
/// `user_parallelism = yes` a body failing in the opaque channel still hands
/// its `Exit` to the joiner.
#[test]
fn a_catch_at_join_names_the_body_error() {
    let source = "enum Bad { Oops(i64) }\nimpl Error for Bad {\n    fn message(ref self) -> String { return \"bad\" }\n}\n\nfn work(n: i64) -> i64 throws {\n    if n > 1 {\n        throw Bad::Oops(n)\n    }\n    return n\n}\n\nfn main() throws {\n    let b = spawn fn { work(7) }\n    let y = b.join() catch {\n        match error {\n            Bad::Oops(c) => c,\n            else => 0,\n        }\n    }\n    println(f\"{y}\")\n}\n";
    let (out, err, code) = ran("spawn-join-typed", source, Build::default());
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out.trim(), "7");
    let exit = "use std::process\n\nfn stop() -> i64 throws {\n    process::exit(4)\n    return 1\n}\n\nfn main() throws {\n    let h = spawn fn { stop() }\n    let v = h.join()\n    println(f\"never {v}\")\n}\n";
    let (out, err, code) = ran("spawn-exit-pool", exit, Build::parallel());
    assert_eq!(code, Some(4), "{err}");
    assert_eq!(out, "");
}

/// ADR-346 D4: `join(within: d)` hands the value back in time, and throws
/// `task::TimedOut` (the task cancelled) once `d` ran out.
#[test]
fn join_within_times_out() {
    let source = "use std::time\n\nfn slow() -> i64 {\n    time::sleep(5.seconds())\n    return 1\n}\n\nfn quick() -> i64 {\n    return 2\n}\n\nfn main() throws {\n    let a = spawn fn { quick() }\n    let x = a.join(within: 2.seconds())\n    let b = spawn fn { slow() }\n    let y = b.join(within: 10.millis()) catch {\n        match error {\n            task::TimedOut => 9,\n            else => 0,\n        }\n    }\n    println(f\"{x} {y}\")\n}\n";
    let (out, err, code) = ran("spawn-join-within", source, Build::default());
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out.trim(), "2 9");
}

const OWN_FATAL: &str = r#"enum Abort { Now(i64) }
impl Error for Abort {
    fn message(ref self) -> String { return "aborted" }
}
impl Fatal for Abort {}

fn f() -> i64 throws {
    throw Abort::Now(2)
}

fn main() throws {
    let v = f() catch { 0 }
    println(f"plain {v}")
}
"#;

/// D1, D2: a program's own `impl Fatal` behaves as `Exit` does at a `catch`.
#[test]
fn an_own_fatal_type_passes_a_plain_catch() {
    assert!(codes(OWN_FATAL).is_empty(), "{:?}", codes(OWN_FATAL));
    let (out, err, code) = ran("own-fatal", OWN_FATAL, Build::default());
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(out, "");
    let named = OWN_FATAL.replace(
        "catch { 0 }",
        "catch {\n        match error {\n            Abort::Now(c) => c\n        }\n    }",
    );
    let (out, err, code) = ran("own-fatal-named", &named, Build::default());
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "plain 2\n");
}

/// ADR-343 D4: a constant code outside 0..=255 is refused.
#[test]
fn a_constant_code_out_of_range_is_refused() {
    let source = "use std::process\n\nfn main() throws {\n    process::exit(300)\n}\n";
    assert!(
        codes(source).contains(&"NK1116".to_string()),
        "{:?}",
        codes(source)
    );
}

/// D1: a fatal type is an error as well; `impl Fatal` alone is `NK1164`, and
/// the help writes the missing `impl Error` (#586).
#[test]
fn a_fatal_type_without_an_error_impl_is_refused() {
    let source = "enum Abort { Now(i64) }\nimpl Fatal for Abort {}\n\nfn main() {}\n";
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    let found: Vec<Finding> = check::check(&parsed, &own, &library).findings;
    assert!(
        found.iter().any(|f| f.code == "NK1164"
            && f.help
                .as_deref()
                .unwrap_or("")
                .contains("impl Error for Abort")),
        "{:?}",
        found.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
    assert!(codes(OWN_FATAL).is_empty(), "{:?}", codes(OWN_FATAL));
}

const SHORT: &str = r#"use std::process

enum Meters { Value(i64) }
enum Pos { At { x: i64, y: i64 } }
enum Done { Only }

fn f() -> i64 throws {
    throw process::Exit(2)
    return 1
}

fn main() {
    let m = Meters(5)
    let p = Pos { x: 1, y: 2 }
    let d = Done
    let a = match m { Meters(v) => v }
    let b = match p { Pos { x, y } => x + y }
    let c = match d { Done::Only => 7 }
    let v = f() catch {
        match error {
            process::Exit(c) => c + 10
        }
    }
    println(f"{a} {b} {c} {v}")
}
"#;

/// ADR-347 D1, D4: an enum of one variant is built and matched by its type's
/// name - positional, named fields and unit - and `process::Exit(c)` is
/// thrown and caught.
#[test]
fn one_variant_is_written_by_the_type_name() {
    assert!(
        codes(SHORT).iter().all(|c| c != "NK1252"),
        "{:?}",
        codes(SHORT)
    );
    let (out, err, code) = ran("short", SHORT, Build::default());
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out, "5 3 7 12\n");
}

/// ADR-347 D4: `throw process::Exit(2)` leaving `main` ends with code 2.
#[test]
fn a_short_exit_ends_main_with_its_code() {
    let source = "use std::process\n\nfn main() throws {\n    throw process::Exit(2)\n}\n";
    let (_, err, code) = ran("short-main", source, Build::default());
    assert_eq!(code, Some(2), "{err}");
}

/// ADR-347 D3: the short form of an enum with two variants is `NK1252`, and
/// the message offers the long form.
#[test]
fn several_variants_refuse_the_short_form() {
    for source in [
        "enum Shape { Circle(i64), Square(i64) }\nfn main() {\n    let s = Shape(1)\n}\n",
        "enum Shape { Circle(i64), Square(i64) }\nfn main() {\n    let s = Shape::Circle(1)\n    let n = match s { Shape(r) => r, else => 0 }\n    println(f\"{n}\")\n}\n",
    ] {
        let refused = match parse_to_ast(source) {
            Ok(_) => panic!("the short form of two variants parses"),
            Err(e) => e.to_string(),
        };
        assert!(refused.contains("NK1252"), "{refused}");
        assert!(refused.contains("`Shape::Circle`"), "{refused}");
    }
}
