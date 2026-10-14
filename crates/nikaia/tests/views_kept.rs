//! **Views kept past a call** (Part I 6.6): where a destination already names
//! the buffer a view parameter points into, the program is lowered.
//!
//! The programs that are lowered, and what they compute, are
//! `tests/language/src/views_kept.nika`; this file keeps what is refused.

mod common;

use nikaia::contracts::LedgerOps;
use nikaia::parser::parse_to_ast;

fn findings(source: &str) -> Vec<nikaia::check::Finding> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = nikaia::contracts::Ledger::infer(&parsed);
    let library =
        nikaia::contracts::Ledger::parse(nikaia::contracts::STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .filter(|f| f.severity == nikaia::check::Severity::Error)
        .collect()
}

/// **A struct of one parameter's view is accepted beside another view**
/// (D25): `Reading { name: a }` points into `a` alone, so `b` is free.
#[test]
fn a_view_from_one_of_two_buffers_is_accepted() {
    let found = findings(
        "struct Reading { name: ref String }\n\n\
         fn pick(a: ref String, b: ref String) -> Reading {\n\
         \x20   return Reading { name: a }\n\
         }\n\n\
         fn main() { println(\"x\") }\n",
    );
    assert!(!found.iter().any(|f| f.code == "NK2302"), "{found:#?}");
}

/// **Owned text is lent to a pattern** (D28): `starts_with`, `ends_with` and
/// `contains` take a view of text or a character, and text of its own handed
/// to one is lent as to any `ref String` parameter - no `NK1102`, and it stays
/// the caller's, in a loop too.
#[test]
fn owned_text_handed_to_a_pattern_is_refused() {
    for (argument, refused) in [
        ("f\"{w}[\"", false),
        ("w", false),
        ("ref w", false),
        ("ref f\"{w}\"", false),
        ("\"ab\"", false),
        ("'a'", false),
    ] {
        let source = format!(
            "fn f(line: ref String, w: String) -> bool {{\n    return line.starts_with({argument})\n}}\n\n\
             fn main() {{\n    let r = f(\"abc\", \"a\")\n    println(f\"{{r}}\")\n}}\n"
        );
        let found = findings(&source);
        let refusal = found.iter().find(|f| f.code == "NK1102");
        assert_eq!(refusal.is_some(), refused, "{argument}: {found:#?}");
        if let Some(refusal) = refusal {
            assert_eq!(
                refusal.help.as_deref(),
                Some("Write `ref` in front of it to pass a view of it."),
                "{argument}"
            );
        }
    }
}

/// **A call on a subject another file of the package declares** keeps nothing
/// where that subject holds no view, and is still refused where it does: the
/// subject is known from the package's ledger, not only from the file the
/// `impl` stands in (found moving the checker's state into Nikaia, #558).
#[test]
fn a_subject_declared_in_another_file_is_known_from_the_ledger() {
    let declaring = parse_to_ast(
        "struct Plain {\n    n: i64,\n}\n\nstruct Holds {\n    name: ref String,\n}\n",
    )
    .expect("the declarations parse");
    let methods = parse_to_ast(
        "impl Plain {\n    fn peek(ref self, label: ref String) -> i64 {\n        return self.n\n    }\n\n\
         \x20   fn show(ref self, label: ref String) -> i64 {\n        return self.peek(label)\n    }\n}\n\n\
         impl Holds {\n    fn peek(ref self, label: ref String) -> i64 {\n        return 1\n    }\n\n\
         \x20   fn show(ref self, label: ref String) -> i64 {\n        return self.peek(label)\n    }\n}\n",
    )
    .expect("the methods parse");
    let library =
        nikaia::contracts::Ledger::parse(nikaia::contracts::STD).expect("std ships a ledger");
    let own = nikaia::contracts::Ledger::infer_package(&[&declaring, &methods], &library);
    let checked = nikaia::check::check_against(
        &methods,
        &[&declaring],
        &own,
        &library,
        &std::collections::BTreeSet::new(),
        &nikaia::check::Newly::default(),
        &nikaia::assets::Reads::none(),
    );
    let refused: Vec<&nikaia::check::Finding> = checked
        .findings
        .iter()
        .filter(|f| f.code == "NK2302")
        .collect();
    assert_eq!(refused.len(), 1, "{refused:#?}");
    assert!(refused[0].message.contains("Holds.show"), "{refused:#?}");
}

/// **Owned text lent to a pattern runs** (D28), and a let-bound function value
/// is lent what it takes as a view, an index read too (D20).
#[test]
fn owned_text_and_index_reads_are_lent() {
    let dir = common::scratch_dir("lent-to-views");
    std::fs::write(
        dir.join("main.nika"),
        "fn main() {\n    let xs: Vec[String] = [\"abc\", \"de\"]\n    let f: fn(ref String, i64) -> i64 sync = fn(a, b) { a.len() + b }\n    let s: String = \"xy\"\n    println(f\"{f(xs[0], 1)} {f(s, 1)} {s}\")\n    let mark = f\"b{1}\"\n    let line = \"ab1c\"\n    println(f\"{line.contains(mark)} {line.starts_with(f\"a{mark}\")}\")\n    for l in xs {\n        println(f\"{l.contains(mark)} {l.ends_with(mark)}\")\n    }\n    println(mark)\n}\n",
    )
    .expect("the source");
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
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "4 3 xy\ntrue true\nfalse false\nfalse false\nb1\n"
    );
}

