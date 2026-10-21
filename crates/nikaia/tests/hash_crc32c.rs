//! **`hash::crc32c(crc, word)` is one CRC-32C step** (ADR-371 D1, D5): the
//! CRC-32C of `word`'s eight bytes, lowest first, continued from `crc`,
//! without a checksum's inversions. The same value at every CPU level: a
//! table at x86-64's baseline, one `crc32` at `x86-64-v3`.

mod common;

use nikaia::contracts::{Ledger, LedgerOps, STD};
use nikaia::parser::parse_to_ast;

fn codes(source: &str) -> Vec<String> {
    let parsed = parse_to_ast(source).expect("the source parses");
    let own = Ledger::infer(&parsed);
    let library = Ledger::parse(STD).expect("std ships a ledger");
    nikaia::check::check_program(&parsed, &own, &library, &std::collections::BTreeSet::new())
        .findings
        .into_iter()
        .map(|f| f.code.to_string())
        .collect()
}

/// Two single steps and a chain of a thousand over xorshift words; the
/// values are the bitwise definition's (the reflected polynomial
/// `0x82F63B78`), computed apart from Nikaia.
const CRC: &str = "use std::hash\n\n\
     fn main() {\n    \
     println(f\"{hash::crc32c(0, 0x0123456789abcdef)}\")\n    \
     println(f\"{hash::crc32c(0xFFFFFFFF, 0x3837363534333231)}\")\n    \
     let mut w: u64 = 0x9E3779B97F4A7C15\n    \
     let mut c: u32 = 0\n    \
     for _ in 0..<1000 {\n        \
     w = w ^ (w << 13)\n        \
     w = w ^ (w >> 7)\n        \
     w = w ^ (w << 17)\n        \
     c = hash::crc32c(c, w)\n    \
     }\n    \
     println(f\"{c}\")\n\
     }\n";

#[test]
fn crc32c_is_described() {
    assert_eq!(codes(CRC), Vec::<String>::new());
}

#[test]
fn crc32c_at_every_cpu_level() {
    if !cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        return;
    }
    for cpu in ["--cpu=x86-64", "--cpu=x86-64-v3"] {
        let dir = common::scratch_dir(&format!("crc32c{cpu}"));
        std::fs::create_dir_all(dir.join("src")).expect("src");
        std::fs::write(
            dir.join("nikaia.toml"),
            "[package]\nname = \"crc\"\nversion = \"0.1.0\"\n",
        )
        .expect("manifest");
        std::fs::write(dir.join("src/main.nika"), CRC).expect("entry");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
            .args(["run", cpu, "--project"])
            .arg(&dir)
            .env(
                "NIKAIA_CACHE_DIR",
                root.join("target").join("nikaia-project-tests"),
            )
            .env_remove("CARGO_TARGET_DIR")
            .output()
            .expect("the nikaia binary runs");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "3919080105\n2675474277\n4259472278\n",
            "{cpu}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
