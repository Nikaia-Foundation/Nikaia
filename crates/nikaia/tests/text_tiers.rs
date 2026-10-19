//! **What a `String` field or result is below is decided by what flows into
//! it** ([ADR-282](../../../docs/specification/adr/adr-282.md)), and each
//! tier is compiled and **run** here at both settings of `user_parallelism`:
//!
//! | what flows in | below |
//! | :--- | :--- |
//! | text of its own only | `String`, unchanged |
//! | views (and literals) only | a view, its buffer placed by ADR-283 |
//! | both | `EitherText`: each value borrowed or owned where it is put in |
//!
//! Not one of these programs writes `ref`, `.clone()` or anything about
//! where text lives. What the one program that reads no file computes is
//! `tests/language/src/text_tiers.nika`.

mod common;

use nikaia::contracts::LedgerOps;
use std::process::Command;

use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

const APP_CONF: &str = "# settings\nhost = example.org\nport = 8080\ninclude = extra.conf\n";
const EXTRA_CONF: &str = "mode = fast\nhost = override.org\n";

fn lowered(source: &str, how: Build) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, how).expect("the source lowers").rust
}

/// Compile the lowering and run it in a directory holding the two files.
fn ran(purpose: &str, source: &str, how: Build) -> String {
    let rust = lowered(source, how);
    let dir = common::scratch_dir(&format!("text-tiers-{purpose}"));
    std::fs::write(dir.join("app.conf"), APP_CONF).expect("write app.conf");
    std::fs::write(dir.join("extra.conf"), EXTRA_CONF).expect("write extra.conf");
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
        "{purpose} did not compile:\n{}\n--- emitted ---\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let out = Command::new(&binary)
        .current_dir(&dir)
        .output()
        .expect("run it");
    assert!(
        out.status.success(),
        "{purpose} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    std::fs::remove_dir_all(&dir).ok();
    printed
}

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

/// Both settings, the same output, and no refusal on the way.
fn runs(purpose: &str, source: &str, expected: &str) {
    assert!(
        findings(source).is_empty(),
        "{purpose}: {:#?}",
        findings(source)
    );
    for how in [Build::default(), Build::parallel()] {
        assert_eq!(ran(purpose, source, how), expected, "{purpose} at {how:?}");
    }
}

/// **The four shapes the README listed as a wall**, each now a program:
/// a `ref String` parameter, a slice of a buffer the function read, a name
/// bound to a literal - all kept in a `String` field - and a view handed back
/// from a function declared `-> String`.
const WALL: &str = r##"use std::fs

struct Person {
    name: String,
}

struct Config {
    host: String,
}

fn make(n: ref String) -> Person {
    return Person { name: n }
}

fn load(path: String) -> Config throws {
    let text = fs::read_to_string(path, fs::Root::Anywhere)
    return Config { host: text.trim() }
}

fn first_word(line: ref String) -> String {
    return line.trim()
}

fn main() throws {
    let ada = make("ada")
    let config = load("app.conf")
    let n = "grace"
    let grace = Person { name: n }
    let word = first_word("  hi  ")
    println(f"{ada.name} {config.host.len()} {grace.name} [{word}]")
}
"##;

#[test]
fn a_view_kept_where_a_string_is_declared_is_a_view() {
    runs("wall", WALL, "ada 62 grace [hi]");
    let rust = lowered(WALL, Build::default());
    // `Person.name` only ever receives views: it is one, and costs nothing.
    assert!(rust.contains("struct Person<'a>"), "{rust}");
    // The slice of the file: the buffer lives in the caller's keep (ADR-283).
    assert!(
        rust.contains("__keep: &'k nikaia_std::tether::Keep"),
        "{rust}"
    );
    // The result is a view of the parameter.
    assert!(rust.contains("fn first_word(line: &str) -> &str"), "{rust}");
    // `Config.host` too, and nothing copies.
    assert!(!rust.contains("to_owned"), "{rust}");
}

/// **Both kinds of text flow into one field**: text of its own where one line
/// builds it, a view where another line cuts it from a buffer. Neither pays
/// for the other - the owned text is moved in, the view is borrowed.
const MIXED: &str = r##"use std::fs

struct Entry {
    key: String,
    note: String,
}

fn parse(path: String) -> Vec[Entry] throws {
    let text = fs::read_to_string(path, fs::Root::Anywhere)
    let mut out: Vec[Entry] = Vec()
    for line in text.lines() {
        let parts: Vec[ref String] = line.split("=").collect()
        if parts.len() == 2 {
            out.push(Entry { key: parts[0].trim(), note: f"from {parts[1].trim()}" })
        } else {
            out.push(Entry { key: f"line {out.len()}", note: line })
        }
    }
    return out
}

fn describe(line: ref String) -> String {
    if line.starts_with("#") {
        return f"comment of {line.len()}"
    }
    return line.trim()
}

fn main() throws {
    let entries = parse("app.conf")
    for e in entries {
        println(f"{e.key}|{e.note}")
    }
    println(f"{entries.len()} {entries[1].key.clone().len()}")
    let first = describe("# settings")
    let second = describe("  port = 8080 ")
    println(f"{first}|{second}")
}
"##;

#[test]
fn both_kinds_of_text_in_one_field_each_as_it_is() {
    runs(
        "mixed",
        MIXED,
        "line 0|# settings\nhost|from example.org\nport|from 8080\ninclude|from extra.conf\n4 4\n\
         comment of 10|port = 8080",
    );
    let rust = lowered(MIXED, Build::default());
    assert!(
        rust.contains("key: nikaia_std::either_text::EitherText<'a>"),
        "{rust}"
    );
    assert!(
        rust.contains("note: nikaia_std::either_text::EitherText<'a1>"),
        "{rust}"
    );
    // A result both kinds flow into: each `return` as it is.
    assert!(
        rust.contains("fn describe(line: &str) -> nikaia_std::either_text::EitherText<'_>"),
        "{rust}"
    );
}

/// **Text of its own only: nothing changes.** The ordinary field is a
/// `String` below, as before this record, and a literal in it is built where
/// it stands (ADR-282 D4). That it runs is in `text_tiers.nika`.
#[test]
fn a_field_only_text_of_its_own_flows_into_stays_a_string() {
    let source = r##"struct Greeting {
    text: String,
}

fn main() {
    let who = "world"
    let a = Greeting { text: f"hello {who}" }
    let b = Greeting { text: "hi" }
    println(f"{a.text} {b.text}")
}
"##;
    let rust = lowered(source, Build::default());
    assert!(
        rust.contains("struct Greeting {\n    text: String,"),
        "{rust}"
    );
}

/// **What the compiler chose is a report** (ADR-282 D26):
/// `--tethers` names each `String` field or result that is not text of its
/// own below, and why.
#[test]
fn the_tiers_are_reported() {
    let parsed = parse_to_ast(MIXED).expect("the source parses");
    let ledger = nikaia::contracts::Ledger::infer(&parsed);
    let report = nikaia::contracts::tether::report(&parsed, &ledger);
    assert!(
        report.contains("`Entry.key` is a view or text of its own, per value"),
        "{report}"
    );
    assert!(
        report.contains("what `describe` hands back is a view or text of its own"),
        "{report}"
    );
    let parsed = parse_to_ast(WALL).expect("the source parses");
    let ledger = nikaia::contracts::Ledger::infer(&parsed);
    let report = nikaia::contracts::tether::report(&parsed, &ledger);
    assert!(
        report.contains("`Person.name` is a view: only views and literals flow into it"),
        "{report}"
    );
}

/// **A published field is never both kinds** (ADR-282 D22): its
/// representation leaves with the package, before the programs that build it
/// exist. The view is refused as before, saying why.
#[test]
fn a_published_field_both_kinds_would_flow_into_stays_text_of_its_own() {
    let source = r##"pub struct Person {
    pub name: String,
}

pub fn named(n: ref String) -> Person {
    return Person { name: n }
}

pub fn built(n: i64) -> Person {
    return Person { name: f"person {n}" }
}

fn main() {
    println(named("ada").name)
}
"##;
    let found = findings(source);
    assert!(
        found
            .iter()
            .any(|f| f.code == "NK1106" && f.message.contains("`Person.name` holds `String`")),
        "{found:#?}"
    );
    // The same for one that may be absent (ADR-282 D12).
    let nullable = source
        .replace("pub name: String,", "pub name: String?,")
        .replace(
            "println(named(\"ada\").name)",
            "println(named(\"ada\").name ?? \"-\")",
        );
    let found = findings(&nullable);
    assert!(found.iter().any(|f| f.code == "NK1106"), "{found:#?}");
}

