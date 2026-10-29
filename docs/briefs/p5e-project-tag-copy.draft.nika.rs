use crate::contracts::LedgerOps;

use std::cell::RefCell;

use std::collections::{BTreeMap, BTreeSet};

use std::ffi::OsString;

use std::path::{Path, PathBuf};

use nikaia_std::rt::exec::block_on;

use nikaia_std::tools::project_check;

use nikaia_std::tools::project_lower;

use nikaia_std::tools::project_members::{
    self, ProjectMembersForeignHost, ProjectMembersHost, ProjectMembersManifest,
    ProjectMembersMember, ProjectMembersRefusal, project_members_canonical as canonical,
};

use nikaia_std::tools::project_settings::{self, ProjectSettingsRefusal, ProjectWords};

use nikaia_std::tools::project_tests::{self, ProjectTestsOutcome as Outcome};

pub use nikaia_std::tools::project_tests::{OutputTest, ProgramTest};

use anyhow::{Context, Result, anyhow};

use orchestrator::cache::{Artifacts, Cache, Choices, Layout, Lockfile};

use orchestrator::project::{
    Cargo, CargoProject, CrateKind, Invocation, Package, Profile, Workspace,
    record_extra_dependencies, resolved_versions, toolchain_can_build_for, toolchain_is_new_enough,
    write_if_changed,
};

use crate::contracts::{Ledger, sync};

use crate::emit::Build;

use crate::manifest::{Dependency, Manifest};

use crate::sysroot::{Codegen, Sysroot};

use crate::{assets, check, diagnostics, modules, refuse, refused};

pub use project_settings::{prove_through_loops, skip_free_at_exit};

/// **What this build links against, and what has been described**
/// ([ADR-290](../../docs/specification/adr/adr-290.md) D1): the struct, how a
/// build reads it and what it gives a program's packages are
/// `ProjectMembersForeign` in `tools/project_members.nika` (P4b).
pub use nikaia_std::tools::project_members::ProjectMembersForeign as Foreign;

impl Project {
    /// The program's name for Cargo: in the shared directory its package's
    /// name and [`Self::root_tag`], elsewhere the package's name (#495).
    fn program_name(&self, package: &str) -> String {
        match std::env::var_os("CARGO_TARGET_DIR") {
            Some(_) => package.to_string(),
            None => format!("{package}-{}", self.root_tag()),
        }
    }
    /// What sets this project's program apart in the shared directory: the
    /// start of the hash of where the project is (#495).
    fn root_tag(&self) -> String {
        let mut named = self.root.to_string_lossy().into_owned();
        if let Some(variant) = &self.variant {
            named.push('\u{1f}');
            named.push_str(variant);
        }
        orchestrator::cache::sha256_hex(named.as_bytes())[..16].to_string()
    }
    /// The program a build in the shared directory produced, under the
    /// project's own name in its own `target/nikaia/bin/` (#495): a hard link
    /// where the file system allows one, a copy where not.
    fn own_copy(&self, built: &Path) -> Result<PathBuf> {
        let dir = self.root.join("target").join("nikaia").join("bin");
        std::fs::create_dir_all(&dir)?;
        let name = built
            .file_name()
            .context("a program has a file name")?
            .to_string_lossy()
            .replacen(&format!("-{}", self.root_tag()), "", 1);
        let own = dir.join(name);
        let _ = std::fs::remove_file(&own);
        if std::fs::hard_link(built, &own).is_err() {
            std::fs::copy(built, &own)
                .with_context(|| format!("copying {} to {}", built.display(), own.display()))?;
        }
        Ok(own)
    }
}

/// Record one invocation, if anybody asked. A trace that cannot be written is
/// silent: this is a debugging facility, and a build must not fail because the
/// place it was told to write a note to is unwritable.
fn trace(invocation: &Invocation, verdict: &str) {
    use std::io::Write;

    let Some(path) = std::env::var_os(TRACE_VAR) else {
        return;
    };
    let crate_name = invocation.crate_name.as_deref().unwrap_or("-");
    // **And whether this one is Cargo's target-info probe**
    // ([ADR-166](../../docs/specification/adr/adr-166.md) D1), because that is
    // the invocation whose standard input decides whether a build starts at all
    // — and a trace that cannot say so cannot answer the question the trace
    // exists for.
    let verdict = match invocation.is_a_probe() {
        true => &format!("{verdict} (target-info probe)"),
        false => verdict,
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        writeln!(file, "{crate_name} {verdict}").ok();
    }
}
