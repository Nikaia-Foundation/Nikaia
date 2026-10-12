//! **A map at a package's boundary is a type parameter each use fills**
//! (ADR-010 D9, #581 step 5): a `pub` item's map positions are hidden
//! parameters `S: nikaia_std::hash::Seed` in declaration order, the lowered
//! library is the same Rust for every program that uses it, and the program
//! decides `Fast` or `Keyed` for its own uses. Every case is handed to
//! `rustc` - library first, then the program against it - because a type
//! error here would reach a user (Part III C.1).

mod common;

use std::path::{Path, PathBuf};

/// A library `lib` and a program `app` beside it, both lowered in this
/// process: the library's Rust, the program's Rust.
struct Two {
    dir: PathBuf,
    lib: String,
    app: String,
}

fn lower(dir: &Path, package: &str) -> String {
    let program = nikaia::comptime_run::defaults_compiled_in(Some(&common::workshop()), || {
        let root = dir.join(package);
        let manifest = nikaia::manifest::Manifest::read(&root.join("nikaia.toml"))?;
        let dependencies = nikaia::project::packages_of(&manifest, &root, false)?;
        nikaia::modules::Program::read_with(&root.join("src/main.nika"), &dependencies)
    })
    .expect("the program reads");
    program
        .emit_reading(Default::default(), &common::reads())
        .expect("the program lowers")
        .rust
}

fn write_app(dir: &Path, name: &str, source: &str) {
    std::fs::create_dir_all(dir.join(name).join("src")).expect("the package");
    std::fs::write(
        dir.join(name).join("nikaia.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[dependencies]\nlib = {{ path = \"../lib\" }}\n"
        ),
    )
    .expect("a manifest");
    std::fs::write(dir.join(name).join("src/main.nika"), source).expect("a source");
}

fn two(purpose: &str, lib: &str, apps: &[(&str, &str)]) -> (PathBuf, String, Vec<String>) {
    let dir = common::scratch_dir(&format!("map-seeds-{purpose}"));
    std::fs::create_dir_all(dir.join("lib/src")).expect("lib");
    std::fs::write(
        dir.join("lib/nikaia.toml"),
        "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");
    std::fs::write(dir.join("lib/src/main.nika"), lib).expect("a source");
    for (name, source) in apps {
        write_app(&dir, name, source);
    }
    let lib_rust = lower(&dir, "lib");
    let apps = apps.iter().map(|(name, _)| lower(&dir, name)).collect();
    (dir, lib_rust, apps)
}

fn one(purpose: &str, lib: &str, app: &str) -> Two {
    let (dir, lib, mut apps) = two(purpose, lib, &[("app", app)]);
    Two {
        dir,
        lib,
        app: apps.remove(0),
    }
}

/// `rustc` on the library, then on the program against it. The library is
/// built once and each program is built against that one file.
fn compiles(dir: &Path, lib: &str, app: &str, which: &str) {
    let lib_path = dir.join("lib.rs");
    std::fs::write(&lib_path, lib).expect("write the library");
    let rlib = dir.join("liblib.rlib");
    if !rlib.exists() {
        let built = common::compile(
            &lib_path,
            &[
                "--crate-type",
                "rlib",
                "--crate-name",
                "lib",
                "-o",
                &rlib.to_string_lossy(),
            ],
        );
        assert!(
            built.status.success(),
            "the library:\n{}\n{lib}",
            String::from_utf8_lossy(&built.stderr)
        );
    }
    let app_path = dir.join(format!("{which}.rs"));
    std::fs::write(&app_path, app).expect("write the program");
    let binary = dir.join(format!("{which}-bin"));
    let built = common::compile(
        &app_path,
        &[
            "--crate-type",
            "bin",
            "--extern",
            &format!("lib={}", rlib.display()),
            "-o",
            &binary.to_string_lossy(),
        ],
    );
    assert!(
        built.status.success(),
        "the program:\n{}\n{app}\n--- the library:\n{lib}",
        String::from_utf8_lossy(&built.stderr)
    );
}