// ---------------------------------------------------------------------------
// Every other place a program declares `String`
// ([ADR-282](../../../docs/specification/adr/adr-282.md))
// ---------------------------------------------------------------------------

/// **A parameter, an annotated `let`, and the elements of a list and a map**
/// are positions too. Every line here puts a view where the program declared
/// `String`, none writes a copy, and nothing is copied.
const EVERYWHERE: &str = r##"use std::fs
use std::collections

struct Person {
    name: String,
}

fn keep(s: String) -> Person {
    return Person { name: s }
}

fn words(text: ref String) -> Vec[String] {
    let mut out: Vec[String] = Vec()
    for w in text.split(" ") {
        out.push(w)
    }
    return out
}

fn main() throws {
    let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
    let first: String = text.trim()
    let p = keep(first)
    let mut names: Vec[String] = Vec()
    let mut counts: collections::HashMap[String, i64] = collections::HashMap()
    for line in text.lines() {
        names.push(line.trim())
        for w in line.split(" ") {
            counts[w] = (counts[w] ?? 0) + 1
        }
    }
    let ws = words("a b c")
    println(f"{p.name.len()} {names.len()} {counts["="] ?? 0} {ws.len()} {ws[2]}")
}
"##;

#[test]
fn a_view_in_a_parameter_a_let_a_list_or_a_map_is_a_view() {
    runs("everywhere", EVERYWHERE, "62 4 3 3 c");
    let rust = lowered(EVERYWHERE, Build::default());
    assert!(rust.contains("fn keep(s: &str)"), "{rust}");
    assert!(rust.contains("-> Vec<&str>"), "{rust}");
    assert!(!rust.contains("to_owned"), "{rust}");
}

/// **Both kinds into one parameter, one `let` and one list**: each value is
/// handed over as it is - borrowed where it is a view, moved where it is text
/// of its own.
const MIXED_EVERYWHERE: &str = r##"use std::fs

fn shout(s: String) -> String {
    return f"{s}!"
}

struct Tag {
    s: String,
}

fn tag(s: String) -> Tag {
    return Tag { s: s }
}

fn main() throws {
    let text = fs::read_to_string("extra.conf", fs::Root::Anywhere)
    let mut last: String = f"nothing"
    let mut all: Vec[String] = Vec()
    for line in text.lines() {
        last = line.trim()
        all.push(line)
        all.push(f"#{all.len()}")
    }
    let loud = shout(last)
    // A parameter that only reads, or hands back what it was given, is lent
    // either kind, from inside a hole as anywhere else.
    println(f"{loud} {shout(f"x")} {tag(last).s} {tag(f"y").s} {all.len()} {all[1]}")
}
"##;

#[test]
fn both_kinds_into_a_parameter_a_let_and_a_list_each_as_it_is() {
    runs(
        "mixed-everywhere",
        MIXED_EVERYWHERE,
        "host = override.org! x! host = override.org y 4 #1",
    );
    let rust = lowered(MIXED_EVERYWHERE, Build::default());
    assert!(
        rust.contains("Vec<nikaia_std::either_text::EitherText<'_>>"),
        "{rust}"
    );
    assert!(rust.contains(".into_either()"), "{rust}");
    assert!(rust.contains("fn shout(s: &str)"), "{rust}");
    // `tag` hands its parameter back in the `Tag` it returns, so it lends
    // it (#678): the `Tag` is a view of either kind, the literal kept beside
    // the call.
    assert!(rust.contains("fn tag(s: &str) -> Tag<'_>"), "{rust}");
    assert!(rust.contains("tag(&last).s"), "{rust}");
}

/// **A `String?` is a position too** ([ADR-282](../../../docs/specification/adr/adr-282.md)
/// D1): views only, and `null`, which is no text of either kind, make it a
/// view that may be absent.
const NULLABLE: &str = r##"use std::fs

struct Entry {
    note: String?,
}

fn port(text: ref String) -> String? {
    for line in text.lines() {
        if line.starts_with("port") {
            return line.trim()
        }
    }
    return null
}

fn main() throws {
    let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
    let e = Entry { note: text.lines().next() }
    let f = Entry { note: null }
    let mut last: String? = null
    for line in text.lines() {
        last = line.trim()
    }
    println(f"{e.note ?? "-"} {f.note ?? "-"} {port(text) ?? "-"} {last ?? "-"}")
}
"##;

#[test]
fn a_view_that_may_be_absent_is_a_view() {
    runs(
        "nullable",
        NULLABLE,
        "# settings - port = 8080 include = extra.conf",
    );
    let rust = lowered(NULLABLE, Build::default());
    assert!(rust.contains("note: Option<&'a str>"), "{rust}");
    assert!(rust.contains("-> Option<&str>"), "{rust}");
    assert!(!rust.contains("to_owned"), "{rust}");
}

/// **Both kinds into a `String?`**: each value goes in as it is, present or
/// absent - a field, a result, a `let` and the element of a list.
const NULLABLE_MIXED: &str = r##"use std::fs

struct Entry {
    note: String?,
}

fn find(text: ref String, fallback: bool) -> String? {
    for line in text.lines() {
        if line.starts_with("port") {
            return line.trim()
        }
    }
    if fallback {
        return f"none in {text.len()}"
    }
    return null
}

fn main() throws {
    let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
    let e = Entry { note: text.lines().next() }
    let f = Entry { note: f"made" }
    let g = Entry { note: null }
    let mut last: String? = null
    for line in text.lines() {
        last = line.trim()
    }
    let n = last ?? "-"
    last = f"{n}!"
    let mut all: Vec[String?] = Vec()
    all.push(text.lines().next())
    all.push(f"x")
    all.push(null)
    println(f"{e.note ?? "-"} {f.note ?? "-"} {g.note ?? "-"} {find(text, true) ?? "-"} {find("a", true) ?? "-"} {last ?? "-"} {all.len()}")
}
"##;

#[test]
fn both_kinds_into_a_value_that_may_be_absent_each_as_it_is() {
    runs(
        "nullable-mixed",
        NULLABLE_MIXED,
        "# settings made - port = 8080 none in 1 include = extra.conf! 3",
    );
    let rust = lowered(NULLABLE_MIXED, Build::default());
    assert!(
        rust.contains("note: Option<nikaia_std::either_text::EitherText<'a>>"),
        "{rust}"
    );
    assert!(rust.contains(".into_either_maybe()"), "{rust}");
    assert!(rust.contains("note: None"), "{rust}");
}

/// **A list going in whole** ([ADR-282](../../../docs/specification/adr/adr-282.md)
/// D3): a literal's items one by one, a `collect` item by item, and a list
/// handed back and bound - with a type or without - is the list it came from,
/// one representation below, so nothing is converted.
const WHOLE: &str = r##"use std::fs

fn words(text: ref String) -> Vec[String] {
    let mut out: Vec[String] = Vec()
    for w in text.split(" ") {
        out.push(w)
    }
    return out
}

fn main() throws {
    let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
    let first = text.lines().next() ?? ""
    let mut a: Vec[String] = [first, f"own"]
    a.push(first.trim())
    let mut b: Vec[String] = text.lines().collect()
    b.push(f"more")
    let mut c = words(text)
    c.push(f"tail")
    let d: Vec[String] = words(text)
    println(f"{a.len()} {b.len()} {c.len()} {a[1]} {b[4]} {c[0]} {d[1]}")
}
"##;

#[test]
fn a_list_going_in_whole_is_handed_over_item_by_item_or_is_the_list_it_came_from() {
    runs("whole", WHOLE, "3 5 9 own more # settings\nhost");
    let rust = lowered(WHOLE, Build::default());
    assert!(rust.contains(".either_items().collect"), "{rust}");
    assert!(rust.contains("first.into_either(), "), "{rust}");
    assert!(
        rust.contains("-> Vec<nikaia_std::either_text::EitherText<'_>>"),
        "{rust}"
    );
}

/// **A view cut from text is typed as one** ([ADR-282](../../../docs/specification/adr/adr-282.md)
/// D1): `trim` on text of its own, `lines`, `split` hand back views, so a view
/// handed straight to a parameter both kinds flow into is borrowed there.
const VIEWS_TYPED: &str = r##"use std::fs

