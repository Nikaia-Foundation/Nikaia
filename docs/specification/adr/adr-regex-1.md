# ADR-regex-1: `std::regex` Is the RE2 Dialect; a Pattern the Build Knows Is Source and Is Compiled Down a Ladder of Engines, a Pattern That Arrives at Run Time Is Input for Rust's Automata Crates

**Status:** Accepted
**Date:** October 28, 2026
**Answers:** whether and how `std` offers regular expressions ([#673](https://github.com/Nikaia-Foundation/Nikaia/issues/673)): which dialect, what the build does with a pattern it knows, which engine runs it, and what a pattern that arrives at run time runs on
**Supersedes / Related:** [ADR-374](adr-374.md) (a position in a text, `text::Match`), [ADR-375](adr-375.md) (`ByteSet`, `Seq`, positions as facts), [ADR-377](adr-377.md) (`std::simd`), [ADR-296](adr-296.md) D7 (a grammar has no regex terminals), D23-D24 (a grammar is compiled to code), [ADR-318](adr-318.md) D5-D6 (a `std` type crosses into a `comptime` by its `constant` column; a `std` function runs at build time), [ADR-311](adr-311.md) (a build-time value crosses in its view form), [ADR-287](adr-287.md) (`comptime`), [ADR-218](adr-218.md) (`unsafe` lives in `crates/unsafe`), [ADR-188](adr-188.md) (a backslash is written `\\`)

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
while it builds - they are literals - and can do what no library can: compile
the pattern to code, choose an engine for its shape, and leave nothing to do
when the first text arrives.

## 2. How others do it

| | when the pattern is compiled | engines | groups |
|---|---|---|---|
| Perl, Python, Java, PCRE | run time | backtracking, exponential in the worst case | yes, with backreferences |
| RE2, Go | run time | DFA built lazily, NFA simulation, literal prefilter | NFA simulation or one-pass |
| Rust `regex` (`regex-automata`) | run time | literal prefilter, full DFA only for small patterns (by default up to 30 NFA states and 40 KiB), else lazy DFA, else Pike VM | Pike VM, one-pass or bounded backtracking over the matched span |
| re2c, logos | build time | DFA emitted as code | re2c: tagged DFA (Laurikari), registers written on transitions |
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

### D3 — The type is `regex::Regex`; a match is a `text::Match`.

`regex::compile(pattern) -> regex::Regex throws regex::Error`. A `Regex` is
read-only and shared between threads. It searches a `String`; matching a
`Bytes` is not decided here. Every position it gives is a `text::Pos` of the
searched text ([ADR-374](adr-374.md)), and a match is `text::Match`, so
`before`, `after` and `between` cut with it.

| call | gives |
|---|---|
| `re.is_match(s)` | `bool` |
| `re.find(s; from: text::Pos)` | `text::Match?` |
| `re.find_all(s)` | `Seq[text::Match]`, left to right, not overlapping |
| `re.captures(s; from: text::Pos)` | `regex::Captures?` |
| `c.group(i)`, `c.named("year")` | `text::Match?` (group 0 is the whole match) |
| `re.replace_all(s, with)` | `String`; `with` may name groups as `$1` or `${year}` |
| `re.split(s)` | `Seq[ref String]`, the pieces between matches |

**Why.** One match type for text, whatever found it. `find_all` is a `Seq`
for the reason [ADR-375](adr-375.md) D3 gives.

### D4 — A pattern the build knows is source code, and the build compiles it.

`regex::compile(p)` where `p` is a build-time text - a literal, a text
composed of other `comptime` values, `asset("date.re")` - is a program in the
pattern language, and the build compiles it as it compiles a `grammar`
([ADR-296](adr-296.md) D23-D24): to code where code is the fastest form, to
tables where it is not (D6). A pattern that does not parse stops the build
with the engine's own message (`NK1244`, *it ran and stopped*).

A pattern that is known only when the program runs is **input**, not source:
`regex::compile(line)` compiles it then, on Rust's automata crates (D8), and a
bad one is a `regex::Error`.

**Why.** [Part II 10.1](../20-nikaia-advance.md) forbids code nobody wrote;
the pattern is code the user wrote, in another notation, as a grammar is. What
the build can do with it, a library that sees it at run time cannot: compile
it to code, pick the engine its shape needs, and lay down the result where the
program reads it in place. **Rejected:** treating a known pattern as a value
the library compiles at run time (it repeats work the build could do, every
time the program starts); tables only, never code (it gives away the fastest
form for the small patterns that are most of them).

### D5 — What the engine is held to.

1. **The same matches** as Rust's `regex` crate gives for the same pattern and
   text, checked by a differential test and by fuzzing against it.
2. **Linear** in the text, with **bounded memory**, for every pattern.
3. **Nothing at run time that the build could do** for a pattern the build
   knows: no parsing, no compiling, no check of a table, as far as the stage
   allows no allocation (D7).
4. **At least as fast as the crate**, faster where knowing the pattern while
   building pays, measured as `CLAUDE.md` *Measuring performance* says.
5. **The pattern decides what is built.** A pattern without a capturing group
   gets no group machinery; group 0, the match's start and end, is always
   there. Which calls a program makes on a `Regex` is not read.
6. **Visible**: the report says what each pattern got (D9).
7. **Bounded in binary size and build time**, by limits the build can be given.

### D6 — A known pattern takes the cheapest stage that fits it.

| stage | for | what runs |
|---|---|---|
| S0 literal | a literal, or a few alternatives of literals | `find`, `ByteSet`, a literal search; no automaton |
| S1 one-pass | a pattern with one way on at every byte | straight-line code, groups included, no set of states |
| S2 small DFA | a DFA under the code limit | the DFA as code: a block per state, a `match` on byte classes; a state that loops on a set of bytes skips them with `ByteSet` |
| S3 DFA | a DFA under the table limit | the DFA's table, read in place (D7) |
| S4 groups | capturing groups, not one-pass, a tagged DFA under its limit | a tagged DFA: a DFA whose transitions write positions into registers; no NFA at run time |
| S5 bit-parallel NFA | up to as many NFA positions as the widest vector the build has | one bit a position, a step a few word or vector operations per byte (Glushkov, as Hyperscan) |
| S6 NFA | anything larger | a flat NFA read in place, run by an NFA simulation and a lazily built DFA of our own |

Which stage a pattern takes is decided by the pattern and the limits alone.
The limits are numbers the build can be given; their defaults are set by
measurement. A stage not yet built falls to the next one that is; under all of
them is the crate at run time from the already checked pattern, which is not a
stage of its own but the floor.

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
([ADR-218](adr-218.md)). A stage of our own (S4-S6) is laid out flat: states as
`u32` words, references as indices into the same table, names in one byte run
with offsets - nothing that must be rebuilt, checked or relocated before it is
read. The crate's NFA is not used for a known pattern: it is heap blocks joined
by pointers and cannot lie in the binary.

**Why.** D5.3. A table that must be rebuilt or validated on first use is work
the build has already done. **Rejected:** replaying the crate's NFA through its
builder at run time (a pass over every state, each time the program starts);
dumping the crate's NFA's memory (its blocks hold addresses of the process
that built them, its layout is not promised, and its handle writes a count).

### D8 — A pattern that arrives at run time runs on Rust's automata crates.

`std`'s Rust half wraps `regex-automata` (with `regex-syntax`), as its maps
wrap `hashbrown`'s table ([ADR-296](adr-296.md) D42). No type of the crate
appears in a program, in the ledger or in a message. The crate is pinned in
`Cargo.lock`; a version that changes what a pattern matches is a change to the
specification's version, with its `CHANGELOG.md` heading. The build uses the
same crates to parse a known pattern, to check it and to build the automata it
lays down.

**Why.** A run-time pattern has nothing to gain from the build; the crate is
the tuned answer for it, and D5.1 makes it the reference the stages of our own
are checked against.

### D9 — `--comptime` says what each known pattern became.

For each pattern the build compiled: the stage it took, the sizes (DFA bytes,
NFA states, tagged registers), which limit sent it further down, whether a
group stage was built because the pattern has capturing groups, and the
prefilter. A pattern on the floor is listed as such.

**Why.** A pattern changes stage silently when it grows; the report is where
that is seen, as `--comptime` already shows what else the build computed.

## 4. What this does not decide

* **Reading the calls a program makes** on a `Regex` to build less (an
  `is_match`-only pattern without a reverse DFA): only once a measurement shows
  it pays.
* **Stages of our own for a pattern that arrives at run time.**
* **Typed groups.** `dsl regex::Regex { (?P<year>\d{4}) } eod` giving a result
  struct with a field per named group is a record of its own.
* **Regex terminals in a `grammar`** stay a goal ([ADR-296](adr-296.md) D7).
* **A raw-string form** (D2), **matching bytes** (D3), **backreferences and
  lookaround** (D1): none is decided against; none is promised.

## 5. Consequences

* A pattern written as a literal cannot be wrong at run time and, once its
  stage is built, costs nothing before its first match.
* The stages S1, S2 and S4-S6 are engines of our own, and each is held to the
  crate's matches by test (D5.1). They are built one at a time, each after a
  measurement says it pays.
* Code per pattern adds build time and binary size; the limits and the report
  (D9) are how that is kept in view.
* A table in the binary is in the target's byte order; a build for another
  target writes the target's.
* `std` gains `regex-automata` and `regex-syntax`; `Cargo.lock` is where their
  versions are decided.
