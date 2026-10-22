<div align="center">
  <img src="nikaia-logo.jpg" alt="Nikaia Logo" width="300" height="164" />
  <h1>N I K A I A</h1>
  <p><strong>Good wins.</strong></p>
  <p>
    Write it like a script. Catch what can be proven at compile time.
    Fast as lightning.
  </p>


  <p>
    <a href="guide/getting-started.md">Getting started</a> •
    <a href="guide/syntax.md">Syntax tour</a> •
    <a href="guide/contracts.md">Contracts</a> •
    <a href="guide/command-line.md">Command line</a> •
    <a href="manifesto.md">Manifesto</a> •
    <a href="docs/project_status_and_roadmap.md">Status &amp; roadmap</a> •
    <a href="docs/specification">Specification</a> •
    <a href="https://nikaia-lang.org/">Documentation site</a> •
    <a href="https://gemini.google.com/gem/1T8viw7ZHA0TwDZDhr6h1mgRBVnw3aTNP?usp=sharing">Gemini explains Nikaia</a>
  </p>

  <img src="assets/badges/version.svg" alt="Version" />
  <img src="assets/badges/status.svg" alt="Status" />
  <img src="assets/badges/self-hosted.svg" alt="Self-hosted share of the compiler" />
  <img src="assets/badges/license.svg" alt="License" />
  <a href="https://nikaia-lang.org/"><img src="assets/badges/docs.svg" alt="Documentation site" /></a>
  <a href="https://github.com/Nikaia-Language/Nikaia"><img src="assets/badges/github.svg" alt="GitHub repository" /></a>
</div>

---

## The idea

Python or JavaScript are quick to write, but the programs are slower than they could be, and
some mistakes only show up in production. C++ or Rust produce fast programs, but a good part
of each program tells the compiler how memory, waiting and sharing should work.

Nikaia's bet is that a compiler which sees the whole program can work much of that out
itself: which data needs protecting, where a function waits, when memory can be freed, and
how to spread work over several cores. You write what should happen, and the compiler picks
how.

* **An `assert` is checked by the compiler**, as a contract every caller must meet.
* **No `async` or `await`.** A call that waits pauses its function; the program goes on.
* **Races, unchecked missing values and unhandled errors are refused** at compile time.
* **No garbage collector and no `free`.** Memory is released at a point you can see.
* **One core or many is one line** in `nikaia.toml`; the code stays the same.
* **Formats are part of the language**: a log or a protocol is described as a grammar.

Nikaia checks your program, and Rust's compiler checks memory safety again. The name comes from my
daughter, **Nika**: in Persian, *nik* means *good*, so the motto is *good wins*. The longer
story is in the [Manifesto](manifesto.md).

---

## An assert is a contract

`percent` asserts that `whole` is positive. `share` only hands its `total` on, so the duty
passes to `share`'s callers, and the bad call in `main` does not build:

```nika
fn percent(part: int, whole: int) -> int {
    assert(whole > 0)
    return part * 100 / whole
}

fn share(done: int, total: int) -> int {
    return percent(done, total)     // hands the duty on to its caller
}

fn at_least_zero(n: int) -> int {
    return 0 if n < 0
    assert(n >= 0)                  // a claim about the result
    return n
}

fn per_row(rows: int) -> int {
    let r = at_least_zero(rows) + 1
    return percent(50, r)           // r >= 1: proved, no check
}

fn main() {
    println(f"{per_row(4)}% {share(3, 4)}%")
    // and this call breaks it:
    println(f"{share(1, 0)}%")
}
```

```text
$ nikaia build
error[NK1207]: This call breaks `share`'s precondition `total > 0` every time it is reached.
   --> …/pre/src/main.nika:24:5
    |
 24 |     println(f"{share(1, 0)}%")
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: Here `total` is 0.
    = note: The precondition is `assert(whole > 0)` in `percent`, carried back to `share`'s entry.
    = help: Pass `share` arguments for which `total > 0` holds, or check them with a guard before the call.
1 claim shown false
```

The `assert` became a precondition, carried back through `share` to the call that passes 0.
`at_least_zero`'s `assert` is a claim about its result, so `per_row`'s call to `percent` is
proved and has no check at run time. A claim the compiler cannot prove is checked at run time
instead.

More: loops without written invariants, the `--asserts` report, and data from outside, in
[Contracts](guide/contracts.md); `main`'s parameters as the command line in [The command
line](guide/command-line.md).

---

## Hello, world

```nika
// src/main.nika
fn main() {
    println("Hello, Nikaia!")
}
```

```sh
nikaia run
```

[Getting started](guide/getting-started.md) installs the compiler. [A tour of the
syntax](guide/syntax.md) shows the language on one page.

---

## Performance

A preliminary comparison on the One Billion Row Challenge, counted with callgrind
([benches/brc](benches/brc/README.md)), instructions a row at 16 codegen units: the fastest
Rust program written by a person runs 131, an AI-written one after several measured rounds
(AI variant 2) 147, Nikaia's safe `fast.nika` 106 and its plain `examples/1brc.nika` 209;
`fast-unsafe.nika`, the same program with its claims written `unsafe { assume(…) }`, runs 98.
Both Nikaia programs mispredict 0.14 branches a row, the person's program 0.47. At
`x86-64-v3` the person's program runs 92 instructions a row, `fast-unsafe.nika` 84.

---

## Where it fits, and where it stands

| | |
| :--- | :--- |
| **Aimed at** | services that mostly wait; data in unusual formats; heavy computation; code written by AI, with the compiler as reviewer ([Manifesto](manifesto.md), §8) |
| **Not planned** | an interactive REPL: the checks look at the whole program, and a typed line has none |
| **Not yet** | something to ship next quarter, a large ecosystem, years of answered questions online |
| **Status** | pre-alpha. [`examples/`](examples/) holds a calculator, a JSON parser, a log analyser, a small HTTP server and more, and the tests compile and run them. No package registry, no editor integration and no database access yet |
| **Self-hosting** | the compiler is moving into Nikaia a module at a time ([ADR-294](docs/specification/adr/adr-294.md)); the badge above shows the share |

What works today, the limits you will hit and the progress over time are in
[Status & roadmap](docs/project_status_and_roadmap.md). Bug reports and "this cannot work
because…" arguments are welcome.

---

## Where to read next

| If you want to… | Read |
| :--- | :--- |
| install the compiler and run a program | [Getting started](guide/getting-started.md) |
| see what the language looks like | [A tour of the syntax](guide/syntax.md) |
| see what the compiler proves | [Contracts](guide/contracts.md) |
| give a program words on the command line | [The command line](guide/command-line.md) |
| know why the project exists | [Manifesto](manifesto.md) |
| see real programs | [examples/](examples/) |
| learn the full rules | [Specification](docs/specification) and [the ADR index](docs/specification/adr/README.md) |
| work on the compiler | [toolchain architecture](docs/toolchain_architecture.md) |
| read it all rendered | [nikaia-lang.org](https://nikaia-lang.org/) |

---

## License

Nikaia is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file.

<div align="center">
  <sub>For Nika. For everyone downstream.</sub>
</div>
