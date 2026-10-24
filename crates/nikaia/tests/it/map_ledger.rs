//! **The ledger key `maps`** (ADR-010 D9, Part III 13.5, #581 step 6): for a
//! `pub` function with a map in its signature, which of those maps are one
//! map and what its body puts into each - `from(p, ...)` (`from()` for
//! none) or `keyed`. It is a permission key (a wrong `from(..)` would let a caller give
//! the fixed seed to keys an adversary chose), so the tests here are about
//! the answer being the cautious one wherever the body is not simply read.

use crate::common;
use nikaia::contracts::LedgerOps;
use std::path::Path;

/// The ledger of a package that is `source` alone.
fn ledger_of(purpose: &str, source: &str) -> (std::path::PathBuf, nikaia::contracts::Ledger) {
    let dir = common::scratch_dir(&format!("map-ledger-{purpose}"));
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(
        dir.join("nikaia.toml"),
        "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");
    std::fs::write(dir.join("src/main.nika"), source).expect("a source");
    let program = nikaia::comptime_run::defaults_compiled_in(Some(&common::workshop()), || {
        let manifest = nikaia::manifest::Manifest::read(&dir.join("nikaia.toml"))?;
        let dependencies = nikaia::project::packages_of(&manifest, &dir, false)?;
        nikaia::modules::Program::read_with(&dir.join("src/main.nika"), &dependencies)
    })
    .expect("the package reads");
    (dir, program.contracts)
}

fn maps_of(ledger: &nikaia::contracts::Ledger, function: &str) -> Vec<String> {
    ledger
        .functions
        .get(function)
        .unwrap_or_else(|| panic!("no `{function}` in the ledger"))
        .maps
        .clone()
}

