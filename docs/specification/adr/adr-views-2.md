# ADR-views-2: `mut` on a `for` Binding Means What It Means on a `let`: the Element of a Place Writes Through, an Owned Element Is the Loop's Own

**Status:** Accepted
**Date:** October 29, 2026 (owner's decision, option 4 of the second decision paper; #951);
narrowed the same day by the owner: `mut` in front of one name of a tuple
pattern, and with it map values written in place, refused now, possible later (section 6)
**Answers:** what `for mut x in xs` means, which today is a parser error
(*`mut` is a reserved word*), and how a loop changes the elements of a list
or an array in place without an index loop (#951)
**Supersedes / Related:**
[ADR-views-1](adr-views-1.md) D1-D4 (a `let mut` view of a place takes and gives back; applied here to each turn);
[ADR-356](adr-356.md) (a `for` head is a `let`'s pattern);
[ADR-282](adr-282.md) D7 (a copy is written or refused, never inserted);
[ADR-118](adr-118.md) (a binding is immutable unless it says `mut`);
Part I 3.3, 6.5; #816, #959 (`let mut` of an entry or an element is a view); #960 (`NK2305` in a `for`); #962 (`NK2305` while a `let mut` view lives)

## 1. Question

**What does `mut` in a `for` head mean, and may it stand there at all?**
`for p in v { p.n += 1 }` is `NK1139`, whose help (*change a copy*) changes
nothing in `v`; `for mut p in v` is the parser's *reserved word*. The only
in-place form is `for i in 0..<v.len() { v[i].n += 1 }`, written about 45
times in `tools/*.nika`; the Rust still to move has about 18 `iter_mut()` and
14 `for mut` / `in &mut` sites (counts of 2026-10-29).

## 2. How others do it

The full table is in the decision paper (#951, second version). In short, from
memory, not checked against the manuals:

| Language | Changing the element in a loop |
| :--- | :--- |
| Python, Kotlin, C# classes | the element is a reference, so a field write reaches the list; a number cannot be changed |
| Swift, Go | the element is a copy: `for var x` (Swift) and `x.n = 1` (Go) compile and are lost, the trap both are known for; in place only through an index |
| C# structs | refused (CS1654); `foreach (ref var x in span)` is the in-place form |
| Rust | `for x in &mut v` writes through; `for mut x in v` is an owned copy and consumes `v`; using `v` in the body is refused |

What transfers: every value language that refuses or loses the change offers
an in-place form. What does not: Rust's reading of `for mut x in v` (own copy,
`v` consumed) rests on `for x in v` consuming `v`, and in Nikaia it lends.
Inside Nikaia the reading that already exists is `let mut`'s: a `mut` binding
of a place writes to the place (Part I 6.5, #816, #959).

## 3. Options, weighed

The paper weighed seven; the four that stood:

* **1, `mut` only over owned elements** (`drain()`, a range); over a lent list
  refused. The common wish stays the index loop.
* **3, `mut` over a lent list is a view of the element**; owned sources left
  undefined.
* **4, 1 and 3 together** (won): `mut` on a `for` binding means what it means
  on a `let`.
* **5, 4, and where the body reads the list the compiler lowers to an index
  loop** instead of `iter_mut()`. Not decided with 4 (section 6).

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | the common case (index loop) | - | yes | `mut` only sometimes | a wall over a lent list |
| 3 | owned sources | yes | yes | with `let mut` | owned sources unsaid |
| **4** | only a body that reads the list | yes | yes (`iter_mut`) | **one rule for `let mut` and `for mut`** | the reverse trap below |
| 5 | the same, with a bounds check | most | yes where it is `iter_mut` | yes | a second lowering to keep |

The reverse trap of 4: a user who wanted a scratch copy,
`for mut n in counts { n = n * 2; println(n) }`, changes `counts`. It is
visible in the head, it is what `let mut c = m[k] ?? continue` does, and it
shows in the program's later output, where Go's lost change shows nowhere.

## 4. Decision

### D1 — A `for` head takes `mut` where a `let` takes it: in front of the whole head

`mut` stands in front of the whole head, `for mut x in xs`,
`for mut (a, b) in pairs`, as it stands in front of a `let`'s,
`let mut (a, b) = …`, and marks every name in it. The parser's *`mut` is a
reserved word* goes.

`mut` in front of **one name inside a tuple pattern** - `let (tx, mut rx) = …`,
`for (k, mut v) in m`, `for (name, (mut line, col)) in places` - is **refused
now, and possible later** (owner, 2026-10-29; section 6). It is not refused
on purpose: it is deferred to keep the pace. The grammar keeps the place:
the parser reads the `mut` there (one pattern grammar for `let` and `for`,
ADR-356) and the refusal is a diagnostic of its own, `NK1307` (Part III, to be
reserved by the supervisor), labelled at the `mut`. Its message says the form
is not supported yet; its help writes what works today: `mut` in front of the
whole pattern (`let mut (tx, rx) = …`, `for mut (a, b) in pairs`), which marks
every name. So no program that compiles today changes meaning when the form
is added, and no new syntax is needed for it.

### D2 — Over a place, the element is a `let mut` view of it

When the `for` walks a **place** - a name, a field or an index of a list
(`Vec`) or an `Array`, held by something the function may
change (a `let mut`, a `mut` parameter, a part of one) - a `mut` binding is a
view of its element, and a change through it reaches the list. This holds for
every element type, a number included: `for mut n in counts { n = n * 2 }`
doubles `counts`. Over a place the function may not change, the `mut` binding is
refused with `NK1139`, labelled at the list: declare it `mut`, or walk a copy of
one's own (`for mut x in xs.clone()`).

### D3 — Over owned elements, the element is the loop's own

When the `for` walks elements it owns - `xs.drain()`, a range, a sequence that
produces owned values - a `mut` binding is a `let mut` of the element, as in
`let mut y = x`: a change stays in the turn.

### D4 — A key and a set element are not changed: their own refusal

`mut` on a map's key (`for mut (k, v) in m`, which marks the key) and on the
element of a set (`for mut s in seen`) is refused, on purpose, with `NK1306`
(Part III), labelled at the `mut`. Its message says why: the key decides
where the entry stands in the map (its order, or its hash), so a key changed in
place would leave it standing in the wrong place. Its help: take the entry out
and insert it again after the loop; for a map's values, which a loop cannot
change in place yet (section 6), change `m[k]` in a loop over keys taken
before it. Rust has no `iter_mut` over a set for the same reason.
(`for (mut k, v) in m` is `NK1307` first: one name of a tuple, D1.)

### D5 — Each turn is a view's block: ADR-views-1 holds per turn

The element view of D2 follows the rules of a `let mut` view (ADR-views-1):
a part may be taken out and is given back before the turn ends - at the end of
the body, a `continue`, a `break`, a `return`, a `throw` - with no `throw` or
failing call in between (`NK2108`). Handing the whole element to something that
keeps it is `NK1131`; `.clone()`, or `drain()`, is the way out.

### D6 — The list is not used in the body while its element is a view

A `for` over a place may not change it in the body (`NK2305`, Part I 3.3,
#960). Under D2 the body also may not **read** it, as with any `let mut` view
of an element until its last use (#962): `for mut p in v { p.n = v[0].n }` is
`NK2305`, labelled at the head and at the use. Its help: read what is needed
into a name before the loop, or write the index loop. Section 6 keeps open the
form that would lift this.

### D7 — Below, it is `iter_mut()`; nothing is copied

`for mut x in v` over a `Vec` or an `Array` lowers to `for x in v.iter_mut()`
with `*x` at its uses. An owned
source (D3) lowers to Rust's `for mut x in …`. No copy is inserted
(ADR-282 D7), and no index or bounds check is written.

### D8 — `NK1139`'s help names `for mut`

A change to a `for` binding without `mut` stays `NK1139`. Over a list or an array its
help writes `for mut x in xs` and says that the change reaches `xs`; for a change
meant for this turn only, `let mut y = x.clone()` (or `let mut y = x` for a
value that copies).

## 5. Consequences

* Part I 3.3 states D1-D4 and D6, and that one `mut` name in a tuple is not
  supported yet; Part I 6.5 states that a `for mut` element is
  a `let mut` view of its place (D2, D5); Part III states `NK1139`'s new help,
  `NK1306` and `NK1307`.
* The parser (`for_stmt`; `tuple_pattern` reads a per-name `mut` only to
  refuse it, `NK1307`), the checker (`check_core_stmt_for`,
  `view_changes.nika`), the lowering (`emit_stmt_for`) and the converter
  (`rs2nika`: `for x in &mut v`, `v.iter_mut()` become `for mut x in v`) are
  built from #951's *To build*. A map's `iter_mut()` / `values_mut()` stays a
  gap row until section 6's first point is built.
* The index loops in `tools/*.nika` that only touch `v[i]` become `for mut`;
  `docs/selfhosting-gaps.md` and `docs/builder-facts.md` drop the gap when it is
  built.

## 6. What this does not decide

* **`mut` in front of one name of a tuple pattern, open follow-up: refused now,
  possible later** (owner, 2026-10-29). `let (tx, mut rx) = …`,
  `for (k, mut v) in m`, `for (name, (mut line, col)) in places` are `NK1307`
  until it is decided. Deferred to keep the pace, not refused on purpose: the
  grammar keeps the place (D1), so adding it later breaks nothing. Its reading,
  when it comes, is the one D1 first wrote: the same as `let mut` on that name
  alone.
* **Map values written in place, open follow-up, with the point above.**
  `for (k, mut v) in m` (lowered `m.iter_mut()`) and `for (_, mut v) in m`
  (`m.values_mut()`) need a `mut` on one name of the pair, since one on the
  whole head marks the key (D4). No form without it is decided: `for mut v in
  m.values()` would be a writable view through an adapter, the point below.
  Until then a map's value is changed by key, `m[k]`, outside a loop over `m`.
* **Option 5 of the paper, open follow-up**: lowering to an index loop where
  the body reads the list or changes another element, so that only a change of
  the list's length or order stays refused. Until it is decided, D6 holds.
* **`mut` over an adapter**, `for (i, mut x) in v.iter().enumerate()`,
  `zip`, `rev`: the element a lending adapter hands is a read-only view, so
  `mut` on it is refused with D4's code and a help that names the index loop.
  Whether an adapter over a place hands writable views (Rust's
  `iter_mut().enumerate()`) is asked on its own.
* A `for mut` inside a lambda over a list outside it: the lambda's own rules
  apply.
