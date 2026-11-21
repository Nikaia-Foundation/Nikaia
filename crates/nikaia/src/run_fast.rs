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

#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::comptime_run::{run_bundle, rustc, unique_beside, write_if_changed};
use nikaia_std::compiler::emit_program::EMIT_PROGRAM_COUNTING_HEAP;

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
    let key = crate::assets::digest(
        format!(
            "{rust}\n{}\n{}\n{crate_name}\n{}",
            crate::comptime_run::rustc_version(),
            bundle.stamp,
            shown_as.display()
        )
        .as_bytes(),
    );
    // **One build of this program at a time** (#1128): runs of the same file
    // at the same moment share the source, the key, the program and `rustc`'s
    // incremental directory, and two that differ (one with `-O`) could leave
    // the key of one beside the program of the other. The lock is held until
    // the program is in place; a run that waited finds it built below.
    let _ = std::fs::create_dir_all(dir);
    let _alone = orchestrator::cache::alone(&dir.join("lock"));
    // **A place for each Rust the program was compiled from**: the same file
    // run as `scripts/x.nika` and as `/abs/scripts/x.nika` names its path in
    // the generated places table either way (ADR-300 D9), so its Rust and key
    // differ; with one place, runs of the two kinds took it from each other
    // and every run compiled the program again.
    let slots = dir.join("slots");
    let slot = slots.join(&key[..16]);
    let binary = slot.join(crate_name);
    let key_file = slot.join("key");
    let said = slot.join("messages");
    // **Built before from the same Rust**: what `rustc` said then is said
    // again, as Cargo replays a fresh crate's warnings. The key file's time
    // says when the place was last used, which is what pruning keeps.
    if binary.is_file() && std::fs::read_to_string(&key_file).is_ok_and(|held| held == key) {
        let _ = std::fs::File::options()
            .append(true)
            .open(&key_file)
            .and_then(|file| file.set_modified(std::time::SystemTime::now()));
        return Ok(Some(Built {
            binary: Some(binary),
            messages: String::from(std::fs::read_to_string(&said).unwrap_or_default()),
        }));
    }
    let _ = std::fs::remove_file(&key_file);
    // The single place of before, held by nothing now.
    let _ = std::fs::remove_file(dir.join("key"));
    let _ = std::fs::remove_file(dir.join("messages"));
    let _ = std::fs::remove_file(dir.join(crate_name));
    let src = dir.join("src");
    let source = src.join(format!("{crate_name}.rs"));
    // **The bundle is linked because the program names it**: an `--extern`
    // nothing names is never loaded, and `std` would be linked statically from
    // its `rlib` instead. At the end, so no line of the program moves.
    //
    // **The bundle declares the program's allocator** (ADR-327 D9, #1844): a
    // dynamic library allocates with its own, so the program's two lines are
    // blanked, line for line, and its allocations and the library's are
    // counted by the one allocator.
    let rust = rust.replacen(EMIT_PROGRAM_COUNTING_HEAP, "\n", 1);
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
        .args(
            bundle.rlibs.iter().flat_map(|(name, rlib)| {
                ["--extern".to_text(), format!("{name}={}", rlib.display())]
            }),
        );
    // Each directory Cargo built one of the bundle's crates in, the proc
    // macros' for the host among them (#1705, #2136).
    for dir in &bundle.deps {
        command
            .arg("-L")
            .arg(format!("dependency={}", dir.display()));
    }
    // **Found where it was built**, by the path written into the program: the
    // program is `exec`ed as it is (#281), with nothing set around it.
    let mut libraries: Vec<PathBuf> = Vec::new();
    if let Some(parent) = bundle.dylib.parent() {
        libraries.push(parent.to_path_buf());
    }
    libraries.extend(bundle.deps.first().cloned());
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
    // **Renamed over the old program, never removed first** (#1128): a run
    // of the same file at the same moment may have this path in hand, and a
    // removal before the rename left it nothing to start. The rename puts the
    // new program in place at once; one started already keeps the old.
    std::fs::create_dir_all(&slot).with_context(|| format!("making {}", slot.display()))?;
    std::fs::rename(&built, &binary)
        .with_context(|| format!("moving {} to {}", built.display(), binary.display()))?;
    let _ = std::fs::remove_dir_all(&scratch);
    write_if_changed(&said, &messages).map_err(anyhow::Error::msg)?;
    std::fs::write(&key_file, key.as_bytes())
        .with_context(|| format!("writing {}", key_file.display()))?;
    prune_slots(&slots);
    Ok(Some(Built {
        binary: Some(binary),
        messages,
    }))
}

