<div align="center">
  <img src="nikaia-logo.jpg" alt="Nikaia Logo" width="300" height="164" />
  <h1>N I K A I A</h1>
  <p><strong>Good wins.</strong></p>
  <p>
    Safe, very fast and easy to read, all three at once.
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
  <a href="https://github.com/Nikaia-Foundation/Nikaia"><img src="assets/badges/github.svg" alt="GitHub repository" /></a>
</div>

---

## The reliable programming language

Nikaia is a statically typed, compiled, type-safe programming language with easy
verification. The compiler catches easy-to-miss mistakes before the program runs, the
program becomes optimised machine code, and the code stays concise.

The compiler is pre-alpha. What works today is in
[Status & roadmap](docs/project_status_and_roadmap.md); the reasons behind the project are in
the [Manifesto](manifesto.md).

---

## Safe, secure, reliable

A program fails in more than one way. Nikaia looks at five of them, checks at compile time
wherever it can, and says for each where its checks stop.

- A call that can fail is handled or passed on, or the program is
  not built. What a function can fail with is worked out from its body, so its signature says
  `throws` and not a list of every error. A task that can panic ends alone, and whoever waits
  for it handles the crash. An index past the end, an overflow or a division by zero that the
  compiler can show happens is refused.
- A claim about data from outside waits for a guard where the data
  arrives, a path from outside stays in the directory the call names, and formats are read by
  parsers built from a grammar.
- An `assert` is a contract, and the compiler checks it at
  every call (below).
- Every change is checked against the rules the code states,
  whoever wrote it, and a change that breaks one is refused with what to write instead.
  `nikaia build --asserts` lists which claims were proved and which are checked at run time.
- A wait with a deadline has to handle its timeout, a lock held
  across a pause is refused, and two tasks that would change the same value at once are
  refused with the value they share named. A task that fills its memory block ends alone with
  `task::OverBudget`, and the tasks beside it run on.

Where the compiler cannot tell what a call touches, as in a Rust crate or a C library, it does
not refuse, and the report says so.

### An assert is a contract

An `assert` at the start of a function is its contract: it says what every caller has to hand
it. In this quicksort, `partition` and `sort` ask for a range that lies inside the list. The
`assert` at the end of `partition` is its promise in return, that the pivot lands inside that
range, and that is what lets `sort` go on with both halves:

```nika
fn partition(mut a: Vec[int], lo: int, hi: int) -> int {
    assert(lo >= 0 && lo <= hi && hi < a.len())   // what every caller must hand in
    let pivot = a[hi]
    let mut i = lo
    for j in lo..<hi {
        if a[j] < pivot {
            let t = a[i]
            a[i] = a[j]
            a[j] = t
            i += 1
        }
    }
    a[hi] = a[i]
    a[i] = pivot
    assert(i >= lo && i <= hi)    // the promise back: the pivot lands inside the range
    return i
}

fn sort(mut a: Vec[int], lo: int, hi: int) {
    assert(lo >= 0 && hi < a.len())               // what every caller must hand in
    if lo < hi {
        let p = partition(a, lo, hi)
        sort(a, lo, p - 1)
        sort(a, p + 1, hi)
    }
}

fn main() {
    let mut v: Vec[int] = [5, 3, 9, 1, 4]
    sort(v, 0, v.len())           // one past the end
}
```

```text
$ nikaia run qs.nika
error[NK1207]: This call breaks `sort`'s precondition `lo >= 0 && hi < a.len()` every time it is reached.
   --> qs.nika:30:5
    |
 30 |     sort(v, 0, v.len())
    |     ^^^^^^^^^^^^^^^^^^^
    |
    = note: Here `a.len()` is 5, `hi` is 5, `lo` is 0.
    = help: Pass `sort` arguments for which `lo >= 0 && hi < a.len()` holds, or check them with a guard before the call.
1 claim shown false
```

The compiler proves each `assert` through loops and recursion, without loop invariants written
by hand. With `sort(v, 0, v.len() - 1)` the program builds, and with these three asserts every
index is proved in range, so none is checked when the program runs. A claim the compiler cannot
prove is refused (`NK1300`); a build that says `unproved-claims = "warn"` builds it with a
warning and checks it at run time.

