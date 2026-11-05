//! `nikaia migrate views-3 PATH...` (ADR-views-3 D4, #1213 step 6). The
//! rewriting is Nikaia (`tools/migrate_views3.nika`); this is the adapter: it
//! finds the files, hands the tool the parameters the keep inference names
//! (the checker's own, `tools/keeps.nika`, read through the ledger), and
//! writes back what changed.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use orchestrator::cache::Layout;

use crate::ast::Expr;
use crate::check::parsed_view_of;
use crate::contracts::{Ledger, LedgerOps, std_ledger};
use crate::parser::{Parsed, parse_to_ast, parse_unrefined};
use crate::project::EXTENSION;

/// `nikaia migrate views-3 PATH...`: the exit code. Each file named, and each
/// `.nika` and `.md` file under a directory named, is rewritten in place where
/// it changes, and named. A file of a package with dependencies is refused:
/// the keep inference would need the dependencies' ledgers.
pub fn run(named: &[PathBuf]) -> Result<i32> {
    let mut files = Vec::new();
    for path in named {
        match path.is_dir() {
            true => files_under(path, &mut files)?,
            false => files.push(path.clone()),
        }
    }
    let mut code = 0;
    for file in &files {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        let migrated = if file.extension().is_some_and(|e| e == "md") {
            nikaia_std::tools::migrate_views3::migrate_views3_markdown(&source, &keeps_alone)
        } else {
            match keeps_in_package(file)? {
                Package::None => {
                    nikaia_std::tools::migrate_views3::migrate_views3_nika(&source, &keeps_alone)
                }
                Package::Keeps(dropped, keep_at) => {
                    let by_value =
                        nikaia_std::tools::migrate_views3::migrate_views3_selves(&source);
                    nikaia_std::tools::migrate_views3::migrate_views3_code(
                        &dropped, &keep_at, &by_value,
                    )
                }
                Package::Refused(root) => {
                    eprintln!(
                        "{}: not migrated: its package {} has dependencies, whose ledgers \
                         the keep inference would need",
                        file.display(),
                        root.display()
                    );
                    code = 1;
                    continue;
                }
            }
        };
        if migrated != source {
            std::fs::write(file, &migrated)
                .with_context(|| format!("cannot write {}", file.display()))?;
            println!("migrated {}", file.display());
        }
    }
    Ok(code)
}

enum Package {
    /// Not a source or a test of a package: the file is read alone.
    None,
    /// The file without its `ref`, and where `keep` goes in that, in
    /// characters, read with the package's other files migrated alike.
    Keeps(String, Vec<i64>),
    Refused(PathBuf),
}

/// **A package's source is inferred with its neighbours**, as the checker
/// infers it: a call into another file of the package keeps what that file's
/// function keeps. Every file is read with its `ref` already dropped, as
/// `migrate_views3_nika` reads one alone.
///
/// **A package's test** (`tests/*.nika`) is read as `nikaia test` reads it: a
/// program of its own that reaches the package by its name, so a call into
/// the package keeps what the package's ledger says (#1213, v17's finding:
/// read alone, every call into it failed closed to `keep`).
fn keeps_in_package(file: &Path) -> Result<Package> {
    let layout = Layout::resolve(file);
    let src = layout.root.join("src");
    let tests = layout.root.join("tests");
    let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let in_src = canonical.starts_with(src.canonicalize().unwrap_or(src.clone()));
    let in_tests = canonical.starts_with(tests.canonicalize().unwrap_or(tests.clone()));
    if !layout.in_project || !(in_src || in_tests) {
        return Ok(Package::None);
    }
    let manifest = crate::manifest::Manifest::read(&layout.root.join("nikaia.toml"))?;
    if !manifest.dependencies().is_empty() {
        return Ok(Package::Refused(layout.root.clone()));
    }
    let mut paths = Vec::new();
    crate::fmt::sources_under(&src, &mut paths)?;
    let mut texts = Vec::new();
    let mut units = Vec::new();
    for path in &paths {
        let written = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let text = nikaia_std::tools::migrate_views3::migrate_views3_code(&written, &[], &[]);
        match parse_unrefined(&text) {
            Ok(parsed) => units.push(parsed),
            Err(_) => return Ok(Package::None),
        }
        texts.push(text);
    }
    let mut refs: Vec<&mut Parsed> = units.iter_mut().collect();
    crate::text_tiers::refine_package(&mut refs);
    let shared: Vec<&Parsed> = units.iter().collect();
    let (ledger, checked) = Ledger::infer_package_checked(&shared, std_ledger());
    if in_tests {
        let Some(name) = manifest.package_name() else {
            return Ok(Package::None);
        };
        let written = std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        let text = nikaia_std::tools::migrate_views3::migrate_views3_code(&written, &[], &[]);
        let Ok(parsed) = parse_to_ast(&text) else {
            return Ok(Package::None);
        };
        let mut library = std_ledger().clone();
        library.absorb(Some(name), ledger);
        let (own, mut checked) = Ledger::infer_package_checked(&[&parsed], &library);
        let bytes = keeps_of(&parsed, &own, &library, &checked.remove(0));
        let keep_at = characters(&text, &bytes);
        return Ok(Package::Keeps(text, keep_at));
    }
    let Some(at) = paths
        .iter()
        .position(|path| path.canonicalize().is_ok_and(|p| p == canonical))
    else {
        return Ok(Package::None);
    };
    let bytes = keeps_of(&units[at], &ledger, std_ledger(), &checked[at]);
    let keep_at = characters(&texts[at], &bytes);
    Ok(Package::Keeps(texts.swap_remove(at), keep_at))
}

/// Where `keep` goes in one file read alone (a Markdown block, a script), in
/// characters; nothing where it does not parse.
fn keeps_alone(source: &str) -> Vec<i64> {
    let Ok(parsed) = parse_to_ast(source) else {
        return Vec::new();
    };
    let (ledger, checked) = Ledger::infer_checked(&parsed);
    characters(source, &keeps_of(&parsed, &ledger, std_ledger(), &checked))
}

/// The byte offsets of the parameters the inference keeps and no mark says so.
fn keeps_of(
    parsed: &Parsed,
    own: &Ledger,
    library: &Ledger,
    checked: &crate::check::Checked,
) -> Vec<i64> {
    let own_methods = nikaia_std::tools::ty::method_index(own);
    let library_methods = nikaia_std::tools::ty::method_index(library);
    nikaia_std::tools::migrate_views3::migrate_views3_keeps(
        &parsed_view_of(parsed),
        own,
        library,
        &nikaia_std::tools::keeps::KeepsIndexes {
            own: &own_methods,
            library: &library_methods,
        },
        &checked.methods,
        &|expr: &Expr| crate::emit::literal_expressions(parsed, expr),
        &checked.findings,
    )
}

/// Byte offsets as character indices, as the tool counts.
fn characters(source: &str, bytes: &[i64]) -> Vec<i64> {
    bytes
        .iter()
        .filter(|&&byte| (byte as usize) <= source.len())
        .map(|&byte| source[..byte as usize].chars().count() as i64)
        .collect()
}

/// Every `.nika` and `.md` file under `dir`, in name order, without `target/`
/// and the hidden directories.
fn files_under(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name == "target" || name.starts_with('.') {
                continue;
            }
            files_under(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|e| e == EXTENSION || e == "md")
        {
            files.push(path);
        }
    }
    Ok(())
}
