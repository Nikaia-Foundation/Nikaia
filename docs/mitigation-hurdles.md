# Four hurdles of the Rust -> Nikaia move: facts and options (owner decides the language)

Where the question comes from: the orchestrator of the #125/#558 self-hosting
sessions, from the builders' commit messages and `docs/selfhosting-plan.md`
("Facts that settle old questions"). What needs the answer now: about 22,500
Rust code lines left in `check/` + `emit/` (704 functions, measured on 707307a).
Nothing is blocked: each hurdle has a workaround. The question is what the
workarounds cost.

## 1. NK2302: more than one view parameter, and the result is a view

**What it is.** A view (`ref String`, `ref T`) is a borrowed piece of another
value. Nikaia writes no lifetimes; the compiler works out which buffer a returned
view points into (ADR-283: "nothing is written for it"). When a function takes
two views and returns one, it cannot tell which buffer the result points into,
and it refuses with NK2302 (Part I 6.6).
**Today (tried):**
* `fn pick(a: ref String, b: ref String) -> ref String { .. return a }` is
  refused, even though every `return` hands back `a` and `b` is only read (probe
  `p/r2.nika`). Part I 6.6 refuses "a result that *may* point into another
  buffer as well", and here it cannot.
* The self-hosting design makes this the common case. A moved method is
  `fn m(ref self, world: ref CheckWorld, ..) -> ref String`, and returning a
  view of `world` is NK2302 (probe `p/m2.nika`). A method that returns a view of
  `self` is accepted (`p/m.nika`).
* Counted in the Rust that is left: 56 functions return a view. 49 of them are
  `&self` methods that will become `ref self, world: ref World` methods, and 7
  are free functions with two or more references.
* The workaround is `.clone()`, an allocation per call, which no callgrind run
  has measured yet. The other workaround is a `Held` struct, which costs lines.

**Options:**
* (a) Compiler precision, no new syntax: infer which parameter the returned
  views come from, and refuse only when two can flow in. It matches the
  sentence Part I 6.6 already has ("may point into another buffer as well") and
  ADR-283's "nothing is written". Cost: medium, since the keeps/tether analysis
  already follows views into results. Risk: a wrong inference is a dangling
  view that rustc rejects (E0106/E0515) rather than undefined behaviour. That
  is a compiler defect all the same: a rustc error reaching the user is a
  Nikaia bug (Part III C.1), so the checker must refuse beforehand every
  program whose result source the emitter cannot name as one lifetime.
* (b) An explicit result source in the source, `-> ref(world) String`, the
  spelling the ledger already uses (ADR-251 D4, `described.contracts`). This
  is a new piece of syntax that runs against ADR-283's "nothing is written".
  Higher cost: parser, checker and emitter `<'a>`.
* (c) Keep it as it is. `.clone()`, `Held` structs and free functions with one view.

