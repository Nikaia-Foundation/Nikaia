//! A grammar entered with a `Bytes`
//! ([ADR-366](../../../docs/specification/adr/adr-366.md) D1, #422).
//!
//! The argument decides: a call with a `Bytes` enters the grammar's copy over
//! bytes (winnow-grammar's `input bytes;`), and a grammar only ever entered
//! with text lowers as it always did.

mod common;

use nikaia::contracts::{Ledger, LedgerOps, STD};
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

/// Runs `source` as `main.nika` in a scratch directory holding `files`, and
/// hands back what it printed.
fn run(name: &str, source: &str, files: &[(&str, &[u8])]) -> String {
    let dir = common::scratch_dir(name);
    for (file, content) in files {
        std::fs::write(dir.join(file), content).expect("the input");
    }
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
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

const PAIR: &str = "grammar Pair {\n    entry rule pair -> i64 = name:until(\";\") sep:\";\" rest:any* { name.len() * 100 + sep.len() * 10 + rest.len() }\n}\n";

/// What Nikaia types as text is checked text over bytes (ADR-366 D2): an
/// `until` and a literal's capture are taken through the backend's
/// `text(...)`, and a run of `any` is the backend's checked text by itself,
/// so the action written for text compiles unchanged.
#[test]
fn what_is_text_over_bytes_is_checked_text() {
    let source = format!(
        "use std::fs\n\n{PAIR}\nfn main() {{\n    let raw = fs::read(\"d\", fs::Root::Anywhere) catch {{ return }}\n    let t = fs::read_to_string(\"d\", fs::Root::Anywhere) catch {{ return }}\n    let a = Pair::pair(raw) catch {{ -1 }}\n    let b = Pair::pair(t) catch {{ -1 }}\n    print(f\"{{a}} {{b}}\")\n}}\n"
    );
    let rust = emit(&source);
    assert!(
        rust.contains("name:text(until(\";\")) sep:text(\";\") rest:any*"),
        "{rust}"
    );
    assert!(
        rust.contains("name:until(\";\") sep:\";\" rest:any*"),
        "the text copy is as before: {rust}"
    );
    let out = run("grammar-bytes-text", &source, &[("d", "ab;cd".as_bytes())]);
    assert_eq!(out, "212 212");
}

const REC: &str = "grammar Rec {\n    entry rule rec -> i64 = n:dec[i64](digit+) \":\" body:bytes(any*) { n * 1000 + body.len() }\n}\n";

/// `bytes(p)` hands back the bytes `p` covered, over either input (ADR-366
/// D4): the backend's `bytes(p)` over bytes, `text(p)` seen as its bytes over
/// text, where the backend has none.
#[test]
fn bytes_of_a_pattern_over_either_input() {
    let source = format!(
        "use std::fs\n\n{REC}\nfn main() {{\n    let raw = fs::read(\"d\", fs::Root::Anywhere) catch {{ return }}\n    let t = fs::read_to_string(\"d\", fs::Root::Anywhere) catch {{ return }}\n    let a = Rec::rec(raw) catch {{ -1 }}\n    let b = Rec::rec(t) catch {{ -1 }}\n    print(f\"{{a}} {{b}}\")\n}}\n"
    );
    let rust = emit(&source);
    // Under `bytes(...)` nothing is checked: `any` is the backend's raw `byte`.
    assert!(rust.contains("body:bytes(byte*)"), "{rust}");
    assert!(rust.contains("body:text(any*)"), "{rust}");
    assert!(rust.contains("let body = body.as_bytes();"), "{rust}");
    let out = run("grammar-bytes-of", &source, &[("d", "3:abc".as_bytes())]);
    assert_eq!(out, "3003 3003");
}

fn codes(source: &str) -> Vec<String> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .map(|f| f.code.to_string())
        .collect()
}

