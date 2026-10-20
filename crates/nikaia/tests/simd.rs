//! **Vectors a program holds** ([ADR-377](../../../docs/specification/adr/adr-377.md),
//! #775): `simd::Lanes[T]` at the build's width, `simd::Fixed[T, N]`,
//! `simd::Mask`, each operation at x86-64's baseline and at `x86-64-v3`
//! (ADR-002 D6), `NK1296` to `NK1299`, a load past the end, and a load proved
//! inside that is read without its check.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The program `source` run as a project at the CPU level `cpu`: whether it
/// ran to the end, and what it printed on stdout and stderr.
fn run_at(name: &str, cpu: &str, source: &str) -> (bool, String, String) {
    let dir = common::scratch_dir(&format!("simd-{name}{cpu}"));
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(
        dir.join("nikaia.toml"),
        "[package]\nname = \"lanes\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    std::fs::write(dir.join("src/main.nika"), source).expect("entry");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .args(["run", cpu, "--project"])
        .arg(&dir)
        .env(
            "NIKAIA_CACHE_DIR",
            repo_root().join("target").join("nikaia-project-tests"),
        )
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `nikaia check` or `nikaia lower` of one file, with `options` after it.
fn tool(name: &str, command: &str, source: &str, options: &[&str]) -> (String, String) {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg(command)
        .arg("main.nika")
        .args(options)
        .output()
        .expect("the nikaia binary runs");
    let rust = std::fs::read_to_string(dir.join("main.rs")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        rust,
    )
}

/// Every operation of D3 and D5 once.
const EVERY_OPERATION: &str = r#"use std::simd

fn main() {
    let b = Bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33])
    let x: simd::Fixed[u64, 2] = simd::Fixed::from([7, 9])
    let m: simd::Fixed[u64, 2] = simd::splat(0xFF)
    let y = x.wrapping_add(m) & m
    let s = (y << 8) >> 4
    let e = y.eq(m)
    let z = simd::select(x.lt(m), x, !m)
    let v: simd::Fixed[u8, 16] = simd::load(b, 1)
    let r = v.shuffle(simd::Fixed::from([15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]))
    let wts: simd::Fixed[i8, 16] = simd::splat(1)
    let d = r.dot_pairs(wts)
    let ones: simd::Fixed[i16, 8] = simd::splat(1)
    let q = d.dot_pairs(ones)
    let w: simd::Lanes[u8] = simd::load(b, 0)
    let k: simd::Lanes[u8] = simd::splat(5)
    let km = w.gt(k)
    println(f"{y.lane(0)} {s.lane(1)} {v.lane(3)} {e.any()} {z.lane(0)} {r.lane(0)} {q.wrapping_sum()} {w.count()} {km.count()} {km.first() ?? -1}")
    for p in km.positions() {
        if p < 7 { println(f"{p}") }
    }
    let a: simd::Fixed[u8, 4] = simd::Fixed::from([250, 1, 2, 3])
    let t: simd::Fixed[u8, 4] = simd::splat(10)
    let sat = a.saturating_add(t)
    let low = a.saturating_sub(t)
    let both = a.lt(t) & a.ne(t) | a.ge(t) & !a.le(t)
    let wide = a.widen()
    let big: simd::Fixed[u32, 4] = simd::Fixed::from([7, 70000, 3, 65535])
    let narrow = big.narrow_saturating()
    println(f"{sat.lane(0)} {low.lane(1)} {a.min(t).lane(0)} {a.max(t).lane(1)} {both.all()} {a.wrapping_sub(t).lane(1)} {a.wrapping_mul(t).lane(0)} {a.min_lane()} {a.max_lane()} {(a ^ t).lane(0)} {(a | t).lane(1)} {wide.lane(0)} {narrow.lane(1)}")
}
"#;

/// ADR-002 D6: the same program at x86-64's baseline (SSE2) and at
/// `x86-64-v3` (AVX2) prints the same, but for `Lanes[u8]`'s count: 16 bytes
/// and 32 (D1).
#[test]
fn every_operation_at_every_cpu_level() {
    if !cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        return;
    }
    let rest = "5\n6\n255 0 10 10 true 247 196 1 250 240 11 250 65535\n";
    for (cpu, lanes) in [("--cpu=x86-64", "16 11"), ("--cpu=x86-64-v3", "32 27")] {
        let (ok, out, err) = run_at("every", cpu, EVERY_OPERATION);
        assert!(ok, "{cpu}: {err}");
        assert_eq!(
            out,
            format!("6 128 5 false 7 17 152 {lanes} 5\n{rest}"),
            "{cpu}"
        );
    }
}

