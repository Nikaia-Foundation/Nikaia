//! **A cap on the processes a child may hold, and its group ended with its
//! starter** (#1919). A test program that forks without end must fail on its
//! own cap, not fill the machine's PID space, and must not outlive the runner
//! that started it.
//!
//! * [`Cap`] puts the child into a `pids` cgroup of its own, below the
//!   starter's, with `pids.max` set to the cap, where such a cgroup can be
//!   made (cgroup v1 or v2, Linux). Where it cannot and the starter is not
//!   root, it lowers the child's `RLIMIT_NPROC` to the tasks the user already
//!   runs plus the cap; root is exempt from that limit, so there the child
//!   runs uncapped. Only the cgroup tells whether the cap was reached.
//! * [`guard_group`] forks a small guard that waits on a pipe from the
//!   starter. When the starter ends by any cause, the kernel closes the pipe,
//!   and the guard ends the child's process group (and the cap's cgroup); a
//!   starter that ends normally says so first, and the guard leaves.

use std::ffi::CString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// The prefix of the cgroups a cap makes; the starter's process id follows,
/// so one whose starter is gone is found and removed by the next [`Cap::new`].
const PREFIX: &str = "nikaia-test-";

/// A cap of `limit` tasks (processes and threads) for one child and every
/// process it starts. Dropping it ends what is left in its cgroup and removes
/// the cgroup.
#[derive(Debug)]
pub struct Cap {
    limit: u32,
    cgroup: Option<PathBuf>,
    nproc: Option<u64>,
}

impl Cap {
    /// A cap of `limit` tasks, made as the module says; a cap of zero caps
    /// nothing.
    pub fn new(limit: u32) -> Cap {
        let mut cap = Cap {
            limit,
            cgroup: None,
            nproc: None,
        };
        if limit == 0 {
            return cap;
        }
        cap.cgroup = made_cgroup(limit);
        if cap.cgroup.is_none() {
            cap.nproc = user_limit(limit);
        }
        cap
    }

    /// The cap in tasks, zero for none.
    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// Where the cap is a cgroup, its directory.
    pub fn cgroup(&self) -> Option<&Path> {
        self.cgroup.as_deref()
    }

    /// Have `command`'s child enter the cap before it runs a line of its own:
    /// it writes itself into the cgroup's `cgroup.procs`, or lowers its own
    /// `RLIMIT_NPROC`, between the fork and the `exec`.
    pub fn apply(&self, command: &mut Command) {
        let procs = self.cgroup.as_ref().and_then(|dir| {
            CString::new(
                dir.join("cgroup.procs")
                    .into_os_string()
                    .into_encoded_bytes(),
            )
            .ok()
        });
        let nproc = self.nproc;
        if procs.is_none() && nproc.is_none() {
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // SAFETY: the hook runs in the child between `fork` and `exec`,
            // where only async-signal-safe calls may be made; it calls only
            // `open`, `write`, `close` and `setrlimit`, on a path made before
            // the fork, and allocates nothing (README, fourth row).
            unsafe {
                command.pre_exec(move || entered(procs.as_ref(), nproc));
            }
        }
    }

    /// Whether a fork or a thread was refused because the cap was reached:
    /// the cgroup's `pids.events` counts it under `max`. Without a cgroup it
    /// cannot be told, and is false.
    pub fn reached(&self) -> bool {
        let Some(dir) = &self.cgroup else {
            return false;
        };
        std::fs::read_to_string(dir.join("pids.events")).is_ok_and(|events| {
            events.lines().any(|line| {
                line.strip_prefix("max ")
                    .and_then(|count| count.trim().parse::<u64>().ok())
                    .is_some_and(|count| count > 0)
            })
        })
    }

    /// End every task left in the cap's cgroup, and in the cgroups below it.
    pub fn end_all(&self) {
        if let Some(dir) = &self.cgroup {
            ended(dir);
        }
    }
}

impl Drop for Cap {
    fn drop(&mut self) {
        if let Some(dir) = self.cgroup.take() {
            ended(&dir);
            removed(&dir);
        }
    }
}

