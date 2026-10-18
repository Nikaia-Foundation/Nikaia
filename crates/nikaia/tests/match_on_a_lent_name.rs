//! **A `match` on a name an enclosing arm lent** (lowering-defects D48), and
//! **an indexed write on an `Array`** (D46): programs `rustc` refused, run and
//! their output compared.
mod common;

use std::process::Command;

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

/// Checks a program has no error, compiles it with `rustc` and runs it,
/// comparing what it prints.
fn runs(purpose: &str, source: &str, expected: &str) {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    let errors: Vec<_> = nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{purpose}: {errors:#?}");
    let rust = emit_program(&parsed, Build::default())
        .expect("it lowers")
        .rust;
    let dir = common::scratch_dir(&format!("lent-name-{purpose}"));
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary).output().expect("run it");
    assert!(out.status.success(), "{purpose} failed");
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected, "{purpose}");
    std::fs::remove_dir_all(&dir).ok();
}

/// D48: the outer arm binds `ref inner` and opens the box into a `&Shape`;
/// the inner `match` is over what it points to, so its own `ref parts` holds.
#[test]
fn a_match_on_a_name_taken_out_of_a_box_runs() {
    runs(
        "boxed",
        "enum Shape {\n    Leaf(i64),\n    Many(Vec[Shape]),\n    Maybe(Shape),\n}\n\
         fn count(parts: ref Vec[Shape]) -> i64 {\n    return parts.len() as i64\n}\n\
         fn size(s: ref Shape) -> i64 {\n\
         \x20   let on = s.clone()\n\
         \x20   match on {\n\
         \x20       Shape::Maybe(inner) => {\n\
         \x20           match inner {\n\
         \x20               Shape::Many(parts) => {\n\
         \x20                   return count(parts)\n\
         \x20               },\n\
         \x20               else => {}\n\
         \x20           }\n\
         \x20           return 0\n\
         \x20       },\n\
         \x20       else => {}\n\
         \x20   }\n\
         \x20   return -1\n\
         }\n\
         fn main() {\n\
         \x20   println(f\"{size(Shape::Maybe(Shape::Many([Shape::Leaf(1), Shape::Leaf(2)])))}\")\n\
         \x20   println(f\"{size(Shape::Maybe(Shape::Leaf(1)))}\")\n\
         \x20   println(f\"{size(Shape::Leaf(1))}\")\n\
         }\n",
        "2\n0\n-1\n",
    );
}

/// D48 without a box: a lent part of a plain enum, matched again, with a
/// number copied out of it and text lent.
#[test]
fn a_match_on_a_lent_part_runs() {
    runs(
        "plain",
        "enum Inner {\n    Leaf(i64),\n    Name(String),\n}\n\
         enum Outer {\n    Wrap(Inner),\n    Other,\n}\n\
         fn take(n: i64) -> i64 {\n    return n * 2\n}\n\
         fn size(s: ref Outer) -> i64 {\n\
         \x20   let on = s.clone()\n\
         \x20   match on {\n\
         \x20       Outer::Wrap(inner) => {\n\
         \x20           match inner {\n\
         \x20               Inner::Leaf(n) => {\n\
         \x20                   return take(n)\n\
         \x20               },\n\
         \x20               Inner::Name(text) => {\n\
         \x20                   return text.len() as i64\n\
         \x20               },\n\
         \x20           }\n\
         \x20       },\n\
         \x20       else => {}\n\
         \x20   }\n\
         \x20   return -1\n\
         }\n\
         fn main() {\n\
         \x20   println(f\"{size(Outer::Wrap(Inner::Leaf(4)))}\")\n\
         \x20   println(f\"{size(Outer::Wrap(Inner::Name(\"abc\")))}\")\n\
         \x20   println(f\"{size(Outer::Other)}\")\n\
         }\n",
        "8\n3\n-1\n",
    );
}

/// D46: `xs[i] = v` on an `Array[T, N]` is the indexed write a `Vec` has
/// (`nikaia_std::index::Set` for `[V; N]`), in a loop and at a constant.
#[test]
fn an_indexed_write_on_an_array_runs() {
    runs(
        "array-write",
        "fn squares() -> Array[i64, 5] {\n\
         \x20   let mut xs: Array[i64, 5] = [0, 0, 0, 0, 0]\n\
         \x20   for i in 0..<xs.len() {\n\
         \x20       xs[i] = i * i\n\
         \x20   }\n\
         \x20   return xs\n\
         }\n\
         fn main() {\n\
         \x20   let mut ys: Array[i64, 3] = [1, 2, 3]\n\
         \x20   ys[1] = 5\n\
         \x20   let xs = squares()\n\
         \x20   println(f\"{ys[1]} {xs[4]}\")\n\
         }\n",
        "5 16\n",
    );
}
