# Getting Started

From nothing to a running Nikaia program, and what to try after that.

Nikaia is **pre-alpha**. Expect small programs to work and larger ones to hit a missing
piece. The [roadmap](../docs/project_status_and_roadmap.md#where-the-project-actually-stands)
lists which.

## What you need

Linux on x86_64 or on 64-bit ARM (aarch64). CI runs the whole suite on both. macOS and
Windows are untested.

Nikaia compiles to Rust and lets the Rust compiler build the binary, so the one thing to
install is a stable Rust toolchain, 1.88 or newer. No nightly toolchain and no extra
system libraries are needed.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

`rustup` reads the channel from `rust-toolchain.toml` and installs it on first use.

## 1. Build the compiler

```sh
git clone https://github.com/Nikaia-Language/Nikaia.git
cd Nikaia
cargo build --release -p nikaia        # about a minute; the binary is target/release/nikaia
export PATH="$PWD/target/release:$PATH"
```

The compiler finds its standard library in the checkout it was built from, so leave
the checkout where it is. If you move it, point `NIKAIA_SYSROOT` at its `crates/`
directory.

## 2. Create a project

A project is a `nikaia.toml` and a `src/main.nika`. There is no `nikaia new` yet, so
create the two files by hand:

```text
hello/
├── nikaia.toml
└── src/
    └── main.nika
```

```toml
# nikaia.toml
[package]
name = "hello"
version = "0.1.0"

[build]
user-parallelism = "no"    # the default; "yes" for the multi-threaded runtime
```

```nika
// src/main.nika
fn main() {
    println("Hello, Nikaia!")
}
```

## 3. Run it

```sh
cd hello
nikaia run              # or: nikaia build
```

The first build compiles the standard library and its dependencies (about 15 s) and
caches them in `~/.cache/nikaia`. Later builds reuse that. Arguments for your program go
after `--`: `nikaia run -- a b c`.

The build writes two files beside the manifest:

* `nikaia.contracts` records what the compiler inferred about your functions: what they
  borrow, whether they can fail, whether they can pause. Commit it: a change in it is a
  change in your program's behaviour that you can review. What the compiler proves about
  an `assert` is shown in [Contracts](contracts.md).
* `nikaia.lock` pins your dependencies.

## 4. When the compiler says no

Most mistakes are reported in Nikaia's own words, with a code and a line that says what
to write instead. For example:

```text
error[NK1139]: You're changing `x`, but it wasn't declared as mutable.
  --> src/main.nika:3:5
   |
 2 |     let x = 5
   |         - declared here without `mut`
 3 |     x = 6
   |     ^ changed here
   |
   = help: Add `mut` where it's declared: `let mut x`.
```

Some errors and warnings still come through from the Rust compiler in Rust's vocabulary.
They point at your `.nika` line, but the wording is Rust's. Reporting one of those is
useful: it is a message Nikaia should have written itself.

---

## The two build settings

Two lines in `nikaia.toml` decide how a program is built. They do not change what it
means, and you do not rewrite code to move between them.

### `target`: which machine

`x86_64-linux` by default, `aarch64-linux` too. The machine decides what the standard
library can offer and what a crash does: an unwind where the machine unwinds, a trap
where it traps. `wasm32-unknown` is accepted as a name, but a program for it is refused
for now, and the compiler says what is missing.

### `user_parallelism`: how much of *your* code runs at once

* **`no`** (the default): your code runs on one thread, over an event loop. Two pieces
  of your code are never running at the same moment, so there are no data races. This is
  the setting for web services, CLI tools and edge workers, where you might otherwise use
  Node.js or Go.
* **`yes`**: a multi-threaded work-stealing runtime spreads the work over the cores, and
  thread safety is checked at compile time. This is the setting for heavy computation,
  where you might otherwise use Rust or C++.

At `no`, only your own code is limited to one thread. Waiting for I/O happens on the
runtime's own threads, and the standard library may still use every core: a file's text,
for example, is checked in chunks across all of them
([ADR-016](../docs/specification/adr/adr-016.md)).

The setting allows parallelism; it does not set a thread count. How many threads serve
`yes` is the runtime's choice, because it depends on the machine.

A library built at `no` is still checked against the rules parallel code needs, so it
also works when a program built at `yes` uses it.

**Design decision:** [ADR-037, the build switches](../docs/specification/adr/adr-037.md)

---

## Programs to try

Real programs live in [`examples/`](../examples/). Copy one into `src/main.nika` and run it:

* [`calc.nika`](../examples/calc/src/main.nika): a four-function calculator built from a grammar.
  `nikaia run -- "2 + 3 * 4"`
* [`tally.nika`](../examples/tally/src/main.nika): counts the lines piped into it, in constant
  memory.
* [`access-log.nika`](../examples/access-log/src/main.nika): a web access log summarised.
* [`json.nika`](../examples/json/src/main.nika), [`config.nika`](../examples/config/src/main.nika): a JSON
  document and an INI file with comments, parsed.
* [`1brc.nika`](../examples/1brc.nika): the One Billion Row Challenge.
* [`hello-http/`](../examples/hello-http/): a project with a dependency, serving HTTP.

There are also two Computer Language Benchmarks Game programs, an HTML table that cannot
be made to leak markup, a program split across three files, and a C library called
through `extern "C"`. All of them except `fortunes` compile, run, and are checked by
`cargo test`. The single-file ones are run at both `user_parallelism` settings and must
print the same output. `fortunes` (the TechEmpower benchmark) is still written ahead of
the compiler, because it needs a database.

[`examples/README.md`](../examples/README.md) lists which gaps in the language each
example exposed, and which are still open.

## Where next

* [A tour of the syntax](syntax.md): the language on one page, for readers coming from
  other languages.
* [Contracts](contracts.md): what the compiler proves about an `assert`.
* [Specification, Part I](../docs/specification/10-nikaia-light.md): the full rules.
* [Specification, Part II](../docs/specification/20-nikaia-advance.md): concurrency,
  parallelism and grammars.
* Working on the compiler itself: [toolchain architecture](../docs/toolchain_architecture.md),
  and `cargo test -p nikaia` (not the whole workspace, see the
  [roadmap](../docs/project_status_and_roadmap.md#the-walls-you-will-hit)).
