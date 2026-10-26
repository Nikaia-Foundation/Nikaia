//! **`entry` on a map** lowers to what rustc takes: `and_modify` on a map of
//! numbers changes the number in the slot (#684), and a literal or a view of
//! text handed to `entry` on a map keyed by `String` is built into the key
//! (#676), as `insert`'s is.

use crate::common;

/// The standard output of `nikaia run`, which must succeed.
fn run(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    let out = String::from_utf8_lossy(&run.stdout).into_owned();
    let err = String::from_utf8_lossy(&run.stderr).into_owned();
    assert!(run.status.success(), "{out}{err}");
    out
}

/// `fn(v) { v += 1 }`, `v = v * 2`, a read, a `mut` parameter, a float and a
/// `bool`: each is the value in the slot, changed there (#684).
#[test]
fn and_modify_on_a_map_of_numbers_changes_the_number() {
    let source = r#"use std::collections

fn main() {
    let mut m: collections::HashMap[String, i64] = collections::HashMap()
    let k = f"a"
    m.entry(k.clone()).and_modify(fn(v) { v += 1 }).or_insert(1)
    m.entry(k.clone()).and_modify(fn(v) { v += 1 }).or_insert(1)
    m.entry(k.clone()).and_modify(fn(v) { v = v * 2 }).or_insert(1)
    m.entry(k.clone()).and_modify(fn(mut v) { if v > 3 { v = v + 10 } }).or_insert(1)
    let mut f: collections::HashMap[String, f64] = collections::HashMap()
    f.entry(k.clone()).and_modify(fn(v) { v -= 0.5 }).or_insert(1.0)
    f.entry(k.clone()).and_modify(fn(v) { v -= 0.5 }).or_insert(1.0)
    let mut b: collections::HashMap[String, bool] = collections::HashMap()
    b.entry(k.clone()).and_modify(fn(v) { v = !v }).or_insert(true)
    b.entry(k.clone()).and_modify(fn(v) { v = !v }).or_insert(true)
    println(f"{m[k] ?? 0} {f[k] ?? 0.0} {b[k] ?? true}")
}
"#;
    assert_eq!(run("entry-number", source), "14 0.5 false\n");
}

/// The issue's own probe: a literal key for `entry` as for `insert` (#676).
#[test]
fn a_literal_key_for_entry_is_built_into_text() {
    let source = r#"use std::collections

fn main() {
    let mut m: collections::HashMap[String, i64] = collections::HashMap()
    m.insert("a", 1)
    m.entry("d").and_modify(fn(v) { v += 1 }).or_insert(1)
    m.entry("d").and_modify(fn(v) { v += 1 }).or_insert(1)
    println(f"{m.len()} {m["d"] ?? 0}")
}
"#;
    assert_eq!(run("entry-literal", source), "2 2\n");
}

/// A `ref String` parameter and the `.text()` of a `Bytes` view are views,
/// and `entry` builds the key from them (#676).
#[test]
fn a_view_key_for_entry_is_built_into_text() {
    let source = r#"use std::collections

struct C { n: i64 }

fn count(k: ref String) -> i64 {
    let mut m: collections::HashMap[String, i64] = collections::HashMap()
    m.entry(k).and_modify(fn(v) { v += 1 }).or_insert(1)
    m.entry(k).and_modify(fn(v) { v += 1 }).or_insert(1)
    return m[k] ?? 0
}

fn main() throws {
    let key = f"a"
    let line = "ab;1.0"
    let mut c: collections::HashMap[String, C] = collections::HashMap()
    let name = line.bytes[0..<2].text()
    c.entry(name).and_modify(fn(x) { x.n += 1 }).or_insert(C { n: 1 })
    c.entry(name).and_modify(fn(x) { x.n += 1 }).or_insert(C { n: 1 })
    let mut n = 0
    for (_, x) in c {
        n = n + x.n
    }
    println(f"{count(key)} {count("b")} {c.len()} {n}")
}
"#;
    assert_eq!(run("entry-view", source), "2 2 1 2\n");
}
