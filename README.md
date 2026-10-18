<div align="center">
  <img src="nikaia-logo.jpg" alt="Nikaia Logo" width="300" height="164" />
  <h1>N I K A I A</h1>
  <p><strong>Good wins.</strong></p>
  <p>
    Write it like a script. Catch what can be proven at compile time.
    Run within about 10 % of tuned Rust, without <code>unsafe</code>.
  </p>

  <p>
    <a href="#the-idea">The idea</a> •
    <a href="#an-assert-is-a-contract">An assert is a contract</a> •
    <a href="#performance-against-tuned-rust">Performance</a> •
    <a href="#hello-world">Hello, world</a> •
    <a href="#progress">Progress</a> •
    <a href="guide/getting-started.md">Getting started</a> •
    <a href="guide/syntax.md">Syntax tour</a> •
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

Languages like Python or JavaScript are quick to write in. The programs are slower than they
could be, they need more memory, and some mistakes only show up when the program runs.

Languages like C++ or Rust produce fast programs. In exchange, a good part of each program
tells the compiler how memory, waiting and sharing should work.

Nikaia's bet is that a compiler which sees the whole program can work much of that out
itself: which data needs protecting, where a function waits, when memory can be freed, and
how to spread work over several cores. You write what should happen, and the compiler picks
how.

What that means in practice:

