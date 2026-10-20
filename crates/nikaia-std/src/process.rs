// crates/nikaia-std/src/process.rs
//
// `std::process`: start another program and wait for what it says
// ([ADR-243](../../../docs/specification/adr/adr-243.md), on
// [ADR-290](../../../docs/specification/adr/adr-290.md) D14's decision that
// `std` gains a subprocess).
//
// **One call, run to the end.** `run` starts a program, hands it its
// arguments, waits for it to finish and hands back its exit code and its two
// output streams as text. That is the shape every caller this tree has wants -
// `cargo metadata` for `nikaia describe`, `rustc` for a compiler written in
// Nikaia - and it is the one that needs no handle a program has to close.
//
// **A non-zero exit is an answer, not a failure.** What fails is not being
// able to start the program, or output that is not text; how the program
// ended is `code` and `ok`, and the caller decides what a `1` means. A tool's
// refusal is information the tool printed, and a `catch` would lose it.
//
// **The wait gives the thread up** (D3). The child is waited for on a thread
// of its own and the calling task parks on the runtime's bell, so a program at
// `user_parallelism = no` keeps serving its other tasks while a build runs.
//
// **What the run cost is that one program's** ([ADR-368](../../../docs/specification/adr/adr-368.md)
// D3). The clock is read just before the start and just after the wait; the
// wait is `wait4` on the child (crate `child-usage`, which holds the `unsafe`),
// after both pipes are read to their end - waiting first would leave a child
// that fills a pipe blocked for ever.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::io::IoError;

/// What a program that ran to its end left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The exit code, or `-1` where the program was ended by a signal and has
    /// none (D2).
    pub code: i64,
    /// Whether the program says it succeeded: an exit code of `0`.
    pub ok: bool,
    /// Everything it wrote to standard output.
    pub stdout: String,
    /// Everything it wrote to standard error.
    pub stderr: String,
    /// From just before it started to just after it was waited for, on the
    /// monotonic clock.
    pub elapsed: Duration,
    /// The processor time it and everything it waited for used in user mode.
    /// Zero where the platform is not Unix.
    pub user_time: Duration,
    /// The same in kernel mode. Zero where the platform is not Unix.
    pub system_time: Duration,
    /// The most memory it held resident, in KiB, as the operating system
    /// reports it for that one program; `None` where the platform is not Unix.
    /// On Linux never below the caller's own high-water mark before the start
    /// (ADR-368 D3).
    pub max_rss_kib: Option<i64>,
}

/// What the thread beside the task hands back: the two streams' bytes, and
/// how the program ended with what it used.
struct Ran {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    waited: child_usage::Waited,
    elapsed: Duration,
}

/// Start the program, read both pipes to their end - standard error on a
/// thread of its own, so that neither pipe fills while the other is read -
/// and only then wait for it.
fn started(
    program: &Path,
    args: &[String],
    dir: &str,
    environment: Option<Vec<(OsString, OsString)>>,
) -> std::io::Result<Ran> {
    let started = Instant::now();
    let mut command = Command::new(program);
    if let Some(environment) = environment {
        command.env_clear().envs(environment);
    }
    let mut child = command
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut errors = child.stderr.take();
    let beside = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        if let Some(pipe) = errors.as_mut() {
            pipe.read_to_end(&mut bytes)?;
        }
        Ok(bytes)
    });
    let mut stdout = Vec::new();
    let read = match child.stdout.take() {
        Some(mut pipe) => pipe.read_to_end(&mut stdout).map(|_| ()),
        None => Ok(()),
    };
    let stderr = beside
        .join()
        .unwrap_or_else(|_| Err(std::io::Error::other("reading standard error panicked")));
    let waited = child_usage::wait(child)?;
    let elapsed = started.elapsed();
    read?;
    Ok(Ran {
        stdout,
        stderr: stderr?,
        waited,
        elapsed,
    })
}