struct Tag {
    s: String,
}

fn tag(s: String) -> Tag {
    return Tag { s: s }
}

fn main() throws {
    let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
    let a = tag(text.lines().next() ?? "")
    let b = tag(f"own")
    let mut all: Vec[Tag] = Vec()
    for line in text.lines() {
        for word in line.split(" ") {
            all.push(tag(word.trim()))
        }
    }
    println(f"{a.s} {b.s} {all.len()} {all[1].s}")
}
"##;

#[test]
fn a_view_cut_from_text_goes_into_a_mixed_parameter_as_it_is() {
    runs("views-typed", VIEWS_TYPED, "# settings own 11 settings");
}

/// **And where it cannot go as it is, it is refused** (ADR-282 D19): a list a
/// published function hands back is text of its own (ADR-282 D23), so a view
/// pushed into it is kept, and the checker says so - where before the
/// `rustc` of the file below did.
#[test]
fn a_view_pushed_into_a_published_list_is_refused_with_the_copy_named() {
    let found = findings(
        r##"use std::fs

pub fn names() -> Vec[String] {
    let mut out: Vec[String] = Vec()
    out.push(f"a")
    return out
}

fn main() throws {
    let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
    let mut c = names()
    for line in text.lines() {
        c.push(line.trim())
    }
    println(f"{c.len()}")
}
"##,
    );
    assert!(
        found.iter().any(|f| f.code == "NK1102"
            && f.message
                .contains("`Vec::push` expects `value` to be `String` here")
            && f.help.iter().any(|h| h.contains(".clone()"))),
        "{found:#?}"
    );
}

/// **A view handed back out of a parameter that holds views**
/// ([ADR-282](../../../docs/specification/adr/adr-282.md)): an element of a
/// list of views, a view field of a struct - the result is the buffer's, not a
/// borrow of the parameter - and a field both kinds flow into, which is handed
/// over with its struct as a `String` field is.
const OUT_OF_A_PARAMETER: &str = r##"use std::fs

struct Entry {
    key: String,
}

struct Seen {
    at: String,
}

fn label(e: Entry) -> String {
    return e.key
}

fn place(s: Seen) -> String {
    return s.at
}

fn first(xs: Vec[String]) -> String {
    return xs[0]
}

fn main() throws {
    let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
    let a = Entry { key: text.trim() }
    let b = Entry { key: f"own" }
    let s = Seen { at: text.lines().next() ?? "" }
    let mut names: Vec[String] = Vec()
    for line in text.lines() {
        names.push(line.trim())
    }
    println(f"{label(b)} {place(s)} {first(names)} {names.len()} {label(a).len()}")
}
"##;

#[test]
fn a_view_handed_back_out_of_a_parameter_that_holds_views_is_the_buffers() {
    runs(
        "out-of-a-parameter",
        OUT_OF_A_PARAMETER,
        "own # settings # settings 4 62",
    );
    let rust = lowered(OUT_OF_A_PARAMETER, Build::default());
    assert!(
        rust.contains("fn first<'a>(xs: &[&'a str]) -> &'a str"),
        "{rust}"
    );
    assert!(
        rust.contains("fn place<'a>(s: &Seen<'a>) -> &'a str"),
        "{rust}"
    );
    assert!(
        rust.contains("fn label(e: Entry<'_>) -> nikaia_std::either_text::EitherText<'_>"),
        "{rust}"
    );
}

/// **A value handed over inside an `f"…"` hole is handed over there**
/// ([ADR-229](../../../docs/specification/adr/adr-229.md)): a key put into a
/// map, and a list literal handed to a function, both written in a hole, into
/// positions both kinds flow into. The hole used to keep its position text of
/// its own, which refused the lines outside it as well.
const IN_A_HOLE: &str = r##"use std::fs
use std::collections

fn count(xs: Vec[String]) -> i64 {
    let mut n = 0
    for x in xs {
        n = n + x.len()
    }
    return n
}

fn main() throws {
    let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
    let mut m: collections::HashMap[String, i64] = collections::HashMap()
    for line in text.lines() {
        m[line] = line.len()
    }
    m[f"own"] = 3
    println(f"{m.insert(text.trim(), 1) ?? 0} {m.len()} {count([text.trim(), f"x"])} {count([f"ab"])}")
}
"##;

#[test]
fn a_value_handed_over_inside_a_hole_is_handed_over_there() {
    runs("in-a-hole", IN_A_HOLE, "0 6 63 2");
    let rust = lowered(IN_A_HOLE, Build::default());
    assert!(
        rust.contains("m.insert(text.trim().into_either(), 1)"),
        "{rust}"
    );
    assert!(
        rust.contains("vec![text.trim().into_either(), \"x\".to_string().into_either()]"),
        "{rust}"
    );
}

// ---------------------------------------------------------------------------
/// **A `String` result that hands back a view of a borrowed parameter is a
/// view result** ([ADR-282](../../../docs/specification/adr/adr-282.md) D2,
/// D12, #663): a field of `ref self`, and a method of the program's that
/// returns a view of it, point into `self`, so `-> String` lowers exactly as
/// `-> ref String` does. Both were refused before, `NK1131` and `NK1104`.
const VIEW_RESULTS: &str = r##"struct Head { path: String }
struct Req { head: Head, body: String }
impl Head {
    pub fn path(ref self) -> String { return self.path }
}
impl Req {
    pub fn path(ref self) -> String { return self.head.path() }
    pub fn body(ref self) -> String { return self.body }
    pub fn trimmed(ref self) -> String { return self.body.trim() }
}
fn main() {
    let r = Req { head: Head { path: f"/{1}" }, body: f" x{2} " }
    println(r.path())
    println(r.body())
    println(r.trimmed())
}
"##;

#[test]
fn a_view_of_a_borrowed_parameter_handed_back_is_a_view_result() {
    runs("view-results", VIEW_RESULTS, "/1\n x2 \nx2");
    let written = VIEW_RESULTS.replace("-> String", "-> ref String");
    assert_eq!(
        lowered(VIEW_RESULTS, Build::default()),
        lowered(&written, Build::default()),
        "`-> String` lowers as `-> ref String` does"
    );
}

/// **A result both kinds flow into hands over text of its own**: the view of
/// `self` is refused, naming `.clone()`, and with it the result owns its text.
/// A published one is refused the same way (ADR-282 D22).
#[test]
fn a_result_both_kinds_flow_into_owns_its_text() {
    let mixed = r##"struct Head { path: String }
impl Head {
    fn pick(ref self, c: bool) -> String {
        if c { return self.path }
        return f"own{1}"
    }
}
fn main() {
    let h = Head { path: f"/{1}" }
    println(h.pick(true))
    println(h.pick(false))
}
"##;
    let found = findings(mixed);
    assert!(found.iter().any(|f| f.code == "NK1131"), "{found:#?}");
    let published = mixed.replace("    fn pick", "    pub fn pick");
    let found = findings(&published);
    assert!(found.iter().any(|f| f.code == "NK1131"), "{found:#?}");
    let copied = mixed.replace("return self.path }", "return self.path.clone() }");
    runs("mixed-result", &copied, "/1\nown1");
    let rust = lowered(&copied, Build::default());
    assert!(
        rust.contains("fn pick(&self, c: bool) -> String {"),
        "{rust}"
    );
}

/// **`examples/http` without `ref` on its results lowers to the same Rust**
/// (#663, #654): its accessors hand back views of the head `std::http1`
/// parsed and of the body, which is what the results are decided to be.
#[test]
fn the_http_example_without_ref_on_its_results_lowers_the_same() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/http/src/main.nika");
    let source = std::fs::read_to_string(&path).expect("examples/http/src/main.nika");
    assert!(
        !source.contains("-> ref String"),
        "the example writes no `ref`"
    );
    let mut with = source.clone();
    for accessor in ["method_text", "path", "target", "body"] {
        let written = format!("fn {accessor}(ref self) -> String");
        assert!(with.contains(&written), "{written}");
        with = with.replace(&written, &format!("fn {accessor}(ref self) -> ref String"));
    }
    for accessor in ["query", "header"] {
        let written = format!("fn {accessor}(ref self, name: ref String) -> String?");
        assert!(with.contains(&written), "{written}");
        with = with.replace(
            &written,
            &format!("fn {accessor}(ref self, name: ref String) -> ref String?"),
        );
    }
    assert!(findings(&source).is_empty(), "{:#?}", findings(&source));
    assert_eq!(
        lowered(&with, Build::default()),
        lowered(&source, Build::default())
    );
}

