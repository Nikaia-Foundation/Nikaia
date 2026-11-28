//! **A view that outlives its buffer is tethered**, and the compiler decides
//! where the buffer lives ([ADR-tether-v1](../../../docs/specification/adr/adr-tether-v1.md)).
//!
//! The programs the design was checked against run as test programs in
//! `tests/compiler/tests/`, at both settings of `user_parallelism` and against
//! real files (#2849): `settings_handed_back_from_a_loader_live_in_the_callers_keep`,
//! `a_list_of_lists_kept_outside_a_loop_holds_every_buffer_it_read`,
//! `views_of_recursive_includes_in_one_map_through_a_mut_parameter`,
//! `a_task_takes_settings_that_view_a_buffer_and_carries_its_keep`,
//! `a_task_takes_a_map_of_views`,
//! `a_map_of_views_that_drops_entries_in_a_loop_holds_each_view`,
//! `a_loader_with_nothing_written_above_it_needs_no_word`,
//! `one_line_of_a_mapped_file_handed_back_keeps_the_mapping`, and those of the
//! container that drops entries. Here is only what their lowering writes, and
//! `chain`, which still runs here: written without `ref`, its `return s.value`
//! of a `for` element's field is refused (NK1104).
//!
//! The programs that need no files - a buffer moved, copied or assigned again -
//! are run as behaviour tests in `tests/language/src/tether_keep.nika`; here
//! only what their lowering writes is checked.

use crate::common;
use nikaia::contracts::LedgerOps;
#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
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
    let dir = common::scratch_dir(&format!("tether-{purpose}"));
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
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_text();
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

const LOADER: &str = r##"use std::fs

struct Setting {
    key: String,
    value: String,
}

fn load(path: String) -> Vec[Setting] throws {
    let text = fs::read_to_string(path, fs::Root::Anywhere)
    let mut settings: Vec[Setting] = []
    for line in text.lines() {
        if line.starts_with("#") { continue }
        let parts: Vec[String] = line.split("=").collect()
        if parts.len() == 2 {
            settings.push(Setting { key: parts[0].trim(), value: parts[1].trim() })
        }
    }
    return settings
}

fn main() throws {
    for s in load("app.conf") {
        println(f"{s.key} -> {s.value}")
    }
}
"##;

const CHAIN: &str = r##"use std::fs

struct Setting {
    key: ref String,
    value: ref String,
}

fn load(path: String) -> Vec[Setting] throws {
    let text = fs::read_to_string(path, fs::Root::Anywhere)
    let stamp = fs::read_to_string("extra.conf", fs::Root::Anywhere)
    let mut settings: Vec[Setting] = []
    for line in text.lines() {
        let parts: Vec[ref String] = line.split("=").collect()
        if parts.len() == 2 {
            settings.push(Setting { key: parts[0].trim(), value: parts[1].trim() })
        }
    }
    println(f"stamp has {stamp.len()} bytes")
    return settings
}

fn host_of(path: String) -> ref String throws {
    let settings = load(path)
    for s in settings {
        if s.key == "host" { return s.value }
    }
    return "localhost"
}

fn main() throws {
    println(host_of("app.conf"))
}
"##;

const TASK: &str = r##"use std::fs

struct Setting {
    key: String,
    value: String,
}

fn load(path: String) -> Vec[Setting] throws {
    let text = fs::read_to_string(path, fs::Root::Anywhere)
    let mut settings: Vec[Setting] = []
    for line in text.lines() {
        let parts: Vec[String] = line.split("=").collect()
        if parts.len() == 2 {
            settings.push(Setting { key: parts[0].trim(), value: parts[1].trim() })
        }
    }
    return settings
}

fn main() throws {
    let settings = load("app.conf")
    let worker = spawn fn {
        let mut n = 0
        for s in settings { n = n + s.value.len() }
        n
    }
    println(f"{worker.join()} bytes of values")
}
"##;

const CACHE: &str = r##"use std::fs
use std::collections

fn main() throws {
    let mut last_seen = collections::HashMap()
    let mut round = 0
    while round < 1000 {
        let text = fs::read_to_string("app.conf", fs::Root::Anywhere)
        for line in text.lines() {
            last_seen.insert(line, round)
        }
        if last_seen.len() > 2 { last_seen.clear() }
        round = round + 1
    }
    println(f"{last_seen.len()} lines remembered")
}
"##;

