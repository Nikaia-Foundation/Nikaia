//! **A fold over a walk on every core, and the cores it runs on**
//! ([ADR-372](../../../docs/specification/adr/adr-372.md) D1, D2, D5-D7, Part
//! II 12.6): `task::cores()`, `Par::fold(init, step, merge; order:, workers:)`
//! and `par_iter(piece:)`, each meaning the same at both settings of
//! `user_parallelism`.

use crate::common;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Every fold the issue asks for (#717): a sum at both orders, with fewer and
/// more workers, with the smallest and the largest piece; a fold into a list
/// under `Kept`; over no elements; a type as `init`; and last, `workers: 0`.
const FOLDS: &str = r#"use std::par
use std::task

fn plus(a: i64, b: i64) -> i64 {
    return a + b
}

struct Tally {
    n: i64,
}

impl Tally {
    pub fn() -> Tally sync {
        return Tally { n: 0 }
    }
}

fn tally_add(t: Tally, x: i64) -> Tally {
    return Tally { n: t.n + x }
}

fn tally_merge(a: Tally, b: Tally) -> Tally {
    return Tally { n: a.n + b.n }
}

fn main() {
    println(f"{task::cores()}")
    let mut xs: Vec[i64] = []
    for i in 0..<1_000_000 {
        xs.push(i)
    }
    let mut sequential = 0
    for x in xs.iter() {
        sequential += x
    }
    let kept = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b })
    let any = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, plus; order: par::Order::Any)
    let two = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b }; workers: 2, order: par::Order::Kept)
    let many = xs.par_iter().map(fn(x) { x * 2 }).fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b }; workers: 64, order: par::Order::Any)
    let small = xs.par_iter(piece: 1).fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b })
    let large = xs.par_iter(piece: 1_000_000).fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b })
    println(f"{sequential} {kept} {any} {two} {many} {small} {large}")
    let listed = xs.par_iter().fold(fn {
        let none: Vec[i64] = []
        none
    }, fn(acc, x) {
        let mut l = acc
        l.push(x)
        return l
    }, fn(a, b) {
        let mut l = a
        for x in b {
            l.push(x)
        }
        return l
    })
    let mut out_of_order = 0
    for i in 0..<listed.len() {
        if listed[i] != xs[i] {
            out_of_order += 1
        }
    }
    println(f"{listed.len()} {out_of_order}")
    let empty: Vec[i64] = []
    let seven = empty.par_iter().fold(fn { 7 }, fn(acc, x) { acc + x }, fn(a, b) { a + b }; order: par::Order::Any)
    let eight = empty.par_iter().fold(fn { 8 }, fn(acc, x) { acc + x }, fn(a, b) { a + b })
    println(f"{seven} {eight}")
    let t = xs.par_iter().fold(Tally, fn(acc, x) { tally_add(acc, x) }, fn(a, b) { tally_merge(a, b) }; order: par::Order::Any)
    println(f"{t.n}")
    let named = xs.par_iter().fold(Tally, fn(acc, x) { tally_add(acc, x) }, tally_merge)
    println(f"{named.n}")
    let none = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) { a + b }; workers: 0)
    println(f"not reached {none}")
}
"#;

/// Lower `source` at a setting of `user_parallelism`, compile it, and hand
/// back the binary and the emitted Rust.
fn build(dir: &Path, source: &str, parallel: &str) -> (PathBuf, String) {
    std::fs::create_dir_all(dir).expect("a directory");
    let input = dir.join("main.nika");
    std::fs::write(&input, source).expect("the source");
    let output = dir.join("main.rs");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .args(["lower", input.to_str().unwrap()])
        .args(["--output", output.to_str().unwrap()])
        .args(["--no-cache", "--user-parallelism", parallel])
        .env("NIKAIA_CACHE_DIR", dir.join("cache"))
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "lowering failed:\n{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let rust = std::fs::read_to_string(&output).expect("the emitted Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &output,
        &["--crate-type", "bin", "-o", binary.to_str().unwrap()],
    );
    assert!(
        compiled.status.success(),
        "the emitted Rust did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    (binary, rust)
}

