//! **A map's trust is the map's** (ADR-010 D1, #431): in a program that reads
//! both a file and a socket, the map filled from the file keeps the fast hash
//! and the one filled from the socket gets the keyed one. Whatever this cannot
//! follow is untrusted.

mod common;

use nikaia::emit::emit_program;
use nikaia::parser::parse_to_ast;

fn lowered(source: &str) -> String {
    let parsed = parse_to_ast(source).expect("the source parses");
    emit_program(&parsed, Default::default())
        .expect("it lowers")
        .rust
}

/// The two maps of one program, each by the line that makes it.
fn the_map_made_by(rust: &str, name: &str) -> String {
    rust.lines()
        .find(|l| l.contains(&format!("let mut {name}")))
        .unwrap_or_else(|| panic!("no `{name}` in:\n{rust}"))
        .to_string()
}

const BOTH: &str = "use std::net\n\
     use std::fs\n\
     use std::collections\n\
     fn main() throws {\n\
     \x20   let text = fs::read_to_string(\"words.txt\", fs::Root::Anywhere)\n\
     \x20   let mut from_file = collections::HashMap()\n\
     \x20   for w in text.split(\" \") { from_file.insert(w, 1) }\n\
     \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
     \x20   let b = c.read()\n\
     \x20   let mut from_socket = collections::HashMap()\n\
     \x20   from_socket.insert(b.len(), 1)\n\
     \x20   println(f\"{from_file.len()} {from_socket.len()}\")\n\
     }\n";

#[test]
fn a_file_map_beside_a_socket_map_keeps_the_fast_hash() {
    let rust = lowered(BOTH);
    let file = the_map_made_by(&rust, "from_file");
    let socket = the_map_made_by(&rust, "from_socket");
    assert!(file.contains("TrustedMap"), "{file}");
    assert!(!socket.contains("TrustedMap"), "{socket}");
    // **And the two kinds of map stand in one program** that `rustc` takes.
    let dir = common::scratch_dir("map-trust");
    let path = dir.join("program.rs");
    std::fs::write(&path, &rust).expect("write the Rust");
    let binary = dir.join("program");
    let compiled = common::compile(
        &path,
        &["--crate-type", "bin", "-o", &binary.to_string_lossy()],
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        compiled.status.success(),
        "{}\n{rust}",
        String::from_utf8_lossy(&compiled.stderr)
    );
}

/// **A map that leaves its function is not followed**: handed to another
/// function, its type is named there too, so it takes the program's answer.
#[test]
fn a_map_that_leaves_its_function_takes_the_programs_answer() {
    let source = BOTH.replace(
        "    println(f\"{from_file.len()} {from_socket.len()}\")\n",
        "    println(f\"{count(from_file)} {from_socket.len()}\")\n",
    ) + "fn count(m: collections::HashMap[ref String, i64]) -> i64 {\n    return m.len()\n}\n";
    let rust = lowered(&source);
    assert!(!rust.contains("TrustedMap"), "{rust}");
}

/// **A key from a function of the program's own is not followed**: it may read
/// anything.
#[test]
fn a_key_from_an_own_function_is_untrusted() {
    let source = BOTH.replace("from_file.insert(w, 1)", "from_file.insert(word(w), 1)")
        + "fn word(w: ref String) -> String {\n    return w.clone()\n}\n";
    let rust = lowered(&source);
    let file = the_map_made_by(&rust, "from_file");
    assert!(!file.contains("TrustedMap"), "{file}");
}

/// **A program with no untrusted source is as before**: every map is fast.
#[test]
fn a_program_that_reads_only_files_keeps_every_map_fast() {
    let source = "use std::fs\n\
         use std::collections\n\
         fn main() throws {\n\
         \x20   let text = fs::read_to_string(\"words.txt\", fs::Root::Anywhere)\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   for w in text.split(\" \") { m.insert(w, 1) }\n\
         \x20   println(f\"{m.len()}\")\n\
         }\n";
    let rust = lowered(source);
    assert!(the_map_made_by(&rust, "m").contains("TrustedMap"), "{rust}");
}

// ---------------------------------------------------------------------------
// **A map's trust is the package's** (ADR-010 D9, #581 step 1).
//
// A map is one type in every file of its package, so whether a key can be an
// adversary's is answered over all the files, in any order: the entry file no
// longer speaks for the others, and `--trust` says what the emitter writes.
// ---------------------------------------------------------------------------

use std::path::{Path, PathBuf};
use std::process::Command;

/// A project of `src/<file>` entries with `src/main.nika` the entry.
fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = common::scratch_dir(&format!("map-trust-{name}"));
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(
        dir.join("nikaia.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
    )
    .expect("manifest");
    for (file, contents) in files {
        std::fs::write(dir.join("src").join(file), contents).expect("write the file");
    }
    dir
}

fn package_rust(dir: &Path) -> String {
    let program = nikaia::comptime_run::defaults_compiled_in(Some(&common::workshop()), || {
        nikaia::modules::Program::read(&dir.join("src/main.nika"))
    })
    .expect("the program reads");
    program
        .emit_reading(Default::default(), &common::reads())
        .expect("the program lowers")
        .rust
}

/// What `--trust` says of each file, in file order.
fn trust_report(dir: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .args(["lower", "src/main.nika", "--trust"])
        .current_dir(dir)
        .output()
        .expect("nikaia runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn compiles(rust: &str, dir: &Path) {
    let path = dir.join("program.rs");
    std::fs::write(&path, rust).expect("write the Rust");
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
}

/// The file that reads nothing: a map it builds and hands to its own function.
const READS_NOTHING: &str = "use std::collections\n\n\
     fn size(m: collections::HashMap[ref String, i64]) -> i64 {\n    return m.len()\n}\n\n\
     fn quiet() -> i64 {\n\
     \x20   let mut m = collections::HashMap()\n\
     \x20   m.insert(\"a\", 1)\n\
     \x20   return size(m)\n}\n";

/// The file that reads a socket.
const READS_A_SOCKET: &str = "use std::net\n\nuse std::collections\n\n\
     pub fn serve() -> i64 throws {\n\
     \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
     \x20   let b = c.read()\n\
     \x20   let mut m = collections::HashMap()\n\
     \x20   m.insert(b.len(), 1)\n\
     \x20   return size_n(m)\n}\n\n\
     fn size_n(m: collections::HashMap[i64, i64]) -> i64 {\n    return m.len()\n}\n";

fn entry_calling(other: &str) -> String {
    format!("fn main() throws {{\n    println(f\"{{{other}()}}\")\n}}\n")
}

fn assert_keyed_everywhere(rust: &str) {
    assert!(
        !rust.contains("TrustedMap"),
        "a socket is read in the package, so no map has the fixed seed:\n{rust}"
    );
    let size = rust
        .lines()
        .find(|l| l.contains("fn size("))
        .expect("`size` is lowered");
    assert!(
        size.contains("HashMap<&str, i64>") && !size.contains("Trusted"),
        "{size}"
    );
}

#[test]
fn a_socket_read_in_another_file_keys_the_entry_files_maps() {
    let dir = project(
        "other-file",
        &[
            (
                "main.nika",
                &(READS_NOTHING.to_string() + &entry_calling("quiet")),
            ),
            ("serve.nika", READS_A_SOCKET),
        ],
    );
    let rust = package_rust(&dir);
    assert_keyed_everywhere(&rust);
    // **And the report is the code's**: both files are keyed.
    let report = trust_report(&dir);
    assert_eq!(
        report
            .matches("hash for a map keyed by the input: keyed")
            .count(),
        2,
        "{report}"
    );
    assert!(!report.contains("fast, fixed seed"), "{report}");
    compiles(&rust, &dir);
    let _ = std::fs::remove_dir_all(&dir);
}

/// **The order of the files changes nothing**: the entry is the one that reads
/// the socket now, and the file that reads nothing is the other one.
#[test]
fn the_order_of_the_files_does_not_change_the_hasher() {
    // The entry reads the socket (and says so first); the other file holds the
    // map that leaves its function.
    let entry = READS_A_SOCKET.replace("pub fn serve", "fn serve") + &entry_calling("serve");
    let quiet = READS_NOTHING.replace("fn quiet", "pub fn quiet");
    let dir = project("reversed", &[("main.nika", &entry), ("quiet.nika", &quiet)]);
    let rust = package_rust(&dir);
    assert_keyed_everywhere(&rust);
    let report = trust_report(&dir);
    assert!(!report.contains("fast, fixed seed"), "{report}");
    compiles(&rust, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    // The same two files with the names swapped, so the sorted order flips.
    let dir = project(
        "reversed-names",
        &[
            (
                "main.nika",
                &(READS_NOTHING.to_string() + &entry_calling("quiet")),
            ),
            ("aaa_serve.nika", READS_A_SOCKET),
        ],
    );
    let rust = package_rust(&dir);
    assert_keyed_everywhere(&rust);
    compiles(&rust, &dir);
    let _ = std::fs::remove_dir_all(&dir);
}

/// **No regression**: a package that reads only files keeps the fast map in
/// every file.
#[test]
fn a_package_that_reads_only_files_keeps_the_fast_map_in_every_file() {
    let reads_a_file = "use std::fs\nuse std::collections\n\n\
         pub fn counted() -> i64 throws {\n\
         \x20   let text = fs::read_to_string(\"words.txt\", fs::Root::Anywhere)\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   for w in text.split(\" \") { m.insert(w, 1) }\n\
         \x20   return size(m)\n}\n";
    let dir = project(
        "files-only",
        &[
            (
                "main.nika",
                &(READS_NOTHING.to_string()
                    + "fn main() throws {\n    println(f\"{quiet()} {counted()}\")\n}\n"),
            ),
            ("counts.nika", reads_a_file),
        ],
    );
    let rust = package_rust(&dir);
    let size = rust.lines().find(|l| l.contains("fn size(")).expect("size");
    assert!(size.contains("TrustedMap"), "{size}");
    let report = trust_report(&dir);
    assert!(!report.contains("keyed, per-process"), "{report}");
    compiles(&rust, &dir);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// **The toolchain's own package** (`lower_tools_at`): each file is lowered with
// the others beside it, and the trust is the package's, not the file's.
// ---------------------------------------------------------------------------

fn tools_lowered(name: &str, files: &[(&str, &str)]) -> String {
    let dir = common::scratch_dir(&format!("map-trust-tools-{name}"));
    for (file, contents) in files {
        std::fs::write(dir.join(file), contents).expect("write the file");
    }
    let rust = nikaia::sysroot::lower_tools_at(&dir, Default::default()).expect("it lowers");
    let _ = std::fs::remove_dir_all(&dir);
    rust
}

const HM_A: &str = "use std::collections\n\n\
     pub struct HmNames {\n    seen: collections::HashSet[String],\n    ids: collections::HashMap[String, i64],\n}\n";
const HM_B: &str = "use std::collections\n\n\
     pub fn hm_count() -> i64 {\n\
     \x20   let mut n = HmNames { seen: collections::HashSet(), ids: collections::HashMap() }\n\
     \x20   n.seen.insert(\"a\")\n\
     \x20   return n.seen.len()\n}\n";
const HM_C: &str = "use std::net\n\nuse std::collections\n\n\
     pub fn hm_serve() -> i64 throws {\n\
     \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
     \x20   let b = c.read()\n\
     \x20   let mut m = collections::HashMap()\n\
     \x20   m.insert(b.len(), 1)\n\
     \x20   return m.len()\n}\n";

/// **No file of the package mixes the two**: a type declared in one file and
/// built and filled in another is one type everywhere (it was `E0308`).
#[test]
fn a_tools_struct_filled_in_another_file_is_one_type() {
    for files in [
        [("hm_a.nika", HM_A), ("hm_b.nika", HM_B)],
        [("hm_b.nika", HM_B), ("hm_a.nika", HM_A)],
    ] {
        let rust = tools_lowered("trusted", &files);
        assert!(!rust.contains("HashSet::new()"), "{rust}");
        assert!(!rust.contains("HashMap::new()"), "{rust}");
        assert!(rust.contains("TrustedSet"), "{rust}");
    }
}

/// **A socket read in any file keys the maps of all of them**, whichever file
/// declares, builds or fills the map.
#[test]
fn a_socket_read_in_one_tools_file_keys_the_maps_of_the_others() {
    let rust = tools_lowered(
        "keyed",
        &[
            ("hm_a.nika", HM_A),
            ("hm_b.nika", HM_B),
            ("hm_c.nika", HM_C),
        ],
    );
    assert!(
        !rust.contains("TrustedMap") && !rust.contains("TrustedSet"),
        "{rust}"
    );
}

// ---------------------------------------------------------------------------
// **One lowered type per class** (ADR-010 D9, #581 steps 2 and 3): a map that
// several declarations share has one Rust type, which is a function of the
// class and of the package, never of the file lowered or of the order the
// files are read in. Every test hands the Rust to `rustc`: a type error
// must never reach a user (Part III C.1).
// ---------------------------------------------------------------------------

/// `lowered`, then `rustc`.
fn lowered_and_compiled(source: &str, name: &str) -> String {
    let rust = lowered(source);
    let dir = common::scratch_dir(&format!("map-trust-{name}"));
    compiles(&rust, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    rust
}

/// The file that declares the type, the one that builds it and the one that
/// fills it: three files, one struct.
const TALLY_DECLARED: &str = "use std::collections\n\n\
     pub struct Tally {\n\
     \x20   counts: collections::HashMap[String, i64],\n\
     \x20   seen: collections::HashSet[String],\n}\n";
const TALLY_BUILT: &str = "use std::collections\n\n\
     pub fn tally_new() -> Tally {\n\
     \x20   return Tally { counts: collections::HashMap(), seen: collections::HashSet() }\n}\n";
const TALLY_FILLED: &str = "pub fn tally_run() -> i64 {\n\
     \x20   let mut t = tally_new()\n\
     \x20   t.counts.insert(\"a\", 1)\n\
     \x20   t.seen.insert(\"b\")\n\
     \x20   return t.counts.len() + t.seen.len()\n}\n";

fn tally_package(name: &str, names: [&str; 3], with_socket: bool) -> String {
    let mut files: Vec<(&str, String)> = vec![
        (names[0], TALLY_DECLARED.to_string()),
        (names[1], TALLY_BUILT.to_string()),
        (names[2], TALLY_FILLED.to_string()),
        (
            "main.nika",
            "fn main() throws {\n    println(f\"{tally_run()}\")\n}\n".to_string(),
        ),
    ];
    if with_socket {
        files.push(("serve.nika", READS_A_SOCKET.to_string()));
    }
    let borrowed: Vec<(&str, &str)> = files.iter().map(|(n, c)| (*n, c.as_str())).collect();
    let dir = project(name, &borrowed);
    let rust = package_rust(&dir);
    compiles(&rust, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    rust
}

/// **Declared in one file, built in a second, filled in a third**: with a
/// socket read anywhere, every mention is keyed; with none, every mention is
/// fast. The file names are permuted, so the order the files are read in
/// changes nothing.
#[test]
fn a_struct_shared_between_files_is_one_type_in_any_order() {
    let orders = [
        ["a_tally.nika", "b_built.nika", "c_filled.nika"],
        ["c_tally.nika", "b_built.nika", "a_filled.nika"],
        ["b_tally.nika", "c_built.nika", "a_filled.nika"],
    ];
    for (at, names) in orders.into_iter().enumerate() {
        let keyed = tally_package(&format!("tally-keyed-{at}"), names, true);
        assert!(
            !keyed.contains("TrustedMap") && !keyed.contains("TrustedSet"),
            "{keyed}"
        );
        assert!(
            !keyed.contains("HashMap::new()") || keyed.contains("HashMap<"),
            "{keyed}"
        );
        let fast = tally_package(&format!("tally-fast-{at}"), names, false);
        assert!(
            fast.contains("TrustedMap") && fast.contains("TrustedSet"),
            "{fast}"
        );
        assert!(
            !fast.contains("HashMap::new()") && !fast.contains("HashSet::new()"),
            "{fast}"
        );
    }
}

/// **Step 3, the join**: a private helper that receives a map made from a
/// file and one made from a socket is one class, and keyed wins.
#[test]
fn a_helper_given_a_file_map_and_a_socket_map_keys_both() {
    let source = BOTH
        .replace("from_file.insert(w, 1)", "from_file.insert(w.len(), 1)")
        .replace(
            "    println(f\"{from_file.len()} {from_socket.len()}\")\n",
            "    println(f\"{count(from_file)} {count(from_socket)}\")\n",
        )
        + "fn count(m: collections::HashMap[i64, i64]) -> i64 {\n    return m.len()\n}\n";
    let rust = lowered_and_compiled(&source, "join-keyed");
    assert!(!rust.contains("TrustedMap"), "{rust}");
}

/// **And without the socket map in it**: the same helper over two maps from
/// files is fast, in its signature and in both maps.
#[test]
fn a_helper_given_two_file_maps_is_fast_in_all_three() {
    let source = "use std::fs\n\
         use std::collections\n\
         fn count(m: collections::HashMap[i64, i64]) -> i64 {\n    return m.len()\n}\n\
         fn main() throws {\n\
         \x20   let text = fs::read_to_string(\"words.txt\", fs::Root::Anywhere)\n\
         \x20   let mut a = collections::HashMap()\n\
         \x20   let mut b = collections::HashMap()\n\
         \x20   for w in text.split(\" \") { a.insert(w.len(), 1) }\n\
         \x20   for w in text.split(\"\\n\") { b.insert(w.len(), 1) }\n\
         \x20   println(f\"{count(a)} {count(b)}\")\n\
         }\n";
    let rust = lowered_and_compiled(source, "join-fast");
    assert!(the_map_made_by(&rust, "a").contains("TrustedMap"), "{rust}");
    assert!(the_map_made_by(&rust, "b").contains("TrustedMap"), "{rust}");
    let count = rust
        .lines()
        .find(|l| l.contains("fn count("))
        .expect("count");
    assert!(count.contains("TrustedMap"), "{count}");
}