/// **Reads past a jump and an iterator's text run** (D30, D31): an option in a
/// list element is cloned out of it, a `match` on what `get` handed back past a
/// jump writes no `ref`, a local f-string is lent to `starts_with`, and
/// `find_map(..) ?? "lit"` written directly is not ambiguous.
#[test]
fn reads_past_a_jump_and_found_text_run() {
    let dir = common::scratch_dir("past-a-jump");
    std::fs::write(
        dir.join("main.nika"),
        "use std::collections\n\nstruct Slot {\n    refusal: String?,\n    n: i64,\n}\n\nenum Shape {\n    Named { name: String, n: i64 },\n    Other,\n}\n\nfn has(path: ref String, names: ref Vec[String]) -> bool {\n    for name in names {\n        let p = f\"{name}.\"\n        if path.starts_with(p) {\n            return true\n        }\n    }\n    false\n}\n\nfn main() {\n    let slots: Vec[Slot] = [Slot { refusal: \"a\", n: 1 }, Slot { refusal: null, n: 2 }]\n    for index in 0..<slots.len() {\n        let r = slots[index].refusal ?? continue\n        println(f\"{r} {slots[index].n}\")\n    }\n    let mut m: collections::HashMap[String, Shape] = collections::HashMap()\n    m.insert(\"k\", Shape::Named { name: \"x\", n: 1 })\n    m.insert(\"o\", Shape::Other)\n    let ks: Vec[String] = [\"k\", \"z\", \"o\"]\n    for k in ks {\n        let held = m.get(k) ?? continue\n        match held {\n            Shape::Named { name, .. } => println(name),\n            Shape::Other => println(\"o\"),\n        }\n    }\n    let ns: Vec[String] = [\"a\", \"b\"]\n    println(f\"{has(\"b.c\", ns)}\")\n    let xs: Vec[String] = [\"a\", \"bc\", \"de\"]\n    let a = xs.iter().find_map(fn(s) { if s.len() > 1 { s } else { null } }) ?? \"none\"\n    let b = xs.iter().filter_map(fn(s) { if s.len() > 2 { s } else { null } }).next() ?? \"none\"\n    println(f\"{a} {b}\")\n}\n",
    )
    .expect("the source");
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
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "a 1\nx\no\ntrue\nbc none\n"
    );
}

/// D35: a field option of a local `get` handed back past a jump is opened as
/// over a `ref` parameter, not moved out of the reference (E0507).
#[test]
fn a_field_of_a_get_local_past_a_jump_runs() {
    let dir = common::scratch_dir("get-local-field");
    std::fs::write(dir.join("main.nika"), "use std::collections\n\nstruct C {\n    signature: String?,\n    n: i64?,\n}\n\nfn f(m: ref collections::HashMap[String, C], k: ref String) -> i64 {\n    let c = m.get(k) ?? return 0\n    let s = c.signature ?? return 1\n    let n = c.n ?? return 2\n    return s.len() + n\n}\n\nfn main() {\n    let mut m: collections::HashMap[String, C] = collections::HashMap()\n    m.insert(\"a\", C { signature: \"xyz\", n: 4 })\n    m.insert(\"b\", C { signature: null, n: 4 })\n    println(f\"{f(m, \"a\")} {f(m, \"b\")} {f(m, \"c\")}\")\n}\n").expect("the source");
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "7 1 0\n");
}

/// D36: `get(k)?.clone() ?? x` over a map clones the value, not the reference
/// (no `.as_ref()` on a `get`, which already lends); an owned option's
/// `?.clone()` stays as it was.
#[test]
fn a_cloned_get_with_a_fallback_runs() {
    let dir = common::scratch_dir("cloned-get");
    std::fs::write(dir.join("main.nika"), "use std::collections\n\nenum T {\n    Unknown,\n    Named(String),\n}\n\nfn pick(m: ref collections::BTreeMap[String, T], k: ref String) -> T {\n    return m.get(k)?.clone() ?? T::Unknown\n}\n\nfn show(t: T) -> String {\n    return match t {\n        T::Unknown => \"?\".clone(),\n        T::Named(n) => n,\n    }\n}\n\nfn main() {\n    let mut m: collections::BTreeMap[String, T] = collections::BTreeMap()\n    m.insert(\"a\", T::Named(\"x\"))\n    let mut args: Vec[T] = []\n    args.push(m.get(\"a\")?.clone() ?? T::Unknown)\n    let o: String? = \"y\".clone()\n    let w = o?.clone() ?? \"z\".clone()\n    println(f\"{show(pick(m, \"a\"))} {show(pick(m, \"b\"))} {args.len()} {w} {o ?? \"\"}\")\n}\n").expect("the source");
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "x ? 1 y y\n");
}
