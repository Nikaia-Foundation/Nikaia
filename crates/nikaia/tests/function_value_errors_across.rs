//! **`from(p)` crosses the package boundary**
//! ([ADR-351](../../../docs/specification/adr/adr-351.md) D4, #620): a `pub`
//! function that calls the code it was given publishes `throws = ["from(p)"]`,
//! and a caller in another package replaces it with what it passed, so a
//! `catch` over the call takes the passed function's errors by name.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LIB: &str = "pub fn now(f: fn() throws) throws {\n\
                   \x20   f()\n\
                   }\n\
                   \n\
                   fn main() {\n\
                   }\n";

const APP: &str = "use lib\n\
                   \n\
                   enum Bad { Oops }\n\
                   impl Error for Bad {\n\
                   \x20   fn message(ref self) -> String { return \"oops\" }\n\
                   }\n\
                   \n\
                   fn fail() throws {\n\
                   \x20   throw Bad::Oops\n\
                   }\n\
                   \n\
                   fn main() {\n\
                   \x20   lib::now(fail) catch {\n\
                   \x20       match error {\n\
                   \x20           Bad::Oops => println(\"caught oops\")\n\
                   \x20       }\n\
                   \x20   }\n\
                   }\n";

fn the_two(purpose: &str) -> PathBuf {
    let dir = common::scratch_dir(purpose);
    for (package, source, manifest) in [
        (
            "lib",
            LIB,
            "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
        ),
        (
            "app",
            APP,
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n",
        ),
    ] {
        std::fs::create_dir_all(dir.join(package).join("src")).expect("the package");
        std::fs::write(dir.join(package).join("nikaia.toml"), manifest).expect("a manifest");
        std::fs::write(dir.join(package).join("src/main.nika"), source).expect("a source");
    }
    dir
}

fn nikaia(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nikaia"))
        .current_dir(dir)
        .args(args)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("the nikaia binary runs")
}

/// **D4**: the ledger says `from(f)`, not `?`, and the caller's `catch` is
/// exhaustive with the one arm its argument can fail with.
#[test]
fn the_library_publishes_from_its_parameter() {
    let dir = the_two("from-published");
    let build = nikaia(&dir.join("lib"), &["build"]);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(build.status.success(), "{stderr}");
    let ledger = std::fs::read_to_string(dir.join("lib/nikaia.contracts")).expect("the ledger");
    assert!(ledger.contains("throws = [\"from(f)\"]"), "{ledger}");
}

/// **D4, run**: the caller's `catch` takes what it passed, by name.
#[test]
#[ignore = "a named function handed to a `fn() throws` parameter lowers in its own channel, not the box (#620)"]
fn a_catch_over_a_library_call_takes_what_was_passed() {
    let dir = the_two("from-across");
    let run = nikaia(&dir.join("app"), &["run"]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "{stdout}{stderr}");
    assert_eq!(stdout, "caught oops\n", "{stderr}");
    let ledger = std::fs::read_to_string(dir.join("lib/nikaia.contracts")).expect("the ledger");
    assert!(ledger.contains("throws = [\"from(f)\"]"), "{ledger}");
}
