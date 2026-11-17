# child-usage

A child process waited for with `wait4`, so that what it used is that one
child's: the processor time it spent in user and in kernel mode, and the most
memory it held resident. `getrusage(RUSAGE_CHILDREN)` answers a different
question - a sum and a maximum over every child waited for so far - which is
why the standard library reports nothing of this kind and why this crate exists
([ADR-368](../../../docs/specification/adr/adr-368.md) D3). Its one dependency
is `libc`, for the declaration of `wait4` and the layout of `struct rusage`.

```rust,no_run
use std::process::{Command, Stdio};

let child = Command::new("true").stdin(Stdio::null()).spawn()?;
let waited = child_usage::wait(child)?;
println!("{:?} {:?} {:?}", waited.status, waited.user_time, waited.max_rss_kib);
# Ok::<(), std::io::Error>(())
```

**Read the pipes first.** `wait` takes the `Child` by value, so it cannot be
waited for twice, and it closes whatever pipe is still in it before it waits:
a child blocked writing to a full pipe nobody reads would otherwise never end.
A caller that wants the output takes `stdout` and `stderr` out of the child and
reads them to their end, then calls `wait`.

**Where the platform reports it.** On Unix `max_rss_kib` is `ru_maxrss`, which
Linux and the BSDs give in KiB and macOS in bytes (divided here). Elsewhere the
child is waited for with the standard library, both times are zero and
`max_rss_kib` is `None`: no other platform's figures are reachable without an
`unsafe` this crate does not hold yet.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `wait` (Unix) | `core::mem::zeroed::<libc::rusage>()` | `struct rusage` is integers and `timeval`s of integers; all-zero is a valid value of each |
| `wait` (Unix) | `libc::wait4(pid, &mut status, 0, &mut usage)` | both pointers are to locals that live across the call and have the types `wait4` writes; `pid` is the id of a child this process spawned and has not waited for, since the `Child` is owned here and the standard library waits only through it |
| `kill_group` (Unix) | `libc::kill(-pid, libc::SIGKILL)` | two integers, no pointer; a negative pid names the group the child leads, and a stale pid at worst fails with `ESRCH` |
| `cap::Cap::apply` (Unix) | `CommandExt::pre_exec` running `open`, `write`, `close`, `setrlimit` | the hook runs between `fork` and `exec`, where only async-signal-safe calls are allowed: it makes only those four, on a path built before the fork and a local `rlimit`, and allocates nothing |
| `cap::wait_unreaped` (Unix) | `libc::waitid(P_PID, pid, &mut info, WEXITED \| WNOWAIT)` | `info` is a zeroed local `siginfo_t` (plain data); `pid` is a child the `Child` owns and nobody has reaped; `WNOWAIT` leaves it to be reaped, so its id stays taken while its group is ended |
| `cap::guard_group` (Unix) | `pipe2`, `fork`, and in the forked guard `signal`, `setpgid`, `dup2`, `close_range`, `read`, `open`, `kill`, `usleep`, `rmdir`, `_exit`; `waitpid` of the guard | the guard is forked from a process with threads, so it calls only async-signal-safe functions, on integers, locals and a path built before the fork, allocates nothing and leaves only by `_exit`; both pipe ends are close-on-exec, and the guard closes every other descriptor at once; `waitpid` reaps the guard this process forked |

The cap (#1919): `cap::Cap` puts a child into a `pids` cgroup of its own
with `pids.max` set, or for a starter that is not root lowers its
`RLIMIT_NPROC`; `cap::guard_group` forks a guard that ends the child's group
when the starter ends, killed included.

## How it is checked

Its own workspace, so its lock file and its checks are its own:

```sh
cargo test && cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo +nightly miri test --all-features                       # Stacked Borrows
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test --all-features
```

`scripts/check-unsafe-crates.sh` runs these for every crate under
`crates/unsafe/`, and CI runs that script.

Miri runs none of the tests: each starts a process, which Miri cannot.
