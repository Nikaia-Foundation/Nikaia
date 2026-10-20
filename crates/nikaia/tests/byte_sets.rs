//! **`bytes::ByteSet` and the searches over it** ([ADR-375](../../../docs/specification/adr/adr-375.md),
//! #754): `find` over a set, `find_all`, `split`, `NK1287`, the consumers
//! `find_all` has of its own (D4), and the same answers at every CPU level
//! (ADR-002 D6).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn run(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let run = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("run")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        run.status.success(),
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).into_owned()
}

fn checked(name: &str, source: &str) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("check")
        .arg("main.nika")
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn lowered(name: &str, source: &str) -> String {
    lowered_with(name, source, &[])
}

/// Lowered with `options` after the file: `--optimization` among them.
fn lowered_with(name: &str, source: &str, options: &[&str]) -> String {
    let dir = common::scratch_dir(name);
    std::fs::write(dir.join("main.nika"), source).expect("the source");
    let out = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(&dir)
        .arg("lower")
        .arg("main.nika")
        .args(options)
        .output()
        .expect("the nikaia binary runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rust = std::fs::read_to_string(dir.join("main.rs")).expect("the lowered program");
    let _ = std::fs::remove_dir_all(&dir);
    rust
}

/// Every search of D3 on one buffer: a set used again, a `'…'` for one byte, a
/// `from`, `split` on a `Bytes`, on a text's own bytes, on a cut and on a
/// `ref Bytes` parameter, a set of five, bytes above `0x7F`, and a set that is
/// empty only at run time.
const SEARCHES: &str = "use std::bytes\n\
\n\
fn pieces(b: ref Bytes, set: bytes::ByteSet) -> i64 {\n\
    let mut n = 0\n\
    for piece in b.split(set) {\n\
        n += piece.len() + 1\n\
    }\n\
    return n\n\
}\n\
\n\
fn main() {\n\
    let seps = bytes::ByteSet([';', '\\n'])\n\
    let data = Bytes(['a', ';', 'b', '\\n', 'c', ';', ';'])\n\
    let high = bytes::ByteSet(['\\xFF', '\\x80', '\\x00'])\n\
    let vowels = bytes::ByteSet(['a', 'e', 'i', 'o', 'u'])\n\
    let s = \"x;yy;z\\nabc\"\n\
    println(f\"{data.find(seps) ?? -1} {data.find(';') ?? -1} {data.find(seps; from: 2) ?? -1}\")\n\
    for p in data.find_all(seps) {\n\
        print(f\"{p} \")\n\
    }\n\
    println(\"\")\n\
    println(f\"{data.find_all(seps).count()} {data.find_all(seps; from: 2).count()} {data.find_all(seps).nth(1) ?? -1} {data.find_all(seps).last() ?? -1}\")\n\
    println(f\"{data.find_all(seps).filter(fn(p) { p > 3 }).count()}\")\n\
    for piece in data.split(seps) {\n\
        print(f\"[{piece.len()}]\")\n\
    }\n\
    println(\"\")\n\
    for piece in s.bytes.split(seps) {\n\
        print(f\"<{piece.len()}>\")\n\
    }\n\
    println(\"\")\n\
    let cut = data[1..<5]\n\
    println(f\"{cut.find_all(seps).count()} {pieces(cut, seps)} {pieces(data, seps)}\")\n\
    println(f\"{s.bytes.find_all(vowels).count()} {s.bytes.find(vowels) ?? -1} {data.find_all(high).count()}\")\n\
    let n = 0\n\
    let mut v: Vec[u8] = []\n\
    for _ in 0..<n {\n\
        v.push(';')\n\
    }\n\
    let none = bytes::ByteSet(v)\n\
    println(f\"{data.find(none) ?? -1} {data.find_all(none).count()} {data.split(none).count()}\")\n\
}\n";

const SEARCHES_PRINT: &str =
    "1 1 3\n1 3 5 6 \n4 3 3 6\n2\n[1][1][1][0][0]\n<1><2><1><3>\n2 5 8\n1 7 0\n-1 0 1\n";

#[test]
fn every_search_of_a_byte_set_answers() {
    assert_eq!(run("byte-sets-searches", SEARCHES), SEARCHES_PRINT);
}

/// D2: a set written empty is refused; the help says a computed one may be.
#[test]
fn a_byte_set_written_empty_is_nk1287() {
    let said = checked(
        "byte-sets-empty",
        "use std::bytes\n\nfn main() {\n    let seps = bytes::ByteSet([])\n    println(f\"{\"a;b\".bytes.find(seps) ?? -1}\")\n}\n",
    );
    assert!(said.contains("NK1287"), "{said}");
    assert!(said.contains("computed at run time may be empty"), "{said}");
}

/// D4: `count` right after `find_all` is the producer's own - one method
/// call on its type, which counts per block - and after a `filter` it is the
/// general walk's.
#[test]
fn a_count_right_after_find_all_is_its_own() {
    let rust = lowered(
        "byte-sets-count",
        "use std::bytes\n\nfn main() {\n    let seps = bytes::ByteSet([';'])\n    let data = Bytes(['a', ';'])\n    let n = data.find_all(seps).count()\n    let m = data.find_all(seps).filter(fn(p) { p > 0 }).count()\n    println(f\"{n} {m}\")\n}\n",
    );
    assert!(rust.contains("data.find_all(seps, 0).count()"), "{rust}");
    assert!(rust.contains("data.find_all(seps, 0).filter("), "{rust}");
}

/// `find_all` and `split` against a byte-by-byte reference written in Nikaia,
/// on pseudo-random buffers of every length from 0 to 199 (every block
/// border), from positions inside a block, with bytes above `0x7F` and sets
/// of one, two, three and more bytes.
const AGAINST_A_REFERENCE: &str = "use std::bytes\n\
\n\
fn reference(data: ref Vec[u8], members: ref Vec[u8], from: i64) -> Vec[i64] {\n\
    let mut out: Vec[i64] = []\n\
    let mut i = from\n\
    while i < data.len() {\n\
        if members.contains(data[i]) {\n\
            out.push(i)\n\
        }\n\
        i += 1\n\
    }\n\
    return out\n\
}\n\
\n\
fn main() {\n\
    let alphabet: Vec[u8] = [';', '\\n', 'a', 'e', '\\x00', '\\x80', '\\xFF', '\\x7F', '\\x15', '\\x95', 'x', 'y']\n\
    let sets: Vec[Vec[u8]] = [[';'], [';', '\\n'], ['\\x00', '\\x80', '\\xFF'], ['a', 'e', '\\x15', '\\x95', '\\xFF'], [';', '\\n', 'a', 'e', '\\x00', '\\x80', '\\xFF', '\\x7F', '\\x15', '\\x95', 'x', 'y']]\n\
    let mut seed: u64 = 7\n\
    let mut wrong = 0\n\
    let mut checked = 0\n\
    for members in sets {\n\
        let set = bytes::ByteSet(members.clone())\n\
        for n in 0..<200 {\n\
            let mut raw: Vec[u8] = []\n\
            for _ in 0..<n {\n\
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)\n\
                raw.push(alphabet[((seed >> 33) % 12) as i64])\n\
            }\n\
            let data = Bytes(raw.clone())\n\
            for from in [0, 1, 15, 17, 33, 63, 64, 65, 130] {\n\
                let want = reference(raw, members, from)\n\
                let got: Vec[i64] = data.find_all(set; from: from).collect()\n\
                let first = data.find(set; from: from) ?? -1\n\
                let count = data.find_all(set; from: from).count()\n\
                let last = data.find_all(set; from: from).last() ?? -1\n\
                for s in data.find_all(set; from: from) {\n\
                    if s < from || s >= data.len() {\n\
                        wrong += 1\n\
                    }\n\
                }\n\
                let want_first = if want.len() > 0 { want[0] } else { -1 }\n\
                let want_last = if want.len() > 0 { want[want.len() - 1] } else { -1 }\n\
                if got != want || first != want_first || count != want.len() || last != want_last {\n\
                    wrong += 1\n\
                }\n\
                checked += 1\n\
            }\n\
            let mut pieces = 0\n\
            let mut length = 0\n\
            for piece in data.split(set) {\n\
                pieces += 1\n\
                length += piece.len()\n\
            }\n\
            let hits = reference(raw, members, 0).len()\n\
            if pieces != hits + 1 || length != n - hits {\n\
                wrong += 1\n\
            }\n\
        }\n\
    }\n\
    println(f\"{checked} {wrong}\")\n\
}\n";

#[test]
fn find_all_and_split_agree_with_a_byte_by_byte_reference() {
    assert_eq!(run("byte-sets-reference", AGAINST_A_REFERENCE), "9000 0\n");
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// D5: each position `find_all(set; from: f)` yields is `f <= s < b.len()`
/// for the bounds walk - from the call, and from a `let` the sequence is held
/// under - so `b[s]` and `b[s - f]` (with `f >= 0`) are written without their
/// check under `remove-bounds-checks:on`; after a `filter` the facts are gone
/// and the check stays.
const POSITIONS: &str = "use std::bytes\n\
\n\
fn total(b: ref Bytes) -> i64 {\n\
    let mut n = 0\n\
    for s in b.find_all(bytes::ByteSet([';'])) {\n\
        n += b[s] as i64\n\
    }\n\
    return n\n\
}\n\
\n\
fn from_f(b: ref Bytes, f: i64) -> i64 {\n\
    if f < 0 {\n\
        return 0\n\
    }\n\
    let mut n = 0\n\
    for s in b.find_all(bytes::ByteSet([';']); from: f) {\n\
        n += b[s - f] as i64\n\
    }\n\
    return n\n\
}\n\
\n\
fn kept(b: ref Bytes) -> i64 {\n\
    let mut n = 0\n\
    let ps = b.find_all(bytes::ByteSet([';']))\n\
    for s in ps {\n\
        n += b[s] as i64\n\
    }\n\
    return n\n\
}\n\
\n\
fn filtered(b: ref Bytes) -> i64 {\n\
    let mut n = 0\n\
    for s in b.find_all(bytes::ByteSet([';'])).filter(fn(s) { s > 2 }) {\n\
        n += b[s] as i64\n\
    }\n\
    return n\n\
}\n\
\n\
fn main() {\n\
    let b = Bytes([';', 'a', ';', 'b', ';'])\n\
    println(f\"{total(b)} {from_f(b, 1)} {kept(b)} {filtered(b)}\")\n\
}\n";

#[test]
fn a_position_find_all_yields_indexes_its_bytes_without_a_check() {
    let on = lowered_with(
        "byte-sets-positions",
        POSITIONS,
        &["--optimization", "remove-bounds-checks:on"],
    );
    let unchecked = on.matches("nikaia_std::proven::read(").count();
    assert_eq!(unchecked, 3, "{on}");
    assert!(on.contains(".filter("), "{on}");
    let filtered = &on[on.find("fn filtered").expect("filtered")..];
    let filtered = &filtered[..filtered.find("\n}").expect("its end")];
    assert!(filtered.contains("nikaia_std::index::get(&b"), "{filtered}");
    let off = lowered("byte-sets-positions-off", POSITIONS);
    assert_eq!(off.matches("nikaia_std::proven::read(").count(), 0, "{off}");
    assert_eq!(
        run("byte-sets-positions-run", POSITIONS),
        "177 195 177 59\n"
    );
}

/// ADR-002 D6: the same program at x86-64's baseline (SSE2) and at
/// `x86-64-v3` (AVX2) prints the same.
#[test]
fn every_cpu_level_finds_the_same() {
    if !cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        return;
    }
    let mut printed = Vec::new();
    for cpu in ["--cpu=x86-64", "--cpu=x86-64-v3"] {
        let dir = common::scratch_dir(&format!("byte-sets{cpu}"));
        std::fs::create_dir_all(dir.join("src")).expect("src");
        std::fs::write(
            dir.join("nikaia.toml"),
            "[package]\nname = \"sets\"\nversion = \"0.1.0\"\n",
        )
        .expect("manifest");
        std::fs::write(dir.join("src/main.nika"), AGAINST_A_REFERENCE).expect("entry");
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
        assert!(
            out.status.success(),
            "{cpu}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        printed.push(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    assert_eq!(printed[0], "9000 0\n");
    assert_eq!(printed[0], printed[1]);
}
