//! A Rust crate reached by `use`, and the reviewed ledger a crate ships beside
//! its `Cargo.toml` ([ADR-290](../../../docs/specification/adr/adr-290.md)
//! D22, Part III 15.2): `nikaia describe` adopts it where its version and its
//! sources still hold, and refuses it where they do not.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn nikaia(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .args(args)
        .arg("--project")
        .arg(dir)
        .env(
            "NIKAIA_CACHE_DIR",
            repo_root().join("target").join("nikaia-project-tests"),
        )
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("the nikaia binary runs")
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

const ENTRY: &str =
    "use fremd\n\nfn main() {\n    let n = fremd::drei()\n    println(f\"{n + 1}\")\n}\n";

/// A project that depends on the path Rust crate `fremd` and writes `use fremd`.
fn a_project_with_a_crate(purpose: &str) -> PathBuf {
    let dir = common::scratch_dir(purpose);
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(
        dir.join("nikaia.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\n\n\
         [dependencies]\nfremd = { type = \"rust\", path = \"fremd\" }\n",
    )
    .expect("manifest");
    std::fs::write(dir.join("src/main.nika"), ENTRY).expect("entry");
    std::fs::create_dir_all(dir.join("fremd/src")).expect("the crate");
    std::fs::write(
        dir.join("fremd/Cargo.toml"),
        "[package]\nname = \"fremd\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("the crate's manifest");
    std::fs::write(dir.join("fremd/src/lib.rs"), "pub fn drei() -> i64 { 3 }\n")
        .expect("the crate's source");
    dir
}

/// The project's drafted description, moved into the crate as the ledger it
/// ships, with a reviewer's line in it.
fn ship_the_draft(dir: &Path) {
    let drafted = nikaia(&["describe", "fremd"], dir);
    assert!(drafted.status.success(), "{}", said(&drafted));
    let text = std::fs::read_to_string(dir.join("contracts/fremd.contracts")).expect("drafted");
    std::fs::write(
        dir.join("fremd/nikaia.contracts"),
        format!("{text}# reviewed in the crate\n"),
    )
    .expect("shipped");
    std::fs::rename(
        dir.join("contracts/fremd.derived"),
        dir.join("fremd/nikaia.derived"),
    )
    .expect("shipped sources");
    std::fs::remove_dir_all(dir.join("contracts")).expect("no project copy");
}

/// **`use` names a Rust crate the manifest declares** (Part III 15.2's `use
/// image`): it was refused as *no dependency called `fremd`*.
#[test]
fn use_names_a_described_rust_crate() {
    let dir = a_project_with_a_crate("shipped-use");
    let described = nikaia(&["describe", "fremd"], &dir);
    assert!(described.status.success(), "{}", said(&described));
    let run = nikaia(&["run"], &dir);
    assert!(run.status.success(), "{}", said(&run));
    assert!(
        String::from_utf8_lossy(&run.stdout).contains('4'),
        "{}",
        said(&run)
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// **A shipped ledger is adopted, not drafted again**: the project's copy is
/// the reviewed text, the reviewer's line included, and the command says where
/// it came from.
#[test]
fn a_shipped_ledger_is_adopted() {
    let dir = a_project_with_a_crate("shipped-adopted");
    ship_the_draft(&dir);
    let adopted = nikaia(&["describe", "fremd"], &dir);
    assert!(adopted.status.success(), "{}", said(&adopted));
    assert!(said(&adopted).contains("adopted"), "{}", said(&adopted));
    let copy = std::fs::read_to_string(dir.join("contracts/fremd.contracts")).expect("copied");
    assert!(copy.contains("# reviewed in the crate"), "{copy}");
    assert!(dir.join("contracts/fremd.derived").is_file());
    let run = nikaia(&["run"], &dir);
    assert!(run.status.success(), "{}", said(&run));
    std::fs::remove_dir_all(&dir).ok();
}

/// **A shipped ledger whose sources moved is refused** (D5's rule, as
/// `NK2505` says it to a build): it describes code that is no longer there.
#[test]
fn a_shipped_ledger_older_than_its_sources_is_refused() {
    let dir = a_project_with_a_crate("shipped-moved");
    ship_the_draft(&dir);
    std::fs::write(
        dir.join("fremd/src/lib.rs"),
        "pub fn drei() -> i64 { 3 }\npub fn vier() -> i64 { 4 }\n",
    )
    .expect("an edit");
    let refused = nikaia(&["describe", "fremd"], &dir);
    assert!(!refused.status.success(), "{}", said(&refused));
    assert!(
        said(&refused).contains("has changed since its shipped description was reviewed"),
        "{}",
        said(&refused)
    );
    assert!(!dir.join("contracts/fremd.contracts").exists());
    std::fs::remove_dir_all(&dir).ok();
}

/// **And one written for another version of the crate.**
#[test]
fn a_shipped_ledger_for_another_version_is_refused() {
    let dir = a_project_with_a_crate("shipped-version");
    ship_the_draft(&dir);
    std::fs::write(
        dir.join("fremd/Cargo.toml"),
        "[package]\nname = \"fremd\"\nversion = \"0.2.0\"\nedition = \"2021\"\n",
    )
    .expect("a new version");
    let refused = nikaia(&["describe", "fremd"], &dir);
    assert!(!refused.status.success(), "{}", said(&refused));
    assert!(
        said(&refused).contains("describes version 0.1.0, and the crate is 0.2.0"),
        "{}",
        said(&refused)
    );
    std::fs::remove_dir_all(&dir).ok();
}
