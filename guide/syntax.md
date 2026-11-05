# A Tour of the Syntax

This page shows what Nikaia looks like, for readers who already program and want to see
the language before they read the specification. It covers the parts you will write every
day, one short example each.

Every snippet here compiles and runs with the current compiler, except a line marked
`does not compile`, which shows what the compiler refuses. To try one, put it in
`src/main.nika` of a project (see [Getting started](getting-started.md)) and run
`nikaia run`.

The full rules are in the specification, [Part I](../docs/specification/10-nikaia-light.md).
Each section below ends with a **Specification** link to the chapter it comes from.

---

## Variables

```nika
let x = 10          // immutable: x cannot be assigned again
let mut count = 0   // `mut` says it may change
count += x
```

Values are immutable unless declared `mut`. Types are inferred. You write one where you
want to, `let big: int = 3000000000`.

There are no semicolons at the end of lines. A comment is `//` or `/* … */`.

**Specification:** [Part I, §2.1 Variables and assignment](../docs/specification/10-nikaia-light.md#21-variables-and-assignment)

## Basic types

| Type | What it is |
| :--- | :--- |
| `int` | the integer for most numbers. In your code it works exactly like an `i64`; in memory the compiler may hold it in fewer bits where it knows the values fit |
| `i64`, `i32`, `u8`, `u64`, `u32` | integers of exactly that width, for a hash, a byte, a wire format or C. Every length and index is an `i64`, and an `int` goes where an `i64` is expected, and back, without `as` |
| `f64` | a floating-point number |
| `bool` | `true` or `false` |
| `scalar` | one Unicode scalar value, `'a'` |
| `String` | text |

An integer that overflows stops the program instead of wrapping silently. Where you mean
to wrap, you say so: `h.wrapping_mul(31)`.
Conversions are written with `as`: `(total as f64) / (count as f64)`.

**Specification:** [Part I, §2.2 Primitive data types](../docs/specification/10-nikaia-light.md#22-primitive-data-types)

## Text and interpolation

```nika
let name = "Nika"
println(f"hello, {name}, your name has {name.len()} letters")
println("braces in a plain string are just braces: {name}")
```

`f"…"` interpolates. A plain `"…"` never does, so JSON and CSS can be written as they
are.

You never choose between a string type and a string-slice type. There is one `String`,
and whether a value is text of its own or a view into text that already exists is the compiler's
decision.

**Specification:** [Part I, §2.5 Strings, plain and interpolated](../docs/specification/10-nikaia-light.md#25-strings-plain-and-interpolated)

## `if`, `match` and blocks are expressions

```nika
let count = 7
let status = if count > 5 { "big" } else { "small" }
```

`if`, `match` and a `{ … }` block each give the value of their last line. You can assign it
to a variable or use it in a calculation, which is why there is no ternary operator.

**Specification:** [Part I, §3.1 Expressions and blocks](../docs/specification/10-nikaia-light.md#31-expressions-and-blocks) · [§3.2 if / else](../docs/specification/10-nikaia-light.md#32-conditional-logic-if--else)

## Loops

```nika
for i in 0..<3 {         // 0, 1, 2: `..<` stops before the end
    println(f"step {i}")
}

let mut n = 0
while n < 5 {
    n += 1
}
```

`a..b` includes its end and `a..<b` excludes it. There is `for`, `while`, `break` and
`continue`. An endless loop is `while true { … }`.

A `for` reads the elements of a list and leaves the list as it was. `for mut` changes each
element in the list itself:

```nika
let mut scores: Vec[int] = [70, 85, 92]
for mut s in scores {
    s += 5
}
println(scores.iter().join(", "))   // 75, 90, 97
```

### Changing a list while a loop reads it

A `for` loop lends its list to the loop body, so the body cannot add elements to that list or
remove them. Such a change could move the elements the loop is still reading, which in many
languages is a crash or a silent bug at run time. Nikaia refuses it when it compiles:

```nika
fn main() {
    let mut users: Vec[String] = ["ada", "bob", "ada"]
    for u in users {
        if u == "bob" {
            users.push("x")         // does not compile: NK2305
        }
    }
}
```

```text
error[NK2305]: `users` is changed here, inside the loop that lends it.
  --> main.nika:5:13
   |
 3 |     for u in users {
   |              ----- `users` is lent to this loop
   ...
 5 |             users.push("x")         // does not compile: NK2305
   |             ^^^^^ `users` changes here
   |
   = help: Collect what the loop wants to add in a list of its own and add it after the loop with `users.extend(more)`, or walk `0..<n` with `let n = users.len()` before the loop.
1 change to data a view still to be used may point into
```

What the loop wants to add goes into a list of its own, and that list is added once the loop
has finished:

```nika
fn main() {
    let mut users: Vec[String] = ["ada", "bob", "ada"]
    let mut more: Vec[String] = []
    for u in users {
        if u == "bob" {
            more.push("x")
        }
    }
    users.extend(more)
    println(users.iter().join(", "))
}
```

```text
ada, bob, ada, x
```

For removing elements the standard library has `retain`, which keeps those the function
accepts. The message names such a method (`retain`, `drain`, `entry`, `swap`) where one fits:

```nika
fn main() {
    let mut users: Vec[String] = ["ada", "bob", "ada"]
    users.retain fn(u) { u != "bob" }
    println(users.iter().join(", "))
}
```

```text
ada, ada
```

`for x in xs.drain()` takes the elements out of the list instead of lending them.

**Specification:** [Part I, §3.3 Loops](../docs/specification/10-nikaia-light.md#33-loops) · [§6.8 When the compiler says no](../docs/specification/10-nikaia-light.md#68-when-the-compiler-says-no)

## Functions

```nika
fn greet(name: String; greeting: String = "Hello") -> String {
    return f"{greeting}, {name}!"
}

println(greet("Nika"))                       // Hello, Nika!
println(greet("Nika"; greeting: "Hallo"))    // Hallo, Nika!
```

The semicolon divides the parameter list. Parameters before it are the data the function
works on and are passed by position. Parameters after it are options: they always have a
default, and they are always passed by name. On `main` the same split is the command line: positional words and `--name`
options ([The command line](command-line.md)).

A `String` parameter needs nothing more: whether the function gets a view of the caller's
text or text of its own is the compiler's decision. `ref String` is written only in an
`extern "C"` declaration; elsewhere it is a warning.

**Specification:** [Part I, §5.1 The subject ; config protocol](../docs/specification/10-nikaia-light.md#51-the-subject--config-protocol)

## Generic functions

```nika
fn pair_of[T](a: T, b: T) -> Vec[T] {
    return [a, b]
}

fn empty[T]() -> Vec[T] {
    return []
}

println(pair_of(1, 2).len())        // 2: `T` is `int` here
let names = empty[String]()         // nothing says what `T` is: name it at the call
println(names.len())                // 0
```

A type parameter is written in square brackets, `[T]`, where Java and TypeScript write
`<T>`. The compiler fills it from the arguments; where no argument says what it is, the call
names it, `empty[String]()`.

**Specification:** [Part I, §4.6 Generics](../docs/specification/10-nikaia-light.md#46-generics-type-parameters)

## Structs and methods

```nika
struct Point {
    x: int,
    y: int,
}

impl Point {
    fn length_squared(self) -> int {
        return self.x * self.x + self.y * self.y
    }
}

let p = Point { x: 3, y: 4 }
println(f"{p.length_squared()}")    // 25
let moved = p with { x: 5 }         // a copy with one field changed
```

A `struct` holds the data, and an `impl` block adds behaviour to it. Behaviour that
several types share is a `trait`.

Everything is private unless it says `pub`.

**Specification:** [Part I, §4.1 Structs](../docs/specification/10-nikaia-light.md#41-structs-custom-data-types) · [§4.2 Constructors](../docs/specification/10-nikaia-light.md#42-constructors-and-instantiation) · [§4.3 Why no classes?](../docs/specification/10-nikaia-light.md#43-why-no-classes-data-vs-behavior) · [§4.7 Traits](../docs/specification/10-nikaia-light.md#47-traits-defining-behavior)

## Enums and `match`

```nika
enum Shape {
    Circle(f64),
    Rectangle { width: f64, height: f64 },
}

fn area(shape: Shape) -> f64 {
    match shape {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle { width, height } => width * height,
    }
}
```

An `enum` is a value that is one of a fixed set of shapes, and each shape may carry
data, like TypeScript's discriminated unions or Kotlin's sealed classes.

`match` must cover every case. Leave out `Shape::Rectangle` and the program does not
compile. When a match is not over an enum, `else =>` catches the rest.

**Specification:** [Part I, §3.4 Pattern matching](../docs/specification/10-nikaia-light.md#34-pattern-matching-match) · [§4.4 Enums](../docs/specification/10-nikaia-light.md#44-enums-algebraic-data-types)

## Null safety

```nika
fn find_nickname(name: String) -> String? {
    if name == "Nika" {
        return "Nik"
    }
    return null
}

let nick = find_nickname("Ada") ?? "no nickname"
println(nick)                    // no nickname
println(find_nickname("Rust"))   // does not compile: NK1102, it may be null
```

A `String` is always a string. Only a `String?` may be `null`, and the compiler makes you
handle it: `??` supplies a fallback, and `?.` reaches into a value only where it exists.

**Specification:** [Part I, §2.3 Nullable types](../docs/specification/10-nikaia-light.md#23-nullable-types-null-safety) · [§3.5 Null safety operators](../docs/specification/10-nikaia-light.md#35-null-safety-operators)

## Lists, maps and lambdas

```nika
use std::collections

let numbers = [1, 2, 3, 4, 5]
let doubled = numbers.map fn(n) { n * 2 }

let mut ages = collections::HashMap()
ages["Nika"] = 7
let age = ages["Nika"] ?? 0      // a key may be missing, so reading one gives a `T?`
```

A lambda is always written `fn(arguments) { body }`. When it is the last argument it
stands after the call, as with `map` above. There is no second short arrow form.

**Specification:** [Part I, §4.5 Collections](../docs/specification/10-nikaia-light.md#45-collections) · [§5.2 One lambda form](../docs/specification/10-nikaia-light.md#52-there-is-one-lambda-form) · [§5.3 Lambdas](../docs/specification/10-nikaia-light.md#53-lambdas-fn---)

## Errors

```nika
enum AgeError {
    Empty,
    NotANumber(String),
}

impl Error for AgeError {
    fn message(self) -> String {
        match self {
            AgeError::Empty => "the input is empty",
            AgeError::NotANumber(text) => f"not a number: {text}",
        }
    }
}

fn parse_age(text: String) -> int throws {
    if text.len() == 0 {
        throw AgeError::Empty
    }
    // …
    return 42
}

fn main() {
    let age = parse_age("") catch {
        println(f"{error}")     // inside `catch`, the failure is called `error`
        0                       // …and the block supplies a replacement value
    }
}
```

A function that can fail says `throws`. What it can fail *with* is worked out by the
compiler. If a function calls something that can fail and neither declares `throws` nor
handles the failure with `catch`, it does not compile, so no failure is silently dropped.
There is no `try`, `?` or `if err != nil`: a failure the function does not catch goes up
to its caller.

**Specification:** [Part I, §7.1 Recoverable errors](../docs/specification/10-nikaia-light.md#71-recoverable-errors-throws)

## Waiting and doing things at the same time

```nika
fn count_words(text: String) -> int {
    return text.split(" ").count()
}

fn add(a: int, b: int) -> int {
    return a + b
}

fn main() {
    let first = "the quick brown fox"
    let second = "jumps over the lazy dog"

    let a = async count_words(first)     // starts at once
    let b = async count_words(second)    // runs beside the first
    println("both counts have started")
    println(f"{a + b} words")            // waits for both here

    let total = add(async count_words(first), async count_words(second))
    println(f"{total} words again")
}
```

```text
both counts have started
9 words
9 words again
```

A function that waits is written like any other function, and a call to it like any other
call, because the compiler finds which functions can pause. Reading a file pauses the
function without blocking the program, and a function that starts to wait leaves its
callers as they are.

Lines run in the order they are written. To run work beside the code after it, you write
`async` before an expression. `async e` starts `e` at once, and the program waits for its
value where the value is first needed: above, both counts run while the first `println`
does its work, and `a + b` waits for both. As an argument, `add(async count_words(first),
async count_words(second))` runs both calls beside each other and calls `add` when both are
done. The function that receives the values gets plain `int`s, so neither its signature nor
its callers change.

Written as a statement, `async e` starts a task without a value, and `async { … }` is a
group: its own lines run in order beside the tasks started in it, and the group ends only
when every one of its tasks has ended, also when the function leaves it early with a
`return`. A task started outside any `async { … }` belongs to the block it is written in,
so it has ended before that block does.

A task's error is thrown where the program waits for the task, and the compiler asks for it
to be handled there, as for any call that can fail:

```nika
struct TooLong { words: int }
impl Error for TooLong {
    fn message(self) -> String {
        return f"{self.words} words is too long"
    }
}

fn short_count(text: String) -> int throws {
    let n = text.split(" ").count()
    if n > 4 {
        throw TooLong { words: n }
    }
    return n
}

fn total(first: String, second: String) -> int throws {
    let a = async short_count(first)
    let b = async short_count(second)
    println("both counts have started")
    return a + b                         // the second count's error is thrown here
}

fn main() {
    let n = total("the quick brown fox", "jumps over the lazy dog") catch {
        println(f"caught: {error}")
        0
    }
    println(f"{n} words")
}
```

```text
both counts have started
caught: 5 words is too long
0 words
```

A failure that nobody reads, such as one of a task started as a statement, is thrown at the
end of its group. Older programs use `overlap { … }` and `spawn`, which the compiler still
accepts; `async` replaces both.

Some of the design is specified and still being built
([#988](https://github.com/Nikaia-Foundation/Nikaia/issues/988),
[#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983)). A task whose call pauses,
such as reading a file, is not yet translated. `async(handle)` is to give a handle with
`join` and `cancel`, `async(deferred)` a handle whose task starts only when asked to, and
`async(detached)` a task that may outlive its function. The options `limit:`, `within:` and
`admission:` are to bound how many tasks run at once, how long they may take and when they
start, and work over every element of a list is to be `xs.par_iter()`, whose lambdas may
pause. With `user_parallelism: yes` the tasks of a group still run one after another for
now.

### What the compiler checks about tasks

The compiler checks that a task has something to run beside: an `async` whose value is
needed in the very next line is refused, and the message suggests removing the word. It
refuses a jump such as `return` out of a task, and a group that starts no task. The
specification also refuses two tasks that would change the same value at once, or both
write to standard output, and names what they share; for `async` this check is still being
built ([#988](https://github.com/Nikaia-Foundation/Nikaia/issues/988)). Where the compiler
cannot tell what a call touches, it does not refuse. A task's error reaches the place where
the program waits, and the compiler asks for it to be handled there. Each task is to get a
memory budget of its own, and a task that uses it up ends with `task::OverBudget` while the
rest of the program runs on; this is specified and not built yet, and memory a Rust crate
or a C library allocates for itself is not counted.

**Specification:** [Part I, Chapter 8 Concurrency](../docs/specification/10-nikaia-light.md#chapter-8-concurrency-doing-things-at-the-same-time)

## Data that more than one task changes

```nika
fn main() {
    let counter = SharedMut(0)
    async {
        async counter.update fn(mut v) { v += 1 }
        async counter.update fn(mut v) { v += 2 }
    }                                         // both tasks have ended here
    println(f"counter is {counter.get()}")    // counter is 3
}
```

A value that several tasks change is put into a `SharedMut`, and it is changed only
through its methods (`get`, `set`, `update`, `access`). The lock is part of the value, so
you cannot forget to take it. The compiler also refuses code that would wait for I/O while
holding it.

Whether this runs on one core or on all of them is not written here at all. It is one line
in `nikaia.toml` ([Getting started](getting-started.md#the-two-build-settings)), and
the source stays the same.

**Specification:** [Part I, §6.2 Unified types](../docs/specification/10-nikaia-light.md#62-unified-types) · [§6.3 The four doors](../docs/specification/10-nikaia-light.md#63-changing-shared-data-the-four-doors) · [Part II, Chapter 12 Thread safety](../docs/specification/20-nikaia-advance.md#chapter-12-thread-safety-and-synchronization)

## Memory: lending and keeping

Nikaia has no garbage collector, and you do not free memory by hand. A value is cleaned up
at the end of the block that owns it, and a file or a socket is closed there too. What makes
this work is that a value has one owner unless you write otherwise, and the owner can lend
the value or hand it over. The compiler checks how each value is used and tells you, before
the program runs, where a use does not fit. The rest of this section shows how that looks
in a program, and then where the model ends.

```nika
struct Person { name: String, age: int }

fn greet(p: Person) {               // only reads `p`: it is lent
    println(f"Hello, {p.name}")
}

fn main() {
    let ada = Person { name: "Ada", age: 36 }
    greet(ada)                      // Hello, Ada
    greet(ada)                      // Hello, Ada: still ours

    let mut team: Vec[Person] = []
    team.push(ada.clone())          // the list keeps what it gets: give it a copy
    println(ada.name)               // Ada

    let age = ada.age               // a number is copied
    println(age + ada.age)          // 72

    team.push(ada)                  // the last use: no copy needed
    println(team.len())             // 2
    println(ada.name)               // does not compile: NK2105, `ada` is in the list now

    for p in team {                 // a `for` lends the list
        println(p.age)              // 36, twice
    }
    println(team.len())             // 2
}
```

A function that only reads a parameter is lent the value, and the caller still has it after
the call: `greet(ada)` can be written twice. A function that changes a parameter says `mut`
before it, and the caller's value is changed in place. A function that keeps a parameter
(stores it, returns it, gives it to a task) takes it over, and so does a list you push into;
after that the old name is gone, and `.clone()` is how both places get a value. Numbers and
other small values are copied wherever they are used. You do not write which of these a
parameter is: the compiler reads it off the body and records it in `nikaia.contracts`, a file
you commit like a lock file.

A loop that changes the list it walks, and a change to text while a view of it is still used,
are refused before the program runs, and the message names a way that fits. Data that several
owners need at once is written `Shared[T]` or `SharedMut[T]`
([Data that more than one task changes](#data-that-more-than-one-task-changes)).

[Ownership](ownership.md) teaches these rules from small programs up: views and the data they
point into, the ledger, the common errors with their fixes, and where the model ends.

**Specification:** [Part I, Chapter 6 Memory and ownership](../docs/specification/10-nikaia-light.md#chapter-6-memory-and-ownership) · [§6.5 References and borrowing](../docs/specification/10-nikaia-light.md#65-references-and-borrowing) · [§6.7 The borrow contract ledger](../docs/specification/10-nikaia-light.md#67-the-borrow-contract-ledger)

## Modules

```nika
// file: src/stock.nika
pub struct Item { pub name: String, pub count: int }
```

```nika
// file: src/main.nika
use std::fs            // a standard-library module: every name from it keeps its prefix

fn main() {
    let item = Item { name: "apples", count: 3 }   // same package: no `use` at all
    println(f"{item.count} {item.name}")
}
```

A package is a directory, and the `.nika` files in it see each other. Another
package is reached by its name, and what comes from it is always written with its prefix,
`fs::read_to_string(…)`. Nothing is imported unprefixed, so you always see where a name
comes from. `pub` makes something visible outside its package.

**Specification:** [Part I, Chapter 9 Project organization](../docs/specification/10-nikaia-light.md#chapter-9-project-organization-and-visibility)

## Grammars

A program can describe a text format as a grammar, and the compiler builds a parser for
it that hands back values of the program's own types:

```nika
grammar Pairs {
    entry rule pair -> (String, i64) = key:NAME "=" value:NUMBER -> { (key, value) }
    rule NAME -> String = s:alpha1 -> { s }
    rule NUMBER -> i64 = n:dec[i64](digit+) -> { n }
}

fn main() throws {
    let (key, value) = Pairs::pair("width = 80")
    println(f"{key} is {value}")
}
```

Input the grammar does not accept makes the call fail, and the program handles that
with `catch` like any other failure. [Grammars and parsers](grammars.md) teaches
grammars in full, from this example up to a complete calculator.

**Specification:** [Part II, Chapter 10 Metaprogramming](../docs/specification/20-nikaia-advance.md#chapter-10-metaprogramming-code-that-writes-code)

---

## Where next

* [Getting started](getting-started.md): install the compiler and run a project.
* [Ownership](ownership.md): the rules of lending, keeping and views, taught in full.
* [Contracts](contracts.md): what the compiler proves about an `assert`.
* [The prover](prover.md): what the compiler proves today, where it stops, and what is planned.
* [Grammars and parsers](grammars.md): describing a text format and parsing it.
* [`examples/`](../examples/): complete programs that compile and run.
* [Specification, Part I](../docs/specification/10-nikaia-light.md): the rules, one
  chapter at a time.