/// **A `String` parameter the body only reads is handed on as one written
/// `ref String` is** (ADR-282 D2, D5, #654): `count(dna, k)` and
/// `counts.get(fragment)` lend it as it is, with no `&` the written view does
/// not get, and a parameter only literals flow into is a view of text, not
/// static text.
#[test]
fn the_k_nucleotide_example_without_ref_on_its_parameters_lowers_the_same() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/k-nucleotide/src/main.nika");
    let source = std::fs::read_to_string(&path).expect("examples/k-nucleotide/src/main.nika");
    assert!(source.contains("dna: String, fragment: String"));
    let with = source
        .replace("dna: String", "dna: ref String")
        .replace("fragment: String", "fragment: ref String");
    assert!(findings(&source).is_empty(), "{:#?}", findings(&source));
    assert_eq!(
        lowered(&with, Build::default()),
        lowered(&source, Build::default())
    );
}

/// **The same for a lent option and for a parameter handed from one reading
/// function to another** (#654): `inner(sep, n)`, `opt(n; sep: sep)` and
/// `shout(s)` lend what they are given as the `ref String` twins do.
const LENT_HANDED_ON: &str = r##"fn inner(s: ref String, n: i64) -> i64 {
    return s.len() + n
}

fn opt(n: i64; sep: ref String = ",") -> i64 {
    return inner(sep, n)
}

fn outer(n: i64; sep: ref String = " ") -> i64 {
    return inner(sep, n) + opt(n; sep: sep)
}

fn shout(t: ref String) {
    println(f"{t}!")
}

fn say(s: ref String) {
    shout(s)
    println(f"{s.starts_with("a")} {inner(s, 0)}")
}

fn main() {
    let a = f"ab{1}"
    say("ada")
    say(a)
    println(f"{outer(1)} {outer(2; sep: a)}")
}
"##;

#[test]
fn a_lent_parameter_or_option_handed_on_lowers_as_a_written_view() {
    let without = LENT_HANDED_ON.replace(": ref String", ": String");
    runs(
        "lent-handed-on",
        &without,
        "ada!\ntrue 3\nab1!\ntrue 3\n4 10",
    );
    assert_eq!(
        lowered(LENT_HANDED_ON, Build::default()),
        lowered(&without, Build::default())
    );
}

/// **`examples/http` without any `ref String` but on its methods' parameters
/// lowers to the same Rust** (#654): `listen(at)` hands its parameter to
/// `std::net` as it is. A method's parameter is not lent yet (ADR-288): a
/// method with a `String` parameter keeps it.
#[test]
fn the_http_example_without_ref_but_on_method_parameters_lowers_the_same() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/http/src/main.nika");
    let source = std::fs::read_to_string(&path).expect("examples/http/src/main.nika");
    let without = source
        .replace("(ref self, name: ref String)", "(ref self, name: KEPT)")
        .replace("ref String", "String")
        .replace("name: KEPT", "name: ref String");
    assert!(without.contains("name: ref String"));
    assert!(findings(&without).is_empty(), "{:#?}", findings(&without));
    assert_eq!(
        lowered(&source, Build::default()),
        lowered(&without, Build::default())
    );
}

// ---------------------------------------------------------------------------
/// **A position only literals flow into is a view of the program's own text**
/// ([ADR-282](../../../docs/specification/adr/adr-282.md) D29, #659):
/// `&'static str` below, with no lifetime and no copy - a field, a list's
/// element, a result (an all-literal `if` and `match` too) and a `let`. A
/// map's keys and values stay text of their own: a lookup may be by any text.
const LITERALS_ONLY: &str = r##"struct Row {
    name: String,
    note: String,
}

struct Team {
    names: Vec[String],
    lead: String,
}

impl Team {
    fn first(ref self) -> String {
        return self.lead
    }
}

fn label(big: bool) -> String {
    if big { "large" } else { "small" }
}

fn kind(n: i64) -> String {
    return match n {
        0 => "zero",
        1 => "one",
        else => "many",
    }
}

fn make() -> Team {
    return Team { names: ["Ada", "Grace"], lead: "Ada" }
}

fn main() {
    let mut rows = Vec()
    for i in 0..<3 {
        rows.push(Row { name: "Hamburg", note: "a <note>" })
    }
    let t = make()
    let x: String = "plain"
    println(f"{rows.len()} {rows[0].name} {rows[2].note}")
    println(f"{label(true)} {label(false)} {kind(1)}")
    println(f"{t.names.len()} {t.names[1]} {t.first()} {x}")
}
"##;

#[test]
fn a_position_only_literals_flow_into_is_static_text() {
    runs(
        "literals-only",
        LITERALS_ONLY,
        "3 Hamburg a <note>\nlarge small one\n2 Grace Ada plain",
    );
    let rust = lowered(LITERALS_ONLY, Build::default());
    assert!(!rust.contains("String::from"), "{rust}");
    for wanted in [
        "struct Row {\n    name: &'static str,\n    note: &'static str,",
        "names: Vec<&'static str>,",
        "lead: &'static str,",
        "fn label(big: bool) -> &'static str",
        "fn kind(n: i64) -> &'static str",
        "fn first(&self) -> &'static str",
        "let x: &'static str = \"plain\"",
    ] {
        assert!(rust.contains(wanted), "{wanted}\n{rust}");
    }
}

/// **`benches/template.nika` without `ref` on its fields** keeps the literals
/// as they are: no `String::from` per row, and `Row` takes no lifetime.
#[test]
fn the_template_bench_without_ref_copies_no_literal() {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benches/template.nika");
    let source = std::fs::read_to_string(&path).expect("benches/template.nika");
    let without = source.replace(": ref String", ": String");
    assert!(findings(&without).is_empty(), "{:#?}", findings(&without));
    let rust = lowered(&without, Build::default());
    assert!(!rust.contains("String::from"), "{rust}");
    assert!(
        rust.contains("pub struct Row {\n    name: &'static str,\n    note: &'static str,"),
        "{rust}"
    );
}

/// **With text of its own beside the literals, the literals are built** as
/// before (ADR-282 D4, D29): a result, a list's element and a field, one
/// `f"…"` each.
#[test]
fn literals_beside_text_of_its_own_are_built() {
    let source = r##"struct Team {
    names: Vec[String],
    lead: String,
}

fn label(big: bool, n: i64) -> String {
    if big { "large" } else { f"small {n}" }
}

fn main() {
    let n = 3
    let t = Team { names: ["Ada", f"Grace {n}"], lead: "Ada" }
    let u = Team { names: ["Bob"], lead: f"Lead {n}" }
    println(f"{label(false, n)} {t.names[1]} {t.lead} {u.lead}")
}
"##;
    runs("literals-beside-own", source, "small 3 Grace 3 Ada Lead 3");
    let rust = lowered(source, Build::default());
    for wanted in [
        "fn label(big: bool, n: i64) -> String",
        "String::from(\"large\")",
        "String::from(\"Ada\")",
        "names: Vec<String>,",
        "lead: String,",
    ] {
        assert!(rust.contains(wanted), "{wanted}\n{rust}");
    }
}

/// **A published position both kinds flow into stays text of its own** and
/// its literals are built (ADR-282 D22, D29); the ledger says `String`.
#[test]
fn a_published_position_literals_and_own_text_flow_into_stays_a_string() {
    let source = r##"pub struct Card {
    pub note: String,
}

pub fn make(n: i64) -> Card {
    if n > 1 {
        return Card { note: f"note {n}" }
    }
    return Card { note: "plain" }
}

fn main() {
    println(f"{make(2).note} {make(1).note}")
}
"##;
    runs("published-mixed", source, "note 2 plain");
    let parsed = parse_to_ast(source).expect("the source parses");
    let ledger = nikaia::contracts::Ledger::infer(&parsed).render();
    assert!(ledger.contains("\"pub note: String\""), "{ledger}");
    let rust = lowered(source, Build::default());
    assert!(rust.contains("String::from(\"plain\")"), "{rust}");
}

/// **A local list linked to a field takes the field's tier** (ADR-282 D16,
/// D29): only literals pushed, and both are static text; one `f"…"` pushed,
/// and both are text of their own, the literal built. Refused with `NK1106`
/// before.
#[test]
fn a_local_list_linked_to_a_field_takes_its_tier() {
    let source = r##"struct Team {
    names: Vec[String],
}

