# ADR-grammar-lowering-v1: A Grammar Is Lowered Name for Name as a Macro Invocation, Guesses No Meaning, Leaves `@frame`'s Check to the Backend, and Is Made Fast Only Where a Measurement Says So

**Status:** Accepted
**Date:** November 19, 2026 (the consolidation, #904; the decisions themselves October 3 to October 20, 2026)
**Answers:** what the lowering of a grammar onto the parser backend is and
refuses to do, how `@frame` reaches the backend, what a fold's step calls is
written as, and what a map lowers to
([#433](https://github.com/Nikaia-Foundation/Nikaia/issues/433),
[#695](https://github.com/Nikaia-Foundation/Nikaia/issues/695),
[#904](https://github.com/Nikaia-Foundation/Nikaia/issues/904))
**Supersedes / Related:** takes the decisions on how a grammar is lowered from
[ADR-296](adr-296.md) (its other decisions stay there);
[ADR-001](adr-001.md) D2 and D4 (the parser backend; Stage 0 is a transpiler);
[ADR-013](adr-013.md) D2 (the fold adapter is a lookup in the program's
declarations); [ADR-300](adr-300.md) (errors reported against the `.nika`
line); [ADR-370](adr-370.md) D2 (an inlining hint written where a measurement
shows it); [ADR-grammar-parallel-v1](adr-grammar-parallel-v1.md) D1 (what
`@frame` declares and what is checked of it); Part I (a map's order is not
promised)

## 1. Question

**What does the lowering of a grammar onto the parser backend write, what does
it refuse to infer, and where may it depart from writing the program name for
name to make the parse fast?**

The parser backend is a Rust crate, and Stage 0 is a transpiler
([ADR-001](adr-001.md) D2, D4), so every grammar reaches the language below as
generated Rust. Without the answers the emitter could invent semantics where
Nikaia and Rust disagree about a name, check a frame twice with two verdicts
that drift, and leave a measured hot path to the backend's heuristics.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| `winnow-grammar` (the backend) | a `grammar! { … }` macro; `#[frame(boundary = …)]` checked by the backend | none beyond the backend's | its ADRs; [ADR-001](adr-001.md) D2 |
| The fast reference implementations of the one-billion-rows workload | hand-inlined hot paths; a map keyed by a name compared a word at a time | the speed lives in hand-written code nobody checks | D4, D5 |
| Rust's `HashMap<&str, V>` | `&str ==` always calls `memcmp` | 31.5 instructions a row for 1BRC's name (#695) | D5 |

What transfers: the backend's macro and its frame check are used as they are
(D1, D3). What does not: hand-written speed in user code; the compiler writes
the hint and the map where it measured a gain (D4, D5).

## 3. Options, weighed

| Option | Why it lost | See |
| :--- | :--- | :--- |
| Inferring accumulator threading from a step's shape | `acc.merged(m)` has the shape of `acc.record(m)` and would be mis-lowered | D2 |
| A second frame check in Nikaia | two verdicts drift; the provenance fix is spans | D3 |
| `#[inline]` on a fold step's callees | whether LLVM honoured it depended on how the codegen units were cut | D4 |
| Every function inlined | bloats the code and the build | D4 |
| A second map type only for text keys | a `Map[String, V]` given to a generic function would be a different Rust type, and rustc would refuse it | D5 |
| Keys stored inline in the slot | the views `keys()` and `drain` hand back could not outlive the map; no gain measured | D5 |

## 4. Decision

### D1 — A grammar reaches the parser backend as a macro invocation.

The lowering emits `grammar! { … }`, name for name with the `.nika` file. A
`grammar` item is never dropped silently.

### D2 — The lowering is syntactic, and never guesses a meaning.

Every action, every `init`/`step`/`merge` and every body is emitted as
written, name for name. Where Nikaia and Rust disagree about a name, the
emitted program fails to compile rather than the printer inventing semantics;
the failure is positioned on the `.nika` line ([ADR-300](adr-300.md)). An
accumulator is never threaded from the shape of a step's tail: a step that
calls a `&mut self` method returning nothing has its accumulator threaded by
the emitter, found by a lookup in the program's declarations
([ADR-013](adr-013.md) D2), and a step whose method returns a new accumulator
is left as written.

### D3 — `@frame` lowers key for key, and Nikaia does not re-implement its check.

`@frame(boundary: "\n")` becomes `#[frame(boundary = "\n")]`, `unchecked`
passes through, and the boundary keeps its escape text. The check that a
boundary cannot occur inside a frame lives once, in the backend: two
implementations would drift, and the harmful drift is Nikaia accepting what
the backend rejects.

### D4 — What a fold's step calls directly is written `#[inline(always)]`. *(The first case of [ADR-370](adr-370.md) D2.)*

The step is the loop body over the input, so it runs once per item, and what
it calls usually has that one call site. So a function or method a `fold`'s
or a `par_fold`'s step calls is written `#[inline(always)]`, found by name,
because the step's receiver is the accumulator, whose type this emitter does
not know (D2). Only here: every function inlined would bloat the code and the
build.

Measured on 1BRC: LLVM left `Summary::record` out of line, because hashbrown's
insert path makes it too big for its heuristic, and no profile setting changed
that (16 or 1 codegen unit, thin LTO: 319.1 instructions a row; fat LTO 317.0);
with the hint, 300.1, the same output
([#433](https://github.com/Nikaia-Foundation/Nikaia/issues/433)). `#[inline]`
alone was not enough: with rustc 1.97, at one unit `record` was inlined but
hashbrown's `entry` stayed a call, at sixteen the other way round, 274 against
231 instructions a row
([#695](https://github.com/Nikaia-Foundation/Nikaia/issues/695)); with
`always` the callee's one call site, the loop's, decides it and no heuristic
of the backend's does.

### D5 — A map is `std`'s own type over hashbrown's table; text keys are hashed and compared a word at a time.

Every Nikaia map lowers to one type of `std`'s own, built on
`hashbrown::HashTable`, with the hash and the equality `std` chooses per key
type. For a key that is text (a `&str` view or an owned `String`) both read the
key a word at a time through its pointer: the hash from its first one or two
words and its length, the equality word by word and then the tail, with no call
of `memcmp`. Which key type it is, is known when compiling (a type-id test the
optimiser removes), so one type serves every key and a map handed to a
generic function stays the same Rust type. Keys are not copied into the slot:
`keys()`, `drain` and a `Map[ref String, V]` hand back views that outlive the
map, and counting instructions shows no gain from an inline copy. A map's
order is not promised (Part I), so the order a different hash gives is no
change of meaning.

In 1BRC's plain program the name's `memcmp` cost 31.5 instructions a row and
its hash 27.1 (#695's attribution): Rust's `&str ==` always calls `memcmp`,
which hashbrown's map with a `&str` key cannot avoid.

## 5. Consequences

* The emitter stays a printer (D1-D3): a name it cannot place is the
  backend's error, positioned on the `.nika` line by [ADR-300](adr-300.md)
  and worded in Rust's words - bearable because the lowering is name for name.
* A frame is checked once, by the backend; what Nikaia checks of what a frame
  reaches is [ADR-grammar-parallel-v1](adr-grammar-parallel-v1.md) D1.
* The lowering departs from name for name only where a measurement shows a
  gain and the output stays the same (D4, D5); what pays is the build, which
  inlines a fold step's callees.

## 6. What this does not decide

* What a fold's lambdas' *types* are, and whether the rule's declared type
  reaches the fold ([ADR-grammar-parallel-v1](adr-grammar-parallel-v1.md) D9;
  [ADR-023](adr-023.md)).
* Inlining hints outside a fold's step: [ADR-370](adr-370.md).
