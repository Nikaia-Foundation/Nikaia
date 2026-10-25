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
