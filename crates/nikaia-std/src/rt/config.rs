//! The runtime configuration file, read at startup
//! ([ADR-303](../../../../docs/specification/adr/adr-303.md) D5).
//!
//! Four settings and no fifth. They are read when the program starts, by the
//! person running it on the machine it runs on - which is why they are *not*
//! `nikaia.toml` keys: a build-time value cannot be tuned by an operator, who
//! is not the person who compiled it. `nikaia.toml` keeps what the compiler
//! must know (`target` and `user_parallelism`,
//! [ADR-037](../../../../docs/specification/adr/adr-037.md) D5) and nothing
//! else.
//!
//! | key | what it decides | default |
//! | :--- | :--- | :--- |
//! | `io-workers` | how many I/O threads run | `1` |
//! | `user-pool` | how large the pool for user code is, at `user_parallelism = yes` | `0` = as many as the machine has |
//! | `io-method` | which mechanism serves a file: `auto`, `uring`, `blocking` | `auto` |
//! | `cleanup-deadline` | how long shutdown drains before cancelling the rest | `30s` |
//!
//! `io-method`'s default is `auto` and it can be pinned - the same shape
//! `--ordering strict` has: the default decides, and a person may overrule it
//! to rule something out in the field. Pinning `uring` on a machine that has
//! no `io_uring` is a *failure to start*, not a silent fallback, because the
//! whole point of pinning is to find out.
//!
//! `cleanup-deadline` arrives here from `nikaia.toml`, where Part III 13.3 had
//! put it. How long a program waits at exit for pending cleanup
//! ([ADR-297](../../../../docs/specification/adr/adr-297.md) D6) is an
//! operating property.

use std::path::{Path, PathBuf};

pub use crate::runtime::run_config::{Config, Method};

/// Where the runtime looks for its configuration, unless the environment says
/// otherwise.
///
/// **Not decided by ADR-303 D5 says "a runtime configuration file, read at
/// startup" and names the four settings; it does not name the file or its
/// search path. This is one constant and one function so that naming it is a
/// one-line change when the decision is made.
pub const FILE: &str = "nikaia-runtime.toml";

/// The environment variable that names the file outright, for the operator who
/// has one binary and several deployments of it.
pub const PATH_VAR: &str = "NIKAIA_RUNTIME_CONFIG";

/// Read the file, wherever it is. Absent is not an error: a program with no
/// configuration file runs on the defaults, and littering one into an
/// operator's directory to make that work would be the wrong trade.
///
/// The text is read by `config_read` in `runtime/run_config.nika`; what is
/// left here touches the machine.
pub fn load() -> Result<Config, String> {
    match path() {
        Some(path) => {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("reading {}: {e}", path.display()))?;
            let read = crate::runtime::run_config::config_read(&text);
            if read.refused.is_empty() {
                Ok(read.config)
            } else {
                Err(format!("in {}: {}", path.display(), read.refused))
            }
        }
        None => Ok(crate::runtime::run_config::config_default()),
    }
}

/// The file this process will read, if there is one.
///
/// The variable first, because it is what one binary in several deployments
/// needs; then the working directory, which is where an operator running
/// `./server` puts it.
pub fn path() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os(PATH_VAR) {
        return Some(PathBuf::from(named));
    }
    let here = Path::new(FILE);
    here.is_file().then(|| here.to_path_buf())
}
