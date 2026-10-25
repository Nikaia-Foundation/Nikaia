//! **A `Bytes` carries what owns its memory; a `ref Bytes` only read takes any
//! run of bytes** ([ADR-373](../../../docs/specification/adr/adr-373.md), #718).
//!
//! `Bytes(v)` hands a `Vec[u8]` over (D2); a mapping is a `Bytes` where one is
//! taken by value - an argument, a field, an element, a `let` - at a count
//! (D3); and one function that only reads takes a `Bytes`, a mapping and a
//! text's bytes alike (D4).

use crate::common;
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

/// The keeping functions of the probes below: one stores a clone, one returns
/// one, one hands its parameter on to the first; and one returns a view.
const KEEPERS: &str = "use std::fs\n\
     struct Holder { body: Bytes }\n\
     fn store(b: ref Bytes) -> Holder { Holder { body: b.clone() } }\n\
     fn twin(b: ref Bytes) -> Bytes { b.clone() }\n\
     fn pass(b: ref Bytes) -> Holder { store(b) }\n\
     fn first(b: ref Bytes) -> ref Bytes { b[0..<1] }\n\
     fn lines(b: ref Bytes) -> i64 { let mut n = 0; for i in 0..<b.len() { if b[i] == 10 { n += 1 } }; n }\n";

/// The `main` that calls each keeper with a `Bytes`, a mapping and a text's
/// bytes, and the view with a mapping.
const KEEPERS_MAIN: &str = "fn main() throws {\n\
     \x20   let mut v: Vec[u8] = []\n\
     \x20   v.push(65)\n\
     \x20   v.push(10)\n\
     \x20   let own = Bytes(v)\n\
     \x20   let m = fs::map(\"in.txt\", fs::Root::Anywhere)\n\
     \x20   let s = \"x\\ny\\n\"\n\
     \x20   let h1 = store(own)\n\
     \x20   let h2 = store(m)\n\
     \x20   let h3 = store(s.bytes)\n\
     \x20   let t1 = twin(own)\n\
     \x20   let t2 = twin(m)\n\
     \x20   let t3 = twin(s.bytes)\n\
     \x20   let p1 = pass(own)\n\
     \x20   let p2 = pass(m)\n\
     \x20   let p3 = pass(s.bytes)\n\
     \x20   println(f\"{h1.body.len()} {h2.body.len()} {h3.body.len()} {t1.len()} {t2.len()} {t3.len()} {p1.body.len()} {p2.body.len()} {p3.body.len()}\")\n\
     \x20   println(f\"{first(m)[0]} {first(own)[0]} {first(s.bytes)[0]} {lines(m)} {lines(own)} {lines(s.bytes)} {h2.body[2]}\")\n\
     }\n";

/// **A `ref Bytes` the body keeps takes the `Bytes` with its owner** (D1, D4):
/// a clone stored, a clone returned and the parameter handed on to either,
/// each called with a `Bytes`, a mapping and a text's bytes; a function that
/// returns a view of its `ref Bytes` still takes a mapping.
#[test]
fn a_kept_ref_bytes_takes_every_owner() {
    let out = printed("bytes-owner-keepers", &format!("{KEEPERS}{KEEPERS_MAIN}"));
    assert_eq!(out, "2 6 4 2 6 4 2 6 4\n97 65 120 3 1 2 98");
}

