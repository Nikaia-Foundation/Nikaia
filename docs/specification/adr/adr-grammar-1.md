# ADR-grammar-1: A Grammar Takes Values - Rule Parameters and Options, Bounds and Guards - and an Action Stands After `->`

**Status:** Accepted
**Date:** October 28, 2026
**Answers:** how a grammar carries a value from one rule to another (YAML's
indentation, #928), and how a brace after a pattern is read once a bound may
hold a value (#932)
**Supersedes / Related:** [ADR-grammar-syntax-v1](adr-grammar-syntax-v1.md) D9, D12-D15 (this
record's D1-D5, moved there); Part I 5.1 (Subject ; Config);
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

For the brace after a pattern: keep the action as the block after the pattern (no arrow; a bound is a brace
*against* the pattern), give the bound a word (`repeat(p, 1..2)`), or take the
arrow back. With values, D33 makes `" "{m}` and `" " {m}` two programs that
differ by a space. A word for the bound changes every bounded repetition and
reads unlike every regular expression; the arrow changes every action, once,
mechanically, and is the engine's own form.

## 4. Decision

### D1 — An action stands after `->`; a brace after a pattern is a bound.

Moved to ADR-grammar-syntax-v1 D9.

### D2 — A rule declares values as a function does: Subject ; Config.

Moved to ADR-grammar-syntax-v1 D12.

### D3 — A bound may be a value.

Moved to ADR-grammar-syntax-v1 D13.

### D4 — A guard: `if(cond)` and `if(cond, "what was expected")`.

Moved to ADR-grammar-syntax-v1 D14.

### D5 — `count(p)` is an `i64`; Nikaia gets no `usize`.

Moved to ADR-grammar-syntax-v1 D15.

### D6 — A rule's values are passed as a function's are: lent, never copied (supervisor, 2026-11-10).

```nika
rule Two(ds: Vec[Dir]) -> Doc = …
… d:Two(ds) …
```

ADR-grammar-syntax-v1 D12 makes a rule's values a function's parameters, so ADR-views-3 D1-D4 hold for
them: the caller writes nothing, a plain value is lent, `keep` keeps it, and
`ref` is refused as everywhere outside `extern "C"` (views-3 D4). The lowering
hands the engine a borrow of a lent value; it does not clone it at each call.
**Why:** a clone per call is a cost the program never asked for (on the yaml
package's large input, 4 % of all instructions, #1548), and a rule that differs
from a function here is a second rule to learn.

## 5. Consequences

* Every grammar in the repository is rewritten with the arrow in one change
  (`scripts/grammar_arrow.nika`); the lowered Rust does not change.
* The engine is winnow-grammar 9b15c55 with winnow 1.0 (0.0.1264).
* A grammar that writes no value means what it meant.

## 6. Not decided here

* Memoisation keyed by position and values; left recursion through a rule with
  values (the engine's ADR 26 leaves both open).
