# What Hello World costs to start (#860)

`hello.nika` prints one line and computes nothing, so every instruction
counted is start-up. `bench.nika` counts it in the four ways a Nikaia
program is started, against `hello.py`:

1. the binary `nikaia build` wrote (`dev` profile);
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
12d353d1c (`NIKAIA=target/bins/12d353d1ca57/nikaia`), rustc 1.97.0, Python 3.13.18, Intel Xeon @ 2.10GHz.

| scenario | instructions | mispredicted | / Python | instructions by program |
|---|---:|---:|---:|---|
| Python 3, `python3 hello.py` | 40,278,939 | 640,489 | 1.00 | python3.13 40,278,939 |
| 1 built binary (`nikaia build`, `dev`) | 508,974 | 8,771 | 0.01 | hello 508,974 |
| 2 `nikaia run`, warm | 50,258,454 | 215,456 | 1.25 | hello 40,930,250, nikaia 9,328,204 |
| 2' `nikaia run`, warm, without its run record | 85,660,651 | 503,816 | 2.13 | hello 40,930,220, nikaia 44,730,431 |
| 3a `nikaia run` after an edit, Rust unchanged | 309,688,549 | 2,247,424 | 7.69 | hello 40,930,250, nikaia 268,758,299 |
| 3b `nikaia run` after an edit, Rust changed | 698,883,654 | 6,183,708 | 17.35 | hello 40,930,263, nikaia 270,674,999, rustc 329,468,276, cc 2,615,476, collect2 457,433, rust-lld 54,210,081, ld.lld 527,126 |
| 4 `nikaia run` of a new file | 750,590,576 | 6,748,796 | 18.63 | hello 40,930,250, nikaia 268,757,067, rustc 383,084,125, cc 2,616,024, collect2 457,390, rust-lld 54,218,571, ld.lld 527,149 |

## Where the warm run goes (scenario 2)

No `rustc` child runs: the version comes from the answer held in a stamp
(`comptime_run::rustc_version`, about 0.23 M), and the file is neither read
nor checked again: its run record matches (about 0.15 M).
Row 2' shows what the record saves: without it the same run reads the file
again and takes it through the front end before it finds the program in its
slot, nikaia 44.7 M instead of 9.3 M (35.4 M, 79 % of nikaia's share, the
whole run 1.70 times fewer instructions).

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
