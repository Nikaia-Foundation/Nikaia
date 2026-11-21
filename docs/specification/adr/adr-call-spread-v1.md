# ADR-call-spread-v1: A Call May Spread a Tuple Into Its Subjects and a Struct Into Its Options; a Name Written Beside the Spread Wins, and the Call Is Checked as if Written Out

**Status:** Accepted
**Date:** November 21, 2026
**Answers:** whether a call can pass the values of a tuple as its positional
arguments and the fields of a struct as its named ones, without writing each
out; needed by a command of `main` (ADR-cli-v1 D9), useful in any call
(issue [#449](https://github.com/Nikaia-Foundation/Nikaia/issues/449))
**Supersedes / Related:** Part I 5.1 (subject and options);
[ADR-cli-v1](adr-cli-v1.md) D9-D11 (a command's words reach it by a spread);
Part I 6 (`..` in a pattern is the values not named)

## 1. Question

A function that hands its options on to another writes every one out:
`focus_next(file, skipped; global: global, verbose: verbose, depth: depth)`.
Where the values already stand together - a tuple of subjects, a struct of
options, a command picked from the command line (ADR-cli-v1 D9) - the call
should take them as they stand, and still be checked as the written-out call
is.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Python | `f(*args, **kwargs)` spreads a sequence and a dict | checked only when the call runs; a misspelt key is a run-time `TypeError` | Python reference, *Calls* |
| JavaScript | `f(...xs)`; `{...a, b: 1}` for objects | positions only for calls; no check of arity | ECMAScript, *SpreadElement* |
| Rust | `S { x: 1, ..base }` for a struct literal; no spread in calls | a call writes every argument; `apply` needs a nightly trait | Rust reference, *Struct expressions* |
| Kotlin | `f(*array)` for a `vararg` only | a spread fills a `vararg`, never named parameters | Kotlin docs, *Functions* |
| Swift | none | every argument written | - |

None checks a spread of named values against the callee while the program is
built; Nikaia's types know every field, so it can.

## 3. Options, weighed

### Option A — `..t` before `;`, `..s` after it, checked as the written-out call

```nika
fn focus_next(file: String, skipped: Vec[String]; global: Global = Global {}) throws { … }
let words = ("areas.md", ["parser"])
let opts = Options { global: Global { verbose: true } }
focus_next(..words; ..opts)
focus_next(..words; ..opts, global: Global {})   // the written name wins
```

### Option B — spread only for the values of a command

`command.run(..command.args; ..command.options)` works, `f(..t)` does not.

### Option C — Python's `*t` and `**s`

| | A | B | C |
| :--- | :--- | :--- | :--- |
| D1 one meaning per form (ADR-258) | `..` is already "the rest" in a pattern | a form that works in one place only | `*` is multiplication, `**` would be a new token |
| D2 a reader sees what is passed | the type of `t`, `s` says it | same | same |
| D3 checked while built | as the written-out call | same | same |
| D4 smallest | one form, both zones | a special case of the form | two forms |
| If it is wrong | a spread is rewritten as named arguments; no program depends on more | widening later is free | - |

**Option A** is chosen; the owner refused B (a form that works only for a
command is a special case).

## 4. Decision

### D1 — `..t` before the `;` spreads a tuple into the subjects.

In a call, an argument `..t` before the `;` (or in a call without one, among
the positional arguments) passes the values of the tuple `t`, in order, as
that many positional arguments at that place. `f(..t)` with
`t: (String, Vec[String])` is `f(t.0, t.1)`; `f(a, ..t)` is `f(a, t.0, t.1)`.

### D2 — `..s` after the `;` spreads a struct into the options.

An argument `..s` among the named arguments passes each field of the struct
`s` as the option of the same name: `f(x; ..s)` with `s: S { a: int, b: bool }`
is `f(x; a: s.a, b: s.b)`. An option the struct has no field for takes its
default.

### D3 — A name written beside the spread wins.

An option written by name in the same call is passed as written, and the
spread's field of that name is not passed: `f(x; ..s, a: 1)` is
`f(x; a: 1, b: s.b)`. The order of the written name and the spread does not
matter.

### D4 — The spread is checked as the call written out.

The call is checked as the call D1-D3 write out, with the codes that call
gets: a tuple of the wrong length `NK1101`, a value of the wrong type
`NK1102`, a field that names no option of the callee `NK1109`. A zone holds
at most one spread; a second one, or a spread of a value that is neither a
tuple (before the `;`) nor a struct (after it), is refused while the program
is built (`NK1355`). The spread value is evaluated once, before the call;
the values it passes are moved or lent as the written-out arguments would be.

## 5. Consequences

* **Part I 5.1** gains a paragraph *A call may spread its arguments* with D1-D4;
  Part III's table carries `NK1355`.
* **What it costs.** The checker writes the call out before checking it; the
  lowering passes the fields (`t.0`, `s.a`) as Rust has no spread.
* **What it risks.** `..` in a call's parentheses is a spread and never a
  range: a range from the start is not a form of the language today, and could
  not take this place later.

## 6. What this does not decide

A spread in a struct literal (`S { ..base, a: 1 }`), a spread in a list
literal, a spread of a `Vec` into a last `Vec[T]` subject, and two spreads in
one zone.
