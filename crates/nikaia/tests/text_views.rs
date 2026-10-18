//! **A `String` that only views reach is a view at every position**
//! ([ADR-282](../../../docs/specification/adr/adr-282.md) D2, #654 step 0):
//! each program here, written with `String`, lowers to the same Rust as the
//! same program written with `ref String`.

mod common;

/// Lowers `source` as `name.nika` and hands back the Rust, without the line
/// table, which names the file.
fn lowered(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    let file = dir.join(format!("{name}.nika"));
    std::fs::write(&file, source).expect("the source");
    let ran = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("lower")
        .arg(format!("{name}.nika"))
        .output()
        .expect("the nikaia binary runs");
    let rust = std::fs::read_to_string(dir.join(format!("{name}.rs"))).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        ran.status.success(),
        "{name}:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    rust.lines()
        .filter(|line| !line.contains(".nika\""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Runs `source` and hands back what it printed.
fn ran(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let ran = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    String::from_utf8_lossy(&ran.stdout).into_owned()
}

/// The program with `ref String` in the places listed, and without.
fn same_rust(name: &str, written: &str, plain: &str) {
    let with_ref = lowered(&format!("{name}_ref"), written);
    let without = lowered(&format!("{name}_plain"), plain);
    assert_eq!(with_ref, without, "{name}: the Rust differs");
}

/// A `String` parameter used as a key into a map keyed by views is lent: a
/// `get` of a `std` map is the one entry by that name with a key, so the
/// `self`-only `get`s no longer read as keeping it (was an owned `String`, and
/// `rustc` refused the `get`: `&str: Borrow<String>`).
#[test]
fn a_string_parameter_used_as_a_map_key_is_lent() {
    let program = r#"use std::collections

fn count(dna: ref String, k: i64) -> collections::HashMap[ref String, i64] {
    let mut counts: collections::HashMap[ref String, i64] = collections::HashMap()
    for i in 0..<dna.len() - k + 1 {
        let fragment = ref dna[i..<i + k]
        counts.insert(fragment, 1)
    }
    return counts
}

fn report(dna: ref String, fragment: TEXT) {
    let counts = count(dna, fragment.len())
    let n = counts.get(fragment) ?? 0
    println(f"{n}\t{fragment}")
}

fn main() {
    let d = "GGTAGGT".clone()
    report(d, "GGT")
}
"#;
    let plain = program.replace("TEXT", "String");
    let rust = lowered("map_key_parameter", &plain);
    assert!(rust.contains("fragment: &str"), "{rust}");
    assert_eq!(ran("map_key_parameter_run", &plain), "1\tGGT\n");
}
