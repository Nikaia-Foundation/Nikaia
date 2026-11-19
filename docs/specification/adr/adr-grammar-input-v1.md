# ADR-grammar-input-v1: A Grammar Reads a `String` or a `Bytes` as Its Argument Decides, Checks Text Where It Becomes Text, Holds What It Read as Views and Symbols, and Interns Into the Caller's Table

**Status:** Accepted
**Date:** November 19, 2026 (the consolidation, #904; the decisions themselves October 3 to November 2, 2026)
**Answers:** what a grammar reads and how it holds what it read: which input
an entry takes, where text cut from `Bytes` is checked as UTF-8, what a view of
the input is and how long it lives, how a view is hashed, whose table an
identifier is interned into, and whether the input buffer must be torn down at
exit ([#557](https://github.com/Nikaia-Foundation/Nikaia/issues/557),
[#664](https://github.com/Nikaia-Foundation/Nikaia/issues/664),
[#904](https://github.com/Nikaia-Foundation/Nikaia/issues/904))
**Supersedes / Related:** takes the decisions on a grammar's input from
[ADR-296](adr-296.md) (its other decisions stay there) and the whole of two
records now in [`docs/history/adr/`](../../history/adr/README.md) (`Bytes`
input; the interning table); [ADR-016](adr-016.md) D1 (a view of a mapping is
text because the mapping was checked); [ADR-005](adr-005.md) D4,
[ADR-283](adr-283.md) D11 (the tether a view takes; content equality on views);
[ADR-010](adr-010.md) (the hasher, chosen by provenance); [ADR-297](adr-297.md)
(cleanup always runs, the rule D13 relaxes); [ADR-014](adr-014.md) (the mapping
and the parallel driver); [ADR-310](adr-310.md) (`asset("…")`, what a build may
read); [ADR-grammar-syntax-v1](adr-grammar-syntax-v1.md) D5, D12 (an entry is
an ordinary call and takes values after its input); [ADR-001](adr-001.md) (the
string interner); Part I 2.6 (text and `Bytes`), Part II 10.6-10.8;
winnow-grammar's ADR 25 (`input bytes;`, the backend this lowers to)

## 1. Question

A grammar is entered by a call (ADR-grammar-syntax-v1 D5). **What may that
call hand it, and how does the result hold what was read: copied, viewed or
interned, checked as text where, and in whose table?**

Without one answer a format that is not UTF-8 (a binary record, a
length-prefixed field) cannot be described, a view cut from such input could be
a `ref String` that is not text, the symbols in a parse's result cannot be read
back as text, and the input buffer's lifetime is nobody's to state.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| winnow-grammar (the engine) | a grammar declares `input bytes;` or reads text | one grammar per input type | its ADR 25 |
| rustc, Roslyn | the interner lives in a session or context the caller owns | | from memory, not checked |
| a parser library with a global interner | the table is hidden | parses on many cores contend for it | from memory, not checked |
| the fast one-billion-rows programs | hash a key from its first bytes; some also compare only the first 8-16 bytes | two keys sharing a prefix merge silently | from memory, not checked |

What does not transfer: a grammar whose type fixes its input, since a call site
already says by its argument which input it has.

## 3. Options, weighed

| Question | Option | Why it lost / won |
| :--- | :--- | :--- |
| the input type | the grammar declares it (`input bytes`); a separate `bytes_grammar` | lost: the same grammar is wanted over both, and the argument already says which |
| | the argument decides | **won** |
| text from `Bytes` | the whole input checked, then parsed as text | lost: what a text input already is, and it still refuses a binary format |
| | a view cut from `Bytes` is `ref String` unchecked | lost: a `ref String` that is not UTF-8 is undefined behaviour in safe code (ADR-016 D1) |
| | checked where it becomes text | **won** |
| a view key's comparison | compare only a prefix on a probe hit | lost: unsound |
| | hash from a prefix, compare whole | **won** |
| the interning table | the entry makes and drops its own | lost: the result's symbols cannot be read, and two parses cannot share spellings |
| | the caller's, handed as `words:` | **won**: the form an entry already has for its values |

## 4. Decision

### D1 — An entry takes a `String` or a `Bytes`; the argument decides.

```nika
let raw = fs::read("data.bin", root)      // Bytes
let e = Record::record(raw)
```

The same grammar is parsed over either. With a `Bytes` it parses its bytes;
with a `String` it parses the text. Nothing in the grammar says which, and a
grammar called with both is compiled for both. Every element of Part II 10.8
means the same over both inputs; the differences are D2 and D3.

### D2 — What becomes text from `Bytes` is checked as UTF-8 where it becomes text.

Over a `Bytes` input, an element that yields text (`until`, `text`, `string`,
`raw_ident`, `intern`, `ident`) checks the bytes it covered, and one that yields
a `scalar` (`any`, `char`) checks what it decodes. What is matched by
comparison - a literal, `digit`, `hex_digit`, the whitespace, a frame boundary,
`dec[T]` - needs no check: no byte it accepts can be anything but what was
compared. A view that passed the check is a view of the input, as over text;
nothing is copied. Over a `String` the same elements check nothing, because the
text was checked when it was made.

### D3 — A byte that is not UTF-8 is a parse error at its own offset.

The parse stops there. No enclosing alternative is retried, because none that
reads the same bytes as text can succeed, and the error must name the byte
rather than the last alternative tried. The entry throws `ParseError` as for
any other failure, with the message `invalid UTF-8 at byte N: 0xXX` (the byte
in hex), where `N` counts from the start of the whole input, in pieces of a
`par_fold` too.

### D4 — `bytes(p)` hands back the bytes `p` covered.

A view of the input's bytes, as `b[a..<b]` is (Part I 2.6), unchecked, over
either input. It is how a grammar keeps a field that is not text.

### D5 — Text used only as a map's key is checked where the map misses.

Over a `Bytes`, text that an element yields and the program uses only to look
up or insert a key of a map of text is checked as UTF-8 only where the map does
not hold it. Bytes equal to a key the map holds are that key's bytes, which
were checked when it went in, so the check would answer the same. A name that
is not UTF-8 equals no key, so it misses and is checked at its own offset as D3
says: the error and what the program printed before it are unchanged. In 1BRC
the check cost 115.6 instructions a row of 338.1 when the mapping was parsed as
`Bytes`; on a miss it runs once a station.

### D6 — Where the bytes come from is said separately, by `asset("…")`.

`comptime ROUTES = Routes::table(asset("routes.txt"))`: `comptime` says when,
`asset` where the bytes come from ([ADR-310](adr-310.md)), the call what is
done with them.

### D7 — Zero-copy is two mechanisms, and the grammar picks per rule.

Identifiers and keywords are interned `Symbol`s; bulk text (string literals,
comments, doc text, DSL bodies) is a tethered slice ([ADR-005](adr-005.md) D4,
[ADR-283](adr-283.md) D11). The guarantee is stated positively: the source
buffer cannot be freed while a token points into it.

### D8 — A view takes the input's lifetime, and the struct holding it comes along.

A view in a rule's result type is emitted with the backend input's lifetime; a
struct with a view field carries the lifetime wherever it is named, named
inside the grammar module and elided in a function signature. Every view is
emitted as `Borrowed`; the tether-state lattice is not implemented here - a
named limitation.

### D9 — Hash a view from its first bytes if you like; never truncate the comparison.

Hashing a view key from its first 8 bytes mixed with its length is sound;
comparing only a prefix on a probe hit is not. Equality on views is content
equality ([ADR-283](adr-283.md) D11). Which hash is used is chosen by
provenance ([ADR-010](adr-010.md)), never at the cost of this property.

### D10 — The caller hands the table to the entry as `words:`

An entry of a grammar that interns (it reaches `ident` or `intern(p)`) takes
the table as a value after its input, as every entry value does
(ADR-grammar-syntax-v1 D12): `ParserGrammar::file(source; words: table)`, with
`table` a `ref winnow_grammar::InternerContext`. Every symbol the parse yields
is interned there; `table.resolve(sym)` reads its text after the call. One
table handed to several parses gives one spelling one symbol across all of
them.

### D11 — An entry whose result can hold a symbol must be handed the table

Where the entry's result type holds a `Symbol` (in a field, an element, a
variant, at any depth), a call without `words:` is refused at the check
(`NK1330`): the symbols would point into a table nobody can reach. An entry
that interns only on the way (a `Symbol` read through `text(..)`, compared in a
guard) may be called without it and keeps a table of its own for the call.

### D12 — The table is made in `std`

`winnow_grammar::InternerContext::new()` makes an empty table; its ledger entry
describes it beside `resolve`. The lowering passes the table into the
`ParseContext` the generated entry builds (`nikaia_std::grammar`), not into
winnow-grammar.

### D13 — Teardown of a large read-only buffer may be skipped at process exit.

A read-only mapping with no `Cleanup` and nothing observable attached may be
left to the kernel at exit - the one relaxation of "cleanup always runs"
([ADR-297](adr-297.md)), by proof of non-observability, not by a flag. It is an
allowance; [ADR-014](adr-014.md) records that it is not taken.

## 5. Consequences

* A binary format is a grammar like any other; a grammar over text costs
  nothing more, and over `Bytes` only the elements that yield text pay a check.
* A parse's symbols can be read as text by the caller, and several parses share
  one table; a grammar without `ident` or `intern` sees no change, and one whose
  result holds no symbol may leave `words:` out.
* Accepted cost: views are emitted as plain borrows (D8).

## 6. What this does not decide

* The tether-state lattice for a grammar's views (D8).
* Dropping the whole-file check for text read from a mapping (ADR-016 §4), a
  mapping as `Bytes` (`fs::map` without the check), and a type for text not yet
  checked: about speed, not about what a program can say, measured in their own
  issue first.
