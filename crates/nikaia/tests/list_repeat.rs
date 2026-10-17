//! `[v; n]`, `n` copies of a value that copies
//! ([ADR-362](../../../docs/specification/adr/adr-362.md) D1-D3): parsed as a
//! list literal of its own kind, typed by its use as every list literal is
//! (`Vec[T]` with none, `Array[T, N]` where one is wanted and `n` is `N`), and
//! refused for a value that does not copy (`NK1280`).

mod common;

use std::process::Command;

use nikaia::ast::{Expr, Item, Stmt};
use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std's ledger");
    nikaia::check::check(&parsed, &own, &library)
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

fn lowered(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Build::default())
        .expect("it lowers")
        .rust
}

/// The value of the first `let` in `main`.
fn first_let(source: &str) -> Expr {
    let parsed = parse_to_ast(source).expect("the source parses");
    let Item::Fn { body, .. } = &parsed.program.items[0].node else {
        panic!("expected a function");
    };
    let Stmt::Let { value, .. } = &body.stmts[0].node else {
        panic!("expected a `let`");
    };
    value.clone()
}

fn in_main(body: &str) -> String {
    format!("fn main() {{\n{body}\n}}\n")
}

/// **D1: `[v; n]` is a list literal of its own kind**, its value and its count
/// each a whole expression, and nested as any list is.
#[test]
fn the_literal_parses_as_its_own_kind() {
    match first_let(&in_main("    let a = [0; 3]")) {
        Expr::ListRepeat { value, count, .. } => {
            assert!(matches!(*value, Expr::LitInt { value: 0, .. }));
            assert!(matches!(*count, Expr::LitInt { value: 3, .. }));
        }
        other => panic!("{other:?}"),
    }
    let source = in_main("    let a = [x + 1; n]");
    match first_let(&source) {
        Expr::ListRepeat { value, count, .. } => {
            assert!(matches!(*value, Expr::Binary { .. }), "{value:?}");
            assert!(matches!(*count, Expr::Variable(..)), "{count:?}");
        }
        other => panic!("{other:?}"),
    }
    match first_let(&in_main("    let a = [[0; 2]; 3]")) {
        Expr::ListRepeat { value, .. } => {
            assert!(matches!(*value, Expr::ListRepeat { .. }), "{value:?}")
        }
        other => panic!("{other:?}"),
    }
    // A plain list keeps parsing as it did.
    assert!(matches!(
        first_let(&in_main("    let a = [1, 2, 3]")),
        Expr::ListLit { .. }
    ));
}

/// **No other `;` inside brackets**: `[1, 2; 3]` is a syntax error.
#[test]
fn a_count_after_several_elements_is_a_syntax_error() {
    assert!(parse_to_ast(&in_main("    let a = [1, 2; 3]")).is_err());
}

/// **D2: where an `Array[T, N]` is wanted, `n` is `N`.**
#[test]
fn an_array_of_n_equal_values_checks() {
    let source = in_main("    let b: Array[u8, 4096] = [0; 4096]\n    println(f\"{b[0]}\")");
    assert!(findings(&source).is_empty(), "{:#?}", findings(&source));
    let rust = lowered(&source);
    assert!(rust.contains("let b: [u8; 4096] = [0; _];"), "{rust}");
}

/// **Another number is `NK1157`, naming both.**
#[test]
fn a_count_that_is_not_the_arrays_length_is_refused() {
    let source = in_main("    let b: Array[u8, 4096] = [0; 4095]\n    println(f\"{b[0]}\")");
    let found: Vec<_> = findings(&source)
        .into_iter()
        .filter(|f| f.code == "NK1157")
        .collect();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].message.contains("4,095") || found[0].message.contains("4095"),
        "{found:#?}"
    );
    assert!(found[0].message.contains("4096"), "{found:#?}");
}

/// **And so is a count the build does not know**, named as written.
#[test]
fn a_count_the_build_does_not_know_is_refused_for_an_array() {
    let source = "fn fill(n: i64) {\n\
                  \x20   let b: Array[u8, 4] = [0; n]\n\
                  \x20   println(f\"{b[0]}\")\n\
                  }\n\
                  fn main() {\n\
                  \x20   fill(4)\n\
                  }\n";
    let found: Vec<_> = findings(source)
        .into_iter()
        .filter(|f| f.code == "NK1157")
        .collect();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].message.contains("`n`"), "{found:#?}");
    assert!(found[0].message.contains('4'), "{found:#?}");
}

