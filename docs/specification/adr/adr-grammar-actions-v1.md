# ADR-grammar-actions-v1: A Rule's Action Never Pauses, a Parse Fails With a Named `ParseError` That Carries Only Its Message, and a Rule's Values Are Lent as a Function's Are

**Status:** Accepted
**Date:** November 19, 2026 (the consolidation, #904; the decisions themselves October 3 to November 10, 2026)
**Answers:** what a rule's action may do and how a parse fails: whether an
action may pause, what that makes of a grammar's entry, what a parse throws and
what that failure carries, and how a rule's values reach it
([#179](https://github.com/Nikaia-Foundation/Nikaia/issues/179),
[#904](https://github.com/Nikaia-Foundation/Nikaia/issues/904),
[#1548](https://github.com/Nikaia-Foundation/Nikaia/issues/1548))
**Supersedes / Related:** takes the decisions on actions and failure from
[ADR-296](adr-296.md) (its other decisions stay there) and the last decision of
a record now in [`docs/history/adr/`](../../history/adr/README.md) (rule values
lent); [ADR-grammar-syntax-v1](adr-grammar-syntax-v1.md) D12 (a rule declares
values as a function does); [ADR-views-3](adr-views-3.md) D1-D4 (lent, `keep`,
`ref` outside `extern "C"` refused); [ADR-292](adr-292.md) and Part II 12.6 (the
other places `sync` is demanded); [ADR-280](adr-280.md) (how a failure travels);
[ADR-186](adr-186.md) (what an entry keeps); Part II 10.1, 10.6

## 1. Question

A rule's action is ordinary code, checked like any other. **May it pause, and
what does a parse fail with?** And since a rule takes values as a function does
(ADR-grammar-syntax-v1 D12), **is a value handed to a rule copied or lent?**

Without the answers a parse that can be cut and run on several cores may wait
on the world in one of its steps, a caller cannot tell from the ledger what a
parse threw, and every call of a rule with a value pays for a copy the program
never asked for.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| winnow-grammar (the engine) | a rule is a function of the generated parser; its values are Rust arguments; an error is the engine's rendered message | a value passed by value is cloned at each call | its ADR 26 |
| pest, Ohm | actions live outside the grammar | the grammar cannot say what an action may do | from memory, not checked |

What does not transfer: an error type of the engine's own, since a program
names what it catches ([ADR-280](adr-280.md)).

## 3. Options, weighed

| Question | Option | Why it lost / won |
| :--- | :--- | :--- |
| a pause in an action | allowed | lost: a parse cut across cores would wait on the world in its steps |
| | refused where the call stands | **won** |
| what a parse throws | the engine's rendered string | lost: a caller's set has no name to `match` on |
| | a named `ParseError` | **won** |
| what it carries | line, column, expectation as fields | lost: nobody has asked; the smaller answer can be taken back |
| | the message only | **won** |
| a rule's values | copied at each call | lost: 4 % of all instructions on the yaml package's large input ([#1548](https://github.com/Nikaia-Foundation/Nikaia/issues/1548)), and a second rule to learn |
| | lent, as a function's | **won** |

## 4. Decision

### D1 — An action may not pause, and a call that can is refused where it stands.

`sync` is asked of every call an action makes and of every call a fold's
`init`, `step` and `merge` make; a call that can pause is `NK2209`, naming the
rule. A parse that can be cut and run on several cores (ADR-grammar-parallel-v1 D2) is one
whose steps do not wait on the world. Refused only where the ledger answered,
as every `sync` rule is.

### D2 — So a grammar's entry is `sync`, and says so in the ledger.

```toml
[fn."Nums::number"]
sync = "asserted"
throws = ["ParseError"]
```

`asserted` because it is a rule of the language, not an inference; the entry
is a leaf of the `sync` fixpoint that holds, so a function whose body is one
call to it is `sync` again.

### D3 — What this does not restrict.

An action may **fail**, may call anything that does not pause, and a grammar
may be entered from a body that pauses; what is refused is a pause inside the
parse.

### D4 — A parse fails with `ParseError`, and every entry says so.

`throws = ["ParseError"]`. The type is `nikaia-std`'s, in the prelude, keyed
in `std.contracts` with no module in front, as `Overtaken` is. The lowering
wraps the backend's rendered message, so a program prints exactly what it did
before; what changes is the channel - with a second named member a caller's
set is a sum a program can `match` on ([ADR-280](adr-280.md)).

### D5 — It carries the message and nothing else.

Fields a program could read - line, column, expectation - are a second
question nobody has asked.

### D6 — A callee the `throws` graph does not hold still has a set.

A grammar entry is not an `Item::Fn`, so the fixpoint reads its set from the
ledger; otherwise what a parse throws would reach no caller.

### D7 — A rule's values are passed as a function's are: lent, never copied.

```nika
rule Two(ds: Vec[Dir]) -> Doc = …
… d:Two(ds) …
```

ADR-grammar-syntax-v1 D12 makes a rule's values a function's parameters, so
[ADR-views-3](adr-views-3.md) D1-D4 hold for them: the caller writes nothing, a
plain value is lent, `keep` keeps it, and `ref` is refused as everywhere
outside `extern "C"` (views-3 D4). The lowering hands the engine a borrow of a
lent value; it does not clone it at each call. **Why:** a clone per call is a
cost the program never asked for, and a rule that differs from a function here
is a second rule to learn.

## 5. Consequences

* Every analysis that reads a function body reads an action and a fold's
  lambdas the same way: pauses (D1), failures (D4, D6). An entry's `touches`
  and `locks` are derived over its grammar's actions, and what it keeps is
  [ADR-186](adr-186.md).
* Accepted cost: `NK2209` only where the ledger can answer (D1).

## 6. What this does not decide

* Fields on `ParseError` (D5), and where an entry's failure is still a
  rendered string ([#179](https://github.com/Nikaia-Foundation/Nikaia/issues/179)).
* Whether the generated parser could be `async` - the backend's question first.
