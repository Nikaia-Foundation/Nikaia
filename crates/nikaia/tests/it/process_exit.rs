// crates/nikaia/tests/process_exit.rs
//
// `process::exit(code)` throws the fatal error `process::Exit`
// ([ADR-345](../../../docs/specification/adr/adr-345.md)): it travels with
// `throws`, a `catch` takes it only in an arm naming it, and at the end of
// `main` the program ends with its code after the cleanups; a constant code
// outside 0..=255 is `NK1116` (ADR-343 D4).

use crate::common;
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

/// ADR-346 D6: a detached task's error goes to standard error at `main`'s end,
/// and the first fatal one's code is the program's; `main` itself runs on.
#[test]
fn a_detached_error_reaches_the_log_at_the_end() {
    let source = "use std::process\nuse std::time\n\nenum Bad { Oops(i64) }\nimpl Error for Bad {\n    fn message(ref self) -> String { return \"bad\" }\n}\n\nfn work(n: i64) -> i64 throws {\n    if n > 1 {\n        throw Bad::Oops(n)\n    }\n    return n\n}\n\nfn stop() -> i64 throws {\n    process::exit(5)\n    return 1\n}\n\nfn main() throws {\n    let w = spawn fn { work(7) }\n    w.detach()\n    time::sleep(20.millis())\n    spawn fn { stop() }\n    time::sleep(20.millis())\n    println(\"end\")\n}\n";
    for (purpose, how) in [
        ("detached", Build::default()),
        ("detached-pool", Build::parallel()),
    ] {
        let (out, err, code) = ran(purpose, source, how);
        assert_eq!(out.trim(), "end", "{err}");
        assert_eq!(code, Some(5), "{err}");
        assert!(err.contains("Error in a detached task: bad"), "{err}");
    }
}

/// ADR-346 D7: a supervisor does not restart a child's fatal `Exit`; it hands
/// it on as `supervisor::Handed`, which is fatal, so a plain `catch` does not
/// take it, and `main` ends with its code.
#[test]
fn a_supervisor_hands_a_fatal_error_on() {
    let source = "use std::process\nuse std::supervisor\n\nfn stop(tries: SharedMut[i64]) throws {\n    tries.update fn(mut v) { v += 1 }\n    process::exit(6)\n}\n\nfn main() throws {\n    let tries = SharedMut(0)\n    let seen = tries.clone()\n    supervisor::run([\n        supervisor::child(fn { stop(tries) }; restart: supervisor::Kind::Permanent),\n    ]) catch {\n        let n = seen.get()\n        println(f\"tries {n}\")\n        throw error\n    }\n}\n";
    let (out, err, code) = ran("supervisor-fatal", source, Build::default());
    assert_eq!(code, Some(6), "{err}");
    assert_eq!(out.trim(), "", "{err}");
}

/// ADR-345 D1 with ADR-346 D6, D7: a program's own fatal type is fatal in a
/// task as `Exit` is - a supervisor does not restart it, and a detached task's
/// sets the status `1` at `main`'s end.
#[test]
fn a_program_fatal_type_is_fatal_in_tasks() {
    let decl = "use std::supervisor\nuse std::time\n\nenum Doom { Over(i64) }\nimpl Error for Doom {\n    fn message(ref self) -> String { return \"doom\" }\n}\nimpl Fatal for Doom {}\n\n";
    let supervised = format!(
        "{decl}fn stop(tries: SharedMut[i64]) throws {{\n    tries.update fn(mut v) {{ v += 1 }}\n    println(\"try\")\n    throw Doom::Over(1)\n}}\n\nfn main() throws {{\n    let tries = SharedMut(0)\n    supervisor::run([\n        supervisor::child(fn {{ stop(tries) }}; restart: supervisor::Kind::Permanent),\n    ]) catch {{\n        println(\"taken\")\n    }}\n}}\n"
    );
    let (out, err, code) = ran("supervisor-own-fatal", &supervised, Build::default());
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(out.trim(), "try", "{err}");
    assert!(err.contains("doom"), "{err}");
    let detached = format!(
        "{decl}fn work() -> i64 throws {{\n    throw Doom::Over(2)\n    return 1\n}}\n\nfn main() throws {{\n    spawn fn {{ work() }}\n    time::sleep(20.millis())\n    println(\"end\")\n}}\n"
    );
    for (purpose, how) in [
        ("detached-own-fatal", Build::default()),
        ("detached-own-fatal-pool", Build::parallel()),
    ] {
        let (out, err, code) = ran(purpose, &detached, how);
        assert_eq!(out.trim(), "end", "{err}");
        assert_eq!(code, Some(1), "{err}");
        assert!(err.contains("Error in a detached task: doom"), "{err}");
    }
}

