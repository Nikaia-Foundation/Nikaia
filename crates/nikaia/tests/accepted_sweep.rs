//! **Every program a checker test accepts compiles in `rustc`** (#705).
//!
//! A test that asserts the checker accepts a program proves nothing about the
//! Rust it lowers to. So the suite is run once with `NIKAIA_DUMP_ACCEPTED`
//! naming a directory: `check::check_against` writes there every one-file
//! source it accepts with no error, as `<test binary>-<hash>.nika`. This test
//! reads that directory afterwards, lowers each program, and has `rustc` check
//! the Rust (`--emit=metadata`: types and borrows, no code).
//!
//! **Not part of an ordinary run** (`#[ignore]`): it needs the suite's dump.
//! CI's `accepted-sweep` step runs it after the tests,
//!
//! ```text
//! NIKAIA_DUMP_ACCEPTED=$dir cargo test --workspace --release --no-fail-fast
//! NIKAIA_DUMP_ACCEPTED=$dir cargo test --release -p nikaia --test accepted_sweep -- --ignored
//! ```
//!
//! **Skipped and counted**: a program `nikaia check` or the lowering refuses
//! on its own (its test checked it against a ledger the test wrote, or it
//! names a package only a project has), and a program with no `main`.
//!
//! **One `rustc` per batch**: each lowered program becomes a module of one
//! crate. Only a batch that does not compile is compiled program by program,
//! so that every failing program is named with its first error.
//!
//! **The allow-list** ([`KNOWN`]) holds the programs `rustc` refuses for a
//! known reason, by the hash in their file name, each with its issue or the
//! record that leaves it to `rustc`. The test fails on a refusal not on it, on
//! an entry that compiles now, and on an entry no test checks any more: the
//! list says exactly what is still broken.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use nikaia::emit::Build;

/// A program `rustc` refuses for a known reason: the hash of its source (its
/// file name's last part), and the issue or record that says why.
const KNOWN: &[(&str, &str)] = &[
    // Left to `rustc` on purpose.
    (
        "018870a09f6d362e",
        "fold_lambdas: `par_fold` over a rule that is not a frame, the grammar backend's refusal (ADR-296 D9)",
    ),
    (
        "6d50da730c46510d",
        "undeclared_type: `http::Response` of a package the file does not have",
    ),
    // Open issues.
    (
        "a10522b5e28c4937",
        "#730 item 5: `Shared(Vec())` whose first use is a read (E0282)",
    ),
];

/// How many programs share one `rustc`.
const BATCH: usize = 40;

/// A dumped program: the hash that names it, the tests that checked it, its
/// path.
struct Dumped {
    hash: String,
    tests: Vec<String>,
    path: PathBuf,
}

fn dumped(dir: &Path) -> Vec<Dumped> {
    let mut by_hash: BTreeMap<String, Dumped> = BTreeMap::new();
    for entry in std::fs::read_dir(dir)
        .expect("the dump directory")
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "nika") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("utf-8 name")
            .to_string();
        let Some((test, hash)) = stem.rsplit_once('-') else {
            continue;
        };
        by_hash
            .entry(hash.to_string())
            .or_insert_with(|| Dumped {
                hash: hash.to_string(),
                tests: Vec::new(),
                path: path.clone(),
            })
            .tests
            .push(test.to_string());
    }
    let mut found: Vec<Dumped> = by_hash.into_values().collect();
    for program in &mut found {
        program.tests.sort();
    }
    found
}

