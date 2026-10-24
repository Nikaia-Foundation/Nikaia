# Contracts: an assert the compiler checks

This page shows what Nikaia does with an `assert`. Formal verification usually means a
second language: specification clauses, ghost code, loop invariants and a proof assistant
beside the program. In Nikaia it is an `assert`. You write `assert(whole > 0)`, and the
compiler turns it into a contract that every caller must meet. A call that breaks it does
not build. A claim the compiler cannot prove yet is refused too, with `NK1300`, unless the
build says `unproved-claims = "warn"`: then it is a warning, and the program checks the
claim when it runs.

Each program below was built with today's compiler, and the output is pasted as it came (the
path of the scratch directory shortened to `…`). To try one, put it in `src/main.nika` of a
project (see [Getting started](getting-started.md)) and run `nikaia build`.

---

## Preconditions and postconditions


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
* **A proved check can be dropped.** Where a proof shows that an index or overflow check
  cannot fail, a build that asks for it (`--optimization`,
  [ADR-306](../docs/specification/adr/adr-306.md)) drops the check.

## A claim decided while building

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

## A loop needs no invariant

The `assert` after the loop in `steps` reads `total`, which the loop changes. Nobody wrote
what holds on every turn. The compiler looks for such a fact itself and uses it only after
three checks: it holds before the first turn, each turn keeps it, and when the loop ends it
gives the claim. `--asserts` prints the fact and how it was found. In `price`, `p` is
followed through each assignment and both sides of the `if`
([ADR-361](../docs/specification/adr/adr-361.md)).

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

## The report: what was proved

`nikaia build --asserts` lists every `assert` and what became of it. *Proved* means no check
is left in the program. A *precondition* is each caller's to show. *Refused*
means the prover could not read the claim: it reads linear integer arithmetic, lengths and
branch conditions, not the product of two names like `w * h`. Such a claim stops the build
with `NK1300`: not proved is not false, but nothing has shown it. With `[build]
unproved-claims = "warn"` in `nikaia.toml`, or `--unproved-claims=warn` on the command line,
it is a warning instead, and the program checks the claim when it runs. A better prover
removes checks and does not change what a correct program does.

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
asserts in …/rep/src/main.nika: 4 - not proved 1 (refused 1, checked at run time 0, checked by a test 0); checked where the words arrive 0; proved 3 (proved 1, preconditions proved at every call 2)
  …/rep/src/main.nika:9  assert(a >= 0)  refused, not proved: `a` is not a whole number the prover reads
  …/rep/src/main.nika:16  assert(n == 3)  proved
  …/rep/src/main.nika:2  assert(count > 0)  precondition of `average`: proved at every call the compiler sees; any other reaches the entry that checks it
  …/rep/src/main.nika:7  assert(w >= 0 && h >= 0)  precondition of `area`: proved at every call the compiler sees; any other reaches the entry that checks it
error[NK1300]: `assert(a >= 0)` is not proved: `a` is not a whole number the prover reads.
  --> …/rep/src/main.nika:9:5
   |
 9 |     assert(a >= 0)
   |     ^^^^^^
   |
   = note: Not proved is not false: the compiler could not show the claim from what it knows where it stands, and it may well hold.
   = note: `nikaia check --asserts` lists every claim and why it is not proved.
   = help: Give the compiler what it needs to prove it, or build with `unproved-claims = "warn"` to have it checked when the program runs.
1 type error
```

With the switch the same claim is a warning, and the program builds and runs:

```text
$ nikaia build --unproved-claims=warn
warning[NK1300]: `assert(a >= 0)` is not proved: `a` is not a whole number the prover reads.
  --> …/rep/src/main.nika:9:5
   |
 9 |     assert(a >= 0)
   |     ^^^^^^
   |
   = note: Not proved is not false: the compiler could not show the claim from what it knows where it stands, and it may well hold.
   = note: `nikaia check --asserts` lists every claim and why it is not proved.
   = note: `unproved-claims = "warn"` lets this build through: the claim is checked when the program runs, and a false one stops it there.
   = help: Give the compiler what it needs to prove it, or build with `unproved-claims = "warn"` to have it checked when the program runs.
$ ./target/nikaia/bin/rep
5 20
```

## `main`'s parameters are its command line

`main`'s caller is the person at the terminal, so an `assert` about its parameters is
checked by the program itself, once the words are read and before the body runs. A breach
ends the program with exit status 2 and a line that says what is wrong. How the words reach
the parameters, options, `--help` and the rest are in [The command line](command-line.md).

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
```

## Data from outside: a guard, not an assert

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

**Specification:** [ADR-269](../docs/specification/adr/adr-269.md) (the rules),
[ADR-270](../docs/specification/adr/adr-270.md) (the solver),
[ADR-361](../docs/specification/adr/adr-361.md) (loops),
[ADR-360](../docs/specification/adr/adr-360.md) (`main`'s parameters).

---

## Where next

* [A tour of the syntax](syntax.md): the rest of the language on one page.
* [Getting started](getting-started.md): install the compiler and run a project.
* [`examples/`](../examples/): complete programs that compile and run.