/// Run `binary` with a runtime configuration file, or with none.
fn run(binary: &Path, at: &Path, config: Option<&str>) -> Output {
    std::fs::create_dir_all(at).expect("a directory to run in");
    let mut command = Command::new(binary);
    command.current_dir(at);
    match config {
        Some(text) => {
            let path = at.join("nikaia-runtime.toml");
            std::fs::write(&path, text).expect("the runtime configuration");
            command.env("NIKAIA_RUNTIME_CONFIG", &path);
        }
        None => {
            command.env_remove("NIKAIA_RUNTIME_CONFIG");
        }
    }
    command.output().expect("run the compiled program")
}

/// What every run prints after its first line, the cores: the sequential sum
/// six times and twice it once, a list in input order, `init`'s values over
/// no elements, and the sum through a type made by `init`.
fn the_folds(out: &Output) -> Vec<String> {
    let sum: i64 = (0..1_000_000).sum();
    let lines: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        lines[1..],
        [
            format!("{sum} {sum} {sum} {sum} {} {sum} {sum}", 2 * sum),
            "1000000 0".to_owned(),
            "7 8".to_owned(),
            format!("{sum}"),
            format!("{sum}"),
        ],
        "{lines:?}"
    );
    // **`workers: 0` panics, naming the option** (D6), after everything else.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{lines:?}");
    assert!(stderr.contains("`workers:` is 0"), "{stderr}");
    lines
}

/// **D1, D2, D5-D7**: the same answers at both settings and both orders, and
/// `task::cores()` is 1 at `no`, `user-pool` at `yes`, and the threads the
/// process may run on where `user-pool` is `0`.
#[test]
fn a_fold_is_the_sequential_fold_at_both_settings_and_both_orders() {
    let dir = common::scratch_dir("parallel-folds");
    let (at_no, rust) = build(&dir.join("no"), FOLDS, "no");
    assert!(rust.contains(".fold_merged("), "{rust}");
    assert!(rust.contains("nikaia_std::par::pieced("), "{rust}");
    assert!(!rust.contains("nikaia_std::par::iter("), "{rust}");
    let lines = the_folds(&run(&at_no, &dir.join("run-no"), Some("user-pool = 3\n")));
    assert_eq!(lines[0], "1");

    let (at_yes, rust) = build(&dir.join("yes"), FOLDS, "yes");
    assert!(
        rust.contains("nikaia_std::par::iter_in_pieces(&xs, 1)"),
        "{rust}"
    );
    assert!(rust.contains(".fold_merged("), "{rust}");
    let lines = the_folds(&run(
        &at_yes,
        &dir.join("run-three"),
        Some("user-pool = 3\n"),
    ));
    assert_eq!(lines[0], "3");
    let lines = the_folds(&run(&at_yes, &dir.join("run-default"), None));
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    assert_eq!(lines[0], threads.to_string());
}

/// **`b.par_pieces(sep; piece:)`** (ADR-372 D3, D4, D7): for every piece size,
/// the pieces' lengths add up to the bytes', none is empty, each but the last
/// ends in `sep`, and a fold counting lines under either order is the count a
/// loop over the whole makes. Empty bytes have no pieces; `piece: 0` panics.
const PIECES: &str = r#"use std::par

fn lines_in(p: ref Bytes) -> i64 {
    let mut n = 0
    let mut at = p.find('\n') ?? -1
    while at >= 0 {
        n += 1
        at = p.find('\n'; from: at + 1) ?? -1
    }
    return n
}

