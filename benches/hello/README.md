# What Hello World costs to start (#860)

`hello.nika` prints one line and computes nothing, so every instruction
counted is start-up. `bench.nika` counts it in the four ways a Nikaia
program is started, against `hello.py`:

1. the binary `nikaia build` wrote (`dev` profile);
2. `nikaia run` of a file run before and unchanged since (warm);
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
c5c2c38a5, rustc 1.97.0, Python 3.13.18, Intel Xeon @ 2.10GHz.

| scenario | instructions | mispredicted | / Python | instructions by program |
|---|---:|---:|---:|---|
| Python 3, `python3 hello.py` | 40,366,012 | 643,338 | 1.00 | python3.13 40,366,012 |
| 1 built binary (`nikaia build`, `dev`) | 508,369 | 8,856 | 0.01 | hello 508,369 |
| 2 `nikaia run`, warm | 50,246,733 | 216,594 | 1.24 | hello 40,929,633, nikaia 9,317,100 |
| 3a `nikaia run` after an edit, Rust unchanged | 309,512,807 | 2,225,395 | 7.67 | hello 40,929,635, nikaia 268,583,172 |
| 3b `nikaia run` after an edit, Rust changed | 698,199,040 | 6,131,265 | 17.30 | hello 40,929,646, nikaia 270,881,881, rustc 328,570,469, cc 2,613,403, collect2 457,434, rust-lld 54,219,951, ld.lld 526,256 |
| 4 `nikaia run` of a new file | 638,300,146 | 5,898,342 | 15.81 | hello 40,929,633, nikaia 156,686,223, rustc 382,870,068, cc 2,613,749, collect2 457,633, rust-lld 54,216,454, ld.lld 526,386 |

## Where the warm run goes (scenario 2)

No `rustc` child runs: the version comes from the answer held in a stamp
(`comptime_run::rustc_version`, about 0.23 M), and the file is neither read
nor checked again: its run record matches (about 0.15 M).

nikaia's 9.3 M, from callgrind's inclusive counts:

| part | instructions | share |
|---|---:|---:|
| `comptime_run::run_bundle`: the bundle's fingerprint walks `std`'s source tree (`read_dir`, `metadata` of each file, sorting the paths) | 6.24 M | 67 % |
| clap's parse of the command line | 1.31 M | 14 % |
| the dynamic loader before `main` | 0.64 M | 7 % |
| `run_directly`, the `exec` of the program | 0.42 M | 5 % |
| the held `rustc` version | 0.23 M | 2 % |
| the run record's lookup | 0.15 M | 2 % |

The program's 40.9 M: 98.9 % is the dynamic loader (`_dl_relocate_object`,
`_dl_lookup_symbol_x`). `nikaia run` links it `prefer-dynamic` against the
debug `libnikaia_run_bundle.so` and `libstd-*.so`, and binding their symbols
costs as much as all of Python's start. Its own `main` is about 0.22 M. The
binary `nikaia build` writes, linked statically, starts in 0.5 M. So the
dynamic link, then the fingerprint walk, are what make the warm run 1.24
times Python.

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
