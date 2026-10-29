# ADR-collections-1: The Ordered Collections Offer What the Hashed Ones Offer, Apart from Hashing: Every Map Has `entry`, Every List and Set Has `extend`

**Status:** Accepted
**Date:** October 29, 2026 (owner's decision, option 1 "full parity"; #950)
**Answers:** whether `BTreeMap` and `BTreeSet` get `entry` and `extend`, which
`HashMap` and `Vec` have and which were `NK1171` on them (#950)
**Supersedes / Related:**
[ADR-288](adr-288.md) D11 (what `std` offers is decided by the coherence of its types; stands, applied here);
[ADR-253](adr-253.md) D2 (`a.extend(b)` joins a list to another; widened by D3 to the argument every `extend` takes);
[ADR-293](adr-293.md) section 6 (an `entry` door of the language's own stays open);
[ADR-282](adr-282.md) D7, D9 (a copy is the one the program writes; the compiler writes the one below)

## 1. Question

**Do the ordered collections have the surface of the hashed ones?** `std` has
`HashMap`/`HashSet` and the ordered `BTreeMap`/`BTreeSet`; a program picks the
ordered one where the order is part of what it prints. `HashMap::entry` with
`or_insert`, `or_insert_with` and `and_modify` works and `Vec::extend` works,
but `BTreeMap::entry` and `BTreeSet::extend` are `NK1171`, and so is
`HashSet::extend`. The self-hosted compiler fills sets with 158 one-statement
`for x in xs { s.insert(x) }` loops and updates ordered maps with a lookup
before the write.

## 2. How others do it

| Language | Its answer | Source |
| :--- | :--- | :--- |
| Rust | `entry` on `HashMap` and `BTreeMap`, each with its own `Entry`; `extend` on every collection (`Extend<T>`) | std docs, from memory, not checked |
| Java | `computeIfAbsent`, `merge` on the `Map` interface, so `TreeMap` and `HashMap` alike; `addAll` on every `Collection` | from memory, not checked |
| C++ | `try_emplace`, `insert(first, last)` on `std::map` and `std::unordered_map` alike | from memory, not checked |
| Python | `setdefault`, `update` on `dict`; `set.update` | from memory, not checked |

No language in the field gives its ordered map fewer of these entries than its
hashed one: the two differ in order and in the bound on the key, not in what a
program does with them.

## 3. Options, weighed

* **A, parity** (won): `BTreeMap::entry`, `BTreeSet::extend`, and the same
  `extend` on `HashSet`.
* **B, `extend` only**, `entry` waiting for the language's own door (ADR-293
  section 6).
* **C, nothing**: the loop and the double lookup stay the forms.

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | nobody | one contract block, the lowering is Rust's own method | one lookup per update, one call per join | a reader who learns `entry` on one map has it on the other | a later door of the language's own replaces `entry` on both at once |
| **B** | the ordered map's user | - | a second lookup per update | a map kind with less, for no reason a reader can see | - |
| **C** | every program with an ordered map | the developer writes the loop | the loop and the second lookup | two map kinds that differ in surface | 158 loops stay |

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

## 5. Consequences

* `std.contracts` gains `BTreeMap::entry`, `HashSet::extend`,
  `BTreeSet::extend`, and `Vec::extend`'s argument widens to D3's.
* The 158 one-statement insert loops of `crates/nikaia-std/src/tools/` become
  `extend` calls, and the lookups before an ordered map's write become `entry`
  where the key is given away anyway (#950).
* **Risk:** `entry` keeps its key on every call, so a site that copied the key
  only on the insert path (`if !m.contains_key(k) { m.insert(k.clone(), v) }`)
  copies it every time under `entry`. Such a site keeps its form; Rust has the
  same trade.

## 6. What this does not decide

* An `entry` door of the language's own (ADR-293 section 6): when it comes, it
  stands for both map kinds.
* `extend` on a map (from pairs): neither map kind has it; D5 makes it both or
  neither.
