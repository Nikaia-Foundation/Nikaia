//! **`target::has(i)` answers for the CPU level the program is built for**
//! (ADR-371 D6): in a `comptime` and at run time alike, `false` for all three
//! at x86-64's baseline and `true` at `x86-64-v3`. One project is built at one
//! level and then the other, so a cached answer of the first level would show.

mod common;

const HAS: &str = "use std::target\n\n\
     comptime CRC: bool = target::has(target::Instruction::Crc32c)\n\
     comptime BMI: bool = target::has(target::Instruction::Bmi2)\n\
     comptime AVX: bool = target::has(target::Instruction::Avx2)\n\n\
     fn main() {\n    \
     println(f\"{CRC} {BMI} {AVX}\")\n    \
     let crc = target::has(target::Instruction::Crc32c)\n    \
     let bmi = target::has(target::Instruction::Bmi2)\n    \
     let avx = target::has(target::Instruction::Avx2)\n    \
     println(f\"{crc} {bmi} {avx}\")\n\
     }\n";

/// `nikaia check` computes the `comptime`s, so it is the binary that is asked:
/// the program has no findings.
#[test]
fn has_is_described() {
    let dir = common::scratch_dir("target-has-check");
    std::fs::create_dir_all(&dir).expect("dir");
    let file = dir.join("has.nika");
    std::fs::write(&file, HAS).expect("file");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("the nikaia binary runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn has_answers_for_the_level_built_for() {
    if !cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        return;
    }
    let dir = common::scratch_dir("target-has");
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(
        dir.join("nikaia.toml"),
        "[package]\nname = \"has\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    std::fs::write(dir.join("src/main.nika"), HAS).expect("entry");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let run = |cpu: &str| {
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
        (
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        )
    };
    let (baseline, said) = run("--cpu=x86-64");
    assert_eq!(baseline, "false false false\nfalse false false\n", "{said}");
    let (v3, said) = run("--cpu=x86-64-v3");
    assert_eq!(v3, "true true true\ntrue true true\n", "{said}");
    let (again, said) = run("--cpu=x86-64");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(again, "false false false\nfalse false false\n", "{said}");
}