fn report(b: ref Bytes, piece: i64) {
    let lens: Vec[i64] = b.par_pieces('\n'; piece: piece).map(fn(p) { p.len() }).collect()
    let mut total = 0
    let mut empty = 0
    let mut unended = 0
    for i in 0..<lens.len() {
        if lens[i] == 0 {
            empty += 1
        }
        total += lens[i]
        if i + 1 < lens.len() && (b.find('\n'; from: total - 1) ?? -1) != total - 1 {
            unended += 1
        }
    }
    let kept = b.par_pieces('\n'; piece: piece).fold(fn { 0 }, fn(n, p) { n + lines_in(p) }, fn(x, y) { x + y })
    let any = b.par_pieces('\n'; piece: piece).fold(fn { 0 }, fn(n, p) { n + lines_in(p) }, fn(x, y) { x + y }; order: par::Order::Any)
    println(f"{piece}: {lens.len()} {total} {empty} {unended} {kept} {any}")
}

fn main() {
    let mut text = ""
    for i in 0..<5000 {
        text += f"station{i % 37};{i % 90}.{i % 10}\n"
    }
    let ended = text.clone()
    text += "unterminated"
    let b = text.bytes
    println(f"{b.len()} {lines_in(b)} {ended.bytes.len()}")
    report(b, 1)
    report(b, 64)
    report(b, 1_000_000)
    report(ended.bytes, 7)
    let whole = b.par_pieces('\n').fold(fn { 0 }, fn(n, p) { n + p.len() }, fn(x, y) { x + y }; order: par::Order::Any)
    let none = ""
    let nothing = none.bytes.par_pieces('\n').map(fn(p) { p.len() }).count()
    let flat = "no separator".bytes.par_pieces('\n'; piece: 2).map(fn(p) { p.len() }).count()
    println(f"{whole} {nothing} {flat}")
    let zero = b.par_pieces('\n'; piece: 0).count()
    println(f"not reached {zero}")
}
"#;

/// How many pieces D3 cuts `bytes` into near every `size` bytes: one per grid
/// point whose next line end differs from the one after it.
fn cuts(bytes: &[u8], size: usize) -> usize {
    let after = |at: usize| -> usize {
        if at == 0 {
            return 0;
        }
        (at..bytes.len())
            .find(|i| bytes[*i] == b'\n')
            .map_or(bytes.len(), |i| i + 1)
    };
    (0..bytes.len().div_ceil(size))
        .filter(|k| after(k * size) != after((k + 1) * size))
        .count()
}

/// The lines [`PIECES`] prints after the lengths, for bytes of `len` with
/// `lines` line ends, the last line unterminated, and `ended` bytes that end
/// in one.
fn the_pieces(out: &Output) {
    let lines: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    let mut ended = String::new();
    for i in 0..5000 {
        ended += &format!("station{};{}.{}\n", i % 37, i % 90, i % 10);
    }
    let whole = format!("{ended}unterminated").into_bytes();
    let len = whole.len();
    let ended = ended.len();
    assert_eq!(
        lines,
        [
            format!("{len} 5000 {ended}"),
            format!("1: 5001 {len} 0 0 5000 5000"),
            format!("64: {} {len} 0 0 5000 5000", cuts(&whole, 64)),
            format!("1000000: 1 {len} 0 0 5000 5000"),
            format!("7: 5000 {ended} 0 0 5000 5000"),
            format!("{len} 0 1"),
        ],
        "{lines:?}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{lines:?}");
    assert!(stderr.contains("`piece:` is 0"), "{stderr}");
}