fn main() {
    let mut names = Vec()
    names.push("Ada")
    names.push("Grace")
    let t = Team { names: names }
    println(f"{t.names.len()} {t.names[1]}")
}
"##;
    runs("linked-literals", source, "2 Grace");
    let rust = lowered(source, Build::default());
    assert!(rust.contains("names: Vec<&'static str>,"), "{rust}");
    assert!(!rust.contains("String::from"), "{rust}");
    let mixed = source.replace("names.push(\"Grace\")", "names.push(f\"Grace {2}\")");
    runs("linked-mixed", &mixed, "2 Grace 2");
    let rust = lowered(&mixed, Build::default());
    assert!(rust.contains("names: Vec<String>,"), "{rust}");
    assert!(rust.contains("names.push(String::from(\"Ada\"))"), "{rust}");
}

/// **Text changed in place, or kept where no position says what it is, is
/// text of its own** (ADR-282 D29): `push_str`, `+=`, a `mut` parameter, a
/// name read out of a result and changed or assigned, a tuple's part and a
/// list beside text of its own all keep the literals' positions `String`.
#[test]
fn text_changed_or_kept_unseen_stays_a_string() {
    let source = r##"struct Row {
    name: String,
}

fn label() -> String {
    return "lbl"
}

fn other() -> String {
    return "oth"
}

fn fill(mut out: String) {
    out.push_str("!")
}

fn pair() -> (i64, String) {
    let r = Row { name: "Ada" }
    return (1, r.name)
}

fn main() {
    let mut s: String = ""
    s.push_str("a")
    let mut t: String = "b"
    t += "c"
    let mut u: String = "d"
    fill(u)
    let mut v = label()
    v.push_str("e")
    let mut w = other()
    w = f"w{1}"
    let r = Row { name: "Bo" }
    let mut names: Vec[String] = []
    names.push(r.name)
    names.push(f"g{2}")
    println(f"{s} {t} {u} {v} {w} {names[0]} {pair().1}")
}
"##;
    runs("changed-in-place", source, "a bc d! lble w1 Bo Ada");
}

// ---------------------------------------------------------------------------
/// **A `String` parameter handed back unchanged is lent**
/// ([ADR-282](../../../docs/specification/adr/adr-282.md) D2, D5, D12, #678):
/// a result into which only parameters flow as they came in is a view of
/// them, and so is a field of a struct literal handed back that only such
/// parameters fill. `x` is lent, not kept, and used again.
const HANDED_BACK: &str = r##"struct Pair { left: String, right: String }
fn pair(a: String, b: String) -> Pair { return Pair { left: a, right: b } }
fn longest(a: String, b: String) -> String {
    if a.len() >= b.len() { return a }
    return b
}
fn main() {
    let x = f"x{1}"
    let y = f"yy{2}"
    println(longest(x, y))
    println(pair(x, y).left)
    println(x)
}
"##;

#[test]
fn a_parameter_handed_back_unchanged_is_lent() {
    runs("handed-back", HANDED_BACK, "yy2\nx1\nx1");
    let written = HANDED_BACK
        .replace(
            "left: String, right: String",
            "left: ref String, right: ref String",
        )
        .replace("(a: String, b: String)", "(a: ref String, b: ref String)")
        .replace("-> String {", "-> ref String {");
    assert_eq!(
        lowered(HANDED_BACK, Build::default()),
        lowered(&written, Build::default()),
        "`String` lowers as `ref String` does"
    );
}

/// The same through a name bound to the parameter, beside a literal, on a
/// `ref self` method, from a `pub fn`, and through a call of one.
const HANDED_BACK_SHAPES: &str = r##"struct Doc { name: String }
impl Doc {
    fn pick(ref self, other: String) -> String { return other }
}
pub fn first(a: String, b: String) -> String { return a }
fn via_let(a: String) -> String {
    let t = a
    return t
}
fn or_none(a: String, c: bool) -> String {
    if c { return a }
    return "none"
}
fn outer(a: String, b: String) -> String { return first(b, a) }
fn main() {
    let x = f"x{1}"
    let y = f"y{2}"
    let d = Doc { name: f"d{2}" }
    println(d.pick(x))
    println(first(x, y))
    println(via_let(x))
    println(or_none(x, false))
    println(outer(x, y))
    let mut v: Vec[String] = []
    v.push(first(x, y))
    println(f"{v[0]} {x} {d.name}")
}
"##;

#[test]
fn a_parameter_handed_back_unchanged_is_lent_in_every_shape() {
    runs(
        "handed-back-shapes",
        HANDED_BACK_SHAPES,
        "x1\nx1\nx1\nnone\ny2\nx1 x1 d2",
    );
    let written = HANDED_BACK_SHAPES
        .replace(
            "other: String) -> String",
            "other: ref String) -> ref String",
        )
        .replace(
            "(a: String, b: String) -> String",
            "(a: ref String, b: ref String) -> ref String",
        )
        .replace("(a: String) -> String", "(a: ref String) -> ref String")
        .replace(
            "(a: String, c: bool) -> String",
            "(a: ref String, c: bool) -> ref String",
        );
    assert_eq!(
        lowered(HANDED_BACK_SHAPES, Build::default()),
        lowered(&written, Build::default()),
        "`String` lowers as `ref String` does"
    );
}

/// **Beside text of its own, the parameter is kept as before**: a result that
/// also hands back text of its own, a name assigned again, a `String?`, a
/// field another literal fills with text of its own, and a parameter also put
/// into an error stay `String`, and the caller hands each its argument.
#[test]
fn a_parameter_handed_back_beside_text_of_its_own_is_kept() {
    let source = r##"fn mixed(a: String, b: bool) -> String {
    if b { return a }
    return f"own{1}"
}
fn relet(a: String, c: bool) -> String {
    let mut t = a
    if c { t = f"z{1}" }
    return t
}
fn maybe(a: String?) -> String? { return a }
struct P { left: String }
fn p(a: String) -> P { return P { left: a } }
fn q() -> P { return P { left: f"own{3}" } }
enum LoadError { Missing(String) }
impl Error for LoadError {
    fn message(ref self) -> String {
        match self {
            LoadError::Missing(w) => { return f"{w} is missing" }
        }
    }
}
fn load(which: String) -> String throws {
    if which == "a" {
        throw LoadError::Missing(which)
    }
    return which
}
fn main() throws {
    println(load(f"b{1}"))
    println(mixed(f"x{1}", true))
    println(relet(f"x{1}", true))
    println(maybe(f"x{1}") ?? "")
    println(p(f"x{1}").left)
    println(q().left)
}
"##;
    runs("handed-back-kept", source, "b1\nx1\nz1\nx1\nx1\nown3");
    let rust = lowered(source, Build::default());
    for kept in [
        "fn mixed(a: String, b: bool) -> String {",
        "fn relet(a: String, c: bool) -> String {",
        "fn p(a: String) -> P {",
        "fn load(which: String) -> ",
    ] {
        assert!(rust.contains(kept), "{kept}\n{rust}");
    }
    let reused = "fn mixed(a: String, b: bool) -> String {\n    if b { return a }\n    return f\"own{1}\"\n}\nfn main() {\n    let x = f\"x{1}\"\n    println(mixed(x, true))\n    println(x)\n}\n";
    // Read again after the call, `x` goes in as a view (#716): the only text
    // `a` is handed, so `a` is one, and `x` is lent, not handed over.
    let found = findings(reused);
    assert!(found.is_empty(), "{found:#?}");
    let rust = lowered(reused, Build::default());
    assert!(rust.contains("fn mixed(a: &str, b: bool)"), "{rust}");
    assert!(rust.contains("mixed(&x, true)"), "{rust}");
}

/// **An argument made in the call lives as long as the struct of views handed
/// back** ([ADR-283](../../../docs/specification/adr/adr-283.md) D24, #678): a
/// `let` bound to `tag(f"own{1}")` keeps the text beside the call, as one
/// bound to a view result does. `rustc` refused it before (E0716).
#[test]
fn an_argument_made_in_the_call_lives_as_long_as_the_struct_handed_back() {
    let source = r##"struct Tag { s: String }
fn tag(s: String) -> Tag { return Tag { s: s } }
fn main() {
    let x = f"x{1}"
    let a = tag(x)
    let b = tag(f"own{1}")
    println(f"{a.s} {b.s} {x}")
}
"##;
    runs("struct-of-views-argument", source, "x1 own1 x1");
    let rust = lowered(source, Build::default());
    assert!(rust.contains(".put(format!(\"own{}\", 1))"), "{rust}");
}

