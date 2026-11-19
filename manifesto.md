# Nikaia: Philosophy & Origin

**For Nika.**
*Because the future belongs to those who build it.*

---

## 1. The Name and the Heart
At the centre is **Nika** — my daughter's name. Everything else comes after that.

The name carries more than one lineage, and the deeper one is not the Greek: in Persian, *nik*
means **good** — virtuous, good the way a person is good and not the way a product is. The
motto is that name with a verb added: **Good wins.**

Not over anyone. What the sentence denies is something else: that good and fast are opposites,
and that being decent to the person writing the code has to be paid for at runtime. Here it is
the other way round. Because the language never makes you write `Arc`, lifetimes or lock orders,
those decisions belong to the compiler. Only because they belong to it can it choose `Rc`
where nothing of yours runs at once and `Arc` where it does, order the locks, and infer borrow contracts across a whole
program. A stricter language would have to take your word for it. Good is not the price of fast
here; it is the reason for it.

The ancient city of **Nikaia** (Νίκαια), which the name also points to, supplies the second
image, described in the next section.

This language is dedicated to her. It is an attempt to leave behind a technological world shaped
less by unnecessary hurdles and more by the freedom to create.

Because so much of software development costs effort that has nothing to do with the actual
problem: the compiler that holds you up, race conditions that only surface under load, the limit
of what one person can hold in their head. This is not a war, and nobody has to be defeated for
it. Nikaia tries to move that complexity into the compiler.

## 2. Not a Compromise
The ancient city of Nikaia is known for its council, a place where people who disagreed sat
down at one table. Programming has its own two camps today. The scripting camp, with Python,
JavaScript, PHP and Ruby, is direct, flexible and fast to write, and often fragile. The systems
camp, with C, C++, Rust and Zig, is fast and in control of the machine, in its newer members also
safe, and often heavy to carry.

The usual reading is a scale with simple at one end and safe and fast at the other, and every
language a point somewhere on it. A language that wants more safety pays in effort, and one that
wants less effort pays in guarantees or in speed. Somewhere along the way that stopped being a
description of today's languages and became a law.

We do not think it is a law. A council is not where each side gives up half of what it wanted.
It is where the table finds an answer both sides can sign. Nikaia does not aim for a point in the
middle of the scale. It aims to be safer than the languages that already join safety and speed,
faster than them, and still simpler than they are. That is the claim this language exists to
prove, and §6 says why we think it can be done.

Nikaia's home is application development, with the backends, the web services, the tools and
programs that have to be fast and right at the same time. That is what it is built for
first.

From there it reaches out in both directions, toward the machine with systems programming down
to bare metal, and toward the script with the small programs we reach for a scripting language
to write.
Out there, Nikaia may serve you well. But this is a young language, and at the edges we make
fewer promises. Close to the hardware, there may be cases where we cannot yet give you full
control over how a structure is laid out in memory, or exactly when it is freed. Close to the
script, Nikaia may not start as instantly or feel as loose as an interpreter, because it checks
more before your program runs. Where we hit such a limit, we will name the trade-off, not
hide it.

A strong centre, honest edges. The claim is made for the centre first.

## 3. The Agora and the Swarm
Nikaia needs two images, and only one of them is a place.

1.  **The Marketplace (Agora), nothing of yours running at once.**
    One square, in constant movement. Trade, exchange, flow. Nobody stands still waiting for
    anybody else: everything is "non-blocking".
    *Optimised for:* I/O density, web services, rapid prototyping.
2.  **The Swarm, every core busy.**
    A swarm has no centre, no commander and no walls to defend. Every worker takes what is in
    front of it, and when it runs out it takes work from a neighbour. The order comes from the
    rules everyone follows. That is what a work-stealing runtime is, and it is why safety here
    comes from the rules and not from everybody's discipline.
    *Optimised for:* compute power, thread safety, every core busy.

## 4. The Manifesto

For a long time we believed we had to choose.

We built our skyscrapers on sand because the concrete was too hard to mix. We wrote software
that felt good and collapsed in the night. Or we forged systems out of pure steel that lasted
forever, but whose construction cost us our joy.

We accepted the dogma: *"Simple is slow. Fast is hard."*

And then we asked the question: what if the weight is not in the tool, but in the way we hold
it? What if the compiler is not our overseer, but our architect?

We called the project **Nikaia**.