#[test]
fn bytes_are_cut_into_pieces_that_end_after_the_separator() {
    let dir = common::scratch_dir("parallel-pieces");
    let (at_no, rust) = build(&dir.join("no"), PIECES, "no");
    assert!(
        rust.contains("nikaia_std::par::pieces_in_order(&"),
        "{rust}"
    );
    the_pieces(&run(&at_no, &dir.join("run-no"), None));
    let (at_yes, rust) = build(&dir.join("yes"), PIECES, "yes");
    assert!(rust.contains("nikaia_std::par::pieces(&"), "{rust}");
    the_pieces(&run(&at_yes, &dir.join("run-yes"), Some("user-pool = 4\n")));
}

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    use nikaia::contracts::LedgerOps;
    let parsed = nikaia::parser::parse_to_ast(source).expect("the source parses");
    let own = nikaia::contracts::Ledger::infer(&parsed);
    let library =
        nikaia::contracts::Ledger::parse(nikaia::contracts::STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

/// **D2: `init`, `step` and `merge` are a walk's lambdas**: they may not
/// pause (`NK2209`) and may not change a name outside them (`NK2107`).
#[test]
fn a_folds_lambdas_are_asked_what_a_walks_are() {
    let pausing = "use std::time\n\
         \n\
         fn slow(x: i64) -> i64 {\n\
         \x20   time::sleep(1.millis())\n\
         \x20   return x\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let xs: Vec[i64] = [1, 2]\n\
         \x20   let s = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + slow(x) }, fn(a, b) { a + b })\n\
         \x20   println(f\"{s}\")\n\
         }\n";
    let found = findings(pausing);
    assert!(found.iter().any(|f| f.code == "NK2209"), "{found:#?}");
    let sharing = "fn main() {\n\
         \x20   let xs: Vec[i64] = [1, 2]\n\
         \x20   let mut merges = 0\n\
         \x20   let s = xs.par_iter().fold(fn { 0 }, fn(acc, x) { acc + x }, fn(a, b) {\n\
         \x20       merges += 1\n\
         \x20       return a + b\n\
         \x20   })\n\
         \x20   println(f\"{s} {merges}\")\n\
         }\n";
    let found = findings(sharing);
    assert!(found.iter().any(|f| f.code == "NK2107"), "{found:#?}");
}

/// **A step and a merge that return the accumulator or change it in place**
/// (ADR-372 D9), in every pairing, by a lambda and by a declared function,
/// at both orders: each line is the returning form's answer.
const IN_PLACE: &str = r#"use std::par

struct Summary {
    n: i64,
    total: i64,
}

impl Summary {
    pub fn() -> Summary sync {
        return Summary { n: 0, total: 0 }
    }

    pub fn record(ref mut self, r: i64) {
        self.n += 1
        self.total += r
    }

    pub fn plus(self, r: i64) -> Summary {
        return Summary { n: self.n + 1, total: self.total + r }
    }

    pub fn merge(ref mut self, other: Summary) {
        self.n += other.n
        self.total += other.total
    }

    pub fn joined(self, other: Summary) -> Summary {
        return Summary { n: self.n + other.n, total: self.total + other.total }
    }
}

fn add_into(mut acc: Summary, r: i64) {
    acc.n += 1
    acc.total += r
}

fn merge_ref(mut a: Summary, b: ref Summary) {
    a.n += b.n
    a.total += b.total
}

fn main() {
    let mut xs: Vec[i64] = []
    for i in 0..<100_000 {
        xs.push(i)
    }
    let returning = xs.par_iter().fold(Summary, fn(acc, r) { acc.plus(r) }, Summary::joined)
    println(f"{returning.n} {returning.total}")
    let both = xs.par_iter().fold(Summary, fn(acc, r) { acc.record(r) }, Summary::merge)
    println(f"{both.n} {both.total}")
    let any = xs.par_iter().fold(Summary, fn(acc, r) { acc.record(r) }, Summary::merge; order: par::Order::Any)
    println(f"{any.n} {any.total}")
    let step_only = xs.par_iter().fold(Summary, fn(acc, r) { acc.record(r) }, Summary::joined; order: par::Order::Any)
    println(f"{step_only.n} {step_only.total}")
    let merge_only = xs.par_iter().fold(Summary, fn(acc, r) { acc.plus(r) }, Summary::merge)
    println(f"{merge_only.n} {merge_only.total}")
    let declared = xs.par_iter().fold(Summary, add_into, merge_ref; workers: 2)
    println(f"{declared.n} {declared.total}")
    let fields = xs.par_iter().fold(Summary, fn(acc, r) {
        acc.n += 1
        acc.total += r
    }, fn(a, b) { a.merge(b) }; order: par::Order::Any)
    println(f"{fields.n} {fields.total}")
}
"#;

/// **The same fold over a grammar's frames**: a `par_fold` whose step and
/// merge are declared functions that change the accumulator in place.
const GRAMMAR_IN_PLACE: &str = r#"grammar Nums {
    @frame(boundary: "\n")
    rule LINE -> i64 = d:dec[i64](digit+) frame_end { d }

    entry rule file -> Summary = par_fold(LINE, Summary, add_into, merge_ref)
}