/// **A struct literal bound to a name handed back is handed back**
/// ([ADR-282](../../../docs/specification/adr/adr-282.md) D2, D12, #678): `let
/// t = Tag { label: s }` read through its fields and then `return t` lends
/// `s` as `return Tag { label: s }` does, and lowers byte for byte as the
/// `ref String` twin, given text of its own (`x`) and a view (`w`).
const HANDED_BACK_THROUGH_A_LET: &str = r##"struct Tag { label: String }
fn tag(s: String) -> Tag {
    let t = Tag { label: s }
    println(t.label.len())
    return t
}
fn main() {
    let x = f" x{1} "
    let a = tag(x)
    let w = x.trim()
    let b = tag(w)
    println(f"{a.label}|{b.label}|{x}")
}
"##;

#[test]
fn a_struct_bound_to_a_name_handed_back_lends_its_fields() {
    runs(
        "handed-back-through-a-let",
        HANDED_BACK_THROUGH_A_LET,
        "4\n2\n x1 |x1| x1",
    );
    let written = HANDED_BACK_THROUGH_A_LET
        .replace("label: String }", "label: ref String }")
        .replace("tag(s: String)", "tag(s: ref String)");
    assert_eq!(
        lowered(HANDED_BACK_THROUGH_A_LET, Build::default()),
        lowered(&written, Build::default()),
        "`String` lowers as `ref String` does"
    );
}

/// **A parameter handed to one that only reads it, and then handed back, is
/// lent** ([ADR-282](../../../docs/specification/adr/adr-282.md) D2, D5, D12,
/// #678): `checked` lowers byte for byte as its `ref String` twin, also where
/// the reader hands it on to another reader, given text of its own (`x`) and
/// a view (`w`). One handed to a function that keeps it stays `String`.
const HANDED_ON_AND_BACK: &str = r##"fn width(a: String) -> i64 { return a.len() as i64 }
fn twice(a: String) -> i64 { return width(a) * 2 }
fn checked(a: String) -> String {
    let n = width(a)
    println(twice(a) + n)
    return a
}
fn main() {
    let x = f" x{1} "
    println(checked(x))
    let w = x.trim()
    println(checked(w))
    println(x)
}
"##;

#[test]
fn a_parameter_handed_to_a_reader_and_back_is_lent() {
    runs(
        "handed-on-and-back",
        HANDED_ON_AND_BACK,
        "12\n x1 \n6\nx1\n x1",
    );
    let written = HANDED_ON_AND_BACK.replace(
        "checked(a: String) -> String",
        "checked(a: ref String) -> ref String",
    );
    assert_eq!(
        lowered(HANDED_ON_AND_BACK, Build::default()),
        lowered(&written, Build::default()),
        "`String` lowers as `ref String` does"
    );
    let kept = r##"fn keep(a: String, mut v: Vec[String]) { v.push(a) }
fn kept(a: String, mut v: Vec[String], c: bool) -> String {
    if c {
        keep(a, v)
        return "none"
    }
    return a
}
fn main() {
    let mut v: Vec[String] = []
    println(kept(f"x{1}", v, true))
    println(kept(f"y{2}", v, false))
    println(v.len())
}
"##;
    runs("handed-to-a-keeper", kept, "none\ny2\n1");
    let rust = lowered(kept, Build::default());
    assert!(rust.contains("fn kept(a: String,"), "{rust}");
}

// ---------------------------------------------------------------------------
// What an enum's variant carries (#680)
// ---------------------------------------------------------------------------

/// A variant whose `String` payload is filled with `{given}` where `n` is not
/// zero, and with a number otherwise.
fn token_program(header: &str, payload: &str, given: &str) -> String {
    format!(
        "{header}enum Tok {{\n    Word({payload}),\n    Num(i64),\n}}\n\n\
         fn first(line: String, n: i64) -> Tok {{\n\
         \x20   if n == 0 {{\n        return Tok::Num(0)\n    }}\n\
         \x20   return Tok::Word({given})\n}}\n\n\
         fn main() {{\n\
         \x20   let text = f\"  hi {{1}} \"\n\
         \x20   for n in 0..<2 {{\n\
         \x20       match first(text, n) {{\n\
         \x20           Tok::Word(w) => println(w),\n\
         \x20           Tok::Num(k) => println(f\"{{k}}\"),\n\
         \x20       }}\n\
         \x20   }}\n}}\n"
    )
}

/// **A variant's payload only views flow into is a view** (ADR-282 D2, D12,
/// #680): `Word(String)` lowers byte for byte as `Word(ref String)`, the enum
/// taking the lifetime a struct of views takes; in a `pub enum` as well
/// (D25: what is published may be a view).
#[test]
fn a_variant_payload_only_views_flow_into_is_a_view() {
    for header in ["", "pub "] {
        let plain = token_program(header, "String", "line.trim()");
        let written = token_program(header, "ref String", "line.trim()");
        let rust = lowered(&plain, Build::default());
        assert!(rust.contains("Word(&'a str)"), "{rust}");
        assert_eq!(rust, lowered(&written, Build::default()), "{header}");
        runs("variant-view", &plain, "0\nhi 1");
    }
    // A list of views as well.
    let list = "enum Tok {\n    Words(Vec[String]),\n    Num(i64),\n}\n\n\
                fn split(line: String) -> Tok {\n\
                \x20   if line.len() == 0 {\n        return Tok::Num(0)\n    }\n\
                \x20   let mut out: Vec[String] = []\n\
                \x20   for w in line.split(\" \") {\n        out.push(w)\n    }\n\
                \x20   return Tok::Words(out)\n}\n\n\
                fn main() {\n\
                \x20   let text = f\"a b {1}\"\n\
                \x20   match split(text) {\n\
                \x20       Tok::Words(ws) => println(f\"{ws.len()}\"),\n\
                \x20       Tok::Num(n) => println(f\"{n}\"),\n\
                \x20   }\n}\n";
    let rust = lowered(list, Build::default());
    assert_eq!(
        rust,
        lowered(&list.replace("String", "ref String"), Build::default())
    );
    runs("variant-list", list, "3");
}

/// **Text of its own stays `String`, literals only are static text, both are
/// `EitherText`** (ADR-282 D29, D14, #680), each run.
#[test]
fn a_variant_payload_takes_the_tier_of_what_flows_in() {
    let owned = token_program("", "String", "f\"w{n}\"");
    let rust = lowered(&owned, Build::default());
    assert!(rust.contains("Word(String)"), "{rust}");
    runs("variant-owned", &owned, "0\nw1");

    let lasting = token_program("", "String", "\"word\"");
    let rust = lowered(&lasting, Build::default());
    assert!(rust.contains("Word(&'static str)"), "{rust}");
    runs("variant-lasting", &lasting, "0\nword");

    let mixed = token_program("", "String", "line.trim()").replace(
        "        return Tok::Num(0)",
        "        return Tok::Word(f\"w{n}\")",
    );
    let rust = lowered(&mixed, Build::default());
    assert!(
        rust.contains("Word(nikaia_std::either_text::EitherText<'a>)"),
        "{rust}"
    );
    runs("variant-mixed", &mixed, "w0\nhi 1");
}

/// **A payload of static text bound in a match arm** (ADR-282 D29, #680):
/// handed into a result beside text of its own, the payload keeps text of its
/// own, as a field read there does; beside literals only, the result is the
/// static text it holds. NK1104 before, on `n` beside `"?".clone()`.
#[test]
fn a_static_payload_bound_in_a_match_arm_follows_where_it_goes() {
    let source = r##"enum T {
    Unknown,
    Named(String),
}

fn show(t: T) -> String {
    return match t {
        T::Unknown => "?".clone(),
        T::Named(n) => n,
    }
}

fn main() {
    println(f"{show(T::Named("x"))} {show(T::Unknown)}")
}
"##;
    runs("static-payload-beside-own", source, "x ?");
    let rust = lowered(source, Build::default());
    assert!(rust.contains("Named(String)"), "{rust}");

    let lasting = source.replace("\"?\".clone()", "\"?\"");
    runs("static-payload-beside-literals", &lasting, "x ?");
    let rust = lowered(&lasting, Build::default());
    assert!(rust.contains("Named(&'static str)"), "{rust}");
}

