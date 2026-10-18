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

## 2. The Space Between Scripting and Systems
The ancient city of Nikaia is known for its **council**: a place where people who disagreed
sat down at one table. Programming has its own two camps today:

* The **scripting camp** (Python, JavaScript, PHP, Ruby): direct, flexible, fast to write. And often
  fragile.
* The **systems camp** (C, C++, Rust, Zig): fast, safe, in control of the machine. And often heavy
  to carry.

The differences are real. But somewhere along the way we stopped calling them differences and
started calling them laws. Why should simple code mean giving up guarantees? Why should safe,
fast software mean that you manage the machinery yourself?

We don't think it has to be that way. We think much of that weight can move from the
programmer into the compiler.

Nikaia is an attempt to show it, starting in the middle.

Nikaia's home is application development: the backends, the web services, the tools and
programs that have to be fast and right at the same time. That is what it is built for
first.

From there it reaches out in both directions. Toward the machine: systems programming, down to
bare metal. Toward the script: the small programs we reach for a scripting language to write.
Out there, Nikaia may serve you well. But this is a young language, and at the edges we make
fewer promises. Close to the hardware, there may be cases where we cannot yet give you full
control over how a structure is laid out in memory, or exactly when it is freed. Close to the
script, Nikaia may not start as instantly or feel as loose as an interpreter, because it checks
more before your program runs. Where we hit such a limit, we will name the trade-off, not
hide it.

A strong center, honest edges. Nikaia doesn't have to win everywhere to matter.

It needs to show that you can write plain, direct code and still get native speed, memory
safety, concurrency the compiler checks, and a compiler that proves more about your program
over time.

We won't declare the divide abolished. We want to show that it is narrower than we thought.

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
the compute that code runs on stopped being cheap. §8 below covers this.

---

## 6. What Nikaia Does Differently

The two build settings are described in [Getting started](guide/getting-started.md#the-two-build-settings).
Below are the main design choices. Each is specified, and each links to the decision record
that argues it.

**1. Functions have no colour.**
There is no `async` and no `await`. Any function may pause on I/O; the compiler builds the
state machine. So there is no split of the ecosystem into a sync half and an async half,
where every library has to exist twice.
→ [Spec Part I](docs/specification/10-nikaia-light.md)

**2. Ownership without lifetime annotations.**
Nikaia compiles through the Rust toolchain, so it inherits the borrow checker's guarantees,
but you never write `'a`. Borrow contracts are inferred over the whole program and written to
a ledger you can read. Cases that safe Rust cannot express directly (a slice stored in a
struct, a task borrowing from its parent) get their own language constructs.
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

### How it works

Nikaia is not a new backend. The frontend lowers a `.nika` file to Rust source text, and
`rustc` compiles that like any other Rust. That makes points 1, 2 and 3 tractable: the safety
machinery already exists and is widely used, and Nikaia's job is to operate it so that you
do not have to. The generated Rust is kept readable on purpose: it is the text that is
compiled, so a compiler bug can be inspected there.

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
