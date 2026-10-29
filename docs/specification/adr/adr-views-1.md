# ADR-views-1: A Part Taken out of a `let mut` View of a Place Is Given Back Before the View Ends, as out of a `mut` Parameter

**Status:** Accepted
**Date:** October 29, 2026 (owner's decision, option 1; #949); D6 October 29, 2026 (owner's decision, option 1; #979)
**Answers:** how a program takes a part out of a place it changes through a
`let mut` view and puts a new one in, without a copy: what Rust writes with
`std::mem::take` and `std::mem::replace` (#949)
**Supersedes / Related:**
[ADR-094](adr-094.md) D7 (a part taken out of a `mut` parameter is given back; widened by D1 to a `let mut` view);
[ADR-293](adr-293.md) D31, D32 (an assignment gives back what was taken);
[ADR-282](adr-282.md) D7 (a copy is written or refused, never inserted);
Part I 6.5; #816 (`let mut x = m[k] ?? continue` on a `mut` map is a view of the entry, lowered to `get_mut`)

## 1. Question

**May a part be taken out of a `let mut` view of a place, and how is it put
back?** `let mut c = ledger.functions[key] ?? continue` is a view of the entry
(#816): a change through `c` reaches the map. Handing `c.ensures` to a callee
that keeps it is `NK1131` today (`c` is only borrowed), there is no `mem`
module (`NK1181`), so a program that wants the old list and a new one in its
place copies the old one (`project_lower_published`,
`crates/nikaia-std/src/tools/project_lower.nika`).

## 2. How others do it

| Language | Its answer | Source |
| :--- | :--- | :--- |
| Rust | `std::mem::take(&mut c.ensures)`, `std::mem::replace(&mut x, v)`: a library call, the type's `Default` left behind | std docs, from memory, not checked |
| Swift | `let old = c.ensures; c.ensures = []` copies the reference and counts it; the buffer is shared until written (copy on write), so the old list is not copied if the place is written before the old one is | from memory, not checked |
| C++ | `std::exchange(c.ensures, {})`, or `std::move`, which leaves a *valid but unspecified* value | from memory, not checked |

What does not transfer: Swift's answer needs reference counts on every list,
and C++'s visible remainder is what ADR-094 D7 rejected for `mut` parameters.

## 3. Options, weighed

* **1, Part I 6.5's rule for `mut` parameters, widened to a `let mut` view**
  (won).
* **2, `std` entries `mem::take` and `mem::replace`**, Rust's names: a second
  way to say what 1 says, and `take` needs a type's empty value as a notion.
* **3, a container method `xs.take()`**: no `Default` question, but it does
  not cover a field of a struct type.

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | nobody | the checker's `NK2108` walk follows views too | no copy; an empty list in the place, nothing allocated | the rule a `mut` parameter has, word for word | the view's form refuses the program, `.clone()` stays the way out |
| **2** | - | - | the same | two forms for one thing | - |
| **3** | struct-typed parts | - | the same | a method that only some types have | - |

## 4. Decision

### D1 — A `let mut` view of a place takes and gives back as a `mut` parameter does

A `let mut` binding that is a view of a place (Part I 6.5: an entry of a map,
an element of a list, a field, written `let mut c = m[k] ?? continue`,
`let mut e = xs[i] ?? return`, `let mut f = s.field`) may have a part taken out
of it and must have it given back. A part is **taken** by what takes it from a
`mut` parameter: handing it to something that keeps it (a keeping argument, a
`push`, an `insert`, a `let` that moves it), and a `match` arm that changes a
part it binds. It is **given back** by assigning the part, or the whole view.

### D2 — The view is whole wherever the place can be seen again

On every way out of the view's block - its end, a `continue`, a `break`, a
`return`, a `throw` - the view is whole again, and between the taking and the
giving back stands no `throw` and no call that can fail. Both are `NK2108`,
whose message names the view and the place it stands for. So no program reads
a map entry, a list element or a field with a part missing.

### D3 — Below, the taking is `mem::take`

A taken part is lowered as `std::mem::take(&mut <place>.<part>)` where its type
has an empty value (a list, a map, a set, text, a number, a `bool`, a `T?`), and
otherwise as `std::mem::replace(&mut <place>.<part>, <stand-in>)`, the form
`emit_core_match.nika` writes for a `match` arm. What stands in the place while
the part is out is not observable (ADR-094 D7). Nothing is copied and, for a
collection, nothing is allocated.

### D4 — The whole view is not given away

A `let mut` view handed whole to something that keeps it stays `NK1131`, as a
`mut` parameter does: the value is the owner's. The way out is `.clone()`, or
taking it out of its owner (`m.remove(k)`).

### D5 — No `mem` module, no `take()` method

`std::mem` stays absent (`NK1181`); D1-D3 are the one form.

### D6 — A `let` over a part is a view while the name is read, and a taking when it is kept

A `let` that binds a part of a `mut` parameter, of `ref mut self` or of a
`let mut` view (`let outer = walk.at_field`) is a **view** of it while the
name is only read. It **takes** the part (D1) when the name is kept: handed
to something that keeps it, returned, or assigned into a place
(`walk.at_field = outer`). Which of the two it is follows from the name's
later uses, as for a parameter (Part I 6.5: a view unless the body keeps it).
A taking follows D2: the part is given back on every way out, and between
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
with D3's stand-in; an assignment of a new value that directly follows may be
folded into one `std::mem::replace(&mut <place>, <new>)`. There is no new
syntax (D5): the four lines

```text
let outer = walk.at_field
walk.at_field = P { .. }
visit(walk)
walk.at_field = outer
```

copy nothing, in a loop or out of one.

## 5. Consequences

* Part I 6.5 states the rule for a `mut` parameter **and** a `let mut` view.
* `project_lower_published` (`crates/nikaia-std/src/tools/project_lower.nika`)
  hands `contract.ensures` and `contract.from` to a keeping callee and assigns
  the published lists, with no copy of the kept pairs (#949, #953).

* D6: the save/put/restore of a field around a call (`tools/emit_stmt.nika`,
  the walkers) is written without `.clone()` (#979).

## 6. What this does not decide

* Whether `part.drain()` takes the part out (the elements go, the empty list
  stays): today a `push` after it is `NK2105` (#949's probe), which is asked on
  its own.
* A `let mut` view of a place inside a lambda: the lambda is a function of its
  own, and its own rules apply.
