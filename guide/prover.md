# The prover: what it shows today, and where it stops

A program can fail at run time in a few ways the compiler can see coming: a claim written as
an `assert` that does not hold, an index past the end of a list, an addition or a
multiplication that leaves its whole-number type. Nikaia's compiler tries to show, while it
builds the program, that none of these happens. Where it shows it, the program carries no
check for it. Where it cannot, an index or an arithmetic step keeps its check and stops the
program at the first bad value, and an `assert` stops the build until the claim is shown or
the build asks for a check at run time instead. A better prover removes checks; it does not
change what a correct program does.

This page describes the reach of that prover as a whole: what it reads, what it does with a
claim it proves and with one it cannot, and how to read its reports. [Contracts](contracts.md)
introduces `assert` itself, preconditions and postconditions, step by step.

Each program below was checked with today's compiler, and the output is pasted as it came.
`nikaia check FILE` checks one file without building it, so each example can be tried as a
single file.

---

## What the prover reads

The prover works on whole numbers. It reads comparisons built from `+`, `-`, a `*` by a
constant and a `/` or `%` by a positive constant, joined with `&&`, `||` and `!`. The names in
them may be parameters, `let` bindings, `let mut` bindings followed through each assignment,
the `len()` of a list or text that does not change, and fields of a value that nothing can
change while the claim is read. It knows which side of an `if` it is on, what a guard such as
`return 0 if n < 0` has ruled out, the range a `for` walks, and every claim proved before.
The full rule is Part III 14.2 of the specification.

From that it decides four kinds of question:

* an `assert`: a claim about the values where it stands;
* a precondition or a postcondition: an `assert` that names only a function's parameters
  becomes something each caller has to show, and one about the value returned becomes
  something each caller knows of the result;
* an index `xs[i]`: whether `0 <= i < xs.len()` holds;
* an arithmetic step: whether a `+`, `-` or `*` stays inside its type, such as `i32`.

## Claims across calls

In `slot` below, the index `xs[i]` depends on what `clamp` returns. `clamp` ends with an
`assert` about its result, so each caller knows that result lies between `0` and `hi`. `slot`
asks its own callers for a non-empty list, so `xs.len() - 1` cannot go below zero, and the
index is shown to be inside the list.

```nika
fn clamp(n: int, hi: int) -> int {
    assert(hi >= 0)
    return 0 if n < 0
    return hi if n > hi
    assert(n >= 0 && n <= hi)
    return n
}

fn slot(xs: Vec[int], n: int) -> int {
    assert(xs.len() > 0)
    let i = clamp(n, xs.len() - 1)
    return xs[i]
}

fn main() {
    let xs = [10, 20, 30]
    println(f"{slot(xs, 7)} {slot(xs, -2)}")
}
```

```text
$ nikaia check --asserts --bounds slot.nika
asserts in slot.nika: 3 - not proved 0 (refused 0, checked at run time 0, checked by a test 0); checked where the words arrive 0; proved 3 (proved 1, preconditions proved at every call 2)
  slot.nika:5  assert(n >= 0 && n <= hi)  proved
  slot.nika:2  assert(hi >= 0)  precondition of `clamp`: proved at every call the compiler sees; any other reaches the entry that checks it
  slot.nika:10  assert(xs.len() > 0)  precondition of `slot`: proved at every call the compiler sees; any other reaches the entry that checks it
bounds in slot.nika: 1 indexes, 1 proved; 1 operations, 1 proved - written without their check: indexes off, operations off
    11  xs.len() - 1  proved
    12  xs[i]         proved
Checked slot.nika
```

Both preconditions are shown at every call in the file, so none of these calls carries a
check. A caller the compiler does not see, such as a function value or another language
calling in, reaches an entry that still checks the precondition. A claim that is false every
time it is reached, or a call that always breaks a precondition, is refused with `NK1207`
and the values that break it; [Contracts](contracts.md) shows both.

## Index and overflow checks