/// **The places used last are kept**, [`KEPT_SLOTS`] of them; the others
/// are removed. A run that started a removed program keeps it, as after a
/// rename over it.
fn prune_slots(slots: &Path) {
    let Ok(entries) = std::fs::read_dir(slots) else {
        return;
    };
    let mut used: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .map(|slot| {
            let when = std::fs::metadata(slot.join("key"))
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            (when, slot)
        })
        .collect();
    used.sort_by_key(|slot| std::cmp::Reverse(slot.0));
    for (_, slot) in used.into_iter().skip(KEPT_SLOTS) {
        let _ = std::fs::remove_dir_all(slot);
    }
}

/// How many compiled forms of one program the run cache keeps: the ways one
/// file is run at the same time (a relative and an absolute path, with and
/// without a setting) are few.
const KEPT_SLOTS: usize = 4;

/// **Whether the user's Cargo would link the program differently** (#786):
/// a linker or flags for `triple` in a configuration file Cargo reads when
/// started where the tools start (that directory upward, then `CARGO_HOME`;
/// [ADR-325](../../../docs/specification/adr/adr-325.md) D3), or in the
/// environment. `rustc` started here reads none of them, so such a program
/// takes Cargo's path, which applies them; they are not handed to `rustc`.
pub(crate) fn cargo_sets_linker_or_flags(triple: &str) -> bool {
    let shouted = triple.to_uppercase().replace(['-', '.'], "_");
    let set = |var: &str| std::env::var_os(var).is_some_and(|v| !v.is_empty());
    let environment = [
        "RUSTFLAGS".to_text(),
        "CARGO_ENCODED_RUSTFLAGS".to_text(),
        "CARGO_BUILD_RUSTFLAGS".to_text(),
        format!("CARGO_TARGET_{shouted}_LINKER"),
        format!("CARGO_TARGET_{shouted}_RUSTFLAGS"),
    ];
    if environment.iter().any(|var| set(var)) {
        return true;
    }
    let start = orchestrator::cache::Layout::where_tools_start();
    let start = start.canonicalize().unwrap_or(start);
    let home = std::env::var_os("CARGO_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    let dirs = start.ancestors().map(|dir| dir.join(".cargo")).chain(home);
    dirs.flat_map(|dir| [dir.join("config.toml"), dir.join("config")])
        .filter(|file| file.is_file())
        .any(|file| config_sets_linker_or_flags(&file, triple))
}

/// One configuration file's answer to [`cargo_sets_linker_or_flags`]. A
/// file that cannot be read as TOML counts as setting them: Cargo, which
/// reads it too, then says what is wrong with it.
fn config_sets_linker_or_flags(file: &Path, triple: &str) -> bool {
    let Ok(config) = std::fs::read_to_string(file)
        .map_err(|_| ())
        .and_then(|text| text.parse::<toml::Table>().map_err(|_| ()))
    else {
        return true;
    };
    let has = |table: Option<&toml::Value>, keys: &[&str]| {
        table
            .and_then(toml::Value::as_table)
            .is_some_and(|table| keys.iter().any(|key| table.contains_key(*key)))
    };
    let build = has(config.get("build"), &["rustflags"]);
    // A `cfg(...)` table may apply to the host as well; it is not evaluated.
    let target = config
        .get("target")
        .and_then(toml::Value::as_table)
        .is_some_and(|targets| {
            targets.iter().any(|(name, table)| {
                (name == triple || name.starts_with("cfg("))
                    && has(Some(table), &["linker", "rustflags"])
            })
        });
    build || target
}