/// **`examples/json` without `ref`** (#680): `Text(String)` lowers byte for
/// byte as `Text(ref String)` did, every parsed string a view of the input.
#[test]
fn the_json_example_lowers_as_its_ref_twin() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/json/src/main.nika"
    );
    let source = std::fs::read_to_string(path).expect("examples/json");
    assert!(source.contains("    Text(String),"), "{source}");
    let written = source.replace("    Text(String),", "    Text(ref String),");
    let rust = lowered(&source, Build::default());
    assert!(rust.contains("Text(&'a str)"), "{rust}");
    assert_eq!(rust, lowered(&written, Build::default()));
}

/// **A `String` parameter changed in place through a name bound to it owns
/// its text** (ADR-282 D29, #681): never mixed, so a view handed to it is
/// refused at the call (`NK1102`), published or not, where before it was
/// taken as `EitherText` and `rustc` refused the `push_str`. With
/// `.clone()` it runs; a caller taking `String` still hands it over
/// (`NK2105`).
#[test]
fn a_parameter_changed_through_a_name_bound_to_it_refuses_a_view() {
    let probe = r##"pub fn greet(who: String) -> String {
    let mut s = who
    s.push_str("!")
    return s
}
pub fn hello(w: ref String) -> String { return greet(w) }
fn main() {
    let w = f"w{1}"
    println(hello(w))
    println(w)
}
"##;
    for source in [probe.to_string(), probe.replace("pub fn", "fn")] {
        let found = findings(&source);
        assert!(
            found.iter().any(|f| f.code == "NK1102"
                && f.message.contains(
                    "`greet` expects `who` to be `String`, but you're passing `ref String`"
                )),
            "{found:#?}"
        );
    }
    runs(
        "changed-through-a-name",
        &probe.replace("greet(w)", "greet(w.clone())"),
        "w1!\nw1",
    );
    let found = findings(&probe.replace("hello(w: ref String)", "hello(w: String)"));
    assert!(found.iter().any(|f| f.code == "NK2105"), "{found:#?}");
}

/// **A map's `String` key is decided by what flows into it** (#659): the
/// examples that wrote `HashMap[ref String, ...]` lower byte for byte as they
/// did. In `1brc` and `access-log` a key walked out of the map whole and
/// entered again adds nothing to it, so the keys stay views of the input;
/// in `k-nucleotide` the fragments are cuts of the text's bytes, views too.
#[test]
fn the_map_examples_without_ref_on_their_keys_lower_as_their_ref_twins() {
    for example in [
        "1brc.nika",
        "access-log/src/main.nika",
        "k-nucleotide/src/main.nika",
    ] {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(example);
        let source = std::fs::read_to_string(&path).expect(example);
        assert!(source.contains("HashMap[String, "), "{example}");
        assert!(!source.contains("HashMap[ref String, "), "{example}");
        let written = source.replace("HashMap[String, ", "HashMap[ref String, ");
        assert!(
            findings(&source).is_empty(),
            "{example}: {:#?}",
            findings(&source)
        );
        let rust = lowered(&source, Build::default());
        assert!(!rust.contains("EitherText"), "{example}: {rust}");
        assert_eq!(rust, lowered(&written, Build::default()), "{example}");
    }
}

/// **A map only literals go into is keyed by static text** (#659, ADR-282
/// D29), and text of its own is looked up in it as the `str` it holds: in
/// the brackets, with `get` and with `contains_key`.
#[test]
fn a_literal_keyed_map_is_static_text_looked_up_by_any_text() {
    let probe = r##"use std::collections

fn main() {
    let mut m: collections::HashMap[String, i64] = collections::HashMap()
    m.insert("a", 1)
    m.insert("b", 2)
    let k = f"a{""}"
    println(f"{m.len()} {m["a"] ?? 0} {m[k] ?? 0} {m.get(k) ?? 0} {m.contains_key(k)}")
}
"##;
    let rust = lowered(probe, Build::default());
    assert!(rust.contains("TrustedMap<&'static str, i64>"), "{rust}");
    assert!(
        rust.contains("m.get(nikaia_std::index::as_text(&k))"),
        "{rust}"
    );
    runs("literal-keyed-map", probe, "2 1 1 1 true");
}

/// **Views of text as keys and as values, looked up by text of its own**
/// (#659): a local map and a struct's field, the field's map walked whole
/// and put back, and a key cut out of the text's bytes.
#[test]
fn a_map_of_views_is_looked_up_by_text_of_its_own() {
    let local = r##"use std::collections

fn main() {
    let s = f"x y x{1}"
    let mut m: collections::HashMap[String, String] = collections::HashMap()
    for w in s.split(" ") {
        m.insert(w, w)
    }
    let k = f"y"
    println(f"{m.len()} {m[k] ?? "-"} {m.get(k) ?? "-"} {m.contains_key(k)} {m["x1"] ?? "-"}")
}
"##;
    let rust = lowered(local, Build::default());
    assert!(rust.contains("TrustedMap<&str, &str>"), "{rust}");
    runs("map-of-views", local, "3 y y true x1");
    let field = r##"use std::collections

struct Reading {
    name: String,
    t: i64,
}

struct Seen {
    m: collections::HashMap[String, i64],
}

impl Seen {
    fn record(ref mut self, r: Reading) {
        self.m.insert(r.name, r.t)
    }

    fn merge(ref mut self, other: Seen) {
        for (name, n) in other.m.drain() {
            self.m.insert(name, n)
        }
    }
}

fn cut(dna: String) -> collections::HashMap[String, i64] {
    let mut counts: collections::HashMap[String, i64] = collections::HashMap()
    for i in 0..<dna.len() {
        let one = dna.bytes[i..<i + 1].text() catch {
            continue
        }
        counts.insert(one, i)
    }
    return counts
}

fn main() {
    let s = f"a b a{1}"
    let mut x = Seen { m: collections::HashMap() }
    let mut y = Seen { m: collections::HashMap() }
    for w in s.split(" ") {
        y.record(Reading { name: w, t: 1 })
    }
    x.merge(y)
    let k = f"b"
    let c = cut(s)
    println(f"{x.m.len()} {x.m[k] ?? 0} {c.len()} {c[k] ?? 0}")
}
"##;
    let rust = lowered(field, Build::default());
    assert!(
        rust.contains("m: collections::TrustedMap<&'a str, i64>"),
        "{rust}"
    );
    assert!(
        rust.contains("fn cut(dna: &str) -> collections::TrustedMap<&str, i64>"),
        "{rust}"
    );
    assert!(!rust.contains("EitherText"), "{rust}");
    runs("map-field-of-views", field, "3 1 4 2");
}

/// **The names a tuple `let` binds from `split_once`** are views of the text
/// cut, past `?? continue`, `?? break` and `?? return` as without a `??`, and
/// so are the names a bare tuple pattern binds in a `match` on it (#687).
#[test]
fn split_once_parts_are_views_of_what_was_cut() {
    let pushed = |jump: &str| {
        format!(
            r#"fn main() {{
    let s = f"x=1 y=2"
    let mut v: Vec[String] = []
    for w in s.split(" ") {{
        let (a, _b) = w.split_once("=") ?? {jump}
        v.push(a)
    }}
    println(f"{{v.len()}}")
}}
"#
        )
    };
    runs("split-once-continue", &pushed("continue"), "2");
    runs("split-once-return", &pushed("return"), "2");
    runs("split-once-break", &pushed("break"), "2");
    let inserted = r#"use std::collections
fn main() {
    let s = f"x=1 y=2 x=3"
    let mut m: collections::HashMap[String, String] = collections::HashMap()
    for w in s.split(" ") {
        let (a, b) = w.split_once("=") ?? continue
        m.insert(a, b)
    }
    println(f"{m.len()}")
}
"#;
    runs("split-once-insert", inserted, "2");
    let matched = r#"fn main() {
    let s = f"x=1 y=2 z"
    let mut v: Vec[String] = []
    for w in s.split(" ") {
        match w.split_once("=") {
            (a, _b) => v.push(a),
            null => {},
        }
    }
    println(f"{v.len()}")
}
"#;
    runs("split-once-match", matched, "2");
    // Cut from text owned by one turn of the loop: the list cannot hold a
    // view of it, so it holds text of its own.
    let short_lived = r#"use std::collections
