//! `nikaia fmt` ([ADR-337](../../../docs/specification/adr/adr-337.md),
//! Part III 13.2): the comments the parser hands beside the tree, and the
//! printer that puts them back.

use nikaia::parser::parse_to_ast;

/// The text of every comment the parser recorded, in order.
fn comments(source: &str) -> Vec<String> {
    let parsed = parse_to_ast(source).expect("parses");
    parsed
        .comments
        .iter()
        .map(|span| source[span.start as usize..span.end as usize].to_string())
        .collect()
}

#[test]
fn every_comment_is_recorded_in_order_with_its_place() {
    let source = "// before the item\n\
                  /// the item's documentation\n\
                  fn main() { // at the end of a line\n\
                  \x20   let a = 1 /* inside /* nested */ a line */ + 2\n\
                  \x20   // inside a block\n\
                  \x20   println(f\"{a}\")\n\
                  }\n";
    assert_eq!(
        comments(source),
        vec![
            "// before the item",
            "/// the item's documentation",
            "// at the end of a line",
            "/* inside /* nested */ a line */",
            "// inside a block",
        ]
    );
}

#[test]
fn slashes_in_a_string_or_a_dsl_body_are_no_comment() {
    let source = "fn page(a: String) -> String {\n\
                  \x20   let url = \"https://example.org\"\n\
                  \x20   return dsl html {\n\
                  \x20       <a href=\"https://example.org\">{a}</a> // not one\n\
                  \x20   } eod\n\
                  }\n\
                  fn main() {\n\
                  \x20   println(page(\"x\")) // one\n\
                  }\n";
    assert_eq!(comments(source), vec!["// one"]);
}

/// **A broken list ends in a `,`** (ADR-337 D1), so the parser takes one
/// after the last parameter, option and argument, as after a literal's last
/// element: what the printer writes parses.
#[test]
fn a_trailing_comma_closes_parameters_options_and_arguments() {
    for source in [
        "fn f(a: i64, b: i64,) -> i64 {\n    return a + b\n}\n",
        "fn f(ref self, a: i64,) {\n}\n",
        "fn f(a: i64; wide: bool = false,) {\n}\n",
        "fn f(wide: bool = false, deep: bool = true,) {\n}\n",
        "fn main() {\n    f(1, 2,)\n    g(1; wide: true,)\n    h(wide: true,)\n}\n",
    ] {
        assert!(parse_to_ast(source).is_ok(), "does not parse:\n{source}");
    }
}

/// The file formatted, checked to be a fixed point: formatting it again
/// changes nothing (ADR-337 D4).
fn formatted(source: &str) -> String {
    let once = nikaia::fmt::format(source).expect("formats");
    let twice = nikaia::fmt::format(&once).expect("formats again");
    assert_eq!(once, twice, "formatting is not idempotent on:\n{once}");
    once
}

#[test]
fn one_tab_per_level_and_no_semicolon() {
    let source = "fn main() {\n  let a = 1;\n  if a > 0 {\n        println(f\"{a}\");\n  }\n}\n";
    assert_eq!(
        formatted(source),
        "fn main() {\n\tlet a = 1\n\tif a > 0 {\n\t\tprintln(f\"{a}\")\n\t}\n}\n"
    );
}

#[test]
fn a_brace_goes_up_and_else_follows_its_brace() {
    let source = "fn sign(a: i64) -> i64\n{\n    if a < 0\n    {\n        return -1\n    }\n    else if a == 0 {\n        return 0\n    }\n    else\n    {\n        return 1\n    }\n}\n";
    assert_eq!(
        formatted(source),
        "fn sign(a: i64) -> i64 {\n\tif a < 0 {\n\t\treturn -1\n\t} else if a == 0 {\n\t\treturn 0\n\t} else {\n\t\treturn 1\n\t}\n}\n"
    );
}

#[test]
fn a_long_call_is_broken_one_argument_a_line_with_a_trailing_comma() {
    let source = "fn main() {\n    let total = combine(first_argument_value, second_argument_value, third_argument_value, fourth_argument)\n}\n";
    assert_eq!(
        formatted(source),
        "fn main() {\n\tlet total = combine(\n\t\tfirst_argument_value,\n\t\tsecond_argument_value,\n\t\tthird_argument_value,\n\t\tfourth_argument,\n\t)\n}\n"
    );
}

