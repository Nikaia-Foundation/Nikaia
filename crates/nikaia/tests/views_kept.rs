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

/// Runs `source` as `main.nika` and returns what it printed and the lowering.
fn run_735(name: &str, source: &str) -> (String, String) {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let lower = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("lower")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary lowers");
    assert!(
        lower.status.success(),
        "{}",
        String::from_utf8_lossy(&lower.stderr)
    );
    let lowered = std::fs::read_to_string(dir.join("main.rs")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    (String::from_utf8_lossy(&run.stdout).into_owned(), lowered)
}

/// #735 (1): a parameter only used as a map's key is no source of the result,
/// so `resolve` keeps nothing for its owned `k`.
#[test]
fn a_key_parameter_is_no_source_of_the_result() {
    let (out, lowered) = run_735(
        "key-no-source",
        r#"use std::collections

struct C {
    pub sig: String?,
}

struct R {
    pub key: String,
    pub c: ref C,
}

fn find(m: ref collections::HashMap[String, C], key: String) -> ref C? sync {
    return m[key]
}

fn resolve(m: ref collections::HashMap[String, C], name: String) -> R? sync {
    let k = f"{name}::new"
    let c: ref C = find(m, k) ?? return null
    return R { key: k.clone(), c: c }
}

fn main() {
    let mut m: collections::HashMap[String, C] = collections::HashMap()
    m.insert("a::new", C { sig: "s" })
    let r = resolve(m, "a") ?? return
    let s = r.c.sig ?? ""
    print(f"{r.key} {s}")
}
"#,
    );
    assert_eq!(out, "a::new s");
    let resolve = lowered
        .lines()
        .find(|l| l.starts_with("fn resolve"))
        .unwrap_or_default();
    assert!(!resolve.contains("__keep"), "{resolve}");
}

/// #735 (2): a field read through a `ref` field is borrowed, not moved, kept
/// or not, past a jump or with a fallback.
#[test]
fn a_field_through_a_ref_field_is_borrowed() {
    let (out, _) = run_735(
        "through-ref-field",
        r#"struct S {
    pub name: String,
}

struct C {
    pub sig: S?,
    pub text: String?,
}

struct R {
    pub c: ref C,
}

fn wrap(c: ref C) -> R sync {
    return R { c: c }
}

fn name_of(r: ref R) -> String? sync {
    let s = r.c.sig ?? return null
    return s.name.clone()
}

fn text_of(r: ref R) -> String sync {
    return (r.c.text ?? "").clone()
}

fn text_or_null(r: ref R) -> String? sync {
    let t = r.c.text ?? return null
    return f"<{t}>"
}

fn local_name(c: ref C) -> String? sync {
    let found = wrap(c)
    let s = found.c.sig ?? return null
    return s.name.clone()
}

fn find(c: ref C) -> R? sync {
    return wrap(c)
}

fn opt_name(c: ref C) -> String? sync {
    let found = find(c) ?? return null
    let s = found.c.sig ?? return null
    let t = found.c.text ?? ""
    return f"{s.name}{t}"
}

fn main() {
    let c = C { sig: S { name: "n" }, text: f"t{1}" }
    let r = wrap(c)
    let a = name_of(r) ?? "-"
    let b = text_of(r)
    let d = text_or_null(r) ?? "-"
    let e = local_name(c) ?? "-"
    let g = opt_name(c) ?? "-"
    print(f"{a} {b} {d} {e} {g}")
}
"#,
    );
    assert_eq!(out, "n t1 <t1> n nt1");
}

/// #735: a chain of map reads past a jump binds a view, and a set of numbers
/// pins the type of a name it is asked for.
#[test]
fn a_chained_map_read_and_a_numbered_key_run() {
    let (out, _) = run_735(
        "chained-read",
        r#"use std::collections

struct S {
    pub name: String,
}

struct C {
    pub sig: S?,
}

fn one(m: ref collections::HashMap[String, C], k: String) -> String? sync {
    let c = m[k] ?? return null
    let s = c.sig ?? return null
    return s.name.clone()
}

fn two(m: ref collections::HashMap[String, C], n: ref collections::HashMap[String, C], k: String) -> String? sync {
    let c = m[k] ?? n[k] ?? return null
    let s = c.sig ?? return null
    return s.name.clone()
}

fn main() {
    let mut m: collections::HashMap[String, C] = collections::HashMap()
    m.insert("a", C { sig: S { name: "n" } })
    let n: collections::HashMap[String, C] = collections::HashMap()
    let a = one(m, "a") ?? "-"
    let b = two(n, m, "a") ?? "-"
    print(f"{a} {b}")
}
"#,
    );
    assert_eq!(out, "n n");
    let (out, _) = run_735(
        "numbered-key",
        r#"use std::collections

fn count(s: ref collections::BTreeSet[i64], n: i64) -> i64 sync {
    let mut hits = 0
    let mut position = 0
    for _ in 0..<n {
        if s.contains(position) {
            hits += 1
        }
        position += 1
    }
    return hits
}

fn main() {
    let mut s: collections::BTreeSet[i64] = collections::BTreeSet()
    s.insert(1)
    s.insert(3)
    print(f"{count(s, 5)}")
}
"#,
    );
    assert_eq!(out, "2");
}
