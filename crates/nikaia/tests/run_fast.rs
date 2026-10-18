//! **`nikaia run` without Cargo** (ADR-321 D3/D4 for a program): a program
//! that depends on nothing but `std` is compiled by `rustc` directly and linked
//! against `std` as a dynamic library. What it prints, how it stops and what a
//! backend error says are what the Cargo build gives; a program with a Rust
//! dependency still takes Cargo's path.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The cache `tests/project.rs` shares, so `std` is built once for both.
fn shared_cache_dir() -> PathBuf {
    repo_root().join("target").join("nikaia-project-tests")
}

fn a_project(purpose: &str, manifest: &str, entry: &str) -> PathBuf {
    let dir = common::scratch_dir(purpose);
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(dir.join("nikaia.toml"), manifest).expect("manifest");
    std::fs::write(dir.join("src/main.nika"), entry).expect("entry");
    dir
}

fn nikaia(args: &[&str], dir: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .args(args)
        .current_dir(dir)
        .env("NIKAIA_CACHE_DIR", shared_cache_dir())
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("NIKAIA_WRAPPER_TRACE")
        .output()
        .expect("the nikaia binary runs")
}

/// **A `rustc` that denies warnings for one crate**, the program's (Cargo
/// names a binary `<name>-<hash>`, so the name is a prefix): the
/// author's choice, as `-D warnings` is, so what it refuses is a refusal of
/// the language below that is no defect of this compiler (#653). Every other
/// call - `-vV`, `std`'s build - is the toolchain's own.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rustc_denying_warnings(dir: &Path, crate_name: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let script = dir.join("rustc-denying-warnings");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ncase \" $* \" in\n  *\" --crate-name {crate_name}\"*) exec rustc -D warnings \"$@\" ;;\nesac\nexec rustc \"$@\"\n"
        ),
    )
    .expect("the wrapper");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("executable");
    script
}

fn said(output: &std::process::Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The program compiled without Cargo, where it is.
fn fast_binary(project: &Path, name: &str) -> PathBuf {
    project.join("target/nikaia/fast").join(name)
}

/// A single file is run, edited and run again: the new output appears, and it
/// was compiled without Cargo.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn an_edited_script_runs_with_its_new_output() {
    let dir = common::scratch_dir("run-fast-edit");
    std::fs::create_dir_all(&dir).expect("dir");
    let script = dir.join("hello.nika");
    std::fs::write(&script, "fn main() {\n    println(\"erster\")\n}\n").expect("script");

    let first = nikaia(&["run", "hello.nika"], &dir);
    assert!(first.status.success(), "{}", said(&first));
    assert_eq!(String::from_utf8_lossy(&first.stdout), "erster\n");

    std::fs::write(&script, "fn main() {\n    println(\"zweiter\")\n}\n").expect("edit");
    let second = nikaia(&["run", "hello.nika"], &dir);
    assert!(second.status.success(), "{}", said(&second));
    assert_eq!(
        String::from_utf8_lossy(&second.stdout),
        "zweiter\n",
        "{}",
        said(&second)
    );
    let cached = std::fs::read_dir(shared_cache_dir().join("run"))
        .expect("the run cache")
        .flatten()
        .any(|entry| fast_binary(&entry.path(), "hello").is_file());
    assert!(cached, "the script was compiled without Cargo");
    std::fs::remove_dir_all(&dir).ok();
}