/// Two frames up, and a pause after the buffer exists: the keep is forwarded, and nothing is a closure that could not pause.
#[test]
fn chain() {
    runs("chain", CHAIN, "stamp has 32 bytes\nexample.org");
}

/// **The shape of what is written**, for the three representations: plain
/// borrows of the caller's keep, a packed value for the task, a held view.
#[test]
fn each_representation_is_the_one_the_plan_chose() {
    let loader = lowered(LOADER, Build::default());
    assert!(loader.contains("fn load<'k>("), "{loader}");
    assert!(
        loader.contains("__keep: &'k nikaia_std::tether::Keep"),
        "{loader}"
    );
    assert!(loader.contains("Vec<Setting<'k, 'k>>"), "{loader}");
    // `put_viewed`: a keep handed in may be a task's, whose `Holding` finds
    // the views again in it rather than copying them (ADR-218 D4).
    assert!(loader.contains("__keep.put_viewed("), "{loader}");
    assert!(
        !loader.contains("Arc"),
        "a frame keep costs no count: {loader}"
    );

    let task = lowered(TASK, Build::parallel());
    assert!(
        task.contains("let __keep_task = std::sync::Arc::new("),
        "{task}"
    );
    // **Held by the task's keep, and no `unsafe`** (ADR-218 D4, #84): the
    // packed value is a `Holding` that finds each view again in the keep.
    assert!(
        task.contains("::new([std::sync::Arc::clone(&__keep_task)]"),
        "{task}"
    );
    assert!(task.contains("settings.get()"), "{task}");
    assert!(!task.contains("unsafe"), "{task}");
    assert!(task.contains("__keep.put_viewed("), "{task}");

    let cache = lowered(CACHE, Build::default());
    // **Checked against the buffer the plan names** (ADR-tether-v1 D15,
    // #2842): the buffer's place comes with it, and the line is held by it.
    assert!(
        cache.contains("nikaia_std::tether::Held::in_place(std::sync::Arc::clone(&__keep_"),
        "{cache}"
    );
    assert!(cache.contains(".put_viewed_at("), "{cache}");
    // **No `unsafe` for a held view** (ADR-tether-v1 D14): the keep finds the view
    // again by address, so what the plan claims is checked, not trusted.
    assert!(!cache.contains("unsafe"), "{cache}");
}

/// **Nothing is kept that needs no keep**: a buffer whose views stay in its
/// scope is the plain local it always was, and a buffer handed on whole is a
/// move.
#[test]
fn a_buffer_nothing_outlives_is_left_alone() {
    let source = "use std::fs\n\
                  fn count(path: String) -> i64 throws {\n\
                  \x20   let text = fs::read_to_string(path, fs::Root::Anywhere)\n\
                  \x20   let mut n = 0\n\
                  \x20   for line in text.lines() { n = n + line.len() as i64 }\n\
                  \x20   return n\n\
                  }\n\
                  fn whole(path: String) -> String throws {\n\
                  \x20   let text = fs::read_to_string(path, fs::Root::Anywhere)\n\
                  \x20   return text\n\
                  }\n\
                  fn main() throws { println(f\"{count(\"app.conf\")} {whole(\"app.conf\").len()}\") }\n";
    let rust = lowered(source, Build::default());
    assert!(!rust.contains("nikaia_std::tether"), "{rust}");
}

// ---------------------------------------------------------------------------
// A container of structs holding views that drops entries in a loop
// ([ADR-tether-v1](../../../docs/specification/adr/adr-tether-v1.md))
// ---------------------------------------------------------------------------

/// The program `NK2304` refused until ADR-tether-v1: records of the lines of files
/// read in a loop, kept in a list that is pruned as it goes, and read back by
/// index, by a `for`, and through a method.
const RECORDS: &str = r##"use std::fs

struct Record {
    name: String,
    size: i64,
}

impl Record {
    fn describe(self) -> String {
        return f"{self.name}:{self.size}"
    }
}

