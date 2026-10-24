# Facts about data: sorted lists, valid trees. Decision basis

Owner's question, 2026-10-24, in the round on claim (b) of
[#796](https://github.com/Nikaia-Foundation/Nikaia/issues/796): can a program
say *this list is sorted* or *this tree has only valid nodes*, and what does the
compiler do with it? Step 1 is decided ([ADR-381](specification/adr/adr-381.md),
[#797](https://github.com/Nikaia-Foundation/Nikaia/issues/797)); step 3 is the
owner's ([#798](https://github.com/Nikaia-Foundation/Nikaia/issues/798)).

## From zero

A **fact about data** is a statement about what a value holds, not about one
number: every element of `xs` is at most the one after it; every child index
in `nodes` points into `nodes`. The prover (Part III 14.2) proves comparisons
of whole numbers - `i < xs.len()`, `lo <= hi` - and nothing else. A fact about
data has two uses: **correctness** (a binary search on an unsorted list gives a
wrong answer, not a crash) and **checks** (an index read out of a node is safe
only if every node's index is). Claim (b) needs the second only.

## What exists today (probed on `target/fast/nikaia`, 0.0.1204)

| Probe | Result |
| :--- | :--- |
| `fn first(v: ref Vec[i64]) { assert(is_sorted(v)); … }`, `is_sorted` a pure `bool` loop | allowed, **refused** `NK1300` *not a comparison of whole numbers*; not a precondition; in a project at `unproved-claims = "warn"` walked at run time, stops on `[3, 1, 2]`; `nikaia run --unproved-claims warn FILE` still refuses (defect, #797 step 0) |
| `struct Tree { nodes, root, assert(0 <= root && root < nodes.len()) }`, read `t.nodes[t.root]` | **proved** (ADR-369 rule, published in the ledger) |
| the same, `let l = t.nodes[t.root].left; t.nodes[l]` | **checked**: no fact about what the elements hold |
| `Tree { nodes: [Node {…}], root: 0 }` | rule **refused** (*`tree` is not a whole number*); with `let ns = [...]` first it is proved (defect, #797 step 0) |
| binary search `while lo < hi { mid = lo + (hi - lo) / 2; v[mid] … }` | all four checks kept (#432): needs `0 <= lo <= hi <= len`, not sortedness |

Besides: a local list's elements get an inferred interval from every write
(ADR-306 D7), not for a parameter or a field, and nothing about elements can be
written. The logic has no quantifier (ADR-270: *asked when a claim does*;
ADR-361: *not decided*). On 2026-10-20 (#770) the owner chose not to pursue
**loop invariants and ghost variables written by the program**; a rule about a
type's data is a different thing, but close to it. The ledger
(`nikaia.contracts`, Part I 6.7, ADR-269 D18) carries borrows, `requires` and
`ensures` of whole-number claims; nothing about elements.

## The ladder

| Step | What | Cost | Decided |
| :--- | :--- | :--- | :--- |
| 0 | the two defects above | small | yes, #797 |
| 1 | a pure `bool` call is a **named property**: proved by name, carried to callers, in the ledger, checked once where not proved; removes only its own check | small: one name kind in `claim_term`, keyed like `xs.len()` | yes, ADR-381, #797 |
| 2 | `std`: `Vec::is_sorted()`, `sort()` ensures it, `binary_search` | small; `binary_search` requiring sortedness refuses unproved callers | #798 Q2 |
| 3 | an **element rule the prover reads**: `xs.all(\|x\| claim)` as bounded *for all*, instantiated at each read `xs[i]`; proved at each write (literal item, `push`, `xs[i] = v`) as ADR-306 D7 proves its interval | large (C of `formal-verification.md`): the logic's first quantifier, instantiation in certificates | #798 Q1 |
| 4 | an order between elements (`sorted` as neighbours, then any `i < j`) | needs induction; Dafny-level | not now |

Step 3 is what serves claim (b): the checker's and emitter's tables hold
indexes into other tables, and every such read is checked today. How many of
the 671 unproved indexes over `tools/*.nika` are of that kind is **not
measured**; `scripts/proved.nika` (owed for #796) should count them by reason
first, and that number decides whether step 3 pays.

## How others do it (short)

Dafny and F\*: predicates with `forall`, triggers choose instantiations.
SPARK: `for all I in A'Range => …` in `Pre`, `Post`, `Type_Invariant`; GNATprove
proves or the run time checks. Whiley: a type with a `where` predicate,
`forall` over indexes. Liquid Haskell: refinements in the element type
(`[{v:Int | v < n}]`), inferred where possible. Wuffs: a bound in the element
type, all proved. Rust, Swift, Go: nothing; the run time checks.

## What a fact may be used for

Proved facts **remove checks**; they never change what a program means, and
the compiler does not swap the program's algorithm (a `contains` stays a scan;
the program calls `binary_search`). A `sort` of a list known sorted is left as
it is: std's sort is already linear on sorted input. That keeps a program's
cost readable from its text (Part I 1.2).

## Does the proof goal need an interpreter?

No. Claim (b) is a statement about the compiler's own code, proved by the
prover the compiler already runs, with checked certificates; `comptime` and
ADR-359 D11 already run code at build time without one. A reference
interpreter (`formal-semantics.md`, level 2) is a test oracle for the
lowering, useful once `emit` has no Rust twin; it is optional and comes after.