/// What a child does between `fork` and `exec` to enter its cap.
#[cfg(unix)]
fn entered(procs: Option<&CString>, nproc: Option<u64>) -> io::Result<()> {
    if let Some(procs) = procs {
        // SAFETY: `procs` is a NUL-terminated path that outlives the calls;
        // the buffer written is a one-byte static (README, fourth row).
        unsafe {
            let fd = libc::open(procs.as_ptr(), libc::O_WRONLY | libc::O_CLOEXEC);
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let wrote = libc::write(fd, b"0".as_ptr().cast(), 1);
            let error = io::Error::last_os_error();
            libc::close(fd);
            if wrote != 1 {
                return Err(error);
            }
        }
    }
    if let Some(nproc) = nproc {
        let limit = libc::rlimit {
            rlim_cur: nproc,
            rlim_max: nproc,
        };
        // SAFETY: a pointer to a local `rlimit` that outlives the call
        // (README, fourth row).
        if unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &limit) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// The directory of this process's `pids` cgroup: under a cgroup v1 mount
/// that carries the `pids` controller, or under the cgroup v2 mount where
/// that hierarchy has it.
#[cfg(target_os = "linux")]
fn own_cgroup() -> Option<PathBuf> {
    let membership = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let mounts = std::fs::read_to_string("/proc/self/mountinfo").ok()?;
    // mountinfo: id parent major:minor root mount-point options ... - type source super-options
    let mount_of = |wanted: &dyn Fn(&str, &str) -> bool| {
        mounts.lines().find_map(|line| {
            let (before, after) = line.split_once(" - ")?;
            let fields: Vec<&str> = before.split(' ').collect();
            let mut rest = after.split(' ');
            let kind = rest.next()?;
            let options = rest.nth(1).unwrap_or("");
            (fields.len() > 4 && wanted(kind, options))
                .then(|| (fields[3].to_string(), fields[4].to_string()))
        })
    };
    let inside = |root: &str, mount: &str, path: &str| {
        let below = path
            .strip_prefix(root)
            .unwrap_or(path)
            .trim_start_matches('/');
        Path::new(mount).join(below)
    };
    for line in membership.lines() {
        let mut parts = line.splitn(3, ':');
        let (_, controllers, path) = (parts.next()?, parts.next()?, parts.next()?);
        if controllers.split(',').any(|c| c == "pids") {
            let (root, mount) = mount_of(&|kind, options| {
                kind == "cgroup" && options.split(',').any(|o| o == "pids")
            })?;
            return Some(inside(&root, &mount, path));
        }
    }
    let path = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?;
    let (root, mount) = mount_of(&|kind, _| kind == "cgroup2")?;
    let dir = inside(&root, &mount, path);
    std::fs::read_to_string(dir.join("cgroup.subtree_control"))
        .is_ok_and(|on| on.split_whitespace().any(|c| c == "pids"))
        .then_some(dir)
}

#[cfg(not(target_os = "linux"))]
fn own_cgroup() -> Option<PathBuf> {
    None
}

/// A new cgroup below this process's own with `pids.max` at `limit`;
/// nothing where none can be made. The cgroups of starters that are gone
/// are ended and removed first.
fn made_cgroup(limit: u32) -> Option<PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = own_cgroup()?;
    swept(&parent);
    let me = std::process::id();
    let dir = parent.join(format!(
        "{PREFIX}{me}-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).ok()?;
    if std::fs::write(dir.join("pids.max"), limit.to_string()).is_err() {
        removed(&dir);
        return None;
    }
    Some(dir)
}

/// End and remove the cgroups below `parent` that a starter made which is
/// no longer running.
fn swept(parent: &Path) {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(starter) = name
            .to_str()
            .and_then(|name| name.strip_prefix(PREFIX))
            .and_then(|rest| rest.split('-').next())
        else {
            continue;
        };
        if !Path::new("/proc").join(starter).exists() {
            let dir = entry.path();
            ended(&dir);
            removed(&dir);
        }
    }
}

/// Kill every task in the cgroup `dir` and in the cgroups below it, until
/// none is left there or a few rounds have passed.
fn ended(dir: &Path) {
    for _ in 0..200 {
        let mut left = false;
        for task in tasks(dir) {
            left = true;
            kill(task);
        }
        if !left {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// The tasks of the cgroup `dir` and of every cgroup below it.
fn tasks(dir: &Path) -> Vec<i32> {
    let mut all: Vec<i32> = std::fs::read_to_string(dir.join("cgroup.procs"))
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|pid| pid.parse().ok())
        .collect();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                all.extend(tasks(&entry.path()));
            }
        }
    }
    all
}

/// Remove the cgroup `dir`, the ones below it first.
fn removed(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                removed(&entry.path());
            }
        }
    }
    let _ = std::fs::remove_dir(dir);
}

#[cfg(unix)]
fn kill(pid: i32) {
    if pid > 0 {
        // SAFETY: two integers; a stale pid at worst fails (README, third row).
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill(_: i32) {}

/// For a starter that is not root, on Linux: the tasks its user runs now
/// plus `limit`, where that is below the hard limit it has; else nothing.
#[cfg(target_os = "linux")]
fn user_limit(limit: u32) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: `getuid` takes nothing and cannot fail.
    let uid = unsafe { libc::getuid() };
    if uid == 0 {
        return None;
    }
    let mut running = 0u64;
    for entry in std::fs::read_dir("/proc").ok()?.flatten() {
        let mine = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.bytes().all(|b| b.is_ascii_digit()))
            && entry.metadata().is_ok_and(|meta| meta.uid() == uid);
        if mine {
            running += std::fs::read_dir(entry.path().join("task"))
                .map(|tasks| tasks.count() as u64)
                .unwrap_or(1);
        }
    }
    let wanted = running + u64::from(limit);
    let mut now = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: a pointer to a local `rlimit` that outlives the call.
    if unsafe { libc::getrlimit(libc::RLIMIT_NPROC, &mut now) } != 0 {
        return None;
    }
    (now.rlim_max == libc::RLIM_INFINITY || now.rlim_max > wanted).then_some(wanted)
}