fn main() {
    let mut v: Vec[String] = []
    let mut m: collections::HashMap[String, String] = collections::HashMap()
    for i in 0..2 {
        let line = f"k{i}=v{i}"
        let (a, b) = line.split_once("=") ?? continue
        v.push(a)
        m.insert(a, b)
    }
    println(f"{v.len()} {v[2]} {m.len()}")
}
"#;
    runs("split-once-owned-line", short_lived, "3 k2 3");
}

/// **A parameter is not lent where a caller's text would die before the
/// result viewing it** ([ADR-282](../../../docs/specification/adr/adr-282.md)
/// D12, #702): `keep` hands `tag(v)`'s result into `b`, which outlives `v`,
/// so `tag` keeps `s` and `Tag.label` holds text of its own. `rustc` refused
/// it before (`v` does not live long enough).
const KEPT_INTO_A_CONTAINER: &str = r##"struct Tag {
    label: String,
}

struct Box {
    items: Vec[Tag],
}

fn tag(s: String) -> Tag {
    return Tag { label: s }
}

fn keep(v: String, mut b: Box) {
    let a = tag(v)
    b.items.push(a)
}

fn main() {
    let mut b = Box { items: [] }
    keep(f"x{1}", b)
    println(b.items.len())
}
"##;

#[test]
fn a_struct_viewing_a_kept_parameter_pushed_into_a_container_owns_its_text() {
    runs("kept-into-a-container", KEPT_INTO_A_CONTAINER, "1");
    let rust = lowered(KEPT_INTO_A_CONTAINER, Build::default());
    assert!(rust.contains("label: String,"), "{rust}");
    assert!(rust.contains("fn tag(s: String) -> Tag {"), "{rust}");
    // Pushed straight in, from a local, from a local handed back, and from a
    // local of one turn of a loop into a list outside it.
    let straight = KEPT_INTO_A_CONTAINER.replace(
        "    let a = tag(v)\n    b.items.push(a)",
        "    b.items.push(tag(v))",
    );
    runs("kept-into-a-container-straight", &straight, "1");
    let local = KEPT_INTO_A_CONTAINER
        .replace(
            "fn keep(v: String, mut b: Box) {\n    let a",
            "fn keep(mut b: Box) {\n    let v = f\"x{1}\"\n    let a",
        )
        .replace("keep(f\"x{1}\", b)", "keep(b)");
    runs("kept-into-a-container-local", &local, "1");
    let handed_back = KEPT_INTO_A_CONTAINER
        .replace(
            "fn keep(v: String, mut b: Box) {\n    let a = tag(v)\n    b.items.push(a)\n}",
            "fn make() -> Tag {\n    let v = f\"x{1}\"\n    return tag(v)\n}",
        )
        .replace("    keep(f\"x{1}\", b)\n", "    b.items.push(make())\n");
    runs("kept-into-a-container-handed-back", &handed_back, "1");
    let looped = KEPT_INTO_A_CONTAINER.replace(
        "    keep(f\"x{1}\", b)\n",
        "    for i in 0..<3 {\n        let v = f\"x{i}\"\n        b.items.push(tag(v))\n    }\n",
    );
    runs("kept-into-a-container-looped", &looped, "3");
}

/// The same `tag` where `keep` only reads what it made: `tag` still lends `s`,
/// and lowers byte for byte as its `ref String` twin.
#[test]
fn a_struct_viewing_a_parameter_only_read_still_lends_it() {
    let read = KEPT_INTO_A_CONTAINER.replace("    b.items.push(a)", "    println(a.label)");
    runs("kept-only-read", &read, "x1\n0");
    let written = read
        .replace("    label: String,", "    label: ref String,")
        .replace("tag(s: String)", "tag(s: ref String)");
    assert_eq!(
        lowered(&read, Build::default()),
        lowered(&written, Build::default()),
        "`String` lowers as `ref String` does"
    );
}

/// **A program method's result views its arguments as a function's does**
/// (#702): `keep` pushes `m.make(v)` into `b`, which outlives `v`, so `make`
/// keeps `s` and `Tag.label` holds text of its own. `rustc` refused it before
/// (`v` does not live long enough).
#[test]
fn a_method_result_viewing_a_kept_parameter_pushed_into_a_container_owns_its_text() {
    let method = KEPT_INTO_A_CONTAINER
        .replace(
            "fn tag(s: String) -> Tag {\n    return Tag { label: s }\n}\n",
            "struct Maker {\n    n: i64,\n}\n\nimpl Maker {\n    fn make(ref self, s: String) -> Tag {\n        return Tag { label: s }\n    }\n}\n",
        )
        .replace(
            "fn keep(v: String, mut b: Box) {\n    let a = tag(v)\n    b.items.push(a)\n}",
            "fn keep(m: Maker, v: String, mut b: Box) {\n    b.items.push(m.make(v))\n}",
        )
        .replace(
            "    keep(f\"x{1}\", b)\n    println(b.items.len())",
            "    let m = Maker { n: 2 }\n    keep(m, f\"x{1}\", b)\n    println(f\"{b.items.len()} {b.items[0].label} {m.n}\")",
        );
    assert!(
        method.contains("m.make(v)") && method.contains("keep(m, "),
        "{method}"
    );
    runs("method-kept-into-a-container", &method, "1 x1 2");
    let rust = lowered(&method, Build::default());
    assert!(rust.contains("label: String,"), "{rust}");
}

/// **An arm that is a name still read later is a view of it** (ADR-282 D7,
/// D13, #712): read in the loop's next turn and after it, `s` is lent to an
/// `EitherText` beside the arm that makes text of its own, and not moved. An
/// arm naming a value nothing reads afterwards still moves it, and the choice
/// stays a plain `String`. That both run is `tests/accepted/712_*.nika`.
#[test]
fn an_arm_naming_text_read_later_is_a_view_of_it() {
    let later = r##"fn main() {
    let s = f"x{1}"
    for i in 0..<2 {
        let c = if i == 1 { s } else { f"y{i}" }
        println(c)
    }
    println(s)
}
"##;
    let rust = lowered(later, Build::default());
    assert!(
        rust.contains("nikaia_std::either_text::either(&s)"),
        "{rust}"
    );
    let moved = r##"fn main() {
    let s = f"x{1}"
    let c = if s.len() == 1 { s } else { f"y{2}" }
    println(c)
}
"##;
    let rust = lowered(moved, Build::default());
    assert!(
        rust.contains("let c = if s.len() == 1 { s } else { format!(\"y{}\", 2) };")
            && !rust.contains("either"),
        "{rust}"
    );
}

/// **A name handed to a parameter that keeps its text, and still read later,
/// goes in as a view** (ADR-282 D7, D13, #716): `t`, read again in the loop's
/// next turn, is lent to `longest`'s `a`, which takes both kinds beside the
/// text of its own the first call hands it. Where nothing reads `t` again, it
/// is handed over as before and `a` stays a `String`. That the first runs is
/// `tests/accepted/716_name_kept_in_loop.nika`.
#[test]
fn a_name_read_later_handed_to_a_keeping_parameter_is_a_view() {
    let longest = r##"fn longest(a: String, b: String) -> String {
    if a.len() >= b.len() {
        return a
    }
    return b
}
"##;
    let later = format!(
        "{longest}{}",
        r##"
fn main() {
    let t = f"t{1}"
    for n in 0..<3 {
        let r = longest(if n == 0 { "lit" } else { f"n{n}" }, "ab")
        println(r)
        let q = longest(t, "abcdefg")
        println(q)
    }
}
"##
    );
    let rust = lowered(&later, Build::default());
    assert!(
        rust.contains("fn longest<'r>(a: nikaia_std::either_text::EitherText<'r>"),
        "{rust}"
    );
    assert!(
        rust.contains("longest(nikaia_std::either_text::either(&t), \"abcdefg\")"),
        "{rust}"
    );
    let moved = format!(
        "{longest}{}",
        r##"
fn main() {
    let t = f"t{1}"
    for n in 0..<3 {
        let r = longest(if n == 0 { "lit" } else { f"n{n}" }, "ab")
        println(r)
    }
    let q = longest(t, "abcdefg")
    println(q)
}
"##
    );
    let rust = lowered(&moved, Build::default());
    assert!(
        rust.contains("fn longest(a: String, b: &'static str)"),
        "{rust}"
    );
    assert!(rust.contains("let q = longest(t, \"abcdefg\");"), "{rust}");
}