An `assert` is a claim you write. Index and overflow checks are claims the compiler writes
for you, one at each `xs[i]` and at each `+`, `-` and `*` of a fixed-size whole number.
`--bounds` lists each of them with what became of it:

```nika
fn sum(xs: Vec[int]) -> int {
    let mut total = 0
    for i in 0..<xs.len() {
        total = total + xs[i]
    }
    return total
}

fn first(xs: Vec[int]) -> int {
    return xs[0]
}

fn percent(part: i32) -> i32 {
    return 0 if part < 0 || part > 1000
    return part * 100
}

fn double(x: i32) -> i32 {
    return x * 2
}

fn main() {
    let xs = [4, 8, 15]
    println(f"{sum(xs)} {first(xs)} {percent(7)} {double(21)}")
}
```

```text
$ nikaia check --bounds checks.nika
bounds in checks.nika: 2 indexes, 1 proved; 3 operations, 1 proved - written without their check: indexes off, operations off
     4  total + xs[i]  checked: a counter a loop changes, with no bound tied to the loop's turns (#1053 step 4, #432)
     4  xs[i]          proved
    10  xs[0]          checked: no fact bounds what it reads - a counter, a loop, a length, or one list as long as another (#432, #383, #388)
    15  part * 100     proved
    19  x * 2          checked: a parameter no precondition bounds: an `assert` at the top of the function makes it one (ADR-269 D5)
Checked checks.nika
```

The index in `sum` is inside the range the loop walks, and the guard in `percent` keeps
`part * 100` inside `i32`. The other three keep their checks: nothing says the list handed to
`first` has an element, nothing bounds the parameter of `double`, and the running total adds
elements whose size nothing states. For a checked line the report names what stopped the
proof and, where one exists, the issue that would supply the missing fact.

A proved check is left out of the program only when the build asks for it, with the
`optimization` setting in `nikaia.toml` or `--optimization` on the command line
([ADR-306](../docs/specification/adr/adr-306.md)). What is proved is the same either way; the
setting only decides whether it is used, and the header of the report says which:

```text
$ nikaia check --bounds --optimization=remove-bounds-checks:on --optimization=remove-overflow-checks:on checks.nika
bounds in checks.nika: 2 indexes, 1 proved; 3 operations, 1 proved - written without their check: indexes on, operations on
```

A check that is not proved stays in the program at every setting.

## Loops

