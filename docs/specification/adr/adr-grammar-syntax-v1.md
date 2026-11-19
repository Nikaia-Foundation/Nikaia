# ADR-grammar-syntax-v1: A Grammar Is Written Scannerless, Ends With `} eod`, Is Entered by an Ordinary Call to an `entry rule`, Takes Values as a Function Does, and Writes Only What Part II 10.8 Lists

**Status:** Accepted
**Date:** November 19, 2026 (the consolidation, #904; the decisions themselves October 3 to October 28, 2026)
**Answers:** how a grammar is written: what the core parser does with foreign
syntax and where it ends, how a program enters a grammar, how a rule carries a
value to another, how an action and a bound are told apart, and which elements
a grammar may write ([#202](https://github.com/Nikaia-Foundation/Nikaia/issues/202),
[#928](https://github.com/Nikaia-Foundation/Nikaia/issues/928),
[#932](https://github.com/Nikaia-Foundation/Nikaia/issues/932),
[#125](https://github.com/Nikaia-Foundation/Nikaia/issues/125))
**Supersedes / Related:** takes the decisions on how a grammar is written from
[ADR-296](adr-296.md) and [ADR-grammar-1](adr-grammar-1.md) (their other
decisions stay there) and the whole of a record now in
[`docs/history/adr/`](../../history/adr/README.md) (spans, `same_line`, `///`);
[ADR-296](adr-296.md) D4 (the two ways a hole binds), D18 (the backend is handed
the grammar's own strings), D27 (`asset("…")`); [ADR-299](adr-299.md) D20, D21
(a package's grammar is `package::Grammar`; `main entry rule`);
[ADR-140](adr-140.md) D3 (a path is `::`); Part I 5.1 (Subject ; Config);
[ADR-037](adr-037.md) (the target decides nothing about what a program means);
[ADR-375](adr-375.md) D5 (a position carries its fact); [ADR-135](adr-135.md)
D3 (a `///` run belongs to the item after it); [ADR-views-3](adr-views-3.md)
D1-D4 (how a value is passed); Part II 10.8 (the grammar's vocabulary);
winnow-grammar's ADR 26 (values in the grammar), the engine a grammar is
lowered to

## 1. Question

A grammar is part of the language: foreign syntax inside a `.nika` file, and
the compiler's own grammar written in the same form. **What does a program
write to declare a grammar, to end it, to enter it, and to carry a value from
one rule to another, and what may a rule's pattern contain?**

Without one answer the parser cannot find where foreign syntax ends, a reader
cannot tell an action from a bound, YAML (whose productions take the
indentation as a parameter) cannot be written, and the compiler's grammar needs
things (a span, whether a line break precedes a token, doc comments) the page
of elements does not offer.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| winnow-grammar (the engine) | a rule takes typed values; an argument, a bound or `if(cond)` naming one is an expression; a user state or a custom parser function for the rest | `count` is a `usize`, so its YAML test writes `m as isize` four times | its ADR 26, `tests/yaml_indent_test.rs` |
| pest | a separate grammar file; a stack in the parser state (`PUSH`, `PEEK`, `POP`) | a push in a branch that lost stays pushed unless the stack is restored | from memory, not checked |
| Go, JavaScript, Kotlin, Swift | the scanner or parser asks whether a line break precedes a token | a token stream the grammar cannot leave | from memory, not checked |
| Rust (`rustc_parse`) | doc comments are tokens attached to the next item; one before nothing is an error | | from memory, not checked |
| Python's tokenizer | INDENT and DEDENT tokens before the grammar | a block scalar's indentation is `n+1+m` | from memory, not checked |

What does not transfer: a token stream (D1 refuses one), and a parser state the
author must keep idempotent under backtracking.

## 3. Options, weighed

| Question | Option | Why it lost / won |
| :--- | :--- | :--- |
| the end of a DSL block | counting braces; a configurable terminator | lost: foreign syntax has unbalanced braces; a terminator per block is a second syntax |
| | `} eod` | **won**: the core parser scans for one marker |
| how a grammar is entered | a `dsl X from e` form; a `parse` alias | lost: a second way to call, and a name chosen for the author |
| | an ordinary call, `Json::value(input)` | **won**: `?`, `catch` and chaining come for free |
| carrying a value | a state stack | lost: not undone by backtracking |
| | values as arguments, Subject ; Config | **won**: a branch that loses takes its arguments with it |
| action against bound | the block right after the pattern is the action (no arrow) | lost: `" "{m}` and `" " {m}` would be two programs differing by a space |
| | a word for the bound, `repeat(p, 1..2)` | lost: changes every bounded repetition, reads unlike every regular expression |
| | the action after `->` | **won**: changes every action once, mechanically, and is the engine's form |
| the span of a match | a rule-level `@=` | lost: a new rule header form |
| | `span(p)` and `here` | **won**: the form of `text(p)` |
| a line break before a token | a user-declared parser state | lost: needs `Default`, and each author carries backtracking's idempotence |
| | a built-in `same_line` | **won**: the generated parser knows what its `WS` skipped |
| `///` and hand-written rules | a doc buffer in a state; `extern rule` | lost: the state above; a lexical rule plus an action that may fail does the same |
| | a rule | **won** |

## 4. Decision

### D1 — Scannerless parsing is a language guarantee, not an implementation detail.

The core parser keeps no global token stream. A grammar consumes the raw
character stream, and whitespace handling is a property of the rule. The
consequence is accepted: Nikaia can never adopt a token-stream frontend without
breaking `dsl`.

### D2 — A DSL block ends explicitly, with `} eod`.

The core parser cannot find the end of foreign syntax by counting braces, so it
scans for the marker, skips the body and parses the rest of the file while the
DSL parser works. A body that contains the literal `} eod` is read from a file
and handed to its grammar by a call (D5, [ADR-296](adr-296.md) D27), never by a
configurable terminator.

### D3 — A grammar is parsed, not carried as text.

`grammar Name { … }` becomes a structure: rules, alternatives, patterns, the
commit point, repetition bounds, `fold`/`par_fold` with their expressions, and
the keyed `@frame`. Which rule a call enters, whether it is a `par_fold`, and
whether the grammar declares a frame are answered from the rules.

### D4 — Comments are whitespace, and string literals are Nikaia's own.

`WS = (WSE | COMMENT)*` is where comment skipping is said. The compiler brings
its own lexical `STRING`, which keeps escapes as written, because the
backend's `string` does not know `\n` - and keeping them is what makes
[ADR-296](adr-296.md) D18's "the backend is handed the same `"\n"`" true.

### D5 — A grammar is entered by an ordinary call, and the rule's name is the function's.

```nika
let data = Json::value(input)
```

`Json` is the grammar, `value` an `entry rule` in it, reached as every qualified
name is, by a path ([ADR-140](adr-140.md) D3; `Json.value(x)` is `NK1147`). A
call gets `?`, `catch`, being passed on and being chained for free. `dsl X
from e` is refused at the parse with the call in the message; `dsl` takes a
**driver** name and only that.

### D6 — Every `entry rule` is an entry, the compiler distinguishes none, and `pub` belongs to the grammar.

As the backend does: one function per entry. The compiler picks no rule as the
grammar's main one - such a pick chooses wrong silently - so there is no
*"grammar has no entry to enter through"* to refuse. The grammar's author may
say which one is main (`main entry rule`, [ADR-299](adr-299.md) D21); that is a
decision written down, not a pick. Naming a rule that is not an entry has its
own message.

Whether a rule is an entry and whether other packages may use it are two
questions, and each has its word. **`entry`** is a decision about the rule:
it was written to be entered on its own - a helper such as 1BRC's `NAME`,
whose `frame_end` means something only inside a frame, was not. So an entry is
never inferred from a call, which would make one out of a typo. **`pub`** is a
contract with other packages, and a grammar's contract is all its entries:
`pub grammar` offers every one of them, as a `pub enum` offers its variants,
and a rule is never `pub` on its own - `pub rule` is refused with both words in
the message. A grammar is entered only from its own file today, so `pub
grammar` is decided and not built, and refused as such.

### D7 — There is no `parse` alias.

A program that wants `Json::parse(x)` names its rule `parse`.

### D8 — Part II 10.8 lists every element a grammar may write, and it is complete.

An element not on the page is not in the language; a built-in the engine
offers that the page does not name is unreachable from a `.nika` grammar.

### D9 — A rule's action stands after `->`; a brace after a pattern is a bound.

```nika
rule pair -> Pair = key:string ":" => val:value -> { Pair { key, val } }
rule HEX -> u8 = d:hex_digit{2} -> { hex_byte(d) }
```

The first `->` of a rule is its result type, as a function's is; each
alternative's action is the block after its own `->`. A brace after a pattern
is a bound wherever spaces stand (`digit{1}`, `digit { 1 }` alike), and a brace
in the input is the literal `"{"`. `pattern { action }` is refused by name,
with the form to write in the message; an arrow anywhere else in a pattern is
refused with `=>` named. **Why:** Part I says whitespace never decides what a
program means, and with D12 both readings of a brace after a pattern would be
valid programs.

### D10 — `tag` and `digit1` are not spelled.

`"x"` and `digit+`; either engine name is `NK1187`, carrying the spelling. The
emitter keeps writing the engine's own spelling.

### D11 — Fixed-width numeric parsing needs bounded repetition in the grammar.

`p{n,m}` (as in `digit{1,2}`) is the only route by which a fixed-width fast
path may be emitted; `digit+` says unbounded, and a compiler that assumed a
width would be inventing a constraint.

### D12 — A rule declares values as a function does: Subject ; Config.

```nika
rule Block(n: i64; c: Ctx = Ctx::BlockIn) -> Yaml = …
… v:Block(n + 1) …
… v:Block(m; c: Ctx::BlockOut) …
```

Before `;` the values the rule works on, positional; after it options, named,
in any order, each with a default (Part I 5.1). An argument is a Nikaia
expression over the values in scope - the rule's parameters and options and the
bindings written before it in the sequence or in one around it - checked by
Nikaia's checker: a wrong one is an `NK` code at the argument, never a message
of the backend. A value shadows a rule of the same name. The lowering passes
every argument to the engine by position, in the declaration's order, defaults
filled in, each written `name = expr`.

**An entry takes values the same way**: `Yml::block(text, 2; c: Ctx::BlockOut)`,
the input first.

### D13 — A bound may be a value.

`" "{m}`, `" "{0, n - 1}`, `byte{len}`, `p{n,}`: each bound is an `i64`
expression. The repetition takes `k` elements with `min <= k <= max`; a bound
below zero admits no `k`, so `{0, n - 1}` with `n = 0` matches nothing. To the
analyses (frames, first sets, ADR-367's step count) a bound they cannot
evaluate has minimum 0 and no maximum.

### D14 — A guard: `if(cond)` and `if(cond, "what was expected")`.

```nika
m:count(" ") if(m > n) v:Content(m) -> { v }
```

`if(cond)` matches nothing; where `cond` is false the alternative fails there
without consuming, and the next one is tried. The message names the text given,
or the condition as written. `cond` is a `bool`; after a cut a failing guard is
an error like any other. A guard stands only inside a pattern.

### D15 — `count(p)` is an `i64`; Nikaia gets no `usize`.

`count(p)` matches `p` as often as it can and yields how often, an `i64`
carrying the fact `0 <= count <= the length of the input left` (as
[ADR-375](adr-375.md) D5's positions carry theirs), so the conversion the
lowering writes for the engine's `usize` is proved. **Why no `usize`:** its
width is the target's ([ADR-037](adr-037.md)); two integer kinds put a
conversion at every comparison (the engine's YAML test writes `m as isize` four
times where Nikaia writes `if(m > n)`); YAML's top level is indentation `-1`,
which an unsigned type cannot hold. A Rust API built on a grammar hands out
`usize` where its users expect one, converted once in its own Rust layer.

**A brace in the input** is written as a literal, `"{" p "}"`; `{ p }` as a
delimiter element is not in the language.

### D16 — `span(p)` yields the byte range `p` matched; `here` yields a position and consumes nothing.

`span(p)` is `p` with the `Span` it covered as its value, as `text(p)` has the
text. `here` is the position as a `u32`. A `Span` is two `u32`
([ADR-294](adr-294.md)) and is offered by `std::text`.

### D17 — `same_line` matches nothing and fails where the whitespace skipped just before it held a line break.

The generated parser records, by position, what its last `WS` skip held, so the
answer is a fact about the input and survives backtracking: a branch that fails
has written what a branch that succeeds would have.

### D18 — A grammar declares no state and no `extern rule`.

A rule reads its input with the elements of 10.8; a number or any other
input that needs a decision is a lexical rule whose action returns a value or
fails with `fail("…")`.

### D19 — `///` is a rule.

A grammar that has doc comments writes a `doc` rule (`"///" until(line_ending)`,
one or more) and uses it where docs may stand. Its `WS` skips `//` and not
`///`. The compiler's grammar refuses a `///` where no item, field or variant
follows, as `rustc` does ([ADR-135](adr-135.md) D3), and it no longer ignores
one.

### D20 — Hardware instructions leave the language core.

`unsafe asm`, its register-constraint table and `clobber` are removed.
Assembly is a library DSL, `dsl x86::Asm { … } eod` ([ADR-299](adr-299.md)
D20), using [ADR-296](adr-296.md) D4's immediate capture. A core construct that
cannot be given meaning on WebAssembly, a stack machine, is a defect in the
core. `unsafe` stays reserved for FFI.

### D21 — Auto-AST and regex terminals are goals, not behaviour.

Actions are required (no generated structs for labelled sequences); fixed
built-in terminals meet the intent of regex terminals. Specification text
promises neither.

## 5. Consequences

* A grammar is read from one page (Part II 10.8), and an element that is not
  there is refused, not reached through the engine.
* A grammar that writes no value means what it meant before values; one that
  does is checked by Nikaia's checker, not by rustc.
* The compiler's own grammar can be written in the grammar language: spans,
  `same_line` and doc rules say what its Rust parser said with a state and
  hand-written rules. A stray `///` is a parse error.
* Accepted cost: no token-stream frontend, ever (D1), and a body holding the
  literal `} eod` must come from a file (D2).

## 6. What this does not decide

* Memoisation keyed by position and values; left recursion through a rule with
  values (the engine's ADR 26 leaves both open).
* Where `Span` is declared in `std::text`, and a way to read the skipped
  whitespace itself (its text, its width): nothing needs it.
