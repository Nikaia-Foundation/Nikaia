//! **A program nested deep still compiles** (E9, ADR-294): the emitter's core
//! moves to Nikaia method by method, and a frame of a moved method is larger
//! than the Rust one it replaces (a `world`, a walk, the build's flags, the
//! flow). The types and the expressions below go through the moved paths -
//! `ty_counted` for the type, `emit_core_operand` for the cast, and
//! `emit_core_index_expr` for the index - 500 deep, and must still lower.

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
    assert!(
        out.status.success(),
        "{name} lowers:\n{}",
        String::from_utf8_lossy(&out.stderr)
            .chars()
            .take(600)
            .collect::<String>()
    );
}

const DEPTH: usize = 500;

#[test]
fn a_type_nested_deep_lowers() {
    let ty = format!("{}i64{}", "Vec[".repeat(DEPTH), "]".repeat(DEPTH));
    lowers(
        &format!(
            "fn count(xs: ref {ty}) -> i64 {{\n    return xs.len()\n}}\n\n\
             fn main() {{\n    let n: {ty} = []\n    println(f\"{{count(n)}}\")\n}}\n"
        ),
        "type",
    );
}

#[test]
fn casts_nested_deep_lower() {
    let cast = format!("1{}", " as i64".repeat(DEPTH));
    lowers(
        &format!("fn main() {{\n    let a = {cast}\n    println(f\"{{a}}\")\n}}\n"),
        "cast",
    );
}

#[test]
fn an_index_nested_deep_lowers() {
    let index = format!("0{}", " + 0".repeat(DEPTH));
    lowers(
        &format!(
            "fn main() {{\n    let xs = [1, 2, 3]\n    let b = xs[{index}]\n    println(f\"{{b}}\")\n}}\n"
        ),
        "index",
    );
}

#[test]
fn lists_nested_deep_lower() {
    let list = format!("{}1{}", "[".repeat(DEPTH), "]".repeat(DEPTH));
    lowers(
        &format!("fn main() {{\n    let a = {list}\n    println(f\"{{a.len()}}\")\n}}\n"),
        "list",
    );
}

#[test]
fn a_coalesce_chain_deep_lowers() {
    let chain = format!("{}0", "x ?? ".repeat(DEPTH));
    lowers(
        &format!(
            "fn main() {{\n    let x: i64? = null\n    let a = {chain}\n    println(f\"{{a}}\")\n}}\n"
        ),
        "coalesce",
    );
}

#[test]
fn a_method_chain_deep_lowers() {
    let chain = format!("s{}", ".clone()".repeat(DEPTH));
    lowers(
        &format!(
            "fn main() {{\n    let s: String = \"a\"\n    let t = {chain}\n    println(t)\n}}\n"
        ),
        "chain",
    );
}