struct Summary {
    n: i64,
    total: i64,
}

impl Summary {
    pub fn() -> Summary sync {
        return Summary { n: 0, total: 0 }
    }
}

fn add_into(mut acc: Summary, r: i64) {
    acc.n += 1
    acc.total += r
}

fn merge_ref(mut a: Summary, b: ref Summary) {
    a.n += b.n
    a.total += b.total
}

fn main() {
    let mut text = ""
    for i in 0..<100_000 {
        text = text + f"{i}\n"
    }
    let s = Nums::file(text) catch { Summary() }
    println(f"{s.n} {s.total}")
}
"#;

/// **D9**: every pairing gives the returning form's answer, at both settings
/// and both orders, and an in-place part runs lent the accumulator.
#[test]
fn a_fold_that_changes_its_accumulator_in_place_is_the_returning_fold() {
    let dir = common::scratch_dir("parallel-folds-in-place");
    let expected = format!("100000 {}", (0..100_000_i64).sum::<i64>());
    for parallel in ["no", "yes"] {
        let (binary, rust) = build(&dir.join(parallel), IN_PLACE, parallel);
        assert!(rust.contains("nikaia_std::par::in_place("), "{rust}");
        let out = run(
            &binary,
            &dir.join(format!("run-{parallel}")),
            Some("user-pool = 3\n"),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines, vec![expected.as_str(); 7], "at {parallel}");

        let (binary, rust) = build(
            &dir.join(format!("grammar-{parallel}")),
            GRAMMAR_IN_PLACE,
            parallel,
        );
        assert!(rust.contains("nikaia_std::par::in_place("), "{rust}");
        let out = run(
            &binary,
            &dir.join(format!("run-grammar-{parallel}")),
            Some("user-pool = 3\n"),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(stdout.trim(), expected, "at {parallel}");
    }
}

/// **D9: any other result type is `NK1102`**, as is a declared function that
/// hands back nothing and does not take the accumulator `mut`.
#[test]
fn a_step_that_hands_back_something_else_is_refused() {
    let text = "struct T {\n\
         \x20   n: i64,\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let xs: Vec[i64] = [1, 2]\n\
         \x20   let s = xs.par_iter().fold(fn { T { n: 0 } }, fn(n, x) { \"a\" }, fn(a, b) { T { n: a.n + b.n } })\n\
         \x20   println(f\"{s.n}\")\n\
         }\n";
    let found = findings(text);
    assert!(found.iter().any(|f| f.code == "NK1102"), "{found:#?}");
    let merge = "struct T {\n\
         \x20   n: i64,\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let xs: Vec[i64] = [1, 2]\n\
         \x20   let s = xs.par_iter().fold(fn { T { n: 0 } }, fn(t, x) { T { n: t.n + x } }, fn(a, b) { \"b\" })\n\
         \x20   println(f\"{s.n}\")\n\
         }\n";
    let found = findings(merge);
    assert!(found.iter().any(|f| f.code == "NK1102"), "{found:#?}");
    let unchanged = "struct T {\n\
         \x20   n: i64,\n\
         }\n\
         \n\
         fn look(t: T, x: i64) {\n\
         \x20   println(f\"{t.n} {x}\")\n\
         }\n\
         \n\
         fn main() {\n\
         \x20   let xs: Vec[i64] = [1, 2]\n\
         \x20   let t = xs.par_iter().fold(fn { T { n: 0 } }, look, fn(a, b) { T { n: a.n + b.n } })\n\
         \x20   println(f\"{t.n}\")\n\
         }\n";
    let found = findings(unchanged);
    assert!(found.iter().any(|f| f.code == "NK1102"), "{found:#?}");
    let fine = "fn main() {\n\
         \x20   let xs: Vec[i64] = [1, 2]\n\
         \x20   let s = xs.par_iter().fold(fn { 0 }, fn(n, x) { n + x }, fn(a, b) { a + b })\n\
         \x20   println(f\"{s}\")\n\
         }\n";
    let found = findings(fine);
    assert!(found.is_empty(), "{found:#?}");
}

/// **A grammar's `par_fold` takes `Par::fold`'s options after its `;`**
/// (ADR-372 D6, Part II 10.7): each typed as there, any other name `NK1109`,
/// a value of another type `NK1102`.
#[test]
fn a_grammars_par_fold_takes_the_options_of_a_fold() {
    let grammar = |options: &str| {
        format!(
            "use std::par\n\n\
             grammar Sum {{\n\
             \x20   @frame(boundary: \"\\n\")\n\
             \x20   rule ROW -> i64 = n:dec[i64](digit+) frame_end {{ n }}\n\n\
             \x20   entry rule total -> i64 = par_fold(ROW, zero, fn(acc, m) {{ acc + m }}, add; {options})\n\
             }}\n\n\
             fn zero() -> i64 {{ return 0 }}\n\
             fn add(a: i64, b: i64) -> i64 {{ return a + b }}\n\n\
             fn main() throws {{\n\
             \x20   println(f\"{{Sum::total(\"1\\n2\\n\")}}\")\n\
             }}\n"
        )
    };
    let fine = findings(&grammar("order: par::Order::Any, workers: 2, piece: 64"));
    assert!(fine.is_empty(), "{fine:?}");
    let unknown = findings(&grammar("pieces: 64"));
    assert!(
        unknown
            .iter()
            .any(|f| f.code == "NK1109" && f.message.contains("`pieces`")),
        "{unknown:?}"
    );
    let typed = findings(&grammar("order: \"any\""));
    assert!(
        typed
            .iter()
            .any(|f| f.code == "NK1102" && f.message.contains("`order:`")),
        "{typed:?}"
    );
    let counted = findings(&grammar("piece: par::Order::Kept"));
    assert!(
        counted
            .iter()
            .any(|f| f.code == "NK1102" && f.message.contains("`piece:`")),
        "{counted:?}"
    );
}

/// **#762: a declared merge taking its second argument by value, and a step
/// lambda handing its accumulator to a `mut` parameter** (`&mut *acc`).
const IN_PLACE_BY_VALUE: &str = "struct Acc { n: i64 }
fn merge(mut a: Acc, other: Acc) { a.n += other.n }
fn main() {
    let xs: Vec[i64] = [1, 2, 3]
    let r = xs.par_iter().fold(fn { Acc { n: 0 } }, fn(acc, x) { acc.n += x }, merge)
    println(f\"{r.n}\")
}
";

const IN_PLACE_HANDED_ON: &str = "struct Acc { n: i64 }
fn add(mut a: Acc, x: i64) { a.n += x }
fn main() {
    let xs: Vec[i64] = [1, 2, 3]
    let r = xs.par_iter().fold(fn { Acc { n: 0 } }, fn(acc, x) { add(acc, x) }, fn(a, b) { a.n += b.n })
    println(f\"{r.n}\")
}
";

#[test]
fn an_in_place_fold_hands_its_arguments_on_as_the_callee_takes_them() {
    let dir = common::scratch_dir("parallel-folds-in-place-762");
    for (name, source) in [("value", IN_PLACE_BY_VALUE), ("handed", IN_PLACE_HANDED_ON)] {
        for parallel in ["no", "yes"] {
            let (binary, _) = build(&dir.join(format!("{name}-{parallel}")), source, parallel);
            let out = run(&binary, &dir.join(format!("run-{name}-{parallel}")), None);
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(stdout.trim(), "6", "{name} at {parallel}");
        }
    }
}
