//! The sysroot, and the two promises it turns into obligations (ADR-002 D4).
//!
//! `std` ships as sources with its Nikaia half already lowered. That buys a
//! project a build graph with nothing of the compiler in it, and it costs two
//! things that have to be checked rather than hoped for:
//!
//! * the committed `.rs` must be what this compiler lowers the `.nika` to, or a
//!   reviewer is reading a file that does not describe the source beside it;
//! * the lowering happens at **one** setting of the build switches, which is
//!   only sound while nothing in those files is switch-sensitive.

use std::collections::BTreeMap;
use std::path::Path;

use nikaia::emit::{self, Build};
use nikaia::sysroot::{self, Sysroot};

/// What `cargo fmt`'s equivalent is for a generated file: the committed bytes
/// are the compiler's output, or the build is red.
///
/// **This is why the pre-lowered Rust is committed rather than produced by a
/// release step.** A fresh checkout has to build with `cargo test --workspace`
/// and no extra steps, and the only other way for `crates/nikaia-std` to have
/// its `text.rs` at that moment is the build dependency on the compiler this
/// decision removed. Committing it puts a generated file in diffs, which is a
/// real cost; this test is what stops that file from being able to lie.
#[test]
fn the_committed_rust_is_what_this_compiler_lowers() {
    let sysroot = Sysroot::resolve();
    let modules = sysroot.std_modules().expect("std's Nikaia modules");
    assert!(
        !modules.is_empty(),
        "std has a Nikaia half, and this test is about it (ADR-014 D1)"
    );
    // **And the toolchain's own**, which rides the same release step
    // ([ADR-290](../../../docs/specification/adr/adr-290.md) D16): the `.rs` is
    // committed, so a change to the `.nika` that is not re-lowered is a silent
    // divergence, and that is as true of `tools/` as of `std`. The tools are
    // one package, lowered into one file (ADR-294).
    let tools = sysroot.tools_dir();
    assert!(
        !sysroot
            .tool_modules()
            .expect("the toolchain's Nikaia modules")
            .is_empty(),
        "`src/tools` holds the reading half of `nikaia describe` (ADR-290 D13)"
    );
    let expected = sysroot::lower_tools(&tools).expect("the toolchain's Nikaia lowers");
    let at = sysroot::tools_lowered_path(&tools);
    let committed = std::fs::read_to_string(&at)
        .unwrap_or_else(|e| panic!("{} is not committed: {e}", at.display()));
    assert_eq!(
        committed,
        expected,
        "{} has drifted from what this compiler lowers {} to. \
         Run `cargo run -p nikaia -- lower-std` and commit the result.",
        at.display(),
        tools.display(),
    );

    for nika in modules {
        let expected = sysroot::lower_std_module(&nika).expect("std's Nikaia half lowers");
        let committed = std::fs::read_to_string(sysroot::lowered_path(&nika))
            .unwrap_or_else(|e| panic!("{} has no committed Rust beside it: {e}", nika.display()));
        assert_eq!(
            committed,
            expected,
            "{} has drifted from what this compiler lowers {} to. \
             Run `cargo run -p nikaia -- lower-std` and commit the result.",
            sysroot::lowered_path(&nika).display(),
            nika.display(),
        );
    }
}