#[test]
fn a_long_list_is_broken_and_a_short_one_kept() {
    let source = "fn main() {\n    let short = [1, 2, 3]\n    let names = [\"alpha\", \"beta\", \"gamma\", \"delta\", \"epsilon\", \"zeta\", \"eta\", \"theta\", \"iota\", \"kappa\"]\n}\n";
    assert_eq!(
        formatted(source),
        "fn main() {\n\tlet short = [1, 2, 3]\n\tlet names = [\n\t\t\"alpha\",\n\t\t\"beta\",\n\t\t\"gamma\",\n\t\t\"delta\",\n\t\t\"epsilon\",\n\t\t\"zeta\",\n\t\t\"eta\",\n\t\t\"theta\",\n\t\t\"iota\",\n\t\t\"kappa\",\n\t]\n}\n"
    );
}

#[test]
fn a_broken_list_keeps_its_lines_and_gains_its_last_comma() {
    let source = "fn main() {\n    let xs = [\n        1,\n        2\n    ]\n}\n";
    assert_eq!(
        formatted(source),
        "fn main() {\n\tlet xs = [\n\t\t1,\n\t\t2,\n\t]\n}\n"
    );
}

#[test]
fn a_long_condition_breaks_before_its_loosest_operator() {
    let source = "fn main() {\n    let ok = first_condition_holds && second_condition_holds || third_condition_holds_too && fourth_one\n}\n";
    assert_eq!(
        formatted(source),
        "fn main() {\n\tlet ok = first_condition_holds && second_condition_holds\n\t\t|| third_condition_holds_too && fourth_one\n}\n"
    );
}

#[test]
fn a_trailing_comment_follows_its_code_after_one_space() {
    let source = "fn main() {\n    let a = 1        // one\n    let bb = 22 // two\n    // its own line\n}\n";
    assert_eq!(
        formatted(source),
        "fn main() {\n\tlet a = 1 // one\n\tlet bb = 22 // two\n\t// its own line\n}\n"
    );
}

#[test]
fn a_doc_run_stays_and_blank_lines_shrink_to_one() {
    let source = "/// The first line of the documentation,\n///   indented inside, never re-wrapped.\npub fn f() {\n}\n\n\n\n// Before the next item.\nfn g() {\n}\n";
    assert_eq!(
        formatted(source),
        "/// The first line of the documentation,\n///   indented inside, never re-wrapped.\npub fn f() {\n}\n\n// Before the next item.\nfn g() {\n}\n"
    );
}

#[test]
fn a_dsl_body_is_copied_byte_for_byte() {
    let source = "fn page(a: String) -> String {\n    return dsl html {\n  <p>   {a}  </p>;  // kept\n        <br>\n    } eod\n}\n";
    assert_eq!(
        formatted(source),
        "fn page(a: String) -> String {\n\treturn dsl html {\n  <p>   {a}  </p>;  // kept\n        <br>\n    } eod\n}\n"
    );
}

#[test]
fn strings_and_literals_keep_their_spelling() {
    let source = "fn main() {\n    let s = \"a;  b // c\"\n    let f = f\"{{x}} {s.len():>4} \\u{41}\"\n    let c = '\\''\n    let x = (1.5e-3 + 0x1F)\n}\n";
    assert_eq!(
        formatted(source),
        "fn main() {\n\tlet s = \"a;  b // c\"\n\tlet f = f\"{{x}} {s.len():>4} \\u{41}\"\n\tlet c = '\\''\n\tlet x = (1.5e-3 + 0x1F)\n}\n"
    );
}

fn nika_files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "target" || name.to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            nika_files(&path, found);
        } else if path.extension().is_some_and(|e| e == "nika") {
            found.push(path);
        }
    }
}

