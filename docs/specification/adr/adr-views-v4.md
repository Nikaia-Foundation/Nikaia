# ADR-views-v4: A Parameter That Keeps Its Argument Says `keep`, `ref` Leaves the Language, a Result's Source Is Inferred or Written `from`, and a `let mut` or `for mut` View of a Place Writes Through and Gives Back What It Takes

**Status:** Accepted
**Date:** November 20, 2026 (the consolidation, #904; the decisions themselves October 29 to November 18, 2026); D32 November 21, 2026 (owner, #2425)
**Answers:** whether a signature says that a parameter keeps its argument, and
with which word; how a view is written once that is said; how a result that
points into a parameter is written (#1213, which absorbed #1211); how a program
takes a part out of a place it changes through a `let mut` view and puts a new
one in, without a copy (#949, #979); what `for mut x in xs` means, and how a
loop changes the elements of a list or an array in place without an index loop
(#951)
**History:** the records this one replaces are archived in [`docs/history/adr`](../../history/adr/README.md); read them only for how the decision was reached
**Supersedes / Related:** takes every decision on views from the three records
now in the history: the parameter and result decisions keep their numbers
D1-D17, the `let mut` view decisions are D18-D23, the `for mut` decisions
D24-D31;
[ADR-094](adr-094.md) (the caller writes no `&`, inferred `keeps`): D2 and D6
are replaced here, D1, D3-D5 are restated here, D7 (a part taken out of a `mut`
parameter is given back) is widened by D18 to a `let mut` view of a place;
[ADR-184](adr-184.md) D1 (`ref X` is a view everywhere): replaced by D4, except
in `extern "C"`; D3's parameter rule replaced by D11;
[ADR-358](adr-358.md) D3 (`ref`/`mut` on a whole pattern parameter): `mut`
and `keep` only (D2, D4);
[ADR-251](adr-251.md) D4 (the ledger's `-> ref(a | b) T`): written
`-> T from a | b` (D10);
[ADR-083](adr-083.md) D2: a field of the subject handed back is a view of it,
not refused (D7); `NK1131` stands where a field is bound or passed;
[ADR-tether-v1](adr-tether-v1.md) D25: a result may point into several parameters, view by
view, and giving an argument up while viewed is allowed (D8, D9);
[ADR-tether-v1](adr-tether-v1.md) D3, D13, D23, D24 (no implicit copy, the walk, the keep):
leaned on, unchanged;
[ADR-293](adr-293.md) D31, D32 (an assignment gives back what was taken);
[ADR-282](adr-282.md) D7 (a copy is written or refused, never inserted);
[ADR-356](adr-356.md) (a `for` head is a `let`'s pattern);
[ADR-118](adr-118.md) (a binding is immutable unless it says `mut`);
[ADR-373](adr-373.md) D1, D4 (`Bytes`);
[ADR-400](adr-400.md) (`std::mem` holds measuring functions only, D22);
Part I 3.3, 5.4, 6.5, 6.6

## 1. Question

Does a function's signature say that it keeps a parameter's argument (stores
it, returns it, hands it to a task), and if so with which word; what is left of
`ref` once a plain parameter means "lent"; and is a result that points into a
parameter written in the signature? And once a `let mut` binding of a place is
a view of it: may a part be taken out of such a view, how is it put back, and
does `mut` on a `for` binding make the element such a view?

Without a word, a function never says it keeps an argument: the compiler infers
it from the body into the ledger's `keeps`, and a caller learns it from an
error when it uses the variable again. A signature alone does not tell a reader
what happens to the value, and a change in a body can turn a lent parameter
into a kept one and break callers in another package without any visible
change to the signature. Keeping is the rarer case: in the ledgers of the
packages and examples, 44 of 175 functions keep at least one parameter.

`let mut c = ledger.functions[key] ?? continue` is a view of the entry: a
change through `c` reaches the map. Without a rule, handing `c.ensures` to a
callee that keeps it is `NK1131` (`c` is only borrowed) and there is no
`mem::take`, so a program that wants the old list and a new one in its
place copies the old one. And without `for mut`, the only in-place loop is
`for i in 0..<v.len() { v[i].n += 1 }`, while `for p in v { p.n += 1 }` is
`NK1139`.

## 2. How others do it

**Keeping and a result's source:**

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Rust | the kept parameter is the plain type, the lent one `&T`, the changed one `&mut T`; a result's source is a lifetime, left out where elision decides it | `&` at every call and in every lent signature; lifetimes where elision fails | the Rust reference, lifetime elision |
| Swift | `consuming`, `borrowing` (default for most), `inout`; experimental `@lifetime(borrow s)` on a result | a word on the rare cases; lifetime dependence still experimental | SE-0377, SE-0446 (from memory, not checked) |
| Mojo | `owned` (renamed `var`), `read` (default), `mut`; a result's origin `-> ref [s] T` | the origin is written on every result that is a reference | Mojo manual, value ownership (from memory, not checked) |
| Hylo | `sink`, `let` (default), `inout` | one word per case, the default unwritten | Hylo language tour |
| C++ | `T&&` with `std::move`, `const T&`, `T&` | a move is spelled at the call; nothing checks a dangling reference | the standard |
| Python, Koka, Lean | nothing is written; sharing is a reference count (Koka and Lean reuse in place) | allocations and counters the code does not show | Perceus (Reinking et al., 2021) |

**Taking a part out of a place** (from memory, not checked): Rust writes
`std::mem::take(&mut c.ensures)` or `std::mem::replace(&mut x, v)`, a library
call that leaves the type's `Default`; Swift's `let old = c.ensures; c.ensures
= []` copies the reference and counts it, the buffer shared until written; C++
writes `std::exchange(c.ensures, {})`, or `std::move`, which leaves a *valid
but unspecified* value.

**Changing the element in a loop** (from memory, not checked):

| Language | Changing the element in a loop |
| :--- | :--- |
| Python, Kotlin, C# classes | the element is a reference, so a field write reaches the list; a number cannot be changed |
| Swift, Go | the element is a copy: `for var x` (Swift) and `x.n = 1` (Go) compile and are lost, the trap both are known for; in place only through an index |
| C# structs | refused (CS1654); `foreach (ref var x in span)` is the in-place form |
| Rust | `for x in &mut v` writes through; `for mut x in v` is an owned copy and consumes `v`; using `v` in the body is refused |

What does not transfer: Python's, Koka's, Lean's and Swift's answers rely on
counting references the program does not write, which Part I 6.5 rules out (no
copy and no count the program did not ask for); C++'s visible remainder is what
ADR-094 D7 rejected for `mut` parameters; Rust's reading of `for mut x in v`
(own copy, `v` consumed) rests on `for x in v` consuming `v`, and in Nikaia it
lends. What transfers: every value language that refuses or loses the change
in a loop offers an in-place form, and inside Nikaia the reading that already
exists is `let mut`'s: a `mut` binding of a place writes to the place.

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

### For taking a part out of a `let mut` view

**1, Part I 6.5's rule for `mut` parameters, widened to a `let mut` view.**
**2, `std` entries `mem::take` and `mem::replace`**, Rust's names: a second way
to say what 1 says, and `take` needs a type's empty value as a notion. **3, a
container method `xs.take()`**: no `Default` question, but it does not cover a
field of a struct type.

### For `mut` on a `for` binding

**1, `mut` only over owned elements** (`drain()`, a range); over a lent list
refused. **3, `mut` over a lent list is a view of the element**; owned sources
left undefined. **4, 1 and 3 together**: `mut` on a `for` binding means what it
means on a `let`. **5, 4, and where the body reads the list the compiler lowers
to an index loop** instead of `iter_mut()`.

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
| **take 1** | nobody | the checker's `NK2108` walk follows views too | no copy; an empty list in the place, nothing allocated | the rule a `mut` parameter has, word for word | the view's form refuses the program, `.clone()` stays the way out |
| **take 2** | - | - | the same | two forms for one thing | - |
| **take 3** | struct-typed parts | - | the same | a method that only some types have | - |
| **for 1** | the common case (index loop) | - | yes | `mut` only sometimes | a wall over a lent list |
| **for 3** | owned sources | yes | yes | with `let mut` | owned sources unsaid |
| **for 4** | only a body that reads the list | yes | yes (`iter_mut`) | one rule for `let mut` and `for mut` | the reverse trap below |
| **for 5** | the same, with a bounds check | most | yes where it is `iter_mut` | yes | a second lowering to keep |

**Keep B** won on D1 and D4: the signature becomes the contract, the common
case stays unwritten, and the body is checked against it. C lost on D3 and on
Part I 6.5. D adds nothing that cannot be added later. **Result D** won
because a returned view, unlike a kept argument, restricts the caller only for
as long as the caller uses it, so inference with a precise error is enough by
default; where there is no body (a trait method, a function type, a foreign
function) inference has nothing to read, and a public library may want the
source fixed against its own body changes. B lost on D4 (two words, and it
cannot say which part comes from where); C alone lost on D1.

**Take 1** won on D4: it is the rule a `mut` parameter already has. 2 lost on
D4 (two forms for one thing), 3 on D1 (a struct-typed part has no such method).

**For 4** won on D4: one rule for `let mut` and `for mut`. Its reverse trap: a
user who wanted a scratch copy, `for mut n in counts { n = n * 2; println(n) }`,
changes `counts`. It is visible in the head, it is what
`let mut c = m[k] ?? continue` does, and it shows in the program's later
output, where Go's lost change shows nowhere. 1 lost on D1 (the common wish
stays the index loop), 3 left owned sources unsaid, and 5 is not decided with
4 (section 6).

## 4. Decision

### D1 — The caller writes nothing; what an argument becomes is the callee's declaration's.

`fs::map(path)`, `serve(db)`, `into.merge(stats)`. Whether an argument is lent,
kept or changed is read off the callee's parameter (D2, D3) and the compiler
writes the reference. Nothing written before an argument says it. A foreign
crate is described before it is called (ADR-rust-boundary-1), so every callee has a
parameter to read.

### D2 — A parameter that keeps its argument says `keep`; a plain parameter is lent.

*Keeps* means: stores it into something that outlives the call (a field of the
subject, a struct or collection handed back), gives it to a task, or passes it
to a callee whose own parameter keeps it. Returning it is not keeping (D12).

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
ever inserted (ADR-tether-v1 D3).

The receiver follows the same rule: `self` is lent, `mut self` is changed in
place (D3), `keep self` consumes the object.

### D3 — A parameter the callee changes says `mut`.

`fn fill(mut out: Vec[i64])` changes the caller's value in place; the call,
`fill(xs)`, shows nothing, as `xs.push(1)` shows nothing. A callee that wants
a changeable copy writes `let mut v = x` inside. A part taken out of a `mut`
parameter is given back before the function is left, and no `throw` or call
that can fail stands in between (`NK2108`); D18 widens this to a `let mut`
view of a place.

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
by ADR-tether-v1 D13's walk, kept per view: a `Pair` whose `left` comes from `a` and
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
and a foreign or Rust function (D14). Elsewhere, for example on a public
library function that wants the source fixed against later changes of its
body, it is optional.

### D9 — A caller may give an argument up while a view of it lives; it may not change it or hand it over.

While a caller uses a view that may point into an argument, the argument's
data stays as it was. Giving the value up (a reassignment, the end of its
scope, a move into another variable, a lent call) moves its buffer into the
anchor of whatever holds the view (ADR-tether-v1 D23, D13 below); it is freed with
the last such view, and no copy is made. Changing it in place, or handing it
to a `keep` parameter, while such a view is still to be used is refused with
`NK2305`, at the change or the hand-over. The message names the view, the call
that made it and the parameter it comes from, speaks of data and never of how
long anything lives, and names the ways out: use the view first, or `.clone()`
the view or the value. A struct literal or list that takes such a text is
followed as the argument was, and a position that must hold text of its own
(it is changed or kept) refuses the move with `NK2305`, naming the place that
makes it so.

### D10 — The ledger writes keeps and sources in the source's spelling.

The ledger records `keeps` per parameter, and a result's sources as the source
writes them: `-> String from a | b`, a struct result by path,
`returns = "left: from a, right: from b"`. A written `from` is recorded as
written. A change of either on a `pub` function is a contract change. A type
has one place for a view per field that holds one (`tethered`, in the order the
fields are written); an `enum` has one place for all its views. A value whose
views point into what the body owns takes the caller's anchor (ADR-tether-v1 D10,
D23).

### D11 — `Array[T]` without a length is a view of a run wherever it does not own.

D4 takes away `ref Array[T]`, ADR-184 D3's one spelling of a view of a run,
while a view of a run is still needed: a byte buffer handed to a parser, a
`comptime` table the program reads (Part I 2.2, Part II 10.2), `windows` and
`chunks`. D5's rule for text applies to it one type over: owning needs a
length, viewing does not.

* A lent parameter `xs: Array[T]` is a view of a run, and `mut xs: Array[T]` a
  changeable one; the caller hands an `Array[T, N]`, a `Vec[T]` or a piece of
  either. ADR-184 D3's function generic over the length stays for a `keep`
  parameter, which owns what it is given.
* A field, an element or a result of type `Array[T]` holds a view where what
  flows into it is a piece of a buffer (D5, D8), tethered as text is. Where it
  would have to own, it is `NK1182`, whose ways out are `Vec[T]` and
  `Array[T, N]`.
* A `comptime` declared `Array[T]` is a view of data the build fixed; nothing
  allocates. `windows` and `chunks` yield `Array[T]` views.
* A changeable run, and a list of a `mut` place, change in place with
  `xs[i] = v`, `swap`, `sort`, `sort_by_key`, `reverse` and `fill(v)`, which
  lower to the methods of Rust's `[T]`; the argument `xs[a..<b]` to a `mut`
  parameter lowers to `&mut v[a..b]`. While the part is handed out, the list
  is not read or changed otherwise (`NK2305`, D18-D23 and D29), so two
  changeable parts of one list never overlap; two at once (Rust's
  `split_at_mut`) need disjointness shown and are not decided here (#1209).

This replaces ADR-184 D3's "under a `ref`" paragraph and the parameter rule for
lent and changed parameters; its field and result rule and D4 (no `&`) stand.

### D12 — Returning a parameter hands back a view of it; it does not keep it.

`fn pick(a: String) -> String { a }` is lent, its result is `from a` (D8), and
the caller's `a` stays usable while the rules of D9 hold. Keeping covers
storing into what outlives the call, giving to a task, and passing to a
parameter that keeps (D2); a function whose result must own the value it hands
back takes it with `keep` or returns `.clone()`.

### D13 — `keep` and `from` are words in one position each, not reserved; the noun of ADR-tether-v1 D23 is "anchor".

`from` names Rust's `From::from`, which a Nikaia program calls through ADR-rust-boundary-1
descriptions and implements, and both words are names in existing programs. So
neither is reserved: `keep` means the mark only before a parameter or `self` in
a parameter list, `from` the source only after a result type, and both are
names everywhere else. A reserved word still means one thing wherever it
appears (Part I); these two are not reserved words.

The place a given-up buffer moves to while a view of it lives, which ADR-tether-v1
D23 calls a *keep*, is called the *anchor* (the caller's anchor, a frame's
anchor), so that one word has one meaning. ADR-tether-v1's text is left as written.

### D14 — Without a body, `from` alone makes a result a view; a function type names the parameters its `from` reads.

Since `ref` is gone (D4), nothing but `from` can say that a body-less result is
a view. So in a trait's method, a function type and a foreign or Rust function,
a result that writes `from` is a view of what it names, and one that writes no
`from` owns its value; there is no third reading. Where the migration finds a
`ref` result there, it writes the `from` that the implementations' bodies
infer, or refuses with the choices.

A function type may name its parameters, as `fn(s: String, n: Int) -> String
from s`; the names are part of nothing but the `from` and documentation, two
function types that differ only in them are the same type, and a function type
whose result writes `from` names the parameters it reads. Positions (`from 0`)
are not used: a number says less than a name and means nothing elsewhere in a
signature.

A function fits a function type only if both results are views or both own
their values. A view result fits where the type's `from` names the parameters
it reads, and those are lent. A result that owns its value does not fit a type
whose result writes `from`, and the reverse does not fit either. The type says
whether the caller gets a view or a value of its own, and the code below holds
the two differently (`Fn(&str) -> &str` cannot hand back a `String` it made).
A function that makes its result is handed to a type without `from`.

### D15 — `Bytes` and `comptime` text follow D11: lent or build-fixed, they are views.

Two more types had only `ref` to say "a view": `ref Bytes`, the run of bytes
ADR-373 D4 lowers a read-only parameter to, and `comptime X: ref String`, text
the build fixed. D11's rule applies to both, one type over:

* A lent parameter `b: Bytes` that the function only reads is ADR-373 D4's run
  of bytes; one the function keeps is `keep b: Bytes` and takes the `Bytes` with
  its owner (ADR-373 D1).
* A `comptime` declared `String` is a view of text the build fixed, as a
  `comptime` `Array[T]` is (D11); nothing allocates.

The migration writes these by dropping `ref`; it keeps the type, and it does
not write `keep` for a parameter whose only use is a read.

### D16 — A list, struct or tuple built from a parameter and handed back holds views of it; its result is `from` it.

D5 says building a value from a parameter is not keeping it, and D12 says
handing a parameter back is a view, not a keep. Together they decide this case:
a function that builds a list, a struct, a variant's value or a tuple from a
lent parameter and returns it hands back views of that parameter.

* `fn names(n: String) -> Vec[String] { ["a", n] }` keeps nothing; the result
  is `from n`, as returning `n` itself is (D12).
* The parameter stays lent; the caller may pass the same value twice, and the
  result lives no longer than what it points into.
* A function that must hand back a value independent of the parameter writes
  `keep` on it, or copies (`n.clone()`); the checker does not choose for it.

### D17 — A code parameter the body calls one call at a time takes a lambda that changes what it captures.

D2 says a lambda handed to a parameter is lent, and Part I 5.4 C says a
parameter the body only calls is immediate and borrows. `std`'s entries such
as `map` let such a lambda change a local it captures (5.4 C: they follow the
lambda in everything), so the user's own code parameter allows it too:
`fn each(xs: Vec[i64], f: fn(i64))` takes `fn(v) { total = total + v }`, and
`total` holds the sum after the call.

* Nothing is written for it: the type stays `fn(i64)`. Whether a code
  parameter may run a lambda that changes its captures is inferred from the
  body, as its context is (5.4 C). A body that calls the parameter one call at
  a time takes such a lambda; that is Rust's `FnMut`, and the lowering chooses
  it.
* A body that calls the parameter from tasks that run at once (`par_iter`,
  `async`, a group) does not take such a lambda: the two calls would change
  one value. The call that hands it over is refused with `NK2104`, naming the
  captured binding, as a binding a task names while the code beside it changes it is refused (Part I 8.1.2). A lambda
  that changes nothing fits there as before.
* A kept code parameter (`keep f: fn(i64)`, a field) follows the same rule by
  where the kept value is called.
* The body alone decides, whoever calls: a parameter run one call at a time
  takes such a lambda from a caller in any file or package. The parameters run
  at once are published in the function's ledger entry (`at_once`, Part III
  13.5), which a caller elsewhere reads for `NK2104`; a `pub` function that
  gains one changes its contract (`NK1208`).
* No such program reaches `rustc` with a borrow error (Part III C.1).

### D18 — A `let mut` view of a place takes and gives back as a `mut` parameter does.

A `let mut` binding that is a view of a place (Part I 6.5: an entry of a map,
an element of a list, a field, written `let mut c = m[k] ?? continue`,
`let mut e = xs[i] ?? return`, `let mut f = s.field`) may have a part taken out
of it and must have it given back. A part is **taken** by what takes it from a
`mut` parameter: handing it to something that keeps it (a keeping argument, a
`push`, an `insert`, a `let` that moves it), and a `match` arm that changes a
part it binds. It is **given back** by assigning the part, or the whole view.

### D19 — The view is whole wherever the place can be seen again.

On every way out of the view's block - its end, a `continue`, a `break`, a
`return`, a `throw` - the view is whole again, and between the taking and the
giving back stands no `throw` and no call that can fail. Both are `NK2108`,
whose message names the view and the place it stands for. So no program reads
a map entry, a list element or a field with a part missing.

### D20 — Below, the taking is `mem::take`.

A taken part is lowered as `std::mem::take(&mut <place>.<part>)` where its type
has an empty value (a list, a map, a set, text, a number, a `bool`, a `T?`), and
otherwise as `std::mem::replace(&mut <place>.<part>, <stand-in>)`, the form
`emit_core_match.nika` writes for a `match` arm. What stands in the place while
the part is out is not observable (ADR-094 D7). Nothing is copied and, for a
collection, nothing is allocated.

### D21 — The whole view is not given away.

A `let mut` view handed whole to something that keeps it stays `NK1131`, as a
`mut` parameter does: the value is the owner's. The way out is `.clone()`, or
taking it out of its owner (`m.remove(k)`).

### D22 — No `mem::take`, no `take()` method.

`std::mem` offers no `take` and no `replace` (it holds only
[ADR-400](adr-400.md)'s measuring functions), and no container has a `take()`
method; D18-D20 are the one form.

### D23 — A `let` over a part is a view while the name is read, and a taking when it is kept.

A `let` that binds a part of a `mut` parameter, of `mut self` or of a
`let mut` view (`let outer = walk.at_field`) is a **view** of it while the
name is only read. It **takes** the part (D18) when the name is kept: handed
to something that keeps it, returned, or assigned into a place
(`walk.at_field = outer`). Which of the two it is follows from the name's
later uses, as for a parameter (Part I 6.5: a view unless the body keeps it).
A taking follows D19: the part is given back on every way out, and between
the taking and the giving back stands no `throw` and no call that can fail
(`NK2108`).

**Inside a loop**, a part taken this way out of something declared outside
the loop is given back before the turn ends: at its end, at a `continue`, at
a `break`. It is then whole at the start of the next turn, so `NK2105`'s loop
case (bound by a `let` inside a loop, declared outside it) does not apply. A
turn that can end with the part still out is `NK2108`, naming the way out.

**The message names the keeping line.** Because what the `let` is depends on
a line further down, every refusal that comes from the taking - a read or a
change of the part before it is given back (`NK2105`), a way out with it still
out (`NK2108`) - names the `let`, the place, and the line that keeps the name
and so made the `let` a taking: *"`walk.at_field` was taken out by
`let outer` at 12, because `outer` is kept at 15 (`walk.at_field = outer`)"*.

Below, the `let` is `std::mem::take(&mut <place>)` or `std::mem::replace`
with D20's stand-in; an assignment of a new value that directly follows may be
folded into one `std::mem::replace(&mut <place>, <new>)`. There is no new
syntax (D22): the four lines

```text
let outer = walk.at_field
walk.at_field = P { .. }
visit(walk)
walk.at_field = outer
```

copy nothing, in a loop or out of one.

### D24 — A `for` head takes `mut` where a `let` takes it: in front of the whole head.

`mut` stands in front of the whole head, `for mut x in xs`,
`for mut (a, b) in pairs`, as it stands in front of a `let`'s,
`let mut (a, b) = …`, and marks every name in it.

`mut` in front of **one name inside a tuple pattern** - `let (tx, mut rx) = …`,
`for (k, mut v) in m`, `for (name, (mut line, col)) in places` - means
`let mut` on that name alone, and the other names stay plain (section 6,
built by #1133). The parser reads the `mut` there (one pattern grammar for
`let` and `for`, ADR-356). A lambda's tuple parameter still takes `mut` only
in front of the whole pattern: one name marked there is `NK1307`, labelled at
the `mut`, not supported yet.

### D25 — Over a place, the element is a `let mut` view of it.

When the `for` walks a **place** - a name, a field or an index of a list
(`Vec`) or an `Array`, held by something the function may change (a `let mut`,
a `mut` parameter, a part of one) - a `mut` binding is a view of its element,
and a change through it reaches the list. This holds for every element type, a
number included: `for mut n in counts { n = n * 2 }` doubles `counts`. Over a
place the function may not change, the `mut` binding is refused with `NK1139`,
labelled at the list: declare it `mut`, or walk a copy of one's own
(`for mut x in xs.clone()`).

### D26 — Over owned elements, the element is the loop's own.

When the `for` walks elements it owns - `xs.drain()`, a range, a sequence that
produces owned values - a `mut` binding is a `let mut` of the element, as in
`let mut y = x`: a change stays in the turn.

### D27 — A key and a set element are not changed: their own refusal.

`mut` on a map's key (`for mut (k, v) in m`, which marks the key) and on the
element of a set (`for mut s in seen`) is refused, on purpose, with `NK1306`,
labelled at the `mut`. Its message says why: the key decides where the entry
stands in the map (its order, or its hash), so a key changed in place would
leave it standing in the wrong place. Its help: take the entry out and insert
it again after the loop; to change a map's values in place, write `mut` in
front of the value alone, `for (k, mut v) in m` (section 6). Rust has no
`iter_mut` over a set for the same reason. `for (mut k, v) in m` marks the key
and is `NK1306` too.

### D28 — Each turn is a view's block: D18-D23 hold per turn.

The element view of D25 follows the rules of a `let mut` view (D18-D23): a
part may be taken out and is given back before the turn ends - at the end of
the body, a `continue`, a `break`, a `return`, a `throw` - with no `throw` or
failing call in between (`NK2108`). Handing the whole element to something that
keeps it is `NK1131`; `.clone()`, or `drain()`, is the way out.

### D29 — The list is not used in the body while its element is a view.

A `for` over a place may not change it in the body (`NK2305`, Part I 3.3).
Under D25 the body also may not **read** it, as with any `let mut` view of an
element until its last use: `for mut p in v { p.n = v[0].n }` is `NK2305`,
labelled at the head and at the use. Its help: read what is needed into a name
before the loop, or write the index loop. Section 6 keeps open the form that
would lift this.

### D30 — Below, it is `iter_mut()`; nothing is copied.

`for mut x in v` over a `Vec` or an `Array` lowers to `for x in v.iter_mut()`
with `*x` at its uses. An owned source (D26) lowers to Rust's `for mut x in …`.
No copy is inserted (ADR-282 D7), and no index or bounds check is written.

### D31 — `NK1139`'s help names `for mut`.

A change to a `for` binding without `mut` stays `NK1139`. Over a list or an
array its help writes `for mut x in xs` and says that the change reaches `xs`;
for a change meant for this turn only, `let mut y = x.clone()` (or
`let mut y = x` for a value that copies).

### D32 — What a `mut` parameter receives may view another parameter; that source is inferred, and is written `from` only where there is no body.

A function may put into a `mut` parameter - an element pushed onto a list, a
field set, a value inserted into a map - a view of another lent parameter, as
it may hand one back as its result (D8):

```nika
fn walk(n: Node, mut out: Vec[Event]) {
    out.push(Event { value: n.raw })   // `out`'s elements view `n`
}
```

The source is inferred from the body by D8's walk, kept per view, exactly as
for a result: here `out`'s elements point into `n`. Nothing is written: not at
the parameter, not at the call. The caller treats the argument passed for `n`
as D9 treats an argument a result views, for as long as what it passed for
`out` lives: it may give the argument up (its buffer moves into the anchor of
the list), and may neither change it nor hand it over. A parameter that only
receives views of itself, or of values the body made, needs no rule here.

It must be written where there is no body to infer it from - a trait's method,
a function type and a foreign or Rust function, D14's places - after the
parameter: `fn(n: Node, mut out: Vec[Event] from n)`. Without it, such a
parameter receives nothing that views another parameter. Elsewhere it may be
written, and the body is then checked against it, as D8 checks a written
result. A `mut` parameter is never inferred to be `keep` (D2) because the body
stores a view of another parameter into it; that inference (`NK1102` on yaml's
`walk`, #2425) was the gap this closes.

The code below is Rust's `&mut` with a shared lifetime: `fn walk<'a>(n: &'a
Node, out: &mut Vec<Event<'a>>)`; Rust writes the lifetime, Nikaia infers it.

## 5. Consequences

A signature tells a reader what a function does with each argument: kept
(`keep`), changed (`mut`) or only looked at (nothing), and, where written, what
its result points into. `ref` is gone from programs outside `extern "C"`,
together with the warnings about a redundant `ref` and the refusal of `ref` on
a copied type; trait signatures decide keeping up front. The cost accepted is
one word and a one-off, mechanical migration that writes `keep` from the
ledger and removes `ref` (#1213).

Part I 6.5 states the take-and-give-back rule for a `mut` parameter **and** a
`let mut` view, and that a `for mut` element is a `let mut` view of its place;
Part I 3.3 states D24-D27 and D29, and that one `mut` name in a tuple marks
that name alone; Part III states `NK1139`'s help, `NK1306` and `NK1307`. A field
saved, replaced and restored around a call, and a list taken out of an entry
and replaced, are written without `.clone()`; an index loop that only touches
`v[i]` is a `for mut`. A map's values are written in place by
`for (k, mut v) in m` (#1133). The cost accepted is the
reverse trap of section 3: a scratch copy is written `.clone()`.

A function that fills a `mut` list with views of another parameter (D32) is
written without a copy and without a mark, as one that returns them is; the
cost accepted is that a caller's freedom with that argument follows from the
callee's body, as it already does for results (D8).

## 6. What this does not decide

* Opt-in automatic sharing for types that declare themselves cheap to share
  (option D for keeping): can be added on top of D2 without breaking code, and
  is not asked now.
* Whether `from` becomes required on public library functions: left optional
  until experience says otherwise.
* Whether `part.drain()` takes the part out of a `let mut` view (the elements
  go, the empty list stays): a `push` after it is `NK2105` (#949's probe),
  which is asked on its own.
* A `let mut` view of a place, or a `for mut` over a list outside it, inside a
  lambda: the lambda is a function of its own, and its own rules apply.
* **`mut` in front of one name of a tuple pattern: built (#1133).**
  `let (tx, mut rx) = …`, `for (k, mut v) in m`,
  `for (name, (mut line, col)) in places` mean `let mut` on that name alone.
  A lambda's tuple parameter is still `NK1307`: not supported yet.
* **Map values written in place: built (#1133).** `for (k, mut v) in m`
  lowers to `m.iter_mut()` and `for (_, mut v) in m` to `m.values_mut()`; the
  key stays a plain name, since `mut` on it is D27's refusal.
  `for mut v in m.values()` would be a writable view through an adapter, the
  point below, and is not decided.
* **Lowering to an index loop** (option 5 for `for mut`) where the body reads
  the list or changes another element, so that only a change of the list's
  length or order stays refused. Until it is decided, D29 holds.
* **`mut` over an adapter**, `for (i, mut x) in v.iter().enumerate()`, `zip`,
  `rev`: the element a lending adapter hands is a read-only view, so `mut` on it
  is refused with D27's code and a help that names the index loop. Whether an
  adapter over a place hands writable views (Rust's `iter_mut().enumerate()`)
  is asked on its own.