#[cfg(not(target_os = "linux"))]
fn user_limit(_: u32) -> Option<u64> {
    None
}

/// **Wait until `child` has ended, without reaping it**: its process id
/// stays taken, as a zombie, so ending its group afterwards cannot reach a
/// process that took the id over. Reap it with `wait` after.
#[cfg(unix)]
pub fn wait_unreaped(child: &Child) -> io::Result<()> {
    let pid = libc::id_t::from(child.id());
    loop {
        // SAFETY: `siginfo_t` is plain data, all-zero valid; the pointer is
        // to that local, which outlives the call; `pid` is a child of this
        // process not yet reaped, since `Child` owns it (README, fifth row).
        let got = unsafe {
            let mut info: libc::siginfo_t = core::mem::zeroed();
            libc::waitid(libc::P_PID, pid, &mut info, libc::WEXITED | libc::WNOWAIT)
        };
        if got == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

/// See the Unix version: elsewhere there is nothing to wait for first.
#[cfg(not(unix))]
pub fn wait_unreaped(_: &Child) -> io::Result<()> {
    Ok(())
}

/// The guard [`guard_group`] started: [`GroupGuard::release`] (or dropping
/// it) tells it the starter ended normally, and waits for it.
#[derive(Debug)]
pub struct GroupGuard {
    #[cfg(unix)]
    write: Option<std::os::fd::OwnedFd>,
    #[cfg(unix)]
    pid: libc::pid_t,
}

impl GroupGuard {
    /// Tell the guard to leave without ending anything, and reap it.
    pub fn release(mut self) {
        self.released();
    }

    #[cfg(unix)]
    fn released(&mut self) {
        use std::io::Write;
        if let Some(write) = self.write.take() {
            let mut pipe = std::fs::File::from(write);
            let _ = pipe.write_all(b"x");
            drop(pipe);
            let mut status = 0;
            loop {
                // SAFETY: `pid` is the guard this process forked and has not
                // reaped; the pointer is to a local (README, sixth row).
                let got = unsafe { libc::waitpid(self.pid, &mut status, 0) };
                if got == self.pid
                    || io::Error::last_os_error().kind() != io::ErrorKind::Interrupted
                {
                    break;
                }
            }
        }
    }

    #[cfg(not(unix))]
    fn released(&mut self) {}
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        self.released();
    }
}

/// **Have the group led by `pid` end with this process** (#1919): fork a
/// guard that ignores the signals a terminal or a group kill sends, leaves
/// this process's group, and blocks reading a pipe only this process writes.
/// When this process ends - killed included - the pipe reads its end, and
/// the guard kills the group `pid` leads, and every task in `cgroup` where
/// one is given (its `cgroup.procs` only, not the cgroups below), removes
/// that cgroup, then exits. [`GroupGuard::release`] sends it a byte first, and it exits
/// without killing anything.
#[cfg(unix)]
pub fn guard_group(pid: u32, cgroup: Option<&Path>) -> io::Result<GroupGuard> {
    use std::os::fd::{FromRawFd, OwnedFd};
    let group = libc::pid_t::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a process id out of range"))?;
    let procs = cgroup.and_then(|dir| {
        CString::new(
            dir.join("cgroup.procs")
                .into_os_string()
                .into_encoded_bytes(),
        )
        .ok()
    });
    let dir = cgroup.and_then(|dir| CString::new(dir.as_os_str().as_encoded_bytes()).ok());
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: a pointer to a local array of two ints, which `pipe2` fills
    // (README, sixth row).
    if unsafe { pipe2_cloexec(&mut fds) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let (read, write) = (fds[0], fds[1]);
    // SAFETY: the child calls only async-signal-safe functions, allocates
    // nothing and leaves with `_exit` (README, sixth row).
    let forked = unsafe { libc::fork() };
    if forked < 0 {
        let error = io::Error::last_os_error();
        // SAFETY: both are descriptors this function just opened.
        unsafe {
            libc::close(read);
            libc::close(write);
        }
        return Err(error);
    }
    if forked == 0 {
        // SAFETY: as above; this is the forked guard.
        unsafe { guarding(read, group, procs.as_ref(), dir.as_ref()) }
    }
    // SAFETY: `read` is a descriptor this function opened and owns.
    unsafe {
        libc::close(read);
    }
    Ok(GroupGuard {
        // SAFETY: `write` is a descriptor this function opened and nobody
        // else owns.
        write: Some(unsafe { OwnedFd::from_raw_fd(write) }),
        pid: forked,
    })
}

/// See the Unix version: elsewhere no guard is started.
#[cfg(not(unix))]
pub fn guard_group(_: u32, _: Option<&Path>) -> io::Result<GroupGuard> {
    Ok(GroupGuard {})
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
unsafe fn pipe2_cloexec(fds: &mut [libc::c_int; 2]) -> libc::c_int {
    // SAFETY: the caller hands a local array of two ints.
    unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_os = "freebsd"))
))]
unsafe fn pipe2_cloexec(fds: &mut [libc::c_int; 2]) -> libc::c_int {
    // SAFETY: the caller hands a local array of two ints; each end is then
    // marked close-on-exec (a fork in between holds them a moment longer).
    unsafe {
        let got = libc::pipe(fds.as_mut_ptr());
        if got == 0 {
            libc::fcntl(fds[0], libc::F_SETFD, libc::FD_CLOEXEC);
            libc::fcntl(fds[1], libc::F_SETFD, libc::FD_CLOEXEC);
        }
        got
    }
}

