<div align="center">
  <img src="nikaia-logo.jpg" alt="Nikaia Logo" width="300" height="164" />
  <h1>N I K A I A</h1>
  <p><strong>Good wins.</strong></p>
  <p>
    Write it like a script. Catch what can be proven at compile time.
    Fast as lightning, without <code>unsafe</code>.
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
could be, they need more memory and more machines, and some mistakes only show up in
production, under load, at night.

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
invariants and a proof assistant beside the program. In Nikaia it is an `assert`. Each program
below was built with today's compiler, and the output is pasted as it came (the path of the
scratch directory shortened to `…`).

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

What the compiler did:

* **It made `assert(whole > 0)` a precondition** of `percent`: each caller has to show that
  `whole` is positive. An `assert` further down a function is carried back to the entry
  through each `let`, assignment and branch (weakest preconditions), so you can put it where
  it reads best.
* **`share` passed the duty on.** It only hands `total` to `percent`, so `total > 0` became
  `share`'s own precondition, and the error lands at the caller that passes the bad value,
  with the value and the `assert` it came from. The same holds for a value that only some
  turns of a loop pass (`for i in 0..<10 { percent(1, i) }` is refused when `i` is 0).
  `refuted-claims = "warn"` in `nikaia.toml` turns the refusal into a warning, for code an
  older prover built and a newer one refutes.
* **A claim about the result is a postcondition.** `at_least_zero` never returns less than
  0, so `per_row` knows `r >= 1`, and its call to `percent` has no check at run time.
* **The contract crosses package boundaries.** A `pub fn`'s contract is published in its
  package's ledger, a consumer's calls are proved against it, and a change that weakens what
  callers can rely on warns its author.
