//! **What two ordered sets share, and their ends**: `union`, `intersection`,
//! `difference`, `first` and `last` on `collections::BTreeSet`. Each is run
//! with `nikaia run`, with an empty set beside it, and its output compared.

use crate::common;

fn run(purpose: &str, body: &str) -> String {
    let dir = common::scratch_dir(purpose);
    let source = format!(
        "use std::cli\nuse std::collections\n\nfn show(xs: ref Vec[i64]) -> String {{\n    let mut out: String = \"[\"\n    for x in xs {{\n        out.push_str(f\" {{x}}\")\n    }}\n    out.push_str(\" ]\")\n    return out\n}}\n\n\
         fn main() {{\n    let mut a: collections::BTreeSet[i64] = collections::BTreeSet()\n    let mut b: collections::BTreeSet[i64] = collections::BTreeSet()\n    let none: collections::BTreeSet[i64] = collections::BTreeSet()\n    for x in [5, 1, 3] {{\n        a.insert(x)\n    }}\n    for x in [3, 4, 5] {{\n        b.insert(x)\n    }}\n{body}\n}}\n"
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
    String::from_utf8_lossy(&run.stdout).into_owned()
}

#[test]
fn union_intersection_and_difference_walk_in_order() {
    let out = run(
        "set-ops",
        "    let both: Vec[i64] = a.union(b).collect()\n    let shared: Vec[i64] = a.intersection(b).collect()\n    let only: Vec[i64] = a.difference(b).collect()\n    let other: Vec[i64] = b.difference(a).collect()\n    let nothing: Vec[i64] = a.intersection(none).collect()\n    let all: Vec[i64] = none.union(a).collect()\n    println(f\"{show(both)} {show(shared)} {show(only)} {show(other)} {show(nothing)} {show(all)}\")",
    );
    assert_eq!(out, "[ 1 3 4 5 ] [ 3 5 ] [ 1 ] [ 4 ] [ ] [ 1 3 5 ]\n");
}

#[test]
fn first_and_last_are_null_when_empty() {
    let out = run(
        "set-ends",
        "    let mut names: collections::BTreeSet[String] = collections::BTreeSet()\n    names.insert(\"b\")\n    names.insert(\"a\")\n    println(f\"{a.first() ?? -1} {a.last() ?? -1} {none.first() ?? -1} {none.last() ?? -1} {names.first() ?? \"none\"} {names.last() ?? \"none\"}\")",
    );
    assert_eq!(out, "1 5 -1 -1 a b\n");
}
