# Decision basis: text moved into a struct while a view of it lives (#942, step 1)

A note for the owner's decision. Nothing here is decided; no ADR or
specification text is written yet. Measured facts are marked **measured**,
everything else **not measured** or *from memory*.

## 1. The feature, from zero

Nikaia lets a function hand back a view of its argument without a copy and
without any lifetime written (Part I 6.5). A YAML parser does exactly that:

```nika
struct View { head: String }
struct Cursor { input: String, pos: i64 }

fn parse(t: String) -> View { return View { head: t } }   // View views t

fn main() throws {
    let text = fs::read_to_string("in.txt", fs::Root::Anywhere)
    let v = parse(text)                          // v views text, zero copy
    let mut c = Cursor { input: text, pos: 0 }   // text moves into the Cursor
    c.pos = c.pos + 1
    println(v.head)                              // v still used
    println(c.input)
}
```

Below, `View.head` becomes `&'a str` and `parse` takes `&str`; `Cursor.input`
stays an owned `String`. So the emitted Rust is
`let v = parse(&text); let mut c = Cursor { input: text, .. }; .. v.head ..`:
`text` is moved while `v` borrows it.

**What the user sees today (measured, `target/fast/nikaia run`, 2026-10-29):**

```
error: Cannot move out of `text` because it is borrowed.
 13 |     let mut c = Cursor { input: text, pos: 0 }
    = note: Consider cloning the value if the performance cost is acceptable.
```

That is rustc's E0505, translated: a message about a file nobody wrote
(Part III C.1 forbids exactly that).

**Why a question arises.** ADR-353 D3 says a caller may *give up* a value while
a view of it lives: its buffer moves into the keep and lives as long as the
views. The data never changes, so `v` and `c` could both read one buffer. But
`c.input` is declared `String`, an owner. One buffer cannot be owned by
`c.input` and kept alive for `v` without either changing what `c.input` is
below, counting owners, copying, or refusing. Which one is the decision.

## 2. Where the question comes from

* Issue #942, filed 2026-10-29 from a converter session (a YAML parser shape:
  `let v = parse(text); let mut c = Cursor { input: text, pos: 0 }`). A real
  program shape needs it; it is not bookkeeping. No program in the repository
  has it yet (grep: only the two tests of step 2).
