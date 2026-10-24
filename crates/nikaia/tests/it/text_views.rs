//! **A `String` that only views reach is a view at every position**
//! ([ADR-282](../../../docs/specification/adr/adr-282.md) D2, #654 step 0):
//! each program here, written with `String`, lowers to the same Rust as the
//! same program written with `ref String`.

use crate::common;

/// Lowers `source` as `name.nika` and hands back the Rust, without the line
/// table, which names the file.
fn lowered(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    let file = dir.join(format!("{name}.nika"));
    std::fs::write(&file, source).expect("the source");
    let ran = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("lower")
        .arg(format!("{name}.nika"))
        .output()
        .expect("the nikaia binary runs");
    let rust = std::fs::read_to_string(dir.join(format!("{name}.rs"))).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        ran.status.success(),
        "{name}:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    rust.lines()
        .filter(|line| !line.contains(".nika\""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Runs `source` and hands back what it printed.
fn ran(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let ran = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    String::from_utf8_lossy(&ran.stdout).into_owned()
}

/// The program with `ref String` in the places listed, and without.
fn same_rust(name: &str, written: &str, plain: &str) {
    let with_ref = lowered(&format!("{name}_ref"), written);
    let without = lowered(&format!("{name}_plain"), plain);
    assert_eq!(with_ref, without, "{name}: the Rust differs");
}

/// A `String` parameter used as a key into a map keyed by views is lent: a
/// `get` of a `std` map is the one entry by that name with a key, so the
/// `self`-only `get`s no longer read as keeping it (was an owned `String`, and
/// `rustc` refused the `get`: `&str: Borrow<String>`).
#[test]
fn a_string_parameter_used_as_a_map_key_is_lent() {
    let program = r#"use std::collections

fn count(dna: ref String, k: i64) -> collections::HashMap[ref String, i64] {
    let mut counts: collections::HashMap[ref String, i64] = collections::HashMap()
    for i in 0..<dna.len() - k + 1 {
        let fragment = dna.bytes[i..<i + k].text() catch {
            continue
        }
        counts.insert(fragment, 1)
    }
    return counts
}

fn report(dna: ref String, fragment: TEXT) {
    let counts = count(dna, fragment.len())
    let n = counts.get(fragment) ?? 0
    println(f"{n}\t{fragment}")
}

fn main() {
    let d = "GGTAGGT".clone()
    report(d, "GGT")
}
"#;
    let plain = program.replace("TEXT", "String");
    let rust = lowered("map_key_parameter", &plain);
    assert!(rust.contains("fragment: &str"), "{rust}");
    assert_eq!(ran("map_key_parameter_run", &plain), "1\tGGT\n");
}

/// A rule's result and a field its action fills, reached only by what the
/// parse captures, are views (were `NK1104` and `NK1106`).
#[test]
fn a_rule_result_and_a_field_filled_by_an_action_are_views() {
    let program = r#"grammar Hits {
    rule WS = "" { }
    entry rule file -> Vec[Hit] = hits:hit* { hits }
    rule hit -> Hit = p:WORD "\n" { Hit { path: p } }
    rule WORD -> TEXT = s:until("\n") { s }
}

pub struct Hit {
    path: TEXT,
}

fn main() {
    let text = "a\nb\n".clone()
    let hits = Hits::file(text) catch { return }
    for h in hits {
        println(h.path)
    }
}
"#;
    same_rust(
        "rule_result",
        &program.replace("TEXT", "ref String"),
        &program.replace("TEXT", "String"),
    );
}

/// A parameter a parse is given, whose result views it, is lent as the view
/// the result points into (was `NK2302`).
#[test]
fn a_parameter_a_parse_views_is_lent() {
    let program = r#"grammar Stock {
    rule FIELD -> TEXT = s:until(";" | frame_end) { s }
    rule COUNT -> i64 = n:dec[i64](digit+) { n }
    @frame(boundary: "\n")
    rule ENTRY -> Entry =
        category:FIELD ";" item:FIELD ";" count:COUNT frame_end
        { Entry { category, item, count } }
    entry rule file -> Vec[Entry] = entries:ENTRY* { entries }
}

pub struct Entry {
    pub category: TEXT,
    pub item: TEXT,
    pub count: int,
}

pub fn read(data: TEXT) -> Vec[Entry] throws {
    return Stock::file(data)
}

fn main() {
    let text = "fruit;apple;3\n".clone()
    let entries = read(text) catch { return }
    for e in entries {
        println(f"{e.category} {e.item} {e.count}")
    }
}
"#;
    same_rust(
        "parsed_parameter",
        &program.replace("TEXT", "ref String"),
        &program.replace("TEXT", "String"),
    );
}

/// A map's key filled only by views - a part of `split_at`, handed to
/// `entry` - is a view (was `TrustedMap<String>`, a copy per line).
#[test]
fn a_map_key_filled_by_views_is_a_view() {
    let program = r#"use std::collections

fn main() {
    let data = "a;1\nb;2\na;3\n".clone()
    let mut seen: collections::HashMap[TEXT, i64] = collections::HashMap()
    for line in data.lines() {
        let at = line.find(";") ?? 0
        let (name, _) = line.split_at(at)
        seen.entry(name).and_modify fn(n) { n += 1 }.or_insert(1)
    }
    println(f"{seen.len()}")
}
"#;
    same_rust(
        "split_key",
        &program.replace("TEXT", "ref String"),
        &program.replace("TEXT", "String"),
    );
}

/// The ledger says `borrowed` for a `String` parameter the body only reads,
/// as it does for one written `ref String`: both are a `&str` below.
#[test]
fn a_lent_string_parameter_is_borrowed_in_the_ledger() {
    let views = |source: &str| {
        let parsed = nikaia::parser::parse_to_ast(source).expect("the source parses");
        let ledger = common::infer(&parsed);
        let contract = ledger.functions.get("report").expect("an entry");
        contract
            .views
            .iter()
            .map(|held| format!("{}: {}", held.position, held.state.as_str()))
            .collect::<Vec<_>>()
    };
    let program = "fn report(dna: TEXT, k: i64) -> i64 {\n    return dna.len() + k\n}\n\n\
                   fn main() {\n    let d = \"GGT\".clone()\n    println(f\"{report(d, 1)}\")\n}\n";
    let written = views(&program.replace("TEXT", "ref String"));
    assert_eq!(written, vec!["dna: borrowed".to_string()]);
    assert_eq!(views(&program.replace("TEXT", "String")), written);
}

/// **An option is lent as a parameter is** (#654 step 0 c1): a `String`
/// option the body only reads is the `&str` one written `ref String` is - at
/// the declaration, for its default, for a literal and for an owned value
/// passed to it.
#[test]
fn a_string_option_the_body_only_reads_is_lent() {
    let program = "fn join(words: ref Vec[String]; sep: TEXT = \" \") -> String {\n\
                   \x20   let mut out = \"\"\n\
                   \x20   for w in words {\n\
                   \x20       if out.len() > 0 {\n\
                   \x20           out = f\"{out}{sep}\"\n\
                   \x20       }\n\
                   \x20       out = f\"{out}{w}\"\n\
                   \x20   }\n\
                   \x20   return out\n\
                   }\n\n\
                   fn main() {\n\
                   \x20   let ws = [\"a\".clone(), \"b\".clone()]\n\
                   \x20   let dash = \"-\".clone()\n\
                   \x20   println(join(ws))\n\
                   \x20   println(join(ws; sep: \"+\"))\n\
                   \x20   println(join(ws; sep: dash))\n\
                   \x20   println(dash)\n\
                   }\n";
    let plain = program.replace("TEXT", "String");
    same_rust(
        "string_option",
        &program.replace("TEXT", "ref String"),
        &plain,
    );
    assert_eq!(ran("string_option", &plain), "a b\na+b\na-b\n-\n");
}

/// **An option the body keeps is text of its own, and only then in the
/// ledger's `keeps`**: one stored is kept, one only read is not listed, and a
/// parameter handed to a kept option is kept too (the fixpoint reads an
/// option as the position after the parameters).
#[test]
fn a_kept_option_is_owned_and_listed_in_keeps() {
    let source = "struct Holder {\n    name: String,\n}\n\n\
                  fn store(label: String = \"kept\") -> Holder {\n    return Holder { name: label }\n}\n\n\
                  fn read(n: i64; sep: String = \",\") -> String {\n    return f\"{n}{sep}\"\n}\n\n\
                  fn hands(text: String) -> Holder {\n    return store(label: text)\n}\n\n\
                  fn main() {\n\
                  \x20   println(store().name)\n\
                  \x20   println(read(1; sep: \";\"))\n\
                  \x20   println(hands(\"t\".clone()).name)\n\
                  }\n";
    let parsed = nikaia::parser::parse_to_ast(source).expect("the source parses");
    let ledger = common::infer(&parsed);
    let keeps = |name: &str| ledger.functions.get(name).expect("an entry").keeps.clone();
    assert_eq!(keeps("store"), vec!["label".to_string()]);
    assert!(keeps("read").is_empty(), "{:?}", keeps("read"));
    assert_eq!(keeps("hands"), vec!["text".to_string()]);
    let rust = lowered("kept_option", source);
    assert!(rust.contains("fn store(label: String)"), "{rust}");
    assert!(rust.contains("fn read(n: i64, sep: &str)"), "{rust}");
    assert!(rust.contains("fn hands(text: String)"), "{rust}");
    assert_eq!(ran("kept_option", source), "kept\n1;\nt\n");
}

/// The `NK1261` warnings of `source`, each message.
fn redundant_refs(source: &str) -> Vec<String> {
    use nikaia::contracts::LedgerOps;
    let parsed = nikaia::parser::parse_to_ast(source).expect("the source parses");
    let own = common::infer(&parsed);
    let library =
        nikaia::contracts::Ledger::parse(nikaia::contracts::STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .filter(|f| f.code == "NK1261")
        .inspect(|f| assert_eq!(f.severity, nikaia::check::Severity::Warning))
        .map(|f| f.message)
        .collect()
}

/// **`ref String` inside Nikaia is the warning `NK1261`** (ADR-282 D2, #654
/// step 1): on a parameter, an option, a field, a result and a `let`, each
/// named in the message, and the program still runs as written.
#[test]
fn ref_string_outside_the_c_boundary_is_a_warning() {
    let source = "struct Named {\n    name: ref String?,\n}\n\n\
                  fn join(a: ref String; sep: ref String = \",\") -> String {\n\
                  \x20   return f\"{a}{sep}\"\n\
                  }\n\n\
                  fn first(a: ref String) -> ref String {\n\
                  \x20   let b: ref String = a\n\
                  \x20   return b\n\
                  }\n\n\
                  fn main() {\n\
                  \x20   let n = Named { name: null }\n\
                  \x20   let j = join(\"x\")\n\
                  \x20   println(f\"{first(j)}{n.name ?? \"-\"}\")\n\
                  }\n";
    let found = redundant_refs(source);
    for (position, written) in [
        ("field", "ref String?"),
        ("parameter", "ref String"),
        ("result", "ref String"),
        ("`let`", "ref String"),
    ] {
        let wanted = format!("a {position} written `{written}`");
        assert!(
            found.iter().any(|m| m.contains(&wanted)),
            "{wanted}: {found:#?}"
        );
    }
    // The two parameters and the option.
    assert_eq!(
        found.iter().filter(|m| m.contains("a parameter")).count(),
        3,
        "{found:#?}"
    );
    assert_eq!(ran("redundant_ref", source), "x,-\n");
}

/// **Plain `String` that the tiers make a view is no warning** (#654 step 1):
/// a field, a `let`, a result and a method's result written `String` are
/// views where only literals flow into them, and the source wrote no `ref`.
#[test]
fn string_the_tiers_made_a_view_is_no_warning() {
    let source = "struct User {\n    name: String,\n}\n\n\
                  impl User {\n    fn title(ref self) -> String {\n        return \"user\"\n    }\n}\n\n\
                  fn label() -> String {\n    return \"label\"\n}\n\n\
                  fn main() {\n\
                  \x20   let nobody: String = \"nobody\"\n\
                  \x20   let u = User { name: \"ada\" }\n\
                  \x20   println(f\"{nobody} {u.name} {u.title()} {label()}\")\n\
                  }\n";
    let views = nikaia::parser::parse_to_ast(source)
        .expect("the source parses")
        .text_views;
    for wanted in ["field User name", "result label", "result User::title"] {
        assert!(views.contains(wanted), "{wanted}: {views:?}");
    }
    assert!(views.iter().any(|v| v.starts_with("let ")), "{views:?}");
    assert_eq!(redundant_refs(source), Vec::<String>::new());
    assert_eq!(ran("tiered_views", source), "nobody ada user label\n");
}

/// **Not at the C boundary**, where `ref String` is the pointer C expects, nor
/// inside a type argument or on plain `String`.
#[test]
fn ref_string_at_the_c_boundary_is_no_warning() {
    let source = "extern \"C\" {\n    fn puts(s: ref String) -> i32\n}\n\n\
                  pub extern fn greet(name: ref String) -> i64 sync {\n    return name.len()\n}\n\n\
                  fn count(names: Vec[ref String], name: String) -> i64 {\n    return names.len() + name.len()\n}\n\n\
                  fn main() {\n    println(f\"{count([\"a\"], \"b\")}\")\n}\n";
    assert_eq!(redundant_refs(source), Vec::<String>::new());
}

/// **A variant's payload is a position too** (#680): written `ref String` it
/// is the warning `NK1261`; written `String` and made a view by the tiers, it
/// is none.
#[test]
fn ref_string_in_a_variant_payload_is_a_warning() {
    let written = "enum Tok {\n    Word(ref String),\n    Num(i64),\n}\n\n\
                   fn first(line: String) -> Tok {\n\
                   \x20   if line.len() == 0 {\n        return Tok::Num(0)\n    }\n\
                   \x20   return Tok::Word(line.trim())\n}\n\n\
                   fn main() {\n\
                   \x20   match first(f\" hi {1} \") {\n\
                   \x20       Tok::Word(w) => println(w),\n\
                   \x20       Tok::Num(n) => println(f\"{n}\"),\n\
                   \x20   }\n}\n";
    let found = redundant_refs(written);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].contains("a payload written `ref String`"),
        "{found:#?}"
    );
    // As long as the written one, so that every byte offset agrees.
    let plain = written.replace("Word(ref String)", "Word(    String)");
    let views = nikaia::parser::parse_to_ast(&plain)
        .expect("the source parses")
        .text_views;
    assert!(views.contains("variant Tok Word 0"), "{views:?}");
    assert_eq!(redundant_refs(&plain), Vec::<String>::new());
    same_rust("variant_payload", written, &plain);
    assert_eq!(ran("variant_payload", &plain), "hi 1\n");
}

/// **Text ordered against text** (#733): a view beside an owned text lowered to
/// `&str < String`, which `rustc` refused. All four orderings, both ways round.
#[test]
fn a_view_orders_against_owned_text() {
    let source = r#"fn less(a: String, b: String) -> bool {
    let owned = b.clone() + "x"
    a < owned
}

fn main() {
    let s = "abc"
    let t = "a".clone() + "b"
    print(f"{less(s, "abd")} {s.trim() >= t} {t > s} {t <= s.trim()}")
}
"#;
    assert_eq!(ran("view_orders_owned", source), "true true false true");
}
