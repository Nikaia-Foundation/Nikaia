//! **A list changed in place, and a number held between bounds**: `retain`,
//! `swap`, `reverse` and `dedup` on `Vec`, and `clamp` on the integers. Each
//! is run with `nikaia run` and its output compared, `swap` with `i64`
//! positions (ADR-293 D20) and each with its empty case where it has one.
mod common;

fn run(purpose: &str, body: &str) -> String {
    let dir = common::scratch_dir(purpose);
    let source = format!(
        "use std::cli\n\nfn show(xs: ref Vec[i64]) -> String {{\n    let mut out: String = \"[\"\n    for x in xs {{\n        out.push_str(f\" {{x}}\")\n    }}\n    out.push_str(\" ]\")\n    return out\n}}\n\n\
         fn main() {{\n    let mut xs: Vec[i64] = [3, 8, 1, 8, 8, 9, 4, 4]\n    let mut none: Vec[i64] = []\n{body}\n}}\n"
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
fn retain_keeps_what_the_function_says_yes_to() {
    let out = run(
        "vec-retain",
        "    let mut seen = 0\n    xs.retain fn(x) {\n        seen = seen + 1\n        x % 2 == 0\n    }\n    none.retain(fn(x) { x > 0 })\n    let mut words: Vec[String] = [\"ab\".clone(), \"c\".clone(), \"de\".clone()]\n    words.retain(fn(w) { w.len() > 1 })\n    println(f\"{show(xs)} {seen} {show(none)} {words.len()} {words[1]}\")",
    );
    assert_eq!(out, "[ 8 8 8 4 4 ] 8 [ ] 2 de\n");
}

#[test]
fn swap_takes_i64_positions() {
    let out = run(
        "vec-swap",
        "    let i: i64 = 0\n    let j: i64 = xs.len() - 1\n    xs.swap(i, j)\n    xs.swap(2, 2)\n    println(show(xs))",
    );
    assert_eq!(out, "[ 4 8 1 8 8 9 4 3 ]\n");
}

#[test]
fn reverse_turns_the_list_around() {
    let out = run(
        "vec-reverse",
        "    xs.reverse()\n    none.reverse()\n    println(f\"{show(xs)} {show(none)}\")",
    );
    assert_eq!(out, "[ 4 4 9 8 8 1 8 3 ] [ ]\n");
}

#[test]
fn dedup_drops_equal_neighbours_only() {
    let out = run(
        "vec-dedup",
        "    xs.dedup()\n    none.dedup()\n    let mut words: Vec[String] = [\"a\".clone(), \"a\".clone(), \"b\".clone()]\n    words.dedup()\n    println(f\"{show(xs)} {show(none)} {words.len()}\")",
    );
    assert_eq!(out, "[ 3 8 1 8 9 4 ] [ ] 2\n");
}

#[test]
fn clamp_holds_a_number_between_bounds() {
    let out = run(
        "int-clamp",
        "    let n: i64 = 15\n    let small: i32 = 3\n    let big: u64 = 70\n    let lo: u32 = 7\n    println(f\"{n.clamp(0, 10)} {(-4).clamp(0, 10)} {n.clamp(0, 20)} {small.clamp(5, 9)} {big.clamp(1, 64)} {lo.clamp(1, 9)}\")",
    );
    assert_eq!(out, "10 0 15 5 64 7\n");
}