/// The guard's whole life, in the forked child: nothing here allocates.
#[cfg(unix)]
unsafe fn guarding(
    read: libc::c_int,
    group: libc::pid_t,
    procs: Option<&CString>,
    dir: Option<&CString>,
) -> ! {
    // SAFETY: every call below is async-signal-safe and takes integers or
    // pointers to locals and to `procs`, made before the fork.
    unsafe {
        for signal in [
            libc::SIGINT,
            libc::SIGTERM,
            libc::SIGHUP,
            libc::SIGQUIT,
            libc::SIGPIPE,
        ] {
            libc::signal(signal, libc::SIG_IGN);
        }
        libc::setpgid(0, 0);
        // Hold nothing but the pipe: another starter's pipes, held here,
        // would keep its reads from ending.
        libc::dup2(read, 0);
        closed_from(1);
        let mut byte = 0u8;
        let got = loop {
            let got = libc::read(0, (&raw mut byte).cast(), 1);
            if got >= 0 || *errno() != libc::EINTR {
                break got;
            }
        };
        if got != 1 {
            libc::kill(-group, libc::SIGKILL);
            if let Some(procs) = procs {
                for _ in 0..200 {
                    if !killed_listed(procs) {
                        break;
                    }
                    libc::usleep(5000);
                }
            }
            // An emptied cgroup is removed, or the next cap sweeps it.
            if let Some(dir) = dir {
                libc::rmdir(dir.as_ptr());
            }
        }
        libc::_exit(0)
    }
}

/// Close every descriptor from `low` up.
#[cfg(unix)]
unsafe fn closed_from(low: libc::c_int) {
    #[cfg(target_os = "linux")]
    // SAFETY: integers only.
    let done = unsafe {
        libc::syscall(
            libc::SYS_close_range,
            low as libc::c_uint,
            libc::c_uint::MAX,
            0,
        )
    } == 0;
    #[cfg(not(target_os = "linux"))]
    let done = false;
    if !done {
        for fd in low..4096 {
            // SAFETY: integers only; a descriptor not open fails with EBADF.
            unsafe {
                libc::close(fd);
            }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn errno() -> *mut libc::c_int {
    // SAFETY: the thread's own errno.
    unsafe { libc::__errno_location() }
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
unsafe fn errno() -> *mut libc::c_int {
    // SAFETY: the thread's own errno.
    unsafe { libc::__error() }
}

/// Kill every task `procs` lists; whether it listed any. Reads into a buffer
/// on the stack and parses it by hand, since the guard may not allocate.
#[cfg(unix)]
unsafe fn killed_listed(procs: &CString) -> bool {
    let mut buffer = [0u8; 16384];
    let mut any = false;
    // SAFETY: `procs` is NUL-terminated; the buffer is a local of the size
    // given.
    unsafe {
        let fd = libc::open(procs.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC);
        if fd < 0 {
            return false;
        }
        let mut pid: libc::pid_t = 0;
        loop {
            let got = libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len());
            if got <= 0 {
                break;
            }
            for &b in &buffer[..got as usize] {
                if b.is_ascii_digit() {
                    pid = pid
                        .saturating_mul(10)
                        .saturating_add(libc::pid_t::from(b - b'0'));
                } else {
                    if pid > 0 {
                        libc::kill(pid, libc::SIGKILL);
                        any = true;
                    }
                    pid = 0;
                }
            }
        }
        if pid > 0 {
            libc::kill(pid, libc::SIGKILL);
            any = true;
        }
        libc::close(fd);
    }
    any
}