/// An entry handed neither a `String` nor a `Bytes` is `NK1282` (ADR-366 D1);
/// either of those is not.
#[test]
fn an_entry_takes_text_or_bytes_only() {
    let refused =
        format!("{GRAMMAR}\nfn read(n: i64) -> i64 {{ return Num::num(n) catch {{ 0 }} }}\n");
    assert!(
        codes(&refused).iter().any(|c| c == "NK1282"),
        "{:?}",
        codes(&refused)
    );
    let list =
        format!("{GRAMMAR}\nfn read(n: Vec[u8]) -> i64 {{ return Num::num(n) catch {{ 0 }} }}\n");
    assert!(
        codes(&list).iter().any(|c| c == "NK1282"),
        "{:?}",
        codes(&list)
    );
    let text =
        format!("{GRAMMAR}\nfn read(t: String) -> i64 {{ return Num::num(t) catch {{ 0 }} }}\n");
    assert!(
        !codes(&text).iter().any(|c| c == "NK1282"),
        "{:?}",
        codes(&text)
    );
    let bytes =
        format!("{GRAMMAR}\nfn read(b: Bytes) -> i64 {{ return Num::num(b) catch {{ 0 }} }}\n");
    assert!(
        !codes(&bytes).iter().any(|c| c == "NK1282"),
        "{:?}",
        codes(&bytes)
    );
}

/// **A binary record** read with `fs::read`: a length, then that many bytes
/// that are not UTF-8, kept by `bytes(p)` (ADR-366 D4).
#[test]
fn a_length_prefixed_binary_record() {
    let source = "use std::fs\n\ngrammar Record {\n    entry rule record -> i64 = \"REC\" n:dec[i64](digit+) \":\" body:bytes(any*) { if body.len() == n { n * 10 } else { -1 } }\n}\n\nfn main() {\n    let raw = fs::read(\"d\", fs::Root::Anywhere) catch { return }\n    let r = Record::record(raw) catch { -2 }\n    println(f\"{r}\")\n}\n";
    let out = run(
        "grammar-bytes-record",
        source,
        &[("d", b"REC4:\x00\xff\xfe\x80")],
    );
    assert_eq!(out, "40");
}

const ROW: &str = "use std::fs\n\ngrammar Row {\n    rule NAME -> ref String = s:until(\";\") { s }\n    entry rule row -> i64 = name:NAME \";\" t:dec[i64](digit+) { name.len() + t }\n}\n\nfn main() {\n    let raw = fs::read(\"d\", fs::Root::Anywhere) catch { return }\n    let r = Row::row(raw) catch {\n        println(f\"{error}\")\n        return\n    }\n    println(f\"{r}\")\n}\n";

/// The first line of what the program printed.
fn first_line(out: &str) -> &str {
    out.lines().next().unwrap_or("")
}

/// **A bad byte in an `until` is a parse error at its own offset** (ADR-366
/// D3), at the start, in the middle and at the end of the name.
#[test]
fn a_bad_byte_in_a_name_is_named_where_it_stands() {
    for (input, at) in [
        (&b"\xffab;12"[..], 0),
        (&b"a\xffb;12"[..], 1),
        (&b"ab\xff;12"[..], 2),
    ] {
        let out = run("grammar-bytes-bad-name", ROW, &[("d", input)]);
        assert!(
            first_line(&out).starts_with(&format!("invalid UTF-8 at byte {at}: 0xff")),
            "{out}"
        );
    }
    let out = run("grammar-bytes-good-name", ROW, &[("d", b"ab;12")]);
    assert_eq!(out, "14");
}

/// **A bad byte inside a `digit` run is the ordinary parse error**: a digit is
/// matched by comparison, so nothing there is checked as UTF-8 (ADR-366 D2).
#[test]
fn a_bad_byte_in_a_digit_run_is_an_ordinary_parse_error() {
    let out = run("grammar-bytes-bad-digit", ROW, &[("d", b"ab;1\xff2")]);
    assert!(!out.contains("invalid UTF-8"), "{out}");
    assert!(!out.is_empty() && out != "15", "a parse error: {out}");
}

