# Nikaia: status and roadmap

One page for the question from outside: where does the language stand, how
risky is it to rely on it, and which parts of the specification are built and
tested and which are still to be built or proved. *As of 0.0.1374, November 10,
2026.*

**How this page stays current.** It holds no list of work and no percentage
counted by hand; the work is in the
[issue tracker](https://github.com/Nikaia-Foundation/Nikaia/issues), and each
row below links to it. A row changes when a chapter changes its level (below),
not when an issue closes, and the bar beside a level is drawn from the level
alone. A figure on this page names the command that recounts it. The open
questions the project researches, with the evidence for each, are on
[Research questions](../research/).

## Where the project actually stands

**Pre-alpha.** Single programs and multi-file projects on Linux build and run,
and the compiler is mostly written in Nikaia itself. The specification is ahead
of the compiler: several chapters are decided and written but only partly built,
and syntax and diagnostics still change between versions with no migration.

| Figure | Today | Recounted by |
| :--- | :--- | :--- |
| Toolchain written in Nikaia (self-hosting, [ADR-294](specification/adr/adr-294.md)) | 87.3 % | `python3 scripts/self_hosting.py` |
| Issues closed / open | 944 / 406 | [the tracker](https://github.com/Nikaia-Foundation/Nikaia/issues) |
| Change packages, one per [CHANGELOG](../CHANGELOG.md) heading | 1367 | `grep -c '^## \[' CHANGELOG.md` |

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
   file ([#993](https://github.com/Nikaia-Foundation/Nikaia/issues/993)). A lock taken inside a lock ([#677](https://github.com/Nikaia-Foundation/Nikaia/issues/677)) and two branches
   that update one shared value ([#985](https://github.com/Nikaia-Foundation/Nikaia/issues/985)) are refused since November.
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

The bar beside each level shows the same thing at a glance: the filled part is
what is built, the plain part the rest of what is known to be needed, and the
hatched part a reserve for what will turn up on the way, as it does in every
chapter. The bar follows from the level and is not a count.

| Chapter | Level | What stands | Open |
| :--- | :--- | :--- | :--- |
| **Part I** | | | |
| 2 Variables and data types | Proven <span class="rq-build rq-inline" role="img" aria-label="Proven"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 94%"></span></span><span class="rq-unknown" style="width: 10%"></span></span> | numbers typed by their uses, one text type ([ADR-282](specification/adr/adr-282.md)), `T?` and `null` | rounding of `/` and `%` unspecified ([#682](https://github.com/Nikaia-Foundation/Nikaia/issues/682)); `text::Pos` ([#743](https://github.com/Nikaia-Foundation/Nikaia/issues/743)); `f32` literals ([#728](https://github.com/Nikaia-Foundation/Nikaia/issues/728), decision) |
| 3 Control flow | Proven <span class="rq-build rq-inline" role="img" aria-label="Proven"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 94%"></span></span><span class="rq-unknown" style="width: 10%"></span></span> | `if`, `while`, `for`, `match`, `break`/`continue` | none known |
| 4 Data structures | Proven <span class="rq-build rq-inline" role="img" aria-label="Proven"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 94%"></span></span><span class="rq-unknown" style="width: 10%"></span></span> | structs, enums, lists, maps, sets, arrays; the collections offer one surface, `BTreeMap::entry` among it ([ADR-collections-1](specification/adr/adr-collections-1.md), [-2](specification/adr/adr-collections-2.md)) | type kinds ([#859](https://github.com/Nikaia-Foundation/Nikaia/issues/859)) |
| 5 Functions and arguments | Proven <span class="rq-build rq-inline" role="img" aria-label="Proven"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 94%"></span></span><span class="rq-unknown" style="width: 10%"></span></span> | `Subject ; Config`, lambdas, inferred `sync`; a parameter says `keep` or `mut`, a plain one is lent, `self` is lent and `keep self` consumes, a lent or `mut` `Array[T]` takes a piece of a list ([ADR-views-3](specification/adr/adr-views-3.md)) | the rest of views-3: `ref` refused outside `extern "C"`, a result's `from` everywhere ([#1213](https://github.com/Nikaia-Foundation/Nikaia/issues/1213)); several lent parameters in one result, enum variants that hold a view ([#1507](https://github.com/Nikaia-Foundation/Nikaia/issues/1507)) |
| 6 Memory and ownership | Rebuilding, **highest risk** <span class="rq-build rq-inline" role="img" aria-label="Rebuilding"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 50%"></span></span><span class="rq-unknown" style="width: 30%"></span></span> | views that outlive their buffer with no annotation (the tether, [ADR-283](specification/adr/adr-283.md)), text tiers, `mut` parameters and views ([ADR-views-1](specification/adr/adr-views-1.md)); views inferred for fields and results, and the migration that drops `ref` ([ADR-views-3](specification/adr/adr-views-3.md)); a struct that hands out views of text it read; each task's memory in a block of its own, and a full block ends only that task with `task::OverBudget` ([ADR-327](specification/adr/adr-327.md), first part) | the views-3 build ([#1213](https://github.com/Nikaia-Foundation/Nikaia/issues/1213), [#1507](https://github.com/Nikaia-Foundation/Nikaia/issues/1507)); most class-3 defects live here; the rest of the memory budgets: per shared value, weighed at a hand-over ([#490](https://github.com/Nikaia-Foundation/Nikaia/issues/490)) |
| 7 Error handling | Proven <span class="rq-build rq-inline" role="img" aria-label="Proven"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 94%"></span></span><span class="rq-unknown" style="width: 10%"></span></span> | `throws` with typed sets, `catch`, errors that keep their site | a code parameter's failures per call ([ADR-error-sets-1](specification/adr/adr-error-sets-1.md), specified) |
| 8 Concurrency | **Rebuilding** <span class="rq-build rq-inline" role="img" aria-label="Rebuilding"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 50%"></span></span><span class="rq-unknown" style="width: 30%"></span></span> | `async e` and `async { … }` groups, bounded by `limit:`, `within:` and `admission:`; a task that panics ends alone with `task::Crashed`; an executor at both `user-parallelism` settings; `spawn` and `overlap` still run beside it | handles and the free task ([#987](https://github.com/Nikaia-Foundation/Nikaia/issues/987), [#988](https://github.com/Nikaia-Foundation/Nikaia/issues/988)); failure order and stitched traces ([#989](https://github.com/Nikaia-Foundation/Nikaia/issues/989)); moving the programs over and removing `overlap`, `task::scope` and `spawn` ([#990](https://github.com/Nikaia-Foundation/Nikaia/issues/990), [#991](https://github.com/Nikaia-Foundation/Nikaia/issues/991), [#992](https://github.com/Nikaia-Foundation/Nikaia/issues/992)); all under [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983) ([ADR-async-5](specification/adr/adr-async-5.md)) |
| 9 Project organisation | Built <span class="rq-build rq-inline" role="img" aria-label="Built"><span class="rq-known" style="width: 85%"><span class="rq-done" style="width: 82%"></span></span><span class="rq-unknown" style="width: 15%"></span></span> | modules, `pub`, packages by path and by version (through crates.io) | no Nikaia package published on crates.io yet |
| **Part II** | | | |
| 10 Metaprogramming | Proven <span class="rq-build rq-inline" role="img" aria-label="Proven"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 94%"></span></span><span class="rq-unknown" style="width: 10%"></span></span> | grammars and `dsl` blocks, used by the `json` and `yaml` packages and the HTTP parser; build-time evaluation; no macros, on purpose ([ADR-298](specification/adr/adr-298.md)) | grammar defects [#1001](https://github.com/Nikaia-Foundation/Nikaia/issues/1001), [#1004](https://github.com/Nikaia-Foundation/Nikaia/issues/1004); streaming a grammar's input one item at a time (in the parser generator, not yet written by Nikaia, [#928](https://github.com/Nikaia-Foundation/Nikaia/issues/928)); specialising per build-time value ([#906](https://github.com/Nikaia-Foundation/Nikaia/issues/906)) |
| 11 Running code at once | Rebuilding <span class="rq-build rq-inline" role="img" aria-label="Rebuilding"><span class="rq-known" style="width: 70%"><span class="rq-done" style="width: 50%"></span></span><span class="rq-unknown" style="width: 30%"></span></span> | the same source on one thread or on a pool (`user_parallelism`), implicit pausing, `par_iter` with `limit:`, `within:` and `admission:` | the free task `async(detached)` ([ADR-async-5](specification/adr/adr-async-5.md), [#988](https://github.com/Nikaia-Foundation/Nikaia/issues/988)) |
| 12 Thread safety | Partial <span class="rq-build rq-inline" role="img" aria-label="Partial"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 60%"></span></span><span class="rq-unknown" style="width: 25%"></span></span> | `Shared`/`Locked`, `access_all`, no lock held across a pause (`NK2202`), no lock taken inside a lock (`NK2203`) | two branches that write one file ([#993](https://github.com/Nikaia-Foundation/Nikaia/issues/993)) |
| **Part III** | | | |
| 13 Toolchain | Built <span class="rq-build rq-inline" role="img" aria-label="Built"><span class="rq-known" style="width: 85%"><span class="rq-done" style="width: 82%"></span></span><span class="rq-unknown" style="width: 15%"></span></span> | `build`, `run`, `check`, `test`, `fmt` ([ADR-337](specification/adr/adr-337.md)), `describe`, `bind`, content-keyed cache; `lsp`, which shows the checker's messages and goes to a definition | `nikaia doc`; completion in the language server ([#103](https://github.com/Nikaia-Foundation/Nikaia/issues/103)) |
| 14 Testing and quality | Proven <span class="rq-build rq-inline" role="img" aria-label="Proven"><span class="rq-known" style="width: 90%"><span class="rq-done" style="width: 94%"></span></span><span class="rq-unknown" style="width: 10%"></span></span> | `test` blocks, output tests, `assert` proved while building (an unproved one refuses the program unless `unproved-claims = "warn"`, [ADR-380](specification/adr/adr-380.md)), contracts across packages; what the prover reaches is on [Research questions: the prover](../research/prover/) | tests of several files ([#926](https://github.com/Nikaia-Foundation/Nikaia/issues/926)); facts about data ([#797](https://github.com/Nikaia-Foundation/Nikaia/issues/797), [#798](https://github.com/Nikaia-Foundation/Nikaia/issues/798)) |
| 15 Interoperability | Partial <span class="rq-build rq-inline" role="img" aria-label="Partial"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 60%"></span></span><span class="rq-unknown" style="width: 25%"></span></span> | calling C (`extern`, libraries from `nikaia.toml`), calling a described Rust crate, a C library built from a package, Python and Node bindings ([ADR-284](specification/adr/adr-284.md)), a package as a Rust library | WebAssembly and bare metal ([ADR-119](specification/adr/adr-119.md)) |
| 16 Hardware instructions | Partial <span class="rq-build rq-inline" role="img" aria-label="Partial"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 60%"></span></span><span class="rq-unknown" style="width: 25%"></span></span> | vectors in `std::simd` | the rest of [ADR-371](specification/adr/adr-371.md) ([#755](https://github.com/Nikaia-Foundation/Nikaia/issues/755), decision; [#876](https://github.com/Nikaia-Foundation/Nikaia/issues/876)) |
| 17 Standard library and packages | Partial <span class="rq-build rq-inline" role="img" aria-label="Partial"><span class="rq-known" style="width: 75%"><span class="rq-done" style="width: 60%"></span></span><span class="rq-unknown" style="width: 25%"></span></span> | text, collections, files, I/O, sockets, processes, `std::regex` ([ADR-regex-1](specification/adr/adr-regex-1.md)), an HTTP/1.1 server that takes many connections at once and times out slow and quiet clients ([ADR-326](specification/adr/adr-326.md)); the packages `json` and `yaml` ([ADR-yaml-1](specification/adr/adr-yaml-1.md), in progress) | the package `db` and the SQL checked while building ([#116](https://github.com/Nikaia-Foundation/Nikaia/issues/116), [ADR-299](specification/adr/adr-299.md)); TLS through `rustls` ([#118](https://github.com/Nikaia-Foundation/Nikaia/issues/118)); HTTP/2; entries the toolchain's move needs ([#922](https://github.com/Nikaia-Foundation/Nikaia/issues/922)) |

### The walls you will hit

| You try to... | What happens |
| :--- | :--- |
| depend on a Nikaia package by version | found through crates.io as the crate `nikaia_<name>`, but none is published there yet |
| use a crate from crates.io | works after `nikaia describe <crate>`, whose output you review and commit |
| get completion or generated docs | not yet: `nikaia lsp` shows the checker's messages and goes to a definition, completion is to come ([#103](https://github.com/Nikaia-Foundation/Nikaia/issues/103)); no `nikaia doc`; [`editors/vscode`](../editors/) highlights syntax |
| talk to a database | only through its C library, as [`examples/sqlite`](../examples/sqlite/) does; the package `db` is being built ([#116](https://github.com/Nikaia-Foundation/Nikaia/issues/116)) |
| serve many connections at once | works, with deadlines for slow and quiet clients; HTTPS ([#118](https://github.com/Nikaia-Foundation/Nikaia/issues/118)) and HTTP/2 not yet |
| write the new `async` | works for `async e`, groups and `par_iter`, with `limit:`, `within:` and `admission:`; handles and the free task are being built ([#988](https://github.com/Nikaia-Foundation/Nikaia/issues/988)); `spawn` and `overlap` work until it replaces them |
| find a function you expect in `std` | sometimes missing; a missing entry is worth an issue |

Some errors still arrive in `rustc`'s words (class 3 above), pointed at your
`.nika` line.

## What comes next

The order is set by the score in [`priorities.md`](priorities.md). Current
direction, in short:

* **The specification first** (owner, 2026-11-03,
  [#1214](https://github.com/Nikaia-Foundation/Nikaia/issues/1214)): the
  compiler is brought to what the specification says before self-hosting goes
  on. A specification is only a claim until a compiler does what it says; with
  the two close together every rule can be tried in a program, and the concepts
  are shown to work rather than described. Built first: views as the
  specification states them
  ([#1213](https://github.com/Nikaia-Foundation/Nikaia/issues/1213)) and the
  task model ([#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983)):
  parser, checker, runtime and lowering of `async`, then `overlap`,
  `task::scope` and `spawn` are removed. Of views-3, the marks `keep` and
  `mut`, the lent `self`, the written `from`, the views of a run (`Array[T]`)
  and the migration that drops `ref` are built; refusing `ref` and several
  views in one result ([#1507](https://github.com/Nikaia-Foundation/Nikaia/issues/1507)) are next. Of the task model, `async e`,
  groups, `limit:`, `within:`, `admission:` and a crash that ends one task are
  built; handles and the free task are next. Grammar rules in traits
  ([#1020](https://github.com/Nikaia-Foundation/Nikaia/issues/1020)) belong to this round too.
* **Safety under load**: memory budgets per task and per shared value
  ([#490](https://github.com/Nikaia-Foundation/Nikaia/issues/490); the task block and `task::OverBudget` are built), TLS for the
  HTTP server through `rustls` ([#118](https://github.com/Nikaia-Foundation/Nikaia/issues/118)) and the database package with its
  SQL checked while building ([#116](https://github.com/Nikaia-Foundation/Nikaia/issues/116)). The server already takes many
  connections at once and times out slow clients.
* **Self-hosting after that** (owner, 2026-10-27): until the compiler holds no
  logic in Rust, every round moves code into Nikaia, largest files first. The
  road and what is left: [`bootstrap.md`](bootstrap.md),
  [`selfhosting-plan.md`](selfhosting-plan.md) and the issues.
* **Compiler defects** a move or a package runs into are fixed as they are met.
* **The `yaml` package**, also as a drop-in Rust crate, with streaming input
  ([#928](https://github.com/Nikaia-Foundation/Nikaia/issues/928)).
* **Research alongside**: how far the prover reaches with nothing written for
  it, and how far proofs carry a program towards the speed of the fastest code
  while it stays safe. The questions, the evidence and where each stands are
  on [Research questions](../research/).
* **Later, decided and not scheduled**: WebAssembly, bare metal, `nikaia doc`,
  completion in the language server.

Every design decision is argued in an [ADR](specification/adr/README.md) and
accepted by a human. The history of each change is in the
[CHANGELOG](../CHANGELOG.md).
