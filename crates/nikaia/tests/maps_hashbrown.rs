//! **A map is `hashbrown`'s table, keyed or trusted** (#420, #434): both kinds
//! of `HashMap` lower to `hashbrown::HashMap` with the hasher each had before,
//! and a program using the whole of what the language offers on a map compiles
//! and prints the same under either.

mod common;

use nikaia::contracts::Provenance;
use nikaia::emit::{Build, emit_program_with_trust};
use nikaia::parser::parse_to_ast;

/// The entry chain on a new and on a found key, with a lambda that captures,
/// `insert` over a key that is there, an index read and write, iteration whose
/// output does not depend on the order, a copy, a map in a struct, `drain`, and
/// a map whose type nothing writes.
const PROGRAM: &str = "use std::collections\n\
     \n\
     struct Tally {\n\
     \x20   counts: collections::HashMap[String, i64],\n\
     }\n\
     \n\
     fn total(m: ref collections::HashMap[String, i64]) -> i64 {\n\
     \x20   let mut sum = 0\n\
     \x20   for (_, v) in m {\n\
     \x20       sum = sum + v\n\
     \x20   }\n\
     \x20   return sum\n\
     }\n\
     \n\
     fn main() {\n\
     \x20   let mut m: collections::HashMap[String, i64] = collections::HashMap()\n\
     \x20   let words = [\"a\", \"b\", \"a\", \"c\", \"a\"]\n\
     \x20   let bonus = 10\n\
     \x20   for w in words {\n\
     \x20       m.entry(w.clone()).and_modify(fn(mut c) { c += 1 }).or_insert_with(fn() { bonus })\n\
     \x20   }\n\
     \x20   m.entry(\"d\").or_insert(3)\n\
     \x20   m.insert(\"z\", 5)\n\
     \x20   let old = m.insert(\"z\", 6)\n\
     \x20   println(f\"{old ?? 0} {m.len()} {m.contains_key(\"a\")} {m.get(\"a\") ?? 0} {m[\"b\"] ?? 0} {m.get(\"q\") ?? -1}\")\n\
     \x20   m[\"c\"] = 42\n\
     \x20   let mut keys: Vec[String] = []\n\
     \x20   for k in m.keys() { keys.push(k.clone()) }\n\
     \x20   keys.sort()\n\
     \x20   let text = keys.iter().join(\", \")\n\
     \x20   println(f\"{text}\")\n\
     \x20   let mut vsum = 0\n\
     \x20   for v in m.values() { vsum = vsum + v }\n\
     \x20   let copy = m.clone()\n\
     \x20   println(f\"{vsum} {total(copy)} {m.is_empty()}\")\n\
     \x20   let t = Tally { counts: m }\n\
     \x20   let mut n = 0\n\
     \x20   for (k, v) in t.counts.drain() { n = n + v + k.len() }\n\
     \x20   let mut plain = collections::HashMap()\n\
     \x20   plain.insert(n, 1)\n\
     \x20   println(f\"{n} {plain.len()}\")\n\
     }\n";

const EXPECTED: &str = "5 5 true 12 10 -1\na, b, c, d, z\n73 73 false\n78 1\n";

fn compiled_and_run(provenance: Provenance, purpose: &str) -> (String, String) {
    let parsed = parse_to_ast(PROGRAM).expect("the source parses");
    let rust = emit_program_with_trust(&parsed, Build::default(), provenance)
        .expect("it lowers")
        .rust;
    let dir = common::scratch_dir(purpose);
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    assert!(
        compiled.status.success(),
        "{}\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let ran = std::process::Command::new(&binary)
        .output()
        .expect("the program runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    (String::from_utf8_lossy(&ran.stdout).into_owned(), rust)
}

#[test]
fn a_trusted_map_is_hashbrowns_table_with_the_fast_hash() {
    let (out, rust) = compiled_and_run(Provenance::Trusted, "maps-hashbrown-trusted");
    assert_eq!(out, EXPECTED, "{rust}");
    assert!(
        rust.contains("collections::TrustedMap::default()"),
        "{rust}"
    );
}

#[test]
fn a_keyed_map_is_hashbrowns_table_with_the_keyed_hash() {
    let (out, rust) = compiled_and_run(Provenance::Untrusted, "maps-hashbrown-keyed");
    assert_eq!(out, EXPECTED, "{rust}");
    // `hashbrown` writes `new` for no hasher this compiler names: the keyed
    // map is made with `default`, and naming its two type arguments is what
    // lets the alias fill the hasher in where no type is written.
    assert!(
        rust.contains("collections::HashMap::<_, _>::default()"),
        "{rust}"
    );
    assert!(!rust.contains("HashMap::new()"), "{rust}");
    assert!(!rust.contains("TrustedMap"), "{rust}");
}

/// The types are the ones the program wrote: `nikaia_std`'s `HashMap` is
/// `hashbrown`'s, with the keyed hasher by default and the fast one in
/// `TrustedMap`.
#[test]
fn the_maps_are_hashbrowns() {
    let keyed = std::any::type_name::<nikaia_std::collections::HashMap<String, i64>>();
    assert!(keyed.starts_with("hashbrown::"), "{keyed}");
    assert!(keyed.ends_with("RandomState>"), "{keyed}");
    let fast = std::any::type_name::<nikaia_std::collections::TrustedMap<String, i64>>();
    assert!(fast.starts_with("hashbrown::"), "{fast}");
    assert!(fast.contains("FxHasher"), "{fast}");
}

/// What `rustc` says about the table is said about the map the program wrote:
/// no crate, no hasher.
#[test]
fn rustcs_words_for_the_table_are_the_programs() {
    use nikaia_std::tools::rustc_words::in_this_language;
    assert_eq!(
        in_this_language(
            "expected `hashbrown::map::HashMap<String, i64, BuildHasherDefault<FxHasher>>`, found `hashbrown::HashMap<String, i64, RandomState>`"
        ),
        "expected `HashMap<String, i64>`, found `HashMap<String, i64>`"
    );
    assert_eq!(
        in_this_language("`hashbrown::hash_map::Entry<'_, String, i64, std::hash::RandomState>`"),
        "`Entry<'_, String, i64>`"
    );
    // A map whose types nothing pins: the hasher is the keyed one's default,
    // and the message names the two the program left open.
    assert_eq!(
        in_this_language("type annotations needed for `hashbrown::map::HashMap<_, _, _>`"),
        "type annotations needed for `HashMap<_, _>`"
    );
}