/// **The guard holds over the repository** (step 3 of #101): every `.nika`
/// in the examples, the tests, the benchmarks and `std` that parses is
/// formatted without the guard refusing it, and formatting it again changes
/// nothing.
#[test]
fn every_nika_file_formats_to_the_same_tree_and_a_fixed_point() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in [
        "examples",
        "tests",
        "benches",
        "crates/nikaia-std/src",
        "crates/nikaia/tests",
        "packages",
    ] {
        nika_files(&root.join(dir), &mut files);
    }
    assert!(files.len() > 100, "found only {} files", files.len());
    let mut failures = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path).expect("reads");
        if parse_to_ast(&source).is_err() {
            continue;
        }
        match nikaia::fmt::format(&source) {
            Err(error) => failures.push(format!("{}: {error}", path.display())),
            Ok(once) => match nikaia::fmt::format(&once) {
                Ok(twice) if twice == once => {}
                Ok(_) => failures.push(format!("{}: not idempotent", path.display())),
                Err(error) => failures.push(format!("{}: again: {error}", path.display())),
            },
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} files:\n{}",
        failures.len(),
        files.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A fresh package directory with these files under it, for `nikaia fmt`.
fn package(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("nikaia-fmt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

/// `nikaia fmt ARGS` in `dir`: the exit code, stdout and stderr.
fn fmt_in(dir: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .arg("fmt")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("runs nikaia");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const MANIFEST: &str = "[package]\nname = \"p\"\nversion = \"0.1.0\"\n";
const DIRTY: &str = "fn main() {\n    let a = 1;\n    println(f\"{a}\");\n}\n";
const CLEAN: &str = "fn main() {\n\tlet a = 1\n\tprintln(f\"{a}\")\n}\n";

#[test]
fn fmt_rewrites_the_package_files_that_change_and_only_those() {
    let root = package(
        "rewrite",
        &[
            ("nikaia.toml", MANIFEST),
            ("src/main.nika", DIRTY),
            ("src/clean.nika", CLEAN),
            ("target/skipped.nika", DIRTY),
        ],
    );
    let before = std::fs::metadata(root.join("src/clean.nika"))
        .unwrap()
        .modified()
        .unwrap();
    let (code, stdout, stderr) = fmt_in(&root.join("src"), &[]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("main.nika"), "{stdout}");
    assert!(!stdout.contains("clean.nika"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(root.join("src/main.nika")).unwrap(),
        CLEAN
    );
    let after = std::fs::metadata(root.join("src/clean.nika"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(before, after, "an unchanged file is not written");
    assert_eq!(
        std::fs::read_to_string(root.join("target/skipped.nika")).unwrap(),
        DIRTY
    );

    // A second run finds nothing to do.
    let (code, stdout, stderr) = fmt_in(&root, &[]);
    assert_eq!((code, stdout.as_str()), (0, ""), "{stderr}");
    let (code, stdout, _) = fmt_in(&root, &["--check"]);
    assert_eq!((code, stdout.as_str()), (0, ""));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn fmt_check_names_the_files_that_would_change_and_writes_nothing() {
    let root = package("check", &[("dirty.nika", DIRTY), ("clean.nika", CLEAN)]);
    let (code, stdout, _) = fmt_in(&root, &["--check", "clean.nika"]);
    assert_eq!((code, stdout.as_str()), (0, ""));
    let (code, stdout, _) = fmt_in(&root, &["--check", "dirty.nika", "clean.nika"]);
    assert_eq!(code, 1);
    assert_eq!(stdout, "dirty.nika\n");
    assert_eq!(
        std::fs::read_to_string(root.join("dirty.nika")).unwrap(),
        DIRTY
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn fmt_names_a_file_it_refuses_and_leaves_it() {
    let broken = "fn main( {\n    let a = 1;\n}\n";
    let root = package("refused", &[("broken.nika", broken), ("dirty.nika", DIRTY)]);
    let (code, stdout, stderr) = fmt_in(&root, &["broken.nika", "dirty.nika"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("broken.nika: not formatted"), "{stderr}");
    assert_eq!(
        std::fs::read_to_string(root.join("broken.nika")).unwrap(),
        broken
    );
    // The others are still formatted.
    assert!(stdout.contains("dirty.nika"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(root.join("dirty.nika")).unwrap(),
        CLEAN
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn fmt_alone_outside_a_package_is_refused() {
    let root = package("outside", &[("dirty.nika", DIRTY)]);
    let (code, _, stderr) = fmt_in(&root, &[]);
    assert_eq!(code, 1);
    assert!(stderr.contains("nikaia.toml"), "{stderr}");
    assert_eq!(
        std::fs::read_to_string(root.join("dirty.nika")).unwrap(),
        DIRTY
    );
    let _ = std::fs::remove_dir_all(&root);
}
