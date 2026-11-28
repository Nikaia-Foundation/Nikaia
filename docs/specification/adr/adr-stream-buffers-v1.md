# ADR-stream-buffers-v1: A Stream Reads Into a Fixed Pool of Buffers, Its Items View Them, and Every Place That Holds a View Is Counted While Building or Refused, So a Stream Copies Nothing and Never Runs Out of Buffers

**Status:** Accepted
**Date:** November 28, 2026 (owner, #2834)
**Answers:** how a program reads a file, a pipe or a connection too large to
hold in memory as a stream of items that view the bytes they were parsed from,
with no copy and no failure at run time; how ADR-grammar-typed-read-v2 D7's
"an item views the input" and "an item may be emitted" hold for such an input
([#2834](https://github.com/Nikaia-Foundation/Nikaia/issues/2834))
**Supersedes / Related:** [ADR-tether-v1](adr-tether-v1.md) (a view that
escapes keeps its buffer alive, by frame or by count; this adds a buffer that
may not be kept without a bound), D15 there (a held view is checked against the
buffer the plan names); [ADR-grammar-typed-read-v2](adr-grammar-typed-read-v2.md)
D7 (a `@stream` entry over a stream); [ADR-loop-failure-v1](adr-loop-failure-v1.md)
D4 (`io::lines()` yields views); [ADR-149](adr-149.md) (a channel is only
bounded, `send` pauses); [ADR-async-5](adr-async-5.md) D2 (what a task takes);
[ADR-037](adr-037.md) D3 (a count per build); [ADR-views-v4](adr-views-v4.md)
D5 (a view is inferred, never written)

## 1. Question

A program in a container reads a YAML or JSON stream of many gigabytes from
standard input and handles it event by event. It should hold a fixed amount of
memory, copy no text (an event's scalar is a view of the bytes it was read
from), and never fail or hang because of how it holds what it read. A view
keeps its buffer alive (ADR-tether-v1); a stream cannot keep every buffer it
read. **Which views of a stream's bytes may live how long, and who stops the
reading while they do?**

## 2. How others do it

| Who | How | Cost |
|---|---|---|
| Netty (Java), `bytes::Bytes` (Rust) | pooled buffers with a reference count; a slice keeps its buffer | a count per slice; a held slice pins its buffer silently |
| quick-xml, serde | borrowed events only from a slice in memory; a stream reader hands out owned events | a copy per event on the stream path |
| SAX (XML) | text handed over in pieces; the handler copies what it keeps | no bound on what the handler keeps |
| LMAX Disruptor | a ring of slots and a cursor per consumer; the producer waits on the slowest cursor | one consumer order, no counts |
| Perceus (Koka, Lean 4; Reinking, Xie, de Moura, Leijen, PLDI 2021) | precise counts: borrow without a count, fuse a dup with a drop, drop at the last use, reuse a freed cell, atomic only where shared | the analysis of ownership the compiler already does |

## 3. Options, weighed

**A, copy on the stream path** (#2735's owned form): simple, but every item of
the case that matters most is copied. **B, keep every buffer a view points
into** (tethering as for text in memory): memory grows with the input; a slow
consumer holds everything. **C, a fixed pool with a count per buffer and a wait
when it is full**: no copy and bounded memory, but a program that holds views
and waits for the next item while every buffer is held hangs. **D, C with every
place that holds a view counted while building**: the pool is sized from the
count, so it never runs out; the only wait is a full channel's, which another
task empties. **Chosen: D.**

## 4. Decision

### D1 — A stream reads into stream buffers from a fixed pool; an item views the buffer it was parsed from.

A stream is input read in order and not held whole: standard input, an open
file read as a stream, a connection's read side, the input of a `@stream`
grammar entry over one (ADR-grammar-typed-read-v2 D7), `io::lines()`
(ADR-loop-failure-v1 D4). It reads into a **stream buffer** taken from a pool
of buffers of one size. A view of one is a view like any other: nothing is
written, its state is in the ledger (ADR-views-v4 D5, ADR-tether-v1 D4).

### D2 — A view of a stream buffer lives one turn, or in a place counted while building; anything else is refused.

| where the view lives | compiler |
|---|---|
| used in the loop's body and dropped there (read, compared, cloned, ignored) | allowed: a plain borrow, no count |
| a fixed number of names across turns (`prev = ev`) | allowed: counts their number |
| a bounded channel to another task (ADR-149) | allowed: counts the channel's capacity and what the receiver holds |
| anything that grows across turns (`list.push(ev)`, a map of views) | refused, `NK2308` |

`NK2308`'s message names the place that grows and the three ways out: copy what
is kept (`.clone()`), keep only what is needed and process the rest in the
body, or hand the items to a task over a bounded channel. A receiver of such a
channel follows the same rule where it holds what it receives; a channel whose
receiver is the task that sends on it is refused (it is a list in disguise),
and so is one whose receiver waits for the stream it is fed from.

### D3 — The pool's size is the sum the compiler counted, so the pool never runs out.

The pool holds the buffers D2's counts need at once, plus the one being read,
plus the one an unfinished token is moved into (D8). The ledger names, per
stream, the places counted and the pool's size. No item waits for a buffer:
the parse waits only for input.

### D4 — Backpressure is the channel's: a full channel stops the sender until its receiver takes an item.

A body that sends on a full channel pauses (Part II 12.5); the parse of a
stream whose body pauses runs in a thread of its own (ADR-grammar-typed-read-v2
D7), where waiting is not a pause inside the parse (`NK2209` stands). A slow
receiver slows the reading; memory stays the pool.

### D5 — A count is the cheapest of four steps, and the ledger names it per loop.

1. **No count**: every view is used in the body (D2's first row).
2. **One cursor**: one receiver takes the items in order (a fixed number of
   names, a channel with one receiver): buffers come free in the order they were
   read, so one "used up to here" replaces the counts, as a ring does.
3. **A count per buffer, plain**: views come free out of order, in one thread.
4. **A count per buffer, atomic and fused**: several receivers in different
   tasks; the views of one buffer handed over together take one step.

A count is taken as Perceus takes one: no count for a borrow, a dup and its drop
cancelled where both are seen, the drop at the last use, a freed buffer reused
for the next read, atomic only where a view crosses a task (ADR-async-5 D2).
A buffer points to nothing, so no count can form a cycle.

### D6 — An item is never larger than a buffer: a longer text is handed over in parts.

Where an item's text can be longer than a buffer, the grammar hands it over in
parts, each a view of one buffer (yaml: `ScalarStart`, `ScalarPart`,
`ScalarEnd`). A part is emitted only where the parse cannot go back past it
(ADR-grammar-typed-read-v2 D7). Keeping all the parts of one text is keeping a
growing list, D2's last row. Comparing a text that may come in parts with a
literal, part by part with no copy, is `std`'s.

### D7 — The reader and the in-memory walk hand out the same item type.

An item over a stream and an item over a `String` or `Bytes` are one type; only
the buffer behind the views differs, and nothing in the source says which
(ADR-views-v4 D5). An item handed to the Rust side carries a counted handle on
its buffer, so it is `'static` without its text copied; the Rust reader of
ADR-grammar-typed-read-v2 D7 hands out views too.

### D8 — A token cut by the end of a buffer is moved to the start of the next.

The unfinished token's bytes, at most the longest token the grammar can leave
unfinished (yaml: an implicit key, at most 1024 characters, YAML 1.2 §7.4.2),
move to the start of the next buffer before it is filled. It is the engine's
move, of no item and of no data the program sees. **Rejected:** tokens that
span buffers (every rule pays for the seam).

### D9 — Nothing about the pool fails at run time.

A stream fails at run time only for its input: a read that fails (the loop
`throws`, ADR-loop-failure-v1 D1), a parse error, a limit of the format
(ADR-yaml-1 D8). The pool does not run out (D3), a wait is a full channel's
(D4), and an item fits a buffer (D6).

**Rejected:** a copy on the stream path (#2735's owned form: superseded);
keeping every buffer a view points into (unbounded); waiting when the pool is
full (a program that holds views and waits for the next item hangs); a pool
that grows (unbounded, an out-of-memory end from outside); copying when the
pool is full (a copy the program did not ask for, ADR-tether-v1 D3).

## 5. Consequences

* A program reads a stream of any size in the memory of its pool, and an item's
  text is a view; what it keeps longer is copied where it says `.clone()`.
* Collecting the items of a stream into a list is refused; collecting copies,
  or processing in the body, or handing over to a task is the way.
* The engine's emit sink holds counted handles, so an emitted item is
  `'static` on every path without `unsafe` (#2834's defect).

## 6. What this does not decide

* The `std` surface of a byte stream (`io::Stream` from standard input, an open
  file, a connection): proposed by its build issue, decided by the supervisor.
* The pool's buffer size and whether a program may name it.
* Whether a part-wise text comparison is a method or a pattern.
