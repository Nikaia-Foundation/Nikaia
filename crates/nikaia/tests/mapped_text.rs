//! A mapping is bytes, and becomes text where it is read as text
//! ([ADR-016](../../../docs/specification/adr/adr-016.md) D5, #710).
//!
//! `fs::map` checks nothing; `m.text()` checks the whole mapping once and
//! throws `NotText` with the earliest bad byte; a cut checks only itself; and a
//! grammar run over a mapping reports a byte that is not UTF-8 at its position,
//! whether the pieces run one after another or in parallel.

mod common;

use std::process::{Command, Output};

/// Runs `source` as `main.nika` beside `files`, at `parallel`.
fn run(name: &str, source: &str, files: &[(&str, &[u8])], parallel: &str) -> Output {
    let dir = common::scratch_dir(name);
    for (file, content) in files {
        std::fs::write(dir.join(file), content).expect("the input");
    }
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let run = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "--user-parallelism", parallel, "main.nika"])
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    run
}

fn printed(run: &Output) -> String {
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

const READS: &str = "use std::fs\n\
\n\
fn main() {\n\
\x20   let m = fs::map(\"d\", fs::Root::Anywhere) catch {\n\
\x20       println(f\"map: {error}\")\n\
\x20       return\n\
\x20   }\n\
\x20   println(f\"{m.len()} {m[0]} {m[10]} {m.u64_le(0)}\")\n\
\x20   let head = m[0..<2].text() catch {\n\
\x20       println(f\"cut: {error}\")\n\
\x20       return\n\
\x20   }\n\
\x20   println(head)\n\
\x20   let whole = m.text() catch {\n\
\x20       println(f\"text: {error}\")\n\
\x20       return\n\
\x20   }\n\
\x20   println(whole)\n\
}\n";

/// A file that is not text maps, is read as bytes (`len`, `b[i]`, `u64_le`),
/// gives the text of a cut before its bad byte, and `text()` names that byte.
#[test]
fn a_mapping_is_bytes_and_its_text_names_the_bad_byte() {
    let out = printed(&run(
        "mapped-bytes",
        READS,
        &[("d", b"ab\x01\0\0\0\0\0;;\xff!")],
        "no",
    ));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        format!("12 97 255 {}", u64::from_le_bytes(*b"ab\x01\0\0\0\0\0"))
    );
    assert_eq!(lines[1], "ab");
    assert!(lines[2].starts_with("text: "), "{out}");
    assert!(lines[2].contains("(at byte 10)"), "{out}");
    assert_eq!(lines.len(), 3, "{out}");
}

/// The text of a mapping that is text, and an empty file, which maps too.
#[test]
fn a_mapping_that_is_text_reads_as_text() {
    let out = printed(&run(
        "mapped-text",
        READS,
        &[("d", "abcdefghijkl\u{e9}".as_bytes())],
        "no",
    ));
    assert_eq!(out.lines().nth(1), Some("ab"), "{out}");
    assert_eq!(out.lines().nth(2), Some("abcdefghijkl\u{e9}"), "{out}");
}

const EMPTY: &str = "use std::fs\n\
\n\
fn main() throws {\n\
\x20   let m = fs::map(\"d\", fs::Root::Anywhere)\n\
\x20   let t = m.text()\n\
\x20   println(f\"{m.len()}[{t}]\")\n\
}\n";

#[test]
fn an_empty_file_maps_and_is_empty_text() {
    assert_eq!(
        printed(&run("mapped-empty", EMPTY, &[("d", b"")], "no")),
        "0[]"
    );
}

const BRC: &str = include_str!("../../../examples/1brc.nika");

/// A grammar driven in pieces over a mapping reports a byte that is not UTF-8
/// as an error at its position, one after another and in parallel.
#[test]
fn a_grammar_over_a_mapping_reports_the_bad_byte_where_it_is() {
    for parallel in ["no", "yes"] {
        let ran = run(
            &format!("mapped-brc-{parallel}"),
            BRC,
            &[("measurements.txt", b"Hamburg;12.0\nBa\xffd;1.0\nOslo;3.0\n")],
            parallel,
        );
        let said = String::from_utf8_lossy(&ran.stderr);
        assert!(!ran.status.success(), "{parallel}: {said}");
        assert!(
            said.contains("invalid UTF-8 at byte 15: 0xff"),
            "{parallel}: {said}"
        );
        assert!(
            said.contains("at line 2, column 3"),
            "{parallel}, the line: {said}"
        );
    }
}

const WHOLE: &str = "use std::fs\n\
\n\
grammar Words {\n\
\x20   entry rule words -> i64 = w:any* { w.len() }\n\
}\n\
\n\
fn main() {\n\
\x20   let m = fs::map(\"d\", fs::Root::Anywhere) catch { return }\n\
\x20   let n = Words::words(m) catch {\n\
\x20       println(f\"{error}\")\n\
\x20       return\n\
\x20   }\n\
\x20   println(f\"{n}\")\n\
}\n";

/// A grammar entered whole with a mapping: text is parsed as before, and a bad
/// byte is a parse error at its position.
#[test]
fn a_grammar_entered_with_a_mapping_checks_its_text() {
    assert_eq!(
        printed(&run("mapped-words", WHOLE, &[("d", b"abc")], "no")),
        "3"
    );
    let out = printed(&run(
        "mapped-words-bad",
        WHOLE,
        &[("d", b"ab\n\xffc")],
        "no",
    ));
    assert!(out.contains("invalid UTF-8 at byte 3: 0xff"), "{out}");
}
