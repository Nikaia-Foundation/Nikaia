# ADR-pointers-v1: A Raw Pointer Is Written Inside `unsafe`; Where It Points Into a Value of the Program the Compiler Checks It, Where It Points Into Raw Memory It Does Not

**Status:** Accepted
**Date:** November 21, 2026 (owner's decision on #2486: option C, and the four questions under its recommendation as proposed)
**Answers:** whether a program may hold a raw pointer, which operations on one
are written where, and what the compiler checks about it
([#2486](https://github.com/Nikaia-Foundation/Nikaia/issues/2486))
**Supersedes / Related:**
[ADR-c-boundary-v1](adr-c-boundary-v1.md) D9 (an address C may keep is this record's pointer; D5 and D7 stay the forms to reach for), D3 (what `unsafe` holds), D4 (what is not said is fail-closed), D11 (`T?` over a non-null address is one word);
[ADR-views-v4](adr-views-v4.md) D9, D11 (a change in place while a view lives, a value lent `mut` for a whole call);
[ADR-283](adr-283.md) D23 (a buffer moves into the anchor of whatever holds its views);
[ADR-037](adr-037.md) D7 (what is observable is written, what is not is inferred, safe as the floor);
[ADR-raw-sinks-1](adr-raw-sinks-1.md) D4, D5 (a dependency's key holds only where the program repeats it);
[ADR-218](adr-218.md) D1, D2 (every `unsafe` topic of the toolchain is a small Rust crate, checked under Miri);
[ADR-005](adr-005.md) Group B (what may cross a thread is checked structurally);
Part I 6.5, 6.6; Part III 15.1, 13.4, C

## 1. Question

**May a program hold a raw pointer, and if so, what does the compiler check
about it?**

Without one, an allocator, an intrusive list, a ring buffer over raw memory and
an address a C library keeps past its call cannot be written in Nikaia; they are
written in Rust, in a crate of [ADR-218](adr-218.md) or a described crate. The
question is not whether a pointer can go wrong: inside `unsafe` it may, as in
Rust, and a call into C already can. It is whether the programmer can keep the
rules a pointer needs. In Rust the author of an `unsafe` block sees every
reference the block could break. In Nikaia the compiler makes many of them: a
lent parameter may become a reference, a `let` over a place is a view, a buffer
moves into an anchor after its last change (Part I 6.6). A pointer the compiler
did not check would break rules its author cannot see.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Rust | `*const T`, `*mut T`; taking one is safe, dereferencing is `unsafe`; nothing is checked | undefined behaviour no compiler sees; Miri finds some of it in tests | from memory, not checked |
| C# | pointers inside `unsafe`; the address of a movable variable is taken only inside `fixed`, which pins it, because the garbage collector moves objects | a scope around every address of a managed value | from memory, not checked |
| Swift | `withUnsafePointer(to:)` / `withUnsafeMutablePointer(to:)` hand a pointer to a closure; using it after the closure is undefined behaviour | no stored pointer into a Swift value | from memory, not checked |
| Go | `unsafe.Pointer` with written rules for converting to and from `uintptr`, because the runtime moves stacks | rules checked by `go vet` in part | from memory, not checked |
| Zig | pointers are ordinary values: `*T` for one element, `[*]T` for many, with arithmetic | nothing hidden moves, so nothing needs tracking | from memory, not checked |

What does not transfer: Rust and Zig rely on every reference and every move
being in the source; here they are not, so their "nothing is checked" would
leave the programmer rules nobody can keep. What does: C#'s and Swift's answer
to a value that moves under a pointer is a scope, and the compiler here knows
every point a value moves, so it can check the scope instead of making the
program write it.

## 3. Options, weighed

### Option A — No raw pointer

Everything a pointer is for is written in Rust, in a crate of ADR-218 or a
described crate.

### Option B — A pointer only inside a block over a place

```nika
unsafe {
    raw::with_address_mut(buffer[0], fn(p) { p.offset(4).write(0) })
}
```

### Option C — A pointer type, checked where it points into a value of the program

```nika
use std::raw

fn fill(mut buf: Vec[u8]) {
    unsafe {
        let p: raw::PointerMut[u8] = raw::address_mut(buf[0])
        p.offset(1).write(7)
    }
}
```

### Option D — A pointer type, nothing checked, as in Rust

The same source as C, with no rule but that it is written inside `unsafe`.

### Weighed

| | D1 the special case pays, not the common one | D2 the compiler works, not the developer | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | the special case pays in a second language | nothing to work; the developer switches language | none | the toolchain's own rule *written in Nikaia only* cannot hold for an `unsafe` topic | every new topic is new Rust; undone by admitting a pointer later |
| **B** | pays only inside the block | the block holds the place, so nothing moves and no view is used meanwhile | none | a second spelling beside `ref` for a pointer that lives for a scope | allocators, intrusive structures and addresses C keeps stay in Rust |
| **C** | pays only where a pointer is written | the compiler checks what it made: the moves it inserted and the views it chose | none: a program without a pointer is unchanged | reuses the views analysis and the boundary's fail-closed rule | a rule too narrow refuses a sound program and is widened; a rule too wide is undefined behaviour Miri may catch |
| **D** | pays only where a pointer is written | the developer must keep rules about references and moves that are not in the source | none | contradicts D7 of ADR-037: what the program cannot see is not left to it | undefined behaviour no review can see; narrowing it later breaks programs |

C was chosen. A lost because every topic a pointer is for would stay in a
second language. B lost because a pointer can never be stored, which is half of
what a pointer is for. D lost because its rules cannot be kept by the person
they bind.

## 4. Decision

### D1 — `raw::Pointer[T]` reads, `raw::PointerMut[T]` reads and writes; neither is null.

Both are types of `std::raw`, named with the module (`raw::Pointer[u8]`), as
`foreign::CStr` is. The pair follows `Shared[T]` / `SharedMut[T]`. A pointer
holds a non-null address; `raw::Pointer[T]?` is the pointer that may be null,
in one machine word ([ADR-c-boundary-v1](adr-c-boundary-v1.md) D11), read with
`??` and `?.` as any `T?`. `p.reading()` makes a `Pointer[T]` of a
`PointerMut[T]`. A pointer is a value: it is moved, copied, stored in a field
and compared with `==` like a number.

### D2 — Every operation on a pointer is written inside `unsafe`.

Taking one (`raw::address(place)`, `raw::address_mut(place)`), reading
(`p.read()`), writing (`p.write(v)`), moving it (`p.offset(n)`, `n` elements),
casting it (`p.cast[U]()`), making and freeing raw memory (D7) and converting
to and from an integer (`p.to_address()`, `raw::from_address[T](a)`) stand
inside an `unsafe` block; outside one each is refused with `NK1143`, as a call
into C is. Holding, handing on and storing a pointer is not an operation. So
all pointer code of a package is in its `unsafe` blocks, and `--trust` can list
it (D10).

### D3 — `raw::address_mut` takes a place the function may change.

A `let mut`, a `mut` parameter, `mut self`, a `for mut` binding, or a part of
one of them: the places a change may reach today (Part I 6.5). Of any other
place it is refused with `NK1138`, as a change through it would be.
`raw::address` takes any place.

### D4 — A pointer into a place holds until the place is next moved or changed by another way.

A move ends it whether the program wrote it (`Cursor { input: text }`, a `keep`
parameter, a hand-over at a last use) or the compiler inserted it: a buffer
moved into the anchor of whatever holds its views, at its creation or after its
last change ([ADR-283](adr-283.md) D23, Part I 6.6). A change ends it where it
does not go through the pointer: an assignment to the place or a part of it, a
method that changes it, a `mut` hand-over. So does the place's end. A use of
the pointer after that is refused with `NK2306`: the message names the place,
the line that took the pointer and the line that moved or changed it, and for
an inserted move it names the view that made the move necessary, so the program
knows what to change. A pointer's own copies hold as long as it does.

The compiler knows each of these points while it builds the program, because it
emits them; `--tethers` prints where each buffer lives. A move inserted at the
creation of a buffer precedes every pointer to it and ends none.

### D5 — A write through a pointer is a change in place of its source; handing one on lends its source.

A write through a `PointerMut[T]` into a place is a change in place of that
place: a view of it still used afterwards is `NK2305`, as for any change
([ADR-views-v4](adr-views-v4.md) D9). Handing a `PointerMut[T]` into a place to
a call is a `mut` hand-over of that place for the call, and handing a
`Pointer[T]` lends it, so a call given both the pointer and a view of the same
place is refused as a call given a value `mut` and a view of it
([ADR-views-v4](adr-views-v4.md) D11).

### D6 — A pointer into a place does not leave the place's scope.

It is not returned, not kept (stored into something that outlives the place,
handed to a `keep` parameter), not given to a task, not handed to an `extern`
declaration's `raw::Pointer` parameter (C may keep it,
[ADR-c-boundary-v1](adr-c-boundary-v1.md) D9) and not converted to an integer.
Each is refused with `NK2307`, naming the place and where the pointer would
outlive it. This is the narrow start: a rule that lets more through later
breaks no program, and one that let everything through could not be narrowed.

### D7 — A pointer into raw memory is not checked.

`raw::alloc[T](count)` hands back a `raw::PointerMut[T]?` to room for `count`
values of `T`, aligned for `T`, uninitialised, or `null` where none is left;
`raw::free(p)` gives it back. Memory from `raw::alloc` has no cleanup: it is the
program's to free. A pointer from `raw::alloc`, from a C function or from an
integer is checked by nothing: how long it holds, how far it reaches, whether
the memory is initialised and what else points there are the program's. That
is the part that is `unsafe` in Rust too, and it is the only such part here.

### D8 — A function with an unchecked pointer operation is fail-closed in the ledger.

A function that reads or writes through a pointer of D7 has `touches`
*everything* and `locks` *may*, as an `extern` declaration
([ADR-c-boundary-v1](adr-c-boundary-v1.md) D4), and so does every caller the
derivation reaches through it. A function whose pointers all point into its own
places (D3-D6) keeps its derived entry: a write through one is counted as the
change of that place it is (D5).

### D9 — A pointer does not cross a thread.

`raw::Pointer[T]` and `raw::PointerMut[T]` are `crosses = false`: a task that
uses one is `NK2501`, a call the compiler cannot see the end of given one is
`NK2502`.

### D10 — A dependency's pointer code is admitted by the program, and `--trust` lists it.

A package's own code needs no key. A dependency whose code holds a pointer
operation (D2) builds into a program only where the program's manifest says
`unsafe-pointers = true` under `[allow.packages.NAME]`, or `--allow=NAME:unsafe-pointers`
gives it; otherwise the first such operation is refused with `NK1309`, as a
dependency's raw sink is ([ADR-raw-sinks-1](adr-raw-sinks-1.md) D4, D5).
`--trust` lists every pointer operation of the package and of each admitted
dependency, with its place, and their number per package.

### D11 — Code with pointer operations runs under Miri in CI.

The tests of a package of this repository that hold a pointer operation run,
as lowered Rust, under Miri with both of its aliasing models (Stacked Borrows
and Tree Borrows), as the crates of [ADR-218](adr-218.md) D2 do. Miri is the
check for what D7 leaves to the program, and for this compiler's own lowering
of D4-D6.

### D12 — The `unsafe` crates of ADR-218 stay.

Each crate under `crates/unsafe/` keeps its place; whether one is rewritten in
Nikaia with this record's pointers is decided for that crate, by its own
issue, not here.

## 5. Consequences

* Allocators, intrusive structures, ring buffers over raw memory and addresses
  C keeps can be written in Nikaia, inside `unsafe`. A new `unsafe` topic of
  the toolchain no longer needs new Rust.
* The pointer the programmer takes into their own values is held to the same
  analysis as a view: what the compiler moved or lent is what it checks. Only
  D7's raw memory is the programmer's alone.
* A program without a pointer is unchanged and pays nothing.
* **What it costs:** two types and a dozen `std::raw` entries; two refusals
  (`NK2306`, `NK2307`); the views analysis follows pointers, which needs every
  inserted move to be exact (#2465 is a defect in exactly that machinery); a
  manifest key; Miri runs in CI for the code that uses it. The lowered Rust
  must stay sound under an aliasing model Rust has not made final; running
  both of Miri's models is how that is watched.

## 6. What this does not decide

* Volatile access, for device registers, and a typed register layer
  ([ADR-119](adr-119.md), a target without an operating system).
* Pointer arithmetic beyond `offset` (a difference of two pointers, alignment
  queries).
* A pointer into a place that leaves its scope under a proof (D6 widened), for
  example into a value the caller keeps alive.
* Whether a program's own pointer code needs a key of its own, as a raw sink
  does; D10 asks it of dependencies only.
