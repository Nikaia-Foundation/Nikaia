//! `examples/1brc.nika`, compiled and run.
//!
//! Everything else in this suite checks a piece of the compiler. This checks
//! the claim: the flagship example is lowered to Rust, that Rust is compiled by
//! the same `rustc` that built this test, the binary is run over a small input,
//! and its output is the one the benchmark asks for.
//!
//! The example is the real file, not a copy. It cannot drift.

mod common;

use std::path::PathBuf;
use std::process::Command;

use nikaia::emit::{Build, emit_program};
use nikaia::parser::parse_to_ast;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_1brc_example_compiles_and_prints_what_the_benchmark_asks_for() {
    let source_path = repo_root().join("examples/1brc.nika");
    let source = std::fs::read_to_string(&source_path).expect("examples/1brc.nika");

    // 1. Lower it.
    let parsed = parse_to_ast(&source).expect("the example parses");
    let lowered = emit_program(&parsed, Build::default()).expect("the example lowers");

    let dir = common::scratch_dir("1brc");
    let rust = dir.join("brc.rs");
    std::fs::write(&rust, &lowered.rust).expect("write the emitted Rust");

    // 2. Compile it.
    let binary = dir.join("brc");
    let compile = common::compile(
        &rust,
        &[
            "--crate-type",
            "bin",
            "-o",
            binary.to_str().expect("utf-8 path"),
        ],
    );

    assert!(
        compile.status.success(),
        "the emitted Rust did not compile:\n{}\n--- emitted ---\n{}",
        String::from_utf8_lossy(&compile.stderr),
        lowered.rust
    );

    // 3. Run it over an input small enough to check by hand.
    let measurements = dir.join("measurements.txt");
    std::fs::write(
        &measurements,
        "Hamburg;12.0\nBulawayo;8.9\nHamburg;-3.4\nPalembang;38.8\nBulawayo;22.1\nHamburg;0.0\n",
    )
    .expect("write the input");

    let run = Command::new(&binary)
        .arg(&measurements)
        .output()
        .expect("run the compiled example");

    assert!(
        run.status.success(),
        "the example failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    // Sorted by station, min/mean/max to one decimal, the whole line in braces.
    // Hamburg: -3.4, 0.0 and 12.0, so the mean is 8.6/3 = 2.8666… -> 2.9.
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "{Bulawayo=8.9/15.5/22.1, Hamburg=-3.4/2.9/12.0, Palembang=38.8/38.8/38.8}"
    );

    // 3a. Rounding as the official baseline rounds, `Math.round(x * 10.0) / 10.0`:
    // a tie goes toward +infinity, and `-0.0` is never printed. A: 0.25 -> 0.3;
    // B: -0.05 -> 0.0; C: 0.15 -> 0.2; D: -1.25 -> -1.2; E: -0.04 -> 0.0;
    // F: -0.3 alone; G: 16.65 -> 16.7.
    std::fs::write(
        &measurements,
        "A;0.2\nA;0.3\nB;-0.1\nB;0.0\nC;0.1\nC;0.2\nD;-1.2\nD;-1.3\n\
         E;-0.1\nE;0.0\nE;0.0\nE;-0.1\nE;0.0\nE;0.0\nE;0.0\nE;0.0\nE;0.0\nE;-0.2\n\
         F;-0.3\nG;-12.3\nG;45.6\n",
    )
    .expect("write the input");
    let ties = Command::new(&binary)
        .arg(&measurements)
        .output()
        .expect("run the compiled example");
    assert_eq!(
        String::from_utf8_lossy(&ties.stdout).trim(),
        "{A=0.2/0.3/0.3, B=-0.1/0.0/0.0, C=0.1/0.2/0.2, D=-1.3/-1.2/-1.2, \
         E=-0.2/0.0/0.0, F=-0.3/-0.3/-0.3, G=-12.3/16.7/45.6}"
    );

    // 3b. The format is one measurement per line and nothing between them
    // (`FILE` is lexical): a blank around a name is part of the name, and a
    // blank line is not a measurement.
    std::fs::write(&measurements, " Hamburg;1.0\nHamburg ;2.0\nHamburg;3.0\n")
        .expect("write the input");
    let blanks = Command::new(&binary)
        .arg(&measurements)
        .output()
        .expect("run the compiled example");
    assert_eq!(
        String::from_utf8_lossy(&blanks.stdout).trim(),
        "{ Hamburg=1.0/1.0/1.0, Hamburg=3.0/3.0/3.0, Hamburg =2.0/2.0/2.0}"
    );
    std::fs::write(&measurements, "Hamburg;1.0\n\nBulawayo;2.0\n").expect("write the input");
    let blank_line = Command::new(&binary)
        .arg(&measurements)
        .output()
        .expect("run the compiled example");
    assert!(
        !blank_line.status.success(),
        "a blank line was taken for a measurement: {}",
        String::from_utf8_lossy(&blank_line.stdout)
    );

    // 4. And it says so when the file is not there, rather than panicking.
    let missing = Command::new(&binary)
        .arg(dir.join("not-a-file.txt"))
        .output()
        .expect("run the compiled example");
    assert!(missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).starts_with("cannot open "),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// **`par_fold`'s options change no answer** (ADR-372 D4-D8, Part II 10.7):
/// the example with `order: Kept` and `order: Any`, small pieces and a pool of
/// three, at `user_parallelism` `no` and `yes`, prints what the example as
/// written prints at `no`; and a file broken in two places reports the first,
/// the same under every one of them.
#[test]
fn the_1brc_example_means_the_same_under_every_order_and_setting() {
    let source = std::fs::read_to_string(repo_root().join("examples/1brc.nika"))
        .expect("examples/1brc.nika");
    let written = "Summary::merge)";
    assert_eq!(source.matches(written).count(), 1, "the par_fold's merge");
    let with = |options: &str| -> String {
        let folded = source.replace(written, &format!("Summary::merge; {options})"));
        format!("use std::par\n{folded}")
    };
    let dir = common::scratch_dir("1brc-orders");
    let stations = [
        "Hamburg",
        "Bulawayo",
        "Palembang",
        "St. John's",
        "Zürich",
        "東京",
    ];
    let mut good = String::new();
    for i in 0..4000 {
        let tenths: i32 = (i * 37) % 999 - 499;
        let sign = if tenths < 0 { "-" } else { "" };
        let station = stations[i.unsigned_abs() as usize % stations.len()];
        good += &format!(
            "{station};{sign}{}.{}\n",
            tenths.abs() / 10,
            tenths.abs() % 10
        );
    }
    let mut lines: Vec<String> = good.lines().map(str::to_owned).collect();
    lines[1234] = "Hamburg;x.1".to_owned();
    lines[3210] = "Bulawayo;1.y".to_owned();
    let bad = lines.join("\n") + "\n";
    let good_path = dir.join("good.txt");
    let bad_path = dir.join("bad.txt");
    std::fs::write(&good_path, &good).expect("write the input");
    std::fs::write(&bad_path, &bad).expect("write the input");

    let mut answers: Vec<(String, String, String)> = Vec::new();
    let variants = [
        ("as-written-no", source.clone(), Build::default()),
        (
            "kept-yes",
            with("order: par::Order::Kept, piece: 64"),
            Build::parallel(),
        ),
        (
            "any-yes",
            with("order: par::Order::Any, piece: 64, workers: 3"),
            Build::parallel(),
        ),
        (
            "any-no",
            with("order: par::Order::Any, piece: 1"),
            Build::default(),
        ),
    ];
    for (name, text, build) in variants {
        let parsed = parse_to_ast(&text).expect("the example parses");
        let lowered = emit_program(&parsed, build).expect("the example lowers");
        let rust = dir.join(format!("{name}.rs"));
        std::fs::write(&rust, &lowered.rust).expect("write the emitted Rust");
        let binary = dir.join(name);
        let compile = common::compile(
            &rust,
            &[
                "--crate-type",
                "bin",
                "-o",
                binary.to_str().expect("utf-8 path"),
            ],
        );
        assert!(
            compile.status.success(),
            "{name}: the emitted Rust did not compile:\n{}\n--- emitted ---\n{}",
            String::from_utf8_lossy(&compile.stderr),
            lowered.rust
        );
        let ran = |input: &PathBuf| {
            Command::new(&binary)
                .arg(input)
                .output()
                .expect("run the compiled example")
        };
        let ok = ran(&good_path);
        assert!(
            ok.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&ok.stderr)
        );
        let failed = ran(&bad_path);
        assert!(!failed.status.success(), "{name}: the broken file was read");
        answers.push((
            name.to_owned(),
            String::from_utf8_lossy(&ok.stdout).into_owned(),
            String::from_utf8_lossy(&failed.stderr).into_owned(),
        ));
    }
    let (_, out, err) = &answers[0];
    assert!(out.contains("Hamburg="), "{out}");
    assert!(err.contains("line 1235"), "{err}");
    for (name, other_out, other_err) in &answers[1..] {
        assert_eq!(other_out, out, "{name}");
        assert_eq!(other_err, err, "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