* **An `assert` is checked by the compiler.** There is no separate proof language. You write
  `assert(whole > 0)`, and the compiler turns it into a contract that every caller must meet.
  A call that breaks it does not build. A claim the compiler cannot prove yet is checked at
  run time. [More below](#an-assert-is-a-contract).
* **Checks the compiler can prove are dropped.** If it shows that an index stays inside its
  list, or a sum inside its type, a build that asks for it drops that check. On the One
  Billion Row Challenge, Nikaia uses about 10 % more instructions than tuned `unsafe` Rust,
  fewer than half its mispredicted branches, and about a quarter less source.
  [The numbers](#performance-against-tuned-rust).
* **No `async` or `await`.** Code that reads files or talks to the network is written like
  any other code. A call that waits pauses its function, and the program goes on with other
  work.
* **Some mistakes are refused at compile time.** Two tasks changing the same data at once is
  one. A value that may be missing and is used without a check is another, and so is an error
  nobody handles. The message says what to write instead.
* **No garbage collector and no `free`.** Memory and files are released at a point you can
  see in the code.
* **One core or many is a setting.** Whether your code may run on several cores is one line
  in `nikaia.toml`. The code stays the same.
* **Formats are part of the language.** To parse a log, a protocol or a small language, you
  describe the format as a grammar.

Nikaia compiles to Rust, and Rust's compiler checks memory safety. Nikaia's job is to give you
those guarantees without making you write out what Rust asks for.

The name comes from my daughter, **Nika**. In Persian, *nik* means *good*, and the motto is
the name with a verb: *good wins*. It means that being kind to the person writing the code
and producing fast programs do not have to conflict. The longer story, and the reasoning
behind each point above, is in the [Manifesto](manifesto.md).

---

## An assert is a contract

Formal verification usually means a second language: specification clauses, ghost code, loop
invariants and a proof assistant beside the program. In Nikaia it is an `assert`.

```nika
fn percent(part: int, whole: int) -> int {
    assert(whole > 0)              // that is the whole specification
    return part * 100 / whole
}

fn report(done: int, total: int) -> int {
    return 0 if total <= 0         // this guard proves the claim for this call
    return percent(done, total)
}

fn main() {
    println(f"{report(3, 4)}%")
    println(f"{percent(1, 0)}%")   // and this call breaks it
}
```

`nikaia build --asserts` on this program, which does not build:

```text
asserts in src/main.nika: 1 - proved 0, preconditions 1, checked at run time 0, checked by a test 0, refused 0
  src/main.nika:2  assert(whole > 0)  precondition of `percent`: one call checks it when it runs; any other call proves it or reaches the entry that checks it

error[NK1207]: This call breaks `percent`'s precondition `whole > 0` every time it is reached.
   --> src/main.nika:13:5
    |
 13 |     println(f"{percent(1, 0)}%")
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: Here `whole` is 0.
    = help: Pass `percent` arguments for which `whole > 0` holds, or check them with a guard before the call.
1 claim shown false
```

What the compiler did:

* **It made `assert(whole > 0)` a precondition** of `percent`. A claim about the result
  would have become a postcondition. An `assert` further down a function is carried back to
  the entry through each `let`, assignment and branch (weakest preconditions), so you can put
  it where it reads best.
* **Each call has to establish it.** `report`'s guard proves it, so that call has no check
  at run time. The call with `0` is shown to break it, with the value, and the program does
  not build. The same holds for a value that only some turns of a loop pass
  (`for i in 0..<10 { percent(1, i) }` is refused when `i` is 0). A function that passes its
  parameter on hands the precondition to its own callers, so the error lands at the caller
  that passes the bad value. `refuted-claims = "warn"` in `nikaia.toml` turns the refusal
  into a warning, for code an older prover built and a newer one refutes.
* **The contract crosses package boundaries.** A `pub fn`'s contract is published in its
  package's ledger, a consumer's calls are proved against it, and a change that weakens what
  callers can rely on warns its author.
* **A claim that is hard to prove is not refused.** The prover reads linear integer
  arithmetic, lengths and branch conditions. A claim outside that is checked when the program
  runs, and `--asserts` lists it with the reason. A better prover removes checks and does not
  change what a correct program does.
* **A proof is checked before it is used.** Each proof comes with a certificate, and a check
  is dropped only when a small checker accepts the certificate. The test suite's proofs are
  also exported as Alethe proofs and verified by the independent checker
  [Carcara](https://github.com/ufmg-smite/carcara).

The rules are [ADR-269](docs/specification/adr/adr-269.md); the solver is
[ADR-270](docs/specification/adr/adr-270.md).

---

## Performance against tuned Rust

The comparison is with Rust as a specialist tunes it, `unsafe` included. On the
[One Billion Row Challenge](benches/brc/README.md), the Rust side is a single-core entry
that maps the file, skips UTF-8 validation and hashes a word at a time:

| 1BRC, per row | Nikaia, [`examples/1brc.nika`](examples/1brc.nika) | Rust, tuned |
| :--- | ---: | ---: |
| instructions | 300 | **272** |
| mispredicted branches | **1.58** | 3.70 |
| tokens of source, `use` included | **618** | 809 |
| `unsafe` | **0** | 1 |
| UTF-8 checked | **yes** | no |
| overflow checked | **yes** | no |
| the same file on every core | **yes** | no |

Both are release builds at `opt-level = 3`, neither with link-time optimisation, on one
core, with the same output byte for byte. The counts come from callgrind over a million rows,
not from a clock, because on a shared machine a time varies more than a count. Nikaia's
overflow checks cost 2 of its 300 instructions.

Where a proof shows that an index or overflow check cannot fail, a build that asks for it
(`--optimization`, [ADR-306](docs/specification/adr/adr-306.md)) drops the check.

The first version of a program does well too. Here the Rust is what a competent programmer
writes first: `read_to_string`, `lines`, a cut at the `;` and the standard `HashMap`. Both
sides use the same algorithm:

| 1BRC, per row | Nikaia, [`naive.nika`](benches/brc/naive.nika) | Rust, [`naive.rs`](benches/brc/src/bin/naive.rs) |
| :--- | ---: | ---: |
| instructions | **598** | 861 |
| mispredicted branches | **8.32** | 8.72 |
| tokens of source, `use` included | **320** | 486 |

That is a third less source and 30 % fewer instructions. The difference comes from the
defaults each language chose: the hasher, the UTF-8 check, and how a one-character pattern is
searched. The [bench's README](benches/brc/README.md) shows where the instructions go.

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

A slightly larger example reads two files at the same time and adds up their sizes. It uses
no `async`, `await`, callback or thread.

```nika
use std::fs

fn size_of(path: String) -> int throws {
    let text = fs::read_to_string(path, fs::Root::Anywhere)
    return text.len()
}

fn main() throws {
    let (a, b) = overlap {
        size_of("a.txt")
        size_of("b.txt")
    }
    println(f"{a + b} characters in both files")
}
```

**Next:** [Getting started](guide/getting-started.md) installs the compiler and runs your
first project. [A tour of the syntax](guide/syntax.md) shows the language on one page, for
readers who know Python, JavaScript, Go, Java or similar.

---

## Good at, and not good at

**Nikaia is aimed at:**

* **Services that mostly wait**, such as web APIs, proxies, and command-line tools that talk
  to the network. Written much like Go or Node.js, without garbage-collection pauses.
* **Data in unusual formats**, such as logs, telemetry and protocol frames, that you would
  otherwise parse with regular expressions.
* **Heavy computation**, such as simulations, image and signal processing, and aggregating
  large inputs, with the parallel parts checked by the compiler.
* **Code written by AI.** When a model writes most of the code, a compiler that refuses
  races, deadlocks and unhandled errors acts as a reviewer
  ([why](manifesto.md), §8).

**Look elsewhere if you need:**

* **An interactive REPL.** Nikaia does not plan one: its checks look at the whole program,
  and a line typed into a prompt has no whole program around it.

For now, also look elsewhere if you need what only time brings:

* **Something to ship next quarter.** Nikaia is pre-alpha (see below).
* **A large ecosystem** of libraries.
* **Years of answered questions online.**

---

## Where the project stands

**Pre-alpha.** The compiler builds real programs: the [`examples/`](examples/) directory holds
a calculator, a JSON parser, a log analyser, a small HTTP server, the One Billion Row
Challenge and more, and the test suite compiles and runs all of them except the one that needs
a database. You will also hit missing pieces quickly: there is no package registry, no
formatter or editor integration yet, and no database access.

The [status & roadmap](docs/project_status_and_roadmap.md#where-the-project-actually-stands)
lists what works today, the limits you will hit, and which parts of the design most need
testing. Bug reports and "this cannot work because…" arguments are welcome. An unfinished
language is the cheapest place to find out you are wrong.

---

## Progress

The compiler is moving into Nikaia itself, a module at a time
([ADR-294](docs/specification/adr/adr-294.md)). The analyses that decide sharing, pausing,
lifetimes of views, which statements may overlap and what a function throws are already
Nikaia code that the Rust half of the compiler calls.

| Self-hosted share of the toolchain | Sep 28 | Sep 29 | Sep 30 | Oct 1 | Oct 2 | Oct 3 | Oct 4 |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| lines of the toolchain written in Nikaia | 2.0 % | 2.4 % | 8.4 % | 10.2 % | 17.6 % | 19.7 % | **30.6 %** |

From the first module on September 28 to 18 625 lines in 61 modules six days later, about
4.8 percentage points a day. `scripts/self_hosting.py` recounts it from the sources.

Claude Opus started writing the compiler on September 5, 2026. Each design decision is
argued in an [ADR](docs/specification/adr/README.md) and accepted by a human.

| in 30 days | in all | per day |
| :--- | ---: | ---: |
| commits | 1 233 | **≈ 41** |
| lines of toolchain code (no blanks, no comments) | 541 → 60 959 | **≈ 2 000** |
| change packages, each a [CHANGELOG](CHANGELOG.md) entry, since numbering began on Sep 19 | 0.0.8 → 0.0.454 | **≈ 28** |

*As of 0.0.454, October 4, 2026. Counted from `git log` and `scripts/self_hosting.py`.*

---

## Where to read next

| If you want to… | Read |
| :--- | :--- |
| install the compiler and run a program | [Getting started](guide/getting-started.md) |
| see what the language looks like | [A tour of the syntax](guide/syntax.md) |
| know why the project exists, and how it compares | [Manifesto](manifesto.md) |
| know what works today and what comes next | [Status & roadmap](docs/project_status_and_roadmap.md) |
| see real programs | [examples/](examples/) |
| learn the full rules | [Specification, Part I](docs/specification/10-nikaia-light.md) |
| see the concurrency and parallelism model | [Specification, Part II](docs/specification/20-nikaia-advance.md) |
| understand a design decision | [the ADR index](docs/specification/adr/README.md) |
| let a model write Nikaia for you | [the spec](docs/specification) + [examples/](examples/) in one context |
| work on the compiler | [toolchain architecture](docs/toolchain_architecture.md) |

---

## License

Nikaia is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file.
On prior art and governance, see the [Manifesto](manifesto.md), §10.

<div align="center">
  <sub>For Nika. For everyone downstream.</sub>
</div>