fn done(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// The library of the issue: a type with a map, a function that makes one, a
/// function that fills one.
const TALLY: &str = "use std::collections\n\n\
     pub struct Tally { pub counts: collections::HashMap[String, i64] }\n\n\
     pub fn tally_new() -> Tally {\n\
     \x20   return Tally { counts: collections::HashMap() }\n}\n\n\
     pub fn count_in(mut t: Tally, word: ref String) {\n\
     \x20   t.counts[word.clone()] = 1\n}\n\n\
     pub fn tally_size(t: ref Tally) -> i64 {\n\
     \x20   return t.counts.len()\n}\n\n\
     fn main() {\n}\n";

/// An app that fills it from a socket.
const FROM_A_SOCKET: &str = "use lib\nuse std::net\nuse std::collections\n\n\
     fn main() throws {\n\
     \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
     \x20   let b = c.read()\n\
     \x20   let mut t = lib::Tally { counts: collections::HashMap() }\n\
     \x20   lib::count_in(t, f\"{b.len()}\")\n\
     \x20   let u = lib::tally_new()\n\
     \x20   println(f\"{lib::tally_size(t)} {lib::tally_size(u)}\")\n}\n";

/// An app whose keys are its own.
const FROM_ITS_OWN: &str = "use lib\nuse std::collections\n\n\
     fn main() throws {\n\
     \x20   let mut t = lib::Tally { counts: collections::HashMap() }\n\
     \x20   lib::count_in(t, \"a\")\n\
     \x20   let u = lib::tally_new()\n\
     \x20   println(f\"{lib::tally_size(t)} {lib::tally_size(u)}\")\n}\n";

/// **(a)** A program that reads a socket fills the library's type with the
/// keyed map: both packages compile, and nothing names the fixed seed.
#[test]
fn a_socket_filled_library_type_is_keyed() {
    let t = one("socket", TALLY, FROM_A_SOCKET);
    assert!(t.lib.contains("nikaia_std::hash::Seed"), "{}", t.lib);
    assert!(
        !t.app.contains("TrustedMap") && !t.app.contains("hash::Fast"),
        "{}",
        t.app
    );
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **(b)** The same program with keys of its own: the fixed seed, and the
/// library's calls take it.
#[test]
fn a_file_filled_library_type_is_fast() {
    let t = one("own", TALLY, FROM_ITS_OWN);
    assert!(t.app.contains("TrustedMap"), "{}", t.app);
    assert!(t.app.contains("hash::Fast"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **(c)** The library is lowered once and two programs choose differently.
#[test]
fn one_library_two_programs_two_choices() {
    let (dir, lib, apps) = two(
        "both",
        TALLY,
        &[("socket", FROM_A_SOCKET), ("own", FROM_ITS_OWN)],
    );
    let again = lower(&dir, "lib");
    assert_eq!(lib, again, "the library lowers to the same Rust every time");
    compiles(&dir, &lib, &apps[0], "socket");
    compiles(&dir, &lib, &apps[1], "own");
    assert!(!apps[0].contains("hash::Fast"));
    assert!(apps[1].contains("hash::Fast"));
    done(&dir);
}

/// **(d)** A `pub` function that takes a map and returns one.
const PASSING: &str = "use std::collections\n\n\
     pub fn merged(a: collections::HashMap[String, i64], extra: ref String) -> collections::HashMap[String, i64] {\n\
     \x20   let mut m = a\n\
     \x20   m[extra.clone()] = 1\n\
     \x20   return m\n}\n\n\
     pub fn size(m: ref collections::HashMap[String, i64]) -> i64 {\n\
     \x20   return m.len()\n}\n\n\
     fn main() {\n}\n";

#[test]
fn a_function_that_takes_and_returns_a_map_serves_both_choices() {
    let own = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"a\", 1)\n\
         \x20   let n = lib::merged(m, \"b\")\n\
         \x20   println(f\"{lib::size(n)}\")\n}\n";
    let socket = "use lib\nuse std::net\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"{b.len()}\", 1)\n\
         \x20   let n = lib::merged(m, \"b\")\n\
         \x20   println(f\"{lib::size(n)}\")\n}\n";
    let (dir, lib, apps) = two("passing", PASSING, &[("own", own), ("socket", socket)]);
    compiles(&dir, &lib, &apps[0], "own");
    compiles(&dir, &lib, &apps[1], "socket");
    assert!(apps[1].contains("collections::HashMap"), "{}", apps[1]);
    assert!(!apps[1].contains("TrustedMap"), "{}", apps[1]);
    done(&dir);
}

/// **A library that reads from outside is as untrusted as the programs that
/// fill its maps**: its body puts keys into a map the program may have given
/// the fixed seed, so the program keeps the keyed one.
#[test]
fn a_library_that_reads_a_socket_keys_the_maps_of_a_trusted_program() {
    let lib = "use std::net\nuse std::collections\n\n\
         pub fn fill(mut m: collections::HashMap[i64, i64]) throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   m[b.len()] = 1\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   lib::fill(m)\n\
         \x20   println(f\"{m.len()}\")\n}\n";
    let t = one("untrusted-lib", lib, app);
    assert!(!t.app.contains("TrustedMap"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **What a library reads elsewhere is not what it puts in a map**
/// (ADR-010 D9, `maps`, #581 step 6): `ping` reads a socket and touches no map,
/// so the ledger of `count_in` and the rest says `from(word)` and `from()`,
/// and a program with its own keys gets the fast hash.
#[test]
fn a_library_that_reads_a_socket_elsewhere_leaves_the_maps_to_the_program() {
    let lib = TALLY.replace(
        "fn main() {\n}\n",
        "pub fn ping() throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   println(f\"{b.len()}\")\n}\n\nfn main() {\n}\n",
    );
    let lib = lib.replace(
        "use std::collections\n",
        "use std::collections\nuse std::net\n",
    );
    let t = one("ping", &lib, FROM_ITS_OWN);
    assert!(t.app.contains("hash::Fast"), "{}", t.app);
    assert!(!t.app.contains("hash::Keyed"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A `keyed` function nobody calls costs the program nothing.**
#[test]
fn a_keyed_function_the_program_never_calls_leaves_it_fast() {
    let lib = TALLY.replace(
        "fn main() {\n}\n",
        "pub fn fill(mut m: collections::HashMap[i64, i64]) throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   m[b.len()] = 1\n}\n\nfn main() {\n}\n",
    );
    let lib = lib.replace(
        "use std::collections\n",
        "use std::collections\nuse std::net\n",
    );
    let t = one("uncalled", &lib, FROM_ITS_OWN);
    assert!(t.app.contains("hash::Fast"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A body that hands the map on says `keyed`**, and a trusted program that
/// calls it gets the keyed hash: the ledger is read, never the body.
#[test]
fn a_map_the_library_hands_on_keys_the_callers_map() {
    let lib = "use std::collections\n\n\
         pub fn pass(m: collections::HashMap[String, i64]) -> i64 {\n\
         \x20   return count(m)\n}\n\n\
         fn count(m: collections::HashMap[String, i64]) -> i64 {\n\
         \x20   return m.len()\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"a\", 1)\n\
         \x20   println(f\"{lib::pass(m)}\")\n}\n";
    let t = one("handed-on", lib, app);
    assert!(!t.app.contains("TrustedMap"), "{}", t.app);
    assert!(!t.app.contains("hash::Fast"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}
/// **A function that only reads its argument is `from(..)`**, though the
/// library reads a socket elsewhere: the program may choose the fast hash.
#[test]
fn a_function_that_only_reads_its_map_leaves_it_to_the_program() {
    let lib = "use std::net\nuse std::collections\n\n\
         pub fn size(m: ref collections::HashMap[String, i64]) -> i64 {\n\
         \x20   return m.len()\n}\n\n\
         pub fn ping() throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   println(f\"{b.len()}\")\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"a\", 1)\n\
         \x20   println(f\"{lib::size(m)}\")\n}\n";
    let t = one("reads-only", lib, app);
    assert!(t.app.contains("TrustedMap"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A body that inserts socket bytes keys that position, and a program with
/// a trusted file map gets the keyed hash through it.**
#[test]
fn a_function_that_inserts_socket_bytes_keys_the_trusted_programs_map() {
    let lib = "use std::net\nuse std::collections\n\n\
         pub fn fill(mut m: collections::HashMap[String, i64]) throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   m[f\"{b.len()}\"] = 1\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\nuse std::collections\nuse std::fs\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"a\", 1)\n\
         \x20   lib::fill(m)\n\
         \x20   println(f\"{m.len()}\")\n}\n";
    let t = one("inserts-socket", lib, app);
    assert!(!t.app.contains("TrustedMap"), "{}", t.app);
    assert!(!t.app.contains("hash::Fast"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **`handle(state, request)`**: the request fills the sessions and the
/// config is only read, so a program whose request comes from a file keeps
/// the fast hash for both, and the library is one function for either.
#[test]
fn a_struct_argument_keeps_the_fast_hash_where_only_one_map_is_filled() {
    let lib = "use std::collections\n\n\
         pub struct State { pub config: collections::HashMap[String, String], pub sessions: collections::HashMap[String, i64] }\n\n\
         pub fn state_new() -> State {\n\
         \x20   return State { config: collections::HashMap(), sessions: collections::HashMap() }\n}\n\n\
         pub fn handle(mut state: State, request: ref String) {\n\
         \x20   state.sessions[request.clone()] = 1\n\
         \x20   println(f\"{state.config.len()}\")\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\n\n\
         fn main() throws {\n\
         \x20   let mut s = lib::state_new()\n\
         \x20   lib::handle(s, \"a\")\n}\n";
    let t = one("handle", lib, app);
    assert!(t.app.contains("hash::Fast"), "{}", t.app);
    assert!(!t.app.contains("hash::Keyed"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A `pub` function has one parameter for each group of positions its body
/// joins** (Part III 17.1): `same` compares its two maps, so they are one
/// type; `merge` only reads one and fills the other, so they are two and a
/// caller may give each its own.
#[test]
fn a_function_declares_one_parameter_per_group_its_body_joins() {
    let lib = "use std::collections\n\n\
         pub fn same(a: ref collections::HashMap[String, i64], b: ref collections::HashMap[String, i64]) -> bool {\n\
         \x20   return a == b\n}\n\n\
         pub fn merge(mut a: collections::HashMap[String, i64], b: ref collections::HashMap[String, i64]) {\n\
         \x20   for k in b.keys() {\n\
         \x20       a[k.clone()] = 1\n\
         \x20   }\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut a = collections::HashMap()\n\
         \x20   let mut b = collections::HashMap()\n\
         \x20   a.insert(f\"a\", 1)\n\
         \x20   b.insert(f\"b\", 1)\n\
         \x20   println(f\"{lib::same(a, b)}\")\n\
         \x20   lib::merge(a, b)\n}\n";
    let t = one("groups", lib, app);
    let same = t.lib.find("fn same<").map(|at| &t.lib[at..]).unwrap_or("");
    let same = same.split('(').next().unwrap_or("");
    assert_eq!(same.matches("__S").count(), 1, "{same}\n{}", t.lib);
    assert!(
        same.contains("__S0: nikaia_std::hash::Seed") && !same.contains("__S1"),
        "{same}"
    );
    let merge = t.lib.find("fn merge<").map(|at| &t.lib[at..]).unwrap_or("");
    let merge = merge.split('(').next().unwrap_or("");
    assert!(
        merge.contains("__S0") && merge.contains("__S1"),
        "{merge}\n{}",
        t.lib
    );
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A type with methods**: the `impl` is generic over the type's parameters,
/// so a method serves a `Fast` value and a `Keyed` one, and the type compares.
const WITH_METHODS: &str = "use std::collections\n\n\
     pub struct Names { pub seen: collections::HashSet[String], pub ids: collections::HashMap[String, i64] }\n\n\
     impl Names {\n\
     \x20   pub fn add(ref mut self, word: ref String) {\n\
     \x20       self.seen.insert(word.clone())\n\
     \x20       self.ids[word.clone()] = 1\n\
     \x20   }\n\
     \x20   pub fn count(ref self) -> i64 {\n\
     \x20       return self.seen.len() + self.ids.len()\n\
     \x20   }\n\
     }\n\n\
     pub fn names_new() -> Names {\n\
     \x20   return Names { seen: collections::HashSet(), ids: collections::HashMap() }\n}\n\n\
     pub fn same(a: ref Names, b: ref Names) -> bool {\n\
     \x20   return a == b\n}\n\n\
     fn main() {\n\
     \x20   let mut n = names_new()\n\
     \x20   n.add(\"a\")\n\
     \x20   println(f\"{n.count()} {same(n, n)}\")\n}\n";

#[test]
fn a_type_with_methods_serves_both_choices() {
    let own = "use lib\nuse std::collections\n\n\
         fn show(n: ref lib::Names) -> bool {\n\
         \x20   return lib::same(n, n)\n}\n\n\
         fn main() throws {\n\
         \x20   let n = lib::Names { seen: collections::HashSet(), ids: collections::HashMap() }\n\
         \x20   let m = lib::names_new()\n\
         \x20   println(f\"{show(n)} {lib::same(n, m)}\")\n}\n";
    let socket = "use lib\nuse std::net\nuse std::collections\n\n\
         fn show(n: ref lib::Names) -> i64 {\n\
         \x20   return n.count()\n}\n\n\
         fn main() throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   let mut n = lib::Names { seen: collections::HashSet(), ids: collections::HashMap() }\n\
         \x20   n.add(f\"{b.len()}\")\n\
         \x20   let m = lib::names_new()\n\
         \x20   println(f\"{show(n)} {lib::same(n, m)}\")\n}\n";
    let (dir, lib, apps) = two("methods", WITH_METHODS, &[("own", own), ("socket", socket)]);
    assert!(
        lib.contains(
            "impl<__S0: nikaia_std::hash::Seed, __S1: nikaia_std::hash::Seed> Names<__S0, __S1>"
        ),
        "{lib}"
    );
    compiles(&dir, &lib, &apps[0], "own");
    compiles(&dir, &lib, &apps[1], "socket");
    assert!(
        apps[0].contains("names_new::<nikaia_std::hash::Fast, nikaia_std::hash::Fast>"),
        "{}",
        apps[0]
    );
    assert!(
        apps[1].contains("names_new::<nikaia_std::hash::Keyed, nikaia_std::hash::Keyed>"),
        "{}",
        apps[1]
    );
    done(&dir);
}

/// **The library's own program uses its items**, trusted and not.
#[test]
fn a_library_uses_its_own_generic_items() {
    let own = "use std::collections\n\n\
         pub struct Tally { pub counts: collections::HashMap[String, i64] }\n\n\
         pub fn tally_new() -> Tally {\n\
         \x20   return Tally { counts: collections::HashMap() }\n}\n\n\
         pub fn count_in(mut t: Tally, word: ref String) {\n\
         \x20   t.counts[word.clone()] = 1\n}\n\n\
         fn total(t: ref Tally) -> i64 {\n\
         \x20   return t.counts.len()\n}\n\n\
         fn main() {\n\
         \x20   let mut t = tally_new()\n\
         \x20   count_in(t, \"a\")\n\
         \x20   println(f\"{total(t)}\")\n}\n";
    let socket = own.replace(
        "fn main() {\n",
        "fn main() throws {\n\x20   let mut c = net::connect(\"127.0.0.1:1\")\n\x20   let b = c.read()\n",
    )
    .replace("use std::collections\n", "use std::collections\nuse std::net\n")
    .replace("count_in(t, \"a\")", "count_in(t, f\"{b.len()}\")");
    for (name, source) in [("own", own), ("socket", &socket[..])] {
        let dir = common::scratch_dir(&format!("map-seeds-itself-{name}"));
        std::fs::create_dir_all(dir.join("lib/src")).expect("lib");
        std::fs::write(
            dir.join("lib/nikaia.toml"),
            "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
        )
        .expect("a manifest");
        std::fs::write(dir.join("lib/src/main.nika"), source).expect("a source");
        let rust = lower(&dir, "lib");
        let path = dir.join("program.rs");
        std::fs::write(&path, &rust).expect("write");
        let built = common::compile(
            &path,
            &[
                "--crate-type",
                "bin",
                "-o",
                &dir.join("program-bin").to_string_lossy(),
            ],
        );
        assert!(
            built.status.success(),
            "{name}:\n{}\n{rust}",
            String::from_utf8_lossy(&built.stderr)
        );
        done(&dir);
    }
}

// ---------------------------------------------------------------------------
// Shapes of a library, each compiled with `rustc` on its own and then against
// one program of each choice. A shape that still reaches `rustc` is an
// `#[ignore]`d test that names why (Part III C.1).
// ---------------------------------------------------------------------------

const FILE_APP: &str = "use lib\nfn main() {\n}\n";

/// A library of this source, handed to `rustc`.
fn library_compiles(purpose: &str, lib: &str) {
    let t = one(purpose, lib, FILE_APP);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A private helper takes the type** a `pub` function takes: the helper is
/// generic over the same parameters.
#[test]
fn a_private_helper_given_a_library_type_serves_its_callers() {
    library_compiles(
        "helper",
        "use std::collections\n\n\
         pub struct Tally { pub counts: collections::HashMap[String, i64] }\n\n\
         fn total(t: ref Tally) -> i64 {\n    return t.counts.len()\n}\n\n\
         pub fn size(t: ref Tally) -> i64 {\n    return total(t)\n}\n\n\
         fn main() {\n}\n",
    );
}

/// **A map made by a `let` and stored in a field.**
#[test]
fn a_let_map_stored_in_a_library_field() {
    library_compiles(
        "let-stored",
        "use std::collections\n\n\
         pub struct Tally { pub counts: collections::HashMap[String, i64] }\n\n\
         pub fn tally_of(word: ref String) -> Tally {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m[word.clone()] = 1\n\
         \x20   return Tally { counts: m }\n}\n\n\
         fn main() {\n}\n",
    );
}

/// **A method that takes a map of its own.**
#[test]
fn a_method_with_a_map_parameter() {
    library_compiles(
        "method-map",
        "use std::collections\n\n\
         pub struct Tally { pub counts: collections::HashMap[String, i64] }\n\n\
         impl Tally {\n\
         \x20   pub fn absorb(ref mut self, other: collections::HashMap[String, i64]) {\n\
         \x20       for (k, v) in other {\n\
         \x20           self.counts[k.clone()] = v\n\
         \x20       }\n\
         \x20   }\n\
         }\n\n\
         fn main() {\n}\n",
    );
}

/// **A variant that holds a map.**
#[test]
fn an_enum_variant_with_a_map() {
    library_compiles(
        "enum-map",
        "use std::collections\n\n\
         pub enum Entry { Empty, Counted(collections::HashMap[String, i64]) }\n\n\
         pub fn counted() -> Entry {\n    return Entry::Counted(collections::HashMap())\n}\n\n\
         fn main() {\n}\n",
    );
}

/// **A generic function whose result is a map no argument fixes.**
#[test]
fn a_generic_function_returning_a_map() {
    let lib = "use std::collections\n\n\
         pub fn empty_for[T](x: T) -> collections::HashMap[String, i64] {\n\
         \x20   return collections::HashMap()\n}\n\n\
         fn main() {\n}\n";
    let app =
        "use lib\n\nfn main() {\n    let m = lib::empty_for(1)\n    println(f\"{m.len()}\")\n}\n";
    let t = one("generic-result", lib, app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A type with a type parameter of its own and a map.**
#[test]
fn a_generic_type_with_a_map() {
    library_compiles(
        "generic-type",
        "use std::collections\n\n\
         pub struct Bag[T] { pub items: Vec[T], pub counts: collections::HashMap[String, i64] }\n\n\
         pub fn bag_new[T]() -> Bag[T] {\n\
         \x20   return Bag { items: [], counts: collections::HashMap() }\n}\n\n\
         fn main() {\n}\n",
    );
}

/// **A map inside a list, and a type that holds another.**
#[test]
fn nested_positions() {
    library_compiles(
        "nested",
        "use std::collections\n\n\
         pub struct Inner { pub counts: collections::HashMap[String, i64] }\n\
         pub struct Outer { pub inner: Inner, pub rows: Vec[collections::HashSet[i64]] }\n\n\
         pub fn outer_new() -> Outer {\n\
         \x20   return Outer { inner: Inner { counts: collections::HashMap() }, rows: [] }\n}\n\n\
         pub fn rows_of(o: ref Outer) -> i64 {\n    return o.rows.len() + o.inner.counts.len()\n}\n\n\
         fn main() {\n}\n",
    );
}

/// **A method of a trait is a barrier** (D9, step 7): its signature and its
/// implementations' are the keyed map, whatever the package, and a `pub`
/// function that hands its map to one has that map fixed keyed too - a hidden
/// parameter could not be shown to be std's hasher.
const SINK: &str = "use std::collections\n\n\
     pub trait Sink {\n    fn put(ref mut self, m: ref collections::HashMap[String, i64])\n}\n\n\
     pub struct Count { pub n: i64 }\n\n\
     impl Sink for Count {\n    fn put(ref mut self, m: ref collections::HashMap[String, i64]) {\n        self.n = m.len()\n    }\n}\n\n\
     pub fn feed(mut k: Count, m: ref collections::HashMap[String, i64]) {\n    k.put(m)\n}\n\n\
     fn main() {\n}\n";

#[test]
fn a_trait_method_with_a_map_is_keyed_whatever_the_package() {
    let app = "use lib\nuse std::net\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"{b.len()}\", 1)\n\
         \x20   let mut k = lib::Count { n: 0 }\n\
         \x20   lib::feed(k, m)\n}\n";
    let t = one("trait", SINK, app);
    assert!(!t.lib.contains("TrustedMap"), "{}", t.lib);
    assert!(!t.lib.contains("fn feed<"), "{}", t.lib);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **And a trusted program that calls it keeps no fast map across it.**
#[test]
fn a_trusted_program_hands_a_trait_method_the_keyed_map() {
    let app = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"{1}\", 1)\n\
         \x20   let mut k = lib::Count { n: 0 }\n\
         \x20   lib::feed(k, m)\n}\n";
    let t = one("trait-trusted", SINK, app);
    assert!(!t.app.contains("TrustedMap"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A trait implemented in a third package that reads a socket**, whose own
/// `pub` items hold no map: the map the program hands to `put` is hashed by a
/// body the program cannot see, so the program stays keyed.
#[test]
fn a_trait_implemented_in_a_third_package_that_reads_a_socket_keys_the_program() {
    let dir = common::scratch_dir("map-seeds-third");
    let write = |name: &str, deps: &str, source: &str| {
        std::fs::create_dir_all(dir.join(name).join("src")).expect("the package");
        std::fs::write(
            dir.join(name).join("nikaia.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[dependencies]\n{deps}"),
        )
        .expect("a manifest");
        std::fs::write(dir.join(name).join("src/main.nika"), source).expect("a source");
    };
    write(
        "api",
        "",
        "use std::collections\n\n\
         pub trait Sink {\n    fn put(ref mut self, m: ref collections::HashMap[String, i64])\n}\n\n\
         fn main() {\n}\n",
    );
    write(
        "third",
        "api = { path = \"../api\" }\n",
        "use api\nuse std::net\nuse std::collections\n\n\
         pub struct Count { pub n: i64 }\n\n\
         impl api::Sink for Count {\n\
         \x20   fn put(ref mut self, m: ref collections::HashMap[String, i64]) throws {\n\
         \x20       let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20       let b = c.read()\n\
         \x20       self.n = m.len() + b.len()\n\
         \x20   }\n}\n\n\
         fn main() {\n}\n",
    );
    write(
        "app",
        "api = { path = \"../api\" }\nthird = { path = \"../third\" }\n",
        "use api\nuse third\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"a\", 1)\n\
         \x20   let mut k = third::Count { n: 0 }\n\
         \x20   k.put(m)\n}\n",
    );
    let app = lower(&dir, "app");
    assert!(!app.contains("TrustedMap"), "{app}");
    assert!(!app.contains("hash::Fast"), "{app}");
    done(&dir);
}

/// **A binding for another language takes no map**: the refusal is Nikaia's,
/// in Nikaia's words, and not a Rust error about a file nobody wrote (C.1).
#[test]
fn a_c_binding_with_a_map_is_refused_in_nikaia_words() {
    let run = |source: &str| {
        let dir = common::scratch_dir("map-seeds-cexport");
        std::fs::create_dir_all(dir.join("src")).expect("src");
        std::fs::write(
            dir.join("nikaia.toml"),
            "[package]\nname = \"cl\"\nversion = \"0.1.0\"\n\n[build]\nartifact = \"c-library\"\n",
        )
        .expect("a manifest");
        std::fs::write(dir.join("src/main.nika"), source).expect("a source");
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
            .args([
                "lower",
                &dir.join("src/main.nika").to_string_lossy(),
                "--no-cache",
            ])
            .output()
            .expect("the binary runs");
        done(&dir);
        String::from_utf8_lossy(&out.stderr).to_string()
    };
    let param = run(
        "use std::collections\n\npub extern fn count(m: collections::HashMap[String, i64]) -> i64 {\n    return m.len()\n}\n\nfn main() {\n}\n",
    );
    assert!(param.contains("does not export yet"), "{param}");
    let handle = run(
        "use std::collections\n\npub struct Tally { pub counts: collections::HashMap[String, i64] }\n\npub extern fn tally_len(t: ref Tally) -> i64 {\n    return t.counts.len()\n}\n\nfn main() {\n}\n",
    );
    assert!(handle.contains("does not export yet"), "{handle}");
}

/// **Shapes step 5 left open, each handed to rustc** (C.1): a method whose
/// result alone holds a map (a method call takes no turbofish), a function
/// of a lifetime-generic signature whose result alone holds one.
const MAKERS: &str = "use std::collections\n\n\
     pub struct Maker { pub n: i64 }\n\n\
     impl Maker {\n\
     \x20   pub fn build(ref self) -> collections::HashMap[String, i64] {\n\
     \x20       return collections::HashMap()\n\
     \x20   }\n}\n\n\
     pub fn tally_of(text: ref String) -> collections::HashMap[String, i64] {\n\
     \x20   return collections::HashMap()\n}\n\n\
     fn main() {\n}\n";

#[test]
fn a_method_with_a_result_only_map_compiles_in_both_programs() {
    let own = "use lib\n\nfn main() throws {\n\
         \x20   let k = lib::Maker { n: 1 }\n\
         \x20   let m = k.build()\n\
         \x20   println(f\"{m.len()}\")\n}\n";
    let socket = "use lib\nuse std::net\n\nfn main() throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   let k = lib::Maker { n: 1 }\n\
         \x20   let mut m = k.build()\n\
         \x20   m.insert(f\"{b.len()}\", 1)\n\
         \x20   println(f\"{m.len()}\")\n}\n";
    let (dir, lib, apps) = two("maker-method", MAKERS, &[("own", own), ("socket", socket)]);
    compiles(&dir, &lib, &apps[0], "own");
    compiles(&dir, &lib, &apps[1], "socket");
    done(&dir);
}

#[test]
fn a_function_with_a_view_and_a_result_only_map_compiles_in_both_programs() {
    let own = "use lib\n\nfn main() throws {\n\
         \x20   let text = \"a b\"\n\
         \x20   let m = lib::tally_of(text)\n\
         \x20   println(f\"{m.len()}\")\n}\n";
    let socket = "use lib\nuse std::net\n\nfn main() throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   let mut m = lib::tally_of(\"a b\")\n\
         \x20   m.insert(f\"{b.len()}\", 1)\n\
         \x20   println(f\"{m.len()}\")\n}\n";
    let (dir, lib, apps) = two("maker-view", MAKERS, &[("own", own), ("socket", socket)]);
    compiles(&dir, &lib, &apps[0], "own");
    compiles(&dir, &lib, &apps[1], "socket");
    done(&dir);
}

/// **A described Rust crate's signature is a barrier** (D9): a function that
/// takes `&HashMap<K, V>` takes std's, so a program that calls it keeps no fast
/// map across the call. The description is read the way a project's is.
#[test]
fn a_described_crates_map_function_keys_the_callers_map() {
    use nikaia::contracts::LedgerOps;
    let dir = common::scratch_dir("map-seeds-described");
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(
        dir.join("nikaia.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");
    let source = "use std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"{1}\", 1)\n\
         \x20   println(f\"{fremd::count(m)}\")\n}\n";
    std::fs::write(dir.join("src/main.nika"), source).expect("a source");
    std::fs::write(
        dir.join("fremd.contracts"),
        "[fn.\"fremd::count\"]\npub = true\nsignature = \"(m: ref collections::HashMap[String, i64]) -> i64\"\n",
    )
    .expect("a description");
    let mut program = nikaia::comptime_run::defaults_compiled_in(Some(&common::workshop()), || {
        nikaia::modules::Program::read_with(&dir.join("src/main.nika"), &[])
    })
    .expect("the program reads");
    program.described =
        nikaia::contracts::Ledger::read_beside(&dir.join("fremd.contracts")).expect("it reads");
    let rust = program
        .emit_reading(Default::default(), &common::reads())
        .expect("the program lowers")
        .rust;
    assert!(!rust.contains("TrustedMap"), "{rust}");
    done(&dir);
}

/// **One parameter per map position of a type, in field order**: two maps of
/// one written type in one `pub struct` are two parameters, and a `pub`
/// function that takes the struct carries both (C.1: this was a rustc error).
#[test]
fn two_maps_of_one_type_in_a_pub_struct_are_two_parameters() {
    let source = "use std::collections\n\n\
         pub struct Pair {\n    pub a: collections::HashMap[String, i64],\n    pub b: collections::HashMap[String, i64],\n}\n\n\
         pub fn bump(mut p: Pair, w: ref String) {\n    p.a[w.clone()] = 1\n    p.b[w.clone()] = 2\n}\n\n\
         fn main() {\n\
         \x20   let mut p = Pair { a: collections::HashMap(), b: collections::HashMap() }\n\
         \x20   bump(p, \"x\")\n\
         \x20   print(f\"{p.a.len()} {p.b.len()}\")\n}\n";
    let dir = common::scratch_dir("map-seeds-pair");
    std::fs::create_dir_all(dir.join("lib/src")).expect("lib");
    std::fs::write(
        dir.join("lib/nikaia.toml"),
        "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");
    std::fs::write(dir.join("lib/src/main.nika"), source).expect("a source");
    let rust = lower(&dir, "lib");
    let path = dir.join("pair.rs");
    std::fs::write(&path, &rust).expect("write");
    let built = common::compile(
        &path,
        &[
            "--crate-type",
            "bin",
            "-o",
            &dir.join("pair-bin").to_string_lossy(),
        ],
    );
    assert!(
        built.status.success(),
        "{}\n{rust}",
        String::from_utf8_lossy(&built.stderr)
    );
    done(&dir);
}

// ---------------------------------------------------------------------------
// Shapes of #581 step 6 that a position-by-position grouping meets.
// ---------------------------------------------------------------------------

/// **A `pub` method whose `self` map is handed to a barrier.** The `impl`'s
/// parameters belong to the type, so one method cannot fix its own position to
/// std's hasher; the type declares no parameter then, its maps are the keyed
/// one, and a program that builds it is no more trusted than that. (It was a
/// `rustc` error: expected `HashMap<_, _, RandomState>`, found `__S0`.)
#[test]
fn a_method_that_hands_its_self_map_to_a_barrier_keys_the_type() {
    let lib = "use std::collections\n\n\
         pub trait Sink {\n    fn put(ref mut self, m: ref collections::HashMap[String, i64])\n}\n\n\
         pub struct Count { pub n: i64 }\n\n\
         impl Sink for Count {\n    fn put(ref mut self, m: ref collections::HashMap[String, i64]) {\n        self.n = m.len()\n    }\n}\n\n\
         pub struct Names { pub ids: collections::HashMap[String, i64] }\n\n\
         impl Names {\n    pub fn flush(ref self, mut k: Count) {\n        k.put(self.ids)\n    }\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut n = lib::Names { ids: collections::HashMap() }\n\
         \x20   n.ids.insert(f\"a\", 1)\n\
         \x20   n.flush(lib::Count { n: 0 })\n}\n";
    let t = one("self-barrier", lib, app);
    assert!(!t.lib.contains("Names<__S"), "{}", t.lib);
    assert!(!t.app.contains("TrustedMap"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A method that hands a map it was given to a barrier** has that position
/// fixed keyed, as a function does (step 7).
#[test]
fn a_method_that_hands_its_argument_map_to_a_barrier_is_keyed() {
    let lib = "use std::collections\n\n\
         pub trait Sink {\n    fn put(ref mut self, m: ref collections::HashMap[String, i64])\n}\n\n\
         pub struct Count { pub n: i64 }\n\n\
         impl Sink for Count {\n    fn put(ref mut self, m: ref collections::HashMap[String, i64]) {\n        self.n = m.len()\n    }\n}\n\n\
         pub struct Feeder { pub n: i64 }\n\n\
         impl Feeder {\n    pub fn feed(ref self, m: ref collections::HashMap[String, i64], mut k: Count) {\n        k.put(m)\n    }\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let mut m = collections::HashMap()\n\
         \x20   m.insert(f\"a\", 1)\n\
         \x20   let f = lib::Feeder { n: 0 }\n\
         \x20   f.feed(m, lib::Count { n: 0 })\n}\n";
    let t = one("arg-barrier", lib, app);
    assert!(!t.app.contains("TrustedMap"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A private struct holding a `pub` one** is filled with the package's
/// answer, not its class's: the program compiles, and in a program that reads
/// a socket elsewhere the `pub` struct is keyed whatever keys fill it.
#[test]
fn a_private_struct_that_holds_a_pub_struct_compiles_in_both_programs() {
    let lib = "use std::collections\n\n\
         pub struct Tally { pub counts: collections::HashMap[String, i64] }\n\n\
         struct Holder { t: Tally }\n\n\
         fn hold() -> Holder {\n\
         \x20   return Holder { t: Tally { counts: collections::HashMap() } }\n}\n\n\
         pub fn total() -> i64 {\n\
         \x20   let h = hold()\n\
         \x20   return h.t.counts.len()\n}\n\n\
         fn main() {\n}\n";
    library_compiles("holder", lib);
    let socket = "use std::net\n\n".to_string() + lib;
    let socket = socket.replace(
        "fn main() {\n}\n",
        "fn main() throws {\n\
         \x20   let mut c = net::connect(\"127.0.0.1:1\")\n\
         \x20   let b = c.read()\n\
         \x20   println(f\"{b.len()} {total()}\")\n}\n",
    );
    let t = one("holder-socket", &socket, FILE_APP);
    assert!(!t.lib.contains("TrustedMap"), "{}", t.lib);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}

/// **A grammar rule's type, by itself** (`Report<'a>`): the rule's result and
/// the fold's accumulator are written with what the package fills a mention
/// with, not with the type's `Keyed` default. A trusted program that builds a
/// `Report` of its own and merges the parsed one into it mixed `Fast` and
/// `Keyed` before (a `rustc` error: *expected `Report<'_>`, found
/// `Report<'_, RandomState>`*).
#[test]
fn a_grammar_rule_type_takes_the_hasher_the_package_fills() {
    let example = include_str!("../../../examples/access-log/src/main.nika");
    let source = example
        .replace(
            "fn main() throws {",
            "fn make() -> Report {\n    return Report()\n}\n\nfn main() throws {",
        )
        .replace(
            "    println(f\"{report.requests} requests, {report.failures} failed\")",
            "    let mut mine = make()\n    mine.merge(report.clone())\n    println(f\"{mine.requests} requests, {report.failures} failed\")",
        );
    let dir = common::scratch_dir("map-seeds-grammar");
    std::fs::create_dir_all(dir.join("gram/src")).expect("the package");
    std::fs::write(
        dir.join("gram/nikaia.toml"),
        "[package]\nname = \"gram\"\nversion = \"0.1.0\"\n",
    )
    .expect("a manifest");
    std::fs::write(dir.join("gram/src/main.nika"), &source).expect("a source");
    let rust = lower(&dir, "gram");
    assert!(
        rust.contains("-> Report<'a, nikaia_std::hash::Fast>"),
        "{rust}"
    );
    let path = dir.join("gram.rs");
    std::fs::write(&path, &rust).expect("write");
    let built = common::compile(
        &path,
        &[
            "--crate-type",
            "bin",
            "-o",
            &dir.join("gram-bin").to_string_lossy(),
        ],
    );
    assert!(
        built.status.success(),
        "{}\n{rust}",
        String::from_utf8_lossy(&built.stderr)
    );
    done(&dir);
}

/// **A method that joins two maps of its own type** (`self.a == self.b`): the
/// type declares one parameter for each, and no signature can say that two of
/// the `impl`'s parameters are one, so the type's maps are the keyed map.
#[test]
fn a_method_that_joins_two_maps_of_its_type_keys_the_type() {
    let lib = "use std::collections\n\n\
         pub struct Pair { pub a: collections::HashSet[String], pub b: collections::HashSet[String] }\n\n\
         impl Pair {\n\
         \x20   pub fn alike(ref self) -> bool {\n\
         \x20       return self.a == self.b\n\
         \x20   }\n}\n\n\
         fn main() {\n}\n";
    let app = "use lib\nuse std::collections\n\n\
         fn main() throws {\n\
         \x20   let p = lib::Pair { a: collections::HashSet(), b: collections::HashSet() }\n\
         \x20   println(f\"{p.alike()}\")\n}\n";
    let t = one("join-own", lib, app);
    assert!(!t.lib.contains("Pair<__S"), "{}", t.lib);
    assert!(!t.app.contains("TrustedSet"), "{}", t.app);
    compiles(&t.dir, &t.lib, &t.app, "app");
    done(&t.dir);
}