* Step 2 (commit 1b1d740, 0.0.1272) is built: `view_changes`
  (`crates/nikaia-std/src/tools/view_changes.nika`, `vc_moved_parts`) follows a
  text moved into a struct field, tuple or list, and a change in place of that
  part while `v` lives is `NK2305` (ADR-353 D4). Step 1, the lowering, is open
  and was sent back for a decision by the builder (comment on #942).
* The owner asked to weigh the new Rust borrow checker (Polonius) before
  building anything it might later solve better. Section 6.

## 3. What is already decided

| Record | What it settles | Read as |
| :--- | :--- | :--- |
| ADR-353 D3 | giving a value up (reassignment, end of scope, **a move into another variable of the same function**, a hand-over to a view parameter) moves its buffer to the keep; no copy | decided; a move into a *field* is **not named** |
| ADR-353 D4 | a change in place of data a view may point into is `NK2305`, at the change | decided, built for fields (step 2) |
| ADR-353 D6 | handing the data to a **callee** that keeps it is `NK2305` at the hand-over; the copy is the program's; copy-on-write rejected | decided for callees; its reason ("a move into a variable of the same function is not refused: the compiler sees every change of that variable") applies just as well to a local struct |
| ADR-353 D7 | a struct has one place per view-holding field; an enum one place | decided |
| ADR-283 D3 | the compiler never inserts a copy | decided; rules out an inserted `.clone()` |
| ADR-283 D9-D10, D23 | a buffer whose views outlive its scope moves into a keep; a function whose result carries views takes the caller's keep | decided; for a *struct* result holding a view of a local, not built for this shape (fact F7) |
| ADR-283 D14 | emitted code for held views writes no `unsafe` | decided; rules out option E as it stands |
| ADR-282 D13-D16 | a declared `String` position is a view, owned, or `EitherText`, by what flows in; fields, tuples, lists, variant payloads are positions | decided and built (facts F5, F6) |
| ADR-001 D1 | the toolchain is stable; no `-Z` flag in any build | decided |
| `docs/history/nightly-cost.md` §3 | `-Zpolonius=next` changed 0 of 30 verdicts at +7.11 % instructions; nightly withdrawn | measured then (older nightly) |

So the open question is narrow: **is a struct/tuple/list/variant literal that
takes a viewed text a "variable of the same function" (D3) or a "new owner"
(D6)**, and, if D3, what the field becomes below.

## 4. Facts, checked by trying them

Probes in Nikaia, run with `target/fast/nikaia run` (built 2026-10-29 07:40,
includes step 2). **All measured.**

| # | Program shape | Today |
| :--- | :--- | :--- |
| F1 | owner's example: `Cursor { input: text }` while `v` lives | rustc E0505, as above |
| F1b | the same with a tuple `(text, 0)`, a list `[text]`, a variant `E::A(text)` | rustc E0505 each |
| F1c | the same inside `fn make() -> Cursor`, returning `c` | rustc E0505 |
| F2 | `let moved = text` then only read `moved` | runs: `text` is `__keep_frame.put(..)`, `moved: &String` |
| F3 | `let moved = text; Cursor { input: moved }` | rustc E0308 (mismatched types): `&String` into `String`. The existing D3 path has the same gap |
| F4 | `c.input.push_str(..)` while `v` lives (step 2) | `NK2305` at the change |
| F5 | `Cursor { input: v.head }` (a view given directly) | runs: tiers make `Cursor<'a> { input: &'a str }` |
| F5b | `E::A(v.head)`, `(v.head, 0)`, `[v.head]` | runs: `E<'a> { A(&'a str) }`, tuple and `Vec<&str>` |
| F6 | one `Cursor` built from a view, another from `f"…"` | runs: `input: EitherText<'a>` (ADR-282 D14) |
| F7 | `fn make() -> Cursor` building `Cursor { input: v.head }`, `v` used after, `c` returned | `NK2105` ("`v.head` was stored in `Cursor.input`"; help: `.clone()`): the tiers make the returned field owned |
| F8 | `Cursor { input: v.head }` and later `c.input.push_str` | `NK1106` ("`Cursor` keeps its `input` … needs text of its own"; help `.clone()`) |
| F9 | the zero-copy reorder: `let mut c = Cursor { input: text }; let v = parse(c.input); c.pos = c.pos + 1` | **`NK2305` at `c.pos = …`: a false refusal.** `v` points into `c.input`, not `c.pos`; the same shape in Rust is accepted by every setting (R9 in §6). Filed as #944 |

Reading of the facts:

* The tiers (option A below) already exist for every literal form #942 names,
  and already fall back to a Nikaia-level refusal with `.clone()` where the
  field must own its text (F7, F8). What is missing is only that a **move** of
  a viewed local into the position counts as a view flowing in.
* F9 matters for the owner's real program: a cursor advances `pos`. Whatever
  is chosen, a `Cursor` whose `pos` changes while `v` lives must not be refused.

## 5. Options, with consequences

All Rust verdicts in the column *rustc* are from §6's matrix (measured).

**A. Text tiers: the move counts as a view flowing in.** `Cursor.input`
becomes `&'a str` (or `EitherText<'a>` where owned text also flows in); the
lowering writes `Cursor { input: &text }` and `text` stays in the frame
(Borrowed) or in the keep (Tethered) as D3 says. Nikaia still treats `text` as
moved (`NK2105` on a later use of `text`).
* rustc: accepted, locally (A1) and when `c` leaves through the caller's keep
  (A2). Returned without a keep: E0515 (A3), which the tiers already turn into
  the owned fallback (F7).
* Consequences: the field's representation changes program-wide, invisible in
  Nikaia; a struct gains a lifetime below; a mixed field pays one branch per
  read (`EitherText`, not measured here); a field changed in place anywhere
  stays owned and the move is refused with `NK1106` (F8); a struct returned
  past the frame gets `NK2105` until ADR-283 D10 is built for struct results
  (F7). Fixes F3 too. Variants, tuples, lists come for free (F5b).
* Risk: a "spooky" program-wide change: adding `c.input.push_str` in one
  function turns a construction elsewhere into a refusal. The message names
  the field, not the far site (not measured how often).

**B. A literal field is a new owner: refuse the move with `NK2305` (D6).**
* rustc never sees it. Simple, local, one rule.
* Consequences: the owner's example is refused; ways out are `.clone()` (one
  copy) or the reorder of F9 (zero copy, once F9's bug is fixed). It
  contradicts D6's own reason (the compiler sees every change of a local
  struct, so D4 covers it) and D3's spirit; F3 (`let moved = text; Cursor {
  input: moved }`) would need the same refusal to stay consistent.
* Risk: refuses programs whose data is never in danger, the outcome ADR-353 §3
  rejected for Rust.

**C. The keep hands back a shared owner (a count).** The field becomes a
counted buffer below (`Rc<str>`/`Arc<str>`, or `Bytes`-like), the keep holds
one count, `c.input` another.
* rustc: accepted (C3). A plain move of an `Rc` local is still E0505 (C2): the
  view must point into the keep, not the local.
* Consequences: the representation changes program-wide just as in A (no
  lifetime, but a count); every move costs an increment, every drop a
  decrement; a change in place needs uniqueness, i.e. copy-on-write, which
  ADR-353 D6 rejected. Swift's model.

**D. Insert the copy, with a note.** rustc accepts (K1). Forbidden by
ADR-283 D3 and ADR-282 D7 (no inserted copy). Listed for completeness.

**E. Unchecked stable address.** Lower `v` as an unchecked view of the heap
buffer; moving a `String` does not move its heap bytes. rustc accepts (U1),
because the borrow is laundered through a raw pointer.
* Soundness rests entirely on Nikaia: D4 must refuse every change in place,
  and `c` must not be dropped or reassigned before `v`'s last use (the drop of
  `c` frees the buffer `v` reads). It depends on `String` having no inline
  (small-string) representation: C++'s `std::move` of a short `std::string`
  copies the bytes, and a `string_view` dangles. Whether moving a `String` is
  a retag under Rust's aliasing models: not measured (Miri not run).
* Violates ADR-283 D14 (no `unsafe` in emitted views). Not recommended.

**F. A self-referential pair** (`ouroboros`, `yoke`, `self_cell` in Rust):
lower `c` and its dependants into one compiler-written owner+views value.
`v` and `c` are separate values here, so it means fusing unrelated locals;
large and only a variant of E with a crate's `unsafe`. Not recommended.

**G. A field-level "kept" type** (`input` holds `&'k String` into the keep, or
a handle): with a reference it is A with `&String` instead of `&str`; with a
handle it is C. No new ground.

