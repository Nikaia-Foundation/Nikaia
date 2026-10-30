# ADR-collections-2: An Array Is Read as a List Is: `Array[T, N]` and Its Run Have `iter()`, and the Sequence That Starts There; So Has Every Set

**Status:** Accepted
**Date:** October 29, 2026 (owner's decision, option A; #955); D3 October 30,
2026 (owner's decision, option B of #1006)
**Answers:** whether `Array[T, N]` gets `iter()` and the sequence surface of
`Vec::iter()`, which was `NK1171` (#955, found by #953 at `ty.nika`'s
`bind_code`); D3: whether `BTreeSet` and `HashSet` get it too (#1006)
**Supersedes / Related:**
[ADR-288](adr-288.md) D11 (what `std` offers is decided by the coherence of its types);
[ADR-282](adr-282.md) D7 (no copy the program did not write);
[ADR-collections-1](adr-collections-1.md) D5 (what one set kind has, the other has, apart from hashing and order; applied by D3);
Part I 3.3 (a `for` lends its list), 4.5 (`Array[T, N]`)

## 1. Question

**Does an array have `iter()`?** `std.contracts` declares `Array::len`,
`is_empty` and `contains` and nothing that walks it, so
`a.iter().zip(b.iter())` over two arrays is refused, and the only way to a
sequence is `.to_vec()`, a copy.

## 2. How others do it

| Language | Its answer | Source |
| :--- | :--- | :--- |
| Rust | `[T; N]` and `[T]` have `iter()`, the slice's iterator, so every `Iterator` method follows | std docs, from memory, not checked |
| Swift | a fixed array is a `Sequence` like any other | from memory, not checked |
| C++ | `std::array` has `begin`/`end` as `std::vector` does | from memory, not checked |
| Go | `range` walks an array as it walks a slice | from memory, not checked |

## 3. Options, weighed

* **A, `iter()` with `Vec::iter()`'s result** (won): `zip`, `map`, `filter`,
  `enumerate`, `rev` follow from the sequence.
* **B, `iter()` alone**: the same in effect, since the chain is the sequence's.
* **C, refused on purpose**, `.to_vec()` first: a copy the program does not
  need (ADR-282 D7), on the type whose point is that nothing is allocated.

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | nobody | one contract entry; the lowering is Rust's `.iter()` | none | an array reads as a list of fixed length | - |
| **C** | every walk over an array | the developer writes the copy | an allocation per walk | an array is indexed as a list but not walked as one | - |

## 4. Decision

### D1 — `a.iter()` on an array is `Vec::iter()`

On `Array[T, N]` and on a run `ref Array[T]` (Part I 3.3), `a.iter()` hands back
the elements one at a time, lent, as the sequence `Vec::iter()` hands back
(`Seq[ref(self) T] sync ends sized`): it can be walked from either end, its
length is known, and every method of a sequence follows (`zip`, `map`,
`filter`, `enumerate`, `rev`, `take`, `skip`, `fold`, `collect`). It is
lowered to Rust's `.iter()` on `[T; N]` and on `[T]`.

### D2 — A `for` lends an array as it lends a list

`for x in a` walks the array without taking it, and `x` is a view of its
element, under the rules of Part I 3.3.

### D3 — Every set has `iter()`, in its order

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
itself** (`s.any(f)`, `s.filter(f)`, Swift's model), which says each thing two
ways, doubles the ledger per set kind, leaves open whether `s.filter` gives a
set or a sequence, and has no counterpart in Rust to lower to. Rust, Java
(`TreeSet.iterator()`), C++ (`std::set::begin`) walk an ordered set in its
order through the iterator their list has, and put no pipeline on the set.

## 5. Consequences

* `std.contracts` gains `Array::iter`; #953's `bind_code` in
  `crates/nikaia-std/src/tools/ty.nika` takes `zip`.
* D3: `std.contracts` gains `BTreeSet::iter` and `HashSet::iter`; K3a part 2
  (`comptime_run.rs`'s `computed`) takes `named.iter().any(..)` (#1006).

## 6. What this does not decide

* The rest of `Vec`'s surface on an array (`first`, `last`, `sort` in place,
  `binary_search`): each is asked by ADR-288 D11's coherence test when a
  program needs it.
* D3 does not give `HashSet` `union`, `intersection` and `difference`, which
  `BTreeSet` has: under ADR-collections-1 D5 their absence is a defect of the
  hashed kind, apart from the order, built when it is met.