/// The constraint ADR-002 D4 states, enforced, and the line ADR-363 draws
/// around it.
///
/// `std`'s Nikaia half is lowered once, at `Build::default()`, and linked into
/// programs built at either setting of `user_parallelism`. That is sound only
/// while nothing in those files lowers differently per switch - and `Shared` is
/// exactly what would break it: ADR-312 D10 makes every count the cheap one at
/// `no`, so a `Shared` here would be lowered once with a count that cannot cross
/// a thread and linked into a program built at `yes`. The day one appears in a
/// `.nika` file here, this test goes red instead of that program going quiet.
///
/// It is written as a comparison rather than as a search for `Shared`, so it
/// catches whatever else the emitter comes to write per switch - which is the
/// third answer this one type has had, and the reason the guard is a byte
/// comparison and not a list of names.
///
/// **The toolchain's own package is not compared** (ADR-363 D1): no program
/// links it, only the compiler runs it, and it is lowered at `yes`, which is
/// correct at one thread too. What is checked for it instead is that the
/// committed file is the package lowered at `yes`.
#[test]
fn stds_nikaia_half_lowers_the_same_at_both_switches() {
    let sysroot = Sysroot::resolve();
    let sequential = Build::parse("x86_64-linux", "no", "yes").expect("a switch that exists");
    let concurrent = Build::parse("x86_64-linux", "yes", "yes").expect("a switch that exists");

    let tools = sysroot.tools_dir();
    let at = sysroot::tools_lowered_path(&tools);
    assert_eq!(
        std::fs::read_to_string(&at)
            .unwrap_or_else(|e| panic!("{} is not committed: {e}", at.display())),
        sysroot::lower_tools_at(&tools, concurrent).expect("the tools lower at `yes`"),
        "{} is not the toolchain's package lowered at `user_parallelism = yes` (ADR-363 D1).",
        at.display(),
    );
    for nika in sysroot.std_modules().expect("std's Nikaia modules") {
        let source = std::fs::read_to_string(&nika).expect("the source reads");
        let parsed = nikaia::parser::parse_to_ast(&source).expect("std's Nikaia half parses");
        let at_no = emit::emit_program(&parsed, sequential).expect("lowers at `no`");
        let at_yes = emit::emit_program(&parsed, concurrent).expect("lowers at `yes`");
        assert_eq!(
            at_no.rust,
            at_yes.rust,
            "{} lowers differently at the two settings of `user_parallelism`, \
             and it is lowered once for both (ADR-002 D4). Nothing switch-sensitive \
             may live in std's Nikaia half until that decision is revisited.",
            nika.display(),
        );
    }
}

/// The defect measurement made visible, as a structural guard.
///
/// `nikaia-std` had the compiler as a build dependency, so Cargo built the
/// compiler library again inside every project's `target/` while the installed
/// compiler was running - 58 of a hello-world's 103 resolved packages existed
/// only for that. Nothing in this crate's manifest may put it back.
#[test]
fn std_needs_nothing_but_rustc_to_build() {
    let std_dir = Sysroot::resolve().std_dir();
    let manifest = std::fs::read_to_string(std_dir.join("Cargo.toml")).expect("std's manifest");

    assert!(
        !manifest.contains("[build-dependencies]"),
        "std has no build dependencies, which is the whole of ADR-002 D4's \
         package-count claim:\n{manifest}"
    );
    assert!(
        !std_dir.join("build.rs").exists(),
        "std has no build script: the Nikaia half is lowered at release time \
         (`nikaia lower-std`), not when the crate is built"
    );
}

/// The ledger travels with the compiler, not with `std` (ADR-005 D8).
///
/// Ledger stability across toolchain versions is explicitly not required, so a
/// `std` that could be paired with a different compiler would let the ledger
/// describe a compiler that is not there. The compiler answers from the copy
/// `include_str!` baked in, and this is the statement that it is that copy.
#[test]
fn the_ledger_the_compiler_answers_from_is_its_own() {
    let shipped = std::fs::read_to_string(Sysroot::resolve().std_dir().join("std.contracts"))
        .expect("the sysroot carries std's ledger as the source of truth");
    assert_eq!(
        shipped,
        nikaia::contracts::STD,
        "the compiler answers from its own baked-in copy; the file in the sysroot \
         is the source it was built from and may not be able to differ from it"
    );
}

/// D7 for the second store: what separates one compiled `std` from another.
///
/// The codegen table is in because hardware optimisation is the one axis where
/// `std` matters more than the compiler - `std`'s code runs inside every user
/// program, while the compiler's speed only costs build time - so two such builds
/// have to coexist rather than evict each other.
#[test]
fn a_different_codegen_table_is_a_different_compiled_std() {
    let sysroot = Sysroot::resolve();
    let plain = sysroot::Codegen::new(&BTreeMap::new(), "unwind");
    let tuned = sysroot::Codegen::new(
        &BTreeMap::from([("opt-level".to_string(), toml::Value::Integer(3))]),
        "unwind",
    );
    let a = sysroot.rlib_cache("x86_64-linux", &plain, "yes");
    let b = sysroot.rlib_cache("x86_64-linux", &tuned, "yes");
    assert_ne!(a, b);
    assert_eq!(
        a.parent(),
        b.parent(),
        "both are entries of one cache, distinguished by the key and not by where \
         they were put"
    );
}

