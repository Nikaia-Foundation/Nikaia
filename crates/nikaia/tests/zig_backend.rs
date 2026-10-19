//! **`nikaia zig`, the experimental Zig back end** (`src/zig.rs`): a checked
//! file is lowered to one `.zig` file, and what the prototype does not cover
//! is refused by name.
//!
//! Running the Zig needs a Zig 0.13 compiler, which CI does not have: the
//! test that runs it compiles only where `NIKAIA_ZIG` names one, and checks
//! that the program prints what its Rust build prints
//! (`fixtures/zig_demo.stdout`).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

const DEMO: &str = include_str!("fixtures/zig_demo.nika");
const DEMO_STDOUT: &str = include_str!("fixtures/zig_demo.stdout");

/// `nikaia zig` on `source`: the Zig written, or the refusal printed.
fn lower(purpose: &str, source: &str) -> (PathBuf, Result<String, String>) {
    let dir = common::scratch_dir(&format!("zig-{purpose}"));
    std::fs::write(dir.join("main.nika"), source).expect("write the source");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .args(["zig", "main.nika"])
        .output()
        .expect("the nikaia binary runs");
    let result = if out.status.success() {
        Ok(std::fs::read_to_string(dir.join("main.zig")).expect("the Zig is written"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    };
    (dir, result)
}

#[test]
fn the_demo_is_lowered() {
    let (_, zig) = lower("demo", DEMO);
    let zig = zig.expect("the demo lowers");
    // Rust's rounding of `/` and `%`, which Zig makes a signed integer say.
    assert!(zig.contains("@divTrunc("), "{zig}");
    assert!(zig.contains("@rem("), "{zig}");
    // A method that changes `self` takes a pointer, so `p` is a `var`...
    assert!(
        zig.contains("pub fn step(self: *Point, dx: i64, dy: i64) void"),
        "{zig}"
    );
    assert!(zig.contains("var p: Point = Point.origin();"), "{zig}");
    // ...and a `let` nothing changes is a `const`.
    assert!(
        zig.contains("const xs: []const i64 = &[_]i64{ 1, 2, 3, 4, 5 };"),
        "{zig}"
    );
    assert!(zig.contains("nk_std.mem.eql(u8, "), "{zig}");
}

#[test]
fn a_literal_takes_its_type_from_the_return() {
    let (_, zig) = lower(
        "hint",
        "fn steps(start: i32) -> i32 {\n    let mut n = start\n    let mut k = 0\n    while n > 0 {\n        n -= 1\n        k += 1\n    }\n    return k\n}\n\nfn main() {\n    println(f\"{steps(3)}\")\n}\n",
    );
    let zig = zig.expect("the file lowers");
    assert!(zig.contains("var k: i32 = 0;"), "{zig}");
}

#[test]
fn an_unused_binding_is_discarded() {
    let (_, zig) = lower(
        "unused",
        "fn main() {\n    let x = 5\n    for k in 0..<2 {\n        println(\"turn\")\n    }\n}\n",
    );
    let zig = zig.expect("the file lowers");
    assert!(zig.contains("const x: i64 = 5;\n    _ = x;"), "{zig}");
}

#[test]
fn an_enum_is_refused_by_name() {
    let (_, refused) = lower(
        "enum",
        "enum Colour {\n    Red,\n    Green,\n}\n\nfn main() {\n    println(\"x\")\n}\n",
    );
    let message = refused.expect_err("an enum is refused");
    assert!(
        message.contains("the Zig prototype lowers functions, structs"),
        "{message}"
    );
}

#[test]
fn the_demo_prints_what_its_rust_build_prints() {
    let Some(zig) = std::env::var_os("NIKAIA_ZIG") else {
        eprintln!("NIKAIA_ZIG names no Zig compiler: the Zig is lowered, not run");
        return;
    };
    let (dir, lowered) = lower("demo-run", DEMO);
    lowered.expect("the demo lowers");
    let out = run_zig(Path::new(&zig), &dir);
    assert_eq!(String::from_utf8_lossy(&out), DEMO_STDOUT);
}

fn run_zig(zig: &Path, dir: &Path) -> Vec<u8> {
    let out = Command::new(zig)
        .current_dir(dir)
        .env("ZIG_GLOBAL_CACHE_DIR", dir.join("zig-cache"))
        .env("ZIG_LOCAL_CACHE_DIR", dir.join("zig-cache"))
        .args(["run", "-O", "ReleaseSafe", "main.zig"])
        .output()
        .expect("the Zig compiler runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}
