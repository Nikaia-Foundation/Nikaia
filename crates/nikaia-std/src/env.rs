//! `std::env` - the process's environment (Part III 17.1).

/// **The value of the environment variable `name`**, or nothing where it is
/// unset, not Unicode, or empty: the toolchain treats an empty variable as an
/// unset one, so a program never has to test for both.
pub fn var(name: &str) -> Option<crate::block::Text> {
    if launcher_var(name.as_bytes()) {
        return None;
    }
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
        .filter(|(name, _)| !launcher_var(name.as_encoded_bytes()))
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

/// **The variable a launcher hands the program it started in**
/// ([ADR-program-path-1](../../../docs/specification/adr/adr-program-path-1.md)
/// D3): the path `env::current_exe` answers, set by `nikaia run` and
/// `nikaia test` to the `nikaia` that was started.
pub const EXE_VAR: &str = "NIKAIA_EXE";

/// **The variable a launcher hands the file the program stands in as**
/// (ADR-program-path-1 D3): the path `env::current_main` answers, set by
/// `nikaia run` to the `.nika` file it was given and by `nikaia test` to the
/// test's file.
pub const MAIN_VAR: &str = "NIKAIA_MAIN";

/// One of the launcher's variables, where it is set and not empty.
fn handed(name: &str) -> Option<crate::fs::Path> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(crate::fs::Path::from)
}

/// **Whether `name` is one of the launcher's variables**, which a program
/// does not see (ADR-program-path-1 D3): `env::var` answers nothing for it,
/// `env::vars` leaves it out, and a process the program starts does not
/// inherit it (`process.rs`), so a child never takes its parent's answer.
pub(crate) fn launcher_var(name: &[u8]) -> bool {
    name == EXE_VAR.as_bytes() || name == MAIN_VAR.as_bytes()
}

/// **The program that was started** (Part III 17.1, ADR-program-path-1 D1):
/// under `nikaia run` and `nikaia test` the `nikaia` that runs it, for a built
/// program the program itself; nothing where the system cannot name it.
pub fn current_exe() -> Option<crate::fs::Path> {
    handed(EXE_VAR).or_else(|| std::env::current_exe().ok())
}

/// **The file the program stands in as it was started** (Part III 17.1,
/// ADR-program-path-1 D2): under `nikaia run x.nika` the file `x.nika`, under
/// `nikaia run` in a project its entry point, under `nikaia test` the test's
/// file, for a built program the program itself; nothing where the system
/// cannot name it.
pub fn current_main() -> Option<crate::fs::Path> {
    handed(MAIN_VAR).or_else(|| std::env::current_exe().ok())
}