/// An overflow stops a program compiled without Cargo with the same words and
/// the same exit code as the program `nikaia build` makes.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn an_overflow_aborts_as_the_cargo_build_does() {
    let dir = a_project(
        "run-fast-overflow",
        "[package]\nname = \"grow\"\nversion = \"0.1.0\"\n",
        "fn grow(n: i32) -> i32 {\n    return n * 2\n}\n\n\
         fn main() {\n    println(\"before\")\n    println(f\"{grow(2000000000)}\")\n}\n",
    );
    let project = dir.to_string_lossy().to_string();

    let ran = nikaia(&["run", "--project", &project], &dir);
    assert!(fast_binary(&dir, "grow").is_file(), "{}", said(&ran));
    let built = nikaia(&["build", "--project", &project], &dir);
    assert!(built.status.success(), "{}", said(&built));
    let cargo = Command::new(dir.join("target/nikaia/bin/grow"))
        .output()
        .expect("the Cargo build runs");

    assert!(!ran.status.success(), "{}", said(&ran));
    assert!(
        said(&ran).contains("attempt to multiply with overflow"),
        "{}",
        said(&ran)
    );
    assert_eq!(ran.status.code(), cargo.status.code(), "{}", said(&ran));
    assert_eq!(ran.stdout, cargo.stdout, "{}", said(&ran));
    assert_eq!(
        String::from_utf8_lossy(&ran.stderr),
        String::from_utf8_lossy(&cargo.stderr)
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A program with a Rust dependency is built by Cargo, as before.
#[test]
fn a_program_with_a_rust_dependency_takes_cargos_path() {
    let dir = common::scratch_dir("run-fast-rust-dependency");
    std::fs::create_dir_all(dir.join("helper/src")).expect("helper");
    std::fs::write(
        dir.join("helper/Cargo.toml"),
        "[package]\nname = \"helper\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("helper manifest");
    std::fs::write(dir.join("helper/src/lib.rs"), "pub fn one() -> i32 { 1 }\n").expect("lib");
    std::fs::create_dir_all(dir.join("app/src")).expect("app");
    std::fs::write(
        dir.join("app/nikaia.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n\
         [dependencies]\nhelper = { type = \"rust\", path = \"../helper\" }\n",
    )
    .expect("app manifest");
    std::fs::write(
        dir.join("app/src/main.nika"),
        "fn main() {\n    println(\"through cargo\")\n}\n",
    )
    .expect("entry");
    let project = dir.join("app").to_string_lossy().to_string();

    let ran = nikaia(&["run", "--project", &project], &dir);
    assert!(ran.status.success(), "{}", said(&ran));
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "through cargo\n");
    assert!(
        !fast_binary(&dir.join("app"), "app").exists(),
        "a program with a Rust dependency is not compiled without Cargo"
    );
    assert!(
        dir.join("app/target/nikaia/bin/app").is_file(),
        "Cargo built it: {}",
        said(&ran)
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// An error in the generated Rust is said against the `.nika` line, as the
/// Cargo build says it. The program is one the frontend passes, with an
/// unused name, and the `rustc` it is compiled with denies warnings
/// ([`rustc_denying_warnings`]).
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn a_backend_error_is_still_said_in_nikaias_words() {
    let dir = a_project(
        "run-fast-backend-error",
        "[package]\nname = \"refused\"\nversion = \"0.1.0\"\n",
        "fn main() {\n    let n: i64 = 1\n    let unused = n + 1\n    println(f\"{n}\")\n}\n",
    );
    let project = dir.to_string_lossy().to_string();
    let rustc = rustc_denying_warnings(&dir, "refused");

    let ran = Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .args(["run", "--project", &project])
        .current_dir(&dir)
        .env("NIKAIA_CACHE_DIR", shared_cache_dir())
        .env("RUSTC", &rustc)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("NIKAIA_WRAPPER_TRACE")
        .output()
        .expect("the nikaia binary runs");
    assert!(!ran.status.success(), "{}", said(&ran));
    let complaint = said(&ran);
    assert!(
        complaint.contains("Unused variable") && !complaint.contains("error[NK"),
        "the backend refused it, not the checker: {complaint}"
    );
    assert!(
        complaint.contains("src/main.nika:3:"),
        "the message names the `.nika` line: {complaint}"
    );
    assert!(complaint.contains("let unused = n + 1"), "{complaint}");
    assert!(!complaint.contains("refused.rs"), "{complaint}");
    assert!(
        !dir.join("target/nikaia/bin").exists(),
        "Cargo was not started: {complaint}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