fn main() throws {
    let mut kept: Vec[Record] = Vec()
    let mut round = 0
    while round < 200 {
        for path in ["app.conf", "extra.conf"] {
            let text = fs::read_to_string(path, fs::Root::Anywhere)
            for line in text.lines() {
                let name = line.trim()
                kept.push(Record { name: name, size: line.len() as i64 })
            }
            while kept.len() > 3 { kept.remove(0) }
        }
        round = round + 1
    }
    for r in kept { println(r.describe()) }
    println(f"{kept.len()} {kept[0].name} {kept[2].describe()}")
    println(kept.last()?.name ?? "-")
}
"##;

#[test]
fn a_list_of_structs_of_views_that_drops_entries_is_held() {
    let rust = lowered(RECORDS, Build::default());
    assert!(
        rust.contains("Vec<nikaia_std::tether::Holding<__Views_Record, 1>>"),
        "{rust}"
    );
    assert!(rust.contains("enum __Views_Record {}"), "{rust}");
    assert!(rust.contains(".map(|held| held.get())"), "{rust}");
    assert!(!rust.contains("unsafe"), "{rust}");
}

/// **A value whose views point into two buffers carries a handle on each**
/// (ADR-tether-v1 D15), and neither buffer is copied.
const PAIRS: &str = r##"use std::fs

struct Pair {
    key: String,
    value: String,
}

fn main() throws {
    let mut pairs: Vec[Pair] = Vec()
    let mut round = 0
    while round < 100 {
        let a = fs::read_to_string("app.conf", fs::Root::Anywhere)
        let b = fs::read_to_string("extra.conf", fs::Root::Anywhere)
        pairs.push(Pair { key: a.trim(), value: b.trim() })
        if pairs.len() > 2 { pairs.remove(0) }
        round = round + 1
    }
    println(f"{pairs.len()} {pairs[0].key.len()} {pairs[1].value.len()}")
}
"##;

#[test]
fn a_struct_of_views_into_two_buffers_holds_both() {
    let rust = lowered(PAIRS, Build::default());
    assert!(rust.contains("Holding<__Views_Pair, 2>"), "{rust}");
}

/// **Dropping entries after the loop is not dropping them while it goes on**:
/// the frame's keep holds the buffers exactly as long, no element carries a
/// handle, and the program is not refused (the case ADR-tether-v1 D18 fixes).
const CLEARED_AFTER: &str = r##"use std::fs

struct Record {
    name: String,
    size: i64,
}

fn main() throws {
    let mut kept: Vec[Record] = Vec()
    for path in ["app.conf", "extra.conf"] {
        let text = fs::read_to_string(path, fs::Root::Anywhere)
        kept.push(Record { name: text.trim(), size: text.len() as i64 })
    }
    println(f"{kept.len()} {kept[1].size}")
    kept.clear()
    println(f"{kept.len()}")
}
"##;

#[test]
fn entries_dropped_only_after_the_loop_need_no_handle() {
    let rust = lowered(CLEARED_AFTER, Build::default());
    assert!(rust.contains("let __keep_frame"), "{rust}");
    assert!(!rust.contains("Holding"), "{rust}");
}

/// **An element taken out stays held, and a field is written through the
/// handle** (ADR-tether-v1 D16, D17): `old` carries its buffer's keep out of the list,
/// a number is stored as it is, and a view made outside the element is found
/// again in its keeps - or, here, is a literal and is copied into them.
const TAKEN_AND_WRITTEN: &str = r##"use std::fs

struct Record {
    name: String,
    size: i64,
}

fn main() throws {
    let mut kept: Vec[Record] = Vec()
    let mut gone = 0
    for path in ["app.conf", "extra.conf", "app.conf"] {
        let text = fs::read_to_string(path, fs::Root::Anywhere)
        kept.push(Record { name: text.trim(), size: text.len() as i64 })
        if kept.len() > 2 {
            let old = kept.remove(0)
            gone = gone + old.size
            println(old.name.len())
        }
    }
    kept[0].size = 7
    kept[1].size += 1
    kept[1].name = "renamed"
    println(f"{gone} {kept[0].size} {kept[1].size} {kept[1].name}")
}
"##;

#[test]
fn an_element_taken_out_stays_held_and_a_field_is_written_through_the_handle() {
    let rust = lowered(TAKEN_AND_WRITTEN, Build::default());
    assert!(rust.contains("old.get()"), "{rust}");
    // A number is stored as it is; a view is found again in the keeps.
    assert!(rust.contains(".with_mut(|held, _| held.size = "), "{rust}");
    assert!(
        rust.contains(".with_mut(|held, keeps| held.name = "),
        "{rust}"
    );
    assert!(!rust.contains("unsafe"), "{rust}");
}

