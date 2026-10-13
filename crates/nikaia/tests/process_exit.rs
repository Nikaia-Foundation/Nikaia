// crates/nikaia/tests/process_exit.rs
//
// `process::exit(code)` ([ADR-343](../../../docs/specification/adr/adr-343.md)):
// `main` ends with the code on the path a return from `main` takes, so the
// cleanups run (D1); code that may end the program is refused in a task,
// directly and over calls (`NK2607`, D2, D3); a constant code outside 0..=255
// is refused (`NK1116`, D4).

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

const IN_MAIN: &str = "use std::process\n\
    \n\
    fn main() {\n\
    \x20   println(\"before\")\n\
    \x20   process::exit(3)\n\
    \x20   println(\"after\")\n\
    }\n";

/// D1: the program ends with the code, and nothing after the call runs.
#[test]
fn exit_in_main_ends_with_its_code() {
    let (out, err, code) = ran("main", IN_MAIN, Build::default());
    assert_eq!(code, Some(3), "{err}");
    assert_eq!(out, "before\n");
    assert!(codes(IN_MAIN).is_empty(), "{:?}", codes(IN_MAIN));
}

const CLEANED: &str = "use std::process\n\
    \n\
    struct Noted {\n\
    \x20   name: String,\n\
    }\n\
    \n\
    impl Cleanup for Noted {\n\
    \x20   fn cleanup(ref mut self) {\n\
    \x20       println(f\"cleanup {self.name}\")\n\
    \x20   }\n\
    }\n\
    \n\
    fn stop() {\n\
    \x20   let inner = Noted { name: f\"inner\" }\n\
    \x20   println(f\"stopping {inner.name}\")\n\
    \x20   process::exit(4)\n\
    }\n\
    \n\
    fn main() {\n\
    \x20   let outer = Noted { name: f\"outer\" }\n\
    \x20   println(f\"start {outer.name}\")\n\
    \x20   stop()\n\
    }\n";

/// D1: the values `main` and the frames below it hold are cleaned up, in the
/// order they die, before the process ends with the code.
#[test]
fn exit_runs_the_cleanups() {
    for how in [Build::default(), Build::parallel()] {
        let (out, err, code) = ran("cleaned", CLEANED, how);
        assert_eq!(code, Some(4), "{err}");
        assert_eq!(
            out, "start outer\nstopping inner\ncleanup inner\ncleanup outer\n",
            "at {how:?}"
        );
    }
}

/// D3: `exit` in a `spawn` is `NK2607`.
#[test]
fn exit_in_a_spawn_is_refused() {
    let source = "use std::process\n\
        \n\
        fn main() {\n\
        \x20   spawn fn { process::exit(1) }\n\
        }\n";
    assert!(
        codes(source).contains(&"NK2607".to_string()),
        "{:?}",
        codes(source)
    );
}

/// D2, D3: two calls deep is the same refusal; the ledger carries `exits`.
#[test]
fn exit_two_calls_deep_in_a_spawn_is_refused() {
    let source = "use std::process\n\
        \n\
        fn inner() {\n\
        \x20   process::exit(2)\n\
        }\n\
        \n\
        fn outer() {\n\
        \x20   inner()\n\
        }\n\
        \n\
        fn main() {\n\
        \x20   spawn fn { outer() }\n\
        }\n";
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    assert!(own.functions["outer"].exits, "outer reaches exit");
    assert!(
        codes(source).contains(&"NK2607".to_string()),
        "{:?}",
        codes(source)
    );
}

/// D3: a branch of `overlap` is a task too.
#[test]
fn exit_in_an_overlap_branch_is_refused() {
    let source = "use std::process\n\
        \n\
        fn stop() {\n\
        \x20   process::exit(1)\n\
        }\n\
        \n\
        fn main() {\n\
        \x20   overlap {\n\
        \x20       stop()\n\
        \x20       println(\"other\")\n\
        \x20   }\n\
        }\n";
    assert!(
        codes(source).contains(&"NK2607".to_string()),
        "{:?}",
        codes(source)
    );
}

/// D4: a constant code outside 0..=255 is refused.
#[test]
fn a_constant_code_out_of_range_is_refused() {
    let source = "use std::process\n\
        \n\
        fn main() {\n\
        \x20   process::exit(300)\n\
        }\n";
    assert!(
        codes(source).contains(&"NK1116".to_string()),
        "{:?}",
        codes(source)
    );
}