`lo` and `hi` are `int`, the type of a length and an index, so `p - 1` may become -1 for an
empty range and no index needs a conversion. More in [Contracts](guide/contracts.md).

---

## Very fast

Programs written in Nikaia become optimised machine code, release memory at a point the code
shows, without collector pauses, and can spread their work over every core.

- On the One Billion Row Challenge, counted with callgrind
  ([benches/brc](benches/brc/README.md)) at the baseline CPU level, Nikaia's safe program
  already does less work than the fastest Rust a person wrote, and mispredicts a third as many
  branches. The figures and how they are taken are in that README.
- Whether your code may run on several cores is one line in
  `nikaia.toml`, and the code does not change:

  ```toml
  [build]
  user-parallelism = "no"    # or "yes"
  ```
- Memory is released without `free` and without lifetimes written by hand: the
  compiler works out when memory can be released, and the release is a point you can see.
- A build setting tells the compiler to leave out the index
  and overflow checks it has proved cannot fail:

  ```toml
  [build]
  optimization = "remove-bounds-checks:on,remove-overflow-checks:on"
  ```

---

## Easy to understand

You write what should happen, and Nikaia works out the technical details other fast languages
make you spell out. When something is wrong, it tells you in plain words what to change.

- A function that waits for a file or the network
  is called like any other, and a wait added deep inside leaves every caller as it is. The
  program waits where it needs the value:

  ```nika
  let a = async count_words(first)    // starts at once
  let b = async count_words(second)   // runs beside the first
  println("both counts have started")
  println(f"{a + b} words")           // waits for both
  ```
- An error says what you wrote, why it cannot work and what to write
  instead, so a person or an agent can fix the code and build again:

  ```text
  error[NK1139]: You're changing `count`, but it wasn't declared as mutable.
  help: Add `mut` where it's declared: `let mut count`.
  ```
- Describe a format as a grammar and the compiler builds
  the parser ([Grammars](guide/grammars.md)).

### Hello, world

```nika
// src/main.nika
fn main() throws {
    println("Hello, Nikaia!")
}
```

```sh
nikaia run
```

[Getting started](guide/getting-started.md) installs the compiler. [A tour of the
syntax](guide/syntax.md) shows the language on one page.

---

## Made for good

Software that fails less hurts fewer people. Programs that do more with less hardware use less
energy. And a language anyone can read lets more people build.

Nikaia is built in the open: each design decision is argued in an
[ADR](docs/specification/adr/README.md) and accepted by a human, and the compiler is moving
into Nikaia a module at a time ([ADR-294](docs/specification/adr/adr-294.md)); the badge above
shows the share. The name comes from my daughter, Nika: in Persian, *nik* means *good*, so the
motto is *good wins*.

---

## Where it fits, and where it stands

| | |
| :--- | :--- |
| **Built for** | services that have to keep running; data processing and batch jobs; backends under load; small machines and cloud bills; code a model writes, with the compiler as reviewer ([Manifesto](manifesto.md), §8); many tasks at once; languages and formats; libraries with one version of each function; small tools that grow |
| **Not planned** | an interactive REPL: the checks look at the whole program, and a typed line has none |
| **Not yet** | something to ship next quarter, a large ecosystem, years of answered questions online |
| **Status** | pre-alpha. A calculator, a JSON parser, a log analyser, a small HTTP server and the One Billion Row Challenge build today ([`examples/`](examples/)). `nikaia fmt` formats, `nikaia lsp` shows the checker's messages in the editor, and a dependency is found by version through crates.io, though no Nikaia package is published there yet. There is no TLS and no database access yet |

No model has Nikaia in its training data yet. Generating it means putting the specification in
the model's context, and `docs/specification/` together with `examples/` serves for that today.

Bug reports and "this cannot work because…" arguments are welcome.

---

## Build Nikaia with us

Nikaia is built in the open, and there is room to help at every size: a clearer error message,
a missing example or a fix as a [pull request](https://github.com/Nikaia-Foundation/Nikaia/pulls);
an issue marked [`ready`](https://github.com/Nikaia-Foundation/Nikaia/issues?q=is%3Aissue+is%3Aopen+label%3Aready),
which is decided and says what to build; a part of the language you look after; or a question
on [Discord](https://discord.gg/CtnPEbPDWu).

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
