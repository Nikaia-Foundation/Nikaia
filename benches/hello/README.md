# What Hello World costs to start (#860)

`hello.nika` prints one line and computes nothing, so every instruction
counted is start-up. `bench.nika` counts it in the four ways a Nikaia
program is started, against `hello.py`, with C's `puts` as the floor:

1. the binary `nikaia build` wrote, with the project's one profile left as
   it is (`dev`) and set for release in `[build.<target>]` (`opt-level =
   3`, fat LTO, one codegen unit, no debug information);
2. `nikaia run` of a file run before and unchanged since (warm); 2' the
   same with its run record removed, the way every warm run went before the
   record (#2421, #2530): the file read again, its program found built;
3. `nikaia run` after an edit: (a) a comment's text changed, which leaves the
   generated Rust as it was; (b) a branch added that is never taken, which
   changes the Rust, so `rustc` compiles it again;
4. `nikaia run` of a file never run (its entry in the user cache removed).

```
nikaia run benches/hello/bench.nika
NIKAIA=target/fast/nikaia nikaia run benches/hello/bench.nika
```

## How it counts

Callgrind, `--branch-sim=yes --trace-children=yes`, every process of the run
summed: `nikaia run` starts `rustc`, and `rustc` the linker. Three things
the bench takes care of:

* `nikaia run` `exec`s the program it built. Callgrind writes no dump of an
  image that `exec`s, so `--dump-before=execve --dump-before=execvp` writes
  one first.
* A child's first dump, taken in the fork before its `exec`, carries a copy
  of its parent's counters up to the fork. It is left out where its command
  is another process's of the same run.
* Valgrind's own lines go to a log file per process (`--log-file`), not to
  standard error. There, the linker's would reach `rustc`, which turns them
  into a warning, and `nikaia run` places that warning in the program again:
  a few hundred million instructions that are no part of a real run.

Every row must print what Python prints, byte for byte, and nothing on
standard error, or the bench stops. In 3a only the comment's text changes:
a new line would move the source places the generated Rust holds. 3b adds
`if N < 0 { println("never") }` and not a `let`, since `rustc` warns of an
unused `let` and the warning costs a lowering of its own. Not counted: the
`std` bundle the program links against, built once per machine and compiler.

## Figures

2026-11-21, compiler `target/fast` (profile `fast`) built from `develop` at
fbdea3925 with the bundle linked `-Bsymbolic-functions` (#2532), rustc
1.97.0, Python 3.13.18, Intel Xeon @ 2.10GHz.

| scenario | instructions | mispredicted | / Python | instructions by program |
|---|---:|---:|---:|---|
| Python 3, `python3 hello.py` | 40,194,104 | 636,884 | 1.00 | python3.13 40,194,104 |
| C `puts`, `cc -O2` (the floor) | 239,480 | 5,547 | 0.01 | hello-c 239,480 |
| 1 built binary (`nikaia build`, `dev`) | 508,309 | 8,891 | 0.01 | hello 508,309 |
| 1 built binary (`nikaia build`, release: `opt-level = 3`, fat LTO) | 440,659 | 8,019 | 0.01 | hello 440,659 |
| 2 `nikaia run`, warm | 11,834,953 | 98,195 | 0.29 | hello 2,521,924, nikaia 9,313,029 |
| 2' `nikaia run`, warm, without its run record | 47,222,002 | 384,695 | 1.17 | hello 2,521,911, nikaia 44,700,091 |
| 3a `nikaia run` after an edit, Rust unchanged | 271,230,735 | 2,047,924 | 6.75 | hello 2,521,911, nikaia 268,708,824 |
| 3b `nikaia run` after an edit, Rust changed | 659,905,317 | 6,077,206 | 16.42 | hello 2,521,924, nikaia 271,057,700, rustc 328,500,405, cc 2,616,124, collect2 454,709, rust-lld 54,227,929, ld.lld 526,526 |
| 4 `nikaia run` of a new file | 712,068,010 | 6,542,879 | 17.72 | hello 2,521,924, nikaia 268,492,215, rustc 383,227,451, cc 2,617,824, collect2 454,776, rust-lld 54,227,320, ld.lld 526,500 |

## Pass or not

| scenario | against Python | verdict |
|---|---:|---|
| 1 built binary, `dev` and release | 0.01 | below; 1.8 to 2.1 times C's floor |
| 2 `nikaia run`, warm | 0.29 | below |
| 3a after an edit, Rust unchanged | 6.75 | above: the front end, below |
| 3b after an edit, Rust changed | 16.42 | above: the front end, `rustc` and the link; floor above Python while `rustc` compiles the program |
| 4 a new file | 17.72 | as 3b, `rustc` with no incremental state |

## Scenario 1: the built binary

The release binary's 0.44 M, from callgrind's inclusive counts:

| part | instructions | share |
|---|---:|---:|
| the dynamic loader binding `libc` (C pays it too: 0.24 M is its whole start) | 0.31 M | 72 % |
| Rust's `std::rt::lang_start`: the main thread's stack bounds, read from `/proc/self/maps` with `sscanf` (`pthread_getattr_np`) | 0.10 M | 22 % |
| the program's `main`: `std`'s budget for `main`'s block, decided before `main` runs (ADR-327 D14), from `/proc/self/fd`, `/proc/self/limits`, the cgroup's memory limit and `/proc/meminfo` | 0.02 M | 5 % |
| printing the line | under 0.01 M | |

What the language adds over C is the second and third rows: Rust's start
and ADR-327 D14's budget. A program that uses nothing still asks the
budget, since D14 decides it before `main` runs, not at the block's first
allocation; 5 % of the start, a decision of its own and no step here.
`dev` costs 0.07 M more than release.

## Where the warm run goes (scenario 2)

No `rustc` child runs: the version comes from the answer held in a stamp
(`comptime_run::rustc_version`, about 0.23 M), and the file is neither read
nor checked again: its run record matches (about 0.15 M).
Row 2' shows what the record saves: without it the same run reads the file
again and takes it through the front end before it finds the program in its
slot, nikaia 44.7 M instead of 9.3 M.

The program's 2.5 M: 91 % is the dynamic loader. `nikaia run` links it
`prefer-dynamic` against the debug `libnikaia_run_bundle.so` and
`libstd-*.so`. A Rust `dylib` exports every function, and the bundle's
calls of its own functions went through 25,691 `GLOB_DAT` slots the loader
filled by symbol lookup at every start: 40.9 M, more than Python's whole
start. The bundle is linked `-Wl,-Bsymbolic-functions` on Linux
(`comptime_bundle_bundle_of`), which binds those calls when it is linked:
470 `GLOB_DAT` slots are left, and the program starts in 2.5 M (its own
`main` about 0.22 M).

nikaia's 9.3 M, from callgrind's inclusive counts:

| part | instructions | share |
|---|---:|---:|
| `comptime_run::run_bundle`: the bundle's fingerprint walks `std`'s source tree (`read_dir`, `metadata` of each file, sorting the paths) | 6.24 M | 67 % |
| clap's parse of the command line | 1.31 M | 14 % |
| the dynamic loader before `main` | 0.64 M | 7 % |
| `run_directly`, the `exec` of the program | 0.42 M | 5 % |
| the held `rustc` version | 0.23 M | 2 % |
| the run record's lookup | 0.15 M | 2 % |

## Scenarios 3 and 4: an edit and a new file

**The front end, 269 M in each** (3a: the whole run but the program).
nikaia's inclusive counts in 3a:

| part | instructions | share |
|---|---:|---:|
| `bounds::proven_taken`, asked by the contracts of the emitter: 86 conditions, each parsed as a program (`prove::condition` to `parser::parse_to_ast`, whose `text_tiers::refine_with` is 100 M) | 116 M | 43 % |
| `modules::Program::of`, inferring the package (`contracts::infer_package_with` 32 M) | 52 M | 19 % |
| `project::check` | 43 M | 16 % |
| `contracts::std_library`, the base library | 13 M | 5 % |
| `comptime_run::run_bundle` | 12 M | 5 % |

The first row is a step of its own: a condition parsed as an expression,
not through the whole program's text tiers, or the proofs held across runs,
since they are the same for the same source. 3a can go below Python only
when the front end of a three-line file is under 38 M; it is 7 times that.

**`rustc` and the link, 386 M in 3b and 441 M in 4**: `rustc` compiles the
program at `opt-level=0` with its incremental state (328 M in 3b; 383 M in
4, where no state is there yet), and `rust-lld` links it against the bundle
(54 M). Their floor is a `rustc` run, which alone is 8 to 10 times Python's
start, so 3b and 4 stay above Python while a changed program is compiled by
`rustc`; what can come down is the front end above (40 % of 3b) and the
link. A new file at a new path is compiled again although the same text ran
elsewhere: the entry is keyed by the path (below), and the slot by a Rust
that holds the path; reusing it would need the places table out of the key.

## What the run cache keys on

`nikaia run FILE` keeps three things in the user cache (`NIKAIA_CACHE_DIR`,
else `$XDG_CACHE_HOME/nikaia`, else `~/.cache/nikaia`):

* **the entry**, `run/<stem>-<16 hex of the SHA-256 of FILE's canonical
  path>` (`project_drive_kept_root`): one per path, so the same text at a new
  path starts with no record and no slot (scenario 4);
* **the run record**, `ran/<16 hex of the SHA-256 of the path as given>`,
  keyed by the compiler's identity, the `rustc` version, the bundle's stamp,
  the settings, the allowlist and the environment variables starting with
  `NIKAIA_`, `RUST` or `CARGO` (`ran_key`), and listing each source file's
  length, modification time and digest. When all match, the program runs
  again without being read (scenario 2);
* **the slot**, `slots/<16 hex of its key>` (`run_fast::compile`), keyed by
  the digest of the generated Rust, the `rustc` version, the bundle's stamp,
  the crate's name and the path it is shown as. An edit whose Rust is the
  same reuses the built program and its messages (3a); any other compiles
  again (3b).