/// What the lowering writes for it: a kept `ref Bytes` is a `&Bytes`, so its
/// clone is a count and a `Bytes` or a mapping reaches it without a copy (the
/// pointer test is `nikaia-std`'s `kept_bytes.rs`); one only read or viewed
/// stays a run of bytes.
#[test]
fn a_kept_ref_bytes_is_lowered_with_its_owner() {
    let dir = common::scratch_dir("bytes-owner-keepers-lowered");
    std::fs::write(dir.join("main.nika"), format!("{KEEPERS}{KEEPERS_MAIN}")).expect("the source");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "main.nika"])
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let rust = std::fs::read_to_string(dir.join("main.rs")).expect("the lowering");
    let _ = std::fs::remove_dir_all(&dir);
    for kept in [
        "fn store(b: &Bytes)",
        "fn twin(b: &Bytes)",
        "fn pass(b: &Bytes)",
    ] {
        assert!(rust.contains(kept), "{kept} missing:\n{rust}");
    }
    for read in ["fn first(b: &[u8])", "fn lines(b: &[u8])"] {
        assert!(rust.contains(read), "{read} missing:\n{rust}");
    }
    assert!(
        rust.contains("store(&*nikaia_std::bytes::kept(&own))")
            && rust.contains("store(&*nikaia_std::bytes::kept(&m))")
            && rust.contains("store(&*nikaia_std::bytes::kept(b))"),
        "{rust}"
    );
}

/// **A method that keeps a `ref Bytes` takes it with its owner too** (#756):
/// `put` stores a clone in `self`; it is a `&Bytes`, and each call wraps the
/// argument in `kept`, for a `Bytes`, a mapping and a text's bytes.
const METHOD_KEEPER: &str = "use std::fs\n\
     struct Holder { body: Bytes }\n\
     impl Holder {\n\
     \x20   fn put(ref mut self, b: ref Bytes) { self.body = b.clone() }\n\
     }\n\
     fn main() throws {\n\
     \x20   let mut v: Vec[u8] = []\n\
     \x20   v.push(65)\n\
     \x20   v.push(10)\n\
     \x20   let own = Bytes(v)\n\
     \x20   let m = fs::map(\"in.txt\", fs::Root::Anywhere)\n\
     \x20   let s = \"x\\ny\\n\"\n\
     \x20   let mut h = Holder { body: own.clone() }\n\
     \x20   h.put(own)\n\
     \x20   let a = h.body.len()\n\
     \x20   h.put(m)\n\
     \x20   let b = h.body.len()\n\
     \x20   h.put(s.bytes)\n\
     \x20   println(f\"{a} {b} {h.body.len()}\")\n\
     }\n";

#[test]
fn a_method_keeping_a_ref_bytes_takes_every_owner() {
    let out = printed("bytes-owner-method", METHOD_KEEPER);
    assert_eq!(out, "2 6 4");
    let dir = common::scratch_dir("bytes-owner-method-lowered");
    std::fs::write(dir.join("main.nika"), METHOD_KEEPER).expect("the source");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "main.nika"])
        .output()
        .expect("the nikaia binary runs");
    assert!(lowered.status.success());
    let rust = std::fs::read_to_string(dir.join("main.rs")).expect("the lowering");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(rust.contains("b: &Bytes"), "{rust}");
    assert!(
        rust.contains(".put(&*nikaia_std::bytes::kept(&own))"),
        "{rust}"
    );
    assert!(
        rust.contains(".put(&*nikaia_std::bytes::kept(&m))"),
        "{rust}"
    );
}

/// An owned `Bytes` compares with a cut, in either order, and with another
/// owned `Bytes`, by content (#764).
#[test]
fn an_owned_bytes_compares_with_a_cut() {
    let out = printed(
        "bytes-owner-eq-cut",
        "fn main() {\n\
         \x20   let mut v: Vec[u8] = []\n\
         \x20   v.push('a')\n\
         \x20   let a = Bytes(v)\n\
         \x20   let mut w: Vec[u8] = []\n\
         \x20   w.push('a')\n\
         \x20   w.push('b')\n\
         \x20   let b = Bytes(w)\n\
         \x20   let mut x: Vec[u8] = []\n\
         \x20   x.push('a')\n\
         \x20   let c = Bytes(x)\n\
         \x20   println(f\"{a == b[0..<1]} {b[0..<1] == a} {a == c} {a == b} {a == b[1..<2]}\")\n\
         }\n",
    );
    assert_eq!(out, "true true true false false");
}