Because intent should count again, and not implementation. Because it pushes back the walls
between the code we dream and the code the machine understands.

**Nikaia: Good wins.**

---

## 5. Why This Exists

Writing fast, correct software today means paying a tax that has nothing to do with your
problem.

You want to read a file and answer a request. Instead you decide whether the function is
`async` or not, and that decision spreads to every caller. You want to keep a name that points
into a buffer you already have. Instead you write lifetime annotations, or you allocate a
copy you didn't need. You want a value two tasks can see. Instead you pick `Rc` or `Arc`
*by hand*, and if you pick wrong the code stops compiling three modules away. You take two
locks and hope everyone else in the codebase takes them in the same order.

None of that is your program. It is bookkeeping, and bookkeeping is the kind of work a
compiler is good at.

Nikaia is a bet that most of that tax is compiler work. You write straight-line code that
says what should happen. The compiler decides the concurrency model, the pointer types, the
lock order, the lifetimes and the state machines. It does this by inference over the whole
program, without a garbage collector.

### Why now

The bet would have been a harder sell fifteen years ago, because for fifteen years nobody had
to take it. Efficiency in software has gone through three eras:

1. **Constraint.** Small machines and small budgets meant you had to understand the machine —
   memory layout, I/O cycles, what the cache does. Every line had physical weight.
2. **Subsidy.** Cheap cloud capacity and cheap capital turned horizontal scaling into the
   answer to every performance question. Hardware quietly paid the bill for inefficient
   software, and the bill compounds: a home computer of the constraint era ran its operating
   system, an editor and a game inside 64 KB, while a chat window today asks for a few hundred
   megabytes, four orders of magnitude more, to show a list of messages. A CRUD service came to need
   an orchestrator to stay up.
3. **The wall.** Training and inference now compete for the same power, silicon and datacenter
   capacity as everything else, and competition prices things. "Throw more servers at it" is no
   longer the cheap answer to a design problem, and in some regions there are no more servers
   to throw.

Nikaia does not go back to era one. You keep the ergonomics the subsidy bought (no manual
thread management, no callbacks, no lifetime bookkeeping), and the compiler pays for them
once, at build time, instead of the runtime paying for them on every request. That trade is
the economic argument for the project.

The same technology is behind both halves of it. AI is why a growing share of code is no
longer typed by a human, and a generator needs guarantees more than comfort. It is also why
the compute that code runs on stopped being cheap. §8 below covers this. And it is the reason a
compiler that carries this much can be built at all, as §6 explains.

---

## 6. How It Can Be All Three

Every new language says it is safe, fast and simple. What matters is why it should hold this
time, when careful people have tried for decades. We see five reasons, and none of them is a
single trick.

### Where the decisions are made

Much of what a programmer decides by hand to get safety or speed is a decision the compiler can
make instead, if it sees enough of the program. Which pointer type a value needs, in which order
locks are taken, how long a borrow lives and whether a function may pause are such decisions.
Nikaia's compiler looks at the whole program and makes them. The programmer writes less, and
because the compiler decides from facts rather than from a guess written down early, it can often
decide better. It picks the cheap reference count where nothing runs at once, the cheap hasher
where no untrusted data is hashed, and no index check where the index is proved in range.
Simplicity and speed come from the same place here, which is why they do not have to be traded
against each other.

### Proofs and optimisation on the same tree

Compilers have long been built as a chain. The source is lowered into one intermediate
representation after another, and each analysis or optimisation runs on whichever layer suits
it. That was a sound answer to a hard problem, and it has a cost. By the time an optimiser runs,
much of what the program said is gone and has to be found again, such as whether an index is in
range, whether two references overlap or whether a call touches anything.