* **A proof is checked before it is used.** Each proof comes with a certificate, and a check
  is dropped only when a small checker accepts the certificate. The test suite's proofs are
  also exported as Alethe proofs and verified by the independent checker
  [Carcara](https://github.com/ufmg-smite/carcara).

A call whose arguments are all known is run while the program is built, so a claim about its
result is decided then:

```nika
fn fib(n: int) -> int {
    let mut a = 0
    let mut b = 1
    for _ in 0..<n {
        let next = a + b
        a = b
        b = next
    }
    return a
}

fn main() {
    assert(fib(10) == 54)
    println(f"{fib(10)}")
}
```

```text
$ nikaia build
error[NK1207]: `fib(10) == 54` is false every time it is reached.
   --> …/fib/src/main.nika:13:5
    |
 13 |     assert(fib(10) == 54)
    |     ^^^^^^^^^^^^^^^^^^^^^
    |
    = note: What is known before it rules the claim out: no value of its own; the claim is false as written.
    = note: `fib(10)` is 55, run while the program was built.
    = help: If the claim is right, the code before it is wrong; if the code is right, the claim is.
1 claim shown false
```

### A loop needs no invariant

The `assert` after the loop in `steps` reads `total`, which the loop changes. Nobody wrote
what holds on every turn. The compiler looks for such a fact itself and uses it only after
three checks: it holds before the first turn, each turn keeps it, and when the loop ends it
gives the claim. `--asserts` prints the fact and how it was found. In `price`, `p` is
followed through each assignment and both sides of the `if`
([ADR-361](docs/specification/adr/adr-361.md)).

```nika
fn steps(n: int) -> int {
    assert(n >= 0)
    let mut i = 0
    let mut total = 0
    while i < n {
        i = i + 1
        total = total + 2
    }
    assert(total == 2 * n)          // no invariant written
    return total
}

fn price(base: int, member: bool) -> int {
    assert(base >= 10)
    let mut p = base
    if member {
        p = p - 5
    } else {
        p = p + 1
    }
    assert(p >= 5)                  // followed through both branches
    return p
}

fn main() {
    println(f"{steps(5)} {price(12, true)}")
}
```

```text
$ nikaia build --asserts
asserts in …/loop/src/main.nika: 4 - not proved 0 (refused 0, checked at run time 0, checked by a test 0); checked where the words arrive 0; proved 4 (proved 2, through a loop 1, preconditions proved at every call 2)
  …/loop/src/main.nika:9  assert(total == 2 * n)  proved through the loop that changes `i`, `total`, with the invariant `i >= 0 && total >= 0 && 2 * i == total && i < n + 1`, found by closed forms and checked by three queries
  …/loop/src/main.nika:21  assert(p >= 5)  proved
  …/loop/src/main.nika:2  assert(n >= 0)  precondition of `steps`: proved at every call the compiler sees; any other reaches the entry that checks it
  …/loop/src/main.nika:14  assert(base >= 10)  precondition of `price`: proved at every call the compiler sees; any other reaches the entry that checks it
$ target/nikaia/bin/loop
10 7
```

### The report: what was proved

`nikaia build --asserts` lists every `assert` and what became of it. *Proved* means no check
is left in the program. A *precondition* is each caller's to show. *Checked at run time*
means the prover could not read the claim: it reads linear integer arithmetic, lengths and
branch conditions, not the product of two names like `w * h`. Such a claim does not stop the
build; the program checks it when it runs. A better prover removes checks and does not
change what a correct program does.

```nika
fn average(total: int, count: int) -> int {
    assert(count > 0)
    return total / count
}

fn area(w: int, h: int) -> int {
    assert(w >= 0 && h >= 0)
    let a = w * h
    assert(a >= 0)
    return a
}

fn main() {
    let scores = [3, 5, 9]
    let n = scores.len()
    assert(n == 3)
    println(f"{average(17, n)} {area(4, 5)}")
}
```

```text
$ nikaia build --asserts
asserts in …/rep/src/main.nika: 4 - not proved 1 (refused 0, checked at run time 1, checked by a test 0); checked where the words arrive 0; proved 3 (proved 1, preconditions proved at every call 2)
  …/rep/src/main.nika:9  assert(a >= 0)  checked at run time: `a` is not a whole number the prover reads
  …/rep/src/main.nika:16  assert(n == 3)  proved
  …/rep/src/main.nika:2  assert(count > 0)  precondition of `average`: proved at every call the compiler sees; any other reaches the entry that checks it
  …/rep/src/main.nika:7  assert(w >= 0 && h >= 0)  precondition of `area`: proved at every call the compiler sees; any other reaches the entry that checks it
```

### `main`'s parameters are its command line

`a` is a word the program needs. `times` comes after the `;`, so it is named: on the command
line it is the option `--times`, and 4 where it is left out. The words come from outside, so
an `assert` about them is checked once they are read, before the body runs. A wrong word
ends the program with exit status 2 and a line that says what is wrong; `--help` lists the
precondition ([ADR-360](docs/specification/adr/adr-360.md)).

```nika
/// Divides 100 by a, a few times.
fn main(a: int; times: int = 4) {
    assert(a > 0)
    for i in 0..<times {
        println(f"{i}: {100 / a}")
    }
}
```

```text
$ nikaia build --asserts
asserts in …/mainp/src/main.nika: 1 - not proved 0 (refused 0, checked at run time 0, checked by a test 0); checked where the words arrive 1; proved 0 (proved 0, preconditions proved at every call 0)
  …/mainp/src/main.nika:3  assert(a > 0)  precondition of `main`, checked where the words arrive
$ ./mainp 5 --times 2
0: 20
1: 20
$ ./mainp 0
error: `a` is 0, which breaks `main`'s precondition `a > 0`.
usage: mainp [--times <int>] <a>
For more, try `mainp --help`.
$ echo $?
2
$ ./mainp five
error: `a` is an `int`, and `five` is not one.
usage: mainp [--times <int>] <a>
For more, try `mainp --help`.
$ ./mainp --help
mainp
Divides 100 by a, a few times.

usage: mainp [--times <int>] <a>

arguments:
  <a>                  int; requires `a > 0`

options:
  --times <int>        int, default 4
  -h, --help           prints this help
```

### Data from outside: a guard, not an assert

What another program prints is not up to your program. If it is not what you expected, that
is not a bug in your code but a case it has to handle. So an `assert` about such data is
refused, and the error says what to write instead:

```nika
use std::process

fn main() throws {
    let out = process::run("git", ["rev-parse", "HEAD"])
    let n: int = out.stdout.len()
    assert(n == 41)
    println(f"commit {out.stdout.trim()}")
}
```

```text
$ nikaia build
error[NK1202]: `n == 41` is a claim about data from outside the program.
  --> …/outa/src/main.nika:6:5
   |
 6 |     assert(n == 41)
   |     ^^^^^^^^^^^^^^^
   |
   = note: `n` comes from outside the program, so nothing the compiler can see says what it holds.
   = note: That it is wrong is a case the program has to handle, not a defect an `assert` catches.
   = help: Check it where it arrives, with a guard the program handles: `throw BadInput(n) if !(n == 41)`, or `return … if !(n == 41)`. After that line, this `assert` is proved.
1 type error
```

After an `if` that handles the bad case, the same `assert` is proved:

```nika
use std::process

fn main() throws {
    let out = process::run("git", ["rev-parse", "HEAD"])
    let n: int = out.stdout.len()
    if n != 41 {
        println("not a commit id")
        return
    }
    assert(n == 41)
    println(f"commit {out.stdout.trim()}")
}
```

```text
$ nikaia build --asserts
asserts in …/outb/src/main.nika: 1 - not proved 0 (refused 0, checked at run time 0, checked by a test 0); checked where the words arrive 0; proved 1 (proved 1, preconditions proved at every call 0)
  …/outb/src/main.nika:10  assert(n == 41)  proved
$ target/nikaia/bin/outb
commit 7e84ae5c523c8b9937bb4faacc6d029bf49d4f75
```

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
  otherwise parse with regular expressions and hope.
* **Heavy computation**, such as simulations, image and signal processing, and aggregating
  large inputs, with the parallel parts checked by the compiler instead of hoped for.
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