/// **A buffer assigned whole is moved, not viewed**: `inner = args` takes the
/// text along, as a `return args` and a `let other = args` already did. Read as
/// views flowing into an `inner` declared around the buffer, `args` went into
/// the frame's keep and the assignment put a view where a `String` goes -
/// `rustc`'s *mismatched types* about a file nobody wrote (found moving
/// `Ty::parse` into Nikaia, ADR-294).
#[test]
fn a_buffer_assigned_whole_is_moved() {
    let source = "fn cut(written: String, front: i64) -> String {
    let c: Vec[scalar] = written.chars().collect()
    let mut out: String = \"\"
    for k in front..<c.len() {
        out.push(c[k])
    }
    return out
}

fn pick(rest: String) -> String {
    let mut inner: String = \"\"
    if rest.starts_with(\"a\") {
        let args = cut(rest, 1)
        if args.len() > 1 {
            inner = args
        }
    }
    return inner
}

fn main() throws {
    println(pick(\"abc\"))
    println(pick(\"ax\"))
    println(pick(\"zzz\"))
}
";
    assert!(
        !lowered(source, Build::default()).contains("put("),
        "nothing is kept: the buffer moves"
    );
}

/// **A copy of a trimmed text holds nothing of it**, with no type written on
/// the view (#293). `last` is `current.trim()`, and `last.clone()` is text of
/// its own; read as a copy of the view, it carried `current` into `out`, put
/// `current` into the frame's keep, and the `current = ""` in the loop then
/// assigned a `String` where the keep's place stood - `rustc` about a file
/// nobody wrote.
#[test]
fn a_copy_of_an_untyped_view_keeps_nothing() {
    let source = "fn pieces(text: String) -> Vec[String] {
    let mut out: Vec[String] = []
    let mut current: String = \"\"
    for c in text.chars() {
        if c == ',' {
            out.push(current.trim().clone())
            current = \"\"
            continue
        }
        current.push(c)
    }
    let last = current.trim()
    if !last.is_empty() {
        out.push(last.clone())
    }
    return out
}

fn main() throws {
    let p = pieces(\"a, b,c\")
    println(f\"{p.len()} {p[2]}\")
}
";
    assert!(
        !lowered(source, Build::default()).contains("put("),
        "nothing is kept: every copy is text of its own"
    );
}

/// **A kept buffer assigned again puts its new value into the keep too**
/// (#293): the view taken before still reads the old text, which the keep goes
/// on holding.
#[test]
fn a_kept_buffer_assigned_again_is_kept_again() {
    let source = "fn first_then(text: String) -> String {
    let mut views: Vec[String] = []
    let mut current: String = \"old\"
    views.push(current.trim())
    current = text.clone()
    views.push(current.trim())
    return f\"{views[0]} {views[1]}\"
}

fn main() throws {
    println(first_then(\"new\"))
}
";
    let rust = lowered(source, Build::default());
    assert!(rust.contains("current = __keep_frame.put("), "{rust}");
}

/// **Owned text handed to a field whole is moved there** (#686), as it is in a
/// struct literal: `l.from = key` inside a loop keeps no view of `key`, so
/// `key` is not put into the frame's keep, and its `String?` is `Some(..)`.
#[test]
fn owned_text_assigned_to_a_field_is_moved_not_kept() {
    let source = "struct L {
    from: String?,
}

fn word(n: i64) -> String {
    return f\"w{n}\"
}

fn main() throws {
    let mut l = L { from: null }
    for i in 0..<2 {
        if i == 1 {
            let key: String? = word(i)
            l.from = key
        }
    }
    println(l.from ?? \"-\")
}
";
    assert!(
        !lowered(source, Build::default()).contains("__keep_frame"),
        "{}",
        lowered(source, Build::default())
    );
    // Only read inside the loop, it stays an owned option, no copy made.
    let read = source.replace("l.from = key", "println(key ?? \"\")");
    let rust = lowered(&read, Build::default());
    assert!(
        rust.contains("let key: Option<String> = Some(word(i));"),
        "{rust}"
    );
    assert!(
        !rust.contains("__keep_frame") && !rust.contains(".clone()"),
        "{rust}"
    );
}