We are convinced the work can be done on the tree the programmer wrote. Nikaia's compiler infers,
proves and decides on its checked syntax tree, and the optimiser we are building reduces that
tree onto a smaller subset of Nikaia itself and rewrites it there, tree to tree. Each rewrite is a
rule that one construct means the same as another where a condition holds, and the condition is
shown by the same prover that checks your contracts. What is proved and what is optimised are the
same program, so a proof turns directly into speed, and every step can be printed as Nikaia, read
and checked again. Where a solver needs a logic of its own, the compiler asks it a question, and
the program does not pass through that logic as a stage
([#1235](https://github.com/Nikaia-Foundation/Nikaia/issues/1235),
[#1593](https://github.com/Nikaia-Foundation/Nikaia/issues/1593),
[ADR-270](docs/specification/adr/adr-270.md)).

### Why existing languages stay where they are

Most of the ideas in this manifesto are not new on their own. What an established language finds
hard is to adopt them without breaking the code already written in it. A language built around a
garbage collector keeps it, a language whose functions are coloured keeps the colour, and a
language whose programs write lifetimes and pick `Rc` or `Arc` by hand keeps those decisions in
millions of lines that made them. Each is held where it stands by its own success. Nikaia has no
installed base to protect, so it can choose each concept on its merits and build the language
around the ones that fit together.

### Why such a compiler can be built now

A compiler that infers ownership over a whole program, builds the state machines, orders the
locks, proves contracts through loops and optimises from those proofs is a very large piece of
work. For a long time that was the honest reason not to try. Language designers and researchers
knew most of the parts, and putting all of them into one language was more than a team could
carry. That has changed. AI agents now write a large share of the code under human direction, and
work that was out of reach for a small team is within it. This project is the test of that
claim. Its specification is argued in a few hundred decision records, and its compiler is being
rewritten in Nikaia, a module at a time.

### The ecosystem Nikaia stands on

New languages often start from zero and wait years for their libraries. Nikaia stands on an
existing ecosystem instead. It builds through Cargo, finds a dependency by version through
crates.io, and calls a Rust crate once its interface is described. Today the compiler writes Rust
and `rustc` builds it. That is the path we have, not the definition of the language. A program
of Nikaia alone may later go to machine code directly, with copy-and-patch as one candidate,
while a program that uses Rust crates keeps a bridge to them. Which backend a program takes
follows from what it depends on, and the guarantees belong to the language
([ADR-002](docs/specification/adr/adr-002.md),
[ADR-rust-boundary-1](docs/specification/adr/adr-rust-boundary-1.md),
[#1235](https://github.com/Nikaia-Foundation/Nikaia/issues/1235)).

### What it looks like in the language

The two build settings are described in [Getting started](guide/getting-started.md#the-two-build-settings).
Below are the main design choices. Each is specified, and each links to the decision record
that argues it.

**1. Functions have no colour.**
Any function may pause on I/O, and the compiler builds the state machine. The program waits
where it needs a value: at the call itself, or, for work started with `async`, where the
value is first used. So there is no split of the ecosystem into a sync half and an async half,
where every library has to exist twice.
→ [Spec Part I](docs/specification/10-nikaia-light.md)

**2. Ownership without lifetime annotations.**
Lifetimes are not written in the source. Borrow contracts are inferred over the whole program, checked by
Nikaia's compiler and written to a ledger you can read; on today's path the Rust that comes out
passes Rust's borrow checker as well, a second opinion on ours. Cases that safe Rust cannot
express directly (a slice stored in a struct, a task borrowing from its parent) get their own
language constructs.
→ [ADR-005](docs/specification/adr/adr-005.md), [ADR-283](docs/specification/adr/adr-283.md)

**3. One source, every runtime.**
You write `Shared[T]`. What it becomes underneath is the compiler's: the atomic reference count
is the default, and a value it can prove never crosses a thread gets the plain one instead,
decided per value ([ADR-037](docs/specification/adr/adr-037.md) D7), and where the build runs
your code on one thread every count is plain ([ADR-312](docs/specification/adr/adr-312.md) D10). The
lock in `Locked[T]` is decided the same way and off the same answer
([ADR-281](docs/specification/adr/adr-281.md) D5-D7), and so is where `spawn`'s tasks run.
Your source file does not encode the deployment decision, so changing it is a line in
`nikaia.toml`, not a refactor.

**4. No lock-order deadlocks.**
Several resources are requested together, with `access_all(a, b)`, and the runtime always
takes them in address order. A deadlock cycle between two `access_all` callers cannot
form. The lambda must be `sync` (provably non-pausing), so a lock can never be
held across an I/O suspension point.
→ [Spec Part II](docs/specification/20-nikaia-advance.md)

**5. Grammars are part of the language.**
`dsl` is an expression. A scannerless grammar can be parsed at compile time or at run time
with the same syntax, and an embedded DSL (SQL, JavaScript, x86) gets its own syntax instead
of string interpolation. A grammar rule marked `@frame` can be folded in parallel with
`par_fold`, and the compiler checks that the input can be cut at its boundary. Parsing
benchmarks are therefore a main target.
→ [ADR-296](docs/specification/adr/adr-296.md)

**6. The compiler tracks where your data came from.**
Trust is a property of where data comes from: bytes off a socket are untrusted, your own
config file is not, and that provenance travels with the value. The compiler picks a
DoS-resistant hasher where untrusted data is hashed and a fast one elsewhere, so a program
does not pay for the worst case everywhere, and you do not have to remember which is which.
→ [ADR-010](docs/specification/adr/adr-010.md)

**7. No garbage collector.**
Deterministic teardown through ownership and RAII, also in functions that pause, where
"when does this file close" is otherwise a hard question.
→ [ADR-297](docs/specification/adr/adr-297.md)

### How it is built today

Today the frontend lowers a `.nika` file to Rust source text, and `rustc` compiles that like any
other Rust. That gives every Nikaia program the Rust ecosystem and a mature code generator from
the first day. The generated Rust is kept readable on purpose, because it is the text that is
compiled and a compiler bug can be inspected there. The interface between the two is text and
nothing else, so the path behind it can change without the language changing.

What you need installed is an ordinary stable Rust toolchain. The Rust
that comes out uses no unstable feature, no `-Z` flag is passed anywhere, and no crate in the
workspace declares a `#![feature(…)]`. A `nikaia` links `libc` and nothing exotic, so the
binary goes where you put it.
→ [ADR-003](docs/specification/adr/adr-003.md), [ADR-001](docs/specification/adr/adr-001.md) D1,
[ADR-004](docs/specification/adr/adr-004.md) D1,
[Toolchain architecture](docs/toolchain_architecture.md)

---

## 7. How It Compares

| | Python / TS | Go | Rust | **Nikaia** |
| :--- | :--- | :--- | :--- | :--- |
| Async in the type system | colours functions | invisible (goroutines) | colours functions | invisible |
| Memory management | GC | GC | ownership, manual annotations | ownership, inferred |
| Pause times | GC pauses | GC pauses | none | none |
| Thread-safe vs single-thread types | n/a | n/a | you choose `Rc`/`Arc` | one type, the compiler decides |
| Data-race protection | none / GIL | detector at runtime | compile time | compile time |
| Deadlock protection | none | none | none | `access_all` ordering |
| Embedded DSLs | strings / metaprogramming | none | proc macros over Rust tokens | scannerless grammars in the language |
| Single-core / multi-core | one runtime | one runtime | you build it | one setting |
| Maturity | production | production | production | pre-alpha |

In the last row Nikaia is behind, and only time changes that.

### A goal, and the evidence so far

This manifesto says what Nikaia wants and why. It does not report what works today, which is in
the [status and roadmap](docs/project_status_and_roadmap.md) and in the
[open issues](https://github.com/Nikaia-Foundation/Nikaia/issues). The claim of §2 is a goal the
language has to prove, and measurements will prove or refute it, not this page.

There is evidence that it can be reached, and soon. On the One Billion Row Challenge, counted with
callgrind, Nikaia's fastest program runs about a third fewer instructions on one thread than the
fastest human-written Rust we measured, in a sixth of its tokens, without `unsafe` and with every
index and overflow checked or proved ([benches/brc](benches/brc/README.md)). The compiler proves
`assert`s through loops and recursion without loop invariants written by hand, and leaves out
the run-time checks it has proved ([Contracts](guide/contracts.md)). And two callers of
`access_all` take their locks in one order, a lock held across a pause is refused, and two tasks
that would change the same value at once are refused at compile time
([Spec Part II](docs/specification/20-nikaia-advance.md)).

Where Nikaia loses a measurement, the measurement says so and names what is known of the cause
([benches/game](benches/game/README.md)).

---

## 8. A Language for the Age of Generated Code

There is an obvious objection to launching a systems language in 2026: if a model writes the
`if err != nil` chains and argues with the borrow checker on your behalf, who cares how
ergonomic the syntax is? "Nicer to type" is a shrinking argument.

The objection is correct, and it leads to a stronger one. When a machine writes the code,
the question is less how pleasant the code is to write and more what the compiler can prove
about code that no human wrote. Under that question the points above read differently.

* **Boilerplate costs context, not just keystrokes.** A model has a finite window and loses the
  thread as it fills. `async`/`await` plumbing, lifetime annotations, `Arc::clone` calls and
  error-propagation chains spend that budget on machinery instead of on your problem. Nikaia
  is dense on purpose. For humans that is a comfort; for a generator it is capacity.
* **Concurrency is where generated code fails silently.** Models are very good at plausible code
  and weak on the memory model. A hallucinated lock order or a shared mutable capture is not a
  compile error in Go or C++; it is a bug that appears in production, under load, once. In
  Nikaia a deadlock between `access_all` callers is unconstructible, a lock cannot be held
  across a suspension point, and data races are rejected at compile time however much runs at
  once. The compiler reviews the class of defect that human review is worst at.
* **Fewer decisions to get wrong.** `Rc` or `Arc`? The sync or the async variant of this API? Is
  this future `Send`? Each is a coin flip a generator can lose, and losing it surfaces as an
  error three modules away, in code the author has never read. In Nikaia these decisions are not
  in the source at all; the compiler and the build settings make them.

This is also the honest answer to a new language's chicken-and-egg problem. Nobody has to learn
Nikaia to get something out of it: hand a model the specification and your requirements, and let
a compiler that rejects lock-order deadlocks and data races check what comes back. A
generated program that runs fast and refuses to race is a better first contact with a language
than a tutorial is.

The caveat: no model has Nikaia in its training data, so generating it means putting the
specification in context. The spec is written to be precise and small enough to fit, and a
ready-made prompt bundle is on the [roadmap](docs/project_status_and_roadmap.md). Until then, `docs/specification/`
plus `examples/` is the bundle.

---

## 9. Dedication — for Nika

**Nika is my daughter, and I love her more than anything I will ever build.**

Everything in this repository — the specification, the compiler, every argument above — sits
downstream of one simple wish: that the world she grows into has fewer walls in it than the one
I found. The language carries her name because she is the reason it exists at all.

*Because the future belongs to those who build it.*

We are already standing in tomorrow's past: everything that will be ordinary in twenty years is
being decided right now. Everyone decides, and everyone prioritises — and it does not have to be
the whole machine. One gear, turning, is enough to move the ones around it.

That is one half of how I think about Nikaia. It is not mainstream. It takes a different route
and argues with settled consensus in several places. That is the part of me swimming against the
current: one person deciding to turn, and accepting that turning alone is slow.

The other half is knowing I am made of the river. Nikaia was written with the help of AI, which
stands on the accumulated knowledge of everyone who ever wrote a compiler, a paper, a textbook,
a Stack Overflow answer. It rests on Rust's borrow checker, on decades of runtime research, on a
language nobody in this repository invented. It was built with money, electricity and machines
that the flow provided. Not one idea in it is uncaused. Nothing here was taken from nowhere.

It also cost less than it should have. Without AI, this project would have lost the
prioritisation to the people I love — and rightly so. What AI changed is the price: it made
things move fast enough that Nikaia could exist without taking its time out of theirs. Money,
yes. Evenings, far fewer than it would have cost a few years ago.

Both are true at once, and the contradiction is not a flaw in the story — it is the story.
Swimming against the stream is still swimming in it. This is a solo effort that only makes sense
as a shared work: it exists to show what a single person can still put into the world, and in
the same breath to admit how much of that was already lying there, done by others, waiting to be
picked up.

So: to everyone whose work is upstream of this one — the named and the uncounted, the people who
built the tools and the people who wrote down why — thank you. Most of you never heard of this
project. And to whoever is downstream, who will take this further or take it apart and build
something better: this was always meant to end up in your hands.

And to Nika: whatever becomes of it, I would rather hand you a world shaped a little less by
walls and a little more by the freedom to build. ❤️

---

## 10. Intellectual Property & Governance

Nikaia introduces what it calls the **Unified Core Architecture**: compile-time
orchestration, deterministic concurrency (`access_all`), and switch-based runtime
transformation.

**For corporate entities & implementers:**
This repository establishes public **prior art** for these architectural concepts, ensuring
they remain unencumbered for the open-source ecosystem.

While Nikaia is released under the **Apache 2.0 License**, the project is designed with
long-term governance in mind to prevent proprietary fragmentation. We actively invite
organizations interested in adopting, extending, or standardizing these concepts to join us as
**Founding Partners** rather than attempting parallel implementations.

The architecture is complex; let's build the standard together.
