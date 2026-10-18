//! A grammar entered with a `Bytes`
//! ([ADR-366](../../../docs/specification/adr/adr-366.md) D1, #422).
//!
//! The argument decides: a call with a `Bytes` enters the grammar's copy over
//! bytes (winnow-grammar's `input bytes;`), and a grammar only ever entered
//! with text lowers as it always did.

mod common;

use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

const GRAMMAR: &str = "grammar Num {\n    entry rule num -> i64 = n:dec[i64](digit+) { n }\n}\n";

fn emit(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Build::default())
        .expect("the source lowers")
        .rust
}

#[test]
fn a_bytes_argument_enters_the_copy_over_bytes() {
    let source = format!(
        "use std::fs\n\n{GRAMMAR}\nfn read() -> i64 {{\n    let raw = fs::read(\"d\", fs::Root::Anywhere) catch {{ return 0 }}\n    return Num::num(raw) catch {{ 0 }}\n}}\n"
    );
    let rust = emit(&source);
    assert!(
        rust.contains("grammar NumOverBytes {\n        input bytes;"),
        "{rust}"
    );
    assert!(
        rust.contains("nikaia_std::grammar::parse_bytes(&*raw, NumOverBytes::parse_num())"),
        "{rust}"
    );
}

#[test]
fn a_grammar_entered_only_with_text_has_no_copy() {
    let source =
        format!("{GRAMMAR}\nfn read(t: String) -> i64 {{ return Num::num(t) catch {{ 0 }} }}\n");
    let rust = emit(&source);
    assert!(!rust.contains("OverBytes"), "{rust}");
    assert!(!rust.contains("input bytes"), "{rust}");
    assert!(
        rust.contains("nikaia_std::grammar::parse(&*t, Num::parse_num())"),
        "{rust}"
    );
}

#[test]
fn a_grammar_parses_what_fs_read_returned() {
    let dir = common::scratch_dir("grammar-bytes-fs-read");
    std::fs::write(dir.join("d"), "4217").expect("the input");
    let source = format!(
        "use std::fs\n\n{GRAMMAR}\nfn main() {{\n    let raw = fs::read(\"d\", fs::Root::Anywhere) catch {{ return }}\n    let t = fs::read_to_string(\"d\", fs::Root::Anywhere) catch {{ return }}\n    let a = Num::num(raw) catch {{ -1 }}\n    let b = Num::num(t) catch {{ -1 }}\n    print(f\"{{a}} {{b}}\")\n}}\n"
    );
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "4217 4217");
}
