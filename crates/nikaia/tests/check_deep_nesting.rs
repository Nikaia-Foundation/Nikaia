//! **A deeply nested expression checks on the default stack** (ADR-294, #125,
//! #558, C-6): the checker's core crosses from Nikaia code into Rust and back
//! at every `expr`, and each crossing is a frame. The binary runs on the main
//! thread's 8 MB stack, so this test runs it there and asks for no more.
//!
//! The shapes are the ones the parser takes: nested calls, nested lists, a
//! chain of `??` and a chain of method calls. **Parentheses, `if`s and blocks
//! are not used**: the parser is exponential on them (16 levels take seconds),
//! which is a defect of its own and not this test's question.
//!
//! The limits today (parser and checker together): calls pass 800, lists 500,
//! a `.abs()` chain 1,600. The test takes 500, 500, 500 and 1,000.

use std::process::Command;

fn lowers(source: &str, name: &str) {
    let dir = std::env::temp_dir().join(format!("nikaia-deep-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    let file = dir.join(format!("{name}.nika"));
    std::fs::write(&file, source).expect("write the source");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .arg("lower")
        .arg(&file)
        .arg("--no-cache")
        .output()
        .expect("the compiler runs");
    let _ = std::fs::remove_dir_all(&dir);
    let said = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "{name} lowers on the default stack, got {:?}: {}",
        out.status,
        said.lines().take(5).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn nested_calls_check_on_the_default_stack() {
    let depth = 500;
    let call = format!("{}1{}", "id(".repeat(depth), ")".repeat(depth));
    lowers(
        &format!(
            "fn id(x: i64) -> i64 {{\n    return x\n}}\n\nfn main() {{\n    let a: i64 = {call}\n    println(f\"{{a}}\")\n}}\n"
        ),
        "calls",
    );
}

#[test]
fn nested_lists_check_on_the_default_stack() {
    let depth = 500;
    let list = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
    lowers(
        &format!("fn main() {{\n    let a = {list}\n    println(f\"{{a.len()}}\")\n}}\n"),
        "lists",
    );
}

#[test]
fn a_coalesce_chain_checks_on_the_default_stack() {
    let chain = "b ?? ".repeat(500);
    lowers(
        &format!(
            "fn main() {{\n    let b: i64? = null\n    let c: i64 = {chain}3\n    println(f\"{{c}}\")\n}}\n"
        ),
        "coalesce",
    );
}

#[test]
fn a_method_chain_checks_on_the_default_stack() {
    let chain = ".abs()".repeat(1000);
    lowers(
        &format!(
            "fn main() {{\n    let a: i64 = 1\n    let d: i64 = a{chain}\n    println(f\"{{d}}\")\n}}\n"
        ),
        "chain",
    );
}