/// ADR-346 D5: on the pool a body failing in the opaque channel hands its
/// error itself to the joiner, so a typed arm at `join` takes it; and a
/// supervisor's child's fatal error reaches an arm that names its own type
/// (D7).
#[test]
fn a_typed_error_crosses_the_pool() {
    let source = "enum Bad { Oops(i64) }\nimpl Error for Bad {\n    fn message(ref self) -> String { return \"bad\" }\n}\n\nfn work(n: i64) -> i64 throws {\n    if n > 1 {\n        throw Bad::Oops(n)\n    }\n    return n\n}\n\nfn main() throws {\n    let b = spawn fn { work(7) }\n    let y = b.join() catch {\n        match error {\n            Bad::Oops(c) => c,\n            else => 0,\n        }\n    }\n    println(f\"{y}\")\n}\n";
    let (out, err, code) = ran("spawn-join-typed-pool", source, Build::parallel());
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out.trim(), "7");
    let supervised = "use std::supervisor\n\nenum Doom { Over(i64) }\nimpl Error for Doom {\n    fn message(ref self) -> String { return \"doom\" }\n}\nimpl Fatal for Doom {}\n\nfn stop() throws {\n    throw Doom::Over(3)\n}\n\nfn watch() throws {\n    supervisor::run([\n        supervisor::child(fn { stop() }; restart: supervisor::Kind::Permanent),\n    ])\n}\n\nfn main() throws {\n    watch() catch {\n        match error {\n            Doom::Over(c) => println(f\"taken {c}\"),\n            else => throw error,\n        }\n    }\n}\n";
    for (purpose, how) in [
        ("supervisor-own-type", Build::default()),
        ("supervisor-own-type-pool", Build::parallel()),
    ] {
        let (out, err, code) = ran(purpose, supervised, how);
        assert_eq!(code, Some(0), "{err}");
        assert_eq!(out.trim(), "taken 3", "{err}");
    }
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

const SCOPE_FAILS: &str = r#"use std::task

enum Bad { Oops(i64) }
impl Error for Bad {
    fn message(ref self) -> String { return "bad" }
}

fn work(n: i64) -> i64 throws {
    if n > 1 {
        throw Bad::Oops(n)
    }
    return n
}

fn main() throws {
    let base = 6
    let y = task::scope fn(s) {
        s.spawn fn { work(base - 5) }
        s.spawn fn {
            let n = work(7)
            println(f"never {n}")
        }
        s.spawn fn { work(9) }
        5
    } catch {
        match error {
            Bad::Oops(c) => c,
            else => 0,
        }
    }
    println(f"{y}")
}
"#;

/// ADR-346 D7: `task::scope` hands a task's error on; the first in start
/// order is what a `catch` around the scope takes, at both settings, and one
/// nobody catches leaves `main`. A fatal one passes a plain `catch`.
#[test]
fn a_scope_hands_its_tasks_errors_on() {
    for how in [Build::default(), Build::parallel()] {
        let (out, err, code) = ran("scope-caught", SCOPE_FAILS, how);
        assert_eq!(code, Some(0), "{err}");
        assert_eq!(out, "7\n");
        let uncaught = SCOPE_FAILS.replace(
            "    } catch {\n        match error {\n            Bad::Oops(c) => c,\n            else => 0,\n        }\n    }\n",
            "    }\n",
        );
        let (out, err, code) = ran("scope-uncaught", &uncaught, how);
        assert_eq!(code, Some(1), "{err}");
        assert_eq!(out, "");
        assert!(err.contains("bad"), "{err}");
        let fatal = SCOPE_FAILS
            .replace("impl Error for Bad {", "impl Fatal for Bad {}\nimpl Error for Bad {")
            .replace(
                "catch {\n        match error {\n            Bad::Oops(c) => c,\n            else => 0,\n        }\n    }",
                "catch { 0 }",
            );
        let (out, err, code) = ran("scope-fatal", &fatal, how);
        assert_eq!(code, Some(1), "{err}");
        assert_eq!(out, "");
    }
}

