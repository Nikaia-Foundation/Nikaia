//! **`nikaia run` without Cargo**, for a program that depends on nothing but
//! `std`: the generated crate is compiled by `rustc` directly and linked
//! against `std` as a dynamic library, the two things build-time code already
//! does ([ADR-321](../../../docs/specification/adr/adr-321.md) D3, D4). Cargo's
//! start and freshness check and the static link of `std` were most of what a
//! one-line edit cost before the program ran.
//!
//! **The program means what its Cargo build means**: the flags are the ones the
//! generated workspace's `dev` profile gives the program's crate (overflow
//! checks on, no debug assertions, no debug information, unwinding), and the
//! bundle is built with what it gives `std` ([`crate::comptime_run::Flavour`]).
//! `rustc`'s diagnostics go to the same translator, as the lines Cargo would
//! have wrapped them in.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::comptime_run::{run_bundle, rustc, unique_beside, write_if_changed};

/// What compiling the program came to.
pub(crate) struct Built {
    /// Where the program is, when it compiled.
    pub(crate) binary: Option<PathBuf>,
    /// What `rustc` said, as Cargo's `compiler-message` lines.
    pub(crate) messages: String,
}

/// **Compile the program in `dir`**, or hand back the one compiled from the
/// same Rust before. `None` where there is no bundle to link against: the
/// caller then takes Cargo's path, which needs none.
///
/// `shown_as` is the generated file Cargo's build compiles: a location `rustc`
/// writes (a diagnostic, a panic's `file:line`) names that file, not the copy
/// compiled here.
pub(crate) fn compile(
    dir: &Path,
    crate_name: &str,
    rust: &str,
    shown_as: &Path,
) -> Result<Option<Built>> {
    let Ok(bundle) = run_bundle() else {
        return Ok(None);
    };
    let binary = dir.join(crate_name);
    let key_file = dir.join("key");
    let said = dir.join("messages");
    let key = crate::assets::digest(
        format!(
            "{rust}\n{}\n{}\n{crate_name}\n{}",
            orchestrator::project::rustc_version(),
            bundle.stamp,
            shown_as.display()
        )
        .as_bytes(),
    );
    // **Built before from the same Rust**: what `rustc` said then is said
    // again, as Cargo replays a fresh crate's warnings.
    if binary.is_file() && std::fs::read_to_string(&key_file).is_ok_and(|held| held == key) {
        return Ok(Some(Built {
            binary: Some(binary),
            messages: std::fs::read_to_string(&said).unwrap_or_default(),
        }));
    }
    let _ = std::fs::remove_file(&key_file);
    let src = dir.join("src");
    let source = src.join(format!("{crate_name}.rs"));
    // **The bundle is linked because the program names it**: an `--extern`
    // nothing names is never loaded, and `std` would be linked statically from
    // its `rlib` instead. At the end, so no line of the program moves.
    write_if_changed(
        &source,
        &format!("{rust}\nextern crate nikaia_bundle as _;\n"),
    )
    .map_err(anyhow::Error::msg)?;
    let scratch = unique_beside(&dir.join("build"));
    std::fs::create_dir_all(&scratch).with_context(|| format!("making {}", scratch.display()))?;
    let built = scratch.join(crate_name);
    // **The toolchain's own `rustc`**, beside its `std`, rather than the
    // `rustup` proxy in front of it: the proxy's start was a sixth of an edit.
    let direct = bundle
        .libdir
        .ancestors()
        .nth(4)
        .map(|sysroot| sysroot.join("bin").join("rustc"))
        .filter(|rustc| std::env::var_os("RUSTC").is_none() && rustc.is_file());
    let mut command = Command::new(direct.unwrap_or_else(|| PathBuf::from(rustc())));
    command
        .current_dir(orchestrator::cache::Layout::where_tools_start())
        .args(["--edition=2024", "--crate-name", crate_name])
        .args(["--crate-type", "bin", "--error-format=json"])
        .args(["-C", "opt-level=0", "-C", "overflow-checks=on"])
        .args(["-C", "debug-assertions=off", "-C", "debuginfo=0"])
        .args(["-C", "panic=unwind", "-C", "prefer-dynamic"])
        .arg("-C")
        .arg(format!("incremental={}", dir.join("incremental").display()))
        .args(["--check-cfg", "cfg(docsrs,test)", "--check-cfg"])
        .arg(match rust.contains("winnow_grammar") {
            true => "cfg(feature, values(\"trace\"))",
            false => "cfg(feature, values())",
        })
        .arg("--extern")
        .arg(format!("nikaia_bundle={}", bundle.dylib.display()))
        .args(bundle.rlibs.iter().flat_map(|(name, rlib)| {
            ["--extern".to_string(), format!("{name}={}", rlib.display())]
        }))
        .arg("-L")
        .arg(format!("dependency={}", bundle.deps.display()));
    // **Found where it was built**, by the path written into the program: the
    // program is `exec`ed as it is (#281), with nothing set around it.
    let mut libraries: Vec<PathBuf> = Vec::new();
    if let Some(parent) = bundle.dylib.parent() {
        libraries.push(parent.to_path_buf());
    }
    libraries.push(bundle.deps.clone());
    libraries.push(bundle.libdir.clone());
    for library in libraries {
        command
            .arg("-C")
            .arg(format!("link-arg=-Wl,-rpath,{}", library.display()));
    }
    let compiled = command
        .arg("--remap-path-prefix")
        .arg(format!("{}={}", source.display(), shown_as.display()))
        .arg("-o")
        .arg(&built)
        .arg(&source)
        .output()
        .context("running rustc")?;
    let mut messages = String::new();
    for line in String::from_utf8_lossy(&compiled.stderr).lines() {
        match line.starts_with('{') {
            true => messages.push_str(&format!(
                "{{\"reason\":\"compiler-message\",\"message\":{line}}}\n"
            )),
            false => eprintln!("{line}"),
        }
    }
    if !compiled.status.success() {
        let _ = std::fs::remove_dir_all(&scratch);
        return Ok(Some(Built {
            binary: None,
            messages,
        }));
    }
    let _ = std::fs::remove_file(&binary);
    std::fs::rename(&built, &binary)
        .with_context(|| format!("moving {} to {}", built.display(), binary.display()))?;
    let _ = std::fs::remove_dir_all(&scratch);
    write_if_changed(&said, &messages).map_err(anyhow::Error::msg)?;
    write_if_changed(&key_file, &key).map_err(anyhow::Error::msg)?;
    Ok(Some(Built {
        binary: Some(binary),
        messages,
    }))
}
