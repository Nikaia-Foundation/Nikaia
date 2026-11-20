# ADR-collections-v3: The Ordered Collections Offer What the Hashed Ones Offer, Apart from Hashing: Every Map Has `entry` and `extend`, Every List and Set Has `extend`, and an Array and Every Set Are Walked with `iter()` as a List Is

**Status:** Accepted
**Date:** November 20, 2026 (the consolidation, #904; the decisions themselves October 29 to November 15, 2026)
**Answers:** whether `BTreeMap` and `BTreeSet` get `entry` and `extend`, which
`HashMap` and `Vec` have and which were `NK1171` on them (#950); whether every
map has `extend` (#1605); whether `Array[T, N]` gets `iter()` and the sequence
surface of `Vec::iter()`, which was `NK1171` (#955); whether `BTreeSet` and
`HashSet` get it too (#1006)
**History:** the records this one replaces are archived in [`docs/history/adr`](../../history/adr/README.md); read them only for how the decision was reached
**Supersedes / Related:** takes every decision on collections from the two
records now in the history: the `entry` and `extend` decisions keep their
numbers D1-D6, the `iter()` decisions on arrays and sets are D7-D9;
[ADR-288](adr-288.md) D11 (what `std` offers is decided by the coherence of its types; stands, applied here);
[ADR-253](adr-253.md) D2 (`a.extend(b)` joins a list to another): changed by D3 to the argument every `extend` takes;
[ADR-293](adr-293.md) section 6 (an `entry` door of the language's own stays open);
[ADR-282](adr-282.md) D7, D9 (a copy is the one the program writes; the compiler writes the one below);
Part I 3.3 (a `for` lends its list), 4.5 (`Array[T, N]`)

## 1. Question

**Do the ordered collections have the surface of the hashed ones, and are an
array and a set walked as a list is?** `std` has `HashMap`/`HashSet` and the
ordered `BTreeMap`/`BTreeSet`; a program picks the ordered one where the order
is part of what it prints. `HashMap::entry` with `or_insert`, `or_insert_with`
and `and_modify` and `Vec::extend` worked, but `BTreeMap::entry`,
`BTreeSet::extend`, `HashSet::extend` and `extend` on either map were
`NK1171`. An array had `len`, `is_empty` and `contains` and nothing that walks
it, so `a.iter().zip(b.iter())` over two arrays was refused and the only way to
a sequence was `.to_vec()`, a copy; a set had no `iter()` either.

## 2. How others do it

| Language | Its answer | Source |
| :--- | :--- | :--- |
| Rust | `entry` on `HashMap` and `BTreeMap`, each with its own `Entry`; `extend` on every collection (`Extend<T>`, maps from pairs); `[T; N]`, `[T]` and every set have `iter()` | std docs, from memory, not checked |
| Java | `computeIfAbsent`, `merge`, `putAll` on the `Map` interface, so `TreeMap` and `HashMap` alike; `addAll` on every `Collection`; `TreeSet.iterator()` in order | from memory, not checked |
| C++ | `try_emplace`, `insert(first, last)` on `std::map` and `std::unordered_map` alike; `std::array` and `std::set` have `begin`/`end` as `std::vector` does | from memory, not checked |
| Python | `setdefault`, `update` on `dict`; `set.update` | from memory, not checked |
| Swift, Go | a fixed array is a `Sequence` like any other; `range` walks an array as it walks a slice | from memory, not checked |

No language in the field gives its ordered map fewer of these entries than its
hashed one: the two differ in order and in the bound on the key, not in what a
program does with them. An array and an ordered set are walked through the
iterator the list has, and no pipeline is put on the set itself.

## 3. Options, weighed

* **A, parity** (won): `BTreeMap::entry`, `extend` on every set and every map,
  `iter()` on an array and on every set with `Vec::iter()`'s result.
* **B, `extend` only**, `entry` waiting for the language's own door (ADR-293
  section 6).
* **C, nothing**: the loop, the double lookup and `.to_vec()` stay the forms.
* **D, the sequence methods on the set itself** (`s.any(f)`, `s.filter(f)`,
  Swift's model).

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | nobody | one contract block, the lowering is Rust's own method | one lookup per update, one call per join, no copy per walk | a reader who learns `entry` on one map has it on the other; an array reads as a list of fixed length | a later door of the language's own replaces `entry` on both at once |
| **B** | the ordered map's user | - | a second lookup per update | a map kind with less, for no reason a reader can see | - |
| **C** | every program with an ordered map, every walk over an array | the developer writes the loop or the copy | the loop, the second lookup, an allocation per walk | two map kinds that differ in surface; an array indexed as a list but not walked as one | the loops stay |
| **D** | - | the ledger doubles per set kind | - | each thing said two ways; open whether `s.filter` gives a set or a sequence; nothing in Rust to lower to | - |

## 4. Decision

### D1 — `BTreeMap::entry` is `HashMap::entry`

`m.entry(k)` on a `BTreeMap` names the place of `k` as it does on a `HashMap`,
and `or_insert(v)`, `or_insert_with(f)` and `and_modify(f)` behave the same: the
map keeps the key, `or_insert` keeps its value, the lambdas run during the
call. `Entry[V]` is one type in the language for both; below it is Rust's
`btree_map::Entry` or `hash_map::Entry`, the lowering's choice.

### D2 — Every set has `extend`

`s.extend(items)` on a `HashSet` or a `BTreeSet` puts every element of `items`
into the set, as `insert` would one by one. The set keeps them, so `items` is
given away, as `Vec::extend`'s is.

### D3 — `extend` takes the same argument wherever it stands

On a `Vec`, a `HashSet` and a `BTreeSet`, `extend` takes a list, a set or a
sequence of owned `T` (`s.extend(xs)`, `s.extend(other)`,
`s.extend(xs.drain())`). It is lowered to Rust's `extend`.

### D4 — Joining a list that is only read copies its elements, not the list

`s.extend(xs.clone())`, where `xs` is used again, is the program's copy of
each element (ADR-282 D7); the compiler writes no list in between and lowers
it to `s.extend(xs.iter().cloned())` (ADR-282 D9, ADR-258's third weight).
So the one call costs what the loop `for x in xs { s.insert(x.clone()) }`
costs.

### D5 — The rule from here on

An entry of `HashMap` or `HashSet` that does not depend on hashing is also an
entry of `BTreeMap` or `BTreeSet`, with the same contract, and the other way
round for every entry that does not depend on order (`first`, `last`, ranges
and `union`'s order stay the ordered kind's). A missing one is a defect
(CLAUDE.md, kind 3), not a question for a decision round.

### D6 — Every map has `extend`

`m.extend(pairs)` on a `HashMap` or a `BTreeMap` puts every pair into the map,
as `insert` would one by one: a later pair with a key already in the map
replaces its value, as a later `insert` does. Following D3, the argument is
another map of the same key and value types, a list of `(K, V)` or a sequence
of owned pairs (`m.extend(other)`, `m.extend(pairs)`,
`m.extend(other.drain())`); it is given away, and D4's copy of each element
applies where it is read again. Lowered to Rust's `extend`. It is the form D2
gives every set and ADR-253 D2 every list, and D5 says what one map kind has,
the other has too; Rust's `Extend<(K, V)>`, Python's `dict.update` and Java's
`putAll` do the same.

### D7 — `a.iter()` on an array is `Vec::iter()`

On `Array[T, N]` and on a run `ref Array[T]` (Part I 3.3), `a.iter()` hands back
the elements one at a time, lent, as the sequence `Vec::iter()` hands back
(`Seq[ref(self) T] sync ends sized`): it can be walked from either end, its
length is known, and every method of a sequence follows (`zip`, `map`,
`filter`, `enumerate`, `rev`, `take`, `skip`, `fold`, `collect`). It is
lowered to Rust's `.iter()` on `[T; N]` and on `[T]`. Refusing it on purpose,
with `.to_vec()` first, would be a copy the program does not need (ADR-282
D7), on the type whose point is that nothing is allocated.

### D8 — A `for` lends an array as it lends a list

`for x in a` walks the array without taking it, and `x` is a view of its
element, under the rules of Part I 3.3.

### D9 — Every set has `iter()`, in its order

`s.iter()` on a `BTreeSet[T]` hands back the elements one at a time, lent,
smallest first, as `Vec::iter()` hands back a list's
(`Seq[ref(self) T] sync ends sized`); on a `HashSet[T]` it hands back the same
sequence in the set's order, which is none, as `HashMap::keys`'s
(`Seq[ref(self) T] sync sized`: it does not run backwards). Every method of a
sequence follows (`any`, `all`, `filter`, `map`, `count`, `fold`, `collect`).
It is lowered to Rust's `.iter()`.

Why: a set is already walked by `for` (Part I 3.3), and `Vec`, arrays and both
maps start a pipeline with `iter()`, `keys()` or `values()`; a set without it
answers the same walk two ways (ADR-288 D11). Rejected: **no `iter`**, with a
`for` loop and a flag for each `any`, or a copy into a `Vec` first (a copy the
program does not need, ADR-282 D7); and **the sequence methods on the set
itself** (option D), which says each thing two ways, doubles the ledger per set
kind, leaves open whether `s.filter` gives a set or a sequence, and has no
counterpart in Rust to lower to.

## 5. Consequences

* `std.contracts` holds `BTreeMap::entry`, `extend` on `HashSet`, `BTreeSet`,
  `HashMap` and `BTreeMap`, `Vec::extend` with D3's argument, `Array::iter`,
  `BTreeSet::iter` and `HashSet::iter`.
* The one-statement insert loops of the self-hosted compiler
  (`crates/nikaia-std/src/compiler/`) are `extend` calls, and the lookups
  before an ordered map's write are `entry` where the key is given away anyway
  (#950); a walk over two arrays is a `zip` (#953), a test over a set an
  `iter().any(..)` (#1006).
* **Risk:** `entry` keeps its key on every call, so a site that copied the key
  only on the insert path (`if !m.contains_key(k) { m.insert(k.clone(), v) }`)
  copies it every time under `entry`. Such a site keeps its form; Rust has the
  same trade.

## 6. What this does not decide

* An `entry` door of the language's own (ADR-293 section 6): when it comes, it
  stands for both map kinds.
* The rest of `Vec`'s surface on an array (`first`, `last`, `sort` in place,
  `binary_search`): each is asked by ADR-288 D11's coherence test when a
  program needs it.
* D9 does not give `HashSet` `union`, `intersection` and `difference`, which
  `BTreeSet` has: under D5 their absence is a defect of the hashed kind, apart
  from the order, built when it is met.