**H. The compiler reorders** (build `c` first, view from `c.input`): changes
evaluation order of calls that may have effects. Rejected; but it is the way
out a message can *name* (F9, once fixed).

**Mixed rule (A, with B's refusal as the defined fallback).** A literal that
takes a viewed text inside the function is D3 (option A); where the position
must hold text of its own (changed in place, returned past the keep, handed to
a keeping callee) the program is refused in Nikaia terms with `.clone()` and
the reorder as ways out. This is A as it already behaves (F7, F8); the open
point is only which code and wording the fallback carries.

## 6. Polonius: does the new borrow checker change anything?

**State (from the brief's sources, not re-fetched):** Polonius "alpha" is on by
default on nightly since 2026-08 (Rust blog, *Enabling the next iteration of
the borrow checker on nightly*), stabilisation expected later in 2026, opt out
with `-Zpolonius=off`. Its gain is flow-sensitive outlives checking: the
conditional return of a borrow (NLL problem case #3). The *Borrow Checker
Within* roadmap items (view types, place-based lifetimes, self-referential
structs, borrowing from a moved value) are not part of it and have no date.

**Measured, 2026-10-29.** The Rust shapes the compiler emits (taken from
`nikaia lower` for F1-F3; `nikaia_std` calls replaced by `std` equivalents and
the `async` dropped, the borrow structure unchanged; `tether::Keep::put` with
its real signature `fn put<B>(&self, B) -> &B`) and the hand-written option
shapes, each compiled `--emit metadata` with stable 1.97.0 and nightly
1.101.0 (2026-10-28) under its default (`-Z polonius` help: "default: next"),
`-Zpolonius=off` and `-Zpolonius=legacy`:

| Case | Shape | stable | nightly default | `=off` | `=legacy` |
| :--- | :--- | :--- | :--- | :--- | :--- |
| R1 | owner's example, as emitted | E0505 | E0505 | E0505 | E0505 |
| R2 | `let moved = text; Cursor { input: moved }`, as emitted | E0308 | E0308 | E0308 | E0308 |
| R3 | `let moved = text`, read only (control) | ok | ok | ok | ok |
| R4 / R5 / R6 | tuple / list / variant, as emitted | E0505 | E0505 | E0505 | E0505 |
| R7 | returning `c`, as emitted | E0505 | E0505 | E0505 | E0505 |
| P2 | the move with `v` dead afterwards (control) | ok | ok | ok | ok |
| A1 | option A, local | ok | ok | ok | ok |
| A2 | option A, `c` returned through the caller's keep | ok | ok | ok | ok |
| A3 | option A, returned without a keep | E0515 | E0515 | E0515 | E0515 |
| A4 | option A, owned text into a `&str` field (why `EitherText`) | E0308 | E0308 | E0308 | E0308 |
| C2 | option C, the `Rc` local moved | E0505 | E0505 | E0505 | E0505 |
| C3 | option C, view of the keep, count into the field | ok | ok | ok | ok |
| K1 | option D, the copy | ok | ok | ok | ok |
| U1 | option E, laundered pointer | ok | ok | ok | ok |
| R9 | F9's reorder, hand-written (disjoint fields) | ok | ok | ok | ok |
| P1 | positive control: `get_or_insert` on a `HashMap` | E0502 | **ok** | E0502 | **ok** |

**Answers.**

1. **Polonius, now, changes no verdict in this question**: 17 of 17 shapes
   identical across all four settings; only the control P1 flips, which shows
   the harness sees Polonius when there is something to see. Same result as
   `nightly-cost.md` §3 (0 of 30).
2. **Why, and why its roadmap does not either:** E0505 is a move while a loan
   is *live*, and `v`'s later use makes it live on every path; flow-sensitive
   outlives does not touch that. The item that would - borrowing from a value
   after it moved, or a self-referential struct - is unscheduled; if it ever
   lands it would let option E's shape go without `unsafe`, nothing else.
3. **Which options stay valid whichever way Rust goes:** A, B, C and D. Each
   emits Rust that NLL accepts, and Polonius is built to accept a superset of
   NLL (measured: no case accepted by NLL and rejected by Polonius, here and in
   the 30-program corpus). B does not reach rustc at all. Only E/F would be
   made better by a future Rust; nothing would be made unnecessary.
4. **Should the lowering rely on it:** no. ADR-001 D1 forbids `-Z`; nothing in
   this question needs it; and once Polonius is stable it arrives without a
   flag, so the lowering gains its acceptances (the `get_or_insert` shape,
   ADR-005 D2) for free. Nothing here should wait for it.

## 7. How other languages do it

*From memory, not checked against a current manual.*

| Language | View into `text`, then `text` moved into a struct |
| :--- | :--- |
| Rust | refused (E0505); idioms: reorder (own first, borrow from the owner), `.clone()`, `Rc`/`Arc`, or a self-referential crate (`yoke`, `self_cell`) over a stable-address owner |
| Swift | `String` is a counted copy-on-write value: the struct shares the buffer, a `Substring` keeps the base alive (option C). The newer non-escapable types (`Span`, lifetime dependencies) refuse consuming the source while the dependant lives (option B) |
| Mojo (origins) | a reference carries the origin of `text`; transferring `text^` while the reference is used is refused |
| Vale | the object is not moved in memory by moving its owner; non-owning references are checked by generation at run time, so the view survives (E, checked at run time) |
| Hylo (Val) | no stored references (mutable value semantics); a view does not outlive its projection, so the program is restructured or copies |
| Zig | a slice is a pointer; moving the struct keeps the heap pointer; nothing checked (E, unchecked) |
| C++ | `string_view` + `std::move`: dangles for short strings (inline storage), works by accident for long ones |
| GC languages | the collector keeps the buffer alive for both |

Checked languages either share by count (Swift) or refuse (Rust, Mojo,
Swift's `Span`). Nikaia's keep is the third way the collector shows: keep the
buffer for the views, without a count where a frame outlives them. Option A is
that way applied to the field.

## 8. Measurements still owed

* Cost of `EitherText` per read where a field becomes mixed (one branch;
  callgrind count not taken).
* How often the program-wide flip of F8 would surprise (needs a corpus of
  parsers; none in the repository has this shape).
* Option C's count traffic, if C is chosen (not measured).
* Option E under Miri (only if the owner wants E kept on the table).

## 9. Risks of a wrong decision

* A wrongly built A or C: at worst a rustc message again (a refusal about a
  file nobody wrote), never a wrong value: rustc still checks every reference.
* B: correct programs refused; users copy where no copy is needed.
* E: undefined behaviour if any of its three conditions fails silently (a
  change in place D4 missed, an early drop, a future inline `String`).

## 10. Recommendation

**Option A, with B's refusal only where the position must own its text**
(the mixed rule of §5). Reasons: it is what ADR-353 D3 and D6's own reason
already say about values the function can see; it reuses the tiers, which
already handle all four literal forms (F5b) and already refuse in Nikaia terms
where owning is needed (F7, F8); it adds no `unsafe`, no count and no copy; it
is valid under every Rust borrow checker, present or announced. It also closes
F3. Before or with it, fix F9 (#944, a false `NK2305` on `c.pos` while `v` views
`c.input`), or the owner's cursor stays refused either way.

Not recommended: D (forbidden), E/F (`unsafe`, ADR-283 D14, representation
risk), C (a count everywhere and copy-on-write, both rejected in ADR-353 D6).
Polonius: neither wait for it nor depend on it.

## 11. Questions for the owner

1. Is a struct, tuple, list or variant literal inside the function a "variable
   of the same function" (ADR-353 D3), so the move is kept, not refused? (A vs B)
2. Is a field's representation changing program-wide (view / `EitherText`)
   because a moved text flows in acceptable, as it already is for a view
   flowing in (ADR-282 D13)?
3. Where the field must own its text (changed in place, returned past the
   frame, handed to a keeping callee): the existing refusals (`NK1106`,
   `NK2105`), or one `NK2305` worded as D6 (hand over a copy, or build the
   struct first and view its field)?
4. Returning such a struct (F1c, F7): stay refused until ADR-283 D10 is built
   for struct results, or build that as part of #942?
5. Confirm: the lowering does not rely on Polonius or any nightly feature
   (ADR-001 D1), and nothing in #942 waits for Rust's roadmap.