/// Run `work` over `items` on as many threads as the machine has.
fn in_parallel<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = Mutex::new(0usize);
    let results: Mutex<Vec<Option<R>>> = Mutex::new(items.iter().map(|_| None).collect());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..threads.min(items.len().max(1)) {
            scope.spawn(|| {
                loop {
                    let at = {
                        let mut n = next.lock().expect("queue");
                        let at = *n;
                        *n += 1;
                        at
                    };
                    let Some(item) = items.get(at) else { break };
                    let result = work(item);
                    results.lock().expect("results")[at] = Some(result);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("results")
        .into_iter()
        .map(|r| r.expect("every item ran"))
        .collect()
}

/// The first error `rustc` reports, with the lines that explain it.
fn first_error(stderr: &str) -> String {
    let mut out = Vec::new();
    for line in stderr.lines().skip_while(|l| !l.starts_with("error")) {
        if !out.is_empty() && (line.starts_with("error") || line.starts_with("warning")) {
            break;
        }
        out.push(line);
    }
    out.join("\n")
}

/// `rustc --emit=metadata` on `rust`, in a scratch directory of its own.
fn rustc_refuses(purpose: &str, rust: &str, crate_type: &str) -> Option<String> {
    let dir = common::scratch_dir(purpose);
    let source = dir.join("sweep.rs");
    std::fs::write(&source, rust).expect("write the emitted Rust");
    let out = common::compile(
        &source,
        &[
            "--crate-type",
            crate_type,
            "--emit=metadata",
            "-A",
            "warnings",
            "--out-dir",
            dir.to_str().expect("utf-8 path"),
        ],
    );
    std::fs::remove_dir_all(&dir).ok();
    (!out.status.success()).then(|| first_error(&String::from_utf8_lossy(&out.stderr)))
}

/// The Rust of one program, or why it was not lowered: `nikaia check` refuses
/// it alone (its test checked it against a ledger the test wrote), or the
/// lowering does.
fn checked_and_lowered(path: &Path) -> Result<String, String> {
    let checked = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .arg("check")
        .arg(path)
        .env_remove("NIKAIA_DUMP_ACCEPTED")
        .output()
        .expect("the nikaia binary runs");
    if !checked.status.success() {
        return Err("`nikaia check` refuses it alone".to_string());
    }
    let read =
        nikaia::modules::Program::read_one(path).map_err(|e| format!("does not read: {e:#}"))?;
    let emitted = read
        .emit(Build::default())
        .map_err(|e| format!("does not lower: {e:#}"))?;
    Ok(emitted.rust)
}

/// The lowered program as a module of a batch.
fn as_module(hash: &str, rust: &str) -> String {
    format!(
        "pub mod p_{hash} {{\n{}\n}}\n",
        rust.replacen("\nfn main() {", "\npub fn main() {", 1)
    )
}

#[test]
#[ignore = "reads the dump of a suite run with NIKAIA_DUMP_ACCEPTED; CI's accepted-sweep step"]
fn every_program_a_checker_test_accepts_compiles() {
    let started = std::time::Instant::now();
    let dir = PathBuf::from(
        std::env::var_os("NIKAIA_DUMP_ACCEPTED")
            .expect("NIKAIA_DUMP_ACCEPTED names the directory the suite dumped into"),
    );
    let programs = dumped(&dir);
    assert!(
        !programs.is_empty(),
        "{} holds no program: was the suite run with NIKAIA_DUMP_ACCEPTED?",
        dir.display()
    );

    // Each is checked and lowered alone, as `nikaia check` and `nikaia lower`
    // do it: in a directory of its own, since a check writes its ledger
    // beside the file.
    let lowered: Vec<Result<String, String>> = in_parallel(&programs, |program| {
        let dir = common::scratch_dir("sweep-source");
        let path = dir.join(format!("p_{}.nika", program.hash));
        std::fs::copy(&program.path, &path).expect("copy a dumped program");
        let result = checked_and_lowered(&path);
        std::fs::remove_dir_all(&dir).ok();
        result
    });
    let mut not_lowered = 0usize;
    let mut no_main = 0usize;
    let mut good: Vec<(&Dumped, String)> = Vec::new();
    for (program, result) in programs.iter().zip(lowered) {
        match result {
            Err(_) => not_lowered += 1,
            Ok(rust) if !rust.contains("\nfn main(") => no_main += 1,
            Ok(rust) => good.push((program, rust)),
        }
    }
    let lowered_at = started.elapsed();

    // A batch at a time; a batch that does not compile, program by program.
    let batches: Vec<&[(&Dumped, String)]> = good.chunks(BATCH).collect();
    let refused: Vec<Vec<(String, String)>> = in_parallel(&batches, |batch| {
        let text: String = batch
            .iter()
            .map(|(program, rust)| as_module(&program.hash, rust))
            .collect();
        if rustc_refuses("sweep-batch", &text, "lib").is_none() {
            return Vec::new();
        }
        batch
            .iter()
            .filter_map(|(program, rust)| {
                rustc_refuses("sweep-one", rust, "bin").map(|e| (program.hash.clone(), e))
            })
            .collect()
    });
    let refused: BTreeMap<String, String> = refused.into_iter().flatten().collect();

    let known: BTreeMap<&str, &str> = KNOWN.iter().copied().collect();
    let by_hash: BTreeMap<&str, &Dumped> = programs.iter().map(|p| (p.hash.as_str(), p)).collect();
    let compiled: BTreeMap<&str, ()> = good.iter().map(|(p, _)| (p.hash.as_str(), ())).collect();
    let mut failures: Vec<String> = Vec::new();
    for (hash, error) in &refused {
        if known.contains_key(hash.as_str()) {
            continue;
        }
        let program = by_hash[hash.as_str()];
        failures.push(format!(
            "{hash} (checked by {}): the checker accepts it, `rustc` refuses it:\n{}\n--- the program ---\n{}",
            program.tests.join(", "),
            error,
            std::fs::read_to_string(&program.path).unwrap_or_default()
        ));
    }
    for (hash, why) in &known {
        if refused.contains_key(*hash) {
            continue;
        }
        if compiled.contains_key(hash) {
            failures.push(format!(
                "{hash} ({why}) compiles now: take it off the allow-list"
            ));
        } else {
            failures.push(format!(
                "{hash} ({why}) is no longer checked by any test, or no longer lowers: take it off the allow-list"
            ));
        }
    }

    eprintln!(
        "{} programs: {} compiled in {} batches, {} refused by rustc ({} on the allow-list), \
         {} skipped as refused or not lowered alone, {} without main; {:.1?} (lowered at {:.1?})",
        programs.len(),
        good.len(),
        batches.len(),
        refused.len(),
        refused
            .keys()
            .filter(|h| known.contains_key(h.as_str()))
            .count(),
        not_lowered,
        no_main,
        started.elapsed(),
        lowered_at,
    );
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
