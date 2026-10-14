//! A trait method's `mut` parameter lowers to `&mut T` in the declaration as
//! it does in the impl ([ADR-094](../../../docs/specification/adr/adr-094.md)
//! D3), so the two agree and a call changes the caller's value.
mod common;

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

const SOURCE: &str = "\
struct Walk {
    log: Vec[String],
}

trait Host {
    fn visit(ref self, mut walk: Walk, depth: i64) -> i64 sync
}

struct Rusty {
    name: String,
}

impl Host for Rusty {
    fn visit(ref self, mut walk: Walk, depth: i64) -> i64 sync {
        walk.log.push(f\"{self.name} {depth}\")
        return depth + 1
    }
}

fn twice[H: Host](host: ref H, mut walk: Walk) -> i64 sync {
    let a = host.visit(walk, 1)
    return host.visit(walk, a)
}

fn main() {
    let host = Rusty { name: \"h\" }
    let mut walk = Walk { log: [] }
    let n = twice(host, walk)
    print(f\"{n} {walk.log.len()} {walk.log[1]}\")
}
";

#[test]
fn a_mut_parameter_of_a_trait_method_is_a_mutable_borrow_in_both() {
    let parsed = parse_to_ast(SOURCE).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    let found =
        nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
            .findings;
    assert!(found.is_empty(), "{found:#?}");
    let rust = emit_program(&parsed, Build::default())
        .expect("the source lowers")
        .rust;
    assert!(rust.contains("walk: &mut Walk"), "{rust}");
    let dir = common::scratch_dir("trait_mut_parameter");
    let file = dir.join("main.rs");
    std::fs::write(&file, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let out = common::compile(&file, &["-o", binary.to_str().expect("utf-8 path")]);
    assert!(
        out.status.success(),
        "the lowering compiles:\n{}\n--- the Rust ---\n{rust}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ran = std::process::Command::new(&binary)
        .output()
        .expect("run the program");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(ran.status.success());
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "3 2 h 2");
}

const A_LIST: &str = "\
trait Asks {
    fn count(ref self, xs: ref Vec[i64]) -> i64 sync
}

struct One {
    add: i64,
}

impl Asks for One {
    fn count(ref self, xs: ref Vec[i64]) -> i64 sync {
        return xs.len() as i64 + self.add
    }
}

fn through[H: Asks](host: ref H, xs: ref Vec[i64]) -> i64 sync {
    return host.count(xs)
}

fn main() {
    let xs: Vec[i64] = [1, 2]
    let one = One { add: 1 }
    print(f\"{through(one, xs)}\")
}
";

/// A trait method's `ref Vec[T]` is a slice in the declaration, as it is in the
/// impl and in a function that hands its own `ref Vec[T]` on: the declaration
/// kept `&Vec<T>`, and `rustc` refused both.
#[test]
fn a_list_parameter_of_a_trait_method_is_a_slice_in_both() {
    let parsed = parse_to_ast(A_LIST).expect("the source parses");
    let rust = emit_program(&parsed, Build::default())
        .expect("the source lowers")
        .rust;
    assert!(
        rust.contains("fn count(&self, xs: &[i64]) -> i64;"),
        "{rust}"
    );
    let dir = common::scratch_dir("trait_list_parameter");
    let file = dir.join("main.rs");
    std::fs::write(&file, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let out = common::compile(&file, &["-o", binary.to_str().expect("utf-8 path")]);
    assert!(
        out.status.success(),
        "the lowering compiles:\n{}\n--- the Rust ---\n{rust}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ran = std::process::Command::new(&binary)
        .output()
        .expect("run the program");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(ran.status.success());
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "3");
}
