//! **A type's rule, an `assert` after a struct's fields**
//! ([ADR-369](../../../docs/specification/adr/adr-369.md)): checked or proved
//! at every literal and at the end of every method that changes the value
//! (D2), and a fact wherever a value of the type is read (D3).

mod common;

use std::process::Command;

use nikaia::bounds::BoundsChecks;
use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

const RING: &str = r#"
comptime CAP: i64 = 1024

struct Piece {
    start: i64,
    end: i64,
    assert(0 <= start && start <= end; message: "a piece ends after it starts")
}

struct Ring {
    slots: Vec[i64],
    used: i64,
    assert(slots.len() == CAP)
}

impl Piece {
    fn widen(ref mut self, n: i64) {
        self.end += n
    }
    fn length(ref self) -> i64 {
        return self.end - self.start
    }
    fn stretch(ref mut self, n: i64) -> i64 {
        self.end += n
        let was = self.length()
        self.start -= n
        return was
    }
}

impl Ring {
    fn at(ref self, x: i64) -> i64 {
        return self.slots[x & (CAP - 1)]
    }
    fn bump(ref mut self, x: i64) {
        self.used += 1
        self.slots[x & (CAP - 1)] += 1
    }
}

fn get(r: ref Ring, x: i64) -> i64 {
    return r.slots[x & (CAP - 1)]
}

fn ring() -> Ring {
    let mut slots: Vec[i64] = []
    slots.resize(CAP, 0)
    return Ring { slots: slots, used: 0 }
}

fn main() {
    let p = Piece { start: 1, end: 3 }
    let mut q = Piece { start: p.start, end: p.end + 2 }
    q.widen(WIDEN)
    let mut r = ring()
    r.bump(5)
    print(f"{p.length()} {q.length()} {r.at(5)} {get(r, 1029)}")
}
"#;

fn lowered(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    let how = Build {
        bounds: BoundsChecks::Removed,
        ..Build::default()
    };
    emit_program(&parsed, how).expect("it lowers").rust
}

/// The lowered body of the function or method `name`, up to the next `fn`.
fn body<'a>(rust: &'a str, name: &str) -> &'a str {
    let from = rust
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("no `{name}` in:\n{rust}"));
    let rest = &rust[from + 3..];
    let to = rest.find("fn ").unwrap_or(rest.len());
    &rest[..to]
}

