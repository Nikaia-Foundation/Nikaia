//! **`std::process`: another program, started and waited for**
//! ([ADR-243](../../../docs/specification/adr/adr-243.md), on
//! [ADR-290](../../../docs/specification/adr/adr-290.md) D14).
//!
//! Every test here **runs a program**, at both settings of
//! `user_parallelism`, because what is claimed is behaviour: the exit code and
//! both streams arrive, a non-zero exit is an answer rather than a failure, a
//! program that is not there is a `catch`, and the wait gives the thread up.

mod common;

use nikaia::contracts::LedgerOps;
use std::process::Command;

use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = nikaia::contracts::Ledger::infer(&parsed);
    let library =
        nikaia::contracts::Ledger::parse(nikaia::contracts::STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
}

/// Lowered at `how`, compiled, and run in a directory of its own, which is
/// also where the program's children start.
fn ran(purpose: &str, source: &str, how: Build) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    let rust = emit_program(&parsed, how).expect("it lowers").rust;
    let dir = common::scratch_dir(&format!("process-{purpose}"));
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary)
        .current_dir(&dir)
        .env("NK_INHERITED", "inherited")
        .output()
        .expect("run it");
    assert!(
        out.status.success(),
        "{purpose} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_dir_all(&dir).ok();
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn runs(purpose: &str, source: &str, expected: &str) {
    let found = findings(source);
    assert!(found.is_empty(), "{purpose}: {found:#?}");
    for how in [Build::default(), Build::parallel()] {
        assert_eq!(ran(purpose, source, how), expected, "{purpose} at {how:?}");
    }
}

/// **The exit code and both streams arrive, and a non-zero exit is an answer.**
/// `exit 3` is not a failure the program has to `catch`: it is `code` and `ok`,
/// and what the tool printed on the way out is still there to read.
#[test]
fn a_program_runs_and_what_it_said_comes_back() {
    let source = "use std::process\n\
                  \n\
                  fn main() throws {\n\
                  \x20   let out = process::run(\"sh\", [\"-c\", \"echo hello; echo oops >&2; exit 3\"])\n\
                  \x20   println(f\"code={out.code} ok={out.ok}\")\n\
                  \x20   print(f\"out={out.stdout}\")\n\
                  \x20   print(f\"err={out.stderr}\")\n\
                  \x20   let fine = process::run(\"true\", [])\n\
                  \x20   println(f\"code={fine.code} ok={fine.ok}\")\n\
                  }\n";
    runs(
        "streams",
        source,
        "code=3 ok=false\nout=hello\nerr=oops\ncode=0 ok=true\n",
    );
}

/// **`dir` is where the child starts**, and the arguments reach it one by one:
/// a space inside one is not a second argument, because there is no shell in
/// between unless the program asks for one.
#[test]
fn the_directory_and_the_arguments_are_the_callers() {
    let source = "use std::process\n\
                  \n\
                  fn main() throws {\n\
                  \x20   let here = process::run(\"pwd\", []; dir: \"/\")\n\
                  \x20   print(here.stdout)\n\
                  \x20   let mut args: Vec[String] = [\"-c\", \"echo $#: $1\", \"sh\"]\n\
                  \x20   args.push(\"one two\")\n\
                  \x20   args.push(\"three\")\n\
                  \x20   let counted = process::run(\"sh\", args)\n\
                  \x20   print(counted.stdout)\n\
                  }\n";
    runs("dir-args", source, "/\n2: one two\n");
}

/// **A program that is not there is a `catch`**, with the name that was looked
/// for in it, as `fs` says which path was not found.
#[test]
fn a_program_that_is_not_there_is_caught() {
    let source = "use std::process\n\
                  use std::io\n\
                  \n\
                  fn main() {\n\
                  \x20   let out = process::run(\"no-such-program-anywhere\", []) catch {\n\
                  \x20       match error {\n\
                  \x20           io::IoError::NotFound(what) => println(f\"not found: {what}\"),\n\
                  \x20           else => println(\"something else\"),\n\
                  \x20       }\n\
                  \x20       return\n\
                  \x20   }\n\
                  \x20   println(f\"ran: {out.code}\")\n\
                  }\n";
    runs("missing", source, "not found: no-such-program-anywhere\n");
}

/// **The wait gives the thread up** (D3), which is the claim a timing test
/// could only suggest.
///
/// At `user_parallelism = no` there is one thread for user code. The first
/// branch starts a child that waits - bounded, so a failure is an answer and
/// not a hang - for a file the second branch's child makes. If waiting for the
/// first child held the thread, the second branch could not start its child
/// until the first gave up, and the first would say `timed out`.
#[test]
fn waiting_for_a_program_gives_the_thread_up() {
    let source = "use std::process\n\
                  \n\
                  fn main() throws {\n\
                  \x20   let (waited, made) = overlap {\n\
                  \x20       process::run(\"sh\", [\"-c\", \"i=0; while [ ! -f marker ]; do i=$((i+1)); if [ $i -gt 500 ]; then echo timed out; exit 1; fi; sleep 0.01; done; echo saw it\"])\n\
                  \x20       process::run(\"sh\", [\"-c\", \"sleep 0.2; touch marker\"])\n\
                  \x20   }\n\
                  \x20   print(waited.stdout)\n\
                  \x20   println(f\"{made.ok}\")\n\
                  }\n";
    runs("gives-up", source, "saw it\ntrue\n");
}

/// **What the run cost** ([ADR-368](../../../docs/specification/adr/adr-368.md)
/// D3): the time it took on the monotonic clock, the processor time, and on
/// Linux the most memory that one child held, which the kernel always reports.
#[cfg(target_os = "linux")]
#[test]
fn what_the_run_cost_comes_back() {
    let source = "use std::process\n\
                  \n\
                  fn main() throws {\n\
                  \x20   let out = process::run(\"sh\", [\"-c\", \"sleep 0.02\"])\n\
                  \x20   let took = out.elapsed.in_nanos()\n\
                  \x20   let rss = out.max_rss_kib ?? 0\n\
                  \x20   let busy = out.user_time.in_nanos() + out.system_time.in_nanos()\n\
                  \x20   println(f\"waited={took >= 20000000} rss={rss > 0} busy={busy >= 0}\")\n\
                  \x20   let fine = process::run(\"true\", [])\n\
                  \x20   println(f\"elapsed={fine.elapsed.in_nanos() > 0} rss={(fine.max_rss_kib ?? 0) > 0}\")\n\
                  }\n";
    runs(
        "costs",
        source,
        "waited=true rss=true busy=true\nelapsed=true rss=true\n",
    );
}

/// **The child's environment** (ADR-243 D5, D6): `env` sets, a later pair
/// winning; `env_remove` takes an inherited variable out; `env_clear` with
/// `env` leaves only what was given; `env_remove` after `env_clear` neither
/// fails nor matters; a name both removed and set is set; and a `PATH` given
/// to the child does not change where the program is found.
#[test]
fn the_childs_environment_is_the_callers_changed_in_order() {
    let source = r#"use std::process

fn main() throws {
    let a = process::run("printenv", ["NK_A"]; env: [("NK_A", "one")])
    print(a.stdout)
    let b = process::run("printenv", ["NK_A"]; env: [("NK_A", "one"), ("NK_A", "two")])
    print(b.stdout)
    let kept = process::run("printenv", ["NK_INHERITED"])
    print(kept.stdout)
    let c = process::run("printenv", ["NK_INHERITED"]; env_remove: ["NK_INHERITED"])
    println(f"removed={c.code}")
    let d = process::run("/usr/bin/env", []; env_clear: true, env: [("ONLY", "1")])
    print(d.stdout)
    let e = process::run("/usr/bin/env", []; env_clear: true, env_remove: ["NK_INHERITED", "NOPE"])
    println(f"cleared={e.code} [{e.stdout}]")
    let f = process::run("printenv", ["NK_INHERITED"]; env_remove: ["NK_INHERITED"], env: [("NK_INHERITED", "set")])
    print(f.stdout)
    let g = process::run("printenv", ["PATH"]; env: [("PATH", "/nonexistent")])
    print(g.stdout)
}
"#;
    runs(
        "env",
        source,
        "one\ntwo\ninherited\nremoved=1\nONLY=1\ncleared=0 []\nset\n/nonexistent\n",
    );
}

/// **An entry no platform accepts starts nothing** (ADR-243 D7): both invalid
/// entries are named in the one `io::IoError::Other`, and the child that would
/// have made the marker never ran.
#[test]
fn every_invalid_entry_is_named_and_nothing_starts() {
    let source = r#"use std::process
use std::io

fn main() throws {
    let name: String = "A=B"
    let mut pairs: Vec[(String, String)] = []
    pairs.push((name, "x".clone()))
    pairs.push(("", "y"))
    let h = process::run("touch", ["marker"]; env: pairs) catch {
        match error {
            io::IoError::Other(what) => println(f"refused: {what}"),
            else => println("something else"),
        }
        let seen = process::run("sh", ["-c", "test -e marker && echo started || echo none"])
        print(seen.stdout)
        return
    }
    println(f"ran {h.code}")
}
"#;
    runs(
        "env-invalid",
        source,
        "refused: touch: the environment has entries no platform accepts: \
         the name \"A=B\" in `env` holds `=`; the name \"\" in `env` is empty\nnone\n",
    );
}

/// **`NK1286`: an invalid entry written as a literal is refused while
/// building** (ADR-243 D7), each at its own place.
#[test]
fn a_literal_invalid_entry_is_refused() {
    let source = "use std::process\n\
                  \n\
                  fn main() throws {\n\
                  \x20   let a = process::run(\"true\", []; env: [(\"A=B\", \"x\"), (\"\", \"x\"), (\"OK\", \"fine\")], env_remove: [\"GONE\"])\n\
                  \x20   println(a.code)\n\
                  }\n";
    let refused: Vec<String> = findings(source)
        .into_iter()
        .filter(|f| f.code == "NK1286")
        .map(|f| f.message)
        .collect();
    assert_eq!(
        refused,
        [
            "The environment name `A=B` holds `=`.",
            "An environment name is empty.",
        ],
        "{refused:#?}"
    );
}
