//! **A failure below the guarded call is the handler's too** (Kap 7.1,
//! #722): a call that can fail nested as an argument, a receiver or an
//! operand of what a `catch` guards carries its error to that `catch`, as
//! the guarded call's own does. Each program is compiled and **run** at both
//! settings of `user_parallelism`.

mod common;

use nikaia::contracts::LedgerOps;
use std::process::Command;

use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn ran(purpose: &str, source: &str, how: Build) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    let rust = emit_program(&parsed, how).expect("the source lowers").rust;
    let dir = common::scratch_dir(&format!("catch-nested-{purpose}"));
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
    assert!(
        out.status.success(),
        "{purpose} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    std::fs::remove_dir_all(&dir).ok();
    printed
}

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = nikaia::contracts::Ledger::infer(&parsed);
    let library =
        nikaia::contracts::Ledger::parse(nikaia::contracts::STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

/// Both settings, the same output, and no refusal on the way.
fn runs(purpose: &str, source: &str, expected: &str) {
    assert!(
        findings(source).is_empty(),
        "{purpose}: {:#?}",
        findings(source)
    );
    for how in [Build::default(), Build::parallel()] {
        assert_eq!(ran(purpose, source, how), expected, "{purpose} at {how:?}");
    }
}

const BAD: &str = r#"struct Bad { code: i64 }
impl Error for Bad {
    fn message(ref self) -> String { f"bad {self.code}" }
}
"#;

/// The issue's probe at three inputs: nothing fails, the argument fails, the
/// call around it fails.
#[test]
fn a_failing_argument_reaches_the_handler() {
    let source = format!(
        r#"{BAD}
fn inner(x: i64) -> i64 throws {{
    if x < 0 {{ throw Bad {{ code: 1 }} }}
    x
}}
fn outer(x: i64) -> i64 throws {{
    if x > 100 {{ throw Bad {{ code: 2 }} }}
    x + 1
}}
fn show(x: i64) {{
    let v = outer(inner(x)) catch {{
        println(f"caught {{error}}")
        return
    }}
    println(f"{{v}}")
}}
fn main() {{
    show(5)
    show(-1)
    show(500)
}}
"#
    );
    runs("argument", &source, "6\ncaught bad 1\ncaught bad 2");
}

/// `a().b() catch`: the receiver fails, the method does not; inside a
/// function that `throws` itself, the handler still takes it.
#[test]
fn a_failing_receiver_reaches_the_handler() {
    let source = format!(
        r#"{BAD}
struct Held {{ n: i64 }}
impl Held {{
    fn get(ref self) -> i64 {{ self.n }}
}}
fn make(x: i64) -> Held throws {{
    if x < 0 {{ throw Bad {{ code: 3 }} }}
    Held {{ n: x }}
}}
fn read(x: i64) -> i64 throws {{
    let v = make(x).get() catch {{
        println(f"caught {{error}}")
        return 0
    }}
    v
}}
fn main() {{
    println(f"{{read(7) catch {{ -1 }}}}")
    println(f"{{read(-7) catch {{ -1 }}}}")
}}
"#
    );
    runs("receiver", &source, "7\ncaught bad 3\n0");
}

/// `f() + g() catch`: either operand's failure is the handler's.
#[test]
fn a_failing_operand_reaches_the_handler() {
    let source = format!(
        r#"{BAD}
fn f(x: i64) -> i64 throws {{
    if x < 0 {{ throw Bad {{ code: x }} }}
    x
}}
fn sum(a: i64, b: i64) -> i64 {{
    f(a) + f(b) catch {{
        println(f"caught {{error}}")
        0
    }}
}}
fn main() {{
    println(f"{{sum(1, 2)}}")
    println(f"{{sum(-4, 2)}}")
    println(f"{{sum(1, -5)}}")
}}
"#
    );
    runs("operand", &source, "3\ncaught bad -4\n0\ncaught bad -5\n0");
}

/// A nested call of `std` that can fail and pauses: its failure, too, is the
/// handler's.
#[test]
fn a_failing_std_call_below_reaches_the_handler() {
    let source = format!(
        r#"use std::fs
{BAD}
fn outer(x: i64) -> i64 throws {{
    if x > 100 {{ throw Bad {{ code: 4 }} }}
    x + 1
}}
fn main() {{
    let v = outer(fs::read_to_string("missing.txt", fs::Root::Anywhere).len()) catch {{
        println("caught")
        return
    }}
    println(f"{{v}}")
}}
"#
    );
    runs("std", &source, "caught");
}