const REFUSED: &str = r#"use std::simd

fn f(x: simd::Fixed[u8, 12], y: simd::Lanes[f64], z: simd::Fixed[u16, 128]) {}

fn main() {
    let x: simd::Fixed[u64, 2] = simd::Fixed::from([7, 9])
    let y = x + x
    let k = 3
    let s = x << k
    let t = x >> 64
    let v: simd::Fixed[u8, 4] = simd::Fixed::from([1, 2, 3, 4])
    let r = v.shuffle(simd::Fixed::from([0, 1, 2, 4]))
    let idx: simd::Fixed[u8, 4] = simd::splat(1)
    let q = v.shuffle(idx)
}
"#;

/// D2 (`NK1297`), D4 (`NK1296`, whose help names both ways), D3 (`NK1298`,
/// `NK1299`).
#[test]
fn each_code() {
    let (said, _) = tool("simd-refused", "check", REFUSED, &[]);
    assert!(
        said.contains("error[NK1297]: `simd::Fixed` can't hold 12 lanes."),
        "{said}"
    );
    assert!(
        said.contains("error[NK1297]: `simd::Lanes` can't hold lanes of `f64`."),
        "{said}"
    );
    assert!(
        said.contains("error[NK1297]: `simd::Fixed` can't hold 128 lanes."),
        "{said}"
    );
    assert!(
        said.contains("error[NK1296]: `+` is not a lane operation"),
        "{said}"
    );
    assert!(
        said.contains("`a.wrapping_add(b)` wraps, `a.saturating_add(b)`"),
        "{said}"
    );
    assert_eq!(said.matches("error[NK1299]").count(), 2, "{said}");
    assert!(
        said.contains("error[NK1298]: The shuffle index 4 is outside the 4 lanes"),
        "{said}"
    );
    assert!(
        said.contains("error[NK1298]: A shuffle's indices are written as constants."),
        "{said}"
    );
}

const PAST_THE_END: &str = r#"use std::simd

fn main() {
    let b = Bytes([1, 2, 3, 4, 5, 6, 7, 8])
    let v: simd::Fixed[u16, 4] = simd::load(b, 1)
    println(f"{v.lane(0)}")
}
"#;

/// D3: a load reads what its cut reads, and stops where the cut would.
#[test]
fn a_load_past_the_end_stops() {
    let (ok, _, err) = run_at("past", "--cpu=x86-64", PAST_THE_END);
    assert!(!ok, "it ran to the end");
    assert!(
        err.contains("out of bounds") || err.contains("index"),
        "{err}"
    );
}

const GUARDED: &str = r#"use std::simd

fn f(b: ref Bytes, at: i64) -> i64 {
    if at >= 0 && at + 32 <= b.len() {
        let v: simd::Fixed[u8, 16] = simd::load(b, at)
        let w: simd::Lanes[u8] = simd::load(b, at)
        return w.wrapping_sum() as i64 + v.lane(0) as i64
    }
    let u: simd::Fixed[u8, 16] = simd::load(b, at)
    return u.lane(1) as i64
}

fn main() {
    println(f"{f(Bytes([1, 2]), 0)}")
}
"#;

/// D3, ADR-306 D5: a load under the guard its cut needs is proved and read
/// through `proven::array`; a `Lanes` is proved for the most it reads on any
/// CPU level; one nothing guards keeps its check.
#[test]
fn a_proved_load_has_no_check() {
    let (said, kept) = tool("simd-kept", "lower", GUARDED, &[]);
    assert_eq!(kept.matches("load_proven").count(), 0, "{said}{kept}");
    let (said, rust) = tool(
        "simd-proved",
        "lower",
        GUARDED,
        &["--optimization", "remove-bounds-checks:on"],
    );
    assert!(
        rust.contains("simd::load_proven(unsafe { nikaia_std::proven::array::<u8, 16>(&b"),
        "{said}{rust}"
    );
    assert!(
        rust.contains("nikaia_std::proven::array::<u8, { simd::WIDTH }>(&b"),
        "{rust}"
    );
    assert_eq!(rust.matches("simd::load(b, at)").count(), 1, "{rust}");
}
