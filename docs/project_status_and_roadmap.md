# Nikaia: status and roadmap

One page for the question from outside: where does the language stand, how
risky is it to rely on it, and which parts of the specification are built and
tested and which are still to be built or proved. *As of 0.0.1311, October 30,
2026.*

**How this page stays current.** It holds no list of work and no percentage
counted by hand; the work is in the
[issue tracker](https://github.com/Nikaia-Foundation/Nikaia/issues), and each
row below links to it. A row changes when a chapter changes its level (below),
not when an issue closes. A figure on this page names the command that recounts
it.

## Where the project actually stands

**Pre-alpha.** Single programs and multi-file projects on Linux build and run,
and the compiler is mostly written in Nikaia itself. The specification is ahead
of the compiler: several chapters are decided and written but only partly built,
and syntax and diagnostics still change between versions with no migration.

| Figure | Today | Recounted by |
| :--- | :--- | :--- |
| Toolchain written in Nikaia (self-hosting, [ADR-294](specification/adr/adr-294.md)) | 82.4 % | `python3 scripts/self_hosting.py` |
| Issues closed / open | 584 / 216 | [the tracker](https://github.com/Nikaia-Foundation/Nikaia/issues) |
| Change packages, one per [CHANGELOG](../CHANGELOG.md) heading | 1304 | `grep -c '^## \[' CHANGELOG.md` |

The version `0.0.N` counts change packages and says nothing about readiness.

### How to read the risk

Most defects are now found by moving the compiler into Nikaia and by writing the
official packages: each move puts real code through the compiler, and each
defect found is fixed in the compiler, with a test, never worked around
(`CLAUDE.md`, *Moving Rust into Nikaia*). Hundreds have been found and fixed
this way. Defects are not equally serious; the order in which they are taken
is scored in [`priorities.md`](priorities.md), and the classes, worst first,
are these:

1. **A wrong value**: a program builds and computes something other than what
   its source says. Rare, and always taken first.
2. **A missing refusal**: the specification forbids something and the compiler
   builds it anyway, for example two `overlap` branches that write the same
   thing ([#985](https://github.com/Nikaia-Foundation/Nikaia/issues/985),
   [#993](https://github.com/Nikaia-Foundation/Nikaia/issues/993)) or a lock
   taken inside a lock ([#677](https://github.com/Nikaia-Foundation/Nikaia/issues/677)).
3. **Accepted, then refused by `rustc`**: Nikaia's checker accepts a program the
   Rust compiler under it refuses. Nothing wrong runs, but the message is in
   Rust's words. This is the largest open group, nearly all in memory and
   ownership (chapter 6).
4. **A wrong refusal**: a correct program is refused; the user writes it another
   way.

Questions that need the owner's decision first are labelled
[`decision`](https://github.com/Nikaia-Foundation/Nikaia/issues?q=is%3Aopen+label%3Adecision);
work that can be built now is labelled
[`ready`](https://github.com/Nikaia-Foundation/Nikaia/issues?q=is%3Aopen+label%3Aready).

## The specification, chapter by chapter

The levels, from most to least settled:

* **Proven**: built, tested, and used in earnest: by the compiler's own Nikaia,
  the official packages or the examples.
* **Built**: built and tested, with little use beyond its tests yet.
* **Partial**: the core is built; named parts are specified and not built.
* **Rebuilding**: built in an older form; a newer decision replaces it.
* **Specified**: decided and written, not built.

| Chapter | Level | What stands | Open |
| :--- | :--- | :--- | :--- |
| **Part I** | | | |
| 2 Variables and data types | Proven | numbers typed by their uses, one text type ([ADR-282](specification/adr/adr-282.md)), `T?` and `null` | rounding of `/` and `%` unspecified ([#682](https://github.com/Nikaia-Foundation/Nikaia/issues/682)); `text::Pos` ([#743](https://github.com/Nikaia-Foundation/Nikaia/issues/743)); `f32` literals ([#728](https://github.com/Nikaia-Foundation/Nikaia/issues/728), decision) |
| 3 Control flow | Proven | `if`, `while`, `for`, `match`, `break`/`continue` | parser gaps such as [#939](https://github.com/Nikaia-Foundation/Nikaia/issues/939) |
| 4 Data structures | Proven | structs, enums, lists, maps, sets, arrays; the collections offer one surface ([ADR-collections-1](specification/adr/adr-collections-1.md), [-2](specification/adr/adr-collections-2.md)) | `BTreeMap::entry` and the rewrite after it ([#950](https://github.com/Nikaia-Foundation/Nikaia/issues/950)); type kinds ([#859](https://github.com/Nikaia-Foundation/Nikaia/issues/859)) |
| 5 Functions and arguments | Proven | `Subject ; Config`, lambdas, inferred `sync` | the receiver rule ([#870](https://github.com/Nikaia-Foundation/Nikaia/issues/870)); `ref` on parameters ([#875](https://github.com/Nikaia-Foundation/Nikaia/issues/875), decision) |
| 6 Memory and ownership | Partial, **highest risk** | views that outlive their buffer with no annotation (the tether, [ADR-283](specification/adr/adr-283.md)), text tiers, `mut` parameters and views ([ADR-views-1](specification/adr/adr-views-1.md)) | most class-3 defects live here; `for mut` ([ADR-views-2](specification/adr/adr-views-2.md)); a struct that hands out views of text it read ([#929](https://github.com/Nikaia-Foundation/Nikaia/issues/929)) |
| 7 Error handling | Proven | `throws` with typed sets, `catch`, errors that keep their site | a code parameter's failures per call ([ADR-error-sets-1](specification/adr/adr-error-sets-1.md), specified) |
| 8 Concurrency | **Rebuilding** | today: `spawn`, `overlap`, an executor at both `user-parallelism` settings, supervision | one concept, the task, started by `async` ([ADR-async-1](specification/adr/adr-async-1.md) to [-4](specification/adr/adr-async-4.md), decided 2026-10-30); the build is [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983) and its sub-issues |
| 9 Project organisation | Built | modules, `pub`, packages by path | no registry, so no dependency by version |
| **Part II** | | | |
| 10 Metaprogramming | Proven | grammars and `dsl` blocks, used by the `json` and `yaml` packages and the HTTP parser; build-time evaluation; no macros, on purpose ([ADR-298](specification/adr/adr-298.md)) | grammar defects [#999](https://github.com/Nikaia-Foundation/Nikaia/issues/999), [#1001](https://github.com/Nikaia-Foundation/Nikaia/issues/1001)-[#1004](https://github.com/Nikaia-Foundation/Nikaia/issues/1004); streaming a grammar's input one item at a time (in the parser generator, not yet written by Nikaia, [#928](https://github.com/Nikaia-Foundation/Nikaia/issues/928)); specialising per build-time value ([#906](https://github.com/Nikaia-Foundation/Nikaia/issues/906)) |
| 11 Running code at once | Rebuilding | the same source on one thread or on a pool (`user_parallelism`), implicit pausing, `par_iter` | the free task `async(detached)` and fan-out by `par_iter` ([ADR-async-2](specification/adr/adr-async-2.md), [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983)) |
| 12 Thread safety | Partial | `Shared`/`Locked`, `access_all`, no lock held across a pause (`NK2202`) | the refusals of class 2 above ([#677](https://github.com/Nikaia-Foundation/Nikaia/issues/677), [#985](https://github.com/Nikaia-Foundation/Nikaia/issues/985)) |
| **Part III** | | | |
| 13 Toolchain | Built | `build`, `run`, `check`, `test`, `fmt` ([ADR-337](specification/adr/adr-337.md)), `describe`, `bind`, content-keyed cache | `nikaia doc`, a language server, a package registry |
| 14 Testing and quality | Proven | `test` blocks, output tests, `assert` proved while building or checked at run time, contracts across packages | tests of several files ([#926](https://github.com/Nikaia-Foundation/Nikaia/issues/926)); facts about data ([#797](https://github.com/Nikaia-Foundation/Nikaia/issues/797), [#798](https://github.com/Nikaia-Foundation/Nikaia/issues/798)) |
| 15 Interoperability | Partial | calling C (`extern`, libraries from `nikaia.toml`), calling a described Rust crate, a C library built from a package, Python and Node bindings ([ADR-284](specification/adr/adr-284.md)) | a package as a Rust library ([#978](https://github.com/Nikaia-Foundation/Nikaia/issues/978)); WebAssembly and bare metal ([ADR-119](specification/adr/adr-119.md)) |
| 16 Hardware instructions | Partial | vectors in `std::simd` | the rest of [ADR-371](specification/adr/adr-371.md) ([#755](https://github.com/Nikaia-Foundation/Nikaia/issues/755), decision; [#876](https://github.com/Nikaia-Foundation/Nikaia/issues/876)) |
| 17 Standard library and packages | Partial | text, collections, files, I/O, sockets, processes, `std::regex` ([ADR-regex-1](specification/adr/adr-regex-1.md)), a minimal HTTP/1.1 server; the packages `json` and `yaml` ([ADR-yaml-1](specification/adr/adr-yaml-1.md), in progress) | `std::db` and the SQL checked while building ([ADR-299](specification/adr/adr-299.md)); TLS, HTTP/2; entries the toolchain's move needs ([#920](https://github.com/Nikaia-Foundation/Nikaia/issues/920), [#922](https://github.com/Nikaia-Foundation/Nikaia/issues/922)) |

### The walls you will hit

| You try to... | What happens |
| :--- | :--- |
| depend on a Nikaia package by version | refused: no registry yet, `path = "..."` only |
| use a crate from crates.io | works after `nikaia describe <crate>`, whose output you review and commit |
| get completion or generated docs | nothing: no language server, no `nikaia doc`; [`editors/vscode`](../editors/) highlights syntax |
| talk to a database | only through its C library, as [`examples/sqlite`](../examples/sqlite/) does; `std::db` is specified, not built |
| serve HTTPS, HTTP/2, or many connections at once | not possible yet |
| write the new `async` | not yet: it is being built ([#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983)); `spawn` and `overlap` work until it replaces them |
| find a function you expect in `std` | sometimes missing; a missing entry is worth an issue |

Some errors still arrive in `rustc`'s words (class 3 above), pointed at your
`.nika` line.

## What comes next

The order is set by the score in [`priorities.md`](priorities.md). Current
direction, in short:

* **Self-hosting first** (owner, 2026-10-27): until the compiler holds no logic
  in Rust, every round moves code into Nikaia, largest files first. The road and
  what is left: [`selfhosting-plan.md`](selfhosting-plan.md),
  [`selfhosting-gaps.md`](selfhosting-gaps.md).
* **Compiler defects** a move or a package runs into are fixed as they are met.
* **The task model** ([#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983)):
  parser, checker, runtime and lowering of `async`, then `overlap`, `task::scope`
  and `spawn` are removed.
* **The `yaml` package**, also as a drop-in Rust crate, with streaming input
  ([#928](https://github.com/Nikaia-Foundation/Nikaia/issues/928)).
* **Later, decided and not scheduled**: `std::db` and the query DSL, WebAssembly,
  bare metal, `nikaia doc`, a language server, a registry.

Every design decision is argued in an [ADR](specification/adr/README.md) and
accepted by a human. The history of each change is in the
[CHANGELOG](../CHANGELOG.md).
