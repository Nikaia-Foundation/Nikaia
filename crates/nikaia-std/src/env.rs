//! `std::env` - the process's environment (Part III 17.1).

/// **The value of the environment variable `name`**, or nothing where it is
/// unset, not Unicode, or empty: the toolchain treats an empty variable as an
/// unset one, so a program never has to test for both.
pub fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The directory for temporary files, as the system names it
/// (`std::env::temp_dir`): `TMPDIR` or `/tmp` on Unix.
pub fn temp_dir() -> crate::fs::Path {
    std::env::temp_dir()
}