/// **Under `par_fold` the offset is the whole input's**, in a piece other than
/// the first too (ADR-366 D3); good input gives the same sum over text and
/// over bytes.
#[test]
fn a_bad_byte_in_a_later_piece_counts_from_the_start() {
    let source = "use std::fs\n\ngrammar Sum {\n    rule NAME -> ref String = s:until(\";\" | frame_end) { s }\n\n    @frame(boundary: \"\\n\")\n    rule ROW -> i64 = name:NAME \";\" => n:dec[i64](digit+) frame_end { n + name.len() }\n\n    entry rule total -> i64 = par_fold(ROW, zero, fn(acc, m) { acc + m }, add)\n}\n\nfn zero() -> i64 { return 0 }\nfn add(a: i64, b: i64) -> i64 { return a + b }\n\nfn main() {\n    let ok = fs::read(\"ok\", fs::Root::Anywhere) catch { return }\n    let okt = fs::read_to_string(\"ok\", fs::Root::Anywhere) catch { return }\n    let a = Sum::total(ok) catch { -1 }\n    let b = Sum::total(okt) catch { -1 }\n    println(f\"{a} {b}\")\n    let raw = fs::read(\"d\", fs::Root::Anywhere) catch { return }\n    let r = Sum::total(raw) catch {\n        println(f\"{error}\")\n        return\n    }\n    println(f\"{r}\")\n}\n";
    let rows: Vec<u8> = (0..4000)
        .flat_map(|i| format!("r{i};{}\n", i % 10).into_bytes())
        .collect();
    // Past seven eighths of the input: in the last piece wherever the
    // machine has more than one core.
    let at = rows
        .windows(6)
        .position(|w| w == b"r3500;")
        .expect("the row")
        + 2;
    let mut bad = rows.clone();
    bad.insert(at, 0xff);
    let out = run(
        "grammar-bytes-pieces",
        source,
        &[("ok", &rows), ("d", &bad)],
    );
    let mut lines = out.lines();
    assert_eq!(lines.next(), Some("36890 36890"), "{out}");
    assert!(
        lines
            .next()
            .is_some_and(|l| l.starts_with(&format!("invalid UTF-8 at byte {at}: 0xff"))),
        "{out}"
    );
}

const FIRST: &str = "use std::fs\n\ngrammar First {\n    entry rule first -> i64 = \"<\" c:any \";\" { if c == '\u{20AC}' { 3 } else if c == '\u{E4}' { 2 } else { 1 } }\n}\n\nfn main() {\n    let raw = fs::read(\"d\", fs::Root::Anywhere) catch { return }\n    let a = First::first(raw) catch {\n        println(f\"{error}\")\n        return\n    }\n    let t = fs::read_to_string(\"d\", fs::Root::Anywhere) catch {\n        println(f\"{a}\")\n        return\n    }\n    let b = First::first(t) catch { -1 }\n    println(f\"{a} {b}\")\n}\n";

/// **A lone `any` is one checked character over bytes too** (ADR-366 D2): the
/// action that compares it with a character literal reads the same over a
/// `String` and over its `Bytes`, for a multi-byte character as well, and a
/// bad or truncated sequence is `invalid UTF-8 at byte N` where it stands.
#[test]
fn a_lone_any_is_a_checked_character_over_bytes() {
    for (input, want) in [("<a;", "1 1"), ("<\u{E4};", "2 2"), ("<\u{20AC};", "3 3")] {
        let out = run("grammar-bytes-any", FIRST, &[("d", input.as_bytes())]);
        assert_eq!(out.trim(), want, "{input}");
    }
    for (input, byte) in [(&b"<\xff;"[..], "0xff"), (&b"<\xe2\x82"[..], "0xe2")] {
        let out = run("grammar-bytes-any-bad", FIRST, &[("d", input)]);
        assert!(
            first_line(&out).starts_with(&format!("invalid UTF-8 at byte 1: {byte}")),
            "{out}"
        );
    }
}