/// `NIKAIA_SYSROOT` is the development override, and it names a sysroot rather
/// than a crate directory - which is what `NIKAIA_STD_PATH` got wrong.
#[test]
fn the_override_names_a_sysroot() {
    let sysroot = Sysroot::new(Path::new("/opt/nikaia/lib"));
    assert_eq!(
        sysroot.std_dir(),
        Path::new("/opt/nikaia/lib").join("nikaia-std")
    );
}

/// **A sysroot module is checked before it is lowered**, which it was not.
///
/// [Part III C.1](../../../docs/specification/30-nikaia-tooling.md) is what
/// that cost, and it was paid: `crates/nikaia-std/src/tools/rust.nika` had a
/// grammar action calling a function that could pause — which `NK2209` exists
/// to refuse, in one sentence, on the `.nika` line — and because
/// `lower_std_module` went straight from the parser to the emitter, what a
/// reader saw instead was `rustc` on the generated file:
///
/// ```text
/// error[E0728]: `await` is only allowed inside `async` functions and blocks
///    --> crates/nikaia-std/src/tools/rust.rs:193:33
/// ```
///
/// A module of `std` is written by the same people who write the compiler,
/// which is exactly why nobody noticed: the checks a *program* gets are the
/// ones this half was going without.
#[test]
fn a_sysroot_module_is_checked_before_it_is_lowered() {
    let root = std::env::temp_dir().join(format!(
        "nikaia-sysroot-check-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let src = root.join("nikaia-std/src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).expect("a sysroot to work in");

    // A grammar whose action calls something that can pause. A grammar's
    // action may not pause (ADR-296 D35), because the generated parser is an
    // ordinary function. The probe used to reach the pause through a method on
    // what a **`match` arm** bound, which resolved to nothing while a pattern
    // binding had no type; since issue #168 such a binding is typed
    // from its variant and resolves, so `flat` pauses where it says it does.
    let module = src.join("probe.nika");
    std::fs::write(
        &module,
        "use std::time\n\
         \n\
         enum Piece {\n\
             Many(Vec[ref String]),\n\
             Nothing,\n\
         }\n\
         \n\
         grammar G {\n\
             entry rule all -> Vec[ref String] = ps:piece* { flat(ps) }\n\
             rule piece -> Piece = s:raw_ident { Piece::Nothing } | any { Piece::Nothing }\n\
         }\n\
         \n\
         fn flat(ps: Vec[Piece]) -> Vec[ref String] {\n\
             time::sleep(1.millis())\n\
             let mut out = Vec()\n\
             for p in ps.drain() {\n\
                 match p {\n\
                     Piece::Many(many) => { for one in many.drain() { out.push(one) } }\n\
                     Piece::Nothing => { }\n\
                 }\n\
             }\n\
             return out\n\
         }\n",
    )
    .expect("write the module");

    let refused = sysroot::lower_std_module(&module).expect_err("a module that cannot lower");
    let said = format!("{refused:#}");
    let _ = std::fs::remove_dir_all(&root);

    assert!(
        said.contains("NK2209"),
        "the refusal is this compiler's, by code: {said}"
    );
    assert!(
        said.contains("probe.nika"),
        "and names the file somebody wrote: {said}"
    );
    assert!(
        !said.contains("E0728") && !said.contains("await"),
        "and never `rustc`'s about the file nobody wrote: {said}"
    );
}

/// **A tool reads the Rust it is handed, described**
/// ([ADR-294](../../../docs/specification/adr/adr-294.md) D9.3): a module in
/// `tools/` is checked and lowered against `tools/described.contracts` too, so
/// a `winnow_grammar::Symbol` copies, compares (and equates: `equates = true`,
/// ADR-341 D4) and `resolve` does not pause. A module of `std` itself is not: a program's `std` names no crate of
/// the toolchain's.
#[test]
fn a_tool_reads_the_names_of_the_tree_as_described() {
    let root = std::env::temp_dir().join(format!(
        "nikaia-sysroot-described-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let source = "pub struct Named { pub name: winnow_grammar::Symbol }\n\
                  \n\
                  pub fn same(a: Named, b: Named) -> bool {\n\
                  \x20   return a == b\n\
                  }\n\
                  \n\
                  pub fn is_main(names: ref winnow_grammar::InternerContext, n: Named) -> bool {\n\
                  \x20   return names.resolve(n.name) == \"main\"\n\
                  }\n\
                  \n\
                  pub fn named(names: ref winnow_grammar::InternerContext, name: winnow_grammar::Symbol) -> bool {\n\
                  \x20   return names.resolve(name) == \"main\" && Named { name: name } == Named { name: name }\n\
                  }\n";
    let lowered_in = |dir: &str| {
        let at = root.join(dir);
        std::fs::create_dir_all(&at).expect("a directory to work in");
        let module = at.join("probe.nika");
        std::fs::write(&module, source).expect("write the module");
        match dir {
            // **The tools are lowered as their package** (ADR-294).
            "tools" => sysroot::lower_tools(&at),
            _ => sysroot::lower_std_module(&module),
        }
    };

    let tool = lowered_in("tools").expect("a tool lowers against the description");
    assert!(
        tool.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct Named"),
        "{tool}"
    );
    assert!(!tool.contains("async fn is_main"), "{tool}");
    // **A copy is not lent** (D4.1, the reading): a `Symbol` parameter is a
    // `Symbol` below, handed to `resolve` and still there to build with.
    assert!(
        tool.contains(
            "pub fn named(names: &winnow_grammar::InternerContext, name: winnow_grammar::Symbol)"
        ),
        "{tool}"
    );

    // `std`'s own module sees `std` alone, so `winnow_grammar::Symbol` is a
    // name nothing there knows. It is refused by this compiler rather than by
    // `rustc`, and not as a type that does not compare: whether a name nobody
    // knows compares is not asked (ADR-204 D1, #379).
    let std_module = lowered_in("src").expect_err("std does not read the description");
    let _ = std::fs::remove_dir_all(&root);
    let said = format!("{std_module:#}");
    assert!(said.contains("NK"), "{said}");
    assert!(!said.contains("NK1188"), "{said}");
}

/// **The toolchain's files are one namespace, and say so** (ADR-286 D1, #125):
/// two private functions of one name in two of them were lowered as one, and
/// a call of the one met the other's parameters - an argument count refused
/// about a function the caller never meant.
#[test]
fn two_tools_that_declare_one_name_are_refused() {
    let dir = std::env::temp_dir().join(format!("nikaia-tools-twice-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    std::fs::write(dir.join("a.nika"), "fn helper() -> i64 { return 1 }\n").expect("write a");
    std::fs::write(
        dir.join("b.nika"),
        "fn helper(x: i64) -> i64 { return x }\n",
    )
    .expect("write b");
    let refused = sysroot::lower_tools(&dir).expect_err("one name, two files");
    let said = format!("{refused:#}");
    assert!(
        said.contains("`helper` is declared twice in this package"),
        "{said}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **A constant another tool declares is read like a function it declares**
/// (ADR-286 D1, ADR-294): each file is checked with the others beside it, and
/// only its own `comptime`s were bound - so `TEXT` from `ty.nika` was
/// `NK1117` in `views.nika` (found moving the compiler's `views` into Nikaia,
/// #125).
#[test]
fn a_constant_another_tool_declares_is_read() {
    let dir = std::env::temp_dir().join(format!("nikaia-tools-constant-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    std::fs::write(
        dir.join("a.nika"),
        "pub comptime WORD: ref String = \"String\"\n",
    )
    .expect("write a");
    std::fs::write(
        dir.join("b.nika"),
        "pub fn written() -> String {\n\
         \x20   let mut out: String = \"ref \"\n\
         \x20   out.push_str(WORD)\n\
         \x20   return out\n\
         }\n",
    )
    .expect("write b");
    let lowered = sysroot::lower_tools(&dir).expect("one namespace, constants too");
    assert!(lowered.contains("out.push_str(WORD)"), "{lowered}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **`lower-std` reports every refusal of a file, in file order** (#575): a
/// file with ten findings cost ten cycles when only the first was said, and
/// each of them could be a rebuild.
#[test]
fn a_tools_file_with_two_refusals_reports_both_in_file_order() {
    let dir = std::env::temp_dir().join(format!("nikaia-tools-two-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    std::fs::write(
        dir.join("two.nika"),
        "fn first_one() -> i64 { return \"text\" }\n\nfn second_one() -> String { return 7 }\n",
    )
    .expect("write the file");
    let refused = sysroot::lower_tools(&dir).expect_err("two refusals");
    let said = format!("{refused:#}");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(said.matches("NK1104").count(), 2, "{said}");
    let (first, second) = (said.find("first_one"), said.find("second_one"));
    assert!(
        first.is_some() && second.is_some() && first < second,
        "in file order: {said}"
    );
    assert!(said.contains("2 refusals"), "{said}");
}

/// **The tools package is the same bytes on one thread as on every core**
/// (#667): `the_committed_rust_is_what_this_compiler_lowers` lowers it on
/// every core, this on one, and both against the committed file.
#[test]
fn the_tools_lower_the_same_on_one_thread_as_on_every_core() {
    let sysroot = Sysroot::resolve();
    let paths = sysroot
        .tool_modules()
        .expect("the toolchain's Nikaia modules");
    let one = sysroot::lower_tools_files_on(&paths, sysroot::tools_build(), Some(1))
        .expect("the toolchain's Nikaia lowers on one thread");
    let at = sysroot::tools_lowered_path(&sysroot.tools_dir());
    let committed = std::fs::read_to_string(&at).expect("the committed package");
    assert!(
        one == committed,
        "{} is not what one thread lowers the package to",
        at.display()
    );
}

/// **On several threads, the refusal said is the first in file order**
/// (#667), as it was when the files were lowered one after the other.
#[test]
fn the_first_refused_file_is_the_one_reported_on_every_core() {
    let dir = std::env::temp_dir().join(format!("nikaia-tools-first-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    let mut paths = Vec::new();
    for name in ["a_first", "b_second", "c_third", "d_fourth"] {
        let path = dir.join(format!("{name}.nika"));
        std::fs::write(&path, format!("fn {name}() -> i64 {{ return \"text\" }}\n"))
            .expect("write a file");
        paths.push(path);
    }
    let refused = sysroot::lower_tools_files_on(&paths, sysroot::tools_build(), Some(4))
        .expect_err("every file is refused");
    let said = format!("{refused:#}");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(said.contains("a_first"), "{said}");
    assert!(!said.contains("b_second"), "{said}");
}

/// **`lower-std` takes what it lowered before from the cache, and only
/// that** (#667): nothing changed is the same bytes again without lowering;
/// a changed file is a new key, so the package is lowered again.
#[test]
fn lower_std_takes_an_unchanged_lowering_from_the_cache() {
    let root = std::env::temp_dir().join(format!("nikaia-lower-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let tools = root.join("nikaia-std").join("src").join("tools");
    std::fs::create_dir_all(&tools).expect("a sysroot to lower");
    let cache = root.join("cache");
    std::fs::write(tools.join("one.nika"), "pub fn one() -> i64 { return 1 }\n").expect("one");
    std::fs::write(
        tools.join("two.nika"),
        "pub fn two() -> i64 { return one() + 1 }\n",
    )
    .expect("two");
    let lower = || {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
            .arg("lower-std")
            .arg("--sysroot")
            .arg(&root)
            .env("NIKAIA_CACHE_DIR", &cache)
            .output()
            .expect("nikaia runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        std::fs::read_to_string(tools.join("package.rs")).expect("the package is written")
    };
    let entries = || {
        std::fs::read_dir(cache.join("lower-std"))
            .map(|dir| {
                dir.filter(|e| e.as_ref().is_ok_and(|e| e.path().is_dir()))
                    .count()
            })
            .unwrap_or(0)
    };
    let first = lower();
    assert!(first.contains("pub fn two"), "{first}");
    assert_eq!(entries(), 1, "the first lowering is kept");
    std::fs::write(tools.join("package.rs"), "stale").expect("spoil the package");
    assert_eq!(lower(), first, "the same sources, the same bytes");
    assert_eq!(entries(), 1, "and no second entry");
    std::fs::write(
        tools.join("two.nika"),
        "pub fn two() -> i64 { return one() + 2 }\n",
    )
    .expect("change two");
    let changed = lower();
    assert_ne!(changed, first, "a changed file is lowered again");
    assert_eq!(entries(), 2);
    let _ = std::fs::remove_dir_all(&root);
}
