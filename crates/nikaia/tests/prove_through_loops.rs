//! **`prove-through-loops:on`** (ADR-361 D10, #767): an index's or an
//! overflow's check that reads a name a loop changes, and that ADR-306's walk
//! leaves open, is asked once more through the loop's invariant - found from
//! the facts the loop is reached with, and checked where the loop is reached
//! and at every end of a turn and every `break`. Under `off`, the default,
//! nothing is asked through a loop; either way a check that is not proved
//! stays, and the program stops where it stops with every optimization `off`.

mod common;

use std::path::Path;
use std::process::{Command, Output};

/// D10's own loop: a slot masked into a table of 16 384, carried from one
/// turn to the next; and one whose index only grows, which really leaves the
/// table when no slot from where it starts holds a zero.
const PROBED: &str = "fn probe(xs: ref Vec[i64], h: i64) -> i64 {
    assert(xs.len() == 16384)
    let mut at = h & 16383
    while true {
        if xs[at] == 0 {
            return at
        }
        at = (at + 1) & 16383
    }
    return -1
}

fn runaway(xs: ref Vec[i64], h: i64) -> i64 {
    assert(xs.len() == 16384)
    let mut at = h & 16383
    while true {
        if xs[at] == 0 {
            return at
        }
        at = at + 1
    }
    return -1
}

fn main() {
    let mut xs: Vec[i64] = []
    xs.resize(16384, 1)
    xs[5] = 0
    println(f\"{probe(xs, 16383 + 7)}\")
    println(f\"{runaway(xs, 16380)}\")
}
";

const ALL_ON: &str = "remove-bounds-checks:on,remove-overflow-checks:on,prove-through-loops:on";
const WALK_ONLY: &str = "remove-bounds-checks:on,remove-overflow-checks:on";

fn nikaia(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("the nikaia binary runs")
}

/// The lowered Rust of `PROBED` under `words`.
fn lowered(purpose: &str, words: &str) -> String {
    let dir = common::scratch_dir(purpose);
    std::fs::write(dir.join("main.nika"), PROBED).expect("the source");
    let out = nikaia(&dir, &["lower", "main.nika", "--optimization", words]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rust = std::fs::read_to_string(dir.join("main.rs")).expect("the lowered program");
    let _ = std::fs::remove_dir_all(&dir);
    rust
}

/// The body of the lowered function `name`.
fn body<'a>(rust: &'a str, name: &str) -> &'a str {
    let at = rust
        .find(&format!("fn {name}__unchecked("))
        .unwrap_or_else(|| panic!("{name}: {rust}"));
    let rest = &rust[at..];
    &rest[..rest.find("\n}").expect("its end")]
}

/// D10's example lowers with neither its index's nor its addition's check
/// under `on`, and with both under `off`.
#[test]
fn a_slot_carried_through_a_loop_is_proved_inside_its_table() {
    let on = lowered("through-loops-on", ALL_ON);
    let probe = body(&on, "probe");
    assert!(!probe.contains("index::get"), "{probe}");
    assert!(probe.contains("wrapping_add(at, 1)"), "{probe}");
    let off = lowered("through-loops-off", WALK_ONLY);
    let probe = body(&off, "probe");
    assert!(probe.contains("nikaia_std::index::get(&xs"), "{probe}");
    assert!(probe.contains("at = at + 1 & 16383"), "{probe}");
}

/// A loop whose bound really fails keeps its checks under `on`, and the
/// program stops at the same index as with every optimization `off`.
#[test]
fn a_loop_that_leaves_its_table_keeps_its_check_and_stops_there() {
    let on = lowered("through-loops-runaway", ALL_ON);
    let runaway = body(&on, "runaway");
    assert!(runaway.contains("nikaia_std::index::get(&xs"), "{runaway}");
    assert!(runaway.contains("at = at + 1;"), "{runaway}");
    let stops = |purpose: &str, words: &[&str]| -> (String, String) {
        let dir = common::scratch_dir(purpose);
        std::fs::write(dir.join("main.nika"), PROBED).expect("the source");
        let mut args = vec!["run", "main.nika"];
        args.extend_from_slice(words);
        let out = nikaia(&dir, &args);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!out.status.success(), "the program stops");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let line = stderr
            .lines()
            .find(|l| l.contains("index out of bounds"))
            .unwrap_or_else(|| panic!("{stderr}"))
            .to_string();
        (String::from_utf8_lossy(&out.stdout).to_string(), line)
    };
    let with = stops("through-loops-run-on", &["--optimization", ALL_ON]);
    let without = stops("through-loops-run-off", &[]);
    assert_eq!(with, without);
    assert_eq!(with.0, "5\n");
    assert!(with.1.contains("the index is 16384"), "{}", with.1);
}

/// D4, D7: the invariant that carried the checks is kept in `nikaia.proofs`
/// under the loop's place, with the strategy that found it; a second build
/// under `--locked` asks nothing the file does not answer.
#[test]
fn the_invariant_through_a_loop_is_kept_in_nikaia_proofs() {
    let dir = common::scratch_dir("through-loops-proofs");
    std::fs::create_dir_all(dir.join("src")).expect("the package");
    std::fs::write(
        dir.join("nikaia.toml"),
        format!(
            "[package]\nname = \"tl\"\nversion = \"0.1.0\"\n\n[build]\noptimization = \"{ALL_ON}\"\n"
        ),
    )
    .expect("a manifest");
    std::fs::write(dir.join("src/main.nika"), PROBED).expect("a source");
    let first = nikaia(&dir, &["build", "--no-cache"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let written = std::fs::read_to_string(dir.join("nikaia.proofs")).expect("written");
    let line = written
        .lines()
        .find(|l| l.contains(" invariant src/main.nika:4:5/bounds "))
        .unwrap_or_else(|| panic!("{written}"));
    assert!(line.contains("guess-and-check=ge(at,0)"), "{line}");
    assert!(line.contains("guess-and-check=le(at,16383)"), "{line}");
    let locked = nikaia(&dir, &["build", "--no-cache", "--locked"]);
    let again = std::fs::read_to_string(dir.join("nikaia.proofs")).expect("still there");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        locked.status.success(),
        "{}",
        String::from_utf8_lossy(&locked.stderr)
    );
    assert_eq!(again, written);
}

/// The word is `on` or `off`, as every optimization's is.
#[test]
fn prove_through_loops_is_on_or_off() {
    let dir = common::scratch_dir("through-loops-word");
    std::fs::write(dir.join("main.nika"), PROBED).expect("the source");
    let out = nikaia(
        &dir,
        &[
            "lower",
            "main.nika",
            "--optimization",
            "prove-through-loops:always",
        ],
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`prove-through-loops` is `on` or `off`, not `always`"),
        "{stderr}"
    );
}
