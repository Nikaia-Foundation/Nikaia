# ADR-regex-1: `std::regex` Is the RE2 Dialect; a Regular Expression Is Typed by Its Engine, a Pattern the Build Knows Is Compiled Down a Ladder of Engines and Run by `std`'s Code Specialised for It, a Pattern That Arrives at Run Time Runs on Rust's Automata Crates

**Status:** Accepted
**Date:** October 28, 2026
**Answers:** whether and how `std` offers regular expressions ([#673](https://github.com/Nikaia-Foundation/Nikaia/issues/673)): which dialect, what type a regular expression has, what the build does with a pattern it knows, which engine runs it, and what a pattern that arrives at run time runs on
**Supersedes / Related:** [ADR-specialization-1](adr-specialization-1.md) (`comptime?`, `@unroll`, branches decided while building, a result type computed from a `comptime` parameter), [ADR-374](adr-374.md) (a position in a text, `text::Match`), [ADR-375](adr-375.md) (`ByteSet`, `Seq`, positions as facts), [ADR-377](adr-377.md) (`std::simd`), [ADR-296](adr-296.md) D7 (a grammar has no regex terminals), D42 (a `std` type over a crate's table), [ADR-318](adr-318.md) D5-D6 (a `std` type crosses into a `comptime` by its `constant` column; a `std` function runs at build time), [ADR-311](adr-311.md) (a build-time value crosses in its view form), [ADR-333](adr-333.md) D3 (a `comptime` parameter), [ADR-287](adr-287.md) (`comptime`), [ADR-295](adr-295.md) (a trait is not a type), [ADR-218](adr-218.md) (`unsafe` lives in `crates/unsafe`), [ADR-188](adr-188.md) (a backslash is written `\\`)

## 1. Question

```nika
comptime DATE = regex::compile("(\\d{4})-(\\d{2})-(\\d{2})")

fn year(line: String) -> String? {
    let c = DATE.captures(line) ?? return null
    return c.group(1)?.text().clone()
}
```

A regular expression is a pattern for a shape of text; an engine finds where
the shape occurs and which parts matched which group. `std` has `find`,
`split`, `ByteSet` and `grammar`, and no engine. A `grammar` is a parser:
ordered choice and a repetition that never gives back what it took, so the
pattern `a*a` accepts `aaa` as a regular expression and never as a grammar
rule.

A library sees a pattern when the program runs. A compiler sees most patterns
while it builds - they are literals - and can do what no library can: choose
an engine for the pattern's shape, emit code made for it, and leave nothing to
do when the first text arrives.

## 2. How others do it

| | when the pattern is compiled | engines | groups |
|---|---|---|---|
| Perl, Python, Java, PCRE | run time | backtracking, exponential in the worst case | yes, with backreferences |
| RE2, Go | run time | DFA built lazily, NFA simulation, literal prefilter | NFA simulation or one-pass |
| Rust `regex` (`regex-automata`) | run time | literal prefilter, full DFA only for small patterns (by default up to 30 NFA states and 40 KiB), else lazy DFA, else Pike VM | Pike VM, one-pass or bounded backtracking over the matched span |
| re2c, logos | build time | DFA emitted as code | re2c: tagged DFA (Laurikari), registers written on transitions |
| C++ CTRE | build time (templates) | a matcher instantiated per pattern; the pattern is part of the type | yes |
| Hyperscan | ahead of the scan, into a database | literals with SIMD, DFAs, bit-parallel NFAs in vector registers | no |

What `regex-automata` 0.4 offers to a build, read in its source: a dense or
sparse DFA serializes to bytes and is used from a borrowed slice of `u32`
words without a copy (`from_bytes`, which validates, or `from_bytes_unchecked`,
which reads only the header); the Thompson NFA, the Pike VM, the one-pass DFA,
the lazy DFA and `meta::Regex` do not serialize. The NFA's states refer to each
other by index; its transition lists, its names and its handle are separate
heap blocks, so it cannot be laid down as bytes and used in place.

## 3. Decisions

### D1 — The dialect is RE2's, as Rust's `regex` crate writes it.

A pattern has no backreference and no lookaround. A search takes time
proportional to the text times the pattern, whatever the pattern is. A match is
the leftmost one; among those, the one the pattern prefers (alternatives in
the order written, a repetition greedy unless marked lazy). Text is matched by
Unicode scalar; `\d`, `\w` and `\s` are Unicode classes, and `(?-u)` turns that
off for a class that means ASCII. `(?:…)` is a group that does not capture.

**Why.** A pattern a program receives at run time must not be able to stop the
program, and every user already knows this dialect. **Rejected:** a
backtracking engine (exponential time on a pattern an input chooses); a dialect
of our own.

**Cost, named:** a pattern that needs a backreference is written in code.
Adding the feature later never changes a pattern that works.

### D2 — A pattern is a plain string, and the backslash is written `\\`.

`"\\d{4}"`, as Part I 2.5 already prints it. A raw-string form is not decided
here ([ADR-188](adr-188.md) §4).

### D3 — A regular expression is `regex::Regex[E]`, typed by its engine; a collection of different ones is `regex::Any`.

`E` is the engine, a type implementing the trait `regex::Engine`: one per
stage of D6 (`regex::Literal`, `regex::OnePass`, `regex::Dfa`,
`regex::TaggedDfa`, `regex::BitNfa`, `regex::FlatNfa`) and `regex::Dynamic` for
a pattern compiled at run time (D8). Each engine's calls are its own; a call
chooses no engine while the program runs. A function over any regular
expression is generic, `fn f[E: regex::Engine](re: regex::Regex[E])`, and is
emitted once per engine. `regex::Any` is an `enum` with one variant per engine,
`re.any()` makes one, and its calls choose the engine at run time - the one
place where that happens, and only where the program writes it.

| call | gives |
|---|---|
| `re.is_match(s)` | `bool` |
| `re.find(s; from: text::Pos)` | `text::Match?` |
| `re.find_all(s)` | `Seq[text::Match]`, left to right, not overlapping |
| `re.captures(s; from: text::Pos)` | `regex::Captures?` |
| `c.group(i)`, `c.named("year")` | `text::Match?` (group 0 is the whole match) |
| `re.replace_all(s, with)` | `String`; `with` may name groups as `$1` or `${year}` |
| `re.split(s)` | `Seq[ref String]`, the pieces between matches |

Every position is a `text::Pos` of the searched text ([ADR-374](adr-374.md)),
and a match is `text::Match`, so `before`, `after` and `between` cut with it.
A `Regex` is read-only and shared between threads. It searches a `String`;
matching a `Bytes` is not decided here.

**Two ways in.** `regex::compile(comptime pattern) -> regex::Regex[E]` takes a
pattern the build knows; the build chooses `E` from the pattern
([ADR-specialization-1](adr-specialization-1.md) D7). A type written where the
result is received is a contract: `comptime DATE: regex::Regex[regex::Dfa] =
regex::compile("…")` is refused when the pattern needs another engine.
`regex::dynamic(pattern) -> regex::Regex[regex::Dynamic] throws regex::Error`
takes a pattern known only at run time. `regex::compile` with a pattern that is
not a build-time value is refused at the call (ADR-333 D3), and the help names
`regex::dynamic`.

**Why.** Which engine a pattern needs is a fact the build knows; held in the
type, it costs nothing while the program runs, and engines added later are
types added, not cases added. **Rejected:** one type whose calls `match` on
the engine (a branch on every call, also where every value has the same
engine); a list of values typed by the trait (a trait is not a type,
[ADR-295](adr-295.md)); one `compile` for both stages (its result's type would
depend on whether a caller wrote `comptime`); the pattern itself as the type's
parameter, `Regex[P]` (every pattern a different type).

### D4 — A pattern the build knows is source code: the build compiles it, and `std`'s code runs specialised for it.

`regex::compile(p)` with a build-time text - a literal, a text composed of
other `comptime` values, `asset("date.re")` - is a program in the pattern
language. The build parses and checks it (a pattern that does not parse stops
the build with the engine's own message, `NK1244`), chooses its engine (D6) and
lays down its tables (D7). The code that runs is `std`'s own, written once in
Nikaia: the calls of D3 take `comptime? self`, so a call on a known pattern
calls a copy made for that pattern, in which the engine's states are constants
and its loops over them are unrolled
([ADR-specialization-1](adr-specialization-1.md) D1-D4). A pattern known only
when the program runs is input, compiled by `regex::dynamic` (D8).

**Why.** [Part II 10.1](../20-nikaia-advance.md) forbids code nobody wrote; the
pattern is code the user wrote, in another notation, and the matcher is code
`std` wrote, specialised for it by the rules every library has. **Rejected:**
a case for regular expressions in the compiler's lowering (the next library
with a pattern language would need its own); treating a known pattern as a
value compiled at run time (it repeats work the build could do, every time the
program starts); tables only, never code (it gives away the fastest form for
the small patterns that are most of them).

### D5 — What the engine is held to.

1. **The matches the specification defines** (D1). Rust's crate, which runs
   `regex::Dynamic` (D8), is the oracle: a pattern compiled by `compile` and by
   `dynamic` gives the same matches on the same text, checked by a
   differential test in Nikaia and by fuzzing. A crate version that disagrees
   with the specification is a finding, not the new meaning.
2. **Linear** in the text, with **bounded memory**, for every pattern.
3. **Nothing at run time that the build could do** for a pattern the build
   knows: no parsing, no compiling, no check of a table, as far as the stage
   allows no allocation (D7).
4. **At least as fast as the crate**, faster where knowing the pattern while
   building pays, measured as `CLAUDE.md` *Measuring performance* says.
5. **The pattern decides what is built.** A pattern without a capturing group
   gets no group machinery; group 0, the match's start and end, is always
   there. Which calls a program makes on a `Regex` is not read.
6. **Visible**: the type names the engine; the report says the rest (D9).
7. **Bounded in binary size and build time**, by limits the build can be given.

### D6 — A known pattern takes the cheapest engine that fits it.

| stage | engine | for | what runs |
|---|---|---|---|
| S0 | `Literal` | a literal, or a few alternatives of literals | `find`, `ByteSet`, a literal search; no automaton |
| S1 | `OnePass` | a pattern with one way on at every byte | straight-line code, groups included, no set of states |
| S2 | `Dfa` | a DFA | under the unroll limit, the DFA as code: a block per state, a comparison per byte range, a state that loops on a set of bytes skipping them with `ByteSet`; over it, or for a pattern that reached the call through a run-time value, the DFA's table, read in place (D7) - one engine, `@unroll(up_to: N)` deciding (ADR-specialization-1 D3) |
| S3 | `TaggedDfa` | capturing groups, not one-pass, a tagged DFA under its limit | a DFA whose transitions write positions into registers; no NFA at run time |
| S4 | `BitNfa` | up to as many NFA positions as the widest vector the build has | one bit a position, a step a few word or vector operations per byte (Glushkov, as Hyperscan) |
| S5 | `FlatNfa` | anything larger | a flat NFA read in place, run by an NFA simulation and a lazily built DFA of our own |

The engine is chosen by the pattern and the limits alone. The limits are
numbers the build can be given; their defaults are set by measurement. A stage
not yet built falls to the next one that is; under all of them is
`regex::Dynamic` built at run time from the already checked pattern, which is
not a stage but the floor.

**Why.** The crate's own default says a full DFA is a small-pattern
optimisation; the expensive cases - a DFA that grows with every position
(`(a|b)*a(a|b){20}`), groups that are not one-pass - are where an NFA is
simulated at run time, and they are also where knowing the pattern ahead
helps most: a tagged DFA removes the NFA from group extraction, a bit-parallel
NFA turns the simulation into word operations. A lazily built DFA, the crate's
answer to size, builds the states the text touches while the program runs; it
stays as the last stage, not as the answer.

### D7 — What the program reads lies in the binary in the form it is used in.

A table crosses as an aligned `ref Array[u32]` ([ADR-311](adr-311.md)) in the
target's byte order and is read where it lies. The crate's DFA is opened with
`from_bytes_unchecked`, which reads its header and nothing else; the bytes come
from the same build with the same crate, and the header carries its version.
The call is `unsafe` and lives in a crate of its own under `crates/unsafe`
([ADR-218](adr-218.md)). An engine of our own (S3-S5) is laid out flat: states
as `u32` words, references as indices into the same table, names in one byte
run with offsets - nothing that must be rebuilt, checked or relocated before it
is read. The crate's NFA is not used for a known pattern: it is heap blocks
joined by pointers and cannot lie in the binary.

**Why.** D5.3. A table that must be rebuilt or validated on first use is work
the build has already done. **Rejected:** replaying the crate's NFA through its
builder at run time (a pass over every state, each time the program starts);
dumping the crate's NFA's memory (its blocks hold addresses of the process
that built them, its layout is not promised, and its handle writes a count).

### D8 — A pattern that arrives at run time runs on Rust's automata crates.

`regex::Regex[regex::Dynamic]` wraps `regex-automata`'s `meta::Regex` (with
`regex-syntax`), as `std`'s maps wrap `hashbrown`'s table
([ADR-296](adr-296.md) D42). No type of the crate appears in a program, in the
ledger or in a message. The crate is pinned in `Cargo.lock`; a version that
changes what a pattern matches is caught by D5.1 before it is taken.

**Why.** A run-time pattern has nothing to gain from the build; the crate is
the tuned answer for it, and the oracle the engines of our own are checked
against.

### D9 — `--comptime` says what each known pattern became.

For each pattern the build compiled: its engine, the sizes (DFA bytes, NFA
states, tagged registers), which limit sent it further down, whether a group
stage was built because the pattern has capturing groups, and the prefilter;
for each call, the copy it calls or why it calls the ordinary function
([ADR-specialization-1](adr-specialization-1.md) D8). A pattern on the floor
is listed as such.

**Why.** A pattern changes engine silently when it grows; the type and the
report are where that is seen.

### D10 — The crates understand and translate a pattern; for a known pattern, the code that runs is ours.

| part | where it runs | by whom |
|---|---|---|
| parsing a pattern, its error message | build; run time for `dynamic` | `regex-syntax` |
| Unicode classes | build; run time for `dynamic` | `regex-syntax` |
| literals for a prefilter | build | `regex-syntax` |
| the NFA | build | `regex-automata`; read through its public states into our flat form |
| determinising and minimising a DFA | build | `regex-automata` |
| whether a pattern is one-pass | build | `regex-automata`'s one-pass builder, which fails on a pattern that is not |
| S0 prefilter | run time | `find`, `ByteSet`; `aho-corasick`'s Teddy for many literals |
| S1, S2 as code | run time | `std`, specialised per pattern |
| S2 as a table | run time | `regex-automata`'s search over its own table, until a measurement says ours pays |
| S3-S5 | build and run time | `std` |
| a run-time pattern | run time | `regex-automata`'s `meta::Regex` |

**Why.** The crates define the dialect (D1) and do years of work on parsing,
Unicode and determinising that a build runs once; what runs per byte for a
pattern the build knows is where knowing it pays (D4). A parser of our own
would have to match the crate's in every flag and message, and could only
differ from it.

## 4. What this does not decide

* **Typed groups.** `dsl regex::Regex { (?P<year>\d{4}) } eod` giving a result
  struct with a field per named group is a record of its own.
* **Regex terminals in a `grammar`** stay a goal ([ADR-296](adr-296.md) D7).
* **Engines of our own for a pattern that arrives at run time.**
* **A raw-string form** (D2), **matching bytes** (D3), **backreferences and
  lookaround** (D1): none is decided against; none is promised.

## 5. Consequences

* A pattern written as a literal cannot be wrong at run time and, once its
  engine is built, costs nothing before its first match.
* The engines S1-S5 are ours, written in Nikaia on
  [ADR-specialization-1](adr-specialization-1.md), each held to the crate by
  test (D5.1) and built one at a time, each after a measurement says it pays.
* Code per pattern adds build time and binary size; the limits, the type and
  the report (D9) are how that is kept in view.
* A table in the binary is in the target's byte order; a build for another
  target writes the target's.
* `std` gains `regex-automata`, `regex-syntax` and, through them,
  `aho-corasick`; `Cargo.lock` is where their versions are decided.