/// The program lowered with its index checks removed where proved, compiled
/// and run: whether it ran to the end, and what it printed to each stream.
fn run(purpose: &str, source: &str) -> (bool, String, String) {
    let dir = common::scratch_dir(&format!("type-rules-{purpose}"));
    std::fs::write(dir.join("rules.nika"), source).expect("write the source");
    let lowered = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["lower", "rules.nika", "--output", "rules.rs"])
        .args(["--optimization", "remove-bounds-checks:on"])
        .output()
        .expect("the nikaia binary runs");
    assert!(
        lowered.status.success(),
        "{purpose} did not lower:\n{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let binary = dir.join("rules");
    let compiled = common::compile(
        &dir.join("rules.rs"),
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{purpose} did not compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("run it");
    std::fs::remove_dir_all(&dir).ok();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// D2: a literal the rule is proved of carries no check; one it is not
/// proved of is checked where it is built.
#[test]
fn a_literal_is_proved_or_checked() {
    let rust = lowered(&RING.replace("WIDEN", "2"));
    let main = body(&rust, "__nikaia_main");
    assert_eq!(
        main.matches("assertion_failed").count(),
        1,
        "only `q`'s literal is checked, with the rule's message:\n{main}"
    );
    assert!(
        main.contains("assert(0 <= piece.start && piece.start <= piece.end"),
        "{main}"
    );
    let ring = body(&rust, "ring");
    assert!(ring.contains("assert(ring.slots.len() == CAP"), "{ring}");
    assert!(ring.contains("the rule of `Ring`"), "{ring}");
}

/// D2: a method that changes the value checks the rule when it ends, and
/// one that only reads it does not.
#[test]
fn a_method_that_changes_the_value_keeps_the_rule() {
    let rust = lowered(&RING.replace("WIDEN", "2"));
    assert!(body(&rust, "widen").contains("assertion_failed"), "{rust}");
    assert!(
        !body(&rust, "length").contains("assertion_failed"),
        "{rust}"
    );
    // `self` handed on mid-way holds the rule: checked before the call, as
    // `length` reads it as a fact.
    let stretch = body(&rust, "stretch");
    assert_eq!(stretch.matches("assertion_failed").count(), 2, "{stretch}");
    let call = stretch.find("self.length()").expect("the call");
    assert!(stretch[..call].contains("assertion_failed"), "{stretch}");
    assert!(!body(&rust, "at").contains("assertion_failed"), "{rust}");
    let (ok, out, _) = run("keeps", &RING.replace("WIDEN", "2"));
    assert!(ok);
    assert_eq!(out.trim(), "2 6 1 1");
    let (ok, _, err) = run("breaks", &RING.replace("WIDEN", "-9"));
    assert!(!ok, "a widen by -9 leaves the end before the start");
    assert!(err.contains("a piece ends after it starts"), "{err}");
}

/// D3: `self` in the type's methods and a parameter of the type hold the
/// rule at the entry, so a masked index into the field is proved inside.
#[test]
fn the_rule_proves_an_index() {
    let rust = lowered(&RING.replace("WIDEN", "2"));
    for name in ["at", "bump", "get"] {
        let lowered = body(&rust, name);
        assert!(
            lowered.contains("nikaia_std::proven::read(&"),
            "`{name}` reads its slot without a check:\n{lowered}"
        );
        assert!(!lowered.contains("index::at("), "{lowered}");
    }
}

/// D3 holds only where every change is checked: a `with` that sets a field
/// of the type outside its methods leaves the index checked.
#[test]
fn a_with_leaves_the_rule_no_fact() {
    let source = RING.replace("WIDEN", "2").replace(
        "    r.bump(5)\n",
        "    r.bump(5)\n    let s = r with { used: 7 }\n    print(f\"{s.used}\")\n",
    );
    let rust = lowered(&source);
    assert!(!body(&rust, "at").contains("proven::read"), "{rust}");
}

/// D1: a rule is an `assert` over the fields, `NK1288` otherwise.
#[test]
fn a_rule_is_an_assert_over_the_fields() {
    for (rule, said) in [
        ("a >= 0", "has to be an `assert`"),
        ("assert(a[0] >= 0)", "may read only its fields"),
        (
            "assert(a.len() > 0; reason: \"x\")",
            "has to be an `assert`",
        ),
    ] {
        let source = format!(
            "struct P {{\n    a: Vec[i64],\n    {rule}\n}}\nfn main() {{\n    let p = P {{ a: [1] }}\n    print(f\"{{p.a.len()}}\")\n}}\n"
        );
        let refused = match parse_to_ast(&source) {
            Ok(_) => panic!("`{rule}` is not a rule"),
            Err(e) => format!("{e:#}"),
        };
        assert!(refused.contains("NK1288"), "{refused}");
        assert!(refused.contains(said), "{refused}");
    }
}

/// D3 holds only where every exit is checked: a method that changes `self`
/// and fails through a call it does not catch leaves the value to a caller
/// that may catch the failure, its rule broken, so the rule is no fact.
#[test]
fn a_failure_passed_on_leaves_the_rule_no_fact() {
    let source = r#"
comptime CAP: i64 = 4

enum Bad {
    Odd,
}

impl Error for Bad {
    fn message(ref self) -> String {
        return "odd"
    }
}

struct Ring {
    slots: Vec[i64],
    assert(slots.len() == CAP; message: "a ring has CAP slots")
}

fn checked(n: i64) -> i64 throws {
    if n % 2 == 1 {
        throw Bad::Odd
    }
    return n
}

impl Ring {
    fn at(ref self, x: i64) -> i64 {
        return self.slots[x & (CAP - 1)]
    }
    fn swap_last(ref mut self, n: i64) throws {
        let was = self.slots.pop() ?? 0
        let m = checked(n)
        self.slots.push(m + was)
    }
}

fn main() {
    let mut slots: Vec[i64] = []
    slots.resize(CAP, 7)
    let mut r = Ring { slots: slots }
    r.swap_last(3) catch {
        print("caught")
    }
    print(f"{r.at(3)}")
}
"#;
    let rust = lowered(source);
    assert!(!body(&rust, "at").contains("proven::read"), "{rust}");
    let (ok, out, err) = run("fails", source);
    assert!(!ok, "the slot read past the end:\n{out}");
    assert!(err.contains("out of bounds"), "{err}");
}

const RING_FILE: &str = r#"
comptime CAP: i64 = 4

pub struct Ring {
    pub slots: Vec[i64],
    assert(slots.len() == CAP; message: "a ring has CAP slots")
}

impl Ring {
    pub fn at(ref self, x: i64) -> i64 {
        return self.slots[x & (CAP - 1)]
    }
}

pub fn ring() -> Ring {
    let mut slots: Vec[i64] = []
    slots.resize(CAP, 7)
    return Ring { slots: slots }
}
"#;

/// A package of two files, `ring.nika` and `main.nika` with `main`'s body,
/// run with its index checks removed where proved: whether it ran to the
/// end, and what it printed to each stream.
fn run_package(purpose: &str, main: &str) -> (bool, String, String) {
    let dir = common::scratch_dir(&format!("type-rules-{purpose}"));
    std::fs::create_dir_all(dir.join("src")).expect("make src");
    std::fs::write(
        dir.join("nikaia.toml"),
        "[package]\nname = \"ringed\"\nversion = \"0.1.0\"\n",
    )
    .expect("write the manifest");
    // Its field private, so only this package can break the rule.
    std::fs::write(
        dir.join("src/ring.nika"),
        RING_FILE.replace("pub slots", "slots"),
    )
    .expect("write ring.nika");
    std::fs::write(
        dir.join("src/main.nika"),
        format!("fn main() {{\n{main}}}\n"),
    )
    .expect("write main.nika");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["run", "--optimization", "remove-bounds-checks:on"])
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// D3 across the files of a package: a file that writes a field of a type
/// another file states a rule of checks no rule, so the rule is no fact
/// anywhere in the package; a file that only reads leaves it one.
#[test]
fn another_file_of_the_package_leaves_the_rule_no_fact() {
    let (ok, out, err) = run_package(
        "file-write",
        "    let mut r = ring()\n    let gone = r.slots.pop() ?? 0\n    print(f\"{gone} {r.at(3)}\")\n",
    );
    assert!(!ok, "the slot read past the end:\n{out}");
    assert!(err.contains("out of bounds"), "{err}");
    let (ok, out, err) = run_package(
        "file-read",
        "    let r = ring()\n    print(f\"{r.slots.len()} {r.at(3)}\")\n",
    );
    assert!(ok, "{err}");
    assert_eq!(out.trim(), "4 7");
}

/// D2 across the files of a package: a literal built in another file than
/// the type's is checked against the type's rule, as one in its own file is.
#[test]
fn a_literal_in_another_file_is_checked() {
    let (ok, out, err) = run_package(
        "file-literal",
        "    let r = Ring { slots: [1, 2] }\n    print(f\"{r.at(3)}\")\n",
    );
    assert!(!ok, "the literal broke the rule:\n{out}");
    assert!(
        err.contains("the rule of `Ring`: a ring has CAP slots"),
        "{err}"
    );
    let (ok, out, err) = run_package(
        "file-literal-kept",
        "    let r = Ring { slots: [1, 2, 3, 4] }\n    print(f\"{r.at(3)}\")\n",
    );
    assert!(ok, "{err}");
    assert_eq!(out.trim(), "4");
}

/// A package another package uses cannot see what that package does with a
/// `pub` field its rule reads: the rule is no fact in the dependency, so a
/// slot popped by the consumer leaves the index checked, with the bounds
/// checks removed where proved.
#[test]
fn a_rule_over_a_pub_field_is_no_fact_in_a_dependency() {
    let dir = common::scratch_dir("type-rules-dependency");
    for (name, manifest, file, source) in [
        (
            "lib",
            "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
            "lib.nika",
            RING_FILE.to_string(),
        ),
        (
            "app",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n",
            "main.nika",
            "use lib\n\nfn main() {\n    let mut r = lib::ring()\n    let gone = r.slots.pop() ?? 0\n    print(f\"{gone} {r.at(3)}\")\n}\n".to_string(),
        ),
    ] {
        std::fs::create_dir_all(dir.join(name).join("src")).expect("make src");
        std::fs::write(dir.join(name).join("nikaia.toml"), manifest).expect("write the manifest");
        std::fs::write(dir.join(name).join("src").join(file), source).expect("write the source");
    }
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(dir.join("app"))
        .args(["run", "--optimization", "remove-bounds-checks:on"])
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("the nikaia binary runs");
    std::fs::remove_dir_all(&dir).ok();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "the slot read past the end:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(err.contains("out of bounds"), "{err}");
}

/// D3 holds only where every change is checked: a field of the type changed
/// outside its methods leaves the index checked.
#[test]
fn a_field_changed_outside_leaves_the_rule_no_fact() {
    let source = RING.replace("WIDEN", "2").replace(
        "    r.bump(5)\n",
        "    r.bump(5)\n    let gone = r.slots.pop() ?? 0\n    print(f\"{gone} {r.at(1023)}\")\n",
    );
    let rust = lowered(&source);
    assert!(!body(&rust, "at").contains("proven::read"), "{rust}");
    let (ok, _, err) = run("outside", &source);
    assert!(!ok, "the slot read past the end");
    assert!(err.contains("out of bounds"), "{err}");
}

/// A generic struct states no rule yet, so none is a fact: `NK1288`.
#[test]
fn a_generic_struct_states_no_rule() {
    let source = "struct P[T] {\n    a: Vec[T],\n    assert(a.len() > 0)\n}\nfn main() {\n    let p = P { a: [1] }\n    print(f\"{p.a.len()}\")\n}\n";
    let refused = match parse_to_ast(source) {
        Ok(_) => panic!("a generic struct's rule is refused"),
        Err(e) => format!("{e:#}"),
    };
    assert!(refused.contains("NK1288"), "{refused}");
    assert!(refused.contains("`P` is generic"), "{refused}");
}

/// D2: a failure names the type, with the rule's message or without one.
#[test]
fn a_failure_names_the_type() {
    let source = r#"
struct Count {
    n: i64,
    assert(n >= 0)
}

impl Count {
    fn down(ref mut self) {
        self.n -= 1
    }
}

fn main() {
    let mut c = Count { n: 0 }
    c.down()
    print(f"{c.n}")
}
"#;
    let (ok, _, err) = run("names", source);
    assert!(!ok, "the count went below 0");
    assert!(err.contains("the rule of `Count`"), "{err}");
    let (ok, _, err) = run("names-said", &RING.replace("WIDEN", "-9"));
    assert!(!ok);
    assert!(
        err.contains("the rule of `Piece`: a piece ends after it starts"),
        "{err}"
    );
}