/// Start `program` with `args` in the directory `dir`, wait for it to finish,
/// and hand back what it said.
///
/// `program` is looked for on the caller's `PATH` where it has no path
/// separator in it, and started by the full name found (ADR-243 D6). The
/// child's environment is the caller's, changed by `env_clear`, then
/// `env_remove`, then `env` (D5); an entry no platform accepts starts nothing
/// and throws `IoError::Other` naming every one (D7). It fails with
/// `IoError::NotFound(program)` where nothing is there,
/// `IoError::PermissionDenied(program)` where it may not be run, and
/// `IoError::NotText` where either stream is not UTF-8.
pub async fn run(
    program: &str,
    args: Vec<String>,
    dir: &str,
    env: Vec<(String, String)>,
    env_remove: Vec<String>,
    env_clear: bool,
) -> Result<Output, IoError> {
    let invalid = invalid_entries(&env, &env_remove);
    if !invalid.is_empty() {
        return Err(IoError::Other(format!(
            "{program}: the environment has entries no platform accepts: {}",
            invalid.join("; ")
        )));
    }
    let named = program.to_string();
    let path = std::env::var_os("PATH");
    let found = found_on_path(
        program,
        path.as_deref(),
        std::env::var_os("PATHEXT").as_deref(),
        cfg!(windows),
    )
    .ok_or_else(|| IoError::NotFound(PathBuf::from(&named)))?;
    let environment = (env_clear || !env_remove.is_empty() || !env.is_empty()).then(|| {
        merged(
            std::env::vars_os(),
            env_clear,
            &env_remove,
            &env,
            cfg!(windows),
        )
    });
    let dir = dir.to_string();
    let finished = crate::rt::io::beside(move || started(&found, &args, &dir, environment))
        .await
        .map_err(|e| IoError::of(e, &named))?;
    let stdout = String::from_utf8(finished.stdout)
        .map_err(|_| IoError::NotText(format!("{named}'s standard output")))?;
    let stderr = String::from_utf8(finished.stderr)
        .map_err(|_| IoError::NotText(format!("{named}'s standard error")))?;
    let status = finished.waited.status;
    Ok(Output {
        code: status.code().map(i64::from).unwrap_or(-1),
        ok: status.success(),
        stdout,
        stderr,
        elapsed: finished.elapsed,
        user_time: finished.waited.user_time,
        system_time: finished.waited.system_time,
        max_rss_kib: finished.waited.max_rss_kib,
    })
}

/// **Every environment entry no platform accepts**, described (ADR-243 D7): a
/// name that is empty or holds `=` or a NUL, in `env` or in `env_remove`, and
/// a value in `env` that holds a NUL. All of them, so one error names each.
pub fn invalid_entries(env: &[(String, String)], env_remove: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    let name_fault = |name: &str| -> Option<&'static str> {
        if name.is_empty() {
            Some("is empty")
        } else if name.contains('=') {
            Some("holds `=`")
        } else if name.contains('\0') {
            Some("holds a NUL")
        } else {
            None
        }
    };
    for (name, value) in env {
        if let Some(fault) = name_fault(name) {
            found.push(format!("the name {name:?} in `env` {fault}"));
        }
        if value.contains('\0') {
            found.push(format!("the value of {name:?} in `env` holds a NUL"));
        }
    }
    for name in env_remove {
        if let Some(fault) = name_fault(name) {
            found.push(format!("the name {name:?} in `env_remove` {fault}"));
        }
    }
    found
}

/// Whether two environment names are the same variable: byte for byte, or
/// without regard to ASCII case where `windows` (ADR-243 D5).
fn same_name(a: &OsStr, b: &OsStr, windows: bool) -> bool {
    if windows {
        a.as_encoded_bytes()
            .eq_ignore_ascii_case(b.as_encoded_bytes())
    } else {
        a == b
    }
}

/// **The child's environment** (ADR-243 D5): `inherited`, or nothing where
/// `clear`; without each name in `remove`; then each pair in `set`, in order,
/// a later pair replacing an earlier one of the same name. Names compare as
/// the platform compares them, `windows` saying which.
pub fn merged(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
    clear: bool,
    remove: &[String],
    set: &[(String, String)],
    windows: bool,
) -> Vec<(OsString, OsString)> {
    let mut environment: Vec<(OsString, OsString)> = if clear {
        Vec::new()
    } else {
        inherited.into_iter().collect()
    };
    for name in remove {
        environment.retain(|(held, _)| !same_name(held, OsStr::new(name), windows));
    }
    for (name, value) in set {
        environment.retain(|(held, _)| !same_name(held, OsStr::new(name), windows));
        environment.push((OsString::from(name), OsString::from(value)));
    }
    environment
}

