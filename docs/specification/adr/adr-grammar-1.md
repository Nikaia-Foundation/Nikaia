# ADR-grammar-1: A Grammar Takes Values - Rule Parameters and Options, Bounds and Guards - and an Action Stands After `->`

**Status:** Accepted
**Date:** October 28, 2026
**Answers:** how a grammar carries a value from one rule to another (YAML's
indentation, #928), and how a brace after a pattern is read once a bound may
hold a value (#932)
**Supersedes / Related:** [ADR-296](adr-296.md) D33 (the action as the block
after the pattern, no arrow: withdrawn by D1); Part I 5.1 (Subject ; Config);
Part I 2.2 (the integer types; a length is an `i64`); [ADR-037](adr-037.md)
(the target decides nothing about what a program means); [ADR-375](adr-375.md)
D5 (a position a search yields carries its fact); winnow-grammar's ADR 26
(values in the grammar), the engine a grammar is lowered to

## 1. Question

YAML's block structure is indentation, and its specification writes the
productions with the indentation `n` and the context `c` as parameters:
`s-indent(n)`, `s-l+block-node(n,c)`. A block's indentation is found on its
first line and must exceed its parent's. A Nikaia rule takes no value, a bound
is a literal, and nothing in a grammar fails on a condition: YAML cannot be
written. **What does a grammar write to carry a value, and how is a brace after
a pattern read when a bound may name one?**

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| winnow-grammar (the engine) | a rule takes typed values; an argument, a bound or `if(cond)` naming one is an expression; arguments by position, `name = expr` marks a value | expressions are Rust, checked by rustc; `count` is a `usize`, so its YAML test writes `m as isize` four times | its ADR 26, `tests/yaml_indent_test.rs` |
| pest | a stack in the parser state, `PUSH`, `PEEK`, `POP` | a push in a branch that lost stays pushed unless the stack is restored | from memory, not checked |
| Python's tokenizer | INDENT and DEDENT tokens before the grammar | `- - a` nests two sequences on one line; a block scalar's indentation is `n+1+m` | from memory, not checked |

A value passed as an argument is a function argument of the generated parser: a
branch that loses takes it with it, so backtracking needs nothing restored.

## 3. Options, weighed

Values as arguments (the engine's answer) against a state stack: the stack is
not undone by backtracking (the engine's ADR 20), so a value is the only form
that is correct without care.

For the brace after a pattern: keep ADR-296 D33 (no arrow; a bound is a brace
*against* the pattern), give the bound a word (`repeat(p, 1..2)`), or take the
arrow back. With values, D33 makes `" "{m}` and `" " {m}` two programs that
differ by a space. A word for the bound changes every bounded repetition and
reads unlike every regular expression; the arrow changes every action, once,
mechanically, and is the engine's own form.

## 4. Decision

### D1 — An action stands after `->`; a brace after a pattern is a bound.

```nika
rule pair -> Pair = key:string ":" => val:value -> { Pair { key, val } }
rule HEX -> u8 = d:hex_digit{2} -> { hex_byte(d) }
```

The first `->` of a rule is its result type, as a function's is; each
alternative's action is the block after its own `->`. A brace after a pattern is
a bound wherever spaces stand. `pattern { action }` is refused by name, with the
form to write; an arrow anywhere else in a pattern is refused with `=>` named.
This withdraws ADR-296 D33. **Why:** Part I says whitespace never decides what a
program means; D33 was the one place in a grammar where it did, and D3 makes
both readings of a brace valid programs.

### D2 — A rule declares values as a function does: Subject ; Config.

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

### D3 — A bound may be a value.

`" "{m}`, `" "{0, n - 1}`, `byte{len}`, `p{n,}`: each bound is an `i64`
expression. The repetition takes `k` elements with `min <= k <= max`; a bound
below zero admits no `k`, so `{0, n - 1}` with `n = 0` matches nothing. To the
analyses (frames, first sets, ADR-367's step count) a bound they cannot
evaluate has minimum 0 and no maximum.

### D4 — A guard: `if(cond)` and `if(cond, "what was expected")`.

```nika
m:count(" ") if(m > n) v:Content(m) -> { v }
```

`if(cond)` matches nothing; where `cond` is false the alternative fails there
without consuming, and the next one is tried. The message names the text given,
or the condition as written. `cond` is a `bool`; after a cut a failing guard is
an error like any other. A guard stands only inside a pattern.

### D5 — `count(p)` is an `i64`; Nikaia gets no `usize`.

`count(p)` matches `p` as often as it can and yields how often, an `i64`
carrying the fact `0 <= count <= the length of the input left` (as ADR-375 D5's
positions carry theirs), so the conversion the lowering writes for the engine's
`usize` is proved. **Why no `usize`:** its width is the target's (ADR-037);
two integer kinds put a conversion at every comparison (the engine's YAML test
writes `m as isize` four times where Nikaia writes `if(m > n)`); YAML's top
level is indentation `-1`, which an unsigned type cannot hold. A Rust API built
on a grammar hands out `usize` where its users expect one, converted once in
its own Rust layer.

**A brace in the input** is written as a literal, `"{" p "}"`: `{ p }` as a
delimiter element (Part II 10.8 until now) was never built and is withdrawn.

## 5. Consequences

* Every grammar in the repository is rewritten with the arrow in one change
  (`scripts/grammar_arrow.nika`); the lowered Rust does not change.
* The engine is winnow-grammar 9b15c55 with winnow 1.0 (0.0.1264).
* A grammar that writes no value means what it meant.

## 6. Not decided here

* Memoisation keyed by position and values; left recursion through a rule with
  values (the engine's ADR 26 leaves both open).