A loop is where most checks sit, and the prover follows it without an invariant written by
hand. It looks for a fact that holds before the first turn, that each turn keeps, and that
gives the claim when the loop ends, and it uses the fact only after showing all three, as [Contracts](contracts.md#a-loop-needs-no-invariant)
shows for an `assert`. A `for` also counts its turns, so a counter that grows by at most one a
turn stays below the length of the list it walks:

```nika
fn evens(xs: Vec[int]) -> i64 {
    let mut n: i64 = 0
    for x in xs {
        if x % 2 == 0 {
            n += 1
        }
    }
    return n
}

fn evens_small(xs: Vec[int]) -> i32 {
    let mut n: i32 = 0
    for x in xs {
        if x % 2 == 0 {
            n += 1
        }
    }
    return n
}

fn main() {
    let xs = [1, 2, 4, 7]
    println(f"{evens(xs)} {evens_small(xs)}")
}
```

```text
$ nikaia check --bounds counters.nika
bounds in counters.nika: 0 indexes, 0 proved; 2 operations, 1 proved - written without their check: indexes off, operations off
     5  n += 1  proved
    15  n += 1  checked: a counter a loop changes, with no bound tied to the loop's turns (#1053 step 4, #432)
Checked counters.nika
```

The two functions differ only in the counter's type, and the second check is right to stay. A
list may hold up to 2^60 elements, which an `i64` counter can count and an `i32` cannot.
The prover takes no shortcut on the size of the data.

By default the compiler asks only `assert`s through a loop's invariant. With
`prove-through-loops:on` in `optimization` it asks index and overflow checks the same way.
In `probe` the slot `at` wraps round a table of 16384 entries, and only an invariant carried
from turn to turn shows that it stays inside:

```nika
fn probe(xs: Vec[i64], h: i64) -> i64 {
    assert(xs.len() == 16384)
    let mut at = h & 16383
    while true {
        if xs[at] == 0 {
            return at
        }
        at = (at + 1) & 16383
    }
    return -1
}

fn main() {
    let mut xs: Vec[i64] = []
    xs.resize(16384, 1)
    xs[5] = 0
    println(f"{probe(xs, 16383 + 7)}")
}
```

```text
$ nikaia check --bounds probe.nika
bounds in probe.nika: 2 indexes, 1 proved; 1 operations, 0 proved - written without their check: indexes off, operations off
     5  xs[at]  checked: no fact bounds what it reads - a counter, a loop, a length, or one list as long as another (#432, #383, #388)
     8  at + 1  checked: no fact bounds what it reads - a counter, a loop, a length, or one list as long as another (#432, #383, #388)
    16  xs[5]   proved
Checked probe.nika
$ nikaia check --bounds --optimization=prove-through-loops:on probe.nika
bounds in probe.nika: 2 indexes, 2 proved; 1 operations, 1 proved - written without their check: indexes off, operations off
     5  xs[at]  proved
     8  at + 1  proved
    16  xs[5]   proved
Checked probe.nika
```

The setting changes what is proved, and so what the other two settings may leave out; it
does not change what the program does.

## When a claim is not shown

What happens next depends on who wrote the claim.

An index or overflow check the prover cannot show stays in the program and does its work when
the program runs. Here `first` gets an empty list:

```nika
fn first(xs: Vec[int]) -> int {
    return xs[0]
}

fn main() {
    let empty: Vec[int] = []
    println(f"{first([3, 4])}")
    println(f"{first(empty)}")
}
```

```text
$ nikaia run stop.nika
3
stop.nika:2: the program stopped: index out of bounds: the len is 0 but the index is 0
```

An `assert` the prover cannot show stops the build with `NK1300`. Not proved is not false, so
the message says the claim may well hold, and why the prover could not show it. With
`unproved-claims = "warn"` in `nikaia.toml`, or `--unproved-claims=warn` on the command line,
the build goes ahead with a warning, and the claim is checked each time the program reaches
it ([ADR-380](../docs/specification/adr/adr-380.md)). An `assert` about `main`'s parameters
is checked where the command line is read, and one about data from outside the program is
refused in favour of a guard; [Contracts](contracts.md) shows each of these.

## Reading the reports

`nikaia check --asserts` (or `nikaia build --asserts`) prints one line per claim. The first
line counts them: *not proved* splits into *refused* (`NK1300`), *checked at run time* (under
`unproved-claims = "warn"`) and *checked by a test*; *proved* splits into claims proved where
they stand, claims proved through a loop, and preconditions proved at every call. Each claim
line names its file and line, the claim as written, and its fate: *proved*, *proved through
the loop* with the invariant it used, *precondition of* a function, or *refused* with the
reason.

`--bounds` prints the same for the checks the compiler writes: the line, the expression,
*proved* or *checked*, and for a checked one the shape that stopped it. The header says which
proved checks this build leaves out. Both reports describe what the prover showed; neither
promises that the fact named would be enough to prove a checked line.

## How a proof is trusted

The prover does not take its own word. Each proof comes with a certificate, and a check is
left out only after a small checker has accepted that certificate
([ADR-270](../docs/specification/adr/adr-270.md)). A proof the checker refuses proves nothing,
and the check stays.

## Where it stops today

The prover reads whole numbers and the shapes listed above. A claim outside them is not
false; the prover cannot say anything about it yet, so the claim is refused or its check
stays. Each `assert` in this program is true for every call `main` makes, and six of them
lie outside what the prover reads today:

```nika
struct Counter {
    n: int,
}

fn area(w: int, h: int) -> int {
    assert(w >= 0 && h >= 0)
    let a = w * h
    assert(a >= 0)                      // a product of two names
    return a
}

fn larger(a: int, b: int) -> int {
    let r = if a > b { a } else { b }
    assert(r >= a)                      // an `if` used as a value
    return r
}

fn ratio(a: f64, b: f64) -> f64 {
    assert(b > 0.0)                     // a floating-point number
    return a / b
}

fn total(xs: Vec[int]) -> int {
    let mut t = 0
    for x in xs {
        assert(x >= 0)                  // what the elements of a list hold
        t = t + x
    }
    return t
}

fn bump(mut c: Counter) -> int {
    assert(c.n >= 0)                    // a field of a value handed over `mut`
    c.n = c.n + 1
    return c.n
}

fn greeting(name: String) -> String {
    assert(name.starts_with("A"))       // what a text holds
    return f"Hello, {name}"
}

fn main() {
    let mut c = Counter { n: 0 }
    println(f"{area(4, 5)} {larger(3, 9)} {ratio(1.0, 4.0)} {total([1, 2])} {bump(c)}")
    println(greeting("Ada"))
}
```

The report, followed by one `NK1300` error per refused claim (left out here):

```text
$ nikaia check --asserts limits.nika
asserts in limits.nika: 7 - not proved 6 (refused 6, checked at run time 0, checked by a test 0); checked where the words arrive 0; proved 1 (proved 0, preconditions proved at every call 1)
  limits.nika:8  assert(a >= 0)  refused, not proved: `a` is not a whole number the prover reads
  limits.nika:14  assert(r >= a)  refused, not proved: `r` is not a whole number the prover reads
  limits.nika:19  assert(b > 0.0)  refused, not proved: `b` is not a whole number the prover reads
  limits.nika:26  assert(x >= 0)  refused, not proved: `x` is not a whole number the prover reads
  limits.nika:33  assert(c.n >= 0)  refused, not proved: `c` is not a whole number the prover reads
  limits.nika:39  assert(name.starts_with("A"))  refused, not proved: it is not a comparison of whole numbers the prover reads
  limits.nika:6  assert(w >= 0 && h >= 0)  precondition of `area`: proved at every call the compiler sees; any other reaches the entry that checks it
```

In order, the prover today does not reason about:

* the product of two names, such as `w * h`: its logic is linear arithmetic, so a product is
  a value it knows nothing about;
* the value of an `if` used as an expression; the same choice written as a `let mut` changed
  in each branch is followed (see `price` in [Contracts](contracts.md#a-loop-needs-no-invariant));
* floating-point numbers;
* what the elements of a list hold: a claim about every element would need a quantifier,
  which its logic does not have;
* a value handed over `mut`: the function may change it, and calls with it may change it, so
  its fields are not followed. A field of a parameter the function only reads is followed;
* the contents of a text, or the result of a call that returns `bool`, such as
  `is_sorted(xs)`.

Some shapes stop a proof even where every value is a whole number. A precondition is not
carried back through a function that calls itself, and a counter that counts down to zero is
not yet tied to the length it started from (the warning's notes are left out of the output):

```nika
fn percent(part: int, whole: int) -> int {
    assert(whole > 0)
    return part * 100 / whole
}

fn halved(part: int, whole: int, depth: int) -> int {
    return percent(part, whole) if depth == 0
    return halved(part, whole, depth - 1) / 2
}

fn last_first(xs: Vec[int]) -> int {
    let mut k = xs.len()
    let mut s = 0
    while k > 0 {
        k -= 1
        s = xs[k]
    }
    return s
}

fn main() {
    println(f"{halved(1, 4, 3)} {last_first([5, 6])}")
}
```

```text
$ nikaia check --asserts --bounds --unproved-claims=warn shapes.nika
asserts in shapes.nika: 1 - not proved 1 (refused 0, checked at run time 1, checked by a test 0); checked where the words arrive 0; proved 0 (proved 0, preconditions proved at every call 0)
  shapes.nika:2  assert(whole > 0)  precondition of `percent`: one call checks it when it runs; any other call proves it or reaches the entry that checks it
bounds in shapes.nika: 1 indexes, 0 proved; 3 operations, 1 proved - written without their check: indexes off, operations off
     3  part * 100  checked: a parameter no precondition bounds: an `assert` at the top of the function makes it one (ADR-269 D5)
     8  depth - 1   checked: a parameter no precondition bounds: an `assert` at the top of the function makes it one (ADR-269 D5)
    15  k -= 1      proved
    16  xs[k]       checked: a counter a loop changes, with no bound tied to the loop's turns (#1053 step 4, #432)
warning[NK1300]: `assert(whole > 0)`, the precondition of `percent`, is not proved: one call does not prove it.
...
Checked shapes.nika
```

Under `unproved-claims = "warn"` an unproved claim becomes a check that runs, and a false one
stops the program. Built as a project with `nikaia build --unproved-claims=warn`, this
`smallest` passes the sorted list and stops at the second:

```nika
fn is_sorted(xs: Vec[int]) -> bool {
    for i in 1..<xs.len() {
        return false if xs[i - 1] > xs[i]
    }
    return true
}

fn smallest(xs: Vec[int]) -> int {
    assert(xs.len() > 0)
    assert(is_sorted(xs))
    return xs[0]
}

fn main() {
    println(f"{smallest([1, 2, 3])}")
    println(f"{smallest([3, 1, 2])}")
}
```

```text
$ ./target/nikaia/bin/sortp
1
…/sortp/src/main.nika:10: the program stopped: assertion failed
    assert(is_sorted(xs))
```

## What is planned

The plans below are recorded in the specification's decision records and in open issues.
Some are decided and waiting to be built, others are still open questions; none is shown as
working on this page.

A call such as `is_sorted(xs)` is to become a named property: proved by name, carried to the
callers as a precondition, published in the package's contract, and checked once where it is
not proved ([ADR-381](../docs/specification/adr/adr-381.md), decided).

The search for loop invariants is a portfolio that grows
([ADR-361](../docs/specification/adr/adr-361.md) D5). Today it tries closed forms, intervals
and guess-and-check; octagons, k-induction and property-directed reachability are the next
strategies in the record. The same record describes `nikaia prove --propose`, by which an
outside program, a language model among them, may suggest invariants that the compiler then
checks like any other (D6); that command is not built yet. Invariants and ghost code written
by hand are not part of the plan
([#770](https://github.com/Nikaia-Foundation/Nikaia/issues/770)): finding the invariant
stays the compiler's job.

Facts about what a collection holds, such as a rule every element meets, would give the logic
its first quantifier. It is an open question, together with sortedness in `std`
([`facts-about-data.md`](../docs/facts-about-data.md)).

The checks `--bounds` keeps name the issues that would supply the missing fact:

* [#386](https://github.com/Nikaia-Foundation/Nikaia/issues/386): a product of two unknowns
* [#432](https://github.com/Nikaia-Foundation/Nikaia/issues/432): a counter, a loop or a
  length with no fact that bounds it
* [#383](https://github.com/Nikaia-Foundation/Nikaia/issues/383): the length of a list whose
  elements take space
* [#388](https://github.com/Nikaia-Foundation/Nikaia/issues/388): one list as long as another
* [#1053](https://github.com/Nikaia-Foundation/Nikaia/issues/1053): counters tied to a loop's
  turns, and the count of proved checks in the compiler's own code
* [#797](https://github.com/Nikaia-Foundation/Nikaia/issues/797): named properties
  (ADR-381)
* [#798](https://github.com/Nikaia-Foundation/Nikaia/issues/798): facts about the elements of
  a collection, and sortedness in `std`

---

## Where next

* [Contracts](contracts.md): `assert`, preconditions and postconditions, step by step.
* [A tour of the syntax](syntax.md): the rest of the language on one page.
* [The command line](command-line.md): `main`'s parameters and the checks made on them.
* [`examples/`](../examples/): complete programs that compile and run.