/// **Where `program` is started from** (ADR-243 D6): itself where it holds a
/// path separator, otherwise the first file of that name on the caller's
/// `path` that may be run - with each `pathext` ending tried where `windows`
/// and the name has none. A directory on `path` that is relative, the empty
/// one among them, is the caller's working directory, so the name found does
/// not depend on the child's `dir`.
pub fn found_on_path(
    program: &str,
    path: Option<&OsStr>,
    pathext: Option<&OsStr>,
    windows: bool,
) -> Option<PathBuf> {
    let separated = program.contains('/') || (windows && program.contains('\\'));
    if separated {
        return Some(PathBuf::from(program));
    }
    if program.is_empty() {
        return None;
    }
    let endings: Vec<String> = if windows && Path::new(program).extension().is_none() {
        pathext
            .and_then(OsStr::to_str)
            .unwrap_or(".COM;.EXE;.BAT;.CMD")
            .split(';')
            .filter(|ending| !ending.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        vec![String::new()]
    };
    let here = std::env::current_dir().ok();
    for directory in std::env::split_paths(path?) {
        let directory = match (directory.is_absolute(), &here) {
            (true, _) => directory,
            (false, Some(here)) => here.join(directory),
            (false, None) => directory,
        };
        for ending in &endings {
            let candidate = directory.join(format!("{program}{ending}"));
            if runnable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// Whether `candidate` is a file this process may start.
fn runnable(candidate: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(candidate) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

// **`exit` throws `Exit`, a fatal error** ([ADR-345](../../../docs/specification/adr/adr-345.md)).
//
// It travels in the error channel like any error, so every frame it leaves
// runs its cleanups as at a `return`. A `catch` takes it only in an arm that
// names it (the compiler writes the pass-through). Arriving at the end of
// `main`, the emitted `fn main` hands the outcome to [`ended`] after the
// runtime finished its drain, and the process ends with the code (D4).

/// What `process::exit(code)` throws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Code(i64),
}

impl std::fmt::Display for Exit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exit::Code(code) => write!(f, "the program ends with exit code {code}"),
        }
    }
}

impl std::error::Error for Exit {}

/// Throw [`Exit`] with `code`. A code outside `0..=255` is an unrecoverable
/// error (ADR-343 D4).
pub fn exit(code: i64) -> Result<(), Exit> {
    if !(0..=255).contains(&code) {
        panic!("process::exit: the exit code {code} is not in 0..=255");
    }
    Err(Exit::Code(code))
}

/// Whether a failure channel holds an [`Exit`], and its code.
///
/// `exit_code` is an `Exit`'s, which ends `main` without a message;
/// `fatal_code` is any fatal error's (ADR-345 D1): an `Exit`'s code, `1` for a
/// program's own `impl Fatal for T` (D4) - what a detached task's fatal error
/// sets at `main`'s end and what a supervisor does not restart (ADR-346 D6, D7).
pub trait Ending {
    fn exit_code(&self) -> Option<i32>;
    fn fatal_code(&self) -> Option<i32> {
        self.exit_code()
    }
}

type FatalProbe = fn(&(dyn std::error::Error + 'static)) -> Option<i32>;

/// The program's own fatal types, for the opaque channel, which cannot ask a
/// trait: `main` registers each before its first statement.
static FATAL: std::sync::Mutex<Vec<FatalProbe>> = std::sync::Mutex::new(Vec::new());

/// **Register a program's fatal type** (ADR-345 D1) so that a box holding it
/// is known to be fatal.
pub fn fatal<T: Ending + std::error::Error + 'static>() {
    FATAL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(|error| error.downcast_ref::<T>().and_then(Ending::fatal_code));
}

fn registered_fatal(error: &(dyn std::error::Error + 'static)) -> Option<i32> {
    let probes = FATAL.lock().unwrap_or_else(|e| e.into_inner());
    probes.iter().find_map(|probe| probe(error))
}

impl Ending for Exit {
    fn exit_code(&self) -> Option<i32> {
        match self {
            Exit::Code(code) => Some(*code as i32),
        }
    }
}

impl<E: Ending> Ending for crate::error::Thrown<E> {
    fn exit_code(&self) -> Option<i32> {
        self.held().exit_code()
    }

    fn fatal_code(&self) -> Option<i32> {
        self.held().fatal_code()
    }
}

/// The opaque channel: an `Exit` inside it, as itself or thrown, still ends the
/// program with its code (ADR-346 D5: a joined task's `Exit` arrives this way).
impl Ending for Box<dyn std::error::Error> {
    fn exit_code(&self) -> Option<i32> {
        if let Some(exit) = self.downcast_ref::<Exit>() {
            return exit.exit_code();
        }
        let inner = opened(&**self);
        if let Some(exit) = inner.downcast_ref::<Exit>() {
            return exit.exit_code();
        }
        known(inner).and_then(|known| known.exit_code())
    }

    fn fatal_code(&self) -> Option<i32> {
        let error: &(dyn std::error::Error + 'static) = &**self;
        let inner = opened(error);
        if let Some(exit) = inner.downcast_ref::<Exit>() {
            return exit.exit_code();
        }
        if let Some(known) = known(inner) {
            return known.fatal_code();
        }
        registered_fatal(error).or_else(|| registered_fatal(inner))
    }
}

/// The error under a [`crate::error::Raised`] envelope, or the error itself.
fn opened<'a>(
    error: &'a (dyn std::error::Error + 'static),
) -> &'a (dyn std::error::Error + 'static) {
    match error.downcast_ref::<crate::error::Raised>() {
        Some(raised) => raised.inner(),
        None => error,
    }
}

/// `std`'s own carriers of a fatal error: a supervisor's hand-over (ADR-346
/// D7) and an error that crossed from the pool.
fn known<'a>(error: &'a (dyn std::error::Error + 'static)) -> Option<&'a dyn Ending> {
    if let Some(handed) = error.downcast_ref::<crate::supervisor::Handed>() {
        return Some(handed);
    }
    if let Some(stopped) = error.downcast_ref::<crate::supervisor::Stopped>() {
        return Some(stopped);
    }
    if let Some(carried) = error.downcast_ref::<crate::task::Carried>() {
        return Some(carried);
    }
    None
}

/// `main`'s outcome after the runtime finished: an `Exit` ends the process with
/// its code; anything else is handed back for the usual end.
pub fn ended<T, E: Ending>(outcome: Result<T, E>) -> Result<T, E> {
    if let Err(error) = &outcome
        && let Some(code) = error.exit_code()
    {
        std::process::exit(code);
    }
    outcome
}

#[cfg(test)]
mod exit_tests {
    use super::Ending;

    #[test]
    fn exit_throws_its_code() {
        let thrown = super::exit(3).unwrap_err();
        assert_eq!(thrown.exit_code(), Some(3));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(list: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        list.iter()
            .map(|(n, v)| (OsString::from(n), OsString::from(v)))
            .collect()
    }

    fn owned(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn set_after_remove_and_later_pair_wins() {
        let inherited = pairs(&[("HOME", "/h"), ("A", "old"), ("B", "b")]);
        let got = merged(
            inherited,
            false,
            &["A".to_string(), "B".to_string()],
            &owned(&[("A", "1"), ("C", "x"), ("C", "y")]),
            false,
        );
        assert_eq!(got, pairs(&[("HOME", "/h"), ("A", "1"), ("C", "y")]));
    }

    #[test]
    fn clear_then_remove_does_nothing() {
        let got = merged(
            pairs(&[("HOME", "/h")]),
            true,
            &["HOME".to_string(), "NOPE".to_string()],
            &owned(&[("ONLY", "1")]),
            false,
        );
        assert_eq!(got, pairs(&[("ONLY", "1")]));
    }

    #[test]
    fn names_compare_by_case_on_unix_and_without_on_windows() {
        let inherited = pairs(&[("Path", "C:\\w"), ("path", "/u")]);
        let unix = merged(
            inherited.clone(),
            false,
            &[],
            &owned(&[("PATH", "/p")]),
            false,
        );
        assert_eq!(
            unix,
            pairs(&[("Path", "C:\\w"), ("path", "/u"), ("PATH", "/p")])
        );
        let windows = merged(
            inherited.clone(),
            false,
            &[],
            &owned(&[("PATH", "x")]),
            true,
        );
        assert_eq!(windows, pairs(&[("PATH", "x")]));
        let removed = merged(inherited, false, &["PATH".to_string()], &[], true);
        assert!(removed.is_empty());
    }

    #[test]
    fn every_invalid_entry_is_named() {
        let found = invalid_entries(
            &owned(&[("", "x"), ("A=B", "y"), ("OK", "v\0w"), ("FINE", "z")]),
            &["N\0".to_string()],
        );
        assert_eq!(found.len(), 4, "{found:?}");
        assert!(invalid_entries(&owned(&[("FINE", "z")]), &["X".to_string()]).is_empty());
    }

    #[test]
    fn a_name_with_a_separator_is_not_looked_up() {
        assert_eq!(
            found_on_path("./x", Some(OsStr::new("")), None, false),
            Some(PathBuf::from("./x"))
        );
        assert_eq!(
            found_on_path(
                "definitely-not-a-program-726",
                std::env::var_os("PATH").as_deref(),
                None,
                false
            ),
            None
        );
    }
}
