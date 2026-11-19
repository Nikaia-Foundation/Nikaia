# ADR-grammar-parallel-v1: A Parse Is Cut in Parallel Only Where a Checked `@frame` Says It May, and Folded by a Written `par_fold` Whose Lambdas Are Ordinary Code

**Status:** Accepted
**Date:** November 19, 2026 (the consolidation, #904; the decisions themselves October 3 to October 19, 2026)
**Answers:** where a format assumption may be written so that a parse can be
cut and run on several cores, what the compiler checks of it, how the cut is
lowered and how fast a grammar scans, what `user_parallelism` changes, and how
a fold's `init`, `step` and `merge` are checked
([#237](https://github.com/Nikaia-Foundation/Nikaia/issues/237),
[#904](https://github.com/Nikaia-Foundation/Nikaia/issues/904))
**Supersedes / Related:** takes the decisions on frames and folds from
[ADR-296](adr-296.md) (its other decisions stay there);
[ADR-372](adr-372.md) D4-D6 (pieces claimed while the work runs, `order:`,
`piece:`, the same for `Par::fold`); [ADR-037](adr-037.md) D1 (a switch selects
a build, not a dialect); [ADR-014](adr-014.md) (the mapping and the parallel
driver); [ADR-013](adr-013.md) (the fold adapter left to a typing pass);
[ADR-023](adr-023.md) (the grammar's bindings); [ADR-276](adr-276.md) D18 (the
lowering's backstop for a jump); [ADR-371](adr-371.md) (a library's portable
lanes); [ADR-grammar-syntax-v1](adr-grammar-syntax-v1.md) D9 (where a pattern
ends); Part II 10.7

## 1. Question

**Where may a grammar say that its input can be cut blindly and parsed on
several cores, what does the compiler check of that claim, and how are the
pieces folded back into one result?**

A grammar is already the file's specification, so it is the one place a format
assumption may be written so that a parse can be cut and run in parallel.
Without the answers the compiler cannot verify a frame, a result could depend
on the number of cores, a scan falls back to one byte at a time, and a fold's
lambdas escape the checks every other piece of code meets.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| The fast reference implementations of the one-billion-rows workload | split the buffer blindly, repair the seam; find the separator a word at a time | the assumption about the format lives in hand-written code nobody checks | D3, D4 |
| `winnow-grammar` (the backend) | `#[frame(boundary = …)]`, checked by the backend; a piece driver over `Parallelism` | none beyond the backend's | its ADRs |

What does not transfer (as a construct of the language; a program may read
words and scan by hand, ADR-320 D12): hand-written SWAR in user code, which is
unreadable and unportable - the grammar states the assumption and the compiler
exploits it (D4).

## 3. Options, weighed

| Option | Why it lost | See |
| :--- | :--- | :--- |
| `sync rule` as the frame marker | `sync` already has two meanings | D1 |
| Rewriting `until(";")` reached from a frame into *until `;` or newline* | the same rule text would parse differently depending on what reached it | D1 |
| Inferring the boundary without a marker | an unrelated edit would silently stop a grammar being parallel | D1 |
| `split_aligned(n, "\n")` by hand | asks the user to restate what the grammar knows and to write the pipeline that follows | D2 |
| Parallelising any associative-looking `fold` | a merge over `f64` is not associative; output would depend on the core count | D2 |
| A user-facing SWAR/SIMD construct | hardware instructions live behind a backend DSL (ADR-grammar-syntax-v1 D20); a library's portable lanes and safe instruction functions are [ADR-371](adr-371.md)'s | D4 |
| A switch that changes more than the executor | two settings that mean different things are two languages | D5 |
| A second walk over a fold's lambdas | two walks over one construct are two chances to disagree, and the old one gave wrong advice | D10 |

## 4. Decision

### D1 — `@frame` declares a resynchronisation unit, and the compiler checks it; it never rewrites the grammar.

```nika
rule NAME -> ref String = s:until(";" | frame_end) { s }

@frame(boundary: "\n")
rule MEASUREMENT -> Reading = name:NAME ";" => temp:TENTHS frame_end { Reading { name, temp } }
```

A frame can be found from any offset by scanning to its boundary. The frontend
walks everything the rule reaches; **safe**: a literal without the boundary, a
built-in whose alphabet excludes it, lookahead, an `until(t)` whose terminator
covers the boundary (`frame_end`, the boundary literal or a prefix of it,
`line_ending` for a newline); **rejected**, naming rule and pattern: a literal
containing it, a built-in that can consume it (`any`, `multispace0`), the
implicit whitespace of a syntactic rule, an `until` that does not cover it
(the message says `until(… | frame_end)`), and `recover(…)` - recover per
frame instead, `(item | until(frame_end)) frame_end`. `frame_end` names the
boundary of the frame the rule is reached from and resolves statically. The
attribute is keyed - `@frame`, `@frame(boundary: "\n")`,
`@frame(boundary: "\n", unchecked)` - and `unchecked` is the greppable
assertion for formats a byte string cannot cut (quotes, start markers,
escapes), each a future key. A frame ends in its boundary. A syntactic
(lowercase) rule the frame reaches skips whitespace before each element, so it
is refused unless the grammar declares its own `WS`, which is then checked as
part of what the frame reaches. The `par_fold` rule that folds the frames
skips no whitespace whatever its name - not at its entry, not at its start,
not between frames - so whitespace between frames is not a frame and fails at
any chunk count (D2).

### D2 — Parallel parsing is a frame plus a monoid: `par_fold`, written, never inferred.

`par_fold(ITEM, init, step, merge)` is `fold` plus a merge, over an `@frame`
item, and the whole body of its rule; the split, seam repair, per-chunk
accumulators and merge are generated, and the program writes
`Measurements::file(data)`. A written `par_fold` says *results from different
chunk counts are the same to me*. Chunk count changes neither what is accepted
nor what is rejected: a `par_fold` rule is lexical in all but name (D1). At
`user_parallelism = no` it is a sequential `fold` - same accumulator, same
merge, same result - which keeps it compiling for `wasm32`.

### D3 — The lowering cuts blindly, then repairs the seam.

Pieces of the input are claimed while the work runs, not cut into one range
per worker first; how large a piece is, the runtime decides, bounded by the
option `piece:` ([ADR-372](adr-372.md) D4, D6), over `task::cores()` workers
by default. Every piece but the first advances past the first boundary at or
after its start, so each frame is owned by exactly one piece, the one that
finds its end; the last runs to EOF, an unterminated last frame is still
yielded and no empty trailing frame is; a frame longer than a piece is not an
error; errors carry absolute offsets, and under `recover` one piece's failure
does not abort the others; the number of pieces does not change results and is
overridable for benchmarking. How accumulators are kept and merged, the
option `order:` decides (ADR-372 D5). The driver takes
`Parallelism::Off | Pieces(n) | Auto`, `Off` identical to the plain parser,
and the cut and merge are public so the pieces can run on Nikaia's own pool.

### D4 — Scanning is a specified complexity: never one byte at a time.

A boundary or `until` terminator is found by a portable word-at-a-time (SWAR)
scan - no target feature, works on `wasm32`; SIMD is a layer on top, never the
baseline. Up to three fixed alternatives are one pass. The same primitive
serves `recover`'s skip.

### D5 — `user_parallelism` chooses the `Parallelism`, and nothing else.

A call to a `par_fold` rule lowers to the piece driver with `Parallelism::Auto`
or, at `no`, the identical call with `Parallelism::Off`; the two programs
differ in one token. A switch selects a build, not a dialect
([ADR-037](adr-037.md) D1). A rule that is not a `par_fold` gets its own
parser, run once over the whole input.

### D6 — A rule that is a `par_fold` gets its binding from the compiler; every other alternative carries an action.

A `par_fold` is the whole body of its rule, so its value is the rule's and
nothing is written after it. Everywhere else an action is required, and the
pattern ends where its action block begins (ADR-grammar-syntax-v1 D9).

### D7 — A fold's `init`, `step` and `merge` are walked by the ordinary expression walk.

They are expressions inside a pattern; they get every rule the language has -
`NK1117`, `NK1132`, `NK1128`, and whatever comes next.

### D8 — Inside the frame the rule's own pattern makes.

A binding beside the fold in the same sequence is in scope in its lambdas.

### D9 — Outside `expected`.

The rule's declared type is what an action builds; a fold's `init` and `step`
build the backend's running value, and holding them to the declared type would
refuse the arithmetic every fold is made of.

### D10 — One walk, not two.

A jump in a fold's step meets `NK1132` through the lambda path every other
lambda takes - *the nearest loop is outside this lambda* - and no second walk
of the fold's own exists beside it. [ADR-276](adr-276.md) D18's backstop in the
lowering stays. Measured: nothing in the corpus is newly refused.

## 5. Consequences

* A reader learns frames and `par_fold` from Part II 10.7.
* A parse that can be cut is one the compiler has checked can be cut, and its
  result is the same at any number of cores and at `user_parallelism = no`;
  what pays is the grammar author, who writes `@frame` and `merge`, and a
  format a byte string cannot cut, which says `unchecked`.
* A fold's lambdas are read by every analysis as any other lambda: names (D7),
  scope (D8), jumps (D10).
* Accepted cost: a fold's lambdas are not held to the rule's declared type
  (D9).

## 6. What this does not decide

* Chunk-count policy beyond `task::cores()` (NUMA, heterogeneous cores, a
  container's CPU quota), and `par_fold` over a stream rather than a mapped
  buffer.
* Further `@frame` keys (`quote`, `start`, `escape`, `scan`) and inferring a
  boundary without a marker.
* What a fold's lambdas' *types* are, and whether the rule's declared type
  reaches the fold (D9; [ADR-023](adr-023.md)).
