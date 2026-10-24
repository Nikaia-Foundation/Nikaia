//! **The emitted code is `unsafe` only where a switch or the program says so**
//! ([ADR-378](../../../docs/specification/adr/adr-378.md)): with every
//! `remove-*` off it holds no `unsafe` the program text does not, and with
//! them on each one it adds is a call into `nikaia_std::proven`, which asserts
//! the dropped check in a debug build.

use crate::common;
use std::process::Command;

/// Every kind of check a proof drops: an index, a run read as a vector and as
/// a word, `+`, `+=`, a shift, a negation.
const DROPPED: &str = r#"use std::simd

fn f(b: ref Bytes, xs: ref Vec[i64], i: i64, k: i64, n: i32) -> i64 {
    assert(b.len() >= 64 && xs.len() == 8 && i >= 0 && i < 8 && k >= 0 && k < 64 && n > -5 && n < 5)
    let v: simd::Fixed[u8, 32] = simd::load(b, 0)
    let w = b.u64_le(8)
    let s = (w >> k) as i64
    let m = -n
    let mut j = i + 1
    j += 2
    return xs[i] + j + s + (v.lane(0) as i64) + (m as i64)
}

fn main() {
    let mut v: Vec[u8] = []
    for j in 0..<64 { v.push(j as u8) }
    println(f"{f(Bytes(v), [1, 2, 3, 4, 5, 6, 7, 8], 3, 5, 2)}")
}
"#;

/// A claim the program vouches for: its own `unsafe`, checked where nothing
/// is dropped (ADR-376 D2).
const VOUCHED: &str = r#"fn f(xs: ref Vec[i64], i: i64) -> i64 {
    unsafe { assume(i >= 0 && i < xs.len()) }
    return xs[i]
}

fn main() {
    println(f"{f([1, 2, 3], 1)}")
}
"#;

const ON: &str = "remove-bounds-checks:on,remove-overflow-checks:on";

fn lowered(name: &str, source: &str, optimization: Option<&str>) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nikaia"));
    command.current_dir(&dir).args(["lower", "main.nika"]);
    if let Some(switches) = optimization {
        command.args(["--optimization", switches]);
    }
    let out = command.output().expect("the nikaia binary runs");
    let rust = std::fs::read_to_string(dir.join("main.rs")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{name} did not lower:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    rust
}

#[test]
fn with_every_switch_off_the_emitted_code_holds_no_unsafe() {
    let rust = lowered("unsafe-off", DROPPED, None);
    assert_eq!(rust.matches("unsafe").count(), 0, "{rust}");
    let rust = lowered("unsafe-off-vouched", VOUCHED, None);
    assert_eq!(rust.matches("unsafe").count(), 0, "{rust}");
}

#[test]
fn with_the_switches_on_each_unsafe_is_a_call_into_proven() {
    let rust = lowered("unsafe-on", DROPPED, Some(ON));
    let all = rust.matches("unsafe").count();
    let proven = rust.matches("unsafe { nikaia_std::proven::").count();
    assert_eq!(all, proven, "{rust}");
    for operation in ["read", "array", "add", "shr", "neg"] {
        assert!(
            rust.contains(&format!("unsafe {{ nikaia_std::proven::{operation}")),
            "no proven::{operation}:\n{rust}"
        );
    }
    // The program's own `unsafe` stays its own: a believed claim.
    let rust = lowered("unsafe-on-vouched", VOUCHED, Some(ON));
    assert!(rust.contains("core::hint::assert_unchecked("), "{rust}");
}
