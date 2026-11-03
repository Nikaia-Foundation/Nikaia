# ADR-views-3: A Parameter That Keeps Its Argument Says `keep`, `ref` Leaves the Language, and a Result's Source Is Inferred or Written `from`

**Status:** Accepted
**Date:** November 3, 2026
**Answers:** whether a signature says that a parameter keeps its argument, and
with which word; how a view is written once that is said; how a result that
points into a parameter is written (#1213, which absorbed #1211)
**History:** the records this one replaces are archived in [`docs/history/adr`](../../history/adr/README.md); read them only for how the decision was reached
**Supersedes / Related:**
[ADR-094](adr-094.md) (the caller writes no `&`, inferred `keeps`): D2 and D6
are replaced here, D1, D3-D5 and D7 are restated here;
[ADR-184](adr-184.md) D1 (`ref X` is a view everywhere): replaced here, except
in `extern "C"`;
[ADR-358](adr-358.md) D3 (`ref`/`mut` on a whole pattern parameter): `mut`
only;
[ADR-251](adr-251.md) D4 (the ledger's `-> ref(a | b) T`): written
`-> T from a | b`;
[ADR-283](adr-283.md) D3, D13, D23, D24 (no implicit copy, the walk, the keep):
leaned on, unchanged;
[ADR-views-1](adr-views-1.md), [ADR-views-2](adr-views-2.md): unchanged.

## 1. Question

Does a function's signature say that it keeps a parameter's argument (stores
it, returns it, hands it to a task), and if so with which word; what is left of
`ref` once a plain parameter means "lent"; and is a result that points into a
parameter written in the signature?

Until now a function never said it kept an argument: the compiler inferred it
from the body into the ledger's `keeps`, and a caller learnt it from an error
when it used the variable again. A signature alone did not tell a reader what
happens to the value, and a change in a body could turn a lent parameter into
a kept one and break callers in another package without any visible change to
the signature. Keeping is the rarer case: in the ledgers of the packages and
examples, 44 of 175 functions keep at least one parameter.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Rust | the kept parameter is the plain type, the lent one `&T`, the changed one `&mut T`; a result's source is a lifetime, left out where elision decides it | `&` at every call and in every lent signature; lifetimes where elision fails | the Rust reference, lifetime elision |
| Swift | `consuming`, `borrowing` (default for most), `inout`; experimental `@lifetime(borrow s)` on a result | a word on the rare cases; lifetime dependence still experimental | SE-0377, SE-0446 (from memory, not checked) |
| Mojo | `owned` (renamed `var`), `read` (default), `mut`; a result's origin `-> ref [s] T` | the origin is written on every result that is a reference | Mojo manual, value ownership (from memory, not checked) |
| Hylo | `sink`, `let` (default), `inout` | one word per case, the default unwritten | Hylo language tour |
| C++ | `T&&` with `std::move`, `const T&`, `T&` | a move is spelled at the call; nothing checks a dangling reference | the standard |
| Python, Koka, Lean | nothing is written; sharing is a reference count (Koka and Lean reuse in place) | allocations and counters the code does not show | Perceus (Reinking et al., 2021) |

What does not transfer: Python's, Koka's and Lean's answer relies on counting
references the program does not write, which Part I 6.5 rules out (no copy and
no count the program did not ask for).

## 3. Options, weighed

### For keeping

**A, do nothing.** Inference stays, the ledger records keeps.

```nika
fn add(mut self, item: Item) { self.items.push(item) }   // keeps item; the signature does not say
```

**B, a word on the kept parameter; everything else is lent.**

```nika
fn add(mut self, keep item: Item) { self.items.push(item) }
fn total(items: Array[Item]) -> i64 { ... }               // lent: the caller keeps items
```

**C, no word; the compiler shares where a kept value is used again**
(a count, and a copy when one side changes it).

**D, B, plus automatic sharing as an opt-in per type.**

### For a result that points into a parameter

**A, inferred**: `fn first_word(s: String) -> String`; the ledger records the
source, an error names it. **B, a second word on the parameter** (`lend s`).
**C, the source on the result**: `-> String from s`, `from a | b`. **D, A by
default, C where written**, and C required where there is no body to infer
from.

### Weighed

| | D1 the special case pays, not the common one | D2 the compiler works, not the developer | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **keep A** | nothing written anywhere | the compiler infers all | none | the contract depends on the body | a library's callers break on a body change |
| **keep B** | only the rarer kept parameter carries a word | the compiler checks the word against the body and its migration writes it | none for a lent parameter | one rule for functions, methods and traits; `ref` becomes redundant | a word to drop again, mechanically |
| **keep C** | nobody writes anything | the compiler inserts counts and copies | a count and an allocation the code does not show | contradicts Part I 6.5 | cost is no longer readable from code |
| **keep D** | as B, plus a type declaration | as B, plus a second behaviour | as B | two behaviours for the same code | can be added on top of B later |
| **result A** | nothing written | the compiler infers and names conflicts | none | as fields and locals | a body change silently changes what callers may do |
| **result B** | a second word beside `keep` | | none | does not say which part comes from which parameter | |
| **result C** | written on every such result | | writing on every result | brings back what removing `ref` removed | |
| **result D** | written only where there is no body, or where a library wants it fixed | the compiler infers by default and checks what is written | none | one spelling, one meaning, checked like `keep` | the optional form can be made required later without breaking code |

**Keep B** won on D1 and D4: the signature becomes the contract, the common
case stays unwritten, and the body is checked against it. C lost on D3 and on
Part I 6.5. D adds nothing that cannot be added later. **Result D** won
because a returned view, unlike a kept argument, restricts the caller only for
as long as the caller uses it, so inference with a precise error is enough by
default; where there is no body (a trait method, a function type, a foreign
function) inference has nothing to read, and a public library may want the
source fixed against its own body changes. B lost on D4 (two words, and it
cannot say which part comes from where); C alone lost on D1.

## 4. Decision

### D1 — The caller writes nothing; what an argument becomes is the callee's declaration's.

`fs::map(path)`, `serve(db)`, `into.merge(stats)`. Whether an argument is lent,
kept or changed is read off the callee's parameter (D2, D3) and the compiler
writes the reference. Nothing written before an argument says it. A foreign
crate is described before it is called (ADR-290), so every callee has a
parameter to read.

### D2 — A parameter that keeps its argument says `keep`; a plain parameter is lent.

*Keeps* means: stores it into something that outlives the call (a field of the
subject, a struct or collection handed back), returns it by value, gives it to
a task, or passes it to a callee whose own parameter keeps it.

```nika
fn add(mut self, keep item: Item) { self.items.push(item) }
```

A plain parameter is lent: the caller's value is usable again after the call.
The compiler still infers keeping over the call graph (ADR-005 D3's engine,
fail closed: a use the inference cannot resolve counts as keeping), and checks
the signature against it: a body that keeps an unmarked parameter is refused,
with a help that adds `keep`; a `keep` on a parameter the body never keeps is
a warning. Methods and trait methods say it too; a trait's signature fixes it
for every implementation. A caller that hands a value to a `keep` parameter
and uses it again is refused, with `.clone()` named as the way out; no copy is
ever inserted (ADR-283 D3).

The receiver follows the same rule: `self` is lent, `mut self` is changed in
place (D3), `keep self` consumes the object.

### D3 — A parameter the callee changes says `mut`.

`fn fill(mut out: Vec[i64])` changes the caller's value in place; the call,
`fill(xs)`, shows nothing, as `xs.push(1)` shows nothing. A callee that wants
a changeable copy writes `let mut v = x` inside. A part taken out of a `mut`
parameter is given back before the function is left, and no `throw` or call
that can fail stands in between (`NK2108`); ADR-views-1 widens this to a
`let mut` view of a place.

### D4 — `ref` leaves the language outside `extern "C"`.

No parameter, receiver, field, element type, local binding, result or
expression is written with `ref`: `ref self` becomes `self`, `ref mut self`
becomes `mut self`, `return ref s` becomes `return s`. Inside `extern "C"`,
`ref` keeps its meaning at the C boundary (Part III). A `ref` written anywhere
else is refused, with a help that removes it.

### D5 — Fields, element types and local bindings are views where the code makes them so, inferred.

A field or an element holds a view, tethered to its buffer (Part I 6.6), where
what flows into it is a piece of a buffer, and a value of its own otherwise;
the ledger lists such fields per type (`tethered`). A field or element the
program changes in place, or hands to a `keep` parameter, holds a value of its
own; where a view would flow into it, the error names where to write
`.clone()`.

`for x in xs` iterates views of the elements and leaves `xs`; iteration that
takes the elements away is written (`xs.drain()`). `let s = totals[name]` and
`let n = config.name` over a place are views; taking the value out is written
(`remove`, `drain`, `.clone()`). `let y = x` over a whole variable is a rename,
and a `let` whose initialiser is a call or a literal owns what it is given.

A struct literal, a variant's value, a tuple or a list built in a function
takes a local or a parameter as a call argument does: it lends it while the
value stays in the function, and building it is not keeping (Part I 6.5).

### D6 — The cleanup point moves with a kept value, and the ledger narrates it.

A callee that keeps a `File` runs its cleanup at the end of the callee, not of
the caller's block. The call shows nothing; the declaration says `keep`. A
change to a parameter's `keep` on a type whose teardown has an effect is a
contract change the ledger names, with the callers whose cleanup moved.

### D7 — A view of the subject is handed back without a mark; the ledger names the receiver.

A method that returns a field or a piece of `self` hands back a view of the
subject; nothing is written at the `return`, and the ledger's `borrows` names
the receiver as the source.

### D8 — A result's source is inferred, and may be written `from`.

Every view a result holds, the result itself or each view inside it (a field,
an element, a variant's value), points into the parameters its ways out reach,
by ADR-283 D13's walk, kept per view: a `Pair` whose `left` comes from `a` and
whose `right` comes from `b` keeps the two apart. Where the body chooses at run
time, the view points into the union, and the function is not refused.

The source may be written on the result:

```nika
fn first_word(s: String) -> String from s { ... }
fn longest(a: String, b: String) -> String from a | b { ... }
```

Where it is written, the body is checked against it: a way out that hands back
a view of a parameter the `from` does not name is refused. It must be written
where there is no body to infer it from: a trait's method, a function type,
and a foreign or Rust function; a result there that is a view without a `from`
is refused. Elsewhere, for example on a public library function that wants the
source fixed against later changes of its body, it is optional.

### D9 — A caller may give an argument up while a view of it lives; it may not change it or hand it over.

While a caller uses a view that may point into an argument, the argument's
data stays as it was. Giving the value up (a reassignment, the end of its
scope, a move into another variable, a lent call) moves its buffer into the
keep of whatever holds the view (ADR-283 D23); it is freed with the last such
view, and no copy is made. Changing it in place, or handing it to a `keep`
parameter, while such a view is still to be used is refused with `NK2305`, at
the change or the hand-over. The message names the view, the call that made it
and the parameter it comes from, speaks of data and never of how long anything
lives, and names the ways out: use the view first, or `.clone()` the view or
the value. A struct literal or list that takes such a text is followed as the
argument was, and a position that must hold text of its own (it is changed or
kept) refuses the move with `NK2305`, naming the place that makes it so.

### D10 — The ledger writes keeps and sources in the source's spelling.

The ledger records `keeps` per parameter as before, and a result's sources as
the source writes them: `-> String from a | b`, a struct result by path,
`returns = "left: from a, right: from b"`. A written `from` is recorded as
written. A change of either on a `pub` function is a contract change. A type
has one place for a view per field that holds one (`tethered`, in the order the
fields are written); an `enum` has one place for all its views. A value whose
views point into what the body owns takes the caller's keep (ADR-283 D10, D23).

### D11 — `Array[T]` without a length is a view of a run wherever it does not own.

*Added November 3, 2026 (#1213), from writing D4 into Parts I-III.*

D4 takes away `ref Array[T]`, ADR-184 D3's one spelling of a view of a run,
while a view of a run is still needed: a byte buffer handed to a parser, a
`comptime` table the program reads (Part I 2.2, Part II 10.2), `windows` and
`chunks`. D5's rule for text applies to it one type over, as ADR-184 D3 already
argued from ADR-282: owning needs a length, viewing does not.

* A lent parameter `xs: Array[T]` is a view of a run, and `mut xs: Array[T]` a
  changeable one; the caller hands an `Array[T, N]`, a `Vec[T]` or a piece of
  either. ADR-184 D3's function generic over the length stays for a `keep`
  parameter, which owns what it is given.
* A field, an element or a result of type `Array[T]` holds a view where what
  flows into it is a piece of a buffer (D5, D8), tethered as text is. Where it
  would have to own, it is `NK1182`, whose ways out become `Vec[T]` and
  `Array[T, N]`.
* A `comptime` declared `Array[T]` is a view of data the build fixed; nothing
  allocates. `windows` and `chunks` yield `Array[T]` views.

This replaces ADR-184 D3's "under a `ref`" paragraph and the parameter rule for
lent and changed parameters; its field and result rule and D4 (no `&`) stand.

### D12 — Returning a parameter hands back a view of it; it does not keep it.

*Added November 3, 2026 (#1213), from writing D2 and D8 into Parts I-III.*

D2's list of what keeps reads "returns it by value"; D8 says a result that
hands back a parameter points into it. D8 governs: `fn pick(a: String) ->
String { a }` is lent, its result is `from a`, and the caller's `a` stays
usable while the rules of D9 hold. Keeping covers storing into what outlives
the call, giving to a task, and passing to a parameter that keeps; a function
whose result must own the value it hands back takes it with `keep` or returns
`.clone()`. D2's list is read without "returns it by value".

### D13 — `keep` and `from` are words in one position each, not reserved; the noun of ADR-283 D23 is "anchor".

*Added November 3, 2026 (#1213).*

`from` names Rust's `From::from`, which a Nikaia program calls through ADR-290
descriptions and implements, and both words are names in existing programs. So
neither is reserved: `keep` means the mark only before a parameter or `self` in
a parameter list, `from` the source only after a result type, and both are
names everywhere else. A reserved word still means one thing wherever it
appears (Part I); these two are not reserved words.

The place a given-up buffer moves to while a view of it lives, which ADR-283
D23 and D9 above call a *keep*, is called the *anchor* in Parts I-III (the
caller's anchor, a frame's anchor), so that one word has one meaning. ADR-283's
text is left as written.

## 5. Consequences

A signature now tells a reader what a function does with each argument: kept
(`keep`), changed (`mut`) or only looked at (nothing), and, where written, what
its result points into. `ref` disappears from programs outside `extern "C"`,
together with the warnings about a redundant `ref` and the refusal of `ref` on
a copied type. All existing code needs a one-off, mechanical migration that
writes `keep` from the ledger and removes `ref`; trait signatures must decide
keeping up front. The cost accepted is one new word and that migration. The
work is #1213.

## 6. What this does not decide

* Opt-in automatic sharing for types that declare themselves cheap to share
  (the former option D of #1211): can be added on top of D2 without breaking
  code, and is not asked now.
* Whether `from` becomes required on public library functions: left optional
  until experience says otherwise.
