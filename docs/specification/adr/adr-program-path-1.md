# ADR-program-path-1: `env::current_exe()` Is the Program That Was Started, `env::current_main()` the File It Stands In As; the Launcher Hands Both Over and No Program Sees How

**Status:** Accepted
**Date:** November 17, 2026 (the owner's decisions of 2026-11-17)
**Answers:** what a program learns about where it is when it asks: under
`nikaia run` and `nikaia test`, is "the running program" the toolchain, the
binary the toolchain built in its cache, or the `.nika` file the developer
named; and is that one question or two
**Supersedes / Related:**
[ADR-397](adr-397.md) (a value read from the environment is untrusted; a path carries no provenance);
[ADR-010](adr-010.md) D8 (no provenance travels a path);
[ADR-243](adr-243.md) D5 (`std` does not change the running program's environment; a child gets another one through `process::run`);
[ADR-218](adr-218.md) (`std` holds no `unsafe` code);
Part III 17.1 (`std::env`)

## 1. Question

Under `nikaia run app.nika`, what does a program get when it asks for its own
path, and is "the executable" the same question as "the program's file"?

`env::current_exe()` used to answer with the operating system's view: under
`nikaia run` and `nikaia test` that is the binary the toolchain built into its
cache (`target/nikaia/bin/…`), a path the developer never wrote, never sees and
cannot rely on, since another toolchain may run the program without building one
(an interpreter keeps no cache). A program that wants the files beside its
source (a fixture, a template, a configuration) has no way to find them, and a
program that wants to start the toolchain again (a test that runs a probe) has
to dig the toolchain out of `/proc`.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Rust | `std::env::current_exe()`: the binary; `cargo run` runs a binary in `target/`, so that path | a script-like program finds `target/debug/…`, not its source | from memory, not checked |
| Go | `os.Executable()`: the binary; under `go run` a temporary binary that is deleted afterwards | the path is useless under `go run`; Go has no "main file" question | from memory, not checked |
| Node.js | `process.execPath` is `node` itself; `require.main.filename` (`process.argv[1]`) is the script | two questions, two answers | from memory, not checked |
| Python | `sys.executable` is the interpreter; `__main__.__file__` (`sys.argv[0]`) is the script; a frozen program (PyInstaller) sets `sys.frozen` and both name the bundle | a frozen program answers both questions with itself | from memory, not checked |
| Deno / Bun | `Deno.execPath()` is `deno`; `Deno.mainModule` is the entry module; Bun has `Bun.main` | a compiled program (`deno compile`) answers with itself | from memory, not checked |
| Ruby | `RbConfig.ruby` is the interpreter; `$0` / `$PROGRAM_NAME` is the script | | from memory, not checked |
| Rust, C, Swift, Zig | a form evaluated at compile time names the source file: `file!()`, `__FILE__`, `#filePath`, `@src()` | a path as it was when compiled, not where the program now runs | from memory, not checked |

The languages that run a source file through a tool (Node, Python, Deno, Ruby)
all ask two questions and answer the first with the tool and the second with the
file; the compiled ones (Rust, Go) ask only the first and, under their `run`
command, answer it with a path nobody wants. Nikaia is both: a program runs from
its file under `nikaia run` and as a binary once built. What does not transfer:
nothing here needs a runtime the others have; the answers are paths.

## 3. Options, weighed

### Option A — keep the operating system's answer

```nika
let exe = env::current_exe()   // under `nikaia run`: …/target/nikaia/bin/app
```

### Option B — `current_exe()` names the `.nika` file under `nikaia run`

```nika
let source = env::current_exe()   // under `nikaia run`: …/app.nika; built: …/app
```

### Option C — two functions: the program that was started, and the file it stands in as

```nika
let exe = env::current_exe()     // under `nikaia run`: the toolchain; built: …/app
let main = env::current_main()   // under `nikaia run`: …/app.nika; built: …/app
```

### Option D — a compile-time form naming the source file

```nika
let here = file()   // the path of this source file as it was compiled
```

### Weighed

| | D1 the special case pays, not the common one | D2 the compiler works, not the developer | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | every program run from its file pays: the cache path is useless to it | the developer digs the toolchain or the file out of `/proc` | none | the answer depends on whether this toolchain keeps a cache | nothing to undo; programs keep their workarounds |
| **B** | a program that wants the toolchain cannot get it | | none | one name, two meanings: an executable that is a text file | programs that start "themselves" start a `.nika` file and fail |
| **C** | none: each question has its answer | the launcher hands the values over | one environment read on the call | the same as Node, Python, Deno; a built program answers both with itself, as a frozen Python program does | a second function to drop; cheap |
| **D** | answers a different question (the file a function is written in, not the file that was started) | | none | a path from compile time, wrong once the program moves | |

## 4. Decision

Option C. A loses on D1 and D2, B on D4: an executable that is a source file is
two meanings under one name. D answers another question and is left for later
(section 6).

### D1 — `env::current_exe() -> fs::Path?` is the program that was started.

Under `nikaia run` and `nikaia test` that is the toolchain, `nikaia` itself; a
built program is itself. It is null where the platform cannot say. The cache
binary is never the answer: whether a toolchain builds one is its own business.

### D2 — `env::current_main() -> fs::Path?` is the file the program stands in as.

| Launched as | `env::current_main()` | `env::current_exe()` |
| :--- | :--- | :--- |
| `nikaia run app.nika` | `app.nika`, absolute | `nikaia` |
| `nikaia run` in a project | the project's entry point (`src/main.nika`), absolute | `nikaia` |
| `nikaia test`, a test program | the test's `.nika` file | `nikaia` |
| `nikaia test`, a test block or an output test of a package | the package's `src/main.nika` | `nikaia` |
| a built program | the program itself | the program itself |
| the platform cannot say | null | null |

A built program answers both questions with itself, as a frozen Python or a
compiled Deno program does: it is the file it stands in as.

### D3 — The launcher hands both over, and no program sees how.

The toolchain hands the two values to the program in the environment
variables `NIKAIA_EXE` and `NIKAIA_MAIN`. They are the launcher's, not the
program's: `env::var` calls them null, `env::vars` leaves them out, and a child
started with `process::run` does not inherit them, so a `nikaia run` started from
a program names its own file and not its parent's. A `nikaia run` that the
toolchain itself starts for a test keeps the test's file. `std` changes nothing
in its environment to do this (ADR-243 D5, ADR-218: no `unsafe`); it reads the
two on the call. Where neither is set, both functions give the operating
system's path of the running binary.

## 5. Consequences

* A program run from its file finds the files beside it from
  `env::current_main()?.parent()`, under `nikaia run`, `nikaia test` and as a
  built program alike, provided the files are shipped beside the binary.
* A program that starts the toolchain again names it with
  `env::current_exe()` under `nikaia run` and `nikaia test`; a test no longer
  reads `/proc`.
* The names `NIKAIA_EXE` and `NIKAIA_MAIN` are the toolchain's: a program that
  sets them for a child it starts with `process::run` sets nothing, as they are
  removed.
* Like every path, neither result carries provenance (ADR-010 D8, ADR-397).

## 6. What this does not decide

* A form evaluated at compile time naming the source file a function is written
  in (Option D, as Rust's `file!()`): a different question, asked when a
  program needs it.
* A path for the cache binary: none is offered, so another toolchain may run a
  program without building one.