const SCOPE_JOINS: &str = r#"use std::task

enum Bad { Oops(i64) }
impl Error for Bad {
    fn message(ref self) -> String { return "bad" }
}
enum Worse { Ugh }
impl Error for Worse {
    fn message(ref self) -> String { return "worse" }
}

fn work(n: i64) -> i64 throws {
    if n > 1 {
        throw Bad::Oops(n)
    }
    return n
}

fn other() throws {
    throw Worse::Ugh
}

fn main() throws {
    task::scope fn(s) {
        s.spawn fn { work(7) }
        s.spawn fn { other() }
        s.spawn fn { work(9) }
        5
    }
}
"#;

const SCOPE_FATAL_SITE: &str = r#"use std::task

enum Bad { Oops(i64) }
impl Fatal for Bad {}
impl Error for Bad {
    fn message(ref self) -> String { return "bad" }
}

fn work(n: i64) -> i64 throws {
    if n > 1 {
        throw Bad::Oops(n)
    }
    return n
}

fn inner() -> i64 throws {
    let base = 6
    let y = task::scope fn(s) {
        s.spawn fn { work(base + 1) }
        5
    } catch { 0 }
    return y
}

fn main() throws {
    let y = inner()
    println(f"{y}")
}
"#;

/// ADR-346 D7: in a sum channel a later failure of the first one's type joins
/// it; a fatal one passing a scope's `catch` into a typed channel keeps the
/// site it was raised at.
#[test]
fn a_scope_joins_in_a_sum_and_keeps_a_fatal_site() {
    for how in [Build::default(), Build::parallel()] {
        let (out, err, code) = ran("scope-joins", SCOPE_JOINS, how);
        assert_eq!(code, Some(1), "{err}");
        assert_eq!(out, "");
        assert!(err.contains("and then:"), "{err}");
        assert!(
            err.contains("worse"),
            "a later failure of another member is kept: {err}"
        );
        let (out, err, code) = ran("scope-fatal-site", SCOPE_FATAL_SITE, how);
        assert_eq!(code, Some(1), "{err}");
        assert_eq!(out, "");
        assert!(err.contains("raised at work"), "{err}");
    }
}

const LEFT_TO_THE_OS: &str = r#"use std::process

struct Noted {
    name: String,
}

impl Cleanup for Noted {
    fn cleanup(ref mut self) {
        println(f"cleanup {self.name}")
    }
}

fn main() throws {
    let held = Noted { name: f"held" }
    let mut many: Vec[String] = []
    for i in 0..1000 {
        many.push(f"row {i}")
    }
    println(f"rows {many.len()} {held.name}")
}

"#;

const LEFT_TO_THE_OS_PLAIN: &str = r#"struct Noted {
    name: String,
}

impl Cleanup for Noted {
    fn cleanup(ref mut self) {
        println(f"cleanup {self.name}")
    }
}

fn main() {
    let held = Noted { name: f"plain" }
    let mut many: Vec[String] = []
    for i in 0..1000 {
        many.push(f"row {i}")
    }
    println(f"rows {many.len()} {held.name}")
}
"#;

