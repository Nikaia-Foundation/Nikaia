//! **A `Bytes` carries what owns its memory; a `ref Bytes` only read takes any
//! run of bytes** ([ADR-373](../../../docs/specification/adr/adr-373.md), #718).
//!
//! `Bytes(v)` hands a `Vec[u8]` over (D2); a mapping is a `Bytes` where one is
//! taken by value - an argument, a field, an element, a `let` - at a count
//! (D3); and one function that only reads takes a `Bytes`, a mapping and a
//! text's bytes alike (D4).

mod common;

use std::process::Command;

/// Runs `source` as `main.nika` beside a file `in.txt` of three lines.
fn printed(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("in.txt"), "a\nb\nc\n").expect("the input");
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let run = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "main.nika"])
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

/// The issue's probe: a mapping handed to a function that reads a `ref Bytes`.
#[test]
fn a_mapping_reaches_a_function_that_reads_bytes() {
    let out = printed(
        "bytes-owner-probe",
        "use std::fs\n\
         fn lines(b: ref Bytes) -> i64 { let mut n = 0; for i in 0..<b.len() { if b[i] == 10 { n += 1 } }; n }\n\
         fn main() throws {\n\
         \x20   let m = fs::map(\"in.txt\", fs::Root::Anywhere)\n\
         \x20   println(f\"{lines(m)}\")\n\
         }\n",
    );
    assert_eq!(out, "3");
}

/// One reading function, four owners: a `Bytes(v)`, a mapping, a text's bytes
/// and a `Bytes` that `fs::read` made.
#[test]
fn one_reader_takes_every_run_of_bytes() {
    let out = printed(
        "bytes-owner-readers",
        "use std::fs\n\
         fn lines(b: ref Bytes) -> i64 { let mut n = 0; for i in 0..<b.len() { if b[i] == 10 { n += 1 } }; n }\n\
         fn main() throws {\n\
         \x20   let mut v: Vec[u8] = []\n\
         \x20   v.push(65)\n\
         \x20   v.push(10)\n\
         \x20   let own = Bytes(v)\n\
         \x20   let m = fs::map(\"in.txt\", fs::Root::Anywhere)\n\
         \x20   let s = \"x\\ny\\n\"\n\
         \x20   let read = fs::read(\"in.txt\", fs::Root::Anywhere)\n\
         \x20   println(f\"{lines(own)} {lines(m)} {lines(s.bytes)} {lines(read)} {own.len()}\")\n\
         }\n",
    );
    assert_eq!(out, "1 3 2 3 2");
}

/// A mapping where a `Bytes` is kept: an argument, a field, an element and a
/// `let`, each at a count - and a mapping kept by what a function returns
/// outlives the binding it was made in.
#[test]
fn a_mapping_is_kept_as_a_bytes() {
    let out = printed(
        "bytes-owner-kept",
        "use std::fs\n\
         struct Holder { body: Bytes }\n\
         fn keep(b: Bytes) -> Holder { Holder { body: b } }\n\
         fn page() -> Holder throws {\n\
         \x20   let m = fs::map(\"in.txt\", fs::Root::Anywhere)\n\
         \x20   return keep(m)\n\
         }\n\
         fn main() throws {\n\
         \x20   let h = page()\n\
         \x20   let m2 = fs::map(\"in.txt\", fs::Root::Anywhere)\n\
         \x20   let mut all: Vec[Bytes] = []\n\
         \x20   all.push(m2)\n\
         \x20   let m3 = fs::map(\"in.txt\", fs::Root::Anywhere)\n\
         \x20   let h2 = Holder { body: m3 }\n\
         \x20   let m4 = fs::map(\"in.txt\", fs::Root::Anywhere)\n\
         \x20   let b: Bytes = m4\n\
         \x20   println(f\"{h.body.len()} {h.body[2]} {all[0].len()} {h2.body.len()} {b.len()}\")\n\
         }\n",
    );
    assert_eq!(out, "6 98 6 6 6");
}