fn done(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

const TALLY: &str = "use std::collections\n\n\
     pub struct Tally { pub counts: collections::HashMap[String, i64] }\n\n\
     pub fn tally_new() -> Tally {\n\
     \x20   return Tally { counts: collections::HashMap() }\n}\n\n\
     pub fn count_in(mut t: Tally, word: ref String) {\n\
     \x20   t.counts[word.clone()] = 1\n}\n\n\
     pub fn tally_size(t: ref Tally) -> i64 {\n\
     \x20   return t.counts.len()\n}\n\n\
     fn main() {\n}\n";

/// **The issue's library**: what a function makes says `from()`, what it adds
/// from an argument says `from(that argument)`, and a read adds nothing.
#[test]
fn the_ledger_says_what_each_function_puts_into_its_maps() {
    let (dir, ledger) = ledger_of("tally", TALLY);
    assert_eq!(maps_of(&ledger, "tally_new"), ["<result>.counts: from()"]);
    assert_eq!(maps_of(&ledger, "count_in"), ["t.counts: from(word)"]);
    assert_eq!(maps_of(&ledger, "tally_size"), ["t.counts: from()"]);
    done(&dir);
}

/// **A body that puts in keys it does not trust says `keyed`**: bytes read
/// from a socket, a call's result, a name nobody follows.
#[test]
fn a_body_that_reads_a_socket_says_keyed() {
    let source = "use std::net\nuse std::collections\n\n\
         pub fn fill(mut m: collections::HashMap[i64, i64]) throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   m[b.len()] = 1\n}\n\n\
         pub fn from_a_call(mut m: collections::HashMap[i64, i64], n: i64) {\n\
         \x20   m[other(n)] = 1\n}\n\n\
         fn other(n: i64) -> i64 {\n\x20   return n\n}\n\n\
         fn main() {\n}\n";
    let (dir, ledger) = ledger_of("socket", source);
    assert_eq!(maps_of(&ledger, "fill"), ["m: keyed"]);
    assert_eq!(maps_of(&ledger, "from_a_call"), ["m: keyed"]);
    done(&dir);
}

/// **A map that goes anywhere the body does not show is `keyed`**, and so is
/// every map of the same function: what is not seen is not trusted (D1).
#[test]
fn a_map_handed_on_says_keyed() {
    let source = "use std::collections\n\n\
         pub fn pass(m: collections::HashMap[String, i64]) -> i64 {\n\
         \x20   return count(m)\n}\n\n\
         fn count(m: collections::HashMap[String, i64]) -> i64 {\n\
         \x20   return m.len()\n}\n\n\
         fn main() {\n}\n";
    let (dir, ledger) = ledger_of("handed-on", source);
    assert_eq!(maps_of(&ledger, "pass"), ["m: keyed"]);
    done(&dir);
}

/// **Maps a value passes between are one line**: a parameter returned, and
/// two parameters the body compares. Two that it only reads from and fills
/// from each other are two lines: the join is what the body needs to be one
/// type, not what the written type says.
#[test]
fn maps_that_pass_between_positions_are_one_line() {
    let source = "use std::collections\n\n\
         pub fn same_map(m: collections::HashMap[String, i64]) -> collections::HashMap[String, i64] {\n\
         \x20   return m\n}\n\n\
         pub fn same(a: ref collections::HashMap[String, i64], b: ref collections::HashMap[String, i64]) -> bool {\n\
         \x20   return a == b\n}\n\n\
         pub fn merge(mut a: collections::HashMap[String, i64], b: ref collections::HashMap[String, i64]) {\n\
         \x20   for k in b.keys() {\n\
         \x20       a[k.clone()] = 1\n\
         \x20   }\n}\n\n\
         fn main() {\n}\n";
    let (dir, ledger) = ledger_of("one-map", source);
    assert_eq!(maps_of(&ledger, "same_map"), ["<result> | m: from()"]);
    assert_eq!(maps_of(&ledger, "same"), ["a | b: from()"]);
    assert_eq!(maps_of(&ledger, "merge"), ["a: from(b)", "b: from()"]);
    done(&dir);
}

/// **One line per position, not per parameter**: `handle` fills only the
/// sessions from the request, so the config says nothing of it.
#[test]
fn a_struct_parameter_has_a_line_for_each_map() {
    let source = "use std::collections\n\n\
         pub struct State { pub config: collections::HashMap[String, String], pub sessions: collections::HashMap[String, i64] }\n\n\
         pub fn handle(mut state: State, request: ref String) {\n\
         \x20   state.sessions[request.clone()] = 1\n\
         \x20   println(f\"{state.config.len()}\")\n}\n\n\
         pub fn load(mut state: State, line: ref String) throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   state.sessions[f\"{b.len()}\"] = 1\n\
         \x20   state.config[line.clone()] = line.clone()\n}\n\n\
         fn main() {\n}\n";
    let source = source.replace(
        "use std::collections\n",
        "use std::collections\nuse std::net\n",
    );
    let (dir, ledger) = ledger_of("positions", &source);
    assert_eq!(
        maps_of(&ledger, "handle"),
        ["state.config: from()", "state.sessions: from(request)"]
    );
    // The socket bytes are in the sessions; the config only gets the line.
    assert_eq!(
        maps_of(&ledger, "load"),
        ["state.config: from(line)", "state.sessions: keyed"]
    );
    done(&dir);
}

/// **A lookup reads and adds nothing**: counting a word reads the old count
/// with `m[k] ?? 0` and writes the new one, and the keys are the word's.
#[test]
fn a_lookup_in_the_map_adds_no_key() {
    let source = "use std::collections\n\n\
         pub fn count_in(mut m: collections::HashMap[String, i64], key: ref String) {\n\
         \x20   let n = m[key.clone()] ?? 0\n\
         \x20   m[key.clone()] = n + 1\n}\n\n\
         fn main() {\n}\n";
    let (dir, ledger) = ledger_of("lookup", source);
    assert_eq!(maps_of(&ledger, "count_in"), ["m: from(key)"]);
    done(&dir);
}

/// **A method says it too**, under the key the ledger gives methods.
#[test]
fn a_method_of_a_type_with_maps_has_a_line() {
    let source = "use std::collections\n\n\
         pub struct Names { pub seen: collections::HashSet[String], pub ids: collections::HashMap[String, i64] }\n\n\
         impl Names {\n\
         \x20   pub fn add(ref mut self, word: ref String) {\n\
         \x20       self.seen.insert(word.clone())\n\
         \x20       self.ids[word.clone()] = 1\n\
         \x20   }\n}\n\n\
         fn main() {\n}\n";
    let (dir, ledger) = ledger_of("method", source);
    assert_eq!(
        maps_of(&ledger, "Names::add"),
        ["self.ids: from(word)", "self.seen: from(word)"]
    );
    done(&dir);
}

/// **A function with no map in its signature has no line**, and the file
/// writes the lines sorted, one key per function, and reads them back.
#[test]
fn the_lines_are_written_sorted_and_read_back() {
    let (dir, ledger) = ledger_of("text", TALLY);
    let text = ledger.render();
    let at = text
        .find("[fn.\"count_in\"]")
        .expect("count_in is in the file");
    assert!(
        text[at..].contains("maps = [\"t.counts: from(word)\"]"),
        "{text}"
    );
    let main = text
        .find("[fn.\"main\"]")
        .map(|m| &text[m + 1..])
        .unwrap_or("");
    let main = main.split("\n[").next().unwrap_or("");
    assert!(!main.contains("maps"), "{text}");
    assert_eq!(text, ledger.render(), "the same bytes twice");
    let path = dir.join("nikaia.contracts");
    std::fs::write(&path, &text).expect("write the ledger");
    let read = nikaia::contracts::Ledger::read_beside(&path).expect("the ledger reads back");
    assert_eq!(maps_of(&read, "count_in"), ["t.counts: from(word)"]);
    assert_eq!(maps_of(&read, "tally_new"), ["<result>.counts: from()"]);
    done(&dir);
}
