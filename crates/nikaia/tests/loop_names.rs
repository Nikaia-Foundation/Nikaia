//! **A loop name used again** keeps both loops' indices typed (#685): two
//! `for i in 0..<n` in one function each bind an `i` that is never negative.

mod common;

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

/// The issue's probe, and the same with the loops swapped.
#[test]
fn a_loop_name_used_again_keeps_the_first_index() {
    let first = r#"fn main() {
    let xs: Vec[String] = ["a", "b", "c"]
    let mut s = String()
    for i in 0..<2 {
        s.push_str(xs[i].clone())
    }
    let total = 3
    for i in 0..<total {
        if i % 2 == 0 { s.push_str("!") }
    }
    println(s)
}
"#;
    assert_eq!(run("loop-name-first", first), "ab!!\n");
    let swapped = r#"fn main() {
    let xs: Vec[String] = ["a", "b", "c"]
    let mut s = String()
    let total = 3
    for i in 0..<total {
        if i % 2 == 0 { s.push_str("!") }
    }
    for i in 0..<2 {
        s.push_str(xs[i].clone())
    }
    println(s)
}
"#;
    assert_eq!(run("loop-name-swapped", swapped), "!!ab\n");
}
