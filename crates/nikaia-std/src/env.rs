//! `std::env` - the process's environment (Part III 17.1).

/// **The value of the environment variable `name`**, or nothing where it is
/// unset, not Unicode, or empty: the toolchain treats an empty variable as an
/// unset one, so a program never has to test for both.
pub fn var(name: &str) -> Option<crate::block::Text> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .map(crate::block::Text::from)
}

/// The directory for temporary files, as the system names it
/// (`std::env::temp_dir`): `TMPDIR` or `/tmp` on Unix.
pub fn temp_dir() -> crate::fs::Path {
    std::env::temp_dir()
}

/// **Every variable of the environment, as the operating system holds it**
/// (#1601): each name and value as raw bytes, in the order the system lists
/// them, a name with an empty value and bytes that are not Unicode included.
/// The one primitive under `env::vars`, which is Nikaia
/// (`runtime/env.nika`): the prefix, what is left out and the order are
/// decided there, not here.
pub(crate) fn pairs() -> Vec<(crate::bytes::Bytes, crate::bytes::Bytes)> {
    std::env::vars_os()
        .map(|(name, value)| {
            (
                crate::bytes::Bytes::from(name.into_encoded_bytes()),
                crate::bytes::Bytes::from(value.into_encoded_bytes()),
            )
        })
        .collect()
}

/// `env::vars(prefix)`: its Nikaia half, `runtime/env.nika` (#1601, ADR-397).
pub use crate::runtime::vars;