/// ADR-348 D1, D2: with `skip-free-at-exit:on` the program's output, its
/// cleanup at the end of `main` and its exit status are the default's; only
/// the allocator and the call after `main`'s last statement are new, and a
/// build without the option carries neither.
#[test]
fn skip_free_at_exit_keeps_output_cleanups_and_status() {
    let on = |how: Build| Build {
        skip_free_at_exit: true,
        ..how
    };
    for how in [Build::default(), Build::parallel()] {
        for (purpose, source, expected) in [
            (
                "os-throws",
                LEFT_TO_THE_OS,
                "rows 1001 held\ncleanup held\n",
            ),
            (
                "os-plain",
                LEFT_TO_THE_OS_PLAIN,
                "rows 1001 plain\ncleanup plain\n",
            ),
        ] {
            let default = ran(purpose, source, how);
            let skipped = ran(purpose, source, on(how));
            assert_eq!(skipped.0, expected, "at {how:?}: {}", skipped.1);
            assert_eq!(skipped, default, "at {how:?}");
            assert_eq!(skipped.2, Some(0));
        }
        let (out, err, code) = ran("os-exit", IN_MAIN, on(how));
        assert_eq!((out.as_str(), code), ("before\n", Some(3)), "{err}");
    }
    let lowered = |how: Build| {
        let parsed = parse_to_ast(LEFT_TO_THE_OS).expect("the source parses");
        emit_program(&parsed, how).expect("the source lowers").rust
    };
    let default = lowered(Build::default());
    assert!(!default.contains("global_allocator"), "{default}");
    assert!(!default.contains("main_ends"), "{default}");
    let skipped = lowered(on(Build::default()));
    assert!(skipped.contains("#[global_allocator]"), "{skipped}");
    assert!(
        skipped.contains("nikaia_std::rt::exit::main_ends();"),
        "{skipped}"
    );
}

/// The word is checked like the other optimizations.
#[test]
fn skip_free_at_exit_is_on_or_off() {
    use nikaia::project::{optimizations, skip_free_at_exit};
    assert!(skip_free_at_exit("skip-free-at-exit:on"));
    assert!(!skip_free_at_exit(
        "skip-free-at-exit:on,skip-free-at-exit:off"
    ));
    assert!(!skip_free_at_exit("remove-bounds-checks:on"));
    assert!(optimizations("skip-free-at-exit:on").is_ok());
    assert!(optimizations("skip-free-at-exit:yes").is_err());
}
const SCOPE_BODY_THROWS: &str = r#"use std::task
use std::time

enum Bad { Oops(i64) }
impl Error for Bad {
    fn message(ref self) -> String { return "bad" }
}

fn work(n: i64) -> i64 throws {
    if n > 1 {
        throw Bad::Oops(n)
    }
    return n
}

fn main() throws {
    let base = 6
    let y = task::scope fn(s) {
        s.spawn fn { work(base - 5) }
        let k = work(base)
        k + 1
    } catch {
        match error {
            Bad::Oops(c) => c,
            else => 0,
        }
    }
    println(f"{y}")
}
"#;

/// ADR-346 D7: the scope's own function may throw; its error is the scope's,
/// after its tasks', at both settings, with tasks that pause and without.
#[test]
fn a_scope_whose_function_throws() {
    for how in [Build::default(), Build::parallel()] {
        let (out, err, code) = ran("scope-body-throws", SCOPE_BODY_THROWS, how);
        assert_eq!(code, Some(0), "{err}");
        assert_eq!(out, "6\n");
        let fine = SCOPE_BODY_THROWS.replace("work(base)", "work(1)");
        let (out, err, code) = ran("scope-body-fine", &fine, how);
        assert_eq!(code, Some(0), "{err}");
        assert_eq!(out, "2\n");
        let first = SCOPE_BODY_THROWS.replace("work(base - 5)", "work(base - 3)");
        let (out, err, code) = ran("scope-body-second", &first, how);
        assert_eq!(code, Some(0), "{err}");
        assert_eq!(out, "3\n", "the task's error comes first");
        let uncaught = SCOPE_BODY_THROWS.replace(
            "    } catch {\n        match error {\n            Bad::Oops(c) => c,\n            else => 0,\n        }\n    }\n",
            "    }\n",
        );
        let (out, err, code) = ran("scope-body-uncaught", &uncaught, how);
        assert_eq!(code, Some(1), "{err}");
        assert_eq!(out, "");
        assert!(err.contains("bad"), "{err}");
        let paused = SCOPE_BODY_THROWS.replace(
            "s.spawn fn { work(base - 5) }",
            "s.spawn fn {\n            time::sleep(1.millis())\n            work(base - 5)\n        }",
        );
        let (out, err, code) = ran("scope-body-paused", &paused, how);
        assert_eq!(code, Some(0), "{err}");
        assert_eq!(out, "6\n");
    }
}
