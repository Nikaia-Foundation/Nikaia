# Ownership: Who Owns a Value, and Who May Use It

Nikaia cleans up memory, files and sockets without a garbage collector and without a call to
free them. It can do that because every value has one owner, and the compiler checks, before
the program runs, how each value is used: who reads it, who changes it, and who keeps it.
Programs that would read freed memory, change a list while a loop walks it, or use a value
after handing it to someone else are refused at that point, and the message names a way that
fits.

This chapter teaches those rules from small programs up, as the compiler checks them today.
It is for readers who have read [the syntax tour](syntax.md). Every program here compiles and
runs with the current compiler, except a line marked `does not compile`, which shows what the
compiler refuses; the error below such a program is the one the compiler prints. To try one,
save it as a file and run `nikaia run file.nika`.

The last section, [A signature that says what it keeps](#a-signature-that-says-what-it-keeps),
shows how a signature writes `keep` and `from`, and what of that is still being built.

---

## One owner, cleaned up at the end of its block

A value belongs to the variable that holds it. When the block that declared the variable
ends, at its closing `}`, the value is cleaned up: its memory is released, and a type that
holds a resource gets its `drop` method called, so a file is closed exactly there.

```nika
struct File { name: String }

impl Drop for File {
    fn drop(mut self) {
        println(f"closing {self.name}")
    }
}

fn main() {
    let log = File { name: "log.txt" }
    if log.name != "" {
        let tmp = File { name: "tmp.txt" }
        println(f"using {tmp.name}")
    }
    println(f"still using {log.name}")
}
```

```text
using tmp.txt
closing tmp.txt
still using log.txt
closing log.txt
```

`tmp` lives in the inner block and is closed when that block ends; `log` lives until the end
of `main`. The order is the reverse of the order of creation, and it can be read off the
source.

**Specification:** [Part I, §6.1 The concept of scope](../docs/specification/10-nikaia-light.md#61-the-concept-of-scope) · [§6.4 Resource cleanup](../docs/specification/10-nikaia-light.md#64-resource-cleanup-raii)

## Lending a value

Most functions only read what they are given. Such a function is lent the value: the caller
keeps it, and can use it again after the call. Nothing is written for this, neither at the
call nor in the declaration.

```nika
struct Person { name: String, age: int }

fn greet(p: Person) {
    println(f"Hello, {p.name}")
}

fn main() {
    let ada = Person { name: "Ada", age: 36 }
    greet(ada)
    greet(ada)
    println(ada.age)
}
```

```text
Hello, Ada
Hello, Ada
36
```

The call does not say how `ada` is passed: the function decides, and the compiler writes
the reference. A `ref` written at the call is refused as unnecessary
([`NK1137`](#nk1137-a-ref-written-at-a-call)).

**Specification:** [Part I, §6.5 References and borrowing](../docs/specification/10-nikaia-light.md#65-references-and-borrowing)

## Handing a value over

A function that keeps what it is given, here by putting it into a list, takes the value over.
After the call the value belongs to the list, and the caller's name for it is gone:

```nika
struct Person { name: String, age: int }

fn enrol(mut team: Vec[Person], keep p: Person) {
    team.push(p)
}

fn main() {
    let ada = Person { name: "Ada", age: 36 }
    let mut team: Vec[Person] = []
    enrol(team, ada)
    println(team.len())
    println(ada.name)               // does not compile: NK2105
}
```

```text
error[NK2105]: You're using `ada.name`, but `ada` was passed to `enrol`, which keeps it.
   --> main.nika:12:5
    |
 12 |     println(ada.name)               // does not compile: NK2105
    |     ^
    |
    = note: `ada` is a `Person`, so it's handed over itself, not a copy. Nikaia never copies behind your back, because a copy costs as much as the value is large.
    = note: Numbers, `bool`s, `scalar`s and views would be fine: they're copied.
    = help: If both need it, hand over a copy: `ada.clone()`.
```

If both places need the value, the program says so with `.clone()`, and the list gets a copy
of its own. On the last use, no copy is needed:

```nika
struct Person { name: String, age: int }

fn enrol(mut team: Vec[Person], keep p: Person) {
    team.push(p)
}

fn main() {
    let ada = Person { name: "Ada", age: 36 }
    let mut team: Vec[Person] = []
    enrol(team, ada.clone())        // a copy: `ada` is used again below
    enrol(team, ada)                // the last use: handed over as it is
    println(team.len())
}
```

```text
2
```

Every copy of a large value is visible in the source in this way, because the compiler does
not insert one. Numbers, `bool`s, `scalar`s and enums whose variants hold nothing are small
and copied wherever they are used, so there is nothing to hand over: `let age = ada.age`
leaves `ada` whole.

The same rule holds for a list you push into, a field you assign, and a `let` that gives the
value a second name: each takes the value over.

**Specification:** [Part I, §6.5 References and borrowing](../docs/specification/10-nikaia-light.md#65-references-and-borrowing)

## Changing a value: `mut`

A function that changes a parameter says `mut` before it, and the caller's value is changed
in place. The call shows nothing extra, just as `xs.push(1)` shows nothing extra:

```nika
fn add_guest(mut names: Vec[String]) {
    names.push("Grace")
}

fn main() {
    let mut names: Vec[String] = ["Ada"]
    add_guest(names)
    println(names.len())
}
```

```text
2
```

Without `mut`, the change is refused where it happens, and the message offers both ways: say
`mut`, or change a copy of your own ([`NK1138`](#nk1138-a-parameter-changed-without-mut)). A
variable is changed only if its `let` says `mut` too.

A loop that changes each element of a list writes `for mut`. Each turn changes the element in
the list itself, so nothing has to be written back:

```nika
struct Person { name: String, age: int }

fn birthday(mut people: Vec[Person]) {
    for mut p in people {
        p.age += 1
    }
}

fn main() {
    let mut people: Vec[Person] = [Person { name: "Ada", age: 36 }, Person { name: "Alan", age: 41 }]
    birthday(people)
    for p in people {
        println(f"{p.name} is {p.age}")
    }
}
```

```text
Ada is 37
Alan is 42
```

**Specification:** [Part I, §6.5 References and borrowing](../docs/specification/10-nikaia-light.md#65-references-and-borrowing)

## Keeping a value beyond the call

A function keeps a parameter when the value outlives the call: the body stores it somewhere
that lives on (a list or a field it was handed `mut`), returns it, gives it to a task, or
passes it on to another function that keeps it. Such a parameter says `keep`, as `enrol`'s
`p` does above; the compiler reads off the body what a function keeps, checks the signature
against it, and records it (see [The contract ledger](#the-contract-ledger)).

A task shows where the line runs. A task started with `async` belongs to a group and ends
before the group does, so it can borrow what was bound outside the group, and the code after
it still has the value:

```nika
fn words(text: String) -> int {
    return text.split(" ").count()
}

fn letters(text: String) -> int {
    return text.len()
}

fn main() {
    let report = "nightly report".to_uppercase()
    let w = async words(report)      // the task reads `report`
    let n = letters(report)          // runs beside the task
    println(f"{w} words, {n} letters")
    println(report)                  // still here: the task only borrowed it
}
```

```text
2 words, 14 letters
NIGHTLY REPORT
```

A task that may run longer than the function that started it works the other way: it takes
the variables it uses with it, and a later use of such a variable is refused
([`NK2101`](#nk2101-a-value-used-after-a-task-took-it)). In the specification that task is
started with `async(detached)`, whose handle rules are still being built
([#988](https://github.com/Nikaia-Foundation/Nikaia/issues/988)).

Building a value out of a parameter and handing it back is different: the result points into
the caller's argument, and the caller still has it.

```nika
struct Badge { owner: String }

fn badge_for(name: String) -> Badge {
    return Badge { owner: name }
}

fn main() {
    let name = "grace".to_uppercase()
    let b = badge_for(name)
    println(b.owner)
    println(name)
}
```

```text
GRACE
GRACE
```

No text is copied here: `b.owner` is a view of `name`'s text. The next section is about such
views.

**Specification:** [Part I, §6.5 References and borrowing](../docs/specification/10-nikaia-light.md#65-references-and-borrowing) · [§8.3 Data ownership in free tasks](../docs/specification/10-nikaia-light.md#83-data-ownership-in-free-tasks-implicit-move)

## Views into a value

A view looks at data that something else owns, without copying it. Many views appear without
being written. A `for` loop walks views of the list's elements, so the list is still there
after the loop. A `let` over a field or an element, `let name = config.name`, is a view of
it, and the struct stays whole:

```nika
struct Config { name: String, retries: int }

fn main() {
    let config = Config { name: "server", retries: 3 }
    let name = config.name
    println(f"{name} retries {config.retries} times")
    println(config.name)
}
```

```text
server retries 3 times
server
```

A function can hand back a view of one of its arguments. Here `longest` hands back one of the
two texts it is given, without copying either:

```nika
fn longest(a: String, b: String) -> String {
    if a.len() >= b.len() {
        return a
    }
    return b
}

fn main() {
    let x = "a long line".to_uppercase()
    let mut y = "short".to_uppercase()
    let r = longest(x, y)
    println(r)
    y = "replaced".to_uppercase()
    println(r)
    println(y)
}
```

```text
A LONG LINE
A LONG LINE
REPLACED
```

### A view and the data it points into

A view stays usable as long as the data it points into does, and the compiler holds that data
for it. Giving the data up while a view of it is in use, by assigning the variable a new
value as above, or by leaving the block, is allowed: the old text stays alive until the last
view of it is gone, and nothing is copied.

What is refused is changing that data in place while a view of it is still to be used,
because the change could move the text the view points at:

```nika
fn longest(a: String, b: String) -> String {
    if a.len() >= b.len() {
        return a
    }
    return b
}

fn main() {
    let x = "a long line".to_uppercase()
    let mut y = "short".to_uppercase()
    let r = longest(x, y)
    y.push_str("ER")                // does not compile: NK2305
    println(r)
}
```

```text
error[NK2305]: `y` is changed here, but `r` may point into its data and is used later.
   --> main.nika:12:5
    |
 11 |     let r = longest(x, y)
    |         - `r` is made here by `longest`
 12 |     y.push_str("ER")                // does not compile: NK2305
    |     ^ `y` changes here
 13 |     println(r)
    |             - `r` is used here, after the change
    |
    = note: `longest` may hand back `b`: `return b`.
    = help: Use `r` before this change, or take a copy of it with `r.clone()` before the change.
1 change to data a view still to be used may point into
```

The fix is one of the two the message names: print `r` before `y` changes, or keep a copy,
`let r = longest(x, y).clone()`.

A view can also leave the function whose local it points into. Then the data goes with it,
into the keeping of the caller, instead of being freed at the end of the function:

```nika
fn words_of(path: String) -> Vec[String] {
    let text = f"one two three from {path}"
    let words: Vec[String] = text.split(" ").collect()
    return words
}

fn main() {
    let ws = words_of("notes.txt")
    println(ws.len())
    println(ws[1])
}
```

```text
5
two
```

`text` is not copied word by word. The list holds views into it, and `text` itself lives on
in `main` until `ws` is gone. This is why a view does not outlive the data it points into:
where a view leaves, its data goes along. `nikaia lower file.nika --tethers` prints where each
such piece of data lives and why:

```text
words_of (keeps):
    `text` lives in the caller's keep, because views of it leave this function
main (keeps):
    `text` lives in `main`'s keep: handed back by `words_of`
```

A view handed to a function that keeps its parameter is refused, because a kept value needs
an owner of its own ([`NK1102`](#nk1102-a-view-handed-to-a-function-that-keeps-it)).

**Specification:** [Part I, §6.5 References and borrowing](../docs/specification/10-nikaia-light.md#65-references-and-borrowing) · [§6.6 Escaping references are tethered](../docs/specification/10-nikaia-light.md#66-escaping-references-are-tethered)

## The contract ledger

The compiler reads off the body which parameters a function keeps and what its result points
into, and writes both, for every function, into a file beside your
code, `nikaia.contracts`. You commit it like a lock file. For the functions on this page it
reads:

```text
[fn."enrol"]
sync = "inferred"
keeps = ["p"]
touches = []
signature = "(mut team: Vec[Person], p: Person)"

[fn."greet"]
sync = "inferred"
locks = true
touches = ["stdout write"]
signature = "(p: Person)"

[fn."longest"]
sync = "inferred"
touches = []
views = ["a: borrowed", "b: borrowed", "<result>: borrowed"]
signature = "(a: ref String, b: ref String) -> ref(a | b) String"
```

`enrol` keeps `p`; `greet` keeps nothing; `longest` is lent both texts, and its result points
into `a` or `b`. When an edit to a body changes one of these lines, the change shows up in
review as a diff of this file, and a caller that breaks because of it gets an error that
names the edit. The file also records whether a function may pause (`sync`) and what it
touches, which is what the checks on tasks read
([the syntax tour](syntax.md#what-the-compiler-checks-about-tasks)).

**Specification:** [Part I, §6.7 The borrow contract ledger](../docs/specification/10-nikaia-light.md#67-the-borrow-contract-ledger) · [Part III, §13.5 The ledger](../docs/specification/30-nikaia-tooling.md#135-the-borrow-contract-ledger-nikaiacontracts)

## Common errors and how to fix them

Each refusal explains itself and names a way out. These are the ones met most often.

### NK2105: a value used after it was handed over

The value went to a function, a list or a field that keeps it, and the program uses the old
name again ([Handing a value over](#handing-a-value-over)). Hand over a copy with `.clone()`
where both need it, or move the use before the hand-over.

### NK2101: a value used after a task took it

A task took the variable with it ([Keeping a value beyond the call](#keeping-a-value-beyond-the-call)).
Make a copy before the task starts and let the task use the copy, or share the value as a
`Shared` or `SharedMut` handle ([Where the model ends](#where-the-model-ends)).

### NK1102: a view handed to a function that keeps it

```nika
struct Person { name: String, age: int }

fn enrol(mut team: Vec[Person], keep p: Person) {
    team.push(p)
}

fn main() {
    let people: Vec[Person] = [Person { name: "Ada", age: 36 }]
    let mut team: Vec[Person] = []
    for p in people {
        enrol(team, p)              // does not compile: NK1102
    }
    println(team.len())
}
```

```text
error[NK1102]: `enrol` expects `p` to be `Person`, but you're passing `ref Person`.
   --> main.nika:11:21
    |
 11 |         enrol(team, p)              // does not compile: NK1102
    |                     ^
    |
    = note: It's declared as `enrol(mut team: Vec[Person], p: Person)`.
    = note: This is a view: it points into a `Person` that something else owns.
    = note: `enrol` keeps its `p` after the call returns, so it needs one of its own.
    = help: Write `.clone()` to hand `enrol` a copy of its own.
1 type error
```

`p` is a view of an element of `people`, and `enrol` keeps what it gets. When `people` is not
needed after the loop, `people.drain()` hands its elements over one by one instead of lending
them. `enrol` then keeps each `Person` itself, and nothing is copied:

```nika
struct Person { name: String, age: int }

fn enrol(mut team: Vec[Person], keep p: Person) {
    team.push(p)
}

fn main() {
    let people: Vec[Person] = [Person { name: "Ada", age: 36 }, Person { name: "Bob", age: 41 }]
    let mut team: Vec[Person] = []
    for p in people.drain() {
        enrol(team, p)
    }
    println(team.len())
}
```

```text
2
```

`people` is handed over with its elements, so using it after the loop is refused
([`NK2105`](#nk2105-a-value-used-after-it-was-handed-over)). Where `people` is still needed,
`enrol(team, p.clone())` hands `enrol` a copy of each element instead.

### NK1138: a parameter changed without `mut`

```nika
fn add_guest(names: Vec[String]) {
    names.push("Grace")             // does not compile: NK1138
}
```

```text
error[NK1138]: You're changing the parameter `names`, but it wasn't declared as mutable.
  --> main.nika:2:5
   |
 1 | fn add_guest(names: Vec[String]) {
   |              ----- declared here without `mut`
 2 |     names.push("Grace")             // does not compile: NK1138
   |     ^^^^^ changed here, by `push`
   |
   = note: A parameter declared `mut names` is changed in place, so the caller sees the change too.
   = help: Write `mut names` in the parameter list. If the caller's value should stay as it was, change a copy instead: `let mut names_own = names`.
1 type error
```

The same for a variable is `NK1139`, which asks for `let mut`.

### NK2305: data changed while a view of it is still used

Either a list changed inside a loop that walks it
([Changing a list while a loop reads it](syntax.md#changing-a-list-while-a-loop-reads-it)),
or a value changed while a view that may point into it is used later ([A view and the data it points into](#a-view-and-the-data-it-points-into)).
Use the view before the change, copy it with `.clone()` before the change, or, in a loop, use
the method the message names.

### NK1137: a `ref` written at a call

```text
error[NK1137]: You don't need the `ref` here: the compiler lends where it's needed.
  --> main.nika:9:5
   |
 9 |     greet(ref ada)
   |     ^^^^^^^^^^^^^^
   |
   = note: `greet` only reads this argument, so it's lent already. Whether an argument is lent or handed over is decided by the function, not the call.
   = help: Remove the `ref`.
1 type error
```

Remove the `ref`; the function's declaration decides whether an argument is lent.

**Specification:** [Part I, §6.8 When the compiler says no](../docs/specification/10-nikaia-light.md#68-when-the-compiler-says-no) · [Part III, C.3 Error code catalogue](../docs/specification/30-nikaia-tooling.md#c3-error-code-catalogue-nk-codes)

## Where the model ends

These checks are made before the program runs. They have limits that are worth knowing.

Data that more than one owner needs at once, such as settings many parts of a program read or
a counter several tasks change, does not fit one owner per value. It is written as
`Shared[T]` (read by all) or `SharedMut[T]` (changed by all, through `get`, `set`, `update`
and `access`), as [the syntax tour shows](syntax.md#data-that-more-than-one-task-changes). The
compiler keeps an owner count for such a value, and it is cleaned up when the last owner is
gone. Handing one handle to a second owner is meant to share the data; that is specified but
not built yet, and a second use of the same handle still fails today
([#1212](https://github.com/Nikaia-Foundation/Nikaia/issues/1212)). An owner count does not
free a cycle of handles that point at each other, and there is no weak handle yet, so such a
cycle stays in memory until the program ends; a graph or a tree whose nodes know their parent
is better written with indices into a list.

A check that runs before the program does refuses some programs that would have worked. A loop
that pushes to the list it walks is one: written with an index over a length taken before the
loop, it is fine, and the message says so.

What a result points into is read off the body unless the signature writes it with `from`, so
a change inside a function can change what its callers may do; the ledger shows such a change
in review and the compiler names it
([A signature that says what it keeps](#a-signature-that-says-what-it-keeps)).

In an `extern "C"` declaration, `ref i32` and `ref mut i32` are the pointers C expects, and
what the C library does with them is outside these checks: its code is trusted as it is. The
same holds for a Rust crate a program uses; Rust's compiler checks it with the limits Rust has,
and its `unsafe` code is trusted.

**Specification:** [Part I, §6.2 Unified types](../docs/specification/10-nikaia-light.md#62-unified-types) · [Part II, Chapter 12 Thread safety](../docs/specification/20-nikaia-advance.md#chapter-12-thread-safety-and-synchronization) · [Part III, §15.1 C interoperability](../docs/specification/30-nikaia-tooling.md#151-c-interoperability)

## A signature that says what it keeps

A parameter whose function keeps the argument (stores it, returns it, gives it to a task, or
passes it on to a function that keeps it) says `keep`. A plain parameter is lent, as most are,
and a `mut` parameter changes the caller's value in place. The receiver follows the same rule:
`self` is lent, `mut self` changes the object in place, and `keep self` takes the object over.
Here is a small cart:

```nika
struct Item { name: String, price: int }

struct Cart { items: Vec[Item] }

impl Cart {
    fn total(self) -> int {
        let mut sum = 0
        for item in self.items {
            sum += item.price
        }
        return sum
    }

    fn add(mut self, keep item: Item) {
        self.items.push(item)
    }

    fn into_items(keep self) -> Vec[Item] {
        return self.items
    }
}

fn main() {
    let mut cart = Cart { items: [] }
    cart.add(Item { name: "pen", price: 3 })
    cart.add(Item { name: "ink", price: 9 })
    println(cart.total())
    let items = cart.into_items()
    println(items.len())
}
```

```text
12
2
```

`total` only reads the cart, `add` changes it and keeps the item it is handed, and
`into_items` takes the whole cart over. The calls in `main` write none of this: what an
argument becomes is read off the callee's declaration, so a reader of a function sees what
happens to each argument without reading its body.

The compiler still works out from the body what a function keeps, and checks the signature
against it. Without the `keep` on `add`, the cart is refused:

```nika
struct Item { name: String, price: int }

struct Cart { items: Vec[Item] }

impl Cart {
    fn total(self) -> int {
        let mut sum = 0
        for item in self.items {
            sum += item.price
        }
        return sum
    }

    fn add(mut self, item: Item) {
        self.items.push(item)           // does not compile: NK2302
    }

    fn into_items(keep self) -> Vec[Item] {
        return self.items
    }
}

fn main() {
    let mut cart = Cart { items: [] }
    cart.add(Item { name: "pen", price: 3 })
    println(cart.total())
}
```

```text
error[NK2302]: `Cart::add` keeps `item`, but `item` is not marked `keep`.
   --> main.nika:15:9
    |
 15 |         self.items.push(item)           // does not compile: NK2302
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: A parameter the body keeps - stores into something that outlives the call, gives to a task, or hands to a parameter that keeps it - says so in the signature (ADR-views-3 D2).
    = note: Raised in a file that writes `keep` at least once, until the migration has written it everywhere (#1213).
    = help: Write `keep item` in the signature of `Cart::add`.
```

The `item` pushed into the cart outlives the call, so the signature has to say so, and the
help writes the word. As the second note says, the compiler asks for `keep` today only in a
file that already writes it somewhere: code written before the word existed still builds,
and `nikaia migrate views-3 <path>` writes `keep` into it from the ledger.

`ref` has nothing left to say beside `keep`. It is written only in an `extern "C"`
declaration, where `ref i32` and `ref mut i32` remain the pointers C expects; the migration
removes it everywhere else.

| Before | Now |
| :--- | :--- |
| `fn total(ref self)` | `fn total(self)` |
| `fn add(ref mut self, item: Item)` | `fn add(mut self, keep item: Item)` |
| `fn into_items(self)` | `fn into_items(keep self)` |
| `fn shout(s: ref String)` | `fn shout(s: String)` |
| `return ref s` | `return s` |

A result that points into a parameter, like `longest` above, is worked out from the body. It
may also be written on the result with `from`, and then the body is checked against it:

```nika
fn first_word(s: String) -> String from s {
    let words: Vec[String] = s.split(" ").collect()
    return words[0]
}

fn longest(a: String, b: String) -> String from a | b {
    if a.len() >= b.len() {
        return a
    }
    return b
}

fn main() {
    println(first_word("hello world"))
    println(longest("ab", "abc"))
}
```

```text
hello
abc
```

The rest of the design is still being built
([#1213](https://github.com/Nikaia-Foundation/Nikaia/issues/1213)): a `keep` the body does not
need is to get a warning, `from` is to be required where there is no body to read it from (a
trait's method, a function type, a foreign function), and the ledger is to write the source in
the same spelling, `-> String from a | b`, where it writes `-> ref(a | b) String` today.

**Specification:** [ADR-views-3](../docs/specification/adr/adr-views-3.md) · [Part I, §6.5 References and borrowing](../docs/specification/10-nikaia-light.md#65-references-and-borrowing)

---

## Where next

* [A tour of the syntax](syntax.md): the rest of the language on one page.
* [Getting started](getting-started.md): install the compiler and run a project.
* [Contracts](contracts.md): what the compiler proves about an `assert`.
* [The prover](prover.md): what the compiler proves today, where it stops, and what is planned.
* [Grammars and parsers](grammars.md): describing a text format and parsing it.
* [Specification, Part I, Chapter 6](../docs/specification/10-nikaia-light.md#chapter-6-memory-and-ownership):
  the rules in full.
