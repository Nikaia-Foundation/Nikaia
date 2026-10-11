//! A method that takes a function and calls itself with it
//! (#558 prerequisite): the function value is lent on, so a loop can hand it
//! to the recursive call more than once.

mod common;

use std::process::Command;

use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

const PROGRAM: &str = "\
struct Tree { value: i64, kids: Vec[Tree] }

impl Tree {
    fn total(ref self, visit: fn(i64) -> i64 sync) -> i64 sync {
        let mut sum = visit(self.value)
        for kid in self.kids {
            sum = sum + kid.total(visit)
        }
        return sum
    }
}

fn main() {
    let tree = Tree { value: 1, kids: [Tree { value: 2, kids: [] }, Tree { value: 3, kids: [] }] }
    println(f\"{tree.total(fn(n) { n * 10 })}\")
}
";

#[test]
fn a_method_hands_its_function_to_itself_in_a_loop() {
    let parsed = parse_to_ast(PROGRAM).expect("the source parses");
    let rust = emit_program(&parsed, Build::default())
        .expect("it lowers")
        .rust;
    let dir = common::scratch_dir("method-code-recursion");
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary).output().expect("run it");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "60\n");
    std::fs::remove_dir_all(&dir).ok();
}
