# ADR-platform-layer-v1: What Nikaia cannot do itself is a platform primitive, declared in Nikaia, implemented per backend, and holding no logic

**Status:** Accepted
**Date:** November 21, 2026 (owner's decision, option 4 of the round on
[#2481](https://github.com/Nikaia-Foundation/Nikaia/issues/2481),
[#2482](https://github.com/Nikaia-Foundation/Nikaia/issues/2482) and
[#2483](https://github.com/Nikaia-Foundation/Nikaia/issues/2483))
**Answers:** where the few lines go that a feature written in Nikaia needs
from the platform and cannot say itself (a copy that keeps a file's mode, a
value the compiler's Rust host knows), now that new features are written in
Nikaia only
**Related:**
[ADR-303](adr-303.md) - the runtime is Nikaia's own; this record names the thin part under it that is not;
[ADR-rust-boundary-1](adr-rust-boundary-1.md) D24-D28 - `std` calls its own Rust by a plain call, described in `std.contracts`; a primitive is such a call;
[ADR-108](adr-108.md) D2 - a path names its root at the call;
[ADR-raw-sinks-1](adr-raw-sinks-1.md) D16, D17 - proof in place of `trusted:`, and the compiler's own build paths, whose items 6 and 7 need the first primitives;
[ADR-258](adr-258.md) - the four weights

## 1. Question

A feature of the compiler or of `std` is written in Nikaia (owner,
2026-10-25 and 2026-10-31). Some of what it needs is not logic but an
operation of the platform: copying a file so that it stays executable, or a
directory that the compiler's Rust host chose and Nikaia cannot learn. Today
such a step stops as blocked (D17 items 6 and 7 stopped on exactly this), or
it is written in Rust where nobody sees what it decides. **Where does such an
operation live, what may it contain, and how does the prover see it?**

Without an answer every such step waits until the whole Rust around it has
moved to Nikaia, or Rust grows again by habit, with decisions in it that the
prover cannot check.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Go | `syscall` and `internal/syscall/*`: one thin file per operating system, many of them generated (`zsyscall_linux_amd64.go`); the logic of `os` above is Go and the same everywhere | a port is a new set of thin files; nothing above changes | from memory, not checked |
| Rust | `std::sys` (the platform abstraction layer): per-target modules under one interface that `std::fs`, `std::process` call | the interface is internal and moves; logic leaks into some targets' modules | from memory, not checked |
| Java | a `native` method is declared in Java, with its signature, and implemented per platform in C | the declaration is all the Java side sees; what the C does is unchecked | from memory, not checked |
| Zig | `std.posix` / `std.os.*` per target, called by the portable `std` | as Go | from memory, not checked |

What transfers: the declaration lives on the high-level side and is what its
tools check against; each platform supplies the same list; the list stays
small because logic above it is written once. What does not: Java and Rust do
not prove anything about the native side either, and neither does this record.
It keeps that side too small to need proving.

## 3. Options, weighed

### Option A: Rust where it is needed (as before)

The step writes what it needs in the Rust host, including the choice around
it (which directory, what to do on failure).

### Option B: wait for the whole move

Nothing is added until `make_cargo`, the outer build and the host are Nikaia
([#125](https://github.com/Nikaia-Foundation/Nikaia/issues/125)). D17 items
6 and 7 wait for that.

### Option C: a platform layer

The operation is declared in `std.contracts` with its contract, implemented
in the backend in a few lines that only call the platform, and everything
that decides is Nikaia calling it.

```nika
use std::fs
fn keep(cached: fs::Path, out: fs::Path, store: fs::Path, target: fs::Path) -> bool {
    fs::copy_between(cached, fs::Root::Dir(store), out, fs::Root::Dir(target)) catch { return false }
    return true
}
```

### Option D: an allowance or `trusted:` at each place

The Nikaia side reads and writes as it can and states an `[allow]` line or
`trusted:` where it cannot prove the path. The owner refused this:
an allowance is not boilerplate but a hole, and the aim is proved safety.

### Weighed

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | every feature near the host pays: its decisions sit where no check reaches | no: the prover sees a Rust call it cannot read | none | breaks "written in Nikaia only" | decisions in Rust are found one by one, late |
| **B** | the feature waits, and so does the safety it brings | yes, once done | none | yes | D17 stays half built for months |
| **C** | the primitive costs a contract entry and a few lines per backend | yes: every call is checked against the contract; the logic is Nikaia | one call, as today's `fs::copy` | it is how `std` already reaches the platform (`fs.rs`, `io.rs` under `std.contracts`) | a primitive that grew logic is moved up; the list shows where to look |
| **D** | every reader pays: each allowance is a line to trust | no: the developer states what the compiler should prove | none | contradicts ADR-raw-sinks-1 D16 | holes in the build's own safety |

C keeps D2 and D4 and costs nothing at run time. It does not replace B: the
move of the outer build to Nikaia stays the goal, and C is what that move
leaves behind.

## 4. Decision

### D1: A primitive is declared in Nikaia.

Its entry in
`crates/nikaia-std/std.contracts` (signature, `touches`, `throws`, `panics`,
`sync`) is its whole meaning for the language. The prover and the checker
read only the entry and check every call against it: the roots a call touches,
what it throws. Nothing about a primitive is known from its implementation.

### D2: A primitive holds no decision.

It does one operation the platform
offers (a system call, a library function, handing a value through) and maps
the platform's error to the declared one. Choosing a path, a fallback, a loop
over cases, parsing, caching, retrying: anything a test would need two
branches for is Nikaia above the primitive. A reviewer applies one test: the
implementation is one platform call plus the error mapping, and the same
lines would be written in any backend.

### D3: Each backend implements the whole list.

A backend supplies every
primitive of `std.contracts` under the same name and contract. Today the one
backend is Rust (`crates/nikaia-std/src/*.rs`). Another backend replaces this
layer, not the Nikaia above it. A primitive a backend lacks is an error
when that backend is built, never a silent fallback.

### D4: A name keeps its root.

A primitive that reaches the file system takes
each name together with the root it is resolved under (ADR-108 D2) and
resolves it the way `fs::copy` does; a name that leaves a `Dir` root throws
`Outside`. An operation on two names takes two roots.

### D5: The host hands values through.

A value that the compiler's Rust host
holds and Nikaia needs (an environment variable it sets for Cargo, a directory
on `ProjectDriveHost`) is a primitive under D2: it hands the value through
and decides nothing new. The Rust that chooses the value stays where it is,
and is listed for the move (#125).

### D6: Every primitive is listed.

The commit that adds one names this
record. The report lists it for #557 and #125. A primitive is the only form
in which new Rust enters the project outside the exception of #1214
(`CLAUDE.md`, *Written in Nikaia only*).

### D7: The first primitives

are the ones D17 of ADR-raw-sinks-1 needs:

1. `fs::copy_between(from, from_root, to, to_root) -> u64`, `touches =
   ["file(from) read", "file(to) write"]`, `throws = ["io::IoError"]`:
   copies a file from one root into another and keeps its permission bits
   (D17 item 6, #2481);
2. `NIKAIA_TARGET_DIR`, set by the outer build to the `--target-dir` it hands
   Cargo (D17 item 6, #2482);
3. accessors on `ProjectDriveHost` for the project root, the sysroot's `std`
   sources and the target directory (D17 item 7, #2483).

## 5. What it costs

* Each backend repeats the list. It stays short because D2 keeps every
  primitive to one platform call.
* The Rust host still chooses the target directory and runs Cargo until
  `make_cargo` and the outer build have moved (#125). This record does not
  move them. It keeps new code from piling onto them.
* The primitive itself is not proved. Its contract is trusted the way every
  `std.contracts` entry is. D2 keeps what is trusted to one call a reviewer
  can read.
