//! **A child process waited for, and what it used**: the processor time it
//! spent in user and in kernel mode, and the most memory it held resident -
//! that one child's, never a maximum over every child waited for so far
//! ([ADR-368](../../../docs/specification/adr/adr-368.md) D3).
//!
//! On Unix the wait is `wait4` on the child's id, which hands back the child's
//! own `struct rusage` with its exit status. Elsewhere it is the standard
//! library's wait, and the figures are what [`Waited`] says they are there.
//!
//! Every `unsafe` in this crate, and the argument for it, is listed in
//! `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::io;
use std::process::{Child, ExitStatus};
use std::time::Duration;

/// How a child ended, and what it used on the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waited {
    /// How it ended.
    pub status: ExitStatus,
    /// The processor time it, and every child of its own it waited for, spent
    /// in user mode. Zero where the platform is not Unix.
    pub user_time: Duration,
    /// The same in kernel mode. Zero where the platform is not Unix.
    pub system_time: Duration,
    /// The most memory it held resident, in KiB, as the operating system
    /// reports it for that child; `None` where the platform is not Unix.
    pub max_rss_kib: Option<i64>,
}

/// Wait for `child` to end, and hand back how it ended and what it used.
///
/// **Read the pipes first.** Whatever pipe is still in `child` is closed
/// before the wait, so a child writing to one nobody reads ends (on a broken
/// pipe) rather than blocking for ever; a caller that wants the output takes
/// `stdout` and `stderr` out of the child and reads them to their end before
/// it calls this. Taking the child by value is what makes the wait the only
/// one: the standard library's `wait` cannot be called after it.
pub fn wait(mut child: Child) -> io::Result<Waited> {
    drop(child.stdin.take());
    drop(child.stdout.take());
    drop(child.stderr.take());
    waited(child)
}

#[cfg(unix)]
fn waited(child: Child) -> io::Result<Waited> {
    use std::os::unix::process::ExitStatusExt;

    let pid = libc::pid_t::try_from(child.id())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a process id out of range"))?;
    let mut status: libc::c_int = 0;
    // SAFETY: `struct rusage` is integers and `timeval`s of integers, and
    // all-zero is a valid value of each (README, first row).
    let mut usage: libc::rusage = unsafe { core::mem::zeroed() };
    loop {
        // SAFETY: both pointers are to locals that outlive the call and have
        // the types `wait4` writes; `pid` is a child this process spawned and
        // nobody has waited for, because the `Child` is owned here (README,
        // second row).
        let got = unsafe { libc::wait4(pid, &mut status, 0, &mut usage) };
        if got == pid {
            break;
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
    Ok(Waited {
        status: ExitStatus::from_raw(status),
        user_time: span(usage.ru_utime),
        system_time: span(usage.ru_stime),
        // `c_long` is an `i64` on 64-bit targets only.
        #[allow(clippy::useless_conversion)]
        max_rss_kib: Some(kib(i64::from(usage.ru_maxrss))),
    })
}

/// A `timeval` as a span; a negative part, which the kernel never reports, is
/// none.
#[cfg(unix)]
fn span(time: libc::timeval) -> Duration {
    let seconds = u64::try_from(time.tv_sec).unwrap_or(0);
    let micros = u64::try_from(time.tv_usec).unwrap_or(0);
    Duration::from_secs(seconds) + Duration::from_micros(micros)
}

/// `ru_maxrss` in KiB: macOS and iOS report bytes, Linux and the BSDs KiB.
#[cfg(unix)]
fn kib(reported: i64) -> i64 {
    if cfg!(target_vendor = "apple") {
        reported / 1024
    } else {
        reported
    }
}

#[cfg(not(unix))]
fn waited(mut child: Child) -> io::Result<Waited> {
    Ok(Waited {
        status: child.wait()?,
        user_time: Duration::ZERO,
        system_time: Duration::ZERO,
        max_rss_kib: None,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::Read;
    use std::process::{Command, Stdio};

    fn shell(script: &str) -> Child {
        Command::new("sh")
            .args(["-c", script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("sh starts")
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn the_exit_code_comes_back() {
        let waited = wait(shell("exit 3")).expect("waited");
        assert_eq!(waited.status.code(), Some(3));
        assert!(!waited.status.success());
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn the_pipes_read_first_then_the_wait() {
        let mut child = shell("echo hello");
        let mut out = String::new();
        child
            .stdout
            .take()
            .expect("piped")
            .read_to_string(&mut out)
            .expect("read");
        let waited = wait(child).expect("waited");
        assert_eq!(out, "hello\n");
        assert!(waited.status.success());
    }

    /// A child that fills a pipe nobody reads ends, because the pipe is closed
    /// before the wait.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn a_pipe_left_unread_does_not_hang_the_wait() {
        let waited = wait(shell("yes | head -c 10000000")).expect("waited");
        assert!(waited.status.code().is_some() || waited.status.signal().is_some());
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn a_busy_child_used_processor_time() {
        let waited =
            wait(shell("i=0; while [ $i -lt 300000 ]; do i=$((i+1)); done")).expect("waited");
        assert!(waited.status.success());
        assert!(
            waited.user_time + waited.system_time > Duration::ZERO,
            "{waited:?}"
        );
    }

    /// **That one child's**, not the largest so far: a small child waited for
    /// after a large one reports its own memory.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn the_memory_is_that_one_childs() {
        let large = wait(shell(
            "x=$(head -c 40000000 /dev/zero | tr '\\0' a); echo ${#x}",
        ))
        .expect("waited");
        let small = wait(shell("exit 0")).expect("waited");
        let (large, small) = (
            large.max_rss_kib.expect("reported"),
            small.max_rss_kib.expect("reported"),
        );
        assert!(small > 0, "{small}");
        assert!(small < large, "{small} KiB after {large} KiB");
    }

    use std::os::unix::process::ExitStatusExt;
}