/// **With no use it is a `Vec`**, and its count is any whole number.
#[test]
fn with_no_use_it_is_a_vec() {
    let source = "fn fill(n: i64) {\n\
                  \x20   let v = [0; n]\n\
                  \x20   let s: String = v\n\
                  \x20   println(s)\n\
                  }\n\
                  fn main() {\n\
                  \x20   fill(4)\n\
                  }\n";
    // A number with no use is open (ADR-285 D34), as in `[0, 1]`; the
    // container is a `Vec` either way.
    let found = findings(source);
    assert!(
        found.iter().any(|f| f.message.contains("`Vec[")),
        "{found:#?}"
    );
    let kept = "fn fill(n: i64) -> i64 {\n\
                \x20   let v = [0; n]\n\
                \x20   let first: i64 = v[0]\n\
                \x20   return first + v.len()\n\
                }\n\
                fn main() {\n\
                \x20   println(f\"{fill(4)}\")\n\
                }\n";
    assert!(findings(kept).is_empty(), "{:#?}", findings(kept));
    assert!(
        lowered(kept).contains("let v: Vec<i64> = vec![0; nikaia_std::count::of(n)];"),
        "{}",
        lowered(kept)
    );
    let fine = "fn fill(n: u32) -> i64 {\n\
                \x20   let v = [0; n]\n\
                \x20   return v.len()\n\
                }\n\
                fn main() {\n\
                \x20   println(f\"{fill(4)}\")\n\
                }\n";
    assert!(findings(fine).is_empty(), "{:#?}", findings(fine));
}

/// **D3: a value that does not copy is `NK1280`**, and the help writes each
/// copy with the program's own names.
#[test]
fn a_value_that_does_not_copy_is_refused() {
    let source = "fn names(name: String) -> Vec[String] {\n\
                  \x20   return [name; 3]\n\
                  }\n\
                  fn main() {\n\
                  \x20   println(f\"{names(f\"a\").len()}\")\n\
                  }\n";
    let found: Vec<_> = findings(source)
        .into_iter()
        .filter(|f| f.code == "NK1280")
        .collect();
    assert_eq!(found.len(), 1, "{:#?}", findings(source));
    let help = found[0].help.clone().unwrap_or_default();
    assert!(
        help.contains("(0..<3).map(fn(_i) { name.clone() }).collect()"),
        "{found:#?}"
    );
}

/// **A list does not copy**, so a nested `[[0; 2]; 3]` with no use is
/// refused, and one an array of arrays is wanted for checks.
#[test]
fn a_nested_literal_copies_only_as_an_array() {
    let vecs = in_main("    let g = [[0; 2]; 3]\n    println(f\"{g.len()}\")");
    assert!(
        findings(&vecs).iter().any(|f| f.code == "NK1280"),
        "{:#?}",
        findings(&vecs)
    );
    let arrays =
        in_main("    let g: Array[Array[i64, 2], 3] = [[0; 2]; 3]\n    println(f\"{g[2][1]}\")");
    assert!(findings(&arrays).is_empty(), "{:#?}", findings(&arrays));
}

/// **A tuple of numbers and a value struct of numbers copy.**
#[test]
fn values_that_copy_check() {
    let tuple = in_main("    let ps = [(1, 2); 3]\n    println(f\"{ps.len()}\")");
    assert!(findings(&tuple).is_empty(), "{:#?}", findings(&tuple));
    let point = "struct Point {\n\
                 \x20   x: i64,\n\
                 \x20   y: f64,\n\
                 }\n\
                 fn main() {\n\
                 \x20   let ps = [Point { x: 1, y: 2.0 }; 3]\n\
                 \x20   println(f\"{ps[2].x}\")\n\
                 }\n";
    assert!(findings(point).is_empty(), "{:#?}", findings(point));
}

/// **The lowering**: `vec![v; n]` for a list, through `count::of` where the
/// count is not written in literals, and a program printing its length and an
/// element.
#[test]
fn a_program_with_both_runs() {
    let source = "fn fill(n: i64) -> Vec[i64] {\n\
                  \x20   return [7; n]\n\
                  }\n\
                  fn main() {\n\
                  \x20   let zeros = [0; 181]\n\
                  \x20   let sevens = fill(3)\n\
                  \x20   let buf: Array[u8, 16] = [9; 16]\n\
                  \x20   let grid: Array[Array[i64, 2], 3] = [[1; 2]; 3]\n\
                  \x20   println(f\"{zeros.len()} {zeros[180]} {sevens.len()} {sevens[2]} {buf.len()} {buf[15]} {grid.len()} {grid[2][1]}\")\n\
                  }\n";
    assert!(findings(source).is_empty(), "{:#?}", findings(source));
    let rust = lowered(source);
    assert!(rust.contains("vec![7; nikaia_std::count::of(n)]"), "{rust}");
    let dir = common::scratch_dir("list-repeat-runs");
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &[
            "--crate-type",
            "bin",
            "-o",
            binary.to_str().expect("utf-8 path"),
        ],
    );
    assert!(
        compiled.status.success(),
        "it did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary).output().expect("run it");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "181 0 3 7 16 9 3 1"
    );
    std::fs::remove_dir_all(&dir).ok();
}
