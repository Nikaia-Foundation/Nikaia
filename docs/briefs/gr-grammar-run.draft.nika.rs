use std::path::{Path, PathBuf};

use std::process::Command;

use crate::ast::Item;

use crate::contracts::ty::Ty;

use crate::contracts::ty::TyOps;

use crate::parser::Parsed;

use nikaia_std::tools::dump::{FieldShape, Shapes, VariantShape};

/// Why a grammar could not be run, where the reason is this compiler's rather
/// than the input's: declared in Nikaia (`tools/check_reports.nika`, ADR-294,
/// #125), the checker's report reads it as it is.
pub use nikaia_std::tools::check_reports::CheckWall as Wall;

impl Workshop {
    fn compile_and_run(&self, at: &Path, parsed: &Parsed, ask: &Ask<'_>) -> Result<String, Wall> {
        let program = driver(parsed, ask)?;
        // **Keyed on what went into it** — the driver text carries the
        // grammar's lowering, the types it uses and the dump, so a digest of it
        // is a digest of everything this sub-project is.
        let key = crate::assets::digest(program.as_bytes());
        let dir = at.join(&key);
        let source = dir.join("src").join("main.rs");
        write_if_changed(&source, &program)?;
        write_if_changed(&dir.join("Cargo.toml"), &manifest(&key, &program))?;

        // **One target directory for every parser on the machine**: what a
        // parser depends on - `std`, `winnow`, the grammar runtime - is the
        // same for all of them, and under each project's workshop it was
        // compiled again for every new project (half a minute before the
        // first grammar could run). Each parser is a crate of its own name
        // (`p<key>`), so they share the directory without meeting.
        let target = shared_target();
        let built = Command::new(cargo())
            .current_dir(orchestrator::cache::Layout::where_tools_start())
            .arg("build")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(dir.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .output()
            .map_err(|error| Wall::DidNotBuild {
                detail: format!("running cargo: {error}"),
            })?;
        if !built.status.success() {
            return Err(Wall::DidNotBuild {
                detail: String::from_utf8_lossy(&built.stderr).to_string(),
            });
        }

        // **The bytes go in a file**, because that is what a parser is pointed
        // at and what the diagnostic's line and column are counted against.
        let input = dir.join("input");
        write_if_changed(&input, ask.input)?;

        let binary = target.join("debug").join(format!("p{key}"));
        let ran = Command::new(&binary)
            .arg(&input)
            .output()
            .map_err(|error| Wall::DidNotBuild {
                detail: format!("running {}: {error}", binary.display()),
            })?;
        match ran.status.success() {
            true => Ok(String::from_utf8_lossy(&ran.stdout).trim().to_string()),
            false => Err(Wall::Refused {
                detail: String::from_utf8_lossy(&ran.stderr).to_string(),
            }),
        }
    }
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// **Written only when it changed**, which is what keeps Cargo's own freshness
/// working: a file rewritten with identical bytes is newer than the fingerprint
/// and rebuilds for ever. The build cache learned this the same way
/// ([ADR-021](../../../docs/specification/adr/adr-021.md)).
fn write_if_changed(path: &Path, text: &str) -> Result<(), Wall> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| Wall::DidNotBuild {
            detail: format!("creating {}: {error}", parent.display()),
        })?;
    }
    if std::fs::read_to_string(path).is_ok_and(|held| held == text) {
        return Ok(());
    }
    std::fs::write(path, text).map_err(|error| Wall::DidNotBuild {
        detail: format!("writing {}: {error}", path.display()),
    })
}

/// The sub-project's manifest.
///
/// **Its own workspace**, because it lives under a `target/` that may be inside
/// one: a member Cargo did not expect is an error about a file nobody wrote.
/// The dependencies are the ones a generated program already gets
/// (`project::runtime_dependencies`), so the parser compiled here is the parser
/// the program links.
/// Where every grammar run's parser is compiled: the user cache, beside the
/// compiled `std` (ADR-002 D4).
///
/// One entry per compiler, so an old compiler's parsers are evicted as a whole
/// (#615).
fn shared_target() -> PathBuf {
    let compiler = orchestrator::cache::sha256_hex(
        format!(
            "{}\n{}",
            env!("CARGO_PKG_VERSION"),
            env!("NIKAIA_RUSTC_VERSION")
        )
        .as_bytes(),
    );
    let entry = orchestrator::cache::Layout::user_cache_dir()
        .join("build-time-parsers")
        .join(&compiler[..16]);
    orchestrator::cache::use_entry(&entry);
    entry.join("target")
}
