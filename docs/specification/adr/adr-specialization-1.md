# ADR-specialization-1: A Function Is Emitted per Build-Time Value Where Its Signature Asks: `comptime?` on a Parameter, `@unroll` on a Loop, Branches on Known Values Decided While Building, a Result Type Computed From a `comptime` Parameter

**Status:** Accepted
**Date:** October 28, 2026
**Answers:** how a library writes code that is shaped by a value known while the program is built - a pattern, a table, a layout - without a macro system and without a special case in the compiler ([#906](https://github.com/Nikaia-Foundation/Nikaia/issues/906))
**Supersedes / Related:** [ADR-304](adr-304.md) (a loop over `T::fields` is unrolled once per type; D5 rejected a second loop word for it; D7-D10 per-turn checking, messages and the report), [ADR-333](adr-333.md) D1, D3-D4 (value parameters of a type; a `comptime` parameter of a function, emitted once per value; a result type may name a bracket parameter), [ADR-370](adr-370.md) (`@inline` asks for a different emission and changes no meaning), [ADR-331](adr-331.md) (attributes and where they stand), [ADR-321](adr-321.md) D7 (the budget of build-time work, `NK1152`), [ADR-311](adr-311.md) (a build-time value crosses in its view form), [ADR-296](adr-296.md) D17 (the lowering never guesses), [ADR-298](adr-298.md) (reserved words), [ADR-regex-1](adr-regex-1.md) (the first user)

## 1. Question

```nika
comptime DATE = regex::compile("(\\d{4})-(\\d{2})-(\\d{2})")

let m = DATE.find(line) ?? return
```

`DATE` is known while the program is built, and so is the automaton the build
made from it. `find` is one function in `std`. To run as code made for this
pattern - a block per state, transitions as comparisons, no table read per
byte - `find` has to be emitted once for this value, with the automaton's
states as constants it can walk while building.

The language unrolls one loop today: a `for` over `T::fields`, once per type
(ADR-304). A `for` over a build-time value is a run-time loop over a constant
table (`for x in TABLE.iter()`). ADR-333 D3 lets a function's parameter be
`comptime`, emitted once per value, but a value must then be known at every
call, and the receiver of a method has no such mark. A library that wants code
shaped by a value has no way to say so; the alternative is a special case in
the compiler for each such library.

## 2. How others do it

| | construct | cost there |
|---|---|---|
| Zig | `comptime` parameters; `inline for` / `inline while` over a comptime value; an `if` on a comptime value is decided at compile time | code size grows with the list; errors point into the unrolled body |
| C++ | templates, `constexpr`, `if constexpr`; CTRE builds a regular expression matcher this way | compile time; messages from deep instantiations |
| D | `static foreach`, `static if`; `mixin` (code from strings) | `mixin`: messages about text nobody wrote |
| Rust | const generics, no loop over a value; `macro_rules!`, procedural macros | the macro systems this language does not have |
| LLVM | unrolls a loop by a factor when it judges it pays | values stay run-time values; nothing in the source can rely on it |

Taken from memory, not checked for this record.

## 3. Decisions

### D1 — A parameter marked `comptime?` makes a copy of the function for each build-time value a call gives it, and an ordinary function for the rest.

```nika
fn find(comptime? self, text: String) -> text::Match? { … }
```

A call whose argument is a build-time value calls the copy made for that
value; two calls with equal values share one copy. A call whose argument is
not calls the ordinary function. `self` may be marked. `comptime` (ADR-333 D3)
stays the contract: the argument must be a build-time value, or the call is
refused. `comptime?` is the same function where it may be one.

**A value is known by rule, never by luck:** a `comptime` binding, a literal, a
`comptime` or `comptime?` parameter inside its copy, a field, element or part
of one, and a call the build may run over such values (Part II 10.2). A `let`
the compiler happens to fold is not one.

**Why.** The mark stands in the signature, where a caller reads it and the
ledger carries it to another package, so a call that does not get the faster
copy was told it might not. **Rejected:** a copy for every constant argument
without a mark (nothing in the signature says it happens, and whether it
happens would depend on what the optimizer folded); a type per value as the
only way (`Regex[P]`; every value a different type, which a list cannot hold).

### D2 — `@unroll` before a `for` makes the loop variable a build-time value in every turn.

```nika
@unroll
for st in t.states {
    if st.accepting { … }        // decided while building, per turn
}
```

The list must be a build-time value. Each turn is a copy of the body with the
loop variable bound to that turn's element, a build-time value: a branch on it
is decided while building (D4), an `@unroll` over one of its parts unrolls,
and it may stand where a build-time value is required. `continue` goes to the
next turn and `break` past the last, as in any loop.

A list that is not a build-time value is refused at the loop with `NK1304`,
naming the list and why it is not known. A list that comes from a `comptime?`
parameter is a build-time value in the copies made for known values; in the
ordinary function the loop is an ordinary loop, and the body is checked for
both (D3).

The language promises nothing about how any loop is laid out in machine code;
an ordinary loop may be unrolled by the backend, as a call may be inlined
without `@inline`. `@unroll` is not that: it decides the stage of a name.

**Why.** What a reader must be able to rely on is what the body may do with
the loop variable, and that is the stage of the name. Like `@inline`
([ADR-370](adr-370.md)), the attribute changes no meaning: a loop and its
unrolled copies compute the same. Unlike `@inline` it is not a hint: the
checker holds the body to it. It is the language's own attribute, written bare
as `@inline` is, and the only one that stands before a statement.
**Rejected:** unrolling every loop over a build-time value (a lookup table of
10 000 entries would become 10 000 copies with no word in the source); a
threshold the compiler picks (a guess, and the same source changes shape when
a list grows); `comptime for` (reads as if the body ran while building, which
is what an ordinary loop in a `comptime` already does); `inline for` (*inline*
names inlining a function, and `inline` is a name in use); a new reserved word
(`@unroll` needs none); an attribute on the function (every loop of it, also
those that should stay loops); `@unroll(strict: false)` (it would either do
nothing or bring back the compiler's guess).

### D3 — `@unroll(up_to: N)` unrolls a list of at most `N` elements and leaves a longer one a loop; the body is checked for both.

The loop variable is a build-time value when the list has at most `N`
elements, and a run-time value otherwise. Which one is decided by the list,
while building. Because either can happen, the body is checked as an unrolled
body and as an ordinary one: the loop variable used where only a build-time
value is allowed is refused with `NK1305`, as is a `comptime?` parameter used
so in its ordinary function. `N` is a build-time integer.

**Why.** A library chooses between code and a table by size; writing the body
twice, once under `@unroll` and once without, lets the two drift. The bound is
written, so nothing is guessed, and the report says which way each loop went.

### D4 — An `if` or a `match` on a value known in this copy and turn is decided while building.

Only the branch taken is emitted. The others are checked once as ordinary code
and are not unrolled. No word marks it.

**Why.** Deciding a branch earlier changes no meaning and only removes code,
unlike unrolling, which adds it; it is what lets a copy for a known value keep
only the part that value needs.

### D5 — An unrolled body is checked per turn, and every turn counts against the build's budget.

ADR-304 D7-D8 unchanged: each turn checked with its value bound, a message
said once, naming the loop and the turn (*while unrolling `t.states`, at turn
3*). Each turn is a step of [ADR-321](adr-321.md) D7's budget; past it the
build stops with `NK1152` and the path.

### D6 — No construct asks whether a value is known while building.

What a program computes depends on its data; how fast, on the stage. A body
that must act differently for a value made while building and one made while
running reads that from the value - a variant, a field - not from the stage.

**Why.** A question like `is_comptime(x)` lets `if is_comptime(x) { 1 } else
{ 2 }` give two answers for the same value, depending on whether a caller
wrote `comptime d = …` or `let d = …`. D2 and D3 rest on unrolled and looped
code meaning the same.

### D7 — A result type may be computed while building from `comptime` parameters.

```nika
pub fn compile(comptime pattern: String) -> Regex[engine_for(pattern)] { … }
```

The type is a call the build may run (Part II 10.2) over the function's
`comptime` parameters, giving a type ([ADR-330](adr-330.md): a type is a
build-time value). Only a `comptime` parameter, not a `comptime?` one: one
function's copies do not differ in their result's type. A type written where
the result is received is checked against it as any type is, so a caller who
writes the type holds the callee to it.

**Why.** What the build computes from a value may decide which type fits it
best - for a pattern, which engine. ADR-333 D4 already lets a result type name
a parameter; this lets it compute from one.

### D8 — `--comptime` shows what was specialised.

For each function marked `comptime?`: its copies and the value each was made
for. For each call: the copy it calls, or *ordinary* and why (the argument is
a `let`, a parameter, a run-time value). For each `@unroll`: the turns, or
*not unrolled* and why (more than `up_to`; a run-time list in the ordinary
function). The branches D4 decided. The types D7 computed.

## 4. What this does not decide

* **`match` arms written from a list**, if an unrolled `if` chain over states
  does not become a jump table; the measurement on #906 decides whether the
  question arises.
* **`comptime?` in a type's brackets.**
* **Removing a `match` arm that no value reaching it can take**, by an analysis
  of the whole program.
* **Values that a trait holds** (a trait is not a type, ADR-295); a list of
  different kinds is an `enum`.

## 5. Consequences

* A library written in Nikaia can emit code shaped by a value - `std::regex`
  first (ADR-regex-1 D6) - with no case for it in the compiler.
* ADR-333 D3 has to be built first: evaluating a value per copy while building
  is the part both need.
* Two new codes: `NK1304` (`@unroll` over a list that is not known while
  building), `NK1305` (a value that may be a run-time value where only a
  build-time value is allowed).
* What is specialised costs build time and binary size where a signature or a
  loop asks for it, and nowhere else; the report shows it.