**Decided (October 11, 2026): (a), with a refusal where two parameters
remain** - [ADR-283](specification/adr/adr-283.md) D25-D26, Part I 6.6, issue
#580. The result points into at most one parameter, read off the body and
written to the ledger (`-> ref(world) String`); two ways out that reach two
parameters, a value that may be a view of either, or one holding views of both
stay `NK2302`, at the function. (b) and union tethering (one lifetime over every
view, which is what plain parameters lowered as views got) are rejected there;
specialising per branch is rejected too.
**The count, re-measured** (`viewfns`-style scan of 707307a): the "56" above
could not be reproduced. In `check/` + `emit/`, 19 functions hand back a view
that is not `'static` and take a receiver or two views: 17 reach one source on
every way out (counting the moved shape, `self` = state, `world` = context), 2
choose between state and world at run time (`payloads`, `options_of`), 0 at
build time; 7 more hand back only literals. In all of `crates/nikaia/src`, 41,
of which 4 choose at run time. In the moved `tools/*.nika`, 2 of the 34
`(ref self, world) -> String` methods clone a view of `world` on a way out.
**Built and measured** (#580 step 8, 0.0.842). The two clones are gone:
`first_generic` and `called_name` hand back `ref String`, taking the interner
(`words: ref winnow_grammar::InternerContext`) and not `world`, because the walk
reads a method of a type another file declares (`EmitWorld.words`) as touching
every argument. Callgrind (`--branch-sim=yes`, `nikaia lower`, binaries built
before and after, two runs each; instructions Ir, mispredicted conditional
branches Bcm): `examples/sqlite` (the `from_c` path) Ir 45,226,764 -> 45,225,843,
Bcm 354,044 -> 355,734; `tests/language/src/shape_bounds.nika` (a walk copy) Ir
31,030,750 -> 31,026,305, Bcm 223,870 -> 225,602; `defects_found_running.nika`
Ir 420,568,668 -> 420,646,650 (a rerun of the same binary moves 20,000 to
240,000). Both paths run once per function copy or foreign handle, so the saved
allocation is below the run-to-run spread; the Bcm rise is the same 1,700 on two
unrelated inputs, which points at the layout of the two binaries and not at these
functions. No gain is claimed.
**Closed** (#580 step 9, 0.0.841): a result whose struct another file declares is
walked like one declared in the file (the package's types that hold a view, from
the ledger), so it is `NK2302` or accepted and compiled, never a rustc error.
Over-refusals of existing code, measured over `tools/*.nika` (`lower-std`),
`tests/`, `examples/` and `benches/`: none from a local touched by a method call
or a `mut` parameter (the imprecision stays, nothing needs it tightened). One from
another cause: `world.words.resolve(..)` with a second view parameter, because
the fields of a type another file declares are unknown to the walk and its method
calls count as touching every argument; the workaround is to pass the field
(`words: ref winnow_grammar::InternerContext`). Making the walk read those fields
from the ledger is not built.

## 2. No lifetimes in the source

**What it is.** Rust's `Checker<'a>` holds `&'a Parsed`; Nikaia has no `'a`.
**Today:** a struct can hold views (`ParsedView { words: ref .., items: ref .. }`,
stored views, ADR-283), so a borrowed context struct already works. 23
functions left have explicit lifetime parameters. What remains of "no
lifetimes" in practice is NK2302 (topic 1) and the `world` parameter threaded
through every method.
**Options:** nothing beyond topic 1. A convention, already followed: one
borrowed context struct per module (`CheckWorld`, `EmitWorld`), passed `ref`.
**Not a language question** once topic 1 is answered.

## 3. `BTreeMap` instead of `HashMap`

**What it is.** A `HashMap` in Nikaia lowers by where its keys come from
(ADR-010 D5): `TrustedMap` (fixed seed) or the keyed default.
**Today (tried):** in `tools`, a `HashSet` field lowers to `HashSet<String>`
until the same file builds one; then the field becomes `TrustedSet<String>`
(probe `defects/probe_hash.nika`, D6). The Rust adapter cannot name a type that
moves. `BTreeMap` works everywhere and is deterministic. Its costs are O(log n)
per operation, not measured, and an `Ord` key (topic 4). 44 `HashMap`/`HashSet`
mentions are left in `check/` + `emit/` (10 type positions).
**Options:**
* (a) Convention (today's): `BTreeMap`/`BTreeSet` in `tools`.
* (b) Compiler: a declared `HashMap` field keeps one lowered type in a package
  that is not a program (`tools`), whatever builds it. That is an ADR-010 D5
  clarification, which the owner decides.
* (c) Leave it.

**Recommendation:** (a) now. Take up (b) only if a callgrind count on a moved
module shows the maps matter.

**Decided (October 11, 2026), neither (b) nor (c) as written** -
[ADR-010](specification/adr/adr-010.md) D9, Part III 17.1, issue #581. The
owner: the type is fixed only once all the information is there, and at a
package boundary both variants stay possible, as with generics. So: one hasher
per class of maps a value passes between, decided after the whole package is
read; a map in a `pub` signature or type is a hidden type parameter
(`nikaia_std::hash::Fast` / `Keyed`, default `Keyed`) that each use fills; the
ledger gains `maps`. No special rule for `tools`. **The mechanism, re-checked
(probes in #581):** D6 is not *whether the file builds one*. `tools` is lowered
with each file's own trust analysis, where a method call on a type another
file declares is unresolved and so untrusted; a project uses the entry file's
analysis for every file, so a socket read in another file gets the fixed seed.
(a) stays the convention until #581 is built and its callgrind count is in.

## 4. `Symbol` is not `Ord`

**What it is.** `winnow_grammar::Symbol` is an interned name, a `u32` handle.
Rust derives `Copy, Eq, Hash` and not `Ord` (`interner.rs:13`).
**Today (tried):** `BTreeMap[Symbol, V]` passes the Nikaia checker and fails in
rustc (`Symbol: Ord`, D15). `HashMap[Symbol, V]` compiles. Builders key by text
(`resolve(s).clone()`, an allocation per key). 6 maps/sets keyed by `Symbol`
are left.
**Options:**
* (a) Add `PartialOrd, Ord` to `Symbol` in winnow-grammar (the owner's crate:
  one line and a rev bump). Note: the order is interning order, not
  alphabetical. A `BTreeMap` iterated into output would change what is
  printed, so it is fine only where the order is not observable.
* (b) The checker refuses an ordered map whose key has no order (D15), and
  `HashMap[Symbol, V]` with a stable lowering (topic 3b).
* (c) Key by text, as today.

**Owner decision** for (a), since it is his crate. (b) is a compiler fix,
needed in any case.

## Ranking: lines saved per cost

| rank | measure | kind | cost | effect on the ~22.5k lines | owner? |
| ---: | :--- | :--- | :--- | :--- | :--- |
| 1 | `rs2nika.py` draft + `--lint` + the rule list in CLAUDE.md | tooling / convention | done (prototype) | 10% of lines exact, 48-60% of draft lines pass unchanged; saves the syntax-gap cycles | no |
| 2 | NK2302 precision (1a) | compiler, ADR-283 D25-D26 + Part I 6.6 | medium | decided; 17 of 19 view-returning functions in `check/`+`emit/` keep views; perf unmeasured | decided (#580) |
| 3 | fix D1, D2, D3, D8, D14 (lowering defects) | compiler | small each | each removes a workaround that costs one build cycle when met | no (bugs) |
| 4 | D15 refusal + D10/D11b checks | compiler | small | turns rustc errors into Nikaia messages | no |
| 5 | `Symbol: Ord` (4a) | dependency | trivial | 6 maps, fewer allocations | **yes** (his crate) |
| 6 | one `HashMap` type per class, a type parameter at the boundary (3) | compiler, ADR-010 D9 + Part III 17.1 | medium | decided (#581); `tools` may use `HashMap` where callgrind shows it | decided (#581) |
| 7 | explicit `ref(param)` results (1b) | language | high | rejected (ADR-283 D25) | decided |
